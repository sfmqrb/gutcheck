//! The Laya decision model on ONNX Runtime: download, prompt layout, scoring, answer cache.
use anyhow::{Context, Result};
use ort::session::{builder::GraphOptimizationLevel, Session};
use ort::value::Tensor;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tokenizers::Tokenizer;

// Pinned community ONNX export of convaiinnovations/laya-multilingual (Apache-2.0).
const REPO: &str = "https://huggingface.co/soyelmismo/laya-multilingual-onnx/resolve/0966c4fa58da6878b39e7e14cb5e93313b82d828";
const FILES: [(&str, &str); 2] = [("model-fp32.onnx", "model.onnx"), ("tokenizer/tokenizer.json", "tokenizer.json")];
// From the checkpoint's rl_agent_config.json. Its fitted temperatures are all 1.0, so none are applied.
// ponytail: hardcoded for this one checkpoint; read rl_agent_config.json if the model becomes swappable.
const HEAD_MAX_LEN: usize = 256;
// Repeated lines reuse their answer. ponytail: cleared when full, swap for an LRU if it ever thrashes.
const CACHE_MAX: usize = 50_000;

/// A question the way Laya's `build_sequence` reads it.
pub struct Question {
    qtype: i64, // 0 choice, 2 noul (yes/no)
    head: Vec<u32>,
    options: Vec<Vec<u32>>, // each starts with the mask token
}

pub struct Model {
    session: Session,
    tok: Tokenizer,
    special: [u32; 3], // cls, sep, mask
    max_len: usize,
    fuzzy: bool,
    cache: HashMap<(usize, String), Vec<f32>>,
}

impl Model {
    /// `max_len` caps tokens per record (question included); longer text is cut. `threads` = 0 lets ONNX Runtime use every core. `fuzzy` answers lines that differ only in numbers and ids once.
    pub fn load(fuzzy: bool, threads: usize, max_len: usize) -> Result<Self> {
        let dir = model_dir()?;
        let tok = Tokenizer::from_file(dir.join("tokenizer.json")).map_err(anyhow::Error::msg)?;
        let id = |t: &str| tok.token_to_id(t).with_context(|| format!("tokenizer has no {t}"));
        let special = [id("<bos>")?, id("<eos>")?, id("<mask>")?];
        let path = std::env::var_os("GUTCHECK_MODEL").map(PathBuf::from).unwrap_or(dir.join("model.onnx"));
        let mut builder = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3).map_err(|e| anyhow::anyhow!("{e}"))?;
        if threads > 0 {
            builder = builder.with_intra_threads(threads).map_err(|e| anyhow::anyhow!("{e}"))?;
        }
        Ok(Self { session: builder.commit_from_file(&path)?, tok, special, max_len, fuzzy, cache: HashMap::new() })
    }

    fn enc(&self, s: &str) -> Result<Vec<u32>> {
        Ok(self.tok.encode(s.replace("<mask>", " "), false).map_err(anyhow::Error::msg)?.get_ids().to_vec())
    }

    fn option(&self, s: &str) -> Result<Vec<u32>> {
        let mut o = vec![self.special[2]];
        o.extend(self.enc(&format!(" {s}"))?.into_iter().take(48));
        Ok(o)
    }

    pub fn yes_no(&self, question: &str) -> Result<Question> {
        Ok(Question {
            qtype: 2,
            head: self.enc(&format!("noul question: {question}"))?,
            options: vec![self.option("false: no, the statement does not hold")?, self.option("true: yes, the statement holds")?],
        })
    }

    pub fn choice(&self, question: &str, labels: &[String]) -> Result<Question> {
        Ok(Question {
            qtype: 0,
            head: self.enc(&format!("choice question: {question}"))?,
            options: labels.iter().map(|l| self.option(l)).collect::<Result<_>>()?,
        })
    }

    /// Probabilities over the question's options (yes/no: `[P(no), P(yes)]`). `id` tells questions apart in the cache.
    pub fn probs(&mut self, id: usize, q: &Question, text: &str) -> Result<Vec<f32>> {
        let key = (id, if self.fuzzy { template(text) } else { text.to_string() });
        if let Some(p) = self.cache.get(&key) {
            return Ok(p.clone());
        }
        let [cls, sep, _] = self.special;
        let (ids, markers) = build_sequence(q, &self.enc(text)?, cls, sep, self.max_len, HEAD_MAX_LEN);
        let (len, k) = (ids.len(), markers.len());
        let outputs = self.session.run(ort::inputs! {
            "input_ids" => Tensor::from_array(([1, len], ids.iter().map(|&t| t as i64).collect::<Vec<_>>()))?,
            "attention_mask" => Tensor::from_array(([1, len], vec![1i64; len]))?,
            "marker_pos" => Tensor::from_array(([1, k], markers.iter().map(|&m| m as i64).collect::<Vec<_>>()))?,
            "marker_mask" => Tensor::from_array(([1, k], vec![true; k]))?,
            "qtype" => Tensor::from_array(([1], vec![q.qtype]))?,
        })?;
        let p = softmax(&outputs["logits"].try_extract_tensor::<f32>()?.1[..k]);
        if self.cache.len() >= CACHE_MAX {
            self.cache.clear();
        }
        self.cache.insert(key, p.clone());
        Ok(p)
    }
}

/// `[CLS] <type> question: ins [SEP] [MASK] opt0 [MASK] opt1 ... [SEP] state [SEP]`, returns (ids, marker positions).
fn build_sequence(q: &Question, state: &[u32], cls: u32, sep: u32, max_len: usize, head_max_len: usize) -> (Vec<u32>, Vec<usize>) {
    let mut opts = q.options.clone();
    let mut budget = head_max_len as isize - opts.iter().map(|o| o.len() as isize).sum::<isize>();
    if budget < 16 {
        let per = 4.max((head_max_len.saturating_sub(16)) / opts.len().max(1));
        opts.iter_mut().for_each(|o| o.truncate(per));
        budget = head_max_len as isize - opts.iter().map(|o| o.len() as isize).sum::<isize>();
    }
    let head = &q.head[..q.head.len().min(budget.max(8) as usize)];
    let mut ids = vec![cls];
    ids.extend(head);
    ids.push(sep);
    let mut markers = vec![];
    for o in &opts {
        markers.push(ids.len());
        ids.extend(o);
    }
    ids.push(sep);
    let room = max_len.saturating_sub(ids.len() + 1);
    ids.extend(&state[..state.len().min(room)]);
    ids.push(sep);
    ids.truncate(max_len);
    markers.retain(|&m| m < max_len);
    (ids, markers)
}

fn softmax(z: &[f32]) -> Vec<f32> {
    let m = z.iter().cloned().fold(f32::MIN, f32::max);
    let e: Vec<f32> = z.iter().map(|x| (x - m).exp()).collect();
    let s: f32 = e.iter().sum();
    e.iter().map(|x| x / s).collect()
}

/// Line shape for `--fuzzy`: digit runs become `#`, long hex ids become `<hex>`.
/// ponytail: hand-rolled, so "85%" and "99%" look alike; a number-aware bucket is the upgrade if that bites.
pub fn template(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut run = String::new();
    let flush = |run: &mut String, out: &mut String| {
        if run.len() >= 8 && run.chars().all(|c| c.is_ascii_hexdigit()) && run.chars().any(|c| c.is_ascii_digit()) {
            out.push_str("<hex>");
        } else {
            let mut prev_digit = false;
            for c in run.chars() {
                let d = c.is_ascii_digit();
                if !(d && prev_digit) {
                    out.push(if d { '#' } else { c });
                }
                prev_digit = d;
            }
        }
        run.clear();
    };
    for c in line.chars() {
        if c.is_alphanumeric() {
            run.push(c);
        } else {
            flush(&mut run, &mut out);
            out.push(c);
        }
    }
    flush(&mut run, &mut out);
    out
}

/// Where the model lives; downloads it (with a progress readout) the first time.
fn model_dir() -> Result<PathBuf> {
    let cache = std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join(".cache")))
        .context("set XDG_CACHE_HOME or HOME")?;
    let dir = cache.join("gutcheck").join("laya-multilingual-0966c4f");
    for (remote, local) in FILES {
        let dest = dir.join(local);
        if dest.exists() {
            continue;
        }
        std::fs::create_dir_all(&dir)?;
        eprintln!("gutcheck: first run, downloading {local} to {} ...", dir.display());
        let tmp = dest.with_extension("part");
        let resp = ureq::get(&format!("{REPO}/{remote}")).call()?;
        let total = resp.body().content_length().unwrap_or(0);
        let mut body = resp.into_body();
        let (mut r, mut f, mut buf, mut done) = (body.as_reader(), std::fs::File::create(&tmp)?, vec![0u8; 1 << 20], 0u64);
        loop {
            let n = r.read(&mut buf)?;
            if n == 0 {
                break;
            }
            f.write_all(&buf[..n])?;
            done += n as u64;
            if total > 0 {
                eprint!("\r  {:>3}% of {} MB", done * 100 / total, total >> 20);
            }
        }
        eprintln!();
        std::fs::rename(&tmp, &dest)?;
    }
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuzzy_template() {
        assert_eq!(template("2026-09-12T09:15:11Z conn from 10.0.0.5 id 3fa9c1d2e4 failed 3 times"), "#-#-#T#:#:#Z conn from #.#.#.# id <hex> failed # times");
        assert_eq!(template("deadbeef stays a word"), "deadbeef stays a word");
    }

    #[test]
    fn sequence_layout_and_truncation() {
        let q = Question { qtype: 2, head: vec![10, 11], options: vec![vec![9, 20], vec![9, 21, 22]] };
        let (ids, m) = build_sequence(&q, &[30, 31, 32], 1, 2, 64, 256);
        assert_eq!(ids, vec![1, 10, 11, 2, 9, 20, 9, 21, 22, 2, 30, 31, 32, 2]);
        assert_eq!(m, vec![4, 6]);
        // state is cut to fit max_len, keeping the trailing [SEP]
        let (ids, _) = build_sequence(&q, &[30; 100], 1, 2, 16, 256);
        assert_eq!((ids.len(), *ids.last().unwrap()), (16, 2));
        // options over the head budget are trimmed evenly to (head_max_len - 16) / n
        let big = Question { qtype: 0, head: vec![10; 50], options: vec![vec![9; 40]; 3] };
        let (_, m) = build_sequence(&big, &[], 1, 2, 1024, 64);
        assert_eq!(m.iter().zip(m.iter().skip(1)).map(|(a, b)| b - a).collect::<Vec<_>>(), vec![16, 16]);
    }
}

//! The Laya decision model on ONNX Runtime: download, prompt layout, scoring, answer cache.
use anyhow::{Context, Result};
use ort::session::{builder::GraphOptimizationLevel, Session};
use ort::value::Tensor;
use std::collections::{HashMap, HashSet};
use std::io::{IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use tokenizers::Tokenizer;

/// The checkpoints gutcheck can run. All are Apache-2.0 Laya models exported to ONNX by the community, pinned to one revision.
#[derive(Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Variant {
    /// mmBERT-base, 322M parameters, 100+ languages, the fastest (default)
    Multilingual,
    /// ModernBERT-large, 421M parameters, English, about 2x slower
    English,
    /// ModernBERT-large fine-tuned on incidents, support tickets, invoices and agent traces
    Typed,
}

struct Spec {
    dir: &'static str,
    onnx: &'static str,      // path of the graph in the remote repo
    tokenizer: &'static str, // path of tokenizer.json in the remote repo
    repo: &'static str,
    special: [&'static str; 4], // cls, sep, mask, pad
    max_len: usize,
    head_max_len: usize,
}

const ONNX_MULTI: &str = "https://huggingface.co/soyelmismo/laya-multilingual-onnx/resolve/0966c4fa58da6878b39e7e14cb5e93313b82d828";
const ONNX_EN: &str = "https://huggingface.co/mariojcr/laya-onnx/resolve/aeebb05497cc473360caf4de1653107cf08d6b10";

impl Variant {
    fn spec(self) -> Spec {
        // max_len and head_max_len come from each checkpoint's rl_agent_config.json.
        match self {
            Variant::Multilingual => Spec { dir: "laya-multilingual-0966c4f", onnx: "model-fp32.onnx", tokenizer: "tokenizer/tokenizer.json", repo: ONNX_MULTI, special: ["<bos>", "<eos>", "<mask>", "<pad>"], max_len: 1024, head_max_len: 256 },
            Variant::English => Spec { dir: "laya-english-aeebb05", onnx: "english/laya.onnx", tokenizer: "english/tokenizer.json", repo: ONNX_EN, special: ["[CLS]", "[SEP]", "[MASK]", "[PAD]"], max_len: 512, head_max_len: 192 },
            Variant::Typed => Spec { dir: "laya-typed-aeebb05", onnx: "typed-decisions/laya-typed-decisions.onnx", tokenizer: "typed-decisions/tokenizer.json", repo: ONNX_EN, special: ["[CLS]", "[SEP]", "[MASK]", "[PAD]"], max_len: 1024, head_max_len: 256 },
        }
    }

    /// Fitted calibration temperature (logits are divided by it); the multilingual checkpoint's are all 1.0.
    fn temperature(self, qtype: i64, options: usize) -> f32 {
        match (self, qtype, options) {
            (Variant::English, 2, _) => 1.9834,
            (Variant::English, _, 2) => 1.9064,
            (Variant::English, _, 3..=5) => 1.7602,
            (Variant::Typed, 2, _) => 1.0575,
            (Variant::Typed, _, _) => 1.0148,
            _ => 1.0,
        }
    }
}

// Repeated lines reuse their answer. ponytail: cleared when full, swap for an LRU if it ever thrashes.
const CACHE_MAX: usize = 50_000;
// Padded tokens per forward pass at most: long records attend quadratically, so they go in smaller batches.
const BATCH_TOKENS: usize = 8192;

/// A question the way Laya's `build_sequence` reads it.
pub struct Question {
    qtype: i64, // 0 choice, 2 noul (yes/no)
    head: Vec<u32>,
    options: Vec<Vec<u32>>, // each starts with the mask token
}

pub struct Model {
    session: Session,
    tok: Tokenizer,
    special: [u32; 4], // cls, sep, mask, pad
    variant: Variant,
    max_len: usize,
    head_max_len: usize,
    fuzzy: bool,
    cache: HashMap<(usize, String), Vec<f32>>,
    /// Records scored by the model so far; the rest were answered from the cache.
    pub calls: usize,
    /// Records per forward pass at most.
    pub batch: usize,
}

impl Model {
    /// `max_len` caps tokens per record (question included, default: the checkpoint's own); longer text is cut.
    /// `threads` = 0 lets ONNX Runtime use every core. `fuzzy` answers lines that differ only in numbers and ids once.
    pub fn load(variant: Variant, fuzzy: bool, threads: usize, max_len: Option<usize>, gpu: bool) -> Result<Self> {
        let spec = variant.spec();
        let dir = model_dir(&spec)?;
        let tok = Tokenizer::from_file(dir.join("tokenizer.json")).map_err(anyhow::Error::msg)?;
        let id = |t: &str| tok.token_to_id(t).with_context(|| format!("tokenizer has no {t}"));
        let special = [id(spec.special[0])?, id(spec.special[1])?, id(spec.special[2])?, id(spec.special[3])?];
        let path = std::env::var_os("GUTCHECK_MODEL").map(PathBuf::from).unwrap_or(dir.join("model.onnx"));
        let mut builder = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3).map_err(|e| anyhow::anyhow!("{e}"))?;
        if threads > 0 {
            builder = builder.with_intra_threads(threads).map_err(|e| anyhow::anyhow!("{e}"))?;
        }
        if gpu {
            #[cfg(target_os = "macos")]
            {
                builder = builder.with_execution_providers([ort::ep::CoreML::default().build()]).map_err(|e| anyhow::anyhow!("{e}"))?;
            }
            #[cfg(not(target_os = "macos"))]
            eprintln!("gutcheck: --gpu needs the macOS build (Core ML); running on the CPU");
        }
        Ok(Self { session: builder.commit_from_file(&path)?, tok, special, variant, max_len: max_len.unwrap_or(spec.max_len), head_max_len: spec.head_max_len, fuzzy, cache: HashMap::new(), calls: 0, batch: 8 })
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

    /// Probabilities over the question's options for each text (yes/no: `[P(no), P(yes)]`). `id` tells questions apart in
    /// the cache. Cached and repeated texts cost nothing; the rest share forward passes, batched with texts of similar length.
    pub fn probs(&mut self, id: usize, q: &Question, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let keys: Vec<String> = texts.iter().map(|t| if self.fuzzy { template(t) } else { t.to_string() }).collect();
        let [cls, sep, ..] = self.special;
        let mut todo = vec![]; // (key, ids, markers) for each text the cache does not know
        let mut seen = HashSet::new();
        for (key, text) in keys.iter().zip(texts) {
            if !self.cache.contains_key(&(id, key.clone())) && seen.insert(key) {
                let (ids, markers) = build_sequence(q, &self.enc(text)?, cls, sep, self.max_len, self.head_max_len);
                todo.push((key, ids, markers));
            }
        }
        todo.sort_by_key(|t| t.1.len()); // little padding per batch
        let mut fresh = HashMap::new();
        let mut rest = &todo[..];
        while !rest.is_empty() {
            let mut n = 1; // sorted, so the last one in a batch is the longest
            while n < rest.len().min(self.batch) && (n + 1) * rest[n].1.len() <= BATCH_TOKENS {
                n += 1;
            }
            let (batch, tail) = rest.split_at(n);
            let seqs: Vec<_> = batch.iter().map(|t| (&t.1[..], &t.2[..])).collect();
            for (t, p) in batch.iter().zip(self.forward(q, &seqs)?) {
                fresh.insert(t.0.clone(), p);
            }
            rest = tail;
        }
        let out = keys.iter().map(|k| fresh.get(k).or_else(|| self.cache.get(&(id, k.clone()))).cloned().context("lost an answer")).collect();
        if self.cache.len() + fresh.len() > CACHE_MAX {
            self.cache.clear();
        }
        self.cache.extend(fresh.into_iter().map(|(k, p)| ((id, k), p)));
        out
    }

    /// One forward pass over sequences of the same question, right-padded to the longest.
    fn forward(&mut self, q: &Question, seqs: &[(&[u32], &[usize])]) -> Result<Vec<Vec<f32>>> {
        let n = seqs.len();
        let len = seqs.iter().map(|s| s.0.len()).max().unwrap_or(0);
        let k = seqs.iter().map(|s| s.1.len()).max().unwrap_or(0);
        let pad = self.special[3] as i64;
        let (mut ids, mut mask, mut pos, mut on) = (Vec::with_capacity(n * len), Vec::with_capacity(n * len), Vec::with_capacity(n * k), Vec::with_capacity(n * k));
        for (s, m) in seqs {
            ids.extend(s.iter().map(|&t| t as i64).chain(std::iter::repeat(pad)).take(len));
            mask.extend((0..len).map(|i| (i < s.len()) as i64));
            pos.extend(m.iter().map(|&x| x as i64).chain(std::iter::repeat(0)).take(k));
            on.extend((0..k).map(|i| i < m.len()));
        }
        let outputs = self.session.run(ort::inputs! {
            "input_ids" => Tensor::from_array(([n, len], ids))?,
            "attention_mask" => Tensor::from_array(([n, len], mask))?,
            "marker_pos" => Tensor::from_array(([n, k], pos))?,
            "marker_mask" => Tensor::from_array(([n, k], on))?,
            "qtype" => Tensor::from_array(([n], vec![q.qtype; n]))?,
        })?;
        self.calls += n;
        let (shape, logits) = outputs["logits"].try_extract_tensor::<f32>()?;
        let width = shape.get(1).map_or(k, |&w| w as usize);
        Ok(seqs.iter().enumerate().map(|(r, (_, m))| {
            let t = self.variant.temperature(q.qtype, m.len());
            softmax(&logits[r * width..r * width + m.len()].iter().map(|z| z / t).collect::<Vec<_>>())
        }).collect())
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
fn model_dir(spec: &Spec) -> Result<PathBuf> {
    let cache = std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join(".cache")))
        .context("set XDG_CACHE_HOME or HOME")?;
    let dir = cache.join("gutcheck").join(spec.dir);
    for (remote, local) in [(spec.onnx, "model.onnx"), (spec.tokenizer, "tokenizer.json")] {
        let dest = dir.join(local);
        if dest.exists() {
            continue;
        }
        std::fs::create_dir_all(&dir)?;
        eprintln!("gutcheck: first run, downloading {local} to {} ...", dir.display());
        let tmp = dest.with_extension("part");
        let resp = ureq::get(&format!("{}/{remote}", spec.repo)).call()?;
        let total = resp.body().content_length().unwrap_or(0);
        let mut body = resp.into_body();
        let (mut r, mut f, mut buf, mut done, mut last_pct) = (body.as_reader(), std::fs::File::create(&tmp)?, vec![0u8; 1 << 20], 0u64, u64::MAX);
        loop {
            let n = r.read(&mut buf)?;
            if n == 0 {
                break;
            }
            f.write_all(&buf[..n])?;
            done += n as u64;
            let pct = if total > 0 { done * 100 / total } else { 0 };
            if pct != last_pct && std::io::stderr().is_terminal() {
                eprint!("\r  {pct:>3}% of {} MB", total >> 20);
            }
            last_pct = pct;
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

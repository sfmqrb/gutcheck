//! gutcheck: System-1 grep. Ask a question about every line of stdin, answered by a
//! non-autoregressive decision model (Laya multilingual, ONNX) in one forward pass per batch.
use anyhow::{bail, Context, Result};
use clap::Parser;
use ort::session::{builder::GraphOptimizationLevel, Session};
use ort::value::Tensor;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use tokenizers::Tokenizer;

// Pinned community ONNX export of convaiinnovations/laya-multilingual (Apache-2.0).
const REPO: &str = "https://huggingface.co/soyelmismo/laya-multilingual-onnx/resolve/0966c4fa58da6878b39e7e14cb5e93313b82d828";
const FILES: [(&str, &str); 2] = [("model-fp32.onnx", "model.onnx"), ("tokenizer/tokenizer.json", "tokenizer.json")];
// From the checkpoint's rl_agent_config.json. Its fitted temperatures are all 1.0, so none are applied.
// ponytail: hardcoded for this one checkpoint; read rl_agent_config.json if the model becomes swappable.
const MAX_LEN: usize = 1024;
const HEAD_MAX_LEN: usize = 256;

#[derive(Parser)]
#[command(version, about = "System-1 grep: filter, score or classify stdin lines with a natural-language question, locally.")]
struct Cli {
    /// The question, e.g. "is this a bug report?"
    question: Option<String>,
    /// Print every line prefixed with P(yes) instead of filtering
    #[arg(short, long)]
    score: bool,
    /// Comma-separated labels (optionally `label=description`); print every line prefixed with the best one
    #[arg(short, long, value_name = "A,B,C")]
    choice: Option<String>,
    /// Keep lines whose P(yes) is at least this
    #[arg(short, long, default_value_t = 0.5)]
    threshold: f32,
    /// Keep lines where the answer is no
    #[arg(short = 'v', long)]
    invert: bool,
}

/// A question the way Laya's `build_sequence` reads it.
struct Question {
    qtype: i64, // 0 choice, 1 score, 2 noul
    head: Vec<u32>,
    options: Vec<Vec<u32>>, // each starts with the mask token
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
        let mut body = ureq::get(&format!("{REPO}/{remote}")).call()?.into_body();
        std::io::copy(&mut body.as_reader(), &mut std::fs::File::create(&tmp)?)?;
        std::fs::rename(&tmp, &dest)?;
    }
    Ok(dir)
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    // `label` or `label=description`; the model reads "label: description", we print the label.
    let labels: Vec<(String, String)> = cli.choice.iter().flat_map(|c| c.split(',')).map(str::trim).filter(|s| !s.is_empty())
        .map(|s| match s.split_once('=') {
            Some((l, d)) => (l.trim().to_string(), format!("{}: {}", l.trim(), d.trim())),
            None => (s.to_string(), s.to_string()),
        }).collect();
    if cli.choice.is_some() && labels.len() < 2 {
        bail!("--choice needs at least two comma-separated labels");
    }
    let question = match (&cli.question, labels.is_empty()) {
        (Some(q), _) => q.clone(),
        (None, false) => "Which category does this text belong to?".into(),
        (None, true) => bail!("give a question, e.g. gutcheck \"is this a bug report?\""),
    };

    let dir = model_dir()?;
    let tok = Tokenizer::from_file(dir.join("tokenizer.json")).map_err(anyhow::Error::msg)?;
    let id = |t: &str| tok.token_to_id(t).with_context(|| format!("tokenizer has no {t}"));
    let (cls, sep, mask) = (id("<bos>")?, id("<eos>")?, id("<mask>")?);
    let enc = |s: &str| -> Result<Vec<u32>> {
        Ok(tok.encode(s.replace("<mask>", " "), false).map_err(anyhow::Error::msg)?.get_ids().to_vec())
    };
    let opt = |s: &str| -> Result<Vec<u32>> {
        let mut o = vec![mask];
        o.extend(enc(&format!(" {s}"))?.into_iter().take(48));
        Ok(o)
    };
    let q = if labels.is_empty() {
        Question { qtype: 2, head: enc(&format!("noul question: {question}"))?,
                   options: vec![opt("false: no, the statement does not hold")?, opt("true: yes, the statement holds")?] }
    } else {
        Question { qtype: 0, head: enc(&format!("choice question: {question}"))?,
                   options: labels.iter().map(|l| opt(&l.1)).collect::<Result<_>>()? }
    };

    let model = std::env::var_os("GUTCHECK_MODEL").map(PathBuf::from).unwrap_or(dir.join("model.onnx"));
    let mut session = Session::builder()?
        .with_optimization_level(GraphOptimizationLevel::Level3).map_err(|e| anyhow::anyhow!("{e}"))?
        .commit_from_file(&model)?;

    // One line per forward pass: batching measured no faster on CPU (compute-bound), and this streams `tail -f`.
    let mut out = std::io::stdout().lock();
    for line in std::io::stdin().lock().lines() {
        let line = line?;
        let (ids, markers) = build_sequence(&q, &enc(&line)?, cls, sep, MAX_LEN, HEAD_MAX_LEN);
        let (len, k) = (ids.len(), markers.len());
        let outputs = session.run(ort::inputs! {
            "input_ids" => Tensor::from_array(([1, len], ids.iter().map(|&t| t as i64).collect::<Vec<_>>()))?,
            "attention_mask" => Tensor::from_array(([1, len], vec![1i64; len]))?,
            "marker_pos" => Tensor::from_array(([1, k], markers.iter().map(|&m| m as i64).collect::<Vec<_>>()))?,
            "marker_mask" => Tensor::from_array(([1, k], vec![true; k]))?,
            "qtype" => Tensor::from_array(([1], vec![q.qtype]))?,
        })?;
        let p = softmax(&outputs["logits"].try_extract_tensor::<f32>()?.1[..k]);
        let r = if !labels.is_empty() {
            let best = (0..k).max_by(|&a, &b| p[a].total_cmp(&p[b])).unwrap();
            writeln!(out, "{}\t{line}", labels[best].0)
        } else if cli.score {
            writeln!(out, "{:.2}\t{line}", p[1])
        } else if (p[1] >= cli.threshold) != cli.invert {
            writeln!(out, "{line}")
        } else { Ok(()) };
        if r.and_then(|_| out.flush()).is_err() { return Ok(()); } // downstream closed (e.g. `| head`)
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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

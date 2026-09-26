//! Turning bytes into records: lines, paragraphs, whole files, JSON fields, diff hunks.
use anyhow::Result;
use std::io::BufRead;

/// One thing to judge. `judge` goes to the model, `show` is what gets printed.
#[derive(Clone)]
pub struct Record {
    pub file: String,
    pub line: usize,
    pub judge: String,
    pub show: String,
}

pub enum Mode {
    /// Each line; the number is how many neighbouring lines either side the model also sees.
    Lines(usize),
    Para,
    Whole,
    Field(String),
    Csv(String),
    Diff,
}

/// Calls `f` for every record; `f` returns false to stop reading this input.
pub fn read(r: impl BufRead, file: &str, mode: &Mode, f: &mut dyn FnMut(Record) -> Result<bool>) -> Result<()> {
    let mut rec = |line: usize, judge: String, show: String| f(Record { file: file.to_string(), line, judge, show });
    let lines = r.split(b'\n').map(|l| l.map(|b| String::from_utf8_lossy(&b).trim_end_matches('\r').to_string()));
    match mode {
        Mode::Lines(0) => {
            for (i, l) in lines.enumerate() {
                let l = l?;
                if !rec(i + 1, l.clone(), l)? {
                    break;
                }
            }
        }
        Mode::Lines(n) => {
            // Needs the whole input to look ahead, so it does not stream.
            let all = lines.collect::<std::io::Result<Vec<_>>>()?;
            for (i, l) in all.iter().enumerate() {
                let window = all[i.saturating_sub(*n)..(i + n + 1).min(all.len())].join("\n");
                if !rec(i + 1, window, l.clone())? {
                    break;
                }
            }
        }
        Mode::Para => {
            let (mut buf, mut start) = (Vec::new(), 1);
            for (i, l) in lines.enumerate() {
                let l = l?;
                if l.trim().is_empty() {
                    if !buf.is_empty() {
                        let text = buf.join("\n");
                        buf.clear();
                        if !rec(start, text.clone(), text)? {
                            return Ok(());
                        }
                    }
                } else {
                    if buf.is_empty() {
                        start = i + 1;
                    }
                    buf.push(l);
                }
            }
            if !buf.is_empty() {
                let text = buf.join("\n");
                rec(start, text.clone(), text)?;
            }
        }
        Mode::Whole => {
            let text = lines.collect::<std::io::Result<Vec<_>>>()?.join("\n");
            rec(1, text.clone(), text)?;
        }
        Mode::Field(name) => {
            for (i, l) in lines.enumerate() {
                let l = l?;
                if l.trim().is_empty() {
                    continue;
                }
                let Ok(v) = serde_json::from_str::<serde_json::Value>(&l) else {
                    eprintln!("gutcheck: {file}:{}: not JSON, skipped", i + 1);
                    continue;
                };
                let Some(field) = name.split('.').try_fold(&v, |v, k| v.get(k)) else { continue };
                let judge = field.as_str().map_or_else(|| field.to_string(), str::to_string);
                if !rec(i + 1, judge, l)? {
                    break;
                }
            }
        }
        Mode::Csv(name) => {
            let bytes = lines_to_bytes(lines)?;
            let mut rd = csv::ReaderBuilder::new().flexible(true).from_reader(bytes.as_slice());
            let Some(col) = rd.headers()?.iter().position(|h| h == name) else { anyhow::bail!("{file}: no column named {name}") };
            for (i, row) in rd.records().enumerate() {
                let row = row?;
                let Some(value) = row.get(col) else { continue };
                let mut w = csv::Writer::from_writer(vec![]);
                w.write_record(&row)?;
                let show = String::from_utf8_lossy(&w.into_inner()?).trim_end().to_string();
                if !rec(i + 2, value.to_string(), show)? {
                    break;
                }
            }
        }
        Mode::Diff => {
            // A hunk runs from its `@@` line while lines start with ' ', '+', '-' or '\'. Works on `git diff` and `git log -p`.
            let (mut path, mut hunk): (String, Option<(usize, String)>) = (file.to_string(), None);
            let mut emit = |path: &str, hunk: Option<(usize, String)>| -> Result<bool> {
                match hunk {
                    Some((line, text)) => f(Record { file: path.to_string(), line, judge: text.clone(), show: text }),
                    None => Ok(true),
                }
            };
            for (i, l) in lines.enumerate() {
                let l = l?;
                if l.starts_with("@@") {
                    if !emit(&path, hunk.take())? {
                        return Ok(());
                    }
                    hunk = Some((i + 1, l));
                } else if let Some(p) = l.strip_prefix("+++ ") {
                    if !emit(&path, hunk.take())? {
                        return Ok(());
                    }
                    path = p.strip_prefix("b/").unwrap_or(p).to_string();
                } else if l.starts_with("diff ") {
                    if !emit(&path, hunk.take())? {
                        return Ok(());
                    }
                } else if let Some((_, text)) = hunk.as_mut().filter(|_| l.starts_with([' ', '+', '-', '\\'])) {
                    text.push('\n');
                    text.push_str(&l);
                } else if !emit(&path, hunk.take())? {
                    return Ok(());
                }
            }
            emit(&path, hunk.take())?;
        }
    }
    Ok(())
}

fn lines_to_bytes(lines: impl Iterator<Item = std::io::Result<String>>) -> Result<Vec<u8>> {
    Ok(lines.collect::<std::io::Result<Vec<_>>>()?.join("\n").into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect(input: &str, mode: Mode) -> Vec<(String, usize, String)> {
        let mut out = vec![];
        read(input.as_bytes(), "f", &mode, &mut |r| { out.push((r.file, r.line, r.judge)); Ok(true) }).unwrap();
        out
    }

    #[test]
    fn modes() {
        assert_eq!(collect("a\nb\n", Mode::Lines(0)).len(), 2);
        assert_eq!(collect("a\nb\nc\n", Mode::Lines(1))[1].2, "a\nb\nc");
        assert_eq!(collect("id,msg\n1,\"hi, you\"\n2,bye\n", Mode::Csv("msg".into())).iter().map(|r| (r.1, r.2.clone())).collect::<Vec<_>>(), vec![(2, "hi, you".to_string()), (3, "bye".to_string())]);
        assert_eq!(collect("a\nb\n\nc\n", Mode::Para), vec![("f".into(), 1, "a\nb".into()), ("f".into(), 4, "c".into())]);
        assert_eq!(collect("{\"m\":\"x\",\"n\":{\"k\":3}}\nnope\n{\"m\":\"y\"}\n", Mode::Field("m".into())).iter().map(|r| r.2.clone()).collect::<Vec<_>>(), vec!["x", "y"]);
        assert_eq!(collect("{\"n\":{\"k\":3}}\n", Mode::Field("n.k".into()))[0].2, "3");
        let diff = "diff --git a/x.rs b/x.rs\n--- a/x.rs\n+++ b/x.rs\n@@ -1 +1 @@\n-old\n+new\n@@ -9 +9 @@\n ctx\n";
        let hunks = collect(diff, Mode::Diff);
        assert_eq!((hunks.len(), hunks[0].0.as_str(), hunks[0].1, hunks[1].1), (2, "x.rs", 4, 7));
        assert_eq!(hunks[0].2, "@@ -1 +1 @@\n-old\n+new");
    }
}

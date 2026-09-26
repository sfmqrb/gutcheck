//! gutcheck: grep for meaning. Ask a question about every line, paragraph, file or diff hunk,
//! answered locally by a non-autoregressive decision model (Laya) in one forward pass.
mod input;
mod model;
mod output;
mod packs;
mod tui;

use anyhow::{bail, Context, Result};
use clap::{CommandFactory, Parser};
use ignore::{overrides::OverrideBuilder, WalkBuilder};
use input::{Mode, Record};
use model::{template, Model, Question, Variant};
use output::{Printer, Shown};
use std::collections::HashSet;
use std::io::{BufRead, BufReader, IsTerminal};
use std::time::Instant;
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about = "grep for meaning: filter, score or classify text with a plain-English question, locally on CPU.",
    override_usage = "gutcheck [OPTIONS] <QUESTION> [PATH]...\n       gutcheck [OPTIONS] -e <QUESTION>... [PATH]...")]
struct Cli {
    /// The question, then files to read (stdin if none). With -e, every argument is a path.
    #[arg(value_name = "QUESTION|PATH")]
    args: Vec<String>,

    /// Another question; a line matches if any fits (all with --all)
    #[arg(short = 'e', long = "question", value_name = "QUESTION")]
    more: Vec<String>,
    /// With several questions, require every one to fit
    #[arg(long)]
    all: bool,
    /// Match when P(yes) is at least this
    #[arg(short = 'p', long, visible_short_alias = 't', visible_alias = "threshold", default_value_t = 0.5, value_name = "P")]
    min_prob: f32,
    /// Keep the lines that do NOT match
    #[arg(short = 'v', long)]
    invert: bool,

    /// Print every record with its P(yes) instead of filtering
    #[arg(short, long)]
    score: bool,
    /// Print every record with the best of these labels, `label` or `label=description`
    #[arg(short, long, value_name = "A,B,C")]
    choice: Option<String>,
    /// Find the cut-off yourself: split the scores into a yes group and a no group (reads everything first)
    #[arg(long, conflicts_with_all = ["score", "rank", "top", "choice"])]
    auto: bool,
    /// Print every record ranked by P(yes), best first
    #[arg(long)]
    rank: bool,
    /// Like --rank, but only the best N
    #[arg(long, value_name = "N")]
    top: Option<usize>,
    /// With -c: a histogram of the labels instead of every record
    #[arg(long, requires = "choice")]
    tally: bool,
    /// One JSON object per printed record
    #[arg(long)]
    json: bool,

    /// Judge paragraphs (blank-line separated) instead of lines
    #[arg(long, conflicts_with_all = ["whole", "diff", "field"])]
    para: bool,
    /// Judge each whole file
    #[arg(long, conflicts_with_all = ["diff", "field"])]
    whole: bool,
    /// Judge each hunk of a unified diff (git diff, git log -p)
    #[arg(long, conflicts_with = "field")]
    diff: bool,
    /// Judge one field of each JSON line (`a.b` for nested); prints the whole line
    #[arg(long, value_name = "NAME")]
    field: Option<String>,
    /// With --field: the input is CSV, and NAME is a column
    #[arg(long, requires = "field")]
    csv: bool,
    /// Also show the model this many lines either side of each line (reads the whole input first)
    #[arg(short = 'C', long, value_name = "N", default_value_t = 0)]
    context: usize,

    /// Search directories recursively (respects .gitignore)
    #[arg(short, long)]
    recursive: bool,
    /// With -r, only files matching this glob; `!glob` excludes. Repeatable
    #[arg(long, value_name = "GLOB")]
    glob: Vec<String>,
    /// With -r, do not read .gitignore
    #[arg(long)]
    no_ignore: bool,

    /// Prefix each match with its line number
    #[arg(short = 'n', long)]
    line_number: bool,
    /// Prefix each match with its file name (default with several files)
    #[arg(short = 'H', long)]
    with_filename: bool,
    /// Print only the number of matches per file
    #[arg(long)]
    count: bool,
    /// Print only the names of files with a match, stopping each at its first match
    #[arg(short = 'l', long)]
    files_with_matches: bool,
    /// Print nothing; stop at the first match and exit 0
    #[arg(short, long)]
    quiet: bool,
    /// Stop each file after this many matches
    #[arg(short = 'm', long, value_name = "NUM")]
    max_count: Option<usize>,

    /// Treat lines that differ only in numbers, timestamps and ids as one line (one model call per shape; big speedup on logs)
    #[arg(short, long)]
    fuzzy: bool,
    /// List the built-in and your own named questions (@name) and exit
    #[arg(long)]
    questions: bool,
    /// Print a shell completion script and exit
    #[arg(long, value_name = "SHELL")]
    completions: Option<clap_complete::Shell>,
    /// Print the man page and exit
    #[arg(long)]
    man: bool,
    /// Explore the scores in a live terminal view: move the threshold, sort, re-ask
    #[arg(short = 'i', long, conflicts_with_all = ["choice", "count", "files_with_matches", "quiet", "json"])]
    interactive: bool,
    /// Read everything and report how many model calls it would take, without loading the model
    #[arg(long)]
    estimate: bool,
    /// Cap on tokens per record, question included; longer text is cut (default: the model's own limit)
    #[arg(long, value_name = "N")]
    max_tokens: Option<usize>,
    /// Which Laya checkpoint to run (default: multilingual, or typed when you use a named question)
    #[arg(short = 'M', long, value_enum)]
    model: Option<Variant>,
    /// Run on the Apple GPU and Neural Engine through Core ML (macOS)
    #[arg(long)]
    gpu: bool,
    /// CPU threads for the model (default: all)
    #[arg(long, default_value_t = 0, value_name = "N")]
    threads: usize,
}

enum Src {
    Stdin,
    File(PathBuf),
}

fn main() {
    std::process::exit(match run() {
        Ok(code) => code,
        Err(e) => {
            if e.chain().any(|c| c.downcast_ref::<std::io::Error>().is_some_and(|io| io.kind() == std::io::ErrorKind::BrokenPipe)) {
                std::process::exit(0); // downstream closed, e.g. `| head`
            }
            eprintln!("gutcheck: {e:#}");
            2
        }
    });
}

fn run() -> Result<i32> {
    let cli = Cli::parse();
    if cli.questions {
        for (name, q) in packs::all() {
            println!("@{name:<11} {q}");
        }
        return Ok(0);
    }
    if let Some(shell) = cli.completions {
        clap_complete::generate(shell, &mut Cli::command(), "gutcheck", &mut std::io::stdout());
        return Ok(0);
    }
    if cli.man {
        clap_mangen::Man::new(Cli::command()).render(&mut std::io::stdout())?;
        return Ok(0);
    }
    let (questions, paths) = if cli.more.is_empty() {
        match cli.args.split_first() {
            Some((q, p)) => (vec![q.clone()], p.to_vec()),
            None => (vec![], vec![]),
        }
    } else {
        (cli.more.clone(), cli.args.clone())
    };
    // `label` or `label=description`; the model reads "label: description", we print the label.
    let labels: Vec<(String, String)> = cli.choice.iter().flat_map(|c| c.split(',')).map(str::trim).filter(|s| !s.is_empty())
        .map(|s| match s.split_once('=') {
            Some((l, d)) => (l.trim().to_string(), format!("{}: {}", l.trim(), d.trim())),
            None => (s.to_string(), s.to_string()),
        }).collect();
    if cli.choice.is_some() && labels.len() < 2 {
        bail!("--choice needs at least two comma-separated labels");
    }
    let used_pack = questions.iter().any(|q| q.starts_with('@'));
    let questions = questions.iter().map(|q| packs::resolve(q)).collect::<Result<Vec<_>>>()?;
    // Named questions were tuned on the typed checkpoint, which is far more accurate on English than the multilingual one.
    let variant = cli.model.unwrap_or(if used_pack { Variant::Typed } else { Variant::Multilingual });
    let questions = match (questions.is_empty(), labels.is_empty()) {
        (false, _) => questions,
        (true, false) => vec!["Which category does this text belong to?".into()],
        (true, true) => bail!("give a question, e.g. gutcheck \"is this a bug report?\" app.log"),
    };
    let mode = match (&cli.field, cli.para, cli.whole, cli.diff) {
        (Some(f), ..) if cli.csv => Mode::Csv(f.clone()),
        (Some(f), ..) => Mode::Field(f.clone()),
        (_, true, ..) => Mode::Para,
        (_, _, true, _) => Mode::Whole,
        (_, _, _, true) => Mode::Diff,
        _ => Mode::Lines(cli.context),
    };
    let sources = sources(&cli, &paths)?;

    if cli.estimate {
        return estimate(&sources, &mode, questions.len(), if variant == Variant::Multilingual { 10.0 } else { 4.0 });
    }

    let started = Instant::now();
    let tty = std::io::stderr().is_terminal() && !cli.quiet;
    if tty {
        eprint!("\x1b[2m  loading model...\x1b[0m");
    }
    let mut model = Model::load(variant, cli.fuzzy, cli.threads, cli.max_tokens, cli.gpu)?;
    if tty {
        eprint!("\r\x1b[K");
    }
    if cli.interactive {
        let mut records = vec![];
        for src in &sources {
            let Some((name, reader)) = open(src)? else { continue };
            input::read(reader, &name, &mode, &mut |r| { records.push(r); Ok(true) })?;
        }
        let picked = tui::run(model, records, questions.clone(), cli.all, cli.min_prob)?;
        let mut printer = Printer::new();
        printer.with_file = cli.with_filename || sources.len() > 1 || matches!(mode, Mode::Diff);
        printer.line_number = cli.line_number;
        let picked = picked.unwrap_or_default();
        for rec in &picked {
            if printer.record(rec, Shown::Plain).is_err() {
                break;
            }
        }
        return Ok(if picked.is_empty() { 1 } else { 0 });
    }
    let qs: Vec<Question> = if labels.is_empty() {
        questions.iter().map(|q| model.yes_no(q)).collect::<Result<_>>()?
    } else {
        model.choice(&questions[0], &labels.iter().map(|l| l.1.clone()).collect::<Vec<_>>()).map(|q| vec![q])?
    };
    let mut printer = Printer::new();
    printer.json = cli.json;
    printer.with_file = cli.with_filename || sources.len() > 1 || matches!(mode, Mode::Diff); // diff hunks carry their own file names
    printer.line_number = cli.line_number;

    let rank = cli.rank || cli.top.is_some();
    let auto = cli.auto;
    let filtering = labels.is_empty() && !cli.score && !rank && !auto;
    let mut ranked: Vec<(f32, Record)> = vec![];
    let mut tally = vec![0usize; labels.len()];
    let (mut judged, mut printed_total) = (0usize, 0usize);
    let (mut any_match, mut had_error, mut stop) = (false, false, false);
    for src in &sources {
        if stop {
            break;
        }
        let (name, reader) = match open(src) {
            Ok(Some(x)) => x,
            Ok(None) => continue, // binary
            Err(e) => {
                eprintln!("gutcheck: {e:#}");
                had_error = true;
                continue;
            }
        };
        let mut in_file = 0usize;
        let mut handle = |rec: Record| -> Result<bool> {
            if rec.judge.trim().is_empty() {
                return Ok(true); // nothing to judge, no model call
            }
            judged += 1;
            let shown_label;
            let (hit, shown) = if !labels.is_empty() {
                let p = model.probs(0, &qs[0], &rec.judge)?;
                let best = (0..p.len()).max_by(|&a, &b| p[a].total_cmp(&p[b])).unwrap();
                shown_label = labels[best].0.clone();
                tally[best] += 1;
                (true, Shown::Label(best, &shown_label))
            } else {
                let mut ps = Vec::with_capacity(qs.len());
                for (i, q) in qs.iter().enumerate() {
                    ps.push(model.probs(i, q, &rec.judge)?[1]);
                }
                let p = if cli.all { ps.iter().cloned().fold(f32::MAX, f32::min) } else { ps.iter().cloned().fold(f32::MIN, f32::max) };
                if cli.score || rank || auto { (true, Shown::Score(p)) } else { ((p >= cli.min_prob) != cli.invert, Shown::Plain) }
            };
            if !hit {
                return Ok(true);
            }
            in_file += 1;
            any_match |= filtering;
            printed_total += 1;
            if rank || auto {
                if let Shown::Score(p) = shown {
                    ranked.push((p, rec));
                }
                return Ok(true);
            }
            let printed = if cli.quiet {
                Ok(())
            } else if cli.files_with_matches {
                printer.line(&name, true)
            } else if cli.count || cli.tally {
                Ok(())
            } else {
                printer.record(&rec, shown)
            };
            if let Err(e) = printed {
                if e.kind() == std::io::ErrorKind::BrokenPipe {
                    stop = true; // downstream closed (e.g. `| head`)
                    return Ok(false);
                }
                return Err(e.into());
            }
            if cli.quiet {
                stop = true;
            }
            Ok(!(cli.quiet || cli.files_with_matches || cli.max_count.is_some_and(|m| in_file >= m)))
        };
        input::read(reader, &name, &mode, &mut handle)?;
        if cli.count && !stop {
            let line = if printer.with_file { format!("{name}:{in_file}") } else { in_file.to_string() };
            let _ = printer.line(&line, false);
        }
    }
    let mut cutoff_note = String::new();
    if auto {
        let cut = auto_cutoff(&ranked.iter().map(|r| r.0).collect::<Vec<_>>());
        cutoff_note = format!(" · cut-off {cut:.2} (auto)");
        for (p, rec) in &ranked {
            if (*p >= cut) != cli.invert {
                any_match = true;
                if printer.record(rec, Shown::Plain).is_err() {
                    break;
                }
            }
        }
    }
    if rank {
        ranked.sort_by(|a, b| b.0.total_cmp(&a.0));
        for (p, rec) in ranked.iter().take(cli.top.unwrap_or(usize::MAX)) {
            if printer.record(rec, Shown::Score(*p)).is_err() {
                break;
            }
        }
    }
    if cli.tally {
        let rows: Vec<(String, usize)> = labels.iter().map(|l| l.0.clone()).zip(tally).collect();
        let _ = printer.tally(&rows);
    }
    if tty && !cli.json {
        let asked = judged * qs.len();
        let reused = if asked > model.calls { format!(" ({} reused)", asked - model.calls) } else { String::new() };
        let head = if filtering { format!("{printed_total} of {judged} matched") } else { format!("{judged} records") };
        eprintln!("\x1b[2m  {head}{cutoff_note} · {} model calls{reused} · {:.1} s\x1b[0m", model.calls, started.elapsed().as_secs_f32());
    }
    // Like grep: 0 if something matched, 1 if not, 2 on error. Score and label modes always print, so they exit 0.
    Ok(if had_error { 2 } else if (filtering || auto) && !any_match { 1 } else { 0 })
}

/// Otsu's method on the log-odds of the scores: the cut that best separates a "yes" group from a "no" group.
/// Falls back to 0.5 when there is nothing to separate.
fn auto_cutoff(scores: &[f32]) -> f32 {
    let mut z: Vec<f32> = scores.iter().map(|p| { let p = p.clamp(1e-4, 1.0 - 1e-4); (p / (1.0 - p)).ln() }).collect();
    z.sort_by(|a, b| a.total_cmp(b));
    let n = z.len();
    if n < 4 || z[n - 1] - z[0] < 1.0 {
        return 0.5;
    }
    let total: f32 = z.iter().sum();
    let (mut below, mut best, mut cut) = (0.0f32, -1.0f32, 0.0f32);
    for k in 1..n {
        below += z[k - 1];
        let (n0, n1) = (k as f32, (n - k) as f32);
        let between = n0 * n1 * (below / n0 - (total - below) / n1).powi(2);
        if between > best {
            (best, cut) = (between, (z[k - 1] + z[k]) / 2.0);
        }
    }
    1.0 / (1.0 + (-cut).exp())
}

/// stdin, the files named, or (with -r) every file under the directories named.
fn sources(cli: &Cli, paths: &[String]) -> Result<Vec<Src>> {
    if paths.is_empty() && !cli.recursive {
        return Ok(vec![Src::Stdin]);
    }
    let mut out = vec![];
    for p in if paths.is_empty() { vec![".".to_string()] } else { paths.to_vec() } {
        let path = PathBuf::from(&p);
        if p == "-" {
            out.push(Src::Stdin);
        } else if path.is_dir() {
            if !cli.recursive {
                bail!("{p}: is a directory (use -r)");
            }
            let mut walk = WalkBuilder::new(&path);
            walk.git_ignore(!cli.no_ignore).ignore(!cli.no_ignore).sort_by_file_path(|a, b| a.cmp(b));
            if !cli.glob.is_empty() {
                let mut globs = OverrideBuilder::new(&path);
                for g in &cli.glob {
                    globs.add(g)?;
                }
                walk.overrides(globs.build()?);
            }
            for entry in walk.build() {
                let entry = entry?;
                if entry.file_type().is_some_and(|t| t.is_file()) {
                    out.push(Src::File(entry.into_path()));
                }
            }
        } else {
            out.push(Src::File(path));
        }
    }
    Ok(out)
}

/// A named reader, or None for a binary file (skipped, like ripgrep).
fn open(src: &Src) -> Result<Option<(String, Box<dyn BufRead>)>> {
    Ok(match src {
        Src::Stdin => Some(("-".into(), Box::new(std::io::stdin().lock()))),
        Src::File(p) => {
            let name = p.display().to_string();
            let mut r = BufReader::new(std::fs::File::open(p).with_context(|| name.clone())?);
            if r.fill_buf()?.iter().take(8192).any(|&b| b == 0) {
                return Ok(None);
            }
            Some((name, Box::new(r)))
        }
    })
}

/// How many model calls the run would take, at the measured calls per second on a laptop CPU.
fn estimate(sources: &[Src], mode: &Mode, questions: usize, calls_per_sec: f64) -> Result<i32> {
    let (mut total, mut distinct, mut shapes) = (0usize, HashSet::new(), HashSet::new());
    for src in sources {
        let Some((name, reader)) = open(src)? else { continue };
        input::read(reader, &name, mode, &mut |rec| {
            total += 1;
            shapes.insert(template(&rec.judge));
            distinct.insert(rec.judge);
            Ok(true)
        })?;
    }
    let secs = |n: usize| format!("{:.0} s", (n * questions) as f64 / calls_per_sec);
    println!("records         {total}");
    println!("distinct        {:<8} about {}", distinct.len(), secs(distinct.len()));
    println!("with --fuzzy    {:<8} about {}", shapes.len(), secs(shapes.len()));
    Ok(0)
}

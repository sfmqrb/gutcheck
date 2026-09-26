//! Printing results: plain, scored, labelled or JSON; colored only on a terminal.
use crate::input::Record;
use serde_json::json;
use std::io::{self, IsTerminal, StdoutLock, Write};

const LABEL_COLORS: [&str; 6] = ["1;36", "1;33", "1;31", "1;32", "1;35", "1;34"];

pub enum Shown<'a> {
    Plain,
    Score(f32),
    Label(usize, &'a str),
}

pub struct Printer {
    out: StdoutLock<'static>,
    color: bool,
    pub json: bool,
    pub with_file: bool,
    pub line_number: bool,
}

impl Printer {
    /// Colors only on a terminal (or CLICOLOR_FORCE=1), never when piped; NO_COLOR turns them off.
    pub fn new() -> Self {
        let color = std::env::var_os("NO_COLOR").is_none() && (io::stdout().is_terminal() || std::env::var_os("CLICOLOR_FORCE").is_some());
        Self { out: io::stdout().lock(), color, json: false, with_file: false, line_number: false }
    }

    fn paint(&self, code: &str, s: &str) -> String {
        if self.color { format!("\x1b[{code}m{s}\x1b[0m") } else { s.to_string() }
    }

    pub fn record(&mut self, rec: &Record, shown: Shown) -> io::Result<()> {
        if self.json {
            let v = match shown {
                Shown::Plain => json!({ "file": rec.file, "line": rec.line, "text": rec.show }),
                Shown::Score(p) => json!({ "file": rec.file, "line": rec.line, "p": p, "text": rec.show }),
                Shown::Label(_, l) => json!({ "file": rec.file, "line": rec.line, "label": l, "text": rec.show }),
            };
            return writeln!(self.out, "{v}");
        }
        let mut prefix = String::new();
        if self.with_file {
            prefix += &self.paint("1;35", &rec.file);
            prefix.push(':');
        }
        if self.line_number {
            prefix += &self.paint("32", &rec.line.to_string());
            prefix.push(':');
        }
        let (lead, text) = match shown {
            Shown::Plain => (String::new(), self.paint("32", &rec.show)),
            Shown::Score(p) => {
                let heat = if p >= 0.75 { "1;31" } else if p >= 0.5 { "1;33" } else { "2" };
                (format!("{}\t", self.paint(heat, &format!("{p:.2}"))), rec.show.clone())
            }
            Shown::Label(i, l) => (format!("{}\t", self.paint(LABEL_COLORS[i % LABEL_COLORS.len()], l)), rec.show.clone()),
        };
        writeln!(self.out, "{lead}{prefix}{text}")
    }

    /// A bare line: a file name for `-l`, a number for `--count`.
    pub fn line(&mut self, s: &str, is_file: bool) -> io::Result<()> {
        let s = if is_file { self.paint("1;35", s) } else { s.to_string() };
        writeln!(self.out, "{s}")
    }
}

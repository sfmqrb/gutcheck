//! Named questions: `gutcheck @secret -r src/`. Built-ins plus your own in `~/.config/gutcheck/questions`.
use anyhow::{bail, Result};
use std::path::PathBuf;

const BUILTIN: [(&str, &str); 12] = [
    ("error", "does this describe an error or a failure?"),
    ("security", "does this describe a security problem, an attack or suspicious activity?"),
    ("secret", "does this contain a password, an API key, a token or another credential?"),
    ("pii", "does this contain personal information such as a name, an email address, a phone number or a home address?"),
    ("bug", "is this a bug report?"),
    ("feature", "is this a feature request?"),
    ("spam", "is this spam or an unsolicited advertisement?"),
    ("frustrated", "is the writer frustrated or angry?"),
    ("urgent", "does this need urgent attention?"),
    ("toxic", "is this rude, hateful or abusive?"),
    ("todo", "does this comment say that something still needs to be done or fixed?"),
    ("question", "is this asking a question?"),
];

fn user_file() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("gutcheck").join("questions"))
}

/// Your own questions, one `name = question` per line (`#` starts a comment). They win over built-ins.
fn user() -> Vec<(String, String)> {
    let text = user_file().and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default();
    parse(&text)
}

fn parse(text: &str) -> Vec<(String, String)> {
    text.lines().filter_map(|l| {
        let l = l.trim();
        let (name, q) = l.split_once('=')?;
        (!l.starts_with('#') && !name.trim().is_empty() && !q.trim().is_empty()).then(|| (name.trim().trim_start_matches('@').to_string(), q.trim().to_string()))
    }).collect()
}

/// `@name` becomes its question; anything else is returned as it is.
pub fn resolve(q: &str) -> Result<String> {
    let Some(name) = q.strip_prefix('@') else { return Ok(q.to_string()) };
    match all().into_iter().find(|(n, _)| n == name) {
        Some((_, q)) => Ok(q),
        None => bail!("no question named @{name} (see `gutcheck --questions`)"),
    }
}

/// Built-ins, then yours (a same-named one of yours replaces the built-in).
pub fn all() -> Vec<(String, String)> {
    let mine = user();
    let mut out: Vec<(String, String)> = BUILTIN.iter().filter(|(n, _)| !mine.iter().any(|(m, _)| m == n)).map(|(n, q)| (n.to_string(), q.to_string())).collect();
    out.extend(mine);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_resolves() {
        assert_eq!(parse("# c\nslow = is this slow?\n@bad = x = y\n\nnope\n"), vec![("slow".to_string(), "is this slow?".to_string()), ("bad".to_string(), "x = y".to_string())]);
        assert_eq!(resolve("plain question?").unwrap(), "plain question?");
        assert!(resolve("@secret").unwrap().contains("credential"));
        assert!(resolve("@nope-not-a-pack").is_err());
    }
}

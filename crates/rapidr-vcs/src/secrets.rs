//! The secret scan before a version is saved (docs/versions-plan.md §5:
//! "a common beginner mistake, so we catch it").
//!
//! The added lines of the changed text files are checked against
//! RapidR's own list of patterns below, written for RapidR from the
//! token formats their issuers publish (no list is copied from another
//! scanner). A match is a warning with the file, the line and what it
//! looks like; the line is shown masked (the secret's first characters
//! and "…"), so the warning itself never spreads the secret.

use std::sync::OnceLock;

use regex::Regex;

/// Files bigger than this aren't scanned (data, not code).
pub(crate) const MAX_SCANNED: usize = 2 * 1024 * 1024;

/// A line that looks like a secret.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SecretWarning {
    /// The file (relative to the project, `/` separated).
    pub path: String,
    /// The line, from 1.
    pub line: usize,
    /// What it looks like, in words: "a private key", "an API key (sk-…)".
    pub what: String,
    /// The line with the secret masked.
    pub masked: String,
}

/// The patterns: what each looks like, and the regular expression (its
/// group 1, when it has one, is the secret itself).
fn patterns() -> &'static [(&'static str, Regex)] {
    static P: OnceLock<Vec<(&'static str, Regex)>> = OnceLock::new();
    P.get_or_init(|| {
        let list: [(&str, &str); 9] = [
            ("a private key", r"-----BEGIN [A-Z0-9 ]*PRIVATE KEY(?: BLOCK)?-----"),
            ("an access key ID (AKIA\u{2026})", r"\b((?:AKIA|ASIA)[0-9A-Z]{16})\b"),
            ("a GitHub token", r"\b((?:ghp|gho|ghu|ghs|ghr)_[A-Za-z0-9]{36,}|github_pat_[A-Za-z0-9_]{40,})"),
            ("an access token (glpat-\u{2026})", r"\b(glpat-[A-Za-z0-9_\-]{20,})"),
            ("an API key (sk-\u{2026})", r"\b(sk-[A-Za-z0-9_\-]{20,})"),
            ("a live payment key", r"\b((?:sk|rk)_live_[0-9A-Za-z]{16,})"),
            ("a chat app token (xox\u{2026})", r"\b(xox[abposr]-[A-Za-z0-9\-]{10,})"),
            ("an API key (AIza\u{2026})", r"\b(AIza[0-9A-Za-z_\-]{35})"),
            ("a password or key written in the code", r#"(?i)\b(?:password|passwd|pwd|secret|client[_-]?secret|api[_-]?key|apikey|access[_-]?token|auth[_-]?token|token)\$?\s*[:=]\s*["']([^"'\s]{8,})["']"#),
        ];
        list.iter().map(|(what, re)| (*what, Regex::new(re).expect("a valid pattern"))).collect()
    })
}

/// Values that are obviously not real secrets (examples, placeholders).
fn placeholder(value: &str) -> bool {
    let v = value.to_ascii_lowercase();
    let first = v.chars().next().unwrap_or(' ');
    v.chars().all(|c| c == first)
        || ["your", "example", "xxxx", "****", "placeholder", "changeme", "dummy", "sample", "<", "{", "$", "%"].iter().any(|p| v.contains(p))
}

/// The warnings for one line.
fn scan_line(path: &str, n: usize, line: &str, out: &mut Vec<SecretWarning>) {
    for (what, re) in patterns() {
        let Some(m) = re.captures(line) else { continue };
        let secret = m.get(1).unwrap_or_else(|| m.get(0).expect("a match"));
        if m.get(1).is_some() && what.starts_with("a password") && placeholder(secret.as_str()) {
            continue;
        }
        let keep: String = secret.as_str().chars().take(4).collect();
        let mut masked = format!("{}{keep}\u{2026}{}", &line[..secret.start()], &line[secret.end()..]);
        if secret.as_str().starts_with("-----BEGIN") {
            masked = line.to_string();
        }
        let masked = masked.trim().chars().take(120).collect();
        out.push(SecretWarning { path: path.to_string(), line: n, what: what.to_string(), masked });
        // (one warning per line is enough)
        break;
    }
}

/// Every line of `text` that looks like a secret (a whole file: RAI can
/// check what it is about to write).
pub fn scan_text(path: &str, text: &str) -> Vec<SecretWarning> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        scan_line(path, i + 1, line, &mut out);
    }
    out
}

/// The lines of `new` that aren't in `old` (RapidR's diff) and look like
/// secrets.
pub(crate) fn scan_added(path: &str, old: &[u8], new: &[u8]) -> Vec<SecretWarning> {
    let (a, b) = (rapidr_diff::text::split_lines(old), rapidr_diff::text::split_lines(new));
    let mut out = Vec::new();
    for ch in rapidr_diff::text::changes(&a, &b) {
        for j in ch.new {
            let line = String::from_utf8_lossy(b[j]);
            scan_line(path, j + 1, line.trim_end_matches(['\n', '\r']), &mut out);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_and_masks() {
        let text = "PRINT 1\nKey$ = \"sk-proj-abcdefghijklmnopqrstuv\"\n-----BEGIN OPENSSH PRIVATE KEY-----\nCONST API_KEY = \"9f8e7d6c5b4a3210\"\nPassword = \"your-password\"\ntoken = \"ghp_0123456789abcdefghijABCDEFGHIJ012345\"\n";
        let w = scan_text("main.rr", text);
        let got: Vec<(usize, &str)> = w.iter().map(|w| (w.line, w.what.as_str())).collect();
        assert_eq!(got, [(2, "an API key (sk-\u{2026})"), (3, "a private key"), (4, "a password or key written in the code"), (6, "a GitHub token")]);
        assert_eq!(w[0].masked, "Key$ = \"sk-p\u{2026}\"");
        assert!(!w.iter().any(|w| w.masked.contains("abcdefghijklmnop")));
        // only added lines count
        let old = "Key$ = \"sk-proj-abcdefghijklmnopqrstuv\"\n";
        let new = "Key$ = \"sk-proj-abcdefghijklmnopqrstuv\"\nPRINT 2\n";
        assert!(scan_added("a.rr", old.as_bytes(), new.as_bytes()).is_empty());
        assert_eq!(scan_added("a.rr", b"", old.as_bytes()).len(), 1);
    }
}

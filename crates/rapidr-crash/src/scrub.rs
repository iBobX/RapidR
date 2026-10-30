//! What a report may not carry, taken out before a line is kept: the home
//! folder's path (`~` in its place, so the user's name stays home), keys,
//! tokens and passwords by their shapes (the shapes `tools/check_secrets.py`
//! and RAI's chat redaction look for: provider keys, `Bearer …`, `api_key=…`,
//! `password=…`, a password in a URL, private-key blocks, JSON Web Tokens),
//! and every value the program noted as a secret (the host's masker:
//! `rapidr_value::secrets`).
//!
//! A report never holds a file's contents or the program's data in the
//! first place (only RapidR's own messages, names and places); this is the
//! second fence.

use std::sync::OnceLock;

/// What stands where a secret was.
pub const REMOVED: &str = "[removed]";

static HOME: OnceLock<Vec<String>> = OnceLock::new();
static MASKER: OnceLock<fn(&str) -> Option<String>> = OnceLock::new();

/// The home folder's spellings (as the system gives it, with `/`, with
/// `\`), longest first.
fn homes() -> &'static [String] {
    HOME.get_or_init(|| {
        let mut out = Vec::new();
        let home = std::env::var("HOME").ok().filter(|h| h.len() > 1).or_else(|| std::env::var("USERPROFILE").ok().filter(|h| h.len() > 2));
        if let Some(h) = home {
            let h = h.trim_end_matches(['/', '\\']).to_string();
            if h.len() > 1 {
                // (the home as the system resolves it: a link, or macOS'
                // /var → /private/var, spells it another way)
                if let Ok(real) = std::fs::canonicalize(&h) {
                    let real = real.to_string_lossy().trim_end_matches(['/', '\\']).to_string();
                    if real.len() > 1 && real != h {
                        out.push(real.replace('\\', "/"));
                        out.push(real);
                    }
                }
                out.push(h.replace('\\', "/"));
                out.push(h.replace('/', "\\"));
                // (macOS: /private/var… is no home; /System/Volumes/Data/Users/… is)
                if h.starts_with("/Users/") {
                    out.push(format!("/System/Volumes/Data{h}"));
                }
                out.push(h);
            }
        }
        out.sort_by_key(|h| std::cmp::Reverse(h.len()));
        out.dedup();
        out
    })
}

/// The host's way to mask the secrets the program noted (`None`: it can't
/// say now — busy — and the text goes on as it is).
pub fn set_masker(f: fn(&str) -> Option<String>) {
    let _ = MASKER.set(f);
}

/// The home folder as `~` (a path that starts with it, anywhere in `text`).
pub fn home(text: &str) -> String {
    let mut out = text.to_string();
    for h in homes() {
        if out.contains(h.as_str()) {
            out = out.replace(h.as_str(), "~");
        }
    }
    out
}

/// `text` scrubbed: the home folder, secrets by shape, the program's noted
/// secrets.
pub fn scrub(text: &str) -> String {
    let mut out = home(text);
    if let Some(mask) = MASKER.get() {
        if let Some(m) = mask(&out) {
            out = m;
        }
    }
    out = private_keys(&out);
    secrets(&out)
}

/// Private-key and certificate blocks (`-----BEGIN … PRIVATE KEY-----` to
/// its END line) taken out whole.
fn private_keys(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find("-----BEGIN ") {
        let head_end = rest[at + 11..].find("-----").map(|e| at + 11 + e + 5);
        let Some(head_end) = head_end else { break };
        let label = &rest[at + 11..head_end - 5];
        if !(label.ends_with("PRIVATE KEY") || label.ends_with("PRIVATE KEY BLOCK")) {
            out.push_str(&rest[..head_end]);
            rest = &rest[head_end..];
            continue;
        }
        out.push_str(&rest[..at]);
        out.push_str(REMOVED);
        // (to its END line, or the end of the text)
        rest = match rest[head_end..].find("-----END ") {
            Some(e) => {
                let from = head_end + e + 9;
                match rest[from..].find("-----") {
                    Some(f) => &rest[from + f + 5..],
                    None => "",
                }
            }
            None => "",
        };
    }
    out.push_str(rest);
    out
}

/// The prefixes keys and tokens start with, and the fewest characters that
/// follow one in a real key (RAI's chat redaction's list, and
/// `tools/check_secrets.py`'s token shapes).
const PREFIXES: &[(&str, usize)] = &[
    ("sk-ant-", 16),
    ("sk-proj-", 16),
    ("sk-or-", 16),
    ("sk-", 20),
    ("sk_live_", 16),
    ("rk_live_", 16),
    ("sk_test_", 16),
    ("xai-", 20),
    ("gsk_", 20),
    ("ghp_", 20),
    ("gho_", 20),
    ("ghs_", 20),
    ("ghu_", 20),
    ("github_pat_", 20),
    ("glpat-", 16),
    ("hf_", 20),
    ("npm_", 30),
    ("AIza", 30),
    ("AKIA", 16),
    ("xoxb-", 16),
    ("xoxp-", 16),
    ("RWRTY0Iy", 30),
];

/// Labels whose value is a secret: `password=…`, `api_key: …`, `Bearer …`.
const LABELS: &[&str] = &["bearer ", "x-api-key", "api_key", "api-key", "apikey", "secret_key", "access_token", "auth_token", "password", "passwd", "pwd", "token", "secret"];

fn key_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_'
}

/// Keys and tokens by shape, labelled secrets, JSON Web Tokens and a
/// password in a URL.
fn secrets(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < text.len() {
        let starts_word = i == 0 || !key_char(text[..i].chars().next_back().unwrap_or(' '));
        let rest = &text[i..];
        if starts_word {
            if let Some(n) = key_at(rest).or_else(|| jwt_at(rest)) {
                out.push_str(REMOVED);
                i += n;
                continue;
            }
            if let Some((label, n)) = labelled(rest) {
                out.push_str(&rest[..label]);
                out.push_str(REMOVED);
                i += label + n;
                continue;
            }
        }
        // (scheme://user:password@host)
        if let Some(auth) = rest.strip_prefix("://") {
            if let Some((user, n)) = url_password(auth) {
                out.push_str("://");
                out.push_str(&auth[..user + 1]);
                out.push_str(REMOVED);
                i += 3 + user + 1 + n;
                continue;
            }
        }
        let c = rest.chars().next().unwrap_or(' ');
        out.push(c);
        i += c.len_utf8();
    }
    out
}

/// The length of the key `text` starts with.
fn key_at(text: &str) -> Option<usize> {
    for (prefix, min) in PREFIXES {
        if let Some(after) = text.strip_prefix(prefix) {
            let n: usize = after.chars().take_while(|c| key_char(*c) || *c == '.').map(char::len_utf8).sum();
            // (a sentence's full stop after a key isn't part of it)
            let body = after[..n].trim_end_matches('.');
            let n = body.len();
            if n >= *min && body.chars().any(|c| c.is_ascii_digit()) && body.chars().any(|c| c.is_ascii_alphabetic()) {
                return Some(prefix.len() + n);
            }
        }
    }
    None
}

/// A JSON Web Token's length (`eyJ….eyJ….…`).
fn jwt_at(text: &str) -> Option<usize> {
    if !text.starts_with("eyJ") {
        return None;
    }
    let n: usize = text.chars().take_while(|c| key_char(*c) || *c == '.').map(char::len_utf8).sum();
    let parts: Vec<&str> = text[..n].split('.').collect();
    (parts.len() == 3 && parts[1].starts_with("eyJ") && parts.iter().all(|p| p.len() >= 10)).then_some(n)
}

/// A label naming a secret and its value: (the label with what separates
/// it, the value's length).
fn labelled(text: &str) -> Option<(usize, usize)> {
    let low: String = text.chars().take(16).collect::<String>().to_ascii_lowercase();
    let label = LABELS.iter().find(|l| low.starts_with(*l))?;
    let mut k = label.len();
    let b = text.as_bytes();
    let bearer = *label == "bearer ";
    if !bearer {
        // (`password=x`, `"apiKey": "x"`, `token: x`; a longer word — "tokens", "passwords" — isn't one)
        if b.get(k).is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_') {
            return None;
        }
        if b.get(k) == Some(&b'"') {
            k += 1;
        }
        while b.get(k) == Some(&b' ') {
            k += 1;
        }
        if !matches!(b.get(k), Some(b':' | b'=')) {
            return None;
        }
        k += 1;
        while b.get(k).is_some_and(|c| matches!(c, b' ' | b'"' | b'\'')) {
            k += 1;
        }
    }
    let n: usize = text[k..].chars().take_while(|c| !c.is_whitespace() && !matches!(c, '"' | '\'' | ',' | ';' | ')' | '}' | ']' | '<' | '>' | '&')).map(char::len_utf8).sum();
    if n == 0 {
        return None;
    }
    let v = &text[k..k + n];
    // (a bearer token, a key: long and mixed; a password: anything but a
    // placeholder or a name in code — `Password = Edit1.Text`)
    let ok = if bearer || label.contains("key") || label.contains("token") {
        n >= 12 && v.chars().any(|c| c.is_ascii_digit()) && v.chars().any(|c| c.is_ascii_alphabetic())
    } else {
        !placeholder(v)
    };
    ok.then_some((k, n))
}

/// A password's value that is no password: a placeholder or code.
fn placeholder(v: &str) -> bool {
    let low = v.to_ascii_lowercase();
    matches!(low.as_str(), "" | "***" | "****" | "…" | "..." | "<password>" | "redacted" | "[removed]" | "none" | "null" | "empty")
        || low.starts_with('$')
        || low.starts_with('{')
        // (a name in code: `Edit1.Text`, `Form.Pass_Field`)
        || (v.contains('.') && v.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_') && v.split('.').all(|p| p.chars().next().is_some_and(|c| c.is_ascii_alphabetic())))
}

/// `user:password@` after `://`: (the user's length, the password's).
fn url_password(text: &str) -> Option<(usize, usize)> {
    let end = text.find(|c: char| c == '/' || c.is_whitespace() || c == '"' || c == '\'' || c == '<' || c == '>').unwrap_or(text.len());
    let auth = &text[..end];
    let at = auth.rfind('@')?;
    let colon = auth[..at].find(':')?;
    let pw = &auth[colon + 1..at];
    if pw.is_empty() || placeholder(pw) {
        return None;
    }
    Some((colon, pw.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_tokens_and_passwords_go() {
        let fake_ant = format!("sk-ant-api03-{}", "Ab12Cd34Ef56Gh78Ij90Kl12");
        let fake_gh = format!("ghp_{}", "aB3dE6gH9jK2mN5pQ8sT1vW4yZ7");
        let t = format!("key {fake_ant} and {fake_gh}.");
        assert_eq!(scrub(&t), "key [removed] and [removed].");
        assert_eq!(scrub("Authorization: Bearer abc123def456ghi789jkl"), "Authorization: Bearer [removed]");
        assert_eq!(scrub("api_key=Zx9Yw8Vu7Ts6Rq5Po4"), "api_key=[removed]");
        assert_eq!(scrub("{\"apiKey\": \"Zx9Yw8Vu7Ts6Rq5Po4\"}"), "{\"apiKey\": \"[removed]\"}");
        assert_eq!(scrub("password=hunter2 user=bob"), "password=[removed] user=bob");
        assert_eq!(scrub("PWD: s3cret!"), "PWD: [removed]");
        // (a password in an address, built from pieces: the secret scan
        // reads this file too)
        let url = |pw: &str| format!("mysql://bob:{pw}@db.example.com/x");
        assert_eq!(scrub(&url("hunter2")), url(REMOVED));
        let jwt = format!("eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.{}", "SflKxwRJSMeKKF2QT4fwpM");
        assert_eq!(scrub(&format!("t={jwt} end")), "t=[removed] end");
        let pem = "a\n-----BEGIN RSA PRIVATE KEY-----\nMIIEow\nAAAA\n-----END RSA PRIVATE KEY-----\nb";
        assert_eq!(scrub(pem), "a\n[removed]\nb");
        // (code and words stay)
        assert_eq!(scrub("Password = Edit1.Text"), "Password = Edit1.Text");
        assert_eq!(scrub("the tokens were read; sk-the-rest-of-a-sentence"), "the tokens were read; sk-the-rest-of-a-sentence");
        assert_eq!(scrub("-----BEGIN CERTIFICATE-----"), "-----BEGIN CERTIFICATE-----");
        assert_eq!(scrub("https://example.com/a@b"), "https://example.com/a@b");
    }

    #[test]
    fn the_home_folder_is_a_tilde() {
        let h = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap();
        let t = format!("{h}/Projects/Clock/main.rr and {h}");
        assert_eq!(home(&t), "~/Projects/Clock/main.rr and ~");
        assert!(!scrub(&t).contains(&h));
    }
}

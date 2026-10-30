//! The credentials a running program holds (an addition: RapidQ has none of
//! this). A value that came out of the OS keychain (`RSecrets.Read`), went
//! into it (`RSecrets.Save`), or was given to a property the language
//! registry marks `secret` (RMySQL's Password, RAI's APIKey) is noted here,
//! and what RapidR itself shows never spells it out:
//!
//! - the debugger shows `••••` for a variable, a watch, a hover or a
//!   component property that holds one, and a logpoint's message masks it;
//! - RapidR's own messages about a call (`[rapidr] Secrets.Save: …`) never
//!   carry the value, and what an AI is given of a program's state goes
//!   through [`mask`].
//!
//! What the program itself does with a value is its own business: `PRINT
//! pw$` prints it, as any BASIC would.
//!
//! The values are kept in this process's memory only, for as long as the
//! program runs; nothing is written anywhere.

use std::borrow::Cow;
use std::sync::Mutex;

/// What is shown in place of a secret.
pub const MASK: &str = "••••";

/// A secret shorter than this is masked only where a value *is* it, not
/// inside longer text (a one-letter password would otherwise mask every
/// word that has that letter).
const MIN_INSIDE: usize = 4;

/// At most this many are remembered (the oldest go first).
const MAX: usize = 256;

static KNOWN: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Notes `value` as a credential (empty and blank values are not).
pub fn note(value: &str) {
    if value.trim().is_empty() {
        return;
    }
    let mut known = KNOWN.lock().unwrap_or_else(|e| e.into_inner());
    if known.iter().any(|k| k == value) {
        return;
    }
    if known.len() >= MAX {
        known.remove(0);
    }
    known.push(value.to_string());
}

/// Whether `text` is a noted credential, or holds one.
pub fn is_secret(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }
    let known = KNOWN.lock().unwrap_or_else(|e| e.into_inner());
    known.iter().any(|k| k == text || (k.chars().count() >= MIN_INSIDE && text.contains(k.as_str())))
}

/// `text` with every noted credential in it replaced by [`MASK`] (a value
/// that is one: all of it).
pub fn mask(text: &str) -> Cow<'_, str> {
    if text.is_empty() {
        return Cow::Borrowed(text);
    }
    let known = KNOWN.lock().unwrap_or_else(|e| e.into_inner());
    if known.iter().any(|k| k == text) {
        return Cow::Owned(MASK.to_string());
    }
    // (the longest first: a secret that holds a shorter one is masked whole)
    let mut inside: Vec<&String> = known.iter().filter(|k| k.chars().count() >= MIN_INSIDE && text.contains(k.as_str())).collect();
    if inside.is_empty() {
        return Cow::Borrowed(text);
    }
    inside.sort_by_key(|k| std::cmp::Reverse(k.len()));
    let mut out = text.to_string();
    for k in inside {
        out = out.replace(k.as_str(), MASK);
    }
    Cow::Owned(out)
}

/// [`mask`] for a crash report (`rapidr_crash::scrub`): `None` when the
/// list is busy — the crash may have come while this thread held it — so
/// the report never waits for it.
pub fn try_mask(text: &str) -> Option<String> {
    let known = KNOWN.try_lock().ok()?;
    let mut inside: Vec<&String> = known.iter().filter(|k| k.as_str() == text || (k.chars().count() >= MIN_INSIDE && text.contains(k.as_str()))).collect();
    if inside.iter().any(|k| k.as_str() == text) {
        return Some(MASK.to_string());
    }
    inside.sort_by_key(|k| std::cmp::Reverse(k.len()));
    let mut out = text.to_string();
    for k in inside {
        out = out.replace(k.as_str(), MASK);
    }
    Some(out)
}

/// Forgets every noted value (a program run again in the same process).
pub fn forget_all() {
    KNOWN.lock().unwrap_or_else(|e| e.into_inner()).clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noted_values_are_masked() {
        note("hunter2-secret");
        note("");
        note("x");
        assert!(is_secret("hunter2-secret"));
        assert!(is_secret("pw=hunter2-secret;"));
        assert!(!is_secret("hunter2"));
        assert_eq!(mask("hunter2-secret"), MASK);
        assert_eq!(mask("user=a password=hunter2-secret"), format!("user=a password={MASK}"));
        // (a short one: only where the value is it)
        assert!(is_secret("x"));
        assert_eq!(mask("x"), MASK);
        assert_eq!(mask("box"), "box");
        assert_eq!(mask(""), "");
        assert!(!is_secret(""));
    }
}

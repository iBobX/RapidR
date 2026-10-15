//! API keys (docs/ide-ai.md §3). Never in a program's source, a project, a
//! bundle, a built app, a log, a capture or what's sent to a model.
//!
//! A program says where its key comes from with a **key source**
//! (RAI.APIKeySource), never the key itself:
//!
//! | Source | The key |
//! |---|---|
//! | `keychain:NAME` | kept under NAME in the OS keychain — the macOS Keychain, Windows' Credential Manager, the Secret Service on Linux (service [`SERVICE`]); on the web, the tab's session, or this device's storage for the page when the user asked to remember it |
//! | `env:VAR` | the environment variable VAR (read, never written; not on the web) |
//! | `ask` | none kept: the program asks its user (RAI's OnKeyNeeded) and sets APIKey |
//!
//! Several sources separated by `|` are tried in order
//! (`keychain:MyApp/OpenAI|env:OPENAI_API_KEY`). A key a program sets at run
//! time (RAI.APIKey) lives in memory only, unless it asks to keep it
//! ([`set_named`]). Without a keychain (a headless Linux) keys are kept for
//! the session only, and [`store_name`] says so.
//!
//! `RAPIDR_AI_KEYSTORE=memory` keeps keys in the process's memory only
//! (tests: the user's keychain is never touched).

use std::collections::HashMap;
use std::sync::Mutex;

/// The keychain's service: every key RapidR keeps is under it, by name.
pub const SERVICE: &str = "dev.rapidr.keys";

static SESSION: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);

fn session_get(name: &str) -> Option<String> {
    SESSION.lock().unwrap_or_else(|e| e.into_inner()).as_ref().and_then(|m| m.get(name).cloned())
}

fn session_set(name: &str, key: Option<&str>) {
    let mut s = SESSION.lock().unwrap_or_else(|e| e.into_inner());
    let m = s.get_or_insert_with(HashMap::new);
    match key {
        Some(k) => {
            m.insert(name.to_string(), k.to_string());
        }
        None => {
            m.remove(name);
        }
    }
}

/// Where a key came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    None,
    /// The keychain (the desktop), or the device (the web, remembered).
    Stored,
    /// This session only.
    Session,
    /// An environment variable.
    Environment,
    /// Set by the program at run time (RAI.APIKey).
    Program,
}

impl Source {
    pub fn as_str(self) -> &'static str {
        match self {
            Source::None => "",
            Source::Stored => "stored",
            Source::Session => "session",
            Source::Environment => "environment",
            Source::Program => "program",
        }
    }
}

fn memory_only() -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::env::var("RAPIDR_AI_KEYSTORE").is_ok_and(|v| v.eq_ignore_ascii_case("memory"))
    }
    #[cfg(target_arch = "wasm32")]
    {
        false
    }
}

/// A source's parts, in order: (kind, argument) — `("keychain", "MyApp")`,
/// `("env", "OPENAI_API_KEY")`, `("ask", "")`.
pub fn parts(spec: &str) -> Vec<(String, String)> {
    spec.split('|')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(|p| match p.split_once(':') {
            Some((k, v)) => (k.trim().to_ascii_lowercase(), v.trim().to_string()),
            None => (p.to_ascii_lowercase(), String::new()),
        })
        .collect()
}

/// Whether a source is well formed; why not.
pub fn check(spec: &str) -> Result<(), String> {
    for (k, v) in parts(spec) {
        match k.as_str() {
            "keychain" | "env" if v.is_empty() => return Err(format!("{k}: needs a name after the colon ({k}:NAME)")),
            "keychain" | "env" | "ask" => {}
            other => return Err(format!("`{other}` isn't a key source: keychain:NAME, env:VARIABLE or ask, separated by |")),
        }
    }
    Ok(())
}

/// The first keychain name a source gives ("" — none).
pub fn keychain_name(spec: &str) -> String {
    parts(spec).into_iter().find(|(k, _)| k == "keychain").map(|(_, v)| v).unwrap_or_default()
}

/// Whether a source ends by asking the program's user.
pub fn asks(spec: &str) -> bool {
    parts(spec).iter().any(|(k, _)| k == "ask")
}

/// The key a source gives now, and where it came from.
pub fn resolve(spec: &str) -> (Option<String>, Source) {
    for (kind, arg) in parts(spec) {
        match kind.as_str() {
            "keychain" => {
                let (k, s) = get_named(&arg);
                if k.is_some() {
                    return (k, s);
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            "env" => {
                if let Ok(k) = std::env::var(&arg) {
                    if !k.trim().is_empty() {
                        return (Some(k.trim().to_string()), Source::Environment);
                    }
                }
            }
            _ => {}
        }
    }
    (None, Source::None)
}

/// The key kept under `name`: the session's, else the keychain's (the
/// device's on the web).
pub fn get_named(name: &str) -> (Option<String>, Source) {
    if name.trim().is_empty() {
        return (None, Source::None);
    }
    if let Some(k) = session_get(name) {
        return (Some(k), Source::Session);
    }
    if !memory_only() {
        if let Some(k) = store::get(name) {
            return (Some(k), Source::Stored);
        }
    }
    (None, Source::None)
}

/// A key as typed, checked: no spaces or line breaks.
pub fn clean(key: &str) -> Result<String, String> {
    let key = key.trim();
    if key.is_empty() {
        return Err("the key is empty".into());
    }
    if key.chars().any(|c| c.is_control() || c == ' ') {
        return Err("a key has no spaces or line breaks: paste it again".into());
    }
    Ok(key.to_string())
}

/// Keeps `key` under `name`: in the keychain (the desktop); on the web for
/// this session, and on this device too when `remember`. Gives where it
/// went, in words.
pub fn set_named(name: &str, key: &str, remember: bool) -> Result<String, String> {
    let key = clean(key)?;
    if name.trim().is_empty() {
        return Err("give the key a name (keychain:NAME)".into());
    }
    if memory_only() || !store::available() {
        session_set(name, Some(&key));
        return Ok(session_words().into());
    }
    if cfg!(target_arch = "wasm32") && !remember {
        store::delete(name);
        session_set(name, Some(&key));
        return Ok(session_words().into());
    }
    store::set(name, &key)?;
    session_set(name, None);
    Ok(store_name())
}

/// Forgets the key kept under `name` (the keychain's, the session's, the
/// device's).
pub fn delete_named(name: &str) {
    session_set(name, None);
    if !memory_only() {
        store::delete(name);
    }
}

/// `…abcd`: the key's last four characters, all that is ever shown of it.
pub fn hint(key: &str) -> String {
    let n = key.chars().count();
    if n < 8 {
        return "(set)".into();
    }
    let tail: String = key.chars().skip(n - 4).collect();
    format!("…{tail}")
}

fn session_words() -> &'static str {
    if cfg!(target_arch = "wasm32") {
        "this browser tab, until it closes"
    } else {
        "memory, until the program ends (this computer has no keychain RapidR can use)"
    }
}

/// Where a key from `source` is, in words; with none, where
/// `RememberKey(False)` keeps one (the keychain; on the web, the tab).
pub fn where_kept(source: Source) -> String {
    match source {
        Source::Session if memory_only() => store_name(),
        Source::Session => session_words().into(),
        Source::Program => "this program's memory, until it ends".into(),
        Source::Environment => "an environment variable".into(),
        Source::Stored => store_name(),
        Source::None if cfg!(target_arch = "wasm32") => session_words().into(),
        Source::None => store_name(),
    }
}

/// Where keys are kept here, in words ("the macOS Keychain").
pub fn store_name() -> String {
    if memory_only() {
        return "memory, for this session only".into();
    }
    store::name()
}

/// Whether the key under `name` is kept on this device (the web's
/// "remember"; on the desktop, whether it is in the keychain).
pub fn remembered(name: &str) -> bool {
    !memory_only() && store::get(name).is_some()
}

/// (Anthropic) The workspace a key that isn't scoped to one works in, from
/// `ANTHROPIC_WORKSPACE_ID` when the program gives none (read, never
/// written; "" — none).
pub fn env_workspace(provider_id: &str) -> String {
    #[cfg(not(target_arch = "wasm32"))]
    if provider_id.eq_ignore_ascii_case("anthropic") {
        return std::env::var("ANTHROPIC_WORKSPACE_ID").map(|w| w.trim().to_string()).unwrap_or_default();
    }
    let _ = provider_id;
    String::new()
}

/// What a build of `source` (a program, its `$INCLUDE`s expanded) should
/// be told about keys, a line each: a key written in the program (anyone
/// with the built app can read it), and on a web page (`web`), that an
/// RAI's keychain and environment find nothing there — the page asks its
/// user ("ask") or has no key. Never the key itself.
pub fn build_warnings(source: &str, web: bool) -> Vec<String> {
    let mut out = Vec::new();
    let mut uses_rai = false;
    let mut sources: Vec<String> = Vec::new();
    for (n, line) in source.lines().enumerate() {
        let (code, strings) = code_and_strings(line);
        let upper = code.to_ascii_uppercase();
        if words(&upper).windows(2).any(|w| w[0] == "AS" && w[1] == "RAI") {
            uses_rai = true;
        }
        // (`APIKey = "…"`, `AI.APIKey = "…"`: the key in the program)
        let assigns_key = words(&upper).windows(2).any(|w| (w[0] == "APIKEY" || w[0].ends_with(".APIKEY")) && w[1] == "=");
        let key_like = strings.iter().any(|s| looks_like_key(s));
        if (assigns_key && strings.iter().any(|s| !s.trim().is_empty())) || key_like {
            out.push(format!(
                "line {}: an API key is written in the program: anyone who has the built app can read it. Keep it in the keychain (APIKeySource = \"keychain:NAME\"), in an environment variable (\"env:NAME\"), or ask the user (\"ask\").",
                n + 1
            ));
        }
        if upper.contains("APIKEYSOURCE") {
            sources.extend(strings.iter().map(|s| s.to_string()));
        }
    }
    if web && uses_rai {
        let never_asks = !sources.is_empty() && sources.iter().all(|s| !asks(s));
        out.push(if never_asks {
            "RAI on a web page: a browser has no keychain and no environment variables (no key from yours goes into the page), so with an APIKeySource that doesn't \"ask\" the page has no key. Add \"|ask\": the page then asks its user for their own key (OnKeyNeeded), kept for the tab.".to_string()
        } else {
            "RAI on a web page: a browser has no keychain and no environment variables (no key from yours goes into the page): the page asks its user for their own key (OnKeyNeeded), kept for the tab — or on their device when the program calls RememberKey(True).".to_string()
        });
    }
    out
}

/// A line's code (strings blanked, the comment cut) and its strings.
fn code_and_strings(line: &str) -> (String, Vec<&str>) {
    let mut code = String::new();
    let mut strings = Vec::new();
    let mut rest = line;
    loop {
        match rest.find(['"', '\'']) {
            Some(i) if rest.as_bytes()[i] == b'"' => {
                code.push_str(&rest[..i]);
                code.push_str("\"\"");
                let after = &rest[i + 1..];
                let end = after.find('"').unwrap_or(after.len());
                strings.push(&after[..end]);
                rest = after.get(end + 1..).unwrap_or("");
            }
            Some(i) => {
                code.push_str(&rest[..i]);
                break;
            }
            None => {
                code.push_str(rest);
                break;
            }
        }
    }
    let upper = code.trim_start().to_ascii_uppercase();
    if upper == "REM" || upper.starts_with("REM ") {
        return (String::new(), Vec::new());
    }
    (code, strings)
}

fn words(code: &str) -> Vec<&str> {
    code.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '$' || c == '='))
        .flat_map(|w| {
            // (`APIKey="x"`: the name and the `=` apart)
            let mut v = Vec::new();
            let mut s = w;
            while let Some(i) = s.find('=') {
                if i > 0 {
                    v.push(&s[..i]);
                }
                v.push("=");
                s = &s[i + 1..];
            }
            if !s.is_empty() {
                v.push(s);
            }
            v
        })
        .collect()
}

/// `text` with every key-shaped word replaced (what an AI's tools read of
/// a project: a key a developer wrote into a file never reaches a model).
pub fn redact(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut word = String::new();
    let flush = |out: &mut String, word: &mut String| {
        if looks_like_key(word) {
            out.push_str("(an API key, not shown)");
        } else {
            out.push_str(word);
        }
        word.clear();
    };
    for c in text.chars() {
        if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
            word.push(c);
        } else {
            flush(&mut out, &mut word);
            out.push(c);
        }
    }
    flush(&mut out, &mut word);
    out
}

/// A string that is, by its look, a provider's key (`sk-…`, `sk-ant-…`,
/// `xai-…` and long enough).
fn looks_like_key(s: &str) -> bool {
    let s = s.trim();
    let prefixed = ["sk-", "sk_", "xai-", "gsk_", "pk-"].iter().any(|p| s.starts_with(p));
    prefixed && s.len() >= 24 && !s.contains(char::is_whitespace)
}

#[cfg(not(target_arch = "wasm32"))]
mod store {
    use std::sync::{Arc, OnceLock};

    use keyring_core::Entry;

    static READY: OnceLock<bool> = OnceLock::new();

    /// The platform's keychain as keyring-core's default store, once.
    fn ready() -> bool {
        *READY.get_or_init(|| {
            #[cfg(target_os = "macos")]
            let store = apple_native_keyring_store::keychain::Store::new().map(|s| s as Arc<keyring_core::CredentialStore>);
            #[cfg(windows)]
            let store = windows_native_keyring_store::Store::new().map(|s| s as Arc<keyring_core::CredentialStore>);
            #[cfg(all(unix, not(target_os = "macos")))]
            let store = zbus_secret_service_keyring_store::Store::new().map(|s| s as Arc<keyring_core::CredentialStore>);
            match store {
                Ok(s) => {
                    keyring_core::set_default_store(s);
                    true
                }
                Err(_) => false,
            }
        })
    }

    pub fn available() -> bool {
        ready()
    }

    pub fn name() -> String {
        if !ready() {
            return super::session_words().into();
        }
        if cfg!(target_os = "macos") {
            "the macOS Keychain".into()
        } else if cfg!(windows) {
            "Windows Credential Manager".into()
        } else {
            "the Secret Service (GNOME Keyring or KWallet)".into()
        }
    }

    pub fn get(account: &str) -> Option<String> {
        if !ready() {
            return None;
        }
        Entry::new(super::SERVICE, account).ok()?.get_password().ok()
    }

    pub fn set(account: &str, key: &str) -> Result<(), String> {
        if !ready() {
            return Err("there is no keychain here".into());
        }
        let e = Entry::new(super::SERVICE, account).map_err(|e| e.to_string())?;
        e.set_password(key).map_err(|e| format!("the keychain refused it: {e}"))
    }

    pub fn delete(account: &str) {
        if ready() {
            if let Ok(e) = Entry::new(super::SERVICE, account) {
                let _ = e.delete_credential();
            }
        }
    }

    /// (tests) A keychain item under another service, set, read, deleted.
    #[cfg(test)]
    pub fn round_trip(service: &str, account: &str, key: &str) -> Result<Option<String>, String> {
        if !ready() {
            return Ok(None);
        }
        let e = Entry::new(service, account).map_err(|e| e.to_string())?;
        e.set_password(key).map_err(|e| e.to_string())?;
        let got = e.get_password().map_err(|e| e.to_string())?;
        e.delete_credential().map_err(|e| e.to_string())?;
        let gone = e.get_password().is_err();
        Ok(gone.then_some(got))
    }
}

#[cfg(target_arch = "wasm32")]
mod store {
    fn storage() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok().flatten()
    }

    fn item(name: &str) -> String {
        format!("{}.{name}", super::SERVICE)
    }

    pub fn available() -> bool {
        storage().is_some()
    }

    pub fn name() -> String {
        "this browser, on this device".into()
    }

    pub fn get(name: &str) -> Option<String> {
        storage()?.get_item(&item(name)).ok().flatten().filter(|k| !k.is_empty())
    }

    pub fn set(name: &str, key: &str) -> Result<(), String> {
        storage().ok_or("this browser keeps nothing for the page")?.set_item(&item(name), key).map_err(|_| "the browser refused to keep it".to_string())
    }

    pub fn delete(name: &str) {
        if let Some(s) = storage() {
            let _ = s.remove_item(&item(name));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sources_and_session_keys() {
        std::env::set_var("RAPIDR_AI_KEYSTORE", "memory");
        assert_eq!(parts("keychain:My App/OpenAI | env:OPENAI_API_KEY|ask"), vec![("keychain".into(), "My App/OpenAI".into()), ("env".into(), "OPENAI_API_KEY".into()), ("ask".into(), String::new())]);
        assert!(check("keychain:X|env:Y|ask").is_ok());
        assert!(check("keychain:").is_err());
        assert!(check("sk-abc123").is_err(), "a key pasted as a source is refused");
        assert_eq!(keychain_name("env:A|keychain:B"), "B");
        assert!(asks("keychain:B|ask") && !asks("keychain:B"));
        assert_eq!(resolve("keychain:t1"), (None, Source::None));
        assert!(set_named("t1", "  ", false).is_err());
        assert!(set_named("t1", "sk two words", false).is_err());
        let r#where = set_named("t1", "sk-local-1234567890", false).unwrap();
        assert!(r#where.contains("memory"), "{where}");
        assert_eq!(resolve("env:RAPIDR_TEST_NOT_SET|keychain:t1"), (Some("sk-local-1234567890".into()), Source::Session));
        std::env::set_var("RAPIDR_TEST_KEY_VAR", "sk-env-0987654321");
        assert_eq!(resolve("env:RAPIDR_TEST_KEY_VAR|keychain:t1").1, Source::Environment);
        assert_eq!(hint("sk-local-1234567890"), "…7890");
        assert_eq!(hint("short"), "(set)");
        delete_named("t1");
        assert_eq!(resolve("keychain:t1").1, Source::None);
    }

    #[test]
    fn builds_are_told_about_keys() {
        let fine = "DIM AI AS RAI\nAI.APIKeySource = \"keychain:My App|ask\"\nAI.APIKey = KeyEdit.Text\n";
        assert!(build_warnings(fine, false).is_empty());
        let web = build_warnings(fine, true);
        assert_eq!(web.len(), 1);
        assert!(web[0].contains("asks its user"), "{web:?}");
        let written = "CREATE AI AS RAI\n    APIKey = \"sk-proj-abcdefghijklmnopqrstuvwxyz\"\nEND CREATE\n";
        let w = build_warnings(written, false);
        assert_eq!(w.len(), 1);
        assert!(w[0].starts_with("line 2: an API key is written in the program"), "{w:?}");
        assert!(!w[0].contains("abcdefghij"), "never the key itself");
        assert_eq!(build_warnings("x$ = \"sk-ant-api03-0123456789abcdefghij\"", false).len(), 1);
        // (a comment, an empty key, a short sk- word: nothing)
        assert!(build_warnings("' APIKey = \"sk-ant-0123456789abcdefghijkl\"\nAI.APIKey = \"\"\nPRINT \"sk-short\"", false).is_empty());
        let never = build_warnings("DIM AI AS RAI\nAI.APIKeySource = \"env:OPENAI_API_KEY\"", true);
        assert!(never[0].contains("doesn't \"ask\""), "{never:?}");
    }

    #[test]
    fn keys_in_files_are_redacted() {
        let text = "AI.APIKey = \"sk-proj-abcdefghijklmnopqrstuvwxyz\"\nOPENAI_API_KEY=sk-ant-api03-0123456789abcdefghij\nPRINT \"sk-short\"\n";
        let r = redact(text);
        assert!(!r.contains("abcdefghij") && !r.contains("0123456789"), "{r}");
        assert_eq!(r.matches("(an API key, not shown)").count(), 2, "{r}");
        assert!(r.contains("PRINT \"sk-short\""), "{r}");
        assert_eq!(redact("no keys here: café"), "no keys here: café");
    }

    /// The real keychain, under a test service (never RapidR's own): set,
    /// read, deleted. `cargo test -p rapidr-ai -- --ignored keychain`.
    #[test]
    #[ignore]
    #[cfg(not(target_arch = "wasm32"))]
    fn keychain_round_trip() {
        let got = store::round_trip("dev.rapidr.test", "rapidr-ai-test", "sk-test-not-a-real-key-0000").unwrap();
        assert_eq!(got.as_deref(), Some("sk-test-not-a-real-key-0000"));
    }
}

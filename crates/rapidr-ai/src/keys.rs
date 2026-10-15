//! API keys (docs/ide-ai.md §3). Never in a project, a workspace file, a
//! bundle, a log or what's sent to a model.
//!
//! * The desktop: the OS keychain — the macOS Keychain, Windows' Credential
//!   Manager, the Secret Service (GNOME Keyring, KWallet) on Linux; service
//!   `dev.rapidr.ai`, the account the provider's id. Without one (a
//!   headless Linux) a key is kept for the session only, and
//!   [`store_name`] says so.
//! * The web: the tab's memory for the session; on this device (the
//!   browser's storage for the page) only when the user asks, after a
//!   warning ([`set`]'s `remember`).
//! * Either: a provider's environment variable (`OPENAI_API_KEY` …) is used
//!   when no key is stored, and never written.
//!
//! `RAPIDR_AI_KEYSTORE=memory` keeps keys in the process's memory only
//! (tests: the user's keychain is never touched).

use std::collections::HashMap;
use std::sync::Mutex;

#[cfg(not(target_arch = "wasm32"))]
use crate::provider::provider;

/// The keychain's service name.
pub const SERVICE: &str = "dev.rapidr.ai";

static SESSION: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);

fn session_get(account: &str) -> Option<String> {
    SESSION.lock().unwrap_or_else(|e| e.into_inner()).as_ref().and_then(|m| m.get(account).cloned())
}

fn session_set(account: &str, key: Option<&str>) {
    let mut s = SESSION.lock().unwrap_or_else(|e| e.into_inner());
    let m = s.get_or_insert_with(HashMap::new);
    match key {
        Some(k) => {
            m.insert(account.to_string(), k.to_string());
        }
        None => {
            m.remove(account);
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
    /// The provider's environment variable.
    Environment,
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

/// The key for `provider_id`, and where it came from.
pub fn get(provider_id: &str) -> (Option<String>, Source) {
    let id = provider_id.to_ascii_lowercase();
    if let Some(k) = session_get(&id) {
        return (Some(k), Source::Session);
    }
    if !memory_only() {
        if let Some(k) = store::get(&id) {
            return (Some(k), Source::Stored);
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(var) = provider(&id).map(|p| p.env_key).filter(|v| !v.is_empty()) {
        if let Ok(k) = std::env::var(var) {
            if !k.trim().is_empty() {
                return (Some(k.trim().to_string()), Source::Environment);
            }
        }
    }
    (None, Source::None)
}

/// (Anthropic) The workspace a key that isn't scoped to one works in, from
/// `ANTHROPIC_WORKSPACE_ID` when the settings give none (read, never
/// written; "" — none).
pub fn env_workspace(provider_id: &str) -> String {
    #[cfg(not(target_arch = "wasm32"))]
    if provider_id.eq_ignore_ascii_case("anthropic") {
        return std::env::var("ANTHROPIC_WORKSPACE_ID").map(|w| w.trim().to_string()).unwrap_or_default();
    }
    let _ = provider_id;
    String::new()
}

/// Keeps `key` for `provider_id`: in the keychain (the desktop), or for
/// this session — and on this device too when `remember` (the web).
/// Gives where it went, in words.
pub fn set(provider_id: &str, key: &str, remember: bool) -> Result<String, String> {
    let id = provider_id.to_ascii_lowercase();
    let key = key.trim();
    if key.is_empty() {
        return Err("the key is empty".into());
    }
    if key.chars().any(|c| c.is_control() || c == ' ') {
        return Err("a key has no spaces or line breaks: paste it again".into());
    }
    if memory_only() || !store::available() {
        session_set(&id, Some(key));
        return Ok(session_words().into());
    }
    if cfg!(target_arch = "wasm32") && !remember {
        store::delete(&id);
        session_set(&id, Some(key));
        return Ok(session_words().into());
    }
    store::set(&id, key)?;
    session_set(&id, None);
    Ok(store_name())
}

/// Forgets the key of `provider_id` (the keychain's, the session's, the
/// device's); an environment variable stays (it isn't RapidR's).
pub fn delete(provider_id: &str) -> Result<(), String> {
    let id = provider_id.to_ascii_lowercase();
    session_set(&id, None);
    if !memory_only() {
        store::delete(&id);
    }
    Ok(())
}

/// `…abcd`: the key's last four characters, all the settings ever show.
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
        "memory, until RapidR Studio closes (this computer has no keychain RapidR can use)"
    }
}

/// Where keys are kept here, in words ("the macOS Keychain").
pub fn store_name() -> String {
    if memory_only() {
        return "memory, for this session only".into();
    }
    store::name()
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

    fn item(account: &str) -> String {
        format!("{}.key.{account}", super::SERVICE)
    }

    pub fn available() -> bool {
        storage().is_some()
    }

    pub fn name() -> String {
        "this browser, on this device".into()
    }

    pub fn get(account: &str) -> Option<String> {
        storage()?.get_item(&item(account)).ok().flatten().filter(|k| !k.is_empty())
    }

    pub fn set(account: &str, key: &str) -> Result<(), String> {
        storage().ok_or("this browser keeps nothing for the page")?.set_item(&item(account), key).map_err(|_| "the browser refused to keep it".to_string())
    }

    pub fn delete(account: &str) {
        if let Some(s) = storage() {
            let _ = s.remove_item(&item(account));
        }
    }
}

/// Whether the key of `provider_id` is kept on this device (the web's
/// "remember"; on the desktop, whether it is in the keychain).
pub fn remembered(provider_id: &str) -> bool {
    !memory_only() && store::get(&provider_id.to_ascii_lowercase()).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_keys_and_hints() {
        std::env::set_var("RAPIDR_AI_KEYSTORE", "memory");
        assert_eq!(get("compatible"), (None, Source::None));
        assert!(set("compatible", "  ", false).is_err());
        assert!(set("compatible", "sk two words", false).is_err());
        let r#where = set("Compatible", "sk-local-1234567890", false).unwrap();
        assert!(r#where.contains("memory"), "{where}");
        assert_eq!(get("compatible"), (Some("sk-local-1234567890".into()), Source::Session));
        assert_eq!(hint("sk-local-1234567890"), "…7890");
        assert_eq!(hint("short"), "(set)");
        delete("compatible").unwrap();
        assert_eq!(get("compatible").1, Source::None);
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

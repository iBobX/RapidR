//! RSecrets: the OS keychain for programs (REQUEST-001 item 4; an addition,
//! RapidQ has none). A program keeps a password, a token or a key by a
//! service and a name — `Secrets.Save "MySQL", "office", pw$` — in the
//! same keychain and through the same code RapidR Studio keeps its AI keys
//! in ([`crate::keys`]): the macOS Keychain, Windows Credential Manager,
//! the Secret Service on Linux.
//!
//! - **Each program has its own items.** A program's service `S` is kept
//!   as `rapidr-app:<program>:S`, `<program>` its file's name in lower case
//!   (`dbstudio.rr`, `DBStudio.exe` and `DBStudio.app` are `dbstudio`), so
//!   no program reaches RapidR Studio's keys ([`crate::keys::SERVICE`]) or
//!   another program's items through RSecrets.
//! - **Never shown.** What is read and what is saved is noted as a
//!   credential (`rapidr_value::secrets`): the debugger shows `••••`, and
//!   the registry marks Read and Save `secret`, so an AI is never given it.
//!   Errors never carry the value.
//! - **The test keystore.** `RAPIDR_AI_KEYSTORE=memory` (or
//!   `RAPIDR_KEYSTORE=memory`) keeps everything in the process's memory,
//!   for tests: the user's keychain is never touched.
//! - **The web** has no keychain: every call fails with
//!   [`NOT_IN_BROWSER`] (no fallback to the page's storage, which any
//!   script on the page could read).

use std::collections::HashMap;
use std::sync::Mutex;

/// Every program's items are under services that start so.
pub const PREFIX: &str = "rapidr-app";

/// The longest value kept, in characters (Windows Credential Manager keeps
/// 2,560 bytes).
pub const MAX_VALUE: usize = 1024;

/// The longest service or name, in characters.
pub const MAX_NAME: usize = 200;

/// What every call says in a browser.
pub const NOT_IN_BROWSER: &str = "RSecrets isn't available in the browser: a web page has no keychain, and RapidR never keeps secrets in the page's storage";

/// The test keystore's items: (keychain service, name) → value.
static MEMORY: Mutex<Option<HashMap<(String, String), String>>> = Mutex::new(None);

fn memory<R>(f: impl FnOnce(&mut HashMap<(String, String), String>) -> R) -> R {
    let mut m = MEMORY.lock().unwrap_or_else(|e| e.into_inner());
    f(m.get_or_insert_with(HashMap::new))
}

/// A program's name as its items are kept under (`<program>` in
/// `rapidr-app:<program>:<service>`): the file name of `path` without its
/// extension, in lower case, letters, digits, `.`, `-` and `_` (anything
/// else `_`); `program` when there is none.
pub fn program_name(path: &str) -> String {
    let file = path.rsplit(['/', '\\']).next().unwrap_or("");
    let stem = match file.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() && ["rr", "rrbc", "bas", "exe", "inc", "app"].iter().any(|e| ext.eq_ignore_ascii_case(e)) => stem,
        _ => file,
    };
    let name: String = stem.chars().map(|c| if c.is_alphanumeric() || matches!(c, '.' | '-' | '_') { c } else { '_' }).flat_map(char::to_lowercase).collect();
    if name.trim_matches(['.', '_']).is_empty() { "program".into() } else { name }
}

/// The keychain service a program's service is kept under.
pub fn keychain_service(program: &str, service: &str) -> String {
    format!("{PREFIX}:{program}:{service}")
}

/// A service's or a name's problem, if it has one.
fn check_name(what: &str, text: &str) -> Result<(), String> {
    if text.trim().is_empty() {
        return Err(format!("the {what} is empty"));
    }
    if text.chars().count() > MAX_NAME {
        return Err(format!("the {what} is longer than {MAX_NAME} characters"));
    }
    if text.chars().any(char::is_control) {
        return Err(format!("the {what} has a control character (a line break, a tab …)"));
    }
    Ok(())
}

fn check(service: &str, name: &str) -> Result<(), String> {
    check_name("service", service)?;
    check_name("name", name)
}

/// Whether secrets can be kept here (a keychain, or the test keystore).
pub fn available() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        false
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        crate::keys::memory_only() || crate::keys::store::available()
    }
}

/// Where secrets are kept here, in words ("the macOS Keychain").
pub fn store_name() -> String {
    if cfg!(target_arch = "wasm32") {
        return "no keychain (a web page has none)".into();
    }
    if crate::keys::memory_only() {
        return "memory, for this run only (the test keystore)".into();
    }
    if !available() {
        return "no keychain (none RapidR can use on this computer)".into();
    }
    crate::keys::store_name()
}

fn unavailable() -> String {
    if cfg!(target_arch = "wasm32") {
        NOT_IN_BROWSER.into()
    } else {
        "this computer has no keychain RapidR can use (on Linux: the Secret Service, GNOME Keyring or KWallet)".into()
    }
}

/// Keeps `value` as `service` / `name` of `program`, replacing what was
/// there.
pub fn save(program: &str, service: &str, name: &str, value: &str) -> Result<(), String> {
    check(service, name)?;
    if value.is_empty() {
        return Err("the value is empty (Delete removes a secret)".into());
    }
    if value.chars().count() > MAX_VALUE {
        return Err(format!("the value is longer than {MAX_VALUE} characters"));
    }
    let kc = keychain_service(program, service);
    if crate::keys::memory_only() {
        memory(|m| m.insert((kc, name.to_string()), value.to_string()));
        return Ok(());
    }
    if !available() {
        return Err(unavailable());
    }
    keychain::set(&kc, name, value)
}

/// The OS keychain's side (the desktop's; a browser has none).
mod keychain {
    #[cfg(not(target_arch = "wasm32"))]
    pub use crate::keys::store::{delete_in as delete, get_in as get, set_in as set};

    #[cfg(target_arch = "wasm32")]
    pub fn set(_: &str, _: &str, _: &str) -> Result<(), String> {
        Err(super::NOT_IN_BROWSER.into())
    }

    #[cfg(target_arch = "wasm32")]
    pub fn get(_: &str, _: &str) -> Result<Option<String>, String> {
        Err(super::NOT_IN_BROWSER.into())
    }

    #[cfg(target_arch = "wasm32")]
    pub fn delete(_: &str, _: &str) -> Result<bool, String> {
        Err(super::NOT_IN_BROWSER.into())
    }
}

/// What is kept as `service` / `name` of `program` (`None`: nothing).
pub fn read(program: &str, service: &str, name: &str) -> Result<Option<String>, String> {
    check(service, name)?;
    let kc = keychain_service(program, service);
    if crate::keys::memory_only() {
        return Ok(memory(|m| m.get(&(kc, name.to_string())).cloned()));
    }
    if !available() {
        return Err(unavailable());
    }
    keychain::get(&kc, name)
}

/// Forgets `service` / `name` of `program`; whether there was one.
pub fn delete(program: &str, service: &str, name: &str) -> Result<bool, String> {
    check(service, name)?;
    let kc = keychain_service(program, service);
    if crate::keys::memory_only() {
        return Ok(memory(|m| m.remove(&(kc, name.to_string())).is_some()));
    }
    if !available() {
        return Err(unavailable());
    }
    keychain::delete(&kc, name)
}

/// Whether something is kept as `service` / `name` of `program`.
pub fn exists(program: &str, service: &str, name: &str) -> Result<bool, String> {
    read(program, service, name).map(|v| v.is_some())
}

/// RSecrets, the component programs use (its properties, its methods).
#[cfg(feature = "component")]
pub mod component {
    use std::cell::RefCell;
    use std::collections::HashMap;

    use rapidr_value::Value;

    thread_local! {
        /// Each RSecrets' last error ("" when its last call worked).
        static ERRORS: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
    }

    /// Whether `type_name` is RSecrets.
    pub fn is_secrets(type_name: &str) -> bool {
        type_name.eq_ignore_ascii_case("RSECRETS")
    }

    fn flag(b: bool) -> Value {
        Value::Integer(if b { -1 } else { 0 })
    }

    fn set_error(name: &str, error: &str) {
        ERRORS.with(|e| e.borrow_mut().insert(name.to_ascii_lowercase(), error.to_string()));
    }

    /// The running program's name, as its items are kept under.
    pub fn program() -> String {
        super::program_name(&rapidr_value::command_line::path())
    }

    /// A property of RSecrets `name` (`None`: not one of its own).
    pub fn get(name: &str, prop: &str) -> Option<Value> {
        Some(match prop {
            "error" => Value::String(ERRORS.with(|e| e.borrow().get(&name.to_ascii_lowercase()).cloned().unwrap_or_default())),
            "available" => flag(super::available()),
            "store" => Value::String(super::store_name()),
            "program" => Value::String(program()),
            _ => return None,
        })
    }

    /// A method of RSecrets `name` (`None`: not one of its own). Its Error
    /// says why a call failed ("" when it worked); the value is never in
    /// it, nor in what the runtime reports.
    pub fn call(name: &str, method: &str, args: &[Value]) -> Option<Value> {
        let text = |i: usize| args.get(i).map(Value::to_string_val).unwrap_or_default();
        let (service, key) = (text(0), text(1));
        let program = program();
        let (result, value) = match method {
            "save" => {
                let value = text(2);
                // (a credential, whether or not it is kept)
                rapidr_value::secrets::note(&value);
                let r = super::save(&program, &service, &key, &value);
                (r.as_ref().map(|_| ()).map_err(Clone::clone), flag(r.is_ok()))
            }
            "read" => match super::read(&program, &service, &key) {
                Ok(v) => {
                    let v = v.unwrap_or_default();
                    rapidr_value::secrets::note(&v);
                    (Ok(()), Value::String(v))
                }
                Err(e) => (Err(e), Value::String(String::new())),
            },
            "delete" => match super::delete(&program, &service, &key) {
                Ok(had) => (Ok(()), flag(had)),
                Err(e) => (Err(e), flag(false)),
            },
            "exists" => match super::exists(&program, &service, &key) {
                Ok(has) => (Ok(()), flag(has)),
                Err(e) => (Err(e), flag(false)),
            },
            _ => return None,
        };
        match result {
            Ok(()) => set_error(name, ""),
            Err(e) => {
                // (said once where the runtime's messages go, never with
                // the value)
                let e = rapidr_value::secrets::mask(&e).into_owned();
                set_error(name, &e);
                report(&format!("[rapidr] {name}.{method}: {e}"));
            }
        }
        Some(value)
    }

    /// A message where the runtime's go (stderr; a browser's console).
    fn report(message: &str) {
        #[cfg(target_arch = "wasm32")]
        web_sys::console::warn_1(&message.into());
        #[cfg(not(target_arch = "wasm32"))]
        eprintln!("{message}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_namespaces() {
        assert_eq!(program_name("/Users/me/apps/DBStudio.rr"), "dbstudio");
        assert_eq!(program_name("C:\\Apps\\DBStudio.exe"), "dbstudio");
        assert_eq!(program_name("/Applications/DBStudio.app/Contents/MacOS/DBStudio"), "dbstudio");
        assert_eq!(program_name("my app:v2.rrbc"), "my_app_v2");
        assert_eq!(program_name(""), "program");
        assert_eq!(keychain_service("dbstudio", "MySQL"), "rapidr-app:dbstudio:MySQL");
        // (never RapidR Studio's own service)
        assert!(!keychain_service("x", crate::keys::SERVICE).starts_with(crate::keys::SERVICE));
    }

    /// The test keystore: saved, read, found, deleted; programs apart; the
    /// keychain never touched.
    #[test]
    fn the_test_keystore() {
        std::env::set_var("RAPIDR_AI_KEYSTORE", "memory");
        assert!(available());
        assert!(store_name().contains("test keystore"), "{}", store_name());
        save("app1", "MySQL", "office", "s3cret-pw").unwrap();
        assert_eq!(read("app1", "MySQL", "office").unwrap().as_deref(), Some("s3cret-pw"));
        assert!(exists("app1", "MySQL", "office").unwrap());
        // (another program, another service: nothing)
        assert_eq!(read("app2", "MySQL", "office").unwrap(), None);
        assert_eq!(read("app1", "MySQL2", "office").unwrap(), None);
        save("app1", "MySQL", "office", "new-pw").unwrap();
        assert_eq!(read("app1", "MySQL", "office").unwrap().as_deref(), Some("new-pw"));
        assert!(delete("app1", "MySQL", "office").unwrap());
        assert!(!delete("app1", "MySQL", "office").unwrap());
        assert!(!exists("app1", "MySQL", "office").unwrap());
        // (refused, with the reason and never the value)
        assert!(save("app1", "", "x", "v").unwrap_err().contains("service is empty"));
        assert!(save("app1", "s", " ", "v").unwrap_err().contains("name is empty"));
        assert!(save("app1", "s", "n", "").unwrap_err().contains("Delete removes"));
        assert!(save("app1", "s", "a\nb", "v").unwrap_err().contains("control character"));
        let long = "x".repeat(MAX_VALUE + 1);
        let e = save("app1", "s", "n", &long).unwrap_err();
        assert!(!e.contains("xxxx"), "{e}");
    }
}

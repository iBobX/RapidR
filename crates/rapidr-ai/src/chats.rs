//! Kept chats (docs/preview4-plan.md R1): an assistant's conversations kept
//! across restarts, per project, so closing Studio or opening the project
//! again never loses one.
//!
//! * **Where**: Studio's data folder, never the project's — a chat is
//!   never shared, committed or shipped with a program by accident. On the
//!   desktop, `RAPIDR_AI_CHATS` when set; else `chats/` beside the
//!   settings store (`RAPIDR_REGISTRY`'s folder, else RapidR's folder of
//!   the user's settings: `~/Library/Application Support/RapidR/chats` on
//!   macOS, `%APPDATA%\RapidR\chats` on Windows, `~/.config/rapidr/chats`
//!   on Linux). One folder a project (its folder's name and a short digest
//!   of its path), one JSON file a chat. On the web, the page's local
//!   storage (`dev.rapidr.chat.<project>.<id>`).
//! * **What**: the chat's title, when it was started and last changed,
//!   the model, and its state (RAI.ChatState: the messages the model is
//!   sent, the transcript shown) — every text passed through [`redact`]
//!   first: an API key or a token a user pasted is never
//!   written.
//! * **Deleting**: [`delete`] removes the file (the item on the web) at
//!   once; nothing is kept elsewhere. Deleting a project's folder of chats
//!   removes them all.

use serde_json::{json, Value};

/// A kept chat, as a list shows it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChatInfo {
    pub id: String,
    pub title: String,
    /// When it was last changed (ISO 8601, UTC).
    pub updated: String,
    pub created: String,
    /// How many questions it has.
    pub questions: u64,
    pub model: String,
    /// The user named it (Rename): its title isn't made from its first
    /// question any more.
    pub named: bool,
}

/// What [`redact`] puts in place of a secret.
pub const REMOVED: &str = "[key removed]";

/// The prefixes keys and tokens start with, and the fewest characters
/// that follow one in a real key (shorter: a word, not a key).
const KEY_PREFIXES: &[(&str, usize)] = &[
    ("sk-ant-", 16),
    ("sk-proj-", 16),
    ("sk-or-", 16),
    ("sk-", 20),
    ("xai-", 20),
    ("gsk_", 20),
    ("ghp_", 20),
    ("gho_", 20),
    ("ghs_", 20),
    ("github_pat_", 20),
    ("glpat-", 16),
    ("hf_", 20),
    ("AIza", 30),
    ("AKIA", 16),
    ("xoxb-", 16),
    ("xoxp-", 16),
];

fn key_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_'
}

/// `text` with every API key and token in it replaced by [`REMOVED`]: the
/// providers' key shapes (`sk-ant-…`, `sk-…`, `xai-…`, GitHub's, Google's,
/// AWS's …), a token after `Bearer ` or an `x-api-key:` / `api_key=` /
/// `access_token=` label, and each of `known` (the key in use, the secrets
/// the program noted) wherever it appears. Code stays as it was: a value
/// counts as a token only when it is long and mixes letters and digits
/// (`ApiKey = KeyEdit.Text` isn't one).
pub fn redact(text: &str, known: &[String]) -> String {
    let mut out = text.to_string();
    // (the exact values first: a key of any shape)
    for k in known.iter().map(|k| k.trim()).filter(|k| k.chars().count() >= 8) {
        if out.contains(k) {
            out = out.replace(k, REMOVED);
        }
    }
    let mut result = String::with_capacity(out.len());
    let chars: Vec<(usize, char)> = out.char_indices().collect();
    let mut i = 0;
    while i < chars.len() {
        let (at, c) = chars[i];
        // (a key starts a word: not in the middle of one)
        let starts_word = i == 0 || !key_char(chars[i - 1].1);
        if starts_word {
            let rest = &out[at..];
            if let Some(len) = key_at(rest) {
                result.push_str(REMOVED);
                let end = at + len;
                while i < chars.len() && chars[i].0 < end {
                    i += 1;
                }
                continue;
            }
            if let Some((label, len)) = labelled_secret(rest) {
                result.push_str(&rest[..label]);
                result.push_str(REMOVED);
                let end = at + label + len;
                while i < chars.len() && chars[i].0 < end {
                    i += 1;
                }
                continue;
            }
        }
        result.push(c);
        i += 1;
    }
    result
}

/// The length of the key `text` starts with, if it starts with one.
fn key_at(text: &str) -> Option<usize> {
    for (prefix, min) in KEY_PREFIXES {
        if let Some(after) = text.strip_prefix(prefix) {
            let n: usize = after.chars().take_while(|c| key_char(*c)).map(char::len_utf8).sum();
            // (a key mixes letters and digits; "sk-the-rest-of-a-sentence" doesn't)
            let body = &after[..n];
            if n >= *min && body.chars().any(|c| c.is_ascii_digit()) && body.chars().any(|c| c.is_ascii_alphabetic()) {
                return Some(prefix.len() + n);
            }
        }
    }
    None
}

/// A label that names a secret and its value: `Bearer xyz…`, `x-api-key:
/// xyz…`, `api_key=xyz…`, `"apiKey": "xyz…"` — (the label's length with
/// what separates it, the value's length).
fn labelled_secret(text: &str) -> Option<(usize, usize)> {
    const LABELS: &[&str] = &["bearer ", "x-api-key", "api_key", "api-key", "apikey", "secret_key", "access_token", "auth_token"];
    let low: String = text.chars().take(16).collect::<String>().to_ascii_lowercase();
    let label = LABELS.iter().find(|l| low.starts_with(*l))?;
    let mut k = label.len();
    let bytes = text.as_bytes();
    if *label != "bearer " {
        // (`"apiKey": "…"`, `api_key = …`, `x-api-key: …`)
        if bytes.get(k) == Some(&b'"') {
            k += 1;
        }
        while bytes.get(k).is_some_and(|b| *b == b' ') {
            k += 1;
        }
        if !matches!(bytes.get(k), Some(b':' | b'=')) {
            return None;
        }
        k += 1;
        while bytes.get(k).is_some_and(|b| *b == b' ' || *b == b'"' || *b == b'\'') {
            k += 1;
        }
    }
    let value: usize = text[k..].chars().take_while(|c| !c.is_whitespace() && !matches!(c, '"' | '\'' | ',' | ';' | ')' | '}' | ']' | '<' | '>')).map(char::len_utf8).sum();
    // (a token is long and mixes letters and digits; code — `KeyEdit1.Text`
    // — isn't one)
    let v = &text[k..k + value];
    let digits = v.chars().filter(char::is_ascii_digit).count();
    let letters = v.chars().filter(char::is_ascii_alphabetic).count();
    (value >= 16 && digits >= 3 && letters >= 3).then_some((k, value))
}

/// Every string in `v` redacted ([`redact`]).
pub fn redact_json(v: &mut Value, known: &[String]) {
    match v {
        Value::String(t) => {
            let r = redact(t, known);
            if r != *t {
                *t = r;
            }
        }
        Value::Array(a) => a.iter_mut().for_each(|x| redact_json(x, known)),
        Value::Object(o) => o.values_mut().for_each(|x| redact_json(x, known)),
        _ => {}
    }
}

/// FNV-1a: a short digest of a path, to tell two projects of the same
/// folder name apart (not a secret's protection).
fn digest(text: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{:08x}", h >> 32)
}

/// The name a project's chats are kept under: its folder's name and a
/// digest of its path ("Calculator-1a2b3c4d"); "no-project" when none is
/// open.
pub fn project_key(project: &str) -> String {
    let p = project.trim().replace('\\', "/");
    let p = p.trim_end_matches('/');
    if p.is_empty() {
        return "no-project".into();
    }
    // (the same folder whatever case a path names it in, where the disk
    // doesn't tell them apart)
    let norm = if cfg!(any(windows, target_os = "macos")) { p.to_lowercase() } else { p.to_string() };
    let name: String = p.rsplit('/').next().unwrap_or(p).chars().filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_')).take(40).collect();
    let name = if name.is_empty() { "project".to_string() } else { name };
    format!("{name}-{}", digest(&norm))
}

/// A new chat's id: when it was made, and a few characters more
/// ("20261022-150405-3fa9").
pub fn new_id() -> String {
    let iso = crate::audit::now_iso();
    let stamp: String = iso.chars().filter(char::is_ascii_digit).collect();
    let (date, time) = stamp.split_at(stamp.len().min(8));
    // (two made in the same second differ: a counter, mixed with the time)
    let mix = u64::from_str_radix(&digest(&iso), 16).unwrap_or(0) ^ next_salt().wrapping_mul(0x9E37_79B9);
    format!("{date}-{}-{:04x}", &time[..time.len().min(6)], mix & 0xffff)
}

fn next_salt() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    N.fetch_add(1, Ordering::Relaxed)
}

/// An id safe as a file name: letters, digits, `-` and `_` only.
fn safe_id(id: &str) -> Option<&str> {
    let id = id.trim();
    (!id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))).then_some(id)
}

fn info_of(v: &Value) -> ChatInfo {
    let s = |k: &str| v[k].as_str().unwrap_or("").to_string();
    ChatInfo { id: s("id"), title: s("title"), updated: s("updated"), created: s("created"), questions: v["questions"].as_u64().unwrap_or(0), model: s("model"), named: v["named"].as_bool().unwrap_or(false) }
}

/// The project's chats, the newest first.
pub fn list(project: &str) -> Vec<ChatInfo> {
    let mut all: Vec<ChatInfo> = store::all(&project_key(project)).iter().filter_map(|t| serde_json::from_str::<Value>(t).ok()).map(|v| info_of(&v)).filter(|c| !c.id.is_empty()).collect();
    all.sort_by(|a, b| b.updated.cmp(&a.updated).then_with(|| b.id.cmp(&a.id)));
    all
}

/// The project's chats whose title or text has `text` in it (any case),
/// the newest first; each with the first line that has it.
pub fn search(project: &str, text: &str) -> Vec<(ChatInfo, String)> {
    let needle = text.trim().to_lowercase();
    let mut out: Vec<(ChatInfo, String)> = Vec::new();
    for t in store::all(&project_key(project)) {
        let Ok(v) = serde_json::from_str::<Value>(&t) else { continue };
        let info = info_of(&v);
        if info.id.is_empty() {
            continue;
        }
        if needle.is_empty() || info.title.to_lowercase().contains(&needle) {
            out.push((info, String::new()));
            continue;
        }
        // (the transcript's entries, as the user saw them)
        let found = v["state"]["log"].as_array().into_iter().flatten().filter_map(|e| e[1].as_str()).flat_map(str::lines).find(|l| l.to_lowercase().contains(&needle));
        if let Some(line) = found {
            let line: String = line.trim().chars().take(120).collect();
            out.push((info, line));
        }
    }
    out.sort_by(|a, b| b.0.updated.cmp(&a.0.updated).then_with(|| b.0.id.cmp(&a.0.id)));
    out
}

/// Keeps chat `id` (a new one when `id` is ""): its title, the model, its
/// state (RAI.ChatState's JSON) — redacted, `known` the secrets to remove
/// whatever their shape. Returns its id.
pub fn save(project: &str, id: &str, title: &str, model: &str, state: &str, known: &[String]) -> Result<String, String> {
    let key = project_key(project);
    let id = if id.trim().is_empty() { new_id() } else { safe_id(id).ok_or("not a chat's id")?.to_string() };
    let mut state: Value = serde_json::from_str(state).map_err(|e| format!("not a chat's state: {e}"))?;
    let before = store::read(&key, &id).and_then(|t| serde_json::from_str::<Value>(&t).ok());
    let now = crate::audit::now_iso();
    let created = before.as_ref().and_then(|b| b["created"].as_str().map(str::to_string)).unwrap_or_else(|| now.clone());
    let named = before.as_ref().and_then(|b| b["named"].as_bool()).unwrap_or(false);
    let title = if named { before.as_ref().and_then(|b| b["title"].as_str()).unwrap_or(title).to_string() } else { title.trim().to_string() };
    let questions = state["marks"].as_array().map_or(0, Vec::len);
    redact_json(&mut state, known);
    let mut v = json!({"version": 1, "id": id, "title": redact(&title, known), "named": named, "project": project.trim(), "created": created, "updated": now, "questions": questions, "model": model, "state": state});
    redact_json(&mut v["project"], known);
    store::write(&key, &id, &v.to_string())?;
    Ok(id)
}

/// Chat `id`'s state (RAI.ChatState's JSON); `None` when there is none.
pub fn load(project: &str, id: &str) -> Option<String> {
    let id = safe_id(id)?;
    let v: Value = serde_json::from_str(&store::read(&project_key(project), id)?).ok()?;
    Some(v["state"].to_string())
}

/// Chat `id` as it is kept (title, dates, state), to put back after a
/// delete (Undo); `None` when there is none.
pub fn raw(project: &str, id: &str) -> Option<String> {
    store::read(&project_key(project), safe_id(id)?)
}

/// Puts back a chat [`raw`] gave (Undo of a delete).
pub fn restore(project: &str, raw: &str) -> Result<String, String> {
    let v: Value = serde_json::from_str(raw).map_err(|e| format!("not a kept chat: {e}"))?;
    let id = v["id"].as_str().and_then(safe_id).ok_or("not a kept chat")?.to_string();
    store::write(&project_key(project), &id, raw)?;
    Ok(id)
}

/// Gives chat `id` the title the user typed (kept from then on).
pub fn rename(project: &str, id: &str, title: &str) -> bool {
    let key = project_key(project);
    let Some(id) = safe_id(id) else { return false };
    let Some(mut v) = store::read(&key, id).and_then(|t| serde_json::from_str::<Value>(&t).ok()) else { return false };
    let title = title.trim();
    if title.is_empty() {
        return false;
    }
    v["title"] = json!(redact(title, &[]));
    v["named"] = json!(true);
    store::write(&key, id, &v.to_string()).is_ok()
}

/// Removes chat `id` for good (its file; on the web, its item).
pub fn delete(project: &str, id: &str) -> bool {
    safe_id(id).is_some_and(|id| store::remove(&project_key(project), id))
}

/// Where the chats are kept, in words (the manual and Settings say it).
pub fn location() -> String {
    store::location()
}

/// A title made from a question: its first line, at most 48 characters,
/// cut at a word.
pub fn title_of(question: &str) -> String {
    let first = question.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("");
    if first.chars().count() <= 48 {
        return first.to_string();
    }
    let cut: String = first.chars().take(48).collect();
    let cut = match cut.rfind(' ') {
        Some(k) if k > 20 => cut[..k].to_string(),
        _ => cut,
    };
    format!("{}…", cut.trim_end_matches([',', '.', ';', ':']))
}

#[cfg(not(target_arch = "wasm32"))]
mod store {
    use std::path::PathBuf;

    /// The folder all chats are under.
    pub fn root() -> Option<PathBuf> {
        if let Some(p) = std::env::var_os("RAPIDR_AI_CHATS").filter(|p| !p.is_empty()) {
            return Some(PathBuf::from(p));
        }
        if let Some(r) = std::env::var_os("RAPIDR_REGISTRY").filter(|p| !p.is_empty()) {
            let r = PathBuf::from(r);
            return Some(r.parent().map(|d| d.join("chats")).unwrap_or_else(|| PathBuf::from("chats")));
        }
        let dir = if cfg!(windows) {
            PathBuf::from(std::env::var_os("APPDATA")?).join("RapidR")
        } else if cfg!(target_os = "macos") {
            PathBuf::from(std::env::var_os("HOME")?).join("Library").join("Application Support").join("RapidR")
        } else {
            let home = std::env::var_os("HOME").map(PathBuf::from);
            std::env::var_os("XDG_CONFIG_HOME").filter(|p| !p.is_empty()).map(PathBuf::from).or_else(|| home.map(|h| h.join(".config")))?.join("rapidr")
        };
        Some(dir.join("chats"))
    }

    fn file(key: &str, id: &str) -> Option<PathBuf> {
        Some(root()?.join(key).join(format!("{id}.json")))
    }

    pub fn all(key: &str) -> Vec<String> {
        let Some(dir) = root().map(|r| r.join(key)) else { return Vec::new() };
        let Ok(rd) = std::fs::read_dir(dir) else { return Vec::new() };
        rd.flatten().filter(|e| e.path().extension().is_some_and(|x| x == "json")).filter_map(|e| std::fs::read_to_string(e.path()).ok()).collect()
    }

    pub fn read(key: &str, id: &str) -> Option<String> {
        std::fs::read_to_string(file(key, id)?).ok()
    }

    pub fn write(key: &str, id: &str, text: &str) -> Result<(), String> {
        let path = file(key, id).ok_or("there is no folder for Studio's data")?;
        // (the chats' folders and files only the user's: SEC-28)
        if let Some(root) = root() {
            crate::private_files::make_dir(&root, true).map_err(|e| format!("can't make {}: {e}", root.display()))?;
        }
        if let Some(dir) = path.parent() {
            crate::private_files::make_dir(dir, true).map_err(|e| format!("can't make {}: {e}", dir.display()))?;
        }
        // (written whole, then put in place: never half a chat)
        let tmp = path.with_extension("json.tmp");
        crate::private_files::write(&tmp, text).map_err(|e| format!("can't write {}: {e}", tmp.display()))?;
        std::fs::rename(&tmp, &path).map_err(|e| format!("can't write {}: {e}", path.display()))
    }

    pub fn remove(key: &str, id: &str) -> bool {
        let Some(path) = file(key, id) else { return false };
        let gone = std::fs::remove_file(&path).is_ok();
        // (a project's folder with no chat left goes too)
        if let Some(dir) = path.parent() {
            let _ = std::fs::remove_dir(dir);
        }
        gone
    }

    pub fn location() -> String {
        let Some(p) = root() else { return "nowhere (no settings folder)".into() };
        if cfg!(unix) {
            if let Some(rest) = std::env::var_os("HOME").filter(|h| !h.is_empty()).and_then(|h| p.strip_prefix(h).ok().map(|r| r.display().to_string())) {
                return format!("~/{rest}");
            }
        }
        p.display().to_string()
    }
}

#[cfg(target_arch = "wasm32")]
mod store {
    const PREFIX: &str = "dev.rapidr.chat.";

    fn storage() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok()?
    }

    pub fn all(key: &str) -> Vec<String> {
        let Some(s) = storage() else { return Vec::new() };
        let want = format!("{PREFIX}{key}.");
        let n = s.length().unwrap_or(0);
        let names: Vec<String> = (0..n).filter_map(|i| s.key(i).ok().flatten()).filter(|k| k.starts_with(&want)).collect();
        names.iter().filter_map(|k| s.get_item(k).ok().flatten()).collect()
    }

    pub fn read(key: &str, id: &str) -> Option<String> {
        storage()?.get_item(&format!("{PREFIX}{key}.{id}")).ok()?
    }

    pub fn write(key: &str, id: &str, text: &str) -> Result<(), String> {
        let s = storage().ok_or("this browser keeps nothing for the page")?;
        s.set_item(&format!("{PREFIX}{key}.{id}"), text).map_err(|_| "this browser's storage for the page is full: delete some chats".to_string())
    }

    pub fn remove(key: &str, id: &str) -> bool {
        storage().is_some_and(|s| s.remove_item(&format!("{PREFIX}{key}.{id}")).is_ok())
    }

    pub fn location() -> String {
        "this browser's storage for the page".into()
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    /// The tests share one process: each runs in its own folder.
    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("rapidr-chats-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn keys_and_tokens_never_reach_the_disk() {
        let anthropic = concat!("sk", "-ant-api03-AbCdEf0123456789xyzXYZ_-0123456789abcdef");
        let openai = concat!("sk", "-proj-0123456789ABCDEFabcdefGHIJ");
        let deepseek = concat!("sk", "-0123456789abcdef0123456789abcdef");
        let text = format!("My key is {anthropic}, the other {openai} and {deepseek}. Header: Authorization: Bearer abcdefghijklmnop0123. x-api-key: {anthropic} api_key=\"QWERTY1234567890qwerty\"");
        let r = redact(&text, &[]);
        for k in [anthropic, openai, deepseek, "abcdefghijklmnop0123", "QWERTY1234567890qwerty"] {
            assert!(!r.contains(k), "{k} left in: {r}");
        }
        assert!(r.starts_with("My key is [key removed], the other [key removed] and [key removed]."), "{r}");
        // (words that only look like the start of one stay)
        let plain = "Use sk-learn, a scikit-learn alias; Bearer of news; AIza is a name; sk-the-rest-of-a-sentence-that-is-long. AI.APIKey = KeyEdit1.Text: APIKeySource = \"keychain:RapidR/x/AI|ask\"";
        assert_eq!(redact(plain, &[]), plain);
        // (a key of no known shape, given as known: removed wherever it is)
        let odd = "zz9-PLURAL-z-alpha-7";
        assert_eq!(redact(&format!("key={odd}!"), &[odd.to_string()]), "key=[key removed]!");
    }

    #[test]
    fn chats_are_kept_per_project_listed_searched_renamed_and_deleted() {
        let dir = scratch("store");
        std::env::set_var("RAPIDR_AI_CHATS", &dir);
        let key = concat!("sk", "-ant-api03-ZZZZ0123456789zzzzZZZZ0123");
        let state = json!({"messages": [{"role": "user", "content": [{"type": "text", "text": format!("Use my key {key} to build a calculator")}]}], "log": [["user", format!("Use my key {key} to build a calculator")], ["assistant", "Done: press Run."]], "marks": [{"log": 0, "msg": 0, "blocks": 0}]}).to_string();
        let a = save("/Users/x/Projects/Calc", "", "Build a calculator", "claude-haiku-5-5", &state, &[]).unwrap();
        let b = save("/Users/x/Projects/Other", "", "Another project's", "m", &state, &[]).unwrap();
        // (a folder per project, under Studio's data folder: never the project's)
        let calc_dir = dir.join(project_key("/Users/x/Projects/Calc"));
        assert!(calc_dir.join(format!("{a}.json")).exists());
        assert!(project_key("/Users/x/Projects/Calc").starts_with("Calc-"));
        assert_ne!(project_key("/a/Calc"), project_key("/b/Calc"));
        assert_eq!(project_key(""), "no-project");
        // (the key never on the disk, in any file)
        for e in std::fs::read_dir(&calc_dir).unwrap().flatten() {
            let text = std::fs::read_to_string(e.path()).unwrap();
            assert!(!text.contains(key) && text.contains(REMOVED), "{text}");
        }
        let listed = list("/Users/x/Projects/Calc");
        assert_eq!(listed.len(), 1);
        assert_eq!((listed[0].id.as_str(), listed[0].title.as_str(), listed[0].questions), (a.as_str(), "Build a calculator", 1));
        assert_eq!(list("/Users/x/Projects/Other")[0].id, b);
        // (the state back, as kept)
        let back: Value = serde_json::from_str(&load("/Users/x/Projects/Calc", &a).unwrap()).unwrap();
        assert_eq!(back["log"][1][1], "Done: press Run.");
        // (search: the title, then the text)
        assert_eq!(search("/Users/x/Projects/Calc", "CALCULATOR").len(), 1);
        let found = search("/Users/x/Projects/Calc", "press run");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].1, "Done: press Run.");
        assert!(search("/Users/x/Projects/Calc", "nothing like it").is_empty());
        // (renamed: the name stays when the chat is saved again)
        assert!(rename("/Users/x/Projects/Calc", &a, "My calculator"));
        save("/Users/x/Projects/Calc", &a, "Build a calculator", "m", &state, &[]).unwrap();
        assert_eq!(list("/Users/x/Projects/Calc")[0].title, "My calculator");
        // (deleted: gone from the disk; Undo puts back what it was)
        let kept = raw("/Users/x/Projects/Calc", &a).unwrap();
        assert!(delete("/Users/x/Projects/Calc", &a));
        assert!(list("/Users/x/Projects/Calc").is_empty());
        assert!(!calc_dir.exists(), "the project's empty folder goes too");
        restore("/Users/x/Projects/Calc", &kept).unwrap();
        assert_eq!(list("/Users/x/Projects/Calc")[0].title, "My calculator");
        // (an id is never a path)
        assert!(load("/Users/x/Projects/Calc", "../../etc/passwd").is_none());
        assert!(!delete("/Users/x/Projects/Calc", "../x"));
        assert!(save("/Users/x/Projects/Calc", "../evil", "t", "m", &state, &[]).is_err());
        std::env::remove_var("RAPIDR_AI_CHATS");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn titles_from_questions() {
        assert_eq!(title_of("  \nBuild me a stopwatch\nwith laps"), "Build me a stopwatch");
        let t = title_of("Please add a Clear button to the form and make the display larger and blue");
        assert!(t.ends_with('…') && t.chars().count() <= 49, "{t}");
        assert!(new_id().len() >= 15);
        assert_ne!(new_id(), new_id());
    }
}

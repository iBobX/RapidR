//! RSQLITE on the web: SQLite itself, compiled to wasm — the components
//! every runtime shares (rapidr-db), with this runtime's properties,
//! events, console and the project's files. A database file is kept in
//! memory for the page's session (a project's `.db` file is copied in when
//! the program first connects to it); nothing is saved to the browser's
//! storage yet. Widgets bound to a component (DataSource / DataField) show
//! its current row and write their edits back.
//!
//! RMySQL is **not implementable in the browser** because the MySQL wire protocol
//! requires raw TCP, which browsers cannot open. `mysql_method` below emits a
//! single clear error and returns a sentinel value so user programs fail loudly
//! instead of silently warning on every call.

use crate::object_web;
use crate::value::{v_int, v_str, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use wasm_bindgen::JsValue;

// ======================================================================
// RMySQL — native-only stub for the web runtime
// ======================================================================

thread_local! {
    static MYSQL_WARNED: RefCell<std::collections::HashSet<String>> =
        RefCell::new(std::collections::HashSet::new());
}

pub fn mysql_method(name: &str, _method: &str, _args: &[Value]) -> Value {
    // Emit the error once per RMySQL instance to avoid log spam.
    let key = name.to_uppercase();
    let already = MYSQL_WARNED.with(|s| {
        let mut set = s.borrow_mut();
        if set.contains(&key) {
            true
        } else {
            set.insert(key.clone());
            false
        }
    });
    if !already {
        web_sys::console::error_1(&JsValue::from_str(&format!(
            "[RapidR] {} (RMySQL) is unavailable on the web runtime: browsers cannot open raw TCP connections to MySQL. Use the native target, or call your backend via RHTTP.",
            name
        )));
    }
    v_int(0)
}

pub fn mysql_get_prop(_name: &str, _prop: &str) -> Value {
    v_str("")
}

pub fn mysql_set_prop(_name: &str, _prop: &str, _val: &Value) {
    // silently accept property assignments (Host=, User=, Password=, …)
    // — they're harmless config and shouldn't error every time.
}

// ======================================================================
// RSQLite
// ======================================================================

struct Web;

impl rapidr_db::Host for Web {
    fn set(&self, name: &str, prop: &str, value: Value) {
        object_web::rp_comp_set(name, prop, value);
    }

    fn get(&self, name: &str, prop: &str) -> Value {
        object_web::rp_comp_get(name, prop)
    }

    fn fire(&self, name: &str, event: &str, args: &[Value]) {
        object_web::rp_fire_event_args(name, event, args);
    }

    fn report(&self, message: &str) {
        let text = JsValue::from_str(message);
        if message.starts_with("[WARN]") {
            web_sys::console::warn_1(&text);
        } else {
            web_sys::console::error_1(&text);
        }
    }

    fn file_bytes(&self, path: &str) -> Option<Vec<u8>> {
        object_web::web_project_file(path)
    }

    fn row_changed(&self, name: &str) {
        sync_bound_widgets(name);
    }
}

pub fn sqlite_method(name: &str, method: &str, args: &[Value]) -> Value {
    rapidr_db::sqlite::method(&Web, name, method, args)
}

/// A widget bound to `db_name` (DataSource / DataField) edited: the row it
/// shows, and the database, take the new value.
pub fn update_bound_data(db_name: &str, field_name: &str, new_val: &str) {
    if let Err(e) = rapidr_db::sqlite::update_bound_field(db_name, field_name, new_val) {
        web_sys::console::error_1(&JsValue::from_str(&format!("[SQLite] {db_name}.{field_name}: {e}")));
    }
}

/// The widgets bound to `db_name` show its current row (nothing without
/// one).
pub fn sync_bound_widgets(db_name: &str) {
    let uname = db_name.to_uppercase();
    let (fields, has_row) = rapidr_db::sqlite::bound_fields(&uname).unwrap_or_default();
    let field_vals: HashMap<String, String> = fields.into_iter().map(|(c, v)| (c.to_uppercase(), v)).collect();
    object_web::rp_sync_bound_widgets(&uname, &field_vals, has_row);
}

// ======================================================================
// The project's files (a bundle's, the IDE's assets)
// ======================================================================

pub(crate) fn get_rapidr_asset(filename: &str) -> Option<String> {
    let window = web_sys::window()?;
    let assets_val = js_sys::Reflect::get(&window, &wasm_bindgen::JsValue::from_str("__rapidr_assets")).ok()?;
    if assets_val.is_undefined() || assets_val.is_null() {
        return None;
    }
    
    let val = js_sys::Reflect::get(&assets_val, &wasm_bindgen::JsValue::from_str(filename)).ok();
    if let Some(v) = val {
        if !v.is_undefined() && !v.is_null() {
            return v.as_string();
        }
    }
    
    let alternative = if filename.starts_with("assets/") {
        filename.strip_prefix("assets/").unwrap()
    } else {
        &format!("assets/{}", filename)
    };
    let val2 = js_sys::Reflect::get(&assets_val, &wasm_bindgen::JsValue::from_str(alternative)).ok();
    if let Some(v) = val2 {
        if !v.is_undefined() && !v.is_null() {
            return v.as_string();
        }
    }
    None
}

/// Decodes base64 (or a `data:…;base64,` URL's payload).
pub fn decode_base64(mut s: &str) -> Option<Vec<u8>> {
    if let Some(pos) = s.find("base64,") {
        s = &s[pos + 7..];
    }
    let s = s.trim();
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    let mut buffer = 0u32;
    let mut bits = 0;
    
    for &b in bytes {
        let val = match b {
            b'A'..=b'Z' => (b - b'A') as u32,
            b'a'..=b'z' => (b - b'a' + 26) as u32,
            b'0'..=b'9' => (b - b'0' + 52) as u32,
            b'+' => 62,
            b'/' => 63,
            b'=' => continue,
            _ if b.is_ascii_whitespace() => continue,
            _ => return None,
        };
        buffer = (buffer << 6) | val;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
        }
    }
    Some(out)
}

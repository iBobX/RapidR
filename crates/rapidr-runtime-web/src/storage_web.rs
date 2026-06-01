//! Web Storage API wrapper — localStorage / sessionStorage.

use crate::value::{v_int, v_null, v_str, Value};

fn get_storage(storage_type: &str) -> Option<web_sys::Storage> {
    let window = web_sys::window()?;
    match storage_type {
        "session" | "sessionstorage" => window.session_storage().ok()?,
        _ => window.local_storage().ok()?,
    }
}

pub fn storage_set(storage_type: &str, key: &str, value: &str) -> Value {
    if let Some(storage) = get_storage(storage_type) {
        let _ = storage.set_item(key, value);
    }
    v_null()
}

pub fn storage_get(storage_type: &str, key: &str) -> Value {
    if let Some(storage) = get_storage(storage_type) {
        match storage.get_item(key) {
            Ok(Some(val)) => return v_str(&val),
            _ => {}
        }
    }
    v_str("")
}

pub fn storage_remove(storage_type: &str, key: &str) -> Value {
    if let Some(storage) = get_storage(storage_type) {
        let _ = storage.remove_item(key);
    }
    v_null()
}

pub fn storage_clear(storage_type: &str) -> Value {
    if let Some(storage) = get_storage(storage_type) {
        let _ = storage.clear();
    }
    v_null()
}

pub fn storage_keys(storage_type: &str) -> Value {
    if let Some(storage) = get_storage(storage_type) {
        let len = storage.length().unwrap_or(0);
        let mut keys = Vec::new();
        for i in 0..len {
            if let Ok(Some(key)) = storage.key(i) {
                keys.push(key);
            }
        }
        return v_str(&keys.join("\n"));
    }
    v_str("")
}

pub fn storage_has_key(storage_type: &str, key: &str) -> Value {
    if let Some(storage) = get_storage(storage_type) {
        match storage.get_item(key) {
            Ok(Some(_)) => return v_int(1),
            _ => {}
        }
    }
    v_int(0)
}

// Session storage variants (legacy wrappers)
pub fn session_storage_set(key: &str, value: &str) -> Value {
    storage_set("session", key, value)
}

pub fn session_storage_get(key: &str) -> Value {
    storage_get("session", key)
}

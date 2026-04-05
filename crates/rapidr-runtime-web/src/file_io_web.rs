//! File I/O stubs for the web runtime.
//!
//! Real file system access is not available in the browser. These stubs
//! emit warnings and return safe defaults so that generated programs that
//! reference file I/O compile and run (gracefully degraded).

use crate::value::{v_int, v_str, Value};
use wasm_bindgen::prelude::*;

fn warn(msg: &str) {
    web_sys::console::warn_1(&JsValue::from_str(msg));
}

pub fn rp_freefile() -> Value {
    warn("[WARN] FREEFILE not supported on web");
    v_int(1)
}

pub fn rp_open(_filename: &Value, _mode: &Value, _file_num: &Value) {
    warn("[WARN] OPEN not supported on web");
}

pub fn rp_close(_file_num: &Value) {
    warn("[WARN] CLOSE not supported on web");
}

pub fn rp_line_input(_file_num: &Value) -> Value {
    warn("[WARN] LINE INPUT # not supported on web");
    v_str("")
}

pub fn rp_print_hash(_file_num: &Value, _items: &[Value]) {
    warn("[WARN] PRINT # not supported on web");
}

pub fn rp_write_hash(_file_num: &Value, _items: &[Value]) {
    warn("[WARN] WRITE # not supported on web");
}

pub fn rp_eof(_file_num: &Value) -> Value {
    v_int(-1) // EOF immediately
}

pub fn rp_lof(_file_num: &Value) -> Value {
    v_int(0)
}

pub fn rp_seek(_file_num: &Value, _position: &Value) {
    warn("[WARN] SEEK not supported on web");
}

pub fn rp_filelen(_filename: &Value) -> Value {
    v_int(0)
}

pub fn rp_dir(_pattern: &Value, _attr: &Value) -> Value {
    warn("[WARN] DIR$ not supported on web");
    v_str("")
}

pub fn rp_mkdir(_path: &Value) {
    warn("[WARN] MKDIR not supported on web");
}

pub fn rp_rmdir(_path: &Value) {
    warn("[WARN] RMDIR not supported on web");
}

pub fn rp_kill(_filename: &Value) {
    warn("[WARN] KILL not supported on web");
}

pub fn rp_rename(_old_name: &Value, _new_name: &Value) {
    warn("[WARN] NAME ... AS not supported on web");
}

pub fn rp_curdir() -> Value {
    v_str("/")
}

pub fn rp_chdir(_path: &Value) {
    warn("[WARN] CHDIR not supported on web");
}

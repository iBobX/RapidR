//! File I/O stubs for the web runtime.
//!
//! Real file system access is not available in the browser. These stubs
//! emit warnings and return safe defaults so that generated programs that
//! reference file I/O compile and run (gracefully degraded).

use crate::value::{v_int, v_str, Value};
/// `RESOURCE(n)`, `RESOURCECOUNT`, `EXTRACTRESOURCE` (shared, rapidr_value::resources).
pub use crate::value::resources::{rp_extractresource, rp_resource, rp_resourcecount};
use wasm_bindgen::prelude::*;

fn warn(msg: &str) {
    web_sys::console::warn_1(&JsValue::from_str(msg));
}

// BASIC file I/O by file number is shared with the desktop runtime and the
// interpreter (rapidr_value::basic_files); the files are the page's own
// (object_web's `web_read_file` / `web_write_file`).
pub use crate::value::basic_files::{close_all as rp_close_all, input_field as rp_input_field};
pub fn rp_freefile() -> Value {
    crate::value::basic_files::freefile()
}
pub fn rp_open(filename: &Value, mode: &Value, file_num: &Value) {
    crate::object_web::install_file_hooks();
    crate::value::basic_files::open(filename, mode, file_num)
}
pub fn rp_close(file_num: &Value) {
    crate::value::basic_files::close(file_num)
}
pub fn rp_line_input(file_num: &Value) -> Value {
    crate::value::basic_files::line_input(file_num)
}
pub fn rp_print_hash(file_num: &Value, items: &[Value]) {
    crate::value::basic_files::print_hash(file_num, items)
}
pub fn rp_write_hash(file_num: &Value, items: &[Value]) {
    crate::value::basic_files::write_hash(file_num, items)
}
pub fn rp_eof(file_num: &Value) -> Value {
    crate::value::basic_files::eof(file_num)
}
pub fn rp_lof(file_num: &Value) -> Value {
    crate::value::basic_files::lof(file_num)
}
pub fn rp_seek(file_num: &Value, position: &Value) {
    crate::value::basic_files::seek(file_num, position)
}

pub fn rp_filelen(filename: &Value) -> Value {
    crate::object_web::install_file_hooks();
    v_int(crate::object_web::web_file_len(&filename.to_string_val()))
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

pub fn rp_kill(filename: &Value) {
    crate::object_web::web_remove_file(&filename.to_string_val());
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

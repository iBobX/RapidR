//! RDATAFRAME's engine as a wasm module of its own (`rapidrframe.js` /
//! `rapidrframe_bg.wasm`, ~1.5 MB brotli): the web runtime loads it before
//! a program that uses data frames starts (rapidr-runtime-web's frame_web.rs)
//! and calls it through these functions — the same `rapidr_frame::call` the
//! desktop links, so a frame answers the same on every runtime.
//!
//! A call goes in as JSON (`rapidr_frame::Call`, with the bytes of the file
//! it reads beside it) and its reply comes back as JSON (`Reply`); a file it
//! writes stays here until [`rapidr_frame_written`] takes it (no bytes
//! through JSON).

use std::cell::RefCell;

use rapidr_frame::{Call, Reply};
use wasm_bindgen::prelude::*;

thread_local! {
    static WRITTEN: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// One RDATAFRAME call: `call` a `rapidr_frame::Call` as JSON, `file` the
/// bytes of the file it reads (`file_error`: why there are none). The
/// reply as JSON (its file's contents taken by [`rapidr_frame_written`]),
/// or `null` for a method RDATAFRAME doesn't have.
#[wasm_bindgen]
pub fn rapidr_frame_call(call: &str, file: Option<Vec<u8>>, file_error: Option<String>) -> String {
    let mut c: Call = match serde_json::from_str(call) {
        Ok(c) => c,
        Err(e) => return reply_json(Reply { value: rapidr_frame::Out::Null, print: None, write: None, grid: None, error: Some(format!("RDataFrame: bad call ({e})")) }),
    };
    c.file = match (file, file_error) {
        (Some(bytes), _) => Some(Ok(bytes)),
        (None, Some(e)) => Some(Err(e)),
        (None, None) => None,
    };
    match rapidr_frame::call(c) {
        Some(reply) => reply_json(reply),
        None => "null".into(),
    }
}

fn reply_json(mut reply: Reply) -> String {
    if let Some((_, bytes)) = reply.write.as_mut() {
        let bytes = std::mem::take(bytes);
        WRITTEN.with(|w| *w.borrow_mut() = bytes);
    }
    serde_json::to_string(&reply).unwrap_or_else(|_| "null".into())
}

/// The contents of the file the last call wrote (`SaveToCsv`, `SaveToJson`).
#[wasm_bindgen]
pub fn rapidr_frame_written() -> Vec<u8> {
    WRITTEN.with(|w| std::mem::take(&mut *w.borrow_mut()))
}

/// RDATAFRAME property `prop` of frame `name`: a `rapidr_frame::Out` as
/// JSON, `null` when there's no such property.
#[wasm_bindgen]
pub fn rapidr_frame_prop(name: &str, prop: &str) -> String {
    rapidr_frame::get_prop(name, prop).and_then(|o| serde_json::to_string(&o).ok()).unwrap_or_else(|| "null".into())
}

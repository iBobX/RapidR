//! RDATAFRAME's engine on the web: `rapidr-frame` (polars) — the very code
//! the desktop links — built as a wasm module of its own
//! (`rapidrframe.js` / `rapidrframe_bg.wasm`, crates/rapidr-frame-web), so
//! pages that don't use data frames never download it (docs/ide-plan.md,
//! D7).
//!
//! It's loaded before a program that uses RDATAFRAME starts
//! (rapidr-vm-host-web's `rapidr_run_bc` awaits [`load`]): by the page's
//! `RAPIDR_FRAME_LOAD` (a function returning a promise of the module, for
//! pages that can't fetch it themselves — the IDE's sandboxed preview), else
//! from the page's `RAPIDR_FRAME` URL, else `rapidrframe.js` beside the page
//! (a bundle's). Calls then go to it synchronously, as JSON (its
//! `rapidr_frame_call`).

use std::cell::RefCell;

use rapidr_frame::{Call, Out, Reply};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

thread_local! {
    /// The module once loaded (its exports).
    static MODULE: RefCell<Option<JsValue>> = const { RefCell::new(None) };
}

/// Whether the module is here.
pub fn loaded() -> bool {
    MODULE.with(|m| m.borrow().is_some())
}

/// Whether a program (its bytecode) uses RDATAFRAME.
pub fn needed(program: &[u8]) -> bool {
    let needle = b"RDATAFRAME";
    program.windows(needle.len()).any(|w| w.eq_ignore_ascii_case(needle))
}

fn global(name: &str) -> Option<JsValue> {
    let v = js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str(name)).ok()?;
    (!v.is_undefined() && !v.is_null()).then_some(v)
}

/// Loads the module (once): the page's `RAPIDR_FRAME_LOAD`, else an import
/// of `RAPIDR_FRAME` or `rapidrframe.js` beside the page.
pub async fn load() -> Result<(), String> {
    if loaded() {
        return Ok(());
    }
    let promise: js_sys::Promise = match global("RAPIDR_FRAME_LOAD").and_then(|f| f.dyn_into::<js_sys::Function>().ok()) {
        Some(f) => f.call0(&JsValue::NULL).map_err(|e| format!("{e:?}"))?.dyn_into().map_err(|_| "RAPIDR_FRAME_LOAD didn't return a promise".to_string())?,
        None => {
            let url = match global("RAPIDR_FRAME").and_then(|v| v.as_string()) {
                Some(u) => u,
                None => {
                    let page = web_sys::window().and_then(|w| w.document()).and_then(|d| d.base_uri().ok().flatten()).unwrap_or_default();
                    web_sys::Url::new_with_base("rapidrframe.js", &page).map(|u| u.href()).unwrap_or_else(|_| "rapidrframe.js".into())
                }
            };
            let import = js_sys::Function::new_with_args("url", "return import(url).then(async (m) => { await m.default(); return m; })");
            import.call1(&JsValue::NULL, &JsValue::from_str(&url)).map_err(|e| format!("{e:?}"))?.dyn_into().map_err(|_| "import didn't return a promise".to_string())?
        }
    };
    let module = JsFuture::from(promise).await.map_err(|e| e.as_string().unwrap_or_else(|| format!("{e:?}")))?;
    MODULE.with(|m| *m.borrow_mut() = Some(module));
    Ok(())
}

fn export(module: &JsValue, name: &str) -> Result<js_sys::Function, String> {
    js_sys::Reflect::get(module, &JsValue::from_str(name)).ok().and_then(|f| f.dyn_into().ok()).ok_or_else(|| format!("the data-frame module has no {name}"))
}

const NOT_LOADED: &str = "RDataFrame: the data-frame module (rapidrframe.js) isn't loaded on this page";

/// One call; `None` for a method RDATAFRAME doesn't have.
pub fn call(call: &Call) -> Option<Reply> {
    let module = match MODULE.with(|m| m.borrow().clone()) {
        Some(m) => m,
        None => return Some(error_reply(NOT_LOADED.into())),
    };
    let run = || -> Result<Option<Reply>, String> {
        let f = export(&module, "rapidr_frame_call")?;
        let json = serde_json::to_string(call).map_err(|e| e.to_string())?;
        let (file, file_error) = match &call.file {
            Some(Ok(bytes)) => (JsValue::from(js_sys::Uint8Array::from(bytes.as_slice())), JsValue::UNDEFINED),
            Some(Err(e)) => (JsValue::UNDEFINED, JsValue::from_str(e)),
            None => (JsValue::UNDEFINED, JsValue::UNDEFINED),
        };
        let out = f.call3(&module, &JsValue::from_str(&json), &file, &file_error).map_err(|e| format!("{e:?}"))?;
        let mut reply: Option<Reply> = serde_json::from_str(&out.as_string().unwrap_or_default()).map_err(|e| e.to_string())?;
        // (a file it wrote: its contents taken apart, not through JSON)
        if let Some((_, bytes)) = reply.as_mut().and_then(|r| r.write.as_mut()) {
            let written = export(&module, "rapidr_frame_written")?.call0(&module).map_err(|e| format!("{e:?}"))?;
            *bytes = js_sys::Uint8Array::new(&written).to_vec();
        }
        Ok(reply)
    };
    run().unwrap_or_else(|e| Some(error_reply(format!("RDataFrame: {e}"))))
}

fn error_reply(e: String) -> Reply {
    Reply { value: Out::Null, print: None, write: None, grid: None, error: Some(e) }
}

/// A property; `None` when there's no such.
pub fn get_prop(name: &str, prop: &str) -> Option<Out> {
    let module = MODULE.with(|m| m.borrow().clone())?;
    let f = export(&module, "rapidr_frame_prop").ok()?;
    let out = f.call2(&module, &JsValue::from_str(name), &JsValue::from_str(prop)).ok()?;
    serde_json::from_str(&out.as_string()?).ok()?
}

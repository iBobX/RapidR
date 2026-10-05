//! The fallback fonts on the kernel's page (docs/web-host-plan.md §3.7,
//! Stage W7): a browser has no system fonts the wasm can draw with, so what
//! the built-in Liberation fonts lack — symbols, Chinese, Japanese, Korean —
//! comes from the Noto chunks `tools/fonts.py` made beside the runtime
//! (`fonts/index.json`: which file has which characters). The first time
//! text has a character no loaded font has (`text::note_missing`), the
//! chunk with it is fetched, added to the kernel's text system, and every
//! window is drawn again with it.
//!
//! Where the chunks are: the page's `RAPIDR_FONTS` (a URL; the IDE's
//! preview and the test pages set it), else `fonts/` beside the page (a
//! bundle's, a `rapidr build --web` site's). Without them (an old build),
//! characters stay boxes, as before.

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

struct Chunk {
    family: String,
    file: String,
    ranges: Vec<(u32, u32)>,
}

#[derive(Default)]
struct State {
    /// `None` until the index came (or failed: then empty).
    chunks: Option<Vec<Chunk>>,
    index_asked: bool,
    /// Characters missing before the index came.
    waiting: Vec<char>,
    /// Chunks asked for (loaded or on their way), by file.
    asked: HashSet<String>,
    /// Characters no chunk has (not looked for again).
    none: HashSet<char>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
    /// Chunks loaded, not yet in the text system (the next frame adds them).
    static LOADED: RefCell<Vec<Vec<u8>>> = const { RefCell::new(Vec::new()) };
}

/// The chunks that came, into the kernel's text system, every window drawn
/// again (the page's frame, the host free).
pub fn apply_loaded() {
    let fonts = LOADED.with(|l| std::mem::take(&mut *l.borrow_mut()));
    if fonts.is_empty() {
        return;
    }
    let done = rapidr_ui_host_web::host::with(|h, _| {
        for data in &fonts {
            h.desk.text.add_font(data.clone());
        }
        h.fonts_changed();
    });
    if done.is_none() {
        // (the host was busy: next frame)
        LOADED.with(|l| l.borrow_mut().extend(fonts));
    }
    rapidr_ui_app::windows::invalidate();
}

/// The fallback fonts' loader installed: the kernel's text tells it what's
/// missing.
pub fn install() {
    rapidr_ui_kernel::text::set_missing_glyph_hook(Some(Rc::new(missing)));
}

fn base() -> String {
    let Some(w) = web_sys::window() else { return "fonts/".into() };
    if let Some(url) = js_sys::Reflect::get(&w, &JsValue::from_str("RAPIDR_FONTS")).ok().and_then(|v| v.as_string()) {
        return if url.ends_with('/') { url } else { format!("{url}/") };
    }
    let page = w.document().and_then(|d| d.base_uri().ok().flatten()).unwrap_or_default();
    web_sys::Url::new_with_base("fonts/", &page).map(|u| u.href()).unwrap_or_else(|_| "fonts/".into())
}

/// A character no loaded font has: its chunk asked for (once).
fn missing(c: char) {
    let file = STATE.with(|s| {
        let mut s = s.borrow_mut();
        if s.none.contains(&c) {
            return None;
        }
        let Some(chunks) = &s.chunks else {
            if !s.waiting.contains(&c) {
                s.waiting.push(c);
            }
            let ask = !s.index_asked;
            s.index_asked = true;
            if ask {
                wasm_bindgen_futures::spawn_local(load_index());
            }
            return None;
        };
        let cp = c as u32;
        let Some(chunk) = chunks.iter().find(|k| k.ranges.iter().any(|&(a, b)| a <= cp && cp <= b)) else {
            s.none.insert(c);
            return None;
        };
        let file = chunk.file.clone();
        s.asked.insert(file.clone()).then_some(file)
    });
    if let Some(file) = file {
        wasm_bindgen_futures::spawn_local(load_chunk(file));
    }
}

/// File `name` of the fallback fonts: from the page's `RAPIDR_FONT_FETCH`
/// (name → a promise of its bytes: the IDE's preview, whose frame can't
/// fetch), else fetched beside the page.
async fn fetch_file(name: &str) -> Option<Vec<u8>> {
    let w = web_sys::window()?;
    if let Some(f) = js_sys::Reflect::get(&w, &JsValue::from_str("RAPIDR_FONT_FETCH")).ok().and_then(|f| f.dyn_into::<js_sys::Function>().ok()) {
        let promise: js_sys::Promise = f.call1(&JsValue::NULL, &JsValue::from_str(name)).ok()?.dyn_into().ok()?;
        let buf = JsFuture::from(promise).await.ok()?;
        return Some(js_sys::Uint8Array::new(&buf).to_vec());
    }
    fetch_bytes(&format!("{}{name}", base())).await
}

async fn fetch_bytes(url: &str) -> Option<Vec<u8>> {
    let w = web_sys::window()?;
    let resp: web_sys::Response = JsFuture::from(w.fetch_with_str(url)).await.ok()?.dyn_into().ok()?;
    if !resp.ok() {
        return None;
    }
    let buf = JsFuture::from(resp.array_buffer().ok()?).await.ok()?;
    Some(js_sys::Uint8Array::new(&buf).to_vec())
}

async fn load_index() {
    let url = format!("{}index.json", base());
    let chunks = match fetch_file("index.json").await.and_then(|b| String::from_utf8(b).ok()) {
        Some(text) => parse_index(&text),
        None => {
            web_sys::console::warn_1(&JsValue::from_str(&format!("[RapidR] no fallback fonts at {url}: characters the built-in fonts lack show as boxes")));
            Vec::new()
        }
    };
    // (the fallback families in the index's order: what draws a character
    // when the face lacks it, once its chunk is here)
    let mut families: Vec<String> = Vec::new();
    for c in &chunks {
        if !families.contains(&c.family) {
            families.push(c.family.clone());
        }
    }
    rapidr_ui_kernel::text::set_fallback_families(families);
    let waiting = STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.chunks = Some(chunks);
        std::mem::take(&mut s.waiting)
    });
    for c in waiting {
        missing(c);
    }
}

/// The index: `{"chunks": [{"family", "file", "ranges": [[first, last] …]} …]}`.
fn parse_index(text: &str) -> Vec<Chunk> {
    let Ok(v) = js_sys::JSON::parse(text) else { return Vec::new() };
    let Ok(list) = js_sys::Reflect::get(&v, &JsValue::from_str("chunks")) else { return Vec::new() };
    let list: js_sys::Array = list.unchecked_into();
    let get = |o: &JsValue, k: &str| js_sys::Reflect::get(o, &JsValue::from_str(k)).ok();
    list.iter()
        .filter_map(|c| {
            let ranges: js_sys::Array = get(&c, "ranges")?.dyn_into().ok()?;
            Some(Chunk {
                family: get(&c, "family")?.as_string()?,
                file: get(&c, "file")?.as_string()?,
                ranges: ranges
                    .iter()
                    .filter_map(|r| {
                        let r: js_sys::Array = r.dyn_into().ok()?;
                        Some((r.get(0).as_f64()? as u32, r.get(1).as_f64()? as u32))
                    })
                    .collect(),
            })
        })
        .collect()
}

async fn load_chunk(file: String) {
    let Some(data) = fetch_file(&file).await else {
        web_sys::console::warn_1(&JsValue::from_str(&format!("[RapidR] could not load the fallback font {}{file}", base())));
        return;
    };
    // (into the kernel's text system at the next frame: every window drawn
    // again)
    LOADED.with(|l| l.borrow_mut().push(data));
    crate::kernel_web::redraw();
}

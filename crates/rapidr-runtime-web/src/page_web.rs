//! The page the runtime runs in: its document and new elements (what the
//! page's own pieces — the Open / Save picker, the tray strip, downloads —
//! are made of; the program's windows are the UI kernel's).

use wasm_bindgen::JsCast;

pub fn document() -> web_sys::Document {
    web_sys::window().expect("no window").document().expect("no document")
}

/// A new element `tag` of the page (not yet in it).
pub fn create_el(tag: &str) -> web_sys::HtmlElement {
    document().create_element(tag).expect("an element").dyn_into::<web_sys::HtmlElement>().expect("an HTML element")
}

/// RGBA pixels as a PNG data URL (an off-screen canvas).
pub fn rgba_data_url(w: usize, h: usize, rgba: &[u8]) -> Option<String> {
    let data = web_sys::ImageData::new_with_u8_clamped_array_and_sh(wasm_bindgen::Clamped(rgba), w as u32, h as u32).ok()?;
    let off = document().create_element("canvas").ok()?.dyn_into::<web_sys::HtmlCanvasElement>().ok()?;
    off.set_width(w as u32);
    off.set_height(h as u32);
    let ctx = off.get_context("2d").ok().flatten()?.dyn_into::<web_sys::CanvasRenderingContext2d>().ok()?;
    ctx.put_image_data(&data, 0.0, 0.0).ok()?;
    off.to_data_url().ok()
}

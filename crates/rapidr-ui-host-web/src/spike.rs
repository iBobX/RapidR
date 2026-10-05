//! The W0 **spike** (docs/web-host-plan.md; feature `spike`): RapidR's UI kernel in the browser —
//! the web counterpart of `rapidr-ui-host-winit`, so a form is the same
//! retained tree, the same input routing, the same display list and the
//! same pixels on the web as on the desktop, instead of a second
//! implementation in DOM elements (the old DOM host, since deleted).
//!
//! - **Drawing.** The kernel paints a [`DisplayList`]; the drawing code
//!   the desktop host uses too (`rapidr-ui-render`: `canvas`, `cpu`,
//!   `images`) rasterizes it with vello_cpu at the page's
//!   `devicePixelRatio`, and the pixels go to a `<canvas>` with
//!   `putImageData`. With feature `gpu`, vello on WebGPU draws the same
//!   list where the browser has it (`gpu_web.rs`, through the same
//!   crate's `gpu`).
//! - **Input.** The page passes pointer, wheel and key events (logical =
//!   CSS pixels, Windows' virtual keys through
//!   `rapidr_value::input::vk_of_key`) to the kernel's input API
//!   ([`FormUi::mouse_down`] …), input methods' compositions as
//!   [`FormUi::ime_preedit`] / [`FormUi::ime_commit`], the clipboard events'
//!   text through a [`MemClipboard`]. Deadlines (the caret's blink) are the
//!   kernel's ([`SpikeForm::wake_in_ms`], [`SpikeForm::tick`]); the page
//!   waits for them with `setTimeout` and draws on `requestAnimationFrame`.
//! - **Accessibility.** The kernel's `AccessNode` tree (the one AccessKit
//!   gets on the desktop) described as ARIA elements ([`aria::mirror`]),
//!   which the page keeps over the canvas.
//! - **No program.** Forms come from a [`MemStore`] ([`forms`]); a few
//!   lines answer their events as the fixtures' handlers do. runtime-core's
//!   `RtStore` and step loop come in the plan's later stages.
//!
//! Everything compiles for the desktop too (wasm-bindgen's exports are
//! inert there), so `cargo check` of the workspace covers it, and
//! `examples/desktop_capture.rs` captures the same forms with the
//! desktop's headless host for the pixel comparison.

use crate::{aria, forms};

use std::cell::RefCell;

use rapidr_ui_kernel::{FormUi, KernelEvent, MemClipboard, MemStore, Mods, TextSystem};
use rapidr_ui_render::{canvas, cpu};
use rapidr_value::input::Button;
use rapidr_value::objects::a11y::Action;
use wasm_bindgen::prelude::*;
use wasm_bindgen::Clamped;
use web_sys::{CanvasRenderingContext2d, ImageData};

pub use rapidr_ui_kernel::DisplayList;

thread_local! {
    /// The fonts and parley's scratch space, shared by every form (as the
    /// desktop's `Desktop::text`).
    static TEXT: RefCell<Option<TextSystem>> = const { RefCell::new(None) };
}

/// Runs `f` with the shared text system (made on first use: the built-in
/// Liberation fonts registered).
pub fn with_text<R>(f: impl FnOnce(&mut TextSystem) -> R) -> R {
    TEXT.with(|t| f(t.borrow_mut().get_or_insert_with(TextSystem::new)))
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = performance, js_name = now)]
    fn performance_now() -> f64;
}

/// Milliseconds on the page's clock (`performance.now()`; 0 off the web).
pub(crate) fn now_ms() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        performance_now()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        0.0
    }
}

/// The modifier keys as the kernel wants them for the platform the page
/// runs on: Cmd is the shortcut key and Option moves by words on a Mac,
/// Ctrl does both elsewhere (as the winit host sets them).
fn mods(shift: bool, ctrl: bool, alt: bool, meta: bool, mac: bool) -> Mods {
    if mac {
        Mods { shift, ctrl, alt, command: meta, word: alt }
    } else {
        Mods { shift, ctrl, alt, command: ctrl, word: ctrl }
    }
}

fn json_str(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// One form of the spike: its store, its kernel tree, its renderer.
#[wasm_bindgen]
pub struct SpikeForm {
    name: String,
    store: MemStore,
    ui: FormUi,
    program: forms::Program,
    scale: f64,
    cpu: Option<cpu::CpuRenderer>,
    /// Draw again even if the kernel says nothing changed (the program
    /// changed the store).
    force: bool,
    /// The platform's modifier convention (`mods`).
    mac: bool,
    /// What the program heard, for the page's log.
    log: Vec<String>,
    /// The last display list (the GPU renderer draws it too).
    last: Option<DisplayList>,
}

#[wasm_bindgen]
impl SpikeForm {
    /// Form `name` (`forms::FORMS`) at `scale` device pixels per CSS
    /// pixel.
    #[wasm_bindgen(constructor)]
    pub fn new(name: &str, scale: f64, mac: bool) -> Result<SpikeForm, JsValue> {
        rapidr_value::objects::bitmap::set_display_scale(scale);
        let store = forms::build(name).ok_or_else(|| JsValue::from_str(&format!("no form {name}")))?;
        let id = forms::form_id(name).unwrap_or(name);
        let ui = FormUi::build(&store, id, true);
        Ok(SpikeForm { name: name.to_string(), store, ui, program: forms::Program::default(), scale, cpu: None, force: true, mac, log: Vec::new(), last: None })
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// The form's caption.
    pub fn caption(&self) -> String {
        rapidr_ui_kernel::store::string(&self.store, &self.ui.form, "caption")
    }

    /// Its client area (an in-window menu bar included): [width, height]
    /// in CSS pixels.
    pub fn size(&self) -> Vec<i32> {
        let (w, h) = self.ui.client;
        vec![w as i32, (h + self.ui.menu_offset) as i32]
    }

    /// The screen's scale changed (`devicePixelRatio`: another monitor,
    /// browser zoom).
    pub fn set_scale(&mut self, scale: f64) {
        if scale != self.scale {
            self.scale = scale;
            rapidr_value::objects::bitmap::set_display_scale(scale);
            self.ui.scale_changed(scale);
            self.force = true;
        }
    }

    /// The caret steady (captures, the pixel comparison), or blinking.
    pub fn set_blinks(&mut self, on: bool) {
        self.ui.blinks = on;
        self.ui.reset_caret();
    }

    /// Whether a frame is due.
    pub fn dirty(&self) -> bool {
        self.force || self.ui.dirty
    }

    /// Paints the form into `ctx` (a canvas of the form's size times the
    /// scale) if it changed, or always with `force`. Returns the frame's
    /// timings as JSON (`{"paint": kernel ms, "raster": vello_cpu ms,
    /// "put": putImageData ms, …}`), or "" when nothing was drawn.
    pub fn render(&mut self, ctx: &CanvasRenderingContext2d, force: bool) -> Result<String, JsValue> {
        if !(force || self.dirty()) {
            return Ok(String::new());
        }
        let t0 = now_ms();
        let list = self.display_list();
        let t1 = now_ms();
        let (w, h) = canvas::device_size(&list);
        let r = self.cpu.get_or_insert_with(|| cpu::CpuRenderer::new(w, h));
        with_text(|ts| r.render(w, h, &list, ts, &self.ui));
        let t2 = now_ms();
        let image = ImageData::new_with_u8_clamped_array_and_sh(Clamped(r.pixmap.data_as_u8_slice()), w, h)?;
        ctx.put_image_data(&image, 0.0, 0.0)?;
        let t3 = now_ms();
        let items = list.items.len();
        self.last = Some(list);
        Ok(format!("{{\"paint\":{:.3},\"raster\":{:.3},\"put\":{:.3},\"w\":{w},\"h\":{h},\"items\":{items}}}", t1 - t0, t2 - t1, t3 - t2))
    }

    /// The last frame's pixels (RGBA, opaque), as drawn.
    pub fn pixels(&self) -> Vec<u8> {
        self.cpu.as_ref().map(|r| r.pixmap.data_as_u8_slice().to_vec()).unwrap_or_default()
    }

    // ---- input (CSS pixels of the canvas = the kernel's logical pixels) ----

    pub fn pointer_down(&mut self, x: f64, y: f64, button: i16, shift: bool, ctrl: bool, alt: bool, meta: bool) {
        let m = mods(shift, ctrl, alt, meta, self.mac);
        with_text(|ts| self.ui.mouse_down(&self.store, ts, x, y, Button::from_dom(button), m));
        self.after_input();
    }

    pub fn pointer_move(&mut self, x: f64, y: f64, shift: bool, ctrl: bool, alt: bool, meta: bool) {
        let m = mods(shift, ctrl, alt, meta, self.mac);
        with_text(|ts| self.ui.mouse_move(&self.store, ts, x, y, m));
        self.after_input();
    }

    /// A button let go; the text an edit's context menu copied or cut, if
    /// it did (the page puts it on the clipboard).
    pub fn pointer_up(&mut self, x: f64, y: f64, button: i16, shift: bool, ctrl: bool, alt: bool, meta: bool, clipboard: Option<String>) -> Option<String> {
        let m = mods(shift, ctrl, alt, meta, self.mac);
        let mut clip = MemClipboard(clipboard.clone());
        with_text(|ts| {
            self.ui.mouse_up(&self.store, ts, x, y, Button::from_dom(button), m);
            // (an edit's context menu picked: Cut, Copy, Paste …)
            self.ui.edit_commands(&self.store, ts, &mut clip);
        });
        self.after_input();
        clip.0.filter(|t| Some(t) != clipboard.as_ref())
    }

    pub fn pointer_leave(&mut self) {
        with_text(|ts| self.ui.mouse_leave(&self.store, ts));
        self.after_input();
    }

    /// The wheel turned `dx`, `dy` notches (positive: right, down).
    pub fn wheel(&mut self, x: f64, y: f64, dx: f64, dy: f64, shift: bool, ctrl: bool, alt: bool, meta: bool) {
        let m = mods(shift, ctrl, alt, meta, self.mac);
        with_text(|ts| self.ui.mouse_wheel(&self.store, ts, (x, y), (dx, dy), m));
        self.after_input();
    }

    /// A key pressed (`KeyboardEvent.key` / `.code`); `text` what it types
    /// (empty: nothing). Whether the kernel knew the key (the page then
    /// keeps the browser from acting on it).
    pub fn key_down(&mut self, key: &str, code: &str, text: &str, shift: bool, ctrl: bool, alt: bool, meta: bool) -> bool {
        let Some(vk) = rapidr_value::input::vk_of_key(key, code) else { return false };
        let m = mods(shift, ctrl, alt, meta, self.mac);
        let mut clip = MemClipboard::default();
        with_text(|ts| self.ui.key_down(&self.store, ts, vk, text, m, &mut clip));
        self.after_input();
        true
    }

    pub fn key_up(&mut self, key: &str, code: &str, shift: bool, ctrl: bool, alt: bool, meta: bool) {
        let Some(vk) = rapidr_value::input::vk_of_key(key, code) else { return };
        self.ui.key_up(vk, mods(shift, ctrl, alt, meta, self.mac));
        self.after_input();
    }

    /// The shortcut key (Cmd / Ctrl) + `letter` with the clipboard's text
    /// `clipboard` (Paste), as the clipboard events give it; the text the
    /// key put on the clipboard (Copy, Cut), if any.
    pub fn clipboard_key(&mut self, letter: &str, clipboard: Option<String>) -> Option<String> {
        let vk = rapidr_value::input::vk_of_key(letter, "")?;
        let m = if self.mac { Mods { command: true, ..Mods::NONE } } else { Mods { ctrl: true, command: true, word: true, ..Mods::NONE } };
        let mut clip = MemClipboard(clipboard);
        with_text(|ts| self.ui.key_down(&self.store, ts, vk, "", m, &mut clip));
        self.ui.key_up(vk, m);
        self.after_input();
        clip.0
    }

    /// An input method's composition (`start`..`end`: its cursor, in bytes
    /// of `text`).
    pub fn ime_preedit(&mut self, text: &str, start: usize, end: usize) {
        let cursor = (!text.is_empty()).then_some((start.min(text.len()), end.min(text.len())));
        with_text(|ts| self.ui.ime_preedit(&self.store, ts, text, cursor));
        self.after_input();
    }

    /// Text committed by an input method, a mobile keyboard or a paste.
    pub fn ime_commit(&mut self, text: &str) {
        with_text(|ts| {
            self.ui.ime_preedit(&self.store, ts, "", None);
            self.ui.ime_commit(&self.store, ts, text);
        });
        self.after_input();
    }

    /// A screen reader's request on node `node` ("click", "focus",
    /// "increment" …), as the user's input would be.
    pub fn access_action(&mut self, node: &str, action: &str) -> bool {
        let Ok(target) = node.parse::<u64>() else { return false };
        let action = match action {
            "click" => Action::Click,
            "focus" => Action::Focus,
            "increment" => Action::Increment,
            "decrement" => Action::Decrement,
            "expand" => Action::Expand,
            "collapse" => Action::Collapse,
            _ => return false,
        };
        let done = with_text(|ts| self.ui.access_action(&self.store, ts, target, action, None));
        self.after_input();
        done
    }

    // ---- time ----

    /// Milliseconds until the kernel's next deadline (the caret's blink, a
    /// held scroll bar's repeat), -1 for none.
    pub fn wake_in_ms(&self) -> f64 {
        let now = rapidr_ui_kernel::tick::now();
        self.ui.next_wake().map_or(-1.0, |at| at.saturating_duration_since(now).as_secs_f64() * 1000.0)
    }

    /// Runs what's due.
    pub fn tick(&mut self) {
        let now = rapidr_ui_kernel::tick::now();
        with_text(|ts| self.ui.tick(&self.store, ts, now));
        self.after_input();
    }

    // ---- what the page mirrors ----

    /// The accessibility tree as the mirror's elements (`aria::mirror`).
    pub fn aria(&mut self) -> String {
        let tree = with_text(|ts| self.ui.access_tree(&self.store, ts));
        aria::mirror(&tree)
    }

    /// The accessibility tree as `RAPIDR_TEST_A11Y` writes it (what
    /// AccessKit gets on the desktop).
    pub fn access_json(&mut self) -> String {
        with_text(|ts| self.ui.access_tree(&self.store, ts)).to_json()
    }

    /// The focused component and, if it's an editor, what the page's text
    /// field mirrors: `{"id", "ime": bool, "text", "start", "end"
    /// (UTF-16, as a textarea counts), "caret": [x, y, w, h] | null}`.
    pub fn focus_info(&mut self) -> String {
        let id = self.ui.focused().unwrap_or("").to_string();
        let ime = self.ui.wants_ime(&self.store);
        let edit = rapidr_value::objects::with_textedit(&id, |t| (t.raw(), t.sel_start, t.sel_len));
        let caret = with_text(|ts| self.ui.ime_area(&self.store, ts));
        let (text, start, end) = match &edit {
            Some((raw, s, l)) => {
                let utf16 = |chars: usize| raw.chars().take(chars).map(char::len_utf16).sum::<usize>();
                (raw.as_str(), utf16(*s), utf16(s + l))
            }
            None => ("", 0, 0),
        };
        let caret = caret.map_or("null".to_string(), |(x, y, w, h)| format!("[{x},{y},{w},{h}]"));
        format!("{{\"id\":{},\"ime\":{ime},\"text\":{},\"start\":{start},\"end\":{end},\"caret\":{caret}}}", json_str(&id), json_str(text))
    }

    /// What the program heard since the last call, one line each.
    pub fn take_log(&mut self) -> Vec<String> {
        std::mem::take(&mut self.log)
    }

    /// The last display list, one line per item (for debugging).
    pub fn dump(&self) -> String {
        self.last.as_ref().map(DisplayList::dump).unwrap_or_default()
    }
}

impl SpikeForm {
    /// The form painted now (the kernel's display list).
    pub fn display_list(&mut self) -> DisplayList {
        rapidr_value::objects::bitmap::set_display_scale(self.scale);
        self.force = false;
        with_text(|ts| self.ui.paint(&self.store, ts, self.scale))
    }

    pub fn ui(&self) -> &FormUi {
        &self.ui
    }

    /// The kernel's events to the program (the runtime's dispatch, in
    /// short); a frame is due when it heard anything.
    fn after_input(&mut self) {
        let events = self.ui.take_events();
        for e in &events {
            if !matches!(e, KernelEvent::Mouse { kind: rapidr_value::input::Mouse::Move, .. }) {
                self.log.push(format!("{e:?}"));
            }
            self.program.dispatch(&mut self.store, e);
        }
        if !events.is_empty() {
            self.force = true;
        }
    }
}

/// The spike's form names.
#[wasm_bindgen]
pub fn form_names() -> Vec<String> {
    forms::FORMS.iter().map(|(n, _)| n.to_string()).collect()
}

/// Registers the fonts now (the first frame's cost measured apart); the
/// time it took, in ms.
#[wasm_bindgen]
pub fn warm_up() -> f64 {
    let t0 = now_ms();
    with_text(|_| ());
    now_ms() - t0
}

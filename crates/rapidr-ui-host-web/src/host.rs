//! The web host (docs/web-host-plan.md §3, Stage W3): the program's forms
//! as windows on the page — each one a `<div>` holding a canvas the kernel
//! draws its frame on (title bar, border, buttons: [`crate::frame`]), a
//! canvas for its client area (the very display list and CPU renderer the
//! desktop's capture uses: the same pixels) and the accessibility mirror
//! over it ([`crate::mirror`]). The forms' kernel sides, stacking, the
//! modal list and the queues are `rapidr_ui_app::desktop`'s `Desktop` — the
//! desktop host's own.
//!
//! **Callbacks never run program code** (the desktop's rule): a DOM
//! listener routes the input into the kernel through the `Desktop`'s entry
//! points (hit test, capture, focus, the models; [`Source::User`], dropped
//! for a window under a modal one or under a test) and then calls the
//! runtime's `wake` hook, which dispatches the queued events (the program's
//! handlers) with this host *not* borrowed. The runtime carries out the
//! program's window commands ([`run_cmds`]), draws the dirty forms on the
//! page's animation frame ([`render`]) and arms one timer for the kernel's
//! deadlines ([`next_wake`]).
//!
//! The page is the screen: Screen.Width / Height are the viewport's, a
//! window's Left / Top its place on the page; windows are stacked, moved
//! (their title bar dragged), sized (their right and bottom edges),
//! maximized, minimized and closed on the page as on a desktop. A screen
//! scale change (another monitor, the browser's zoom) draws every window
//! again at the new `devicePixelRatio` (OnScaleChanged); a canvas whose
//! backing the browser dropped (`contextlost`) is drawn again when it comes
//! back.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

use rapidr_ui_app::desktop::{Desktop, HostCmd, Source};
use rapidr_ui_kernel::tick::Instant;
use rapidr_ui_kernel::{Clipboard, Mods, Store};
use rapidr_ui_render::cpu::CpuRenderer;
use rapidr_value::input::Button;
use rapidr_value::objects::a11y::Action;
use rapidr_value::objects::codec::Pixels;
use wasm_bindgen::prelude::*;
use wasm_bindgen::{Clamped, JsCast};
use web_sys::{CanvasRenderingContext2d, Document, Element, HtmlCanvasElement, HtmlElement, ImageData};

use crate::frame::{self, Look, Part};
use crate::mirror::Mirror;

type Listener = Closure<dyn FnMut(web_sys::Event)>;

/// The page's clipboard as the kernel's edits use it: the text the
/// clipboard events last carried (copy, cut, paste) — the browser lets a
/// page read the system's clipboard only inside a paste event.
#[derive(Clone, Default)]
struct PageClipboard(Rc<RefCell<Option<String>>>);

impl Clipboard for PageClipboard {
    fn get_text(&mut self) -> Option<String> {
        self.0.borrow().clone()
    }
    fn set_text(&mut self, text: &str) {
        *self.0.borrow_mut() = Some(text.to_string());
    }
}

/// A drag on a window's frame: by its title bar (moved) or an edge
/// (resized); the pointer's place on the page then, and the window's place
/// (or inside size) then.
#[derive(Clone, Copy)]
struct Drag {
    /// The edges dragged (left, top, right, bottom); `None`: the title bar.
    resize: Option<(bool, bool, bool, bool)>,
    from: (f64, f64),
    /// The window's place on the page then.
    start: (i64, i64),
    /// Its inside's size then (an edge's drag).
    size: (i64, i64),
}

/// The edges a window is resized by (as Windows': every side and corner),
/// with their cursors.
/// A minimized window's width (its title bar's).
const MIN_WIDTH: i64 = 160;

const EDGES: [(&str, &str); 8] = [("e", "ew-resize"), ("s", "ns-resize"), ("se", "nwse-resize"), ("w", "ew-resize"), ("n", "ns-resize"), ("nw", "nwse-resize"), ("ne", "nesw-resize"), ("sw", "nesw-resize")];

/// One form's window on the page.
struct Win {
    root: HtmlElement,
    frame: HtmlCanvasElement,
    fctx: CanvasRenderingContext2d,
    client: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
    mirror: Mirror,
    /// The web-only components' elements over the client canvas (§3.6),
    /// clipped to it.
    overlays: HtmlElement,
    /// Over those, the open drop-down list and menus (the kernel draws
    /// them apart while the form has such elements: `popups_apart`); shown
    /// only while one is open.
    popups: HtmlCanvasElement,
    pctx: CanvasRenderingContext2d,
    pcpu: Option<CpuRenderer>,
    grips: Vec<HtmlElement>,
    cpu: Option<CpuRenderer>,
    fcpu: Option<CpuRenderer>,
    /// The inside's size (logical) and the scale last laid out.
    inside: (i64, i64),
    scale: f64,
    look: Option<Look>,
    frame_dirty: bool,
    /// The client canvas must be drawn again whatever the kernel says (its
    /// backing came back, the scale changed).
    force: bool,
    drag: Option<Drag>,
    /// The title bar button pressed.
    pressed: Option<Part>,
    /// Minimized (only its title bar shows).
    minimized: bool,
    /// Where a minimized window sits along the page's bottom edge (as
    /// Windows' minimized windows line up): its slot, from the left.
    min_slot: Option<usize>,
    /// Its place and inside before it was maximized.
    normal: Option<((i64, i64), (i64, i64))>,
    _listeners: Vec<Listener>,
}

/// The web host: the program's windows on the page.
pub struct WebHost {
    pub desk: Desktop,
    wins: BTreeMap<String, Win>,
    /// The page's devicePixelRatio (every window's scale).
    pub dpr: f64,
    mac: bool,
    clip: PageClipboard,
    doc: Document,
    /// The pointer on the page, as last seen (MOUSEX / MOUSEY).
    pub mouse: (f64, f64),
    /// Open / Save dialogs' answers (W8: the page's file dialogs; until
    /// then every one is cancelled).
    files: HashMap<u64, Vec<String>>,
}

thread_local! {
    static HOST: RefCell<Option<WebHost>> = const { RefCell::new(None) };
    static STORE: Cell<Option<&'static dyn Store>> = const { Cell::new(None) };
    static WAKE: RefCell<Option<Rc<dyn Fn()>>> = const { RefCell::new(None) };
    /// (RapidR's OnDropFiles) Whether a form takes files dropped on it (it
    /// has an OnDropFiles handler): the runtime's answer.
    static ACCEPTS_DROP: RefCell<Option<Rc<dyn Fn(&str) -> bool>>> = const { RefCell::new(None) };
    static SCALE_WATCH: RefCell<Option<Listener>> = const { RefCell::new(None) };
    /// The component types the page shows as its own elements over the
    /// canvas (the runtime's web-only components), uppercase.
    static OVERLAY_TYPES: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// The component types whose elements (`#rr-<name>`, marked
/// `data-rr-overlay`) the host places over its windows' canvases at their
/// nodes' places, clipped to their parents (docs/web-host-plan.md §3.6).
pub fn set_overlay_types(types: &[&str]) {
    OVERLAY_TYPES.with(|o| *o.borrow_mut() = types.iter().map(|t| t.to_uppercase()).collect());
}

/// The performance harness's probe (docs/ide-plan.md §6.2): when the page
/// set `window.RAPIDR_FRAME_TIMES` to an array, each drawn window frame
/// appends [the frame's work in ms, Date.now() when it ended].
fn frame_time(started: Instant) {
    let Some(win) = web_sys::window() else { return };
    let Ok(list) = js_sys::Reflect::get(&win, &JsValue::from_str("RAPIDR_FRAME_TIMES")) else { return };
    if !js_sys::Array::is_array(&list) {
        return;
    }
    let ms = started.elapsed().as_secs_f64() * 1000.0;
    let now = js_sys::Date::now();
    let entry = js_sys::Array::of2(&JsValue::from_f64(ms), &JsValue::from_f64(now));
    list.unchecked_into::<js_sys::Array>().push(&entry);
}

fn is_overlay(type_name: &str) -> bool {
    OVERLAY_TYPES.with(|o| o.borrow().iter().any(|t| t.eq_ignore_ascii_case(type_name)))
}

const STYLE: &str = r#"
.rr-kwin { position: absolute; box-sizing: content-box; outline: none; user-select: none; -webkit-user-select: none; }
.rr-kwin canvas { position: absolute; display: block; touch-action: none; }
.rr-kwin .rr-kgrip { position: absolute; background: transparent; touch-action: none; }
.rr-kwin .rr-koverlays { position: absolute; overflow: hidden; pointer-events: none; }
.rr-kwin .rr-koverlays > * { pointer-events: auto; }
.rr-a11y, .rr-a11y * { position: absolute; margin: 0; padding: 0; border: 0; background: transparent; color: transparent;
  outline: none; overflow: hidden; pointer-events: none; white-space: pre; box-sizing: border-box; }
.rr-a11y input, .rr-a11y textarea { caret-color: transparent; resize: none; padding: 1px 3px; font: 13px "Liberation Sans", Arial, sans-serif; line-height: 15px; }
.rr-a11y ::selection { background: transparent; color: transparent; }
"#;

fn window() -> web_sys::Window {
    web_sys::window().expect("a page")
}

fn px(v: f64) -> String {
    format!("{v}px")
}

fn set_style(el: &HtmlElement, props: &[(&str, String)]) {
    let st = el.style();
    for (k, v) in props {
        if st.get_property_value(k).ok().as_deref() != Some(v.as_str()) {
            let _ = st.set_property(k, v);
        }
    }
}

/// The modifier keys as the kernel wants them on the platform the page runs
/// on: Cmd is the shortcut key and Option moves by words on a Mac, Ctrl
/// does both elsewhere (as the winit host sets them).
fn mods_of(shift: bool, ctrl: bool, alt: bool, meta: bool, mac: bool) -> Mods {
    if mac {
        Mods { shift, ctrl, alt, command: meta, word: alt }
    } else {
        Mods { shift, ctrl, alt, command: ctrl, word: ctrl }
    }
}

fn mouse_mods(e: &web_sys::MouseEvent, mac: bool) -> Mods {
    mods_of(e.shift_key(), e.ctrl_key(), e.alt_key(), e.meta_key(), mac)
}

/// The shortcut key alone (the clipboard keys).
fn command(mac: bool) -> Mods {
    if mac {
        Mods { command: true, ..Mods::NONE }
    } else {
        Mods { ctrl: true, command: true, word: true, ..Mods::NONE }
    }
}

/// The web host installed on the page: the program's components are read
/// through `store`; `wake` is the runtime's turn after input (it dispatches
/// what the kernel queued and asks for a frame). Once per page.
pub fn install(store: &'static dyn Store, wake: Rc<dyn Fn()>) {
    STORE.with(|s| s.set(Some(store)));
    WAKE.with(|w| *w.borrow_mut() = Some(wake));
    HOST.with(|h| {
        let mut h = h.borrow_mut();
        if h.is_some() {
            return;
        }
        let doc = window().document().expect("a document");
        // (the windows' and the mirror's look)
        if doc.get_element_by_id("rr-kernel-style").is_none() {
            if let Ok(style) = doc.create_element("style") {
                style.set_id("rr-kernel-style");
                style.set_text_content(Some(STYLE));
                if let Some(head) = doc.head() {
                    let _ = head.append_child(&style);
                }
            }
        }
        let clip = PageClipboard::default();
        let desk = Desktop::new(Box::new(clip.clone()));
        let mac = window().navigator().platform().map(|p| p.contains("Mac") || p.contains("iP")).unwrap_or(false);
        *h = Some(WebHost { desk, wins: BTreeMap::new(), dpr: window().device_pixel_ratio().max(0.25), mac, clip, doc, mouse: (0.0, 0.0), files: HashMap::new() });
    });
    watch_scale();
}

pub fn installed() -> bool {
    HOST.with(|h| h.borrow().is_some())
}

/// (RapidR's OnDropFiles) `accepts(form)`: whether files dropped on the
/// form's window are the program's (else the drop is refused — the page
/// isn't replaced by the file either).
pub fn on_drop_files(accepts: Rc<dyn Fn(&str) -> bool>) {
    ACCEPTS_DROP.with(|a| *a.borrow_mut() = Some(accepts));
}

fn accepts_drop(form: &str) -> bool {
    ACCEPTS_DROP.with(|a| a.borrow().clone()).is_some_and(|f| f(form))
}

/// Files dropped on form `id`'s window: each read whole into the page's
/// files under its name (as a file the user picks in an Open dialog), then
/// one OnDropFiles with their names, in order.
fn drop_files(id: &str, list: web_sys::FileList) {
    let files: Vec<web_sys::File> = (0..list.length()).filter_map(|i| list.get(i)).collect();
    if files.is_empty() {
        return;
    }
    let reads = js_sys::Array::new();
    for f in &files {
        reads.push(&f.array_buffer());
    }
    let names: Vec<String> = files.iter().map(web_sys::File::name).collect();
    let id = id.to_string();
    let done = Closure::once(move |buffers: JsValue| {
        let buffers = js_sys::Array::from(&buffers);
        let mut stored = Vec::new();
        for (i, name) in names.iter().enumerate() {
            let bytes = js_sys::Uint8Array::new(&buffers.get(i as u32)).to_vec();
            if rapidr_value::objects::write_file(name, &bytes).is_ok() {
                stored.push(name.clone());
            }
        }
        input(|h, _| {
            for name in &stored {
                h.desk.files_dropped(&id, name);
            }
        });
    });
    let failed = Closure::once(|e: JsValue| {
        web_sys::console::warn_1(&JsValue::from_str(&format!("[rapidr] dropped files: can't read them ({e:?})")));
    });
    let _ = js_sys::Promise::all(&reads).then2(&done, &failed);
    done.forget();
    failed.forget();
}

fn store() -> Option<&'static dyn Store> {
    STORE.with(Cell::get)
}

/// Runs `f` on the host and the program's store (`None` before it's
/// installed, or while it's busy: a host callback on the stack).
pub fn with<R>(f: impl FnOnce(&mut WebHost, &'static dyn Store) -> R) -> Option<R> {
    let store = store()?;
    HOST.with(|h| {
        let mut h = h.try_borrow_mut().ok()?;
        let h = h.as_mut()?;
        Some(f(h, store))
    })
}

/// The runtime's turn (after input; nothing borrowed).
fn wake() {
    if let Some(w) = WAKE.with(|w| w.borrow().clone()) {
        w();
    }
}

/// Input into a window's kernel, then the runtime's turn. An event the
/// browser sends while the host is busy (a focus moved by the mirror's own
/// sync) is the host's doing, not the user's: dropped.
fn input(f: impl FnOnce(&mut WebHost, &'static dyn Store)) {
    if with(f).is_some() {
        wake();
    }
}

/// Whether the host is busy (a callback of its own on the stack): the
/// runtime's turn waits.
pub fn busy() -> bool {
    HOST.with(|h| h.try_borrow_mut().is_err())
}

/// The screen: the page's viewport (logical = CSS pixels).
pub fn screen() -> (i64, i64) {
    let w = window();
    let get = |v: Result<JsValue, JsValue>| v.ok().and_then(|v| v.as_f64()).unwrap_or(0.0) as i64;
    (get(w.inner_width()).max(1), get(w.inner_height()).max(1))
}

/// The page's devicePixelRatio, re-read when it changes (another monitor,
/// the browser's zoom): every window is drawn again at it and hears
/// OnScaleChanged.
fn watch_scale() {
    let dpr = window().device_pixel_ratio();
    let Ok(Some(mq)) = window().match_media(&format!("(resolution: {dpr}dppx)")) else { return };
    let cb = Closure::<dyn FnMut(web_sys::Event)>::new(move |_e: web_sys::Event| {
        let dpr = window().device_pixel_ratio().max(0.25);
        input(|h, _store| {
            h.dpr = dpr;
            let ids: Vec<String> = h.wins.keys().cloned().collect();
            for id in ids {
                h.desk.scale_changed(&id, dpr);
                if let Some(w) = h.wins.get_mut(&id) {
                    w.force = true;
                    w.frame_dirty = true;
                }
            }
        });
        watch_scale();
    });
    let opts = web_sys::AddEventListenerOptions::new();
    opts.set_once(true);
    let _ = mq.add_event_listener_with_callback_and_add_event_listener_options("change", cb.as_ref().unchecked_ref(), &opts);
    SCALE_WATCH.with(|s| *s.borrow_mut() = Some(cb));
}

impl WebHost {
    /// The shown forms' next deadline (a caret's blink, a held scroll bar).
    pub fn next_wake(&self) -> Option<Instant> {
        self.desk.next_wake()
    }

    /// Whether a frame is due: a shown form the kernel or the host must
    /// draw again.
    pub fn frame_due(&self) -> bool {
        self.wins.iter().any(|(id, w)| w.root.style().get_property_value("display").ok().as_deref() != Some("none") && (w.force || w.frame_dirty || self.desk.forms.get(id).is_some_and(|f| f.ui.dirty)))
    }

    /// The window on top: the active one (it has the keyboard).
    fn top(&self) -> Option<String> {
        self.desk.stacking().last().cloned()
    }

    /// Open / Save dialog `id`'s answer (W8: cancelled until the page's).
    pub fn file_answer(&mut self, id: u64) -> Option<Vec<String>> {
        Some(self.files.remove(&id).unwrap_or_default())
    }

    // ------------------------------------------------- window commands --

    /// The program's window commands carried out on the page (`Desktop`'s
    /// queue: made, shown on top, hidden, titled, sized, moved, maximized
    /// …); the user's own changes come back as the kernel's events.
    pub fn run_cmds(&mut self, store: &dyn Store) {
        let before = self.top();
        for cmd in std::mem::take(&mut self.desk.cmds) {
            match cmd {
                HostCmd::Show(f) => self.show(&f),
                HostCmd::Hide(f) => {
                    if let Some(w) = self.wins.get(&f) {
                        let _ = w.root.style().set_property("display", "none");
                    }
                }
                HostCmd::Title(f) | HostCmd::Icon(f) => self.frame_dirty(&f),
                HostCmd::Border(f) => {
                    if let Some(w) = self.wins.get_mut(&f) {
                        w.frame_dirty = true;
                        w.force = true;
                    }
                }
                HostCmd::Shape(f) => self.apply_shape(&f),
                HostCmd::Size(f) => {
                    if let Some(w) = self.wins.get_mut(&f) {
                        w.force = true;
                    }
                }
                HostCmd::Position(f) => self.place(&f),
                HostCmd::Minimize(f) => self.set_state(store, &f, rapidr_value::window_state::WS_MINIMIZED),
                HostCmd::State(f) => {
                    let state = self.desk.forms.get(&f).map_or(0, |w| w.spec.state);
                    self.set_state(store, &f, state);
                }
                // (pop-up menus are the kernel's on the page)
                HostCmd::Popup { .. } => {}
                // (W8: the page's file dialogs; cancelled until then)
                HostCmd::FileDialog { id, .. } => {
                    self.files.insert(id, Vec::new());
                }
                HostCmd::Forget(f) => {
                    if let Some(w) = self.wins.remove(&f) {
                        w.root.remove();
                    }
                }
                HostCmd::Resize { form, w, h } => self.desk.resized(&form, w, h),
                HostCmd::Fullscreen(f) => self.set_state(store, &f, rapidr_value::window_state::WS_MAXIMIZED),
                // (Form.Modified: the page's beforeunload reads it from the
                // forms when the page is being left — runtime-web)
                HostCmd::Modified(_) => {}
            }
        }
        // (z-order, and the active window's frame)
        let order = self.desk.stacking();
        for (i, id) in order.iter().enumerate() {
            if let Some(w) = self.wins.get(id) {
                let _ = w.root.style().set_property("z-index", &(10 + i).to_string());
            }
        }
        let after = self.top();
        if before != after {
            for id in [before, after].into_iter().flatten() {
                self.frame_dirty(&id);
            }
        }
    }

    fn frame_dirty(&mut self, id: &str) {
        if let Some(w) = self.wins.get_mut(id) {
            w.frame_dirty = true;
        }
    }

    /// Form `id`'s window shown on top (made the first time).
    fn show(&mut self, id: &str) {
        if !self.wins.contains_key(id) {
            if let Some(w) = self.make(id) {
                self.wins.insert(id.to_string(), w);
                // (an outline ShapeForm gave it before it showed)
                self.apply_shape(id);
            } else {
                return;
            }
        }
        let dpr = self.dpr;
        if let Some(f) = self.desk.form(id) {
            f.scale = dpr;
            f.state = f.spec.state;
            f.ui.dirty = true;
        }
        if let Some(w) = self.wins.get_mut(id) {
            let _ = w.root.style().set_property("display", "block");
            w.force = true;
            w.frame_dirty = true;
        }
        self.place(id);
    }

    /// The window's outline (`ShapeForm`, `rapidr_value::shape`): the page
    /// shows — and the mouse reaches — only the bitmap's pixels that
    /// aren't its transparent colour, counted from the window's top left
    /// corner, its frame included, as RapidQ's runtime gives a window its
    /// region; what's outside is the page.
    fn apply_shape(&self, id: &str) {
        let Some(w) = self.wins.get(id) else { return };
        let style = w.root.style();
        match rapidr_value::shape::get(id) {
            Some(shape) => {
                let path = format!("path('{}')", shape.svg_path(0, 0));
                let _ = style.set_property("clip-path", &path);
                // (the window's shadow is its outline's too)
                let _ = style.set_property("box-shadow", "none");
            }
            None => {
                let _ = style.remove_property("clip-path");
            }
        }
    }

    /// The window at its Left / Top on the page.
    fn place(&mut self, id: &str) {
        let Some(pos) = self.desk.forms.get(id).and_then(|f| f.spec.position) else { return };
        if let Some(w) = self.wins.get(id) {
            // (minimized: its title bar in its slot along the page's bottom)
            let pos = match w.min_slot {
                Some(slot) => ((slot as i64) * (MIN_WIDTH + 4), screen().1 - frame::inset(true, true).1 - rapidr_value::layout::FORM_BORDER),
                None => pos,
            };
            set_style(&w.root, &[("left", px(pos.0 as f64)), ("top", px(pos.1 as f64))]);
        }
    }

    /// WindowState on the page: maximized (the viewport, less the frame),
    /// minimized (its title bar only) or restored; the program hears it as
    /// the user's resize and move, and its WindowState.
    fn set_state(&mut self, store: &dyn Store, id: &str, state: i64) {
        use rapidr_value::window_state::{WS_MAXIMIZED, WS_MINIMIZED};
        let Some(f) = self.desk.forms.get(id) else { return };
        let (pos, inside, border, caption) = (f.spec.position.unwrap_or((0, 0)), f.spec.size, f.spec.border, !f.spec.no_caption);
        let Some(w) = self.wins.get_mut(id) else { return };
        match state {
            WS_MAXIMIZED => {
                if w.normal.is_none() {
                    w.normal = Some((pos, inside));
                }
                w.minimized = false;
                w.min_slot = None;
                let (sw, sh) = screen();
                let (ow, oh) = frame::outer((0, 0), border, caption);
                let (iw, ih) = ((sw - ow).max(1), (sh - oh).max(1));
                w.frame_dirty = true;
                if let Some(f) = self.desk.form(id) {
                    f.ui.sync(store);
                }
                self.desk.moved(id, 0, 0);
                self.desk.resized(id, iw, ih);
            }
            WS_MINIMIZED => {
                w.minimized = true;
                w.frame_dirty = true;
                let taken: Vec<usize> = self.wins.values().filter_map(|w| w.min_slot).collect();
                let slot = (0..).find(|s| !taken.contains(s)).unwrap_or(0);
                if let Some(w) = self.wins.get_mut(id) {
                    w.min_slot = Some(slot);
                }
            }
            _ => {
                w.minimized = false;
                w.min_slot = None;
                w.frame_dirty = true;
                if let Some((p, s)) = w.normal.take() {
                    if let Some(f) = self.desk.form(id) {
                        f.ui.sync(store);
                    }
                    self.desk.moved(id, p.0, p.1);
                    self.desk.resized(id, s.0, s.1);
                }
            }
        }
        self.place(id);
        self.desk.window_state(id, state);
    }

    // ------------------------------------------------------ the window --

    /// Form `id`'s window's elements and listeners.
    fn make(&mut self, id: &str) -> Option<Win> {
        let doc = &self.doc;
        let root: HtmlElement = doc.create_element("div").ok()?.dyn_into().ok()?;
        root.set_class_name("rr-kwin");
        root.set_attribute("data-rr-form", id).ok()?;
        let frame: HtmlCanvasElement = doc.create_element("canvas").ok()?.dyn_into().ok()?;
        frame.set_class_name("rr-kframe");
        frame.set_attribute("aria-hidden", "true").ok()?;
        let client: HtmlCanvasElement = doc.create_element("canvas").ok()?.dyn_into().ok()?;
        client.set_class_name("rr-kclient");
        // (the picture is the mirror's to describe)
        client.set_attribute("aria-hidden", "true").ok()?;
        let mirror = Mirror::new(doc, id)?;
        let overlays: HtmlElement = doc.create_element("div").ok()?.dyn_into().ok()?;
        overlays.set_class_name("rr-koverlays");
        let popups: HtmlCanvasElement = doc.create_element("canvas").ok()?.dyn_into().ok()?;
        popups.set_class_name("rr-kpopups");
        popups.set_attribute("aria-hidden", "true").ok()?;
        let _ = popups.style().set_property("display", "none");
        root.append_child(&frame).ok()?;
        root.append_child(&client).ok()?;
        root.append_child(&overlays).ok()?;
        root.append_child(&popups).ok()?;
        root.append_child(mirror.root()).ok()?;
        let mut grips = Vec::new();
        for (edge, cursor) in EDGES {
            let g: HtmlElement = doc.create_element("div").ok()?.dyn_into().ok()?;
            g.set_class_name("rr-kgrip");
            g.set_attribute("data-edge", edge).ok()?;
            let _ = g.style().set_property("cursor", cursor);
            root.append_child(&g).ok()?;
            grips.push(g);
        }
        let _ = root.style().set_property("display", "none");
        doc.body()?.append_child(&root).ok()?;
        let ctx_of = |c: &HtmlCanvasElement| -> Option<CanvasRenderingContext2d> {
            let opts = web_sys::ContextAttributes2d::new();
            opts.set_alpha(false);
            c.get_context_with_context_options("2d", &opts).ok()??.dyn_into().ok()
        };
        let fctx = ctx_of(&frame)?;
        let ctx = ctx_of(&client)?;
        // (a layer: transparent where nothing is open)
        let pctx: CanvasRenderingContext2d = popups.get_context("2d").ok()??.dyn_into().ok()?;
        let mut w = Win {
            root,
            frame,
            fctx,
            client,
            ctx,
            mirror,
            overlays,
            popups,
            pctx,
            pcpu: None,
            grips,
            cpu: None,
            fcpu: None,
            inside: (0, 0),
            scale: 0.0,
            look: None,
            frame_dirty: true,
            force: true,
            drag: None,
            pressed: None,
            minimized: false,
            min_slot: None,
            normal: None,
            _listeners: Vec::new(),
        };
        w._listeners = listeners(&w, id, self.mac);
        Some(w)
    }

    /// Form `id` as the user's: its window on top when input may reach it
    /// (else the modal window that keeps it from it).
    fn activate(&mut self, id: &str) {
        let target = if self.desk.accepts_input(id) { id.to_string() } else { self.desk.modal.last().cloned().unwrap_or_else(|| id.to_string()) };
        if self.top().as_deref() != Some(target.as_str()) {
            self.desk.show(&target);
        }
    }

    // ---------------------------------------------------------- drawing --

    /// Every shown window the kernel (or the host) must draw again, drawn:
    /// the kernel's display list rasterized by vello_cpu at the page's
    /// scale into the client canvas, the frame into its own, the mirror
    /// brought up to date.
    pub fn render(&mut self, store: &dyn Store) {
        let top = self.top();
        let ids: Vec<String> = self.wins.keys().cloned().collect();
        for id in ids {
            let shown = self.desk.forms.get(&id).is_some_and(|f| f.shown);
            if !shown {
                continue;
            }
            let Desktop { forms, text, .. } = &mut self.desk;
            let Some(f) = forms.get_mut(&id) else { continue };
            let Some(w) = self.wins.get_mut(&id) else { continue };
            let scale = f.scale;
            let mut drawn = false;
            if w.force || f.ui.dirty || (w.scale - scale).abs() > f64::EPSILON {
                let started = Instant::now();
                rapidr_value::objects::bitmap::set_display_scale(scale);
                f.ui.popups_apart = f.ui.nodes.iter().any(|n| is_overlay(&n.type_name));
                let list = f.ui.paint(store, text, scale);
                let (dw, dh) = rapidr_ui_render::canvas::device_size(&list);
                if list.size != w.inside || (w.scale - scale).abs() > f64::EPSILON {
                    w.inside = list.size;
                    w.scale = scale;
                    w.frame_dirty = true;
                }
                if w.client.width() != dw || w.client.height() != dh {
                    w.client.set_width(dw);
                    w.client.set_height(dh);
                }
                let r = w.cpu.get_or_insert_with(|| CpuRenderer::new(dw, dh));
                r.render(dw, dh, &list, text, &f.ui);
                if let Ok(image) = ImageData::new_with_u8_clamped_array_and_sh(Clamped(r.pixmap.data_as_u8_slice()), dw, dh) {
                    let _ = w.ctx.put_image_data(&image, 0.0, 0.0);
                }
                let popups = if f.ui.popups_apart { Some(f.ui.paint_popups(store, text, scale)) } else { None };
                match popups.filter(|p| !p.items.is_empty()) {
                    Some(list) => {
                        if w.popups.width() != dw || w.popups.height() != dh {
                            w.popups.set_width(dw);
                            w.popups.set_height(dh);
                        }
                        let r = w.pcpu.get_or_insert_with(|| CpuRenderer::new(dw, dh));
                        r.render_transparent(dw, dh, &list, text, &f.ui);
                        // (the canvas wants straight alpha; the pixmap's is premultiplied)
                        let mut px = r.pixmap.data_as_u8_slice().to_vec();
                        for p in px.as_chunks_mut::<4>().0 {
                            let a = u32::from(p[3]);
                            if a != 0 && a != 255 {
                                for c in &mut p[..3] {
                                    *c = ((u32::from(*c) * 255 + a / 2) / a).min(255) as u8;
                                }
                            }
                        }
                        if let Ok(image) = ImageData::new_with_u8_clamped_array_and_sh(Clamped(&px), dw, dh) {
                            let _ = w.pctx.put_image_data(&image, 0.0, 0.0);
                        }
                        let _ = w.popups.style().set_property("display", "block");
                    }
                    None => {
                        let _ = w.popups.style().set_property("display", "none");
                    }
                }
                w.force = false;
                drawn = true;
                frame_time(started);
            }
            let look = Look {
                title: f.spec.title.clone(),
                active: top.as_deref() == Some(id.as_str()),
                border: f.spec.border,
                caption: !f.spec.no_caption,
                frame: f.spec.frame,
                maximized: f.state == rapidr_value::window_state::WS_MAXIMIZED,
                theme: rapidr_value::theme::generation(),
                icon: f.spec.icon.clone(),
            };
            if w.look.as_ref() != Some(&look) {
                w.look = Some(look);
                w.frame_dirty = true;
            }
            if w.frame_dirty {
                w.frame_dirty = false;
                layout(w);
                let look = w.look.clone().expect("a look");
                let size = frame::outer(w.inside, look.border, look.caption);
                let shown_size = if w.minimized { (MIN_WIDTH, frame::inset(look.border, look.caption).1 + rapidr_value::layout::FORM_BORDER) } else { size };
                let list = frame::paint(&look, shown_size, scale);
                // (the window's corners and its shadow on the page)
                let (radius, inside, shadow) = frame::css(&look);
                let _ = w.frame.style().set_property("border-radius", &radius);
                let _ = w.frame.style().set_property("box-shadow", &shadow);
                let _ = w.client.style().set_property("border-radius", &inside);
                let (dw, dh) = rapidr_ui_render::canvas::device_size(&list);
                if w.frame.width() != dw || w.frame.height() != dh {
                    w.frame.set_width(dw);
                    w.frame.set_height(dh);
                }
                let r = w.fcpu.get_or_insert_with(|| CpuRenderer::new(dw, dh));
                r.render(dw, dh, &list, text, &f.ui);
                if let Ok(image) = ImageData::new_with_u8_clamped_array_and_sh(Clamped(r.pixmap.data_as_u8_slice()), dw, dh) {
                    let _ = w.fctx.put_image_data(&image, 0.0, 0.0);
                }
            }
            if drawn {
                let tree = f.ui.access_tree(store, text);
                let focused = f.ui.focused().map(str::to_string);
                let hints = hints(&f.ui, store);
                w.mirror.sync(&tree, focused.as_deref(), top.as_deref() == Some(id.as_str()), &hints);
                place_overlays(&self.doc, w, &f.ui);
            }
        }
    }

    /// A font was added to the text system: every window, frame and
    /// inside, drawn again.
    pub fn fonts_changed(&mut self) {
        for w in self.wins.values_mut() {
            w.force = true;
            w.frame_dirty = true;
        }
    }

    /// Form `id` drawn by the CPU renderer at its scale, as the desktop's
    /// `RAPIDR_CAPTURE` saves it (a test's capture: the very pixels).
    pub fn capture(&mut self, store: &dyn Store, id: &str) -> Option<Pixels> {
        let Desktop { forms, text, .. } = &mut self.desk;
        let f = forms.get_mut(&id.to_lowercase())?;
        let list = f.ui.paint(store, text, f.scale);
        Some(rapidr_ui_render::cpu::capture(&list, text, &f.ui))
    }

    /// Brings window `id`'s mirror up to date now (its focus moves inside
    /// the user's gesture: a phone shows its keyboard only then).
    fn sync_mirror(&mut self, store: &dyn Store, id: &str) {
        let active = self.top().as_deref() == Some(id);
        let Desktop { forms, text, .. } = &mut self.desk;
        let (Some(f), Some(w)) = (forms.get_mut(id), self.wins.get_mut(id)) else { return };
        let tree = f.ui.access_tree(store, text);
        let focused = f.ui.focused().map(str::to_string);
        let hints = hints(&f.ui, store);
        w.mirror.sync(&tree, focused.as_deref(), active, &hints);
    }
}

/// A window's elements laid out for its inside and frame (logical = CSS
/// pixels; the canvases' backing is the device's).
fn layout(w: &mut Win) {
    let border = w.look.as_ref().is_some_and(|l| l.border);
    let caption = w.look.as_ref().is_none_or(|l| l.caption);
    let (ix, iy) = frame::inset(border, caption);
    let (iw, ih) = w.inside;
    let (ow, oh) = frame::outer(w.inside, border, caption);
    let shown_h = if w.minimized { iy + rapidr_value::layout::FORM_BORDER } else { oh };
    let ow = if w.minimized { MIN_WIDTH } else { ow };
    set_style(&w.root, &[("width", px(ow as f64)), ("height", px(shown_h as f64))]);
    set_style(&w.frame.clone().unchecked_into(), &[("left", px(0.0)), ("top", px(0.0)), ("width", px(ow as f64)), ("height", px(shown_h as f64)), ("display", if border { "block" } else { "none" }.into())]);
    let inside = if w.minimized { "none" } else { "block" };
    set_style(&w.client.clone().unchecked_into(), &[("left", px(ix as f64)), ("top", px(iy as f64)), ("width", px(iw as f64)), ("height", px(ih as f64)), ("display", inside.into())]);
    set_style(w.mirror.root(), &[("left", px(ix as f64)), ("top", px(iy as f64)), ("width", px(iw as f64)), ("height", px(ih as f64)), ("display", inside.into())]);
    set_style(&w.overlays, &[("left", px(ix as f64)), ("top", px(iy as f64)), ("width", px(iw as f64)), ("height", px(ih as f64)), ("display", inside.into())]);
    set_style(&w.popups.clone().unchecked_into(), &[("left", px(ix as f64)), ("top", px(iy as f64)), ("width", px(iw as f64)), ("height", px(ih as f64))]);
    // (the edges a resizable window is dragged by: 4 pixels outside, 2 in)
    let resizable = border && !w.minimized && w.look.as_ref().is_some_and(|l| l.frame.resizable && !l.maximized);
    for g in &w.grips {
        let edge = g.get_attribute("data-edge").unwrap_or_default();
        // (the sides between the corners; the corners 12 pixels square)
        let r = match edge.as_str() {
            "e" => (ow - 2, 8, 6, oh - 16),
            "s" => (8, oh - 2, ow - 16, 6),
            "w" => (-4, 8, 6, oh - 16),
            "n" => (8, -4, ow - 16, 6),
            "nw" => (-4, -4, 12, 12),
            "ne" => (ow - 8, -4, 12, 12),
            "sw" => (-4, oh - 8, 12, 12),
            _ => (ow - 8, oh - 8, 12, 12),
        };
        set_style(g, &[("left", px(r.0 as f64)), ("top", px(r.1 as f64)), ("width", px(r.2 as f64)), ("height", px(r.3 as f64)), ("display", if resizable { "block" } else { "none" }.into())]);
    }
}

/// The web-only components' elements of form `ui` over its canvas: each at
/// its node's place in the client area, hidden with it, clipped to its
/// parents' rectangles (a scrolled or smaller parent cuts it as the kernel
/// cuts what it draws).
fn place_overlays(doc: &Document, w: &Win, ui: &rapidr_ui_kernel::FormUi) {
    for n in ui.nodes.iter().filter(|n| is_overlay(&n.type_name)) {
        let Some(el) = doc.get_element_by_id(&format!("rr-{}", n.id)).and_then(|e| e.dyn_into::<HtmlElement>().ok()) else { continue };
        if el.get_attribute("data-rr-overlay").is_none() {
            continue;
        }
        if el.parent_element().as_ref() != Some(w.overlays.as_ref()) {
            let _ = w.overlays.append_child(&el);
        }
        let (x, y, width, height) = n.abs;
        // (the parents' rectangles: what of it they show)
        let (mut cl, mut ct, mut cr, mut cb) = (x, y, x + width, y + height);
        let mut p = n.parent;
        while let Some(i) = p {
            let (px0, py0, pw, ph) = ui.nodes[i].abs;
            cl = cl.max(px0);
            ct = ct.max(py0);
            cr = cr.min(px0 + pw);
            cb = cb.min(py0 + ph);
            p = ui.nodes[i].parent;
        }
        let clip = if (cl, ct, cr, cb) == (x, y, x + width, y + height) {
            String::new()
        } else {
            format!("inset({}px {}px {}px {}px)", (ct - y).max(0), (x + width - cr).max(0), (y + height - cb).max(0), (cl - x).max(0))
        };
        let shown = n.shown && cr > cl && cb > ct;
        set_style(&el, &[("left", px(x as f64)), ("top", px(y as f64)), ("width", px(width as f64)), ("height", px(height as f64)), ("display", if shown { "" } else { "none" }.into()), ("clip-path", clip)]);
    }
}

/// The form's text fields' hints for autofill (their AutoComplete — a
/// RapidR property — and their names), by accessibility node.
fn hints(ui: &rapidr_ui_kernel::FormUi, store: &dyn Store) -> HashMap<u64, crate::mirror::Hint> {
    let form = (rapidr_value::objects::a11y::node_id(&ui.form), crate::mirror::Hint { autocomplete: String::new(), name: ui.form.clone() });
    ui.nodes
        .iter()
        .filter(|n| !is_overlay(&n.type_name))
        .map(|n| {
            let field = matches!(n.type_name.as_str(), "REDIT" | "RMEMO" | "RRICHEDIT" | "RCOMBOBOX");
            let autocomplete = if field { store.get(&n.id, "autocomplete").to_string_val().trim().to_string() } else { String::new() };
            (rapidr_value::objects::a11y::node_id(&n.id), crate::mirror::Hint { autocomplete, name: n.id.clone() })
        })
        .chain(std::iter::once(form))
        .collect()
}

/// The frame's size as shown (a minimized window: its title bar alone).
fn shown_frame(w: &Win, look: &Look) -> (i64, i64) {
    let size = frame::outer(w.inside, look.border, look.caption);
    if w.minimized {
        (MIN_WIDTH, frame::inset(look.border, look.caption).1 + rapidr_value::layout::FORM_BORDER)
    } else {
        size
    }
}

// ------------------------------------------------------------- input --

/// Where a pointer event is in an element (logical = CSS pixels).
fn at(el: &Element, e: &web_sys::MouseEvent) -> (f64, f64) {
    let r = el.get_bounding_client_rect();
    (f64::from(e.client_x()) - r.left(), f64::from(e.client_y()) - r.top())
}

fn listen(target: &web_sys::EventTarget, kind: &str, out: &mut Vec<Listener>, f: impl FnMut(web_sys::Event) + 'static) {
    let cb = Closure::<dyn FnMut(web_sys::Event)>::new(f);
    let opts = web_sys::AddEventListenerOptions::new();
    // (wheel and touch: the page mustn't scroll instead)
    opts.set_passive(false);
    let _ = target.add_event_listener_with_callback_and_add_event_listener_options(kind, cb.as_ref().unchecked_ref(), &opts);
    out.push(cb);
}

/// The text the kernel's copy / cut put on the page's clipboard, for the
/// system's (`navigator.clipboard`, allowed inside the user's gesture).
fn to_system_clipboard(text: &str) {
    let _ = window().navigator().clipboard().write_text(text);
}

fn listeners(w: &Win, id: &str, mac: bool) -> Vec<Listener> {
    let mut out = Vec::new();
    // ---- the client area: the pointer, through the kernel's routing (on
    // the drop-down / menu layer too, which is the client area's while it
    // shows) ----
    for client in [Element::from(w.client.clone()), Element::from(w.popups.clone())] {
        {
            let (id, el) = (id.to_string(), client.clone());
            listen(&client, "pointerdown", &mut out, move |e| {
                let Ok(e) = e.dyn_into::<web_sys::PointerEvent>() else { return };
                // (keeps the focus where the kernel puts it, not on the page)
                e.prevent_default();
                let _ = el.set_pointer_capture(e.pointer_id());
                let p = at(&el, &e);
                let m = mouse_mods(&e, mac);
                input(|h, store| {
                    h.activate(&id);
                    h.desk.mouse_down(store, &id, p, Button::from_dom(e.button()), m, Source::User);
                    h.sync_mirror(store, &id);
                });
            });
        }
        {
            let (id, el) = (id.to_string(), client.clone());
            listen(&client, "pointermove", &mut out, move |e| {
                let Ok(e) = e.dyn_into::<web_sys::PointerEvent>() else { return };
                let p = at(&el, &e);
                let m = mouse_mods(&e, mac);
                input(|h, store| {
                    h.mouse = (f64::from(e.client_x()), f64::from(e.client_y()));
                    h.desk.mouse_move(store, &id, p.0, p.1, m, Source::User);
                    // (the pointer: Screen.Cursor, else the component's —
                    // the desktop's rule)
                    let cursor = rapidr_ui_app::desktop::cursor_at(&mut h.desk, store, &id, p);
                    if let Some(el) = el.dyn_ref::<HtmlElement>() {
                        set_style(el, &[("cursor", cursor.css().into())]);
                    }
                });
            });
        }
        {
            let (id, el) = (id.to_string(), client.clone());
            listen(&client, "pointerup", &mut out, move |e| {
                let Ok(e) = e.dyn_into::<web_sys::PointerEvent>() else { return };
                let p = at(&el, &e);
                let m = mouse_mods(&e, mac);
                input(|h, store| {
                    let before = h.clip.0.borrow().clone();
                    h.desk.mouse_up(store, &id, p, Button::from_dom(e.button()), m, Source::User);
                    // (an edit's context menu copied or cut)
                    let after = h.clip.0.borrow().clone();
                    if after != before {
                        if let Some(t) = after {
                            to_system_clipboard(&t);
                        }
                    }
                    h.sync_mirror(store, &id);
                });
            });
        }
        {
            let (id, el) = (id.to_string(), client.clone());
            listen(&client, "pointerleave", &mut out, move |e| {
                let Ok(e) = e.dyn_into::<web_sys::PointerEvent>() else { return };
                if !el.has_pointer_capture(e.pointer_id()) {
                    input(|h, store| h.desk.mouse_leave(store, &id, Source::User));
                }
            });
        }
        listen(&client, "contextmenu", &mut out, |e| e.prevent_default());
        // (RapidR's OnDropFiles: files dragged from the computer onto the
        // window — refused by a form without the handler, and never the
        // browser's own drop, which would replace the page by the file)
        {
            let id = id.to_string();
            listen(&client, "dragover", &mut out, move |e| {
                let Ok(e) = e.dyn_into::<web_sys::DragEvent>() else { return };
                e.prevent_default();
                if let Some(dt) = e.data_transfer() {
                    dt.set_drop_effect(if accepts_drop(&id) { "copy" } else { "none" });
                }
            });
        }
        {
            let id = id.to_string();
            listen(&client, "drop", &mut out, move |e| {
                let Ok(e) = e.dyn_into::<web_sys::DragEvent>() else { return };
                e.prevent_default();
                if !accepts_drop(&id) {
                    return;
                }
                if let Some(list) = e.data_transfer().and_then(|dt| dt.files()) {
                    drop_files(&id, list);
                }
            });
        }
        {
            let (id, el) = (id.to_string(), client.clone());
            listen(&client, "wheel", &mut out, move |e| {
                let Ok(e) = e.dyn_into::<web_sys::WheelEvent>() else { return };
                e.prevent_default();
                // (notches, positive down: a line mode's 3 lines, a pixel mode's
                // 48 logical pixels — the desktop host's touchpad rule)
                let k = match e.delta_mode() {
                    1 => 1.0 / 3.0,
                    2 => 1.0,
                    _ => 1.0 / 48.0,
                };
                let p = at(&el, &e);
                let m = mouse_mods(&e, mac);
                input(|h, store| h.desk.mouse_wheel(store, &id, p, (e.delta_x() * k, e.delta_y() * k), m, Source::User));
            });
        }
    }
    for c in [&w.client, &w.frame] {
        let id = id.to_string();
        listen(c, "contextrestored", &mut out, move |_| {
            input(|h, _| {
                if let Some(w) = h.wins.get_mut(&id) {
                    w.force = true;
                    w.frame_dirty = true;
                }
            });
        });
    }
    // ---- the frame: the title bar's drag and buttons ----
    let fr: Element = w.frame.clone().into();
    {
        let (id, el) = (id.to_string(), fr.clone());
        listen(&fr, "pointerdown", &mut out, move |e| {
            let Ok(e) = e.dyn_into::<web_sys::PointerEvent>() else { return };
            e.prevent_default();
            let p = at(&el, &e);
            let page = (f64::from(e.client_x()), f64::from(e.client_y()));
            let _ = el.set_pointer_capture(e.pointer_id());
            input(|h, _store| {
                h.activate(&id);
                if !h.desk.accepts_input(&id) {
                    return;
                }
                let Some(f) = h.desk.forms.get(&id) else { return };
                let (pos, maximized) = (f.spec.position.unwrap_or((0, 0)), f.state == rapidr_value::window_state::WS_MAXIMIZED);
                let Some(w) = h.wins.get_mut(&id) else { return };
                let Some(look) = w.look.clone() else { return };
                let size = shown_frame(w, &look);
                match frame::hit(&look, size, p.0, p.1) {
                    Part::Title if e.detail() == 2 && (look.frame.maximize || w.minimized) => {
                        let to = if maximized || w.minimized { rapidr_value::window_state::WS_NORMAL } else { rapidr_value::window_state::WS_MAXIMIZED };
                        w.drag = None;
                        h.desk.cmds.push(HostCmd::State(id.clone()));
                        if let Some(f) = h.desk.form(&id) {
                            f.spec.state = to;
                        }
                    }
                    Part::Title if !maximized && !w.minimized => w.drag = Some(Drag { resize: None, from: page, start: pos, size: (0, 0) }),
                    part @ (Part::Close | Part::Maximize | Part::Minimize) => w.pressed = Some(part),
                    _ => {}
                }
            });
        });
    }
    {
        let id = id.to_string();
        listen(&fr, "pointermove", &mut out, move |e| {
            let Ok(e) = e.dyn_into::<web_sys::PointerEvent>() else { return };
            let page = (f64::from(e.client_x()), f64::from(e.client_y()));
            input(|h, _| {
                let Some(d) = h.wins.get(&id).and_then(|w| w.drag) else { return };
                let to = (d.start.0 + (page.0 - d.from.0).round() as i64, d.start.1 + (page.1 - d.from.1).round() as i64);
                h.desk.moved(&id, to.0, to.1);
                h.place(&id);
            });
        });
    }
    {
        let (id, el) = (id.to_string(), fr.clone());
        listen(&fr, "pointerup", &mut out, move |e| {
            let Ok(e) = e.dyn_into::<web_sys::PointerEvent>() else { return };
            let p = at(&el, &e);
            input(|h, _| {
                let Some(w) = h.wins.get_mut(&id) else { return };
                w.drag = None;
                let Some(pressed) = w.pressed.take() else { return };
                let Some(look) = w.look.clone() else { return };
                if frame::hit(&look, shown_frame(w, &look), p.0, p.1) != pressed {
                    return;
                }
                let state = h.desk.forms.get(&id).map_or(0, |f| f.state);
                match pressed {
                    Part::Close => h.desk.close_box(&id, Source::User),
                    Part::Maximize | Part::Minimize => {
                        use rapidr_value::window_state::{WS_MAXIMIZED, WS_MINIMIZED, WS_NORMAL};
                        let to = match pressed {
                            Part::Maximize if state == WS_MAXIMIZED => WS_NORMAL,
                            Part::Maximize => WS_MAXIMIZED,
                            _ if state == WS_MINIMIZED => WS_NORMAL,
                            _ => WS_MINIMIZED,
                        };
                        if let Some(f) = h.desk.form(&id) {
                            f.spec.state = to;
                        }
                        h.desk.cmds.push(HostCmd::State(id.clone()));
                    }
                    _ => {}
                }
            });
        });
    }
    // ---- the edges: resized ----
    for g in &w.grips {
        let edge = g.get_attribute("data-edge").unwrap_or_default();
        let sides = (edge.contains('w'), edge.contains('n'), edge.contains('e'), edge.contains('s'));
        let ge: Element = g.clone().into();
        {
            let (id, el) = (id.to_string(), ge.clone());
            listen(&ge, "pointerdown", &mut out, move |e| {
                let Ok(e) = e.dyn_into::<web_sys::PointerEvent>() else { return };
                e.prevent_default();
                let _ = el.set_pointer_capture(e.pointer_id());
                let page = (f64::from(e.client_x()), f64::from(e.client_y()));
                input(|h, _| {
                    h.activate(&id);
                    if !h.desk.accepts_input(&id) {
                        return;
                    }
                    let (size, pos) = h.desk.forms.get(&id).map_or(((0, 0), (0, 0)), |f| (f.spec.size, f.spec.position.unwrap_or((0, 0))));
                    if let Some(w) = h.wins.get_mut(&id) {
                        w.drag = Some(Drag { resize: Some(sides), from: page, start: pos, size });
                    }
                });
            });
        }
        {
            let id = id.to_string();
            listen(&ge, "pointermove", &mut out, move |e| {
                let Ok(e) = e.dyn_into::<web_sys::PointerEvent>() else { return };
                let page = (f64::from(e.client_x()), f64::from(e.client_y()));
                input(|h, store| {
                    let Some(d) = h.wins.get(&id).and_then(|w| w.drag) else { return };
                    let Some((l, t, r, b)) = d.resize else { return };
                    let (dx, dy) = ((page.0 - d.from.0).round() as i64, (page.1 - d.from.1).round() as i64);
                    // (a left or top edge moves the window as much as it
                    // grows it: the opposite edge stays)
                    let iw = if r { d.size.0 + dx } else if l { d.size.0 - dx } else { d.size.0 }.max(1);
                    let ih = if b { d.size.1 + dy } else if t { d.size.1 - dy } else { d.size.1 }.max(1);
                    let left = if l { d.start.0 + d.size.0 - iw } else { d.start.0 };
                    let top = if t { d.start.1 + d.size.1 - ih } else { d.start.1 };
                    if let Some(f) = h.desk.form(&id) {
                        f.ui.sync(store);
                    }
                    if (left, top) != d.start || l || t {
                        h.desk.moved(&id, left, top);
                        h.place(&id);
                    }
                    h.desk.resized(&id, iw, ih);
                });
            });
        }
        {
            let id = id.to_string();
            listen(&ge, "pointerup", &mut out, move |_| {
                input(|h, _| {
                    if let Some(w) = h.wins.get_mut(&id) {
                        w.drag = None;
                    }
                });
            });
        }
    }
    // ---- keys, the clipboard, a screen reader: the mirror ----
    let m: Element = w.mirror.root().clone().into();
    {
        let id = id.to_string();
        listen(&m, "keydown", &mut out, move |e| {
            let Ok(e) = e.dyn_into::<web_sys::KeyboardEvent>() else { return };
            // (an input method's: its composition events carry the text)
            if e.is_composing() || e.key_code() == 229 || e.key() == "Dead" || e.key() == "Process" {
                return;
            }
            let key = e.key();
            let cmd = if mac { e.meta_key() } else { e.ctrl_key() };
            let k = key.to_lowercase();
            // (Copy, Cut, Paste: the clipboard events, which carry the
            // clipboard's text without asking for a permission; the
            // browser's own keys; F6 / Ctrl+Tab leave a program embedded in
            // a page — no keyboard trap, WCAG 2.1.2)
            if cmd && ["c", "x", "v", "r", "t", "w", "l", "n", "+", "-", "=", "0"].contains(&k.as_str()) {
                return;
            }
            // (a page that is the application — RapidR Studio's — says so with
            // `window.RAPIDR_APP_KEYS = true`: F5 runs, F6 moves between its
            // areas, Ctrl+Tab between its documents, as an IDE's keys)
            let app_keys = web_sys::window().and_then(|w| js_sys::Reflect::get(&w, &"RAPIDR_APP_KEYS".into()).ok()).and_then(|v| v.as_bool()).unwrap_or(false);
            if key == "F12" || (!app_keys && (key == "F5" || key == "F6" || (key == "Tab" && e.ctrl_key()))) {
                return;
            }
            let Some(vk) = rapidr_value::input::vk_of_key(&key, &e.code()) else { return };
            let printable = key.chars().count() == 1 && !e.ctrl_key() && !e.meta_key();
            let text = if printable { key.clone() } else { String::new() };
            let mods = mods_of(e.shift_key(), e.ctrl_key(), e.alt_key(), e.meta_key(), mac);
            e.prevent_default();
            input(|h, store| {
                h.desk.key_down(store, &id, vk, &text, mods, Source::User);
                h.sync_mirror(store, &id);
            });
        });
    }
    {
        let id = id.to_string();
        listen(&m, "keyup", &mut out, move |e| {
            let Ok(e) = e.dyn_into::<web_sys::KeyboardEvent>() else { return };
            if e.is_composing() || e.key_code() == 229 {
                return;
            }
            let Some(vk) = rapidr_value::input::vk_of_key(&e.key(), &e.code()) else { return };
            let mods = mods_of(e.shift_key(), e.ctrl_key(), e.alt_key(), e.meta_key(), mac);
            input(|h, _| h.desk.key_up(&id, vk, mods, Source::User));
        });
    }
    for (kind, letter) in [("copy", 67), ("cut", 88)] {
        let id = id.to_string();
        listen(&m, kind, &mut out, move |e| {
            let Ok(e) = e.dyn_into::<web_sys::ClipboardEvent>() else { return };
            e.prevent_default();
            input(|h, store| {
                let before = h.clip.0.replace(None);
                h.desk.key_down(store, &id, letter, "", command(mac), Source::User);
                h.desk.key_up(&id, letter, command(mac), Source::User);
                let copied = h.clip.0.borrow().clone();
                match copied {
                    Some(t) => {
                        if let Some(d) = e.clipboard_data() {
                            let _ = d.set_data("text/plain", &t);
                        }
                    }
                    None => *h.clip.0.borrow_mut() = before,
                }
                h.sync_mirror(store, &id);
            });
        });
    }
    {
        let id = id.to_string();
        listen(&m, "paste", &mut out, move |e| {
            let Ok(e) = e.dyn_into::<web_sys::ClipboardEvent>() else { return };
            e.prevent_default();
            let text = e.clipboard_data().and_then(|d| d.get_data("text/plain").ok()).unwrap_or_default();
            input(|h, store| {
                *h.clip.0.borrow_mut() = Some(text.clone());
                h.desk.key_down(store, &id, 86, "", command(mac), Source::User);
                h.desk.key_up(&id, 86, command(mac), Source::User);
                h.sync_mirror(store, &id);
            });
        });
    }
    // A screen reader moving the focus onto a node: the kernel's focus.
    {
        let id = id.to_string();
        listen(&m, "focusin", &mut out, move |e| {
            let node = e.target().and_then(|t| t.dyn_into::<HtmlElement>().ok()).and_then(|el| el.dataset().get("node")).and_then(|n| n.parse::<u64>().ok());
            let Some(node) = node else { return };
            input(|h, store| {
                if h.wins.get(&id).is_some_and(|w| w.mirror.focusing() || w.mirror.focused_node() == Some(node)) {
                    return;
                }
                h.activate(&id);
                h.desk.access_action(store, &id, node, Action::Focus, None);
            });
        });
    }
    // A screen reader's click (VoiceOver's VO-Space, NVDA's Enter): no
    // pointer reaches the mirror.
    {
        let id = id.to_string();
        listen(&m, "click", &mut out, move |e| {
            let node = e.target().and_then(|t| t.dyn_into::<HtmlElement>().ok()).and_then(|el| el.dataset().get("node")).and_then(|n| n.parse::<u64>().ok());
            let Some(node) = node else { return };
            e.stop_propagation();
            input(|h, store| {
                h.desk.access_action(store, &id, node, Action::Click, None);
            });
        });
    }
    // The text fields (an edit's, a memo's mirror element): what input
    // methods, phone keyboards, dictation, the emoji picker and autofill
    // type arrives here.
    {
        let id = id.to_string();
        listen(&m, "compositionstart", &mut out, move |_| {
            input(|h, store| {
                let Desktop { forms, text, .. } = &mut h.desk;
                let area = forms.get_mut(&id).and_then(|f| f.ui.ime_area(store, text));
                if let Some(w) = h.wins.get_mut(&id) {
                    w.mirror.compose_start(area);
                }
            });
        });
    }
    {
        let id = id.to_string();
        listen(&m, "compositionupdate", &mut out, move |e| {
            let Ok(e) = e.dyn_into::<web_sys::CompositionEvent>() else { return };
            let data = e.data().unwrap_or_default();
            input(|h, store| {
                let n = data.len();
                h.desk.ime_preedit(store, &id, &data, (!data.is_empty()).then_some((n, n)), Source::User);
            });
        });
    }
    {
        let id = id.to_string();
        listen(&m, "compositionend", &mut out, move |e| {
            let Ok(e) = e.dyn_into::<web_sys::CompositionEvent>() else { return };
            let data = e.data().unwrap_or_default();
            input(|h, store| {
                if let Some(w) = h.wins.get_mut(&id) {
                    w.mirror.compose_end();
                }
                h.desk.ime_preedit(store, &id, "", None, Source::User);
                h.desk.ime_commit(store, &id, &data, Source::User);
                h.sync_mirror(store, &id);
            });
        });
    }
    {
        let id = id.to_string();
        listen(&m, "beforeinput", &mut out, move |e| {
            let Ok(e) = e.dyn_into::<web_sys::InputEvent>() else { return };
            if e.is_composing() {
                return;
            }
            let key = |h: &mut WebHost, store: &dyn Store, vk: i64| {
                h.desk.key_down(store, &id, vk, "", Mods::NONE, Source::User);
                h.desk.key_up(&id, vk, Mods::NONE, Source::User);
            };
            let kind = e.input_type();
            let data = e.data().or_else(|| e.data_transfer().and_then(|d| d.get_data("text/plain").ok())).unwrap_or_default();
            let handled = with(|h, store| {
                if h.wins.get(&id).is_some_and(|w| w.mirror.composing()) {
                    return false;
                }
                match kind.as_str() {
                    "insertText" | "insertReplacementText" | "insertFromPaste" | "insertFromDrop" => {
                        h.desk.ime_preedit(store, &id, "", None, Source::User);
                        h.desk.ime_commit(store, &id, &data, Source::User);
                    }
                    "deleteContentBackward" => key(h, store, 8),
                    "deleteContentForward" => key(h, store, 46),
                    "insertLineBreak" | "insertParagraph" => key(h, store, 13),
                    _ => return false,
                }
                h.sync_mirror(store, &id);
                true
            });
            if handled == Some(true) {
                e.prevent_default();
                wake();
            }
        });
    }
    // Autofill and password managers: a field's value set without
    // `beforeinput` — the user's edit, the whole text replaced.
    {
        let id = id.to_string();
        listen(&m, "input", &mut out, move |e| {
            let Some(field) = e.target() else { return };
            // (a code editor's field holds a window of its lines: never
            // typed over whole)
            let name = field.dyn_ref::<web_sys::Element>().and_then(|el| el.get_attribute("data-rr-name"));
            if name.is_some_and(|n| rapidr_value::objects::is_code(&n)) {
                return;
            }
            let value = if let Some(i) = field.dyn_ref::<web_sys::HtmlInputElement>() {
                i.value()
            } else if let Some(t) = field.dyn_ref::<web_sys::HtmlTextAreaElement>() {
                t.value()
            } else {
                return;
            };
            input(|h, store| {
                let Some(w) = h.wins.get(&id) else { return };
                if w.mirror.composing() || w.mirror.last_value(&field) == Some(value.clone()) {
                    return;
                }
                // (select all, then the filled text typed over it)
                h.desk.key_down(store, &id, 65, "", command(mac), Source::User);
                h.desk.key_up(&id, 65, command(mac), Source::User);
                h.desk.ime_commit(store, &id, &value, Source::User);
                h.sync_mirror(store, &id);
            });
        });
    }
    out
}

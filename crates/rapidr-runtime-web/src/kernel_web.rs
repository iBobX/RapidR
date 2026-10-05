//! The UI kernel as the web runtime's host (docs/web-host-plan.md, Stage
//! W3; feature `kernel`, the default host since W4 — `?host=dom` asks for
//! the old DOM host until it is deleted): the program's
//! forms drawn by the same kernel, the same display lists and the same CPU
//! renderer as on the desktop, as windows on the page
//! (`rapidr_ui_host_web::host`), their accessibility as an ARIA mirror.
//!
//! What doesn't depend on the host is `rapidr_ui_app`'s, the desktop's
//! own: the kernel's events as the program's ([`rapidr_ui_app::dispatch`]),
//! forms shown and closed, the modal list, menus, lists, the test script.
//! It works through [`Web`] — the web's `Program` (over `object_web.rs`,
//! `layout_web.rs`, `scroll_web.rs`, `mdi_web.rs`) and `Windows` (over the
//! web host) — and reads the components through [`WebStore`].
//!
//! **The page's turn** (§3.2): a DOM event → the kernel's input (inside the
//! host's listener) → [`turn`]: the queued events dispatched (handlers run,
//! or queued for the VM), the test script's step → [`schedule`]: one
//! `requestAnimationFrame` — the windows the program made visible shown,
//! the owner-drawn lists' events, the program's window commands carried out,
//! the dirty forms drawn, the mirrors synced — then one timer for the
//! earliest deadline (the kernel's: a caret, a held scroll bar; the test
//! script's next step). The program's own changes (a property set, a
//! handler run by a timer or a slice of the VM) ask for the frame through
//! `object_web` / `gui_web`, which call into here when the kernel hosts.
//!
//! Not here yet (Stage W4): the VM's waits through `rapidr_ui_app::waits`
//! — ShowModal keeps the web's own (the VM suspended until the form
//! closes, `dialog_web`), the message boxes and the colour / font / file
//! dialogs stay the page's (`dialog_web`), QTIMERs their `setInterval`s.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use rapidr_ui_app::desktop::{self, WindowSpec};
use rapidr_ui_app::file_dialog::Request;
use rapidr_ui_app::program::Then;
use rapidr_ui_app::windows::{invalidate, push_op, restructure, take_notify, take_ops};
use rapidr_ui_app::{forms, lists, script, testhooks, Program, ScriptInput, WindowOp, Windows};
use rapidr_ui_host_web::host;
use rapidr_ui_kernel::components::form::Container;
use rapidr_ui_kernel::Store;
use rapidr_value::layout::Constraints;
use rapidr_value::objects::font::Font;
use rapidr_value::Value;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::object_web::{form_of, get_children_of, rp_comp_get, rp_comp_set, rp_comp_set_prop_only, rp_comp_type, rp_fire_event, rp_fire_event_args, rp_fire_event_then, rp_has_handler};

thread_local! {
    static ON: Cell<Option<bool>> = const { Cell::new(None) };
    static FRAME_ASKED: Cell<bool> = const { Cell::new(false) };
    static TURNING: Cell<bool> = const { Cell::new(false) };
    static LATER: Cell<bool> = const { Cell::new(false) };
    /// The deadline timer: its handle and when it fires (performance.now()).
    static DEADLINE: Cell<Option<(i32, f64)>> = const { Cell::new(None) };
    /// A form's WindowState as the program last set it (the app's
    /// `set_window_state` wants the one before).
    static STATES: RefCell<std::collections::HashMap<String, i64>> = RefCell::new(std::collections::HashMap::new());
    /// The page's Open / Save dialogs' answers by request id (`None`: open).
    static FILES: RefCell<std::collections::HashMap<u64, Option<Vec<String>>>> = RefCell::new(std::collections::HashMap::new());
    /// A GUI test's results once its script ended (`rapidr_test_results`).
    static RESULTS: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Whether the kernel hosts the program's forms: always, unless the page
/// asked for the old DOM host (`?host=dom` in its address, or
/// `RAPIDR_HOST = "dom"` set on the page before the runtime starts) — only
/// until the DOM host is deleted (docs/web-host-plan.md §5).
pub fn on() -> bool {
    if let Some(on) = ON.with(Cell::get) {
        return on;
    }
    let dom = web_sys::window().is_some_and(|w| {
        let search = w.location().search().unwrap_or_default();
        let global = js_sys::Reflect::get(&w, &JsValue::from_str("RAPIDR_HOST")).ok().and_then(|v| v.as_string()).unwrap_or_default();
        search.split(['?', '&']).any(|p| p.eq_ignore_ascii_case("host=dom")) || global.eq_ignore_ascii_case("dom")
    });
    ON.with(|o| o.set(Some(!dom)));
    !dom
}

fn lower(s: &str) -> String {
    s.to_lowercase()
}

// ---------------------------------------------------------------- the store --

/// The program's components as the kernel reads them (`rapidr_ui_kernel::
/// Store` over the registry: `rp_comp_get`, `rp_comp_type`, the children by
/// Parent in creation order), with the desktop `RtStore`'s rules
/// (`rapidr_value::component_defaults::kernel_reads_unset`); a kernel-drawn
/// dialog's parts from its own store. Reading never runs program code.
pub struct WebStore;

/// A kernel-drawn dialog's component (`rapidr:…`): its own store answers.
fn dialog<R>(id: &str, f: impl FnOnce(&dyn Store) -> R) -> Option<R> {
    if !rapidr_ui_kernel::dialogs::is_dialog(id) {
        return None;
    }
    rapidr_ui_app::dialogs::with_store(id, f)
}

impl Store for WebStore {
    fn get(&self, id: &str, prop: &str) -> Value {
        if let Some(v) = dialog(id, |s| s.get(id, prop)) {
            return v;
        }
        let v = rp_comp_get(id, prop);
        if prop.eq_ignore_ascii_case("color") && rapidr_value::component_defaults::kernel_reads_unset(&rp_comp_type(id), prop, &v, || rp_comp_get(id, "__colorset").to_bool()) {
            return Value::Null;
        }
        v
    }

    fn type_of(&self, id: &str) -> String {
        if let Some(t) = dialog(id, |s| s.type_of(id)) {
            return t;
        }
        rp_comp_type(id).to_ascii_uppercase()
    }

    fn children(&self, id: &str) -> Vec<(String, String)> {
        if let Some(c) = dialog(id, |s| s.children(id)) {
            return c;
        }
        children(id)
    }

    fn font(&self, id: &str) -> Font {
        if let Some(f) = dialog(id, |s| s.font(id)) {
            return f;
        }
        rapidr_value::objects::font_from_props(id, &|i, p| rp_comp_get(i, p))
    }
}

/// A component's children, ids lowercase (as the desktop's registry keeps
/// them: the kernel's node ids — and so the accessibility tree's — are the
/// same on both).
fn children(id: &str) -> Vec<(String, String)> {
    get_children_of(id).into_iter().map(|(n, t)| (lower(&n), t)).collect()
}

static STORE: WebStore = WebStore;

// ------------------------------------------------------------ the program --

/// The web runtime for `rapidr_ui_app`: its program and the page's windows.
#[derive(Clone, Copy)]
pub struct Web;

impl Program for Web {
    fn get(self, id: &str, prop: &str) -> Value {
        rp_comp_get(id, prop)
    }
    fn set(self, id: &str, prop: &str, value: Value) {
        rp_comp_set(id, prop, value)
    }
    fn store(self, id: &str, prop: &str, value: Value) {
        rp_comp_set_prop_only(id, prop, value)
    }
    fn type_of(self, id: &str) -> String {
        rp_comp_type(id)
    }
    fn children(self, id: &str) -> Vec<(String, String)> {
        children(id)
    }
    fn form_of(self, id: &str) -> Option<String> {
        form_of(id).map(|f| lower(&f))
    }
    fn flag(self, id: &str, prop: &str, default: bool) -> bool {
        rapidr_ui_kernel::store::flag(&WebStore, id, prop, default)
    }
    fn fire(self, id: &str, event: &str) {
        rp_fire_event(id, event)
    }
    fn fire_args(self, id: &str, event: &str, args: &[Value]) {
        rp_fire_event_args(id, event, args)
    }
    fn fire_then(self, id: &str, event: &str, args: &[Value], then: Then) {
        rp_fire_event_then(id, event, args, then)
    }
    fn has_handler(self, id: &str, event: &str) -> bool {
        rp_has_handler(id, event)
    }
    fn in_host_callback(self) -> bool {
        // (and the VM between two time slices: its handler isn't over — a
        // test's next event waits for it, as on the desktop)
        // — and while the program sleeps or waits for the page's own work, a
        // SLEEP or a download, which hold a desktop program whole)
        host::busy() || crate::dialog_web::is_yielded() || crate::dialog_web::is_waiting()
    }
    fn quietly(self, f: &mut dyn FnMut()) {
        crate::layout_web::quietly(f)
    }
    fn constraints(self, form: &str) -> Constraints {
        crate::layout_web::constraints_of(form)
    }
    fn client_changed(self, form: &str) {
        crate::layout_web::client_changed(form);
        crate::scroll_web::update(form);
    }
    fn container(self, action: Container) {
        match action {
            Container::Scrolled { id, dx, dy } => crate::scroll_web::user_scrolled(&id, (dx, dy)),
            Container::SplitBegin(id) => {
                crate::layout_web::splitter_begin(&id);
            }
            Container::SplitMove(delta) => crate::layout_web::splitter_move(delta),
            Container::SplitEnd => crate::layout_web::splitter_end(),
            Container::Mdi { form, component, action } => crate::mdi_web::user(&form, &component, action),
            Container::Resize { .. } => {}
        }
        invalidate();
    }

    // (the DirectX lane's: directx_web.rs, as the desktop's directx.rs)
    fn form_built(self, id: &str) {
        crate::directx_web::form_shown(&id.to_uppercase());
    }
    fn form_fullscreen(self, id: &str) -> bool {
        crate::directx_web::fullscreen(&id.to_uppercase())
    }
    // (Stage W4: the timers' heap — a QDXTIMER's period and its frames, a
    // QDXJOYSTICK's looks, as the desktop's directx.rs)
    fn timer_period(self, id: &str) -> Option<std::time::Duration> {
        let name = id.to_uppercase();
        let is_dx = matches!(rp_comp_type(&name).as_str(), "RDXTIMER" | "RDXJOYSTICK");
        is_dx.then(|| std::time::Duration::from_millis(crate::directx_web::timer_interval(&name, rp_comp_get(&name, "interval").to_i64()).max(1) as u64))
    }
    fn timer_firing(self, id: &str) -> bool {
        crate::directx_web::timer_fired(&id.to_uppercase())
    }
}

// ------------------------------------------------------------ the windows --

/// The host, installed the first time a window is needed; under a GUI test
/// (`rapidr_set_test_env`) its script starts.
fn ensure() {
    if host::installed() {
        return;
    }
    host::install(&STORE, Rc::new(turn));
    let capture = testhooks::Capture::from_env();
    host::with(|h, _| {
        // (carets blink, but not under a test: captures must be steady)
        h.desk.blinks = capture.is_none();
        h.desk.ignore_user = capture.is_some();
    });
    if let Some(c) = capture {
        let next = rapidr_ui_kernel::tick::now() + std::time::Duration::from_secs_f64(c.delay.max(0.0));
        script::start(c, next);
    }
}

fn spec_of(name: &str) -> WindowSpec {
    desktop::window_spec(Web, name)
}

/// The program's changes into the kernel's forms (window commands, new
/// forms, trees rebuilt, layouts read again), then onto the page.
fn sync() {
    ensure();
    rapidr_ui_app::menus::dump_if_changed(Web);
    let notify = take_notify();
    let ops = take_ops();
    let modal = forms::modal_forms();
    host::with(|h, store| {
        for op in ops {
            h.desk.apply(store, op, true, spec_of);
        }
        h.desk.sync_forms(store, modal, notify);
        h.run_cmds(store);
    });
}

/// The events the host queued, each dispatched to completion (a kernel-drawn
/// dialog's to it, the program's as its events).
fn dispatch_pending() {
    loop {
        let events = host::with(|h, _| std::mem::take(&mut h.desk.events)).unwrap_or_default();
        if events.is_empty() {
            return;
        }
        for e in events {
            match e {
                rapidr_ui_app::desktop::HostEvent::Kernel(form, ev) if rapidr_ui_kernel::dialogs::is_dialog(&form) => rapidr_ui_app::dialogs::event(Web, &form, ev),
                rapidr_ui_app::desktop::HostEvent::Kernel(_, ev) => rapidr_ui_app::dispatch::dispatch(Web, ev),
                rapidr_ui_app::desktop::HostEvent::Wake => {}
            }
        }
    }
}

// ------------------------------------------------------------- the waits --
//
// (Stage W4) The interpreter's waits are `rapidr_ui_app::waits`', as the
// desktop's interpreter has them (`waits::set_cooperative`): ShowModal, a
// kernel-drawn dialog (MESSAGEBOX …, the colour and font dialogs), INPUT$'s
// key and DOEVENTS leave a wait there, and the VM suspends after the
// builtin (`dialog_web::suspend_for_wait`). Each page turn serves the
// innermost one ([`serve_waits`]): over or answered, the VM goes on with its
// result in place of the builtin's — the ModalResult, the button's IDYES,
// Execute's 1 / 0 / -1, INPUT$'s 1 / 0. Meanwhile the page runs: the
// program's timers tick and their handlers run (and may wait in turn: the
// innermost wait ends first). A native web build can't suspend: its
// ShowModal shows the form and returns, and its dialogs stay the page's.

/// The interpreter runs the page's program: its waits are the VM's to serve
/// (`rapidr-vm-host-web` says so as a program starts).
pub fn set_interpreter(on: bool) {
    rapidr_ui_app::waits::set_cooperative(on);
}

/// Whether the program waits in the VM's way (an interpreter; not a native
/// web build, whose code can't suspend).
pub fn cooperative() -> bool {
    rapidr_ui_app::waits::cooperative()
}

/// Leaves wait `w` to the VM (it suspends after this builtin); `false` when
/// it can't (a native build, a VM that can't suspend here).
fn begin_wait(w: rapidr_ui_app::waits::Wait) -> bool {
    if !cooperative() || !crate::dialog_web::suspend_for_wait() {
        return false;
    }
    rapidr_ui_app::waits::start(w);
    schedule();
    // (served from the page's next turn)
    later();
    true
}

/// The innermost waits that are over, each ended — the VM goes on with its
/// result (and may start another, served in turn). Only while the VM is
/// free; else a moment later.
fn serve_waits() {
    use rapidr_ui_app::waits::{self, Wait};
    loop {
        if !crate::dialog_web::modal_waiting() {
            return;
        }
        if !crate::dialog_web::vm_free() {
            later();
            return;
        }
        rapidr_ui_app::dialogs::give_hooked(Web);
        // (DOEVENTS: a turn — and every timer that was due when it was called
        // fires before it returns, each once the handler before it has run)
        if let Some(Wait::Once(since)) = waits::turn() {
            fire_timers();
            if rapidr_ui_app::timers::held_back() && rapidr_ui_app::timers::due_since(since) {
                return;
            }
            waits::pop();
            crate::dialog_web::resume_wait(Value::Null);
            // (one DOEVENTS a serving: a `DO: DOEVENTS: LOOP` gives the rest of
            // the page's turn — input, the test script, a frame — its due)
            return;
        }
        if let Some(result) = waits::answered(Web) {
            crate::dialog_web::resume_wait(result);
            continue;
        }
        if waits::over() {
            match waits::pop() {
                Some(Wait::Form(form)) => {
                    // (the timers stop with the modal form, as the desktop's)
                    stop_timers();
                    let result = forms::modal_ended(Web, &form);
                    crate::dialog_web::resume_wait(Value::Integer(result));
                }
                Some(_) => crate::dialog_web::resume_wait(Value::Null),
                None => return,
            }
            continue;
        }
        return;
    }
}

/// A native web build's modal forms that closed: off the modal list (its
/// ShowModal returned when it showed them).
fn modals_closed() {
    if cooperative() {
        return;
    }
    for name in forms::modal_forms() {
        if !forms::modal_waits(&name) && !rapidr_ui_kernel::dialogs::is_dialog(&name) {
            forms::remove_modal(&name);
        }
    }
}

/// Every timer the program made, disabled (`Enabled = 0`): a ShowModal's
/// end, as the desktop's `rp_stop_all_timers`.
fn stop_timers() {
    for name in crate::object_web::timer_names() {
        rp_comp_set(&name, "enabled", Value::Integer(0));
    }
}

/// The timers due, fired (the app's heap: Interval and Enabled read again
/// each tick, armed again once the handler has run) — the interpreter's
/// handlers queued ahead of them first.
fn fire_timers() {
    use rapidr_ui_app::timers;
    timers::take_held_back();
    timers::fire_due(Web);
}

/// The page's turn after input or a deadline: what the kernel queued
/// dispatched, the waits served, the due timers fired, the test script's
/// step, then a frame asked for and the next deadline armed. Never while
/// the host is busy (a listener of its own on the stack): then a moment
/// later.
pub fn turn() {
    if TURNING.with(Cell::get) || host::busy() {
        later();
        return;
    }
    TURNING.with(|t| t.set(true));
    sync();
    dispatch_pending();
    serve_waits();
    modals_closed();
    fire_timers();
    serve_waits();
    script::step(Web);
    dispatch_pending();
    serve_waits();
    modals_closed();
    TURNING.with(|t| t.set(false));
    schedule();
    arm_deadline();
}

/// A turn as soon as the page's current task is over.
fn later() {
    if LATER.with(|l| l.replace(true)) {
        return;
    }
    let cb = Closure::once_into_js(|| {
        LATER.with(|l| l.set(false));
        turn();
    });
    if let Some(w) = web_sys::window() {
        let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), 0);
    }
}

/// Something changed: the windows drawn again at the next animation frame
/// (once, however many changes).
pub fn schedule() {
    if FRAME_ASKED.with(|f| f.replace(true)) {
        return;
    }
    let cb = Closure::once_into_js(frame);
    if let Some(w) = web_sys::window() {
        let _ = w.request_animation_frame(cb.unchecked_ref());
    }
}

/// The program changed what's drawn: drawn again.
pub fn redraw() {
    invalidate();
    schedule();
}

/// The program changed the component tree: built again, drawn.
pub fn rebuild() {
    restructure();
    schedule();
}

/// An animation frame: the windows the program made visible shown, the
/// owner-drawn lists' events, the program's changes onto the page, the
/// kernel's deadlines run, the dirty windows drawn; the next deadline armed.
fn frame() {
    FRAME_ASKED.with(|f| f.set(false));
    if TURNING.with(Cell::get) || host::busy() {
        later();
        return;
    }
    forms::show_pending(Web);
    lists::pre_paint(Web, &forms::shown_forms());
    sync();
    host::with(|h, store| h.desk.tick(store, rapidr_ui_kernel::tick::now()));
    dispatch_pending();
    // (what the handlers changed, into this frame)
    sync();
    host::with(|h, store| h.render(store));
    arm_deadline();
}

/// One timer for the earliest deadline: the kernel's (a caret's blink, a
/// held scroll bar's repeat), the program's next QTIMER, a test hook's
/// answer to a dialog and the test script's next step.
fn arm_deadline() {
    let now = rapidr_ui_kernel::tick::now();
    let kernel = host::with(|h, _| h.next_wake()).flatten();
    let next = [kernel, script::next_step(), rapidr_ui_app::timers::next_due(), rapidr_ui_app::dialogs::hook_wake()].into_iter().flatten().min();
    let Some(at) = next else { return };
    let ms = at.saturating_duration_since(now).as_secs_f64() * 1000.0;
    let Some(w) = web_sys::window() else { return };
    let perf_now = w.performance().map_or(0.0, |p| p.now());
    let due = perf_now + ms;
    if let Some((handle, when)) = DEADLINE.with(Cell::get) {
        if when <= due + 0.5 {
            return;
        }
        w.clear_timeout_with_handle(handle);
    }
    let cb = Closure::once_into_js(|| {
        DEADLINE.with(|d| d.set(None));
        turn();
    });
    if let Ok(handle) = w.set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), ms.ceil().clamp(0.0, 86_400_000.0) as i32) {
        DEADLINE.with(|d| d.set(Some((handle, due))));
    }
}

impl Windows for Web {
    fn start(self) {
        ensure();
    }
    fn started(self) -> bool {
        host::installed()
    }
    fn flush(self) {
        sync();
    }
    fn dispatch_pending(self) {
        dispatch_pending();
    }
    fn headless(self) -> bool {
        false
    }
    fn forced_scale(self) -> Option<f64> {
        None
    }
    fn window_scale(self, _form: &str) -> Option<f64> {
        host::with(|h, _| h.dpr)
    }
    fn screen(self) -> (i64, i64) {
        host::screen()
    }
    fn work_area(self) -> (i64, i64) {
        host::screen()
    }
    fn menu_in_window(self) -> bool {
        true
    }
    fn stacking(self) -> Vec<String> {
        host::with(|h, _| h.desk.stacking()).unwrap_or_default()
    }
    fn place_of(self, comp: &str) -> Option<(String, (i64, i64))> {
        host::with(|h, store| desktop::place_of(Web, &mut h.desk, store, comp)).flatten()
    }
    fn system_resized(self, form: &str, (iw, ih): (i64, i64), (left, top): (i64, i64)) {
        host::with(|h, store| {
            if let Some(f) = h.desk.forms.get_mut(form) {
                f.ui.sync(store);
            }
            h.desk.resized(form, iw, ih);
            h.desk.moved(form, left, top);
        });
    }
    fn open_popup(self, form: &str, menu: &str, x: i64, y: i64, _program: bool) {
        host::with(|h, store| {
            if let Some(f) = h.desk.forms.get_mut(form) {
                f.ui.sync(store);
                f.ui.open_popup(menu, x, y);
            }
        });
        redraw();
    }
    fn popup_open(self, form: &str) -> bool {
        host::with(|h, _| h.desk.forms.get(form).is_some_and(|f| f.ui.popup_open().is_some())).unwrap_or(false)
    }
    fn open_dialog(self, id: &str, title: &str, (w, h): (i64, i64)) {
        ensure();
        let (sw, sh) = host::screen();
        let (fw, fh) = rapidr_value::layout::form_frame(3);
        let spec = WindowSpec {
            title: title.to_string(),
            size: (w, h),
            position: Some(((sw - w - fw) / 2, (sh - h - fh) / 2)),
            border: true,
            icon: None,
            frame: rapidr_ui_app::desktop::Frame { resizable: false, close: true, minimize: false, maximize: false },
            state: 0,
        };
        host::with(|hst, store| {
            hst.desk.ensure_form(store, id, false, spec);
            hst.desk.show(id);
            hst.run_cmds(store);
        });
        schedule();
    }
    fn close_dialog(self, id: &str) {
        host::with(|h, store| {
            h.desk.forget(id);
            h.run_cmds(store);
        });
        schedule();
    }
    // (Stage W8, pulled forward: the page's Open / Save dialog — the
    // program's files, a name, Upload… — answered into FILES; the VM's wait
    // for it is the kernel host's, as any dialog's)
    fn ask_files(self, id: u64, _form: Option<&str>, req: &Request) {
        FILES.with(|f| f.borrow_mut().insert(id, None));
        let done = std::rc::Rc::new(move |paths: Vec<String>| {
            FILES.with(|f| f.borrow_mut().insert(id, Some(paths)));
            later();
        });
        crate::object_web::page_file_dialog(req.save, req.multi, req.title.as_deref().unwrap_or(""), &req.filters, req.filter_index, req.file_name.as_deref().unwrap_or(""), done);
    }
    fn files_answer(self, id: u64) -> Option<Vec<String>> {
        FILES.with(|f| {
            let mut f = f.borrow_mut();
            match f.get(&id) {
                Some(Some(_)) => f.remove(&id).flatten(),
                Some(None) => None,
                // (never asked: cancelled)
                None => Some(Vec::new()),
            }
        })
    }
    fn script_input(self, input: ScriptInput) {
        if matches!(input, ScriptInput::Hold(_)) {
            // (no system loop holds the page)
            return;
        }
        host::with(|h, store| desktop::script_input(Web, &mut h.desk, store, input));
    }
    fn capture_and_end(self, _prefix: &str) {
        test_end();
    }
}

// ------------------------------------------------------------ test hooks --

/// A GUI test's environment (`RAPIDR_CAPTURE`, `RAPIDR_TEST_EVENTS`,
/// `RAPIDR_TEST_DUMP` …, as the desktop's test hooks read them from the
/// process's): an object of names and values, set before the program runs.
pub fn set_test_env(vars: &JsValue) {
    let mut out = Vec::new();
    if let Ok(entries) = js_sys::Object::entries(vars.unchecked_ref::<js_sys::Object>()).dyn_into::<js_sys::Array>() {
        for e in entries.iter() {
            let pair: js_sys::Array = e.unchecked_into();
            if let (Some(k), Some(v)) = (pair.get(0).as_string(), pair.get(1).as_string()) {
                out.push((k, v));
            }
        }
    }
    testhooks::set_vars(out);
}

/// The test's end: `RAPIDR_TEST_DUMP`'s lines, each shown window's
/// accessibility tree and its capture (the wasm's own pixels, as the
/// desktop's `RAPIDR_CAPTURE` BMP: the same bytes for the same pixels),
/// bottom to top, kept for `rapidr_test_results`; the program ends.
fn test_end() {
    let dump = testhooks::dump_lines(&testhooks::parse_dump(&testhooks::var("RAPIDR_TEST_DUMP").unwrap_or_default()), |c| script::shown_up(Web, c), |comp, prop| rp_comp_get(comp, prop).to_string_val());
    sync();
    let shots = host::with(|h, store| {
        let order = h.desk.stacking();
        let mut trees = Vec::new();
        let mut shots = Vec::new();
        for f in &order {
            trees.extend(h.desk.access_json(store, f));
            if let Some(px) = h.capture(store, f) {
                let title = h.desk.forms.get(f).map(|w| w.spec.title.clone()).unwrap_or_default();
                shots.push((title, rapidr_value::objects::codec::encode_bmp(&px)));
            }
        }
        (trees, shots)
    });
    let (trees, shots) = shots.unwrap_or_default();
    let q = |s: &str| {
        let mut out = String::from("\"");
        for c in s.chars() {
            match c {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
                c => out.push(c),
            }
        }
        out.push('"');
        out
    };
    let dump: Vec<String> = dump.iter().map(|l| q(l)).collect();
    let shots: Vec<String> = shots.iter().map(|(t, b)| format!("{{\"title\":{},\"bmp\":\"{}\"}}", q(t), base64(b))).collect();
    // (the trees as RAPIDR_TEST_A11Y's file has them, a string: their ids are
    // 64-bit, more than a JavaScript number keeps)
    let a11y = format!("[{}]\n", trees.join(",\n"));
    let json = format!("{{\"dump\":[{}],\"a11y\":{},\"captures\":[{}]}}", dump.join(","), q(&a11y), shots.join(","));
    RESULTS.with(|r| *r.borrow_mut() = Some(json));
    // (the desktop's process exits here; the page stays as it is, its
    // windows and their mirrors for the test to look at — the script is
    // over, so nothing more is fired at the program)
}

/// A GUI test's results once its script ended (JSON: `dump`, `a11y`,
/// `captures`), `None` before.
pub fn test_results() -> Option<String> {
    RESULTS.with(|r| r.borrow().clone())
}

fn base64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let n = (u32::from(c[0]) << 16) | (u32::from(*c.get(1).unwrap_or(&0)) << 8) | u32::from(*c.get(2).unwrap_or(&0));
        for i in 0..4 {
            if i <= c.len() {
                out.push(T[((n >> (18 - 6 * i)) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

// --------------------------------------------- the facade (object_web, gui_web) --
//
// What `object_web` and `gui_web` ask of the GUI while the kernel hosts:
// the desktop facade's calls (runtime-core's `ui/kernel.rs`), into
// `rapidr_ui_app`.

/// A component made: the tree built again (its form's, once shown).
pub fn created(_name: &str) {
    rebuild();
}

/// A property the program set (stored already): what follows it on the
/// desktop — a form's window titled, moved, sized, shown or hidden, its
/// frame, icon and state; anything else drawn again.
pub fn set_prop(name: &str, prop: &str, val: &Value) {
    let is_form = forms::is_form(Web, name);
    match prop {
        "visible" if is_form => {
            if val.to_bool() {
                forms::show_visible(Web, name);
            } else {
                forms::set_visible(Web, name, false);
            }
        }
        "visible" => forms::set_visible(Web, name, val.to_bool()),
        "caption" => forms::set_caption(Web, name, &val.to_string_val()),
        "left" | "top" if is_form => forms::move_form(Web, name),
        "left" | "top" | "width" | "height" => forms::apply_geometry(Web, name),
        "borderstyle" if is_form => forms::set_form_border(Web, name),
        "icon" | "icohandle" if is_form => forms::apply_icon(Web, name),
        "windowstate" if is_form => {
            let now = val.to_i64();
            let from = STATES.with(|s| s.borrow_mut().insert(lower(name), now)).unwrap_or(rapidr_value::window_state::WS_NORMAL);
            forms::set_window_state(Web, name, from);
        }
        "parent" => restructure(),
        "down" => forms::toggle_down_set(Web, name),
        _ => invalidate(),
    }
    schedule();
}

/// What only the GUI knows of a property (`gui_web_get_prop`'s, the live
/// DOM's in the DOM host): a form's Visible is whether its window shows (as
/// the desktop's); everything else is the store's (Null: read it there).
pub fn get_prop(name: &str, prop: &str) -> Value {
    if prop == "visible" && forms::is_form(Web, name) {
        if let Some(shown) = forms::window_shown(name) {
            return Value::Boolean(shown);
        }
    }
    Value::Null
}

/// A component's method the GUI does (`gui_web_method`'s): `None` for the
/// ones the kernel host leaves to the rest.
pub fn method(name: &str, comp_type: &str, method: &str, args: &[Value]) -> Option<Value> {
    let v = Value::Null;
    match (comp_type, method) {
        // (the lists lane's: the drawn tree's rows)
        ("RTREEVIEW", "getitemat") => {
            let arg = |i: usize| args.get(i).map_or(0, Value::to_i64);
            return Some(Value::Integer(lists::tree_item_at(name, arg(0), arg(1))));
        }
        ("RFORM", "show") => forms::show(Web, name),
        // (the interpreter waits until the form closes: a wait it serves,
        // its result the ModalResult; a native web build's returns at once)
        ("RFORM", "showmodal") => {
            forms::begin_modal(Web, name);
            begin_wait(rapidr_ui_app::waits::Wait::Form(lower(name)));
        }
        ("RFORM", "hide") => forms::hide(Web, name),
        ("RFORM", "close") => forms::close(Web, name),
        ("RFORM", "center") => forms::center(Web, name),
        ("RFORM", "addbordericons" | "delbordericons") => {
            let bits = rapidr_value::builtins::border_icons(&crate::object_web::rp_comp_get_stored(name, "bordericons"), args, method == "addbordericons");
            rp_comp_set(name, "bordericons", bits);
            redraw();
        }
        ("RCANVAS" | "RFORM", "refresh" | "repaint" | "update" | "paint") => {
            invalidate();
            rp_fire_event(name, "onpaint");
        }
        (_, "refresh" | "repaint" | "invalidate" | "update") => invalidate(),
        (_, "show") => forms::show(Web, name),
        (_, "hide") => forms::hide(Web, name),
        (_, "setfocus" | "focus") => {
            let form = form_of(name).map(|f| lower(&f));
            if let Some(form) = form {
                host::with(|h, store| {
                    if let Some(f) = h.desk.forms.get_mut(&form) {
                        f.ui.sync(store);
                        f.ui.focus_id(store, &lower(name));
                        f.ui.dirty = true;
                    }
                });
            }
        }
        (_, "setparent") if !args.is_empty() => {
            rp_comp_set(name, "parent", Value::String(args[0].to_string_val()));
            restructure();
        }
        _ => return None,
    }
    schedule();
    Some(v)
}

/// `Form.Close` / a modal form's ModalResult set (gui_web's `close_form`).
pub fn close_form(name: &str) {
    forms::close(Web, name);
    schedule();
}

/// A form hidden without OnClose (END).
pub fn hide_form(name: &str) {
    forms::hide_window(name);
    schedule();
}

/// Whether form `name`'s window was made (it showed once).
pub fn form_window_exists(name: &str) -> bool {
    forms::form_window_exists(name)
}

pub fn any_form_shown() -> bool {
    forms::any_shown()
}

/// Whether `name` shows: visible up to its form, whose window shows (the
/// tests' `__shown`).
pub fn element_shown(name: &str) -> bool {
    script::shown_up(Web, name)
}

/// The main program is over (or waits in a ShowModal): the windows it made
/// visible show.
pub fn finalize() {
    ensure();
    forms::show_pending(Web);
    schedule();
}

/// MOUSEX / MOUSEY: the pointer in the topmost form's client area.
pub fn mouse_in_form() -> (i64, i64) {
    host::with(|h, _| {
        let (mx, my) = h.mouse;
        let top = h.desk.stacking().last().cloned();
        let Some(f) = top.and_then(|t| h.desk.forms.get(&t).map(|f| (f.spec.position.unwrap_or((0, 0)), f.spec.border, f.ui.menu_offset))) else { return (mx as i64, my as i64) };
        let ((x, y), border, menu) = f;
        let (ix, iy) = rapidr_ui_host_web::frame::inset(border);
        (mx as i64 - x - ix, my as i64 - y - iy - menu)
    })
    .unwrap_or((0, 0))
}

/// `RAPIDR_TEST_RESIZE` / `rapidr_test_resize` in the browser: form `name`
/// resized to `w` × `h` (Width, Height) as a user's drag would.
pub fn test_resize(name: &str, w: i64, h: i64) {
    let (fw, fh) = rapidr_value::layout::form_frame(rp_comp_get(name, "borderstyle").to_i64());
    let form = lower(name);
    host::with(|hst, store| {
        if let Some(f) = hst.desk.forms.get_mut(&form) {
            f.ui.sync(store);
        }
        hst.desk.resized(&form, w - fw, h - fh);
    });
    push_op(WindowOp::Size(form, (w - fw, h - fh)));
    turn();
}

/// `PopupMenu.Popup(X, Y)`.
pub fn popup(name: &str, x: i64, y: i64) {
    rapidr_ui_app::menus::popup(Web, name, x as i32, y as i32);
}

/// A QTREEVIEW changed: OnDeletion for the nodes the program deleted (the
/// desktop's `tree_refresh`), drawn again.
pub fn tree_refresh(name: &str) {
    lists::tree_refresh(Web, &lower(name));
    redraw();
}

/// A generated web program's form shown (its `Show` from the prelude).
pub fn show_form(name: &str) {
    forms::show(Web, name);
    schedule();
}

// ------------------------------------------- timers, dialogs, waits (W4) --

/// A QTIMER the program made (`__gui_register_timer`): it ticks while the
/// program waits, in the app's heap.
pub fn register_timer(name: &str) {
    rapidr_ui_app::timers::register(name);
}

/// A timer's Enabled or Interval changed (or its OnTimer was bound): it
/// ticks if it's enabled now and the program's windows started.
pub fn timer_changed(name: &str) {
    rapidr_ui_app::timers::changed(Web, name);
    arm_deadline();
}

/// Whether the interpreter waits for the page's kernel-drawn dialogs (a
/// native web build keeps the page's own, which can answer at once).
pub fn dialogs_here() -> bool {
    cooperative()
}

/// MESSAGEBOX / MESSAGEDLG / SHOWMESSAGE / MSGBOX: the kernel's message
/// box (`rapidr_ui_app::dialogs::message`), the desktop's: `then` maps the
/// button chosen (`None`: Escape, the close box) to the builtin's result —
/// given at once by a test's hook, else the end of a wait the VM serves
/// (its placeholder result `Null` here). `None`: not here (a native web
/// build: the page's dialog).
pub fn choice(title: &str, text: &str, labels: &[&str], icon: Option<rapidr_value::dialogs::MsgIcon>, beep: bool, then: impl FnOnce(Option<usize>) -> Value + 'static) -> Option<Value> {
    if !dialogs_here() {
        return None;
    }
    ensure();
    match rapidr_ui_app::dialogs::message(Web, title, text, labels, icon, beep, then) {
        rapidr_ui_app::dialogs::Pending::Done(v) => Some(v),
        rapidr_ui_app::dialogs::Pending::Open(id) => {
            begin_wait(rapidr_ui_app::waits::Wait::Dialog(id));
            Some(Value::Null)
        }
    }
}

/// `Dialog.Execute` of a QCOLORDIALOG / QFONTDIALOG (and, W8, the file
/// dialogs): the kernel's dialog, its result as `choice`'s. `None`: not
/// here.
pub fn execute(name: &str, comp_type: &str) -> Option<Value> {
    if !dialogs_here() {
        return None;
    }
    ensure();
    match rapidr_ui_app::dialogs::execute(Web, &lower(name), comp_type) {
        rapidr_ui_app::dialogs::Pending::Done(v) => Some(v),
        rapidr_ui_app::dialogs::Pending::Open(id) => {
            begin_wait(rapidr_ui_app::waits::Wait::Dialog(id));
            Some(Value::Null)
        }
    }
}

/// INPUT$'s wait with windows (the desktop's `gui_wait_key`): 1 when a key
/// is in INKEY$'s queue; else the VM waits (`Wait::Key`: 1 when a key
/// comes, 0 when no window is left). `None`: no window shown (the page's
/// own keys, as before).
pub fn wait_key() -> Option<Value> {
    if !host::installed() || !forms::any_shown() || !cooperative() {
        return None;
    }
    rapidr_ui_app::timers::start_all(Web);
    if rapidr_value::console::key_waiting() {
        return Some(Value::Integer(1));
    }
    begin_wait(rapidr_ui_app::waits::Wait::Key).then_some(Value::Null)
}

/// `DOEVENTS` (the desktop's): the timers start; the windows the program
/// made visible show; then a turn, in which every timer due now fires
/// before it returns — only when one is due (a `DO: DOEVENTS: LOOP` runs
/// at full speed otherwise; the time slices give the page its turns).
/// `false`: not a wait (nothing due, or a native build).
pub fn doevents() -> bool {
    if !host::installed() {
        return false;
    }
    rapidr_ui_app::timers::start_all(Web);
    forms::show_pending(Web);
    let now = rapidr_ui_kernel::tick::now();
    let due = rapidr_ui_app::timers::next_due().is_some_and(|at| at <= now);
    // (or the time slice is over: the page gets its turn — painting, input —
    // inside this wait too, not in a pause of the page's own)
    (due || crate::dialog_web::slice_over()) && begin_wait(rapidr_ui_app::waits::Wait::Once(now))
}

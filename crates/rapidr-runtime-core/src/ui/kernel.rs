//! The UI kernel as the runtime's desktop host (the only one;
//! docs/desktop-host-plan.md §1.3–§1.5): the facade's functions for the
//! kernel (`rapidr_ui_kernel`) and its winit or headless host
//! (`rapidr_ui_host_winit`). What doesn't depend on the host — the kernel's
//! events as the program's, forms shown and closed, the modal list, the
//! timers, the waits' bookkeeping, menus, the test script — is
//! `rapidr_ui_app`'s, shared with the web runtime (docs/web-host-plan.md,
//! Stage W2); it works through [`Rt`] (the program: `program.rs`; the
//! windows: here). This file keeps what pumps or waits for the host.
//!
//! **Program code runs only here, between pumps** — in [`step`]:
//!
//! ```text
//! step(max_wait):
//!   show_pending()                    forms made Visible show
//!   t = min(max_wait, next timer, next test step)
//!   pump(t)                           the host's callbacks route input into
//!                                     the kernel; HostEvents queued
//!   run the handlers the safety net deferred
//!   dispatch HostEvents               OnClick, OnKeyDown, OnClose, OnResize …
//!   fire due timers                   Interval / Enabled read again each tick
//!   one test-script step
//! ```
//!
//! A native handler may call `ShowModal`, which steps again (a nested pump,
//! legal: no host callback is on the stack). The interpreter keeps its
//! protocol: `ShowModal` starts a wait the VM serves with [`gui_pump_wait`],
//! one step each, and its handlers run after the pump returns. So does every
//! builtin that waits for the user — DOEVENTS, Popup, the message boxes, the
//! file / colour / font dialogs, INPUT$ (`rapidr_ui_app::waits::Wait`) —
//! never inside the builtin, where the VM's handlers could only queue: its
//! timers' handlers run during a dialog as a native build's do, and the wait
//! ends with the builtin's result. Timers fire only once the interpreter's
//! handlers queued ahead of them have run (as a native build's run as
//! they're dispatched): `step` holds them back for the next step otherwise.
//!
//! While the system holds a pump (a native menu tracked, Windows' size /
//! move loop), the host's tracking tick gives the program a turn from inside
//! it ([`tracking_tick`]): the due timers fire and what they change is
//! drawn, as RapidQ's WM_TIMERs during a menu. Nothing waits in there
//! ([`held`]).
//!
//! What the host needs from the program goes into queues the next pump
//! takes (window commands, "something changed": `rapidr_ui_app::windows`);
//! what the facade is asked (Visible, Scale, Handle …) is answered from
//! state kept there, never from the host, which is borrowed while it pumps.

use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::time::{Duration, Instant};

use rapidr_ui_app::waits::{self, Wait};
use rapidr_ui_app::windows::{invalidate, push_op, restructure, take_notify, take_ops};
use rapidr_ui_app::{forms, lists, script, timers, ScriptInput, WindowOp, Windows};
use rapidr_ui_host_winit::{Desktop, Host, HostEvent, WindowSpec};
use rapidr_ui_kernel::Clipboard;

use super::kernel_store::RtStore;
use super::program::Rt;
use super::testhooks::Capture;
use crate::object::{rp_comp_get, rp_comp_set};
use crate::value::{v_int, v_null, Value};

// The buttons and menus lane's part: Popup's and AutoPopup's menu shown.
mod menus;
// The dialogs and platform lane's: kernel-drawn message boxes, colour and
// font dialogs, rfd's Open / Save sheets; window frames, cursors, $THEME.
pub(super) mod dialogs;
mod platform;

// ------------------------------------------------------------------ state --

/// The host and the kernel's forms: borrowed while the host pumps.
struct Kern {
    host: Box<dyn Host>,
    desk: Desktop,
}

#[derive(Default)]
struct State {
    theme: String,
    // (timers during native menu tracking)
    /// The screen, the work area and the monitors as the host last said
    /// (answered while the host pumps: a tracking tick's handlers).
    screen: Option<(i64, i64)>,
    work_area: Option<(i64, i64)>,
    monitors: Option<i64>,
    /// What tracking ticks couldn't do inside the system's loop, said once
    /// each.
    held_warned: HashSet<&'static str>,
    /// A tick queued something for after the held loop (a message box):
    /// the host is asked to end it.
    end_loop: bool,
}

thread_local! {
    /// Made once, never dropped: winit windows dropped during the main
    /// thread's TLS teardown abort the process (Stage 0), so the host lives
    /// until the process ends.
    static KERN: Cell<Option<&'static RefCell<Kern>>> = const { Cell::new(None) };
    static ST: RefCell<State> = RefCell::new(State::default());
    /// (timers during native menu tracking) Inside a tracking tick: the
    /// system holds the pump, nothing may wait.
    static HELD: Cell<bool> = const { Cell::new(false) };
}

fn st<R>(f: impl FnOnce(&mut State) -> R) -> R {
    ST.with(|s| f(&mut s.borrow_mut()))
}

fn kern() -> Option<&'static RefCell<Kern>> {
    KERN.with(Cell::get)
}

/// Runs `f` on the host and the kernel's forms (`None` before the host
/// started, or while it pumps).
fn with_kern<R>(f: impl FnOnce(&mut Kern) -> R) -> Option<R> {
    let k = kern()?;
    let mut k = k.try_borrow_mut().ok()?;
    Some(f(&mut k))
}

fn lower(name: &str) -> String {
    name.to_lowercase()
}

/// `RAPIDR_SCALE`: the screen's scale for tests.
fn forced_scale() -> Option<f64> {
    std::env::var("RAPIDR_SCALE").ok().and_then(|s| s.parse::<f64>().ok()).filter(|s| *s > 0.0)
}

/// The OS's clipboard as the kernel's edits use it (the runtime's, which
/// keeps `RAPIDR_TEST_CLIPBOARD`).
struct RtClipboard;

impl Clipboard for RtClipboard {
    fn get_text(&mut self) -> Option<String> {
        Some(crate::globals::clipboard_text())
    }
    fn set_text(&mut self, text: &str) {
        crate::globals::set_clipboard_text(text);
    }
}

/// The host, started the first time a window is needed: headless under a
/// GUI test (`RAPIDR_CAPTURE` without `RAPIDR_CAPTURE_WINDOWS`), else winit.
fn ensure_host() {
    if kern().is_some() {
        return;
    }
    let capture = Capture::from_env();
    let headless = capture.is_some() && std::env::var_os("RAPIDR_CAPTURE_WINDOWS").is_none();
    let mut host = rapidr_ui_host_winit::new_host(headless, forced_scale());
    // (timers during native menu tracking: the program's turn inside a held
    // pump)
    host.set_tracking_hook(Box::new(|desk, _store| tracking_tick(desk)));
    let mut desk = Desktop::new(Box::new(RtClipboard));
    // (the text lane's: carets blink, but not under a test — captures must
    // be steady)
    desk.blinks = capture.is_none();
    // (a form is drawn at the screen's scale before its window is made)
    desk.default_scale = host.default_scale();
    if let Some(c) = capture {
        // (only the test's own events drive it)
        desk.ignore_user = true;
        let next = Instant::now() + Duration::from_secs_f64(c.delay.max(0.0));
        script::start(c, next);
    }
    let k: &'static RefCell<Kern> = Box::leak(Box::new(RefCell::new(Kern { host, desk })));
    KERN.with(|c| c.set(Some(k)));
}

fn started() -> bool {
    kern().is_some()
}

// ------------------------------------------------------------- the pump --

/// What the program changed, into the kernel's forms: window commands,
/// new forms' kernel sides, trees rebuilt, layouts read again
/// (`rapidr_ui_app::desktop`'s, shared with the web host).
fn sync_desk(desk: &mut Desktop) {
    rapidr_ui_app::menus::dump_if_changed(Rt);
    let store = RtStore;
    let notify = take_notify();
    let ops = take_ops();
    let modal = forms::modal_forms();
    for op in ops {
        desk.apply(&store, op, menu_in_window(), spec_of);
    }
    platform::sync(desk);
    desk.sync_forms(&store, modal, notify);
}

/// The host's turn: up to `timeout` (`None`: until something happens).
/// Program code fired meanwhile waits ([`crate::object::rp_in_host_callback`]).
fn pump(timeout: Option<Duration>) {
    // (a tracking tick's handler: the host pumps already, held by the system)
    if held() {
        return;
    }
    let Some(k) = kern() else {
        if let Some(t) = timeout {
            std::thread::sleep(t);
        }
        return;
    };
    crate::object::rp_in_host_callback(|| {
        let Ok(mut k) = k.try_borrow_mut() else { return };
        sync_desk(&mut k.desk);
        let Kern { host, desk } = &mut *k;
        // (the system tray's icons, when the program changed them)
        let tray = rapidr_value::tray::revision();
        if TRAY_SHOWN.with(|t| t.replace(tray)) != tray {
            host.tray_sync(&rapidr_value::tray::shown());
        }
        // (the kernel's deadlines due: tick.rs)
        desk.tick(&RtStore, rapidr_ui_kernel::tick::now());
        host.pump(timeout, desk, &RtStore);
    });
}

thread_local! {
    /// The tray's revision the host shows (rapidr_value::tray).
    static TRAY_SHOWN: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// One step of the innermost wait (see the module's doc).
pub fn step(max_wait: Option<Duration>) {
    // (inside the system's loop nothing waits: see `tracking_tick`)
    if held() {
        return;
    }
    ensure_host();
    forms::show_pending(Rt);
    // (the interpreter: timers held back for the handlers queued ahead of
    // them, which the VM has run since — or which wait, and this is their
    // wait's step: the timers fire now, before anything else)
    if timers::take_held_back() && timers::fire_due(Rt) {
        return;
    }
    // (the lists lane's owner-draw events, before the windows are drawn)
    lists::pre_paint(Rt, &forms::shown_forms());
    let now = Instant::now();
    let mut t = max_wait;
    let mut at_most = |d: Duration| t = Some(t.map_or(d, |t| t.min(d)));
    if let Some(at) = timers::next_due() {
        at_most(at.saturating_duration_since(now));
    }
    if let Some(at) = script::next_step() {
        at_most(at.saturating_duration_since(now));
    }
    if with_kern(|k| !k.desk.events.is_empty()).unwrap_or(false) {
        at_most(Duration::ZERO);
    }
    // (the kernel's deadlines: a caret's blink, a held scroll bar's repeat)
    if let Some(at) = with_kern(|k| k.desk.next_wake()).flatten() {
        at_most(at.saturating_duration_since(now));
    }
    // (a test hook's answer to a dialog, once it has been open long enough)
    if let Some(at) = rapidr_ui_app::dialogs::hook_wake() {
        at_most(at.saturating_duration_since(now));
    }
    // (work waited for in the background — a QDOWNLOAD's transfer: its
    // progress shown, its end noticed)
    if rapidr_ui_app::dialogs::tasks_open() {
        at_most(rapidr_ui_app::dialogs::TASK_STEP);
    }
    let queued = crate::object::rp_vm_events_queued();
    pump(t);
    crate::object::rp_run_deferred();
    dispatch_pending();
    // (the system tray's clicks: the form's WndProc)
    for (key, mouse) in with_kern(|k| k.host.tray_clicks()).unwrap_or_default() {
        rapidr_ui_app::tray::deliver(Rt, key, &mouse);
    }
    rapidr_ui_app::dialogs::give_hooked(Rt);
    // The timers fire once the handlers before them have run, as in a
    // native build (its handlers run as they're dispatched). The
    // interpreter's were only queued: the timers wait for the next step,
    // after the VM has run them — or inside the wait one of them starts, so
    // a timer's handler is never queued behind a handler waiting for a
    // dialog (silent all through it).
    if crate::object::rp_vm_events_queued() != queued {
        timers::hold_back();
    } else if timers::fire_due(Rt) {
        // (a timer's handler queued for the VM runs before anything else)
        return;
    }
    script::step(Rt);
}

/// The events the host queued, handled (each to completion; a handler may
/// step again).
fn dispatch_pending() {
    for e in with_kern(|k| std::mem::take(&mut k.desk.events)).unwrap_or_default() {
        match e {
            // (a kernel-drawn dialog's: never the program's)
            HostEvent::Kernel(form, ev) if rapidr_ui_kernel::dialogs::is_dialog(&form) => rapidr_ui_app::dialogs::event(Rt, &form, ev),
            HostEvent::Kernel(_, ev) => rapidr_ui_app::dispatch::dispatch(Rt, ev),
            HostEvent::Wake => {}
        }
    }
}

// --------------------------------------------------------------- timers --

/// A QTIMER the program made (generated programs call it).
pub fn gui_register_timer(name: &str) {
    timers::register(name);
}

/// A timer's Enabled or Interval changed: it ticks if it's enabled now.
pub fn gui_timer_changed(name: &str) {
    timers::changed(Rt, name);
}

// ------------------------------------------ timers during menu tracking --
//
// (docs/desktop-host-plan.md, "Timers during native menu tracking") While
// the system holds a pump — a native menu tracked (macOS' menu bar, the
// host's context menu), Windows' size / move loop — the host's tracking
// timer gives the program turns from inside it (`rapidr_ui_host_winit::
// tracking`), as Windows' modal loops dispatch RapidQ's WM_TIMERs. A turn
// runs only what can finish in there:
//
// - The due timers fire. Their handlers run as from a step: native ones
//   directly (the host callback flag lifted: `rp_program_turn`), the VM's by
//   the wait it's in, which lent itself (`rp_serve_program`), each before
//   the next timer. Owner-draw events follow, then what they all changed
//   goes into the kernel's forms and the host draws them.
// - Nothing waits ([`held`]): a nested pump can't run inside the system's
//   loop. DOEVENTS returns at once (this is the program's turn). Popup's
//   menu, SHOWMESSAGE / MSGBOX and a one-button MESSAGEBOX / MESSAGEDLG show
//   once the held loop ends (the menu is closed for a box, as Windows
//   closes a menu when a dialog takes the focus); the handler goes on at
//   once (a one-button box answers its button). ShowModal, a MESSAGEBOX /
//   MESSAGEDLG with a choice, the Open / Save / colour / font dialogs and
//   INPUT$'s wait need the user's answer now and can't have it: they answer
//   as dismissed at once (mrCancel, the Escape answer, Cancel, a closed
//   window), and say so once on stderr.
// - Clicks and keys queued before the loop took the mouse, and the
//   handlers the safety net deferred, wait for the pump (they may need it).

/// Inside a tracking tick: the system holds the pump, so nothing may wait.
pub(super) fn held() -> bool {
    HELD.with(Cell::get)
}

/// A wait a tracking tick's handler asked for and can't have: said once.
pub(super) fn held_cannot(what: &'static str) {
    if st(|s| s.held_warned.insert(what)) {
        eprintln!("[rapidr] {what} while a menu holds the window system: answered as dismissed (nothing can wait for the user there)");
    }
}

/// Something shows once the held loop is over (`job`, after the pump): the
/// host is asked to end the loop (the menu closes).
pub(super) fn after_held(job: Box<dyn FnOnce()>) {
    crate::object::rp_defer_job(job);
    st(|s| s.end_loop = true);
}

/// The host's tracking tick (`set_tracking_hook`): the program's turn while
/// the system holds the pump — the due timers' handlers, the owner-draw
/// events after them, what they changed into the kernel's forms. Answers
/// when the next one is due.
fn tracking_tick(desk: &mut Desktop) -> rapidr_ui_host_winit::tracking::Turn {
    struct Held(bool);
    impl Drop for Held {
        fn drop(&mut self) {
            HELD.with(|h| h.set(self.0));
        }
    }
    let _held = Held(HELD.with(|h| h.replace(true)));
    crate::object::rp_program_turn(|| {
        timers::fire_due_then(Rt, crate::object::rp_serve_program);
        lists::pre_paint(Rt, &forms::shown_forms());
        crate::object::rp_serve_program();
    });
    sync_desk(desk);
    desk.tick(&RtStore, rapidr_ui_kernel::tick::now());
    let next = [timers::next_due(), desk.next_wake()].into_iter().flatten().min();
    rapidr_ui_host_winit::tracking::Turn { next, end_loop: st(|s| std::mem::take(&mut s.end_loop)) }
}

// ---------------------------------------------------------------- forms --

/// Whether a QMAINMENU is a bar inside its form's window: everywhere but
/// macOS (its system menu bar), unless `RAPIDR_MENU=window`.
fn menu_in_window() -> bool {
    !cfg!(target_os = "macos") || std::env::var("RAPIDR_MENU").is_ok_and(|v| v.eq_ignore_ascii_case("window"))
}

/// The height of a form's in-window main menu (0 without one, and on
/// macOS).
pub fn menu_offset(form: &str) -> i32 {
    forms::menu_offset(Rt, form)
}

fn spec_of(name: &str) -> WindowSpec {
    rapidr_ui_app::desktop::window_spec(Rt, name)
}

/// (the DirectX lane's) The program is the active application (one of its
/// windows has the keyboard; always on the headless host, and under a GUI
/// test, whose script is the user — real windows there needn't get the
/// keyboard from the system).
pub fn app_active() -> bool {
    rapidr_ui_app::testhooks::under_test() || with_kern(|k| k.host.active()).unwrap_or(true)
}

/// The host has no system to ask (the headless host of the GUI tests).
fn headless() -> bool {
    // (while the host pumps — a tracking tick's handler — the one started)
    with_kern(|k| k.host.headless()).unwrap_or_else(|| !started() || headless_known())
}

/// (the WindowState lane's) `Form.WindowState` set (it was `from`):
/// `rapidr_ui_app::forms::set_window_state` — the system maximizes,
/// minimizes or restores the window; the headless host's is simulated.
pub fn gui_set_window_state(name: &str, from: i64) {
    forms::set_window_state(Rt, name, from);
}

// ---------------------------------------------------- the facade: draw --

pub fn redraw_widget(_name: &str) {
    invalidate();
}
pub fn gui_redraw(_name: &str) {
    invalidate();
}
pub fn canvas_redraw(_name: &str) {
    invalidate();
}
pub fn picture_refresh(_name: &str) {
    invalidate();
}
pub fn tab_control_changed(_name: &str) {
    invalidate();
}
pub fn list_refresh(_name: &str) {
    invalidate();
}
pub fn listview_refresh(_name: &str) {
    invalidate();
}
pub fn grid_refresh(_name: &str) {
    invalidate();
}
pub fn tree_refresh(name: &str) {
    lists::tree_refresh(Rt, name);
    invalidate();
}
pub fn header_refresh(_name: &str) {
    invalidate();
}
pub fn dirtree_refresh(_name: &str) {
    invalidate();
}
pub fn gui_apply_font(_name: &str) {
    invalidate();
}
/// The program set a QCOOLBTN's / QOVALBTN's Down: the others of its
/// group come up (host-neutral: rapidr_value::toggle_group).
pub fn toggle_down_set(name: &str) {
    forms::toggle_down_set(Rt, name);
}
pub fn schedule_menu_sync() {
    invalidate();
}

/// A caption: a form's is its window's title.
pub fn gui_set_caption(name: &str, text: &str) {
    forms::set_caption(Rt, name, text);
}

// ----------------------------------------------- the facade: structure --

pub fn attach_late(_name: &str) {
    restructure();
}
pub fn gui_set_parent(_child: &str, _parent: &str) {
    restructure();
}
pub fn gui_widget_add_items(_name: &str, _items: &str) {
    invalidate();
}
pub fn gui_widget_clear(_name: &str) {
    invalidate();
}
pub fn stack_widgets(_names: &[String]) {
    restructure();
}
pub fn ensure_menu_widget(_name: &str) {
    restructure();
}

/// `Visible`: a built form's window shows or hides (no OnShow: Show fires
/// that); a component is read from the store when painted.
pub fn gui_set_visible(name: &str, visible: bool) {
    forms::set_visible(Rt, name, visible);
}

/// Left / Top / Width / Height: a form's window takes its new size.
pub fn gui_apply_geometry(name: &str) {
    forms::apply_geometry(Rt, name);
}

// ------------------------------------------------- the facade: windows --

/// Shows a form without waiting (OnShow when it wasn't showing).
pub fn gui_show(name: &str) {
    forms::show(Rt, name);
}

/// `Form.Visible = True`: its Show; a form not built yet (its own CREATE)
/// shows once the program waits.
pub fn gui_show_visible(name: &str) {
    forms::show_visible(Rt, name);
}

/// Hides a form's window (no OnClose).
pub fn gui_hide(name: &str) {
    forms::hide(Rt, name);
}

/// `Form.Close` and the window's close box: OnClose's `Action` (it starts
/// as `caHide`) decides whether the form goes, stays or is minimized.
pub fn gui_close(name: &str) {
    forms::close(Rt, name);
}

/// The screen's size (logical pixels).
fn screen() -> (i64, i64) {
    ensure_host();
    // (the winit host knows its monitor after its first pump)
    if with_kern(|k| k.host.headless()) == Some(false) && !forms::any_built() {
        pump(Some(Duration::ZERO));
    }
    // (while the host pumps — a tracking tick's handler — as it last said)
    match with_kern(|k| k.host.screen()) {
        Some(s) => {
            st(|st| st.screen = Some(s));
            s
        }
        None => st(|s| s.screen).unwrap_or(rapidr_ui_host_winit::HEADLESS_SCREEN),
    }
}

/// `Form.Center`: on the screen's middle (when shown; ShowModal centres a
/// form asked to be before it showed).
pub fn gui_center(name: &str) {
    forms::center(Rt, name);
}

/// `Form.Left` / `Form.Top` set by the program: the window moves there.
pub fn gui_move_form(name: &str) {
    forms::move_form(Rt, name);
}

/// `Form.BorderStyle`: bsNone (0) takes away the window's frame.
pub fn gui_set_form_border(name: &str) {
    forms::set_form_border(Rt, name);
}

pub fn gui_apply_icon(name: &str) {
    forms::apply_icon(Rt, name);
}

/// `Application.Icon` changed: every form without its own.
pub fn gui_apply_icons() {
    forms::apply_icons(Rt);
}

pub fn gui_menu_popup(name: &str, x: i32, y: i32) {
    rapidr_ui_app::menus::popup(Rt, name, x, y);
}

// ---------------------------------------------------- the facade: text --
//
// The kernel's edits read the shared text model (`TextEdit::revision`) and
// write the user's edits into it as they're typed: nothing to push or pull.

pub fn text_push(_name: &str) {
    invalidate();
}
pub fn gui_set_text(_name: &str, _text: &str) {
    invalidate();
}
pub fn gui_set_input_value(_name: &str, _text: &str) {
    invalidate();
}
pub fn text_pull(_name: &str) {}

/// A QMEMO's / QRICHEDIT's Text: the store's (the text lane draws them).
pub fn gui_get_text(name: &str) -> String {
    crate::object::stored(&lower(name), "text").map(|v| v.to_string_val()).unwrap_or_default()
}

pub fn gui_get_input_value(_name: &str) -> Option<String> {
    None
}

// --------------------------------------------------- the facade: waits --

/// Shows a form modally: a native build steps until it closes (handlers
/// may nest another); the interpreter is left a wait it serves with
/// [`gui_pump_wait`]. Returns its ModalResult.
pub fn gui_showmodal(name: &str) -> i64 {
    ensure_host();
    let name = lower(name);
    // (a tracking tick's handler: nothing waits inside the system's loop —
    // closed at once, as by its close box)
    if held() {
        held_cannot("ShowModal");
        return rapidr_value::events::modal_result(0);
    }
    forms::begin_modal(Rt, &name);
    if waits::cooperative() {
        waits::start(Wait::Form(name));
        return 0;
    }
    forms::show_pending(Rt);
    while forms::modal_waits(&name) {
        step(None);
    }
    // (the timers stop with the modal form)
    crate::object::rp_stop_all_timers();
    forms::modal_ended(Rt, &name)
}

/// Whether `name` is shown modally now (setting its ModalResult closes it).
pub fn is_modal(name: &str) -> bool {
    forms::is_modal(name)
}

/// `DOEVENTS`: pending events, timers and redraws get their turn. (The
/// interpreter's is a wait it serves itself, so a tracking tick in that
/// turn can run its handlers.)
pub fn gui_doevents() {
    // (a tracking tick's handler: this is the program's turn already)
    if !started() || held() {
        return;
    }
    timers::start_all(Rt);
    forms::show_pending(Rt);
    if waits::cooperative() {
        waits::start(Wait::Once(rapidr_ui_kernel::tick::now()));
        return;
    }
    step(Some(Duration::ZERO));
}

/// INPUT$'s wait with windows: events are served until a key reaches
/// INKEY$'s queue. `None` without a window shown; `Some(false)` when the
/// last window closed first. (The interpreter's is a wait it serves itself,
/// `Wait::Key`, so its timers' handlers run meanwhile: its result — 1 or
/// 0 — replaces this one.)
pub fn gui_wait_key() -> Option<bool> {
    if !started() || !forms::any_shown() {
        return None;
    }
    // (a tracking tick's handler: no key can come inside the system's loop
    // — as if the windows had closed)
    if held() && !rapidr_value::console::key_waiting() {
        held_cannot("INPUT$");
        return Some(false);
    }
    timers::start_all(Rt);
    if waits::cooperative() && !rapidr_value::console::key_waiting() {
        waits::start(Wait::Key);
        return Some(true);
    }
    while !rapidr_value::console::key_waiting() {
        forms::show_pending(Rt);
        if !forms::any_shown() {
            return Some(false);
        }
        step(None);
    }
    Some(true)
}

/// A method that waits for work done in the background (QDOWNLOAD's
/// LeechFile; the I/O lane's): `poll` gives its result once it's done.
/// While it runs the program's windows paint and its timers tick, as during
/// a dialog: a native build steps until it's done; the interpreter is left
/// a wait it serves itself (`Wait::Dialog`), whose result replaces this
/// one. Without windows (a console program, or before the first form) it
/// simply waits.
pub fn gui_wait_task(mut poll: impl FnMut() -> Option<Value> + 'static) -> Value {
    if !started() || held() {
        loop {
            if let Some(v) = poll() {
                return v;
            }
            std::thread::sleep(rapidr_ui_app::dialogs::TASK_STEP);
        }
    }
    timers::start_all(Rt);
    use rapidr_ui_app::dialogs::{finished, task_wait, Pending};
    match task_wait(poll) {
        Pending::Done(v) => v,
        Pending::Open(id) if waits::cooperative() => {
            waits::start(Wait::Dialog(id));
            v_null()
        }
        Pending::Open(id) => loop {
            if let Some(v) = finished(id) {
                return v;
            }
            step(Some(rapidr_ui_app::dialogs::TASK_STEP));
            crate::object::rp_serve_program();
        },
    }
}

/// For the bytecode VM: `ShowModal` returns at once and leaves its wait to
/// the VM, which steps with [`gui_pump_wait`].
pub fn gui_set_cooperative_waits(on: bool) {
    waits::set_cooperative(on);
}

pub fn gui_take_wait_started() -> bool {
    waits::take_started()
}

/// Starts waiting for the program's windows (after the main program). The
/// host starts with the first window: a console program never opens the
/// system's windowing (no display needed, no Dock icon).
pub fn gui_begin_app_wait() {
    waits::begin_app();
}

/// One step of the innermost wait: `None` while it goes on, `Some` when
/// it's over (a ShowModal's: its ModalResult; a dialog's: its builtin's
/// result).
pub fn gui_pump_wait() -> Option<Value> {
    // (a tracking tick never starts a wait: see `tracking_tick`)
    if held() {
        return Some(v_null());
    }
    forms::show_pending(Rt);
    match waits::turn() {
        // (one turn: Popup's — its menu shown and tracked in these pumps,
        // its pick dispatched before Popup returns)
        Some(Wait::Popup) => {
            waits::pop();
            pump(Some(Duration::ZERO));
            pump(Some(Duration::ZERO));
            dispatch_pending();
            return Some(v_null());
        }
        // (DOEVENTS: a turn — and every timer due when it was called fires
        // before it returns, each once the handler before it has run, which
        // the VM does between these turns: a native build's DOEVENTS runs
        // them all in its turn)
        Some(Wait::Once(since)) => {
            step(Some(Duration::ZERO));
            if timers::held_back() && timers::due_since(since) {
                return None;
            }
            waits::pop();
            return Some(v_null());
        }
        _ => {}
    }
    // (a dialog's, INPUT$'s, a kernel-drawn menu's: their answer is the
    // result of the builtin that started them)
    if let Some(result) = waits::answered(Rt) {
        return Some(result);
    }
    if waits::over() {
        if let Some(Wait::Form(form)) = waits::pop() {
            crate::object::rp_stop_all_timers();
            return Some(v_int(forms::modal_ended(Rt, &form)));
        }
        return Some(v_null());
    }
    step(None);
    None
}

/// The program's windows until none is left.
pub fn run_gui_event_loop() {
    if held() {
        return;
    }
    // (the host starts with the first window: see gui_begin_app_wait)
    forms::show_pending(Rt);
    while forms::any_shown() {
        step(None);
    }
}

/// MESSAGEBOX / MESSAGEDLG / SHOWMESSAGE: a kernel-drawn modal dialog
/// (`rapidr_ui_app::dialogs`, dialogs.rs) with its icon, beeping when
/// asked; `then` maps the button chosen (`None` for Escape or the close
/// box) to the builtin's result — returned here in a native build, the end
/// of the wait the interpreter serves in its.
pub fn gui_choice(
    title: &str,
    text: &str,
    labels: &[&str],
    icon: Option<rapidr_value::dialogs::MsgIcon>,
    beep: bool,
    then: impl FnOnce(Option<usize>) -> Value + 'static,
) -> Value {
    dialogs::choice(title, text, labels, icon, beep, then)
}

/// Open / Save (rfd, async), colour and font (kernel-drawn) dialogs.
pub fn gui_dialog_execute(name: &str, comp_type: &str) -> Value {
    dialogs::execute(name, comp_type)
}

// ------------------------------------------------- the facade: queries --

/// Whether a form's window shows (`None` before it's built).
pub fn window_shown(name: &str) -> Option<bool> {
    forms::window_shown(name)
}

pub fn form_window_exists(name: &str) -> bool {
    forms::form_window_exists(name)
}

/// Form.Scale: its screen's scale (Screen.Scale before it shows).
pub fn form_scale(name: &str) -> f64 {
    forms::form_scale(Rt, name)
}

/// MOUSEX / MOUSEY: the mouse in the topmost form's client area.
pub fn mouse_in_form() -> (i64, i64) {
    let (mx, my) = mouse();
    let top = with_kern(|k| k.desk.stacking().last().and_then(|f| k.desk.forms.get(f).map(|w| (f.clone(), w.spec.position.unwrap_or((0, 0)))))).flatten();
    match top {
        Some((f, (x, y))) => {
            let (fw, fh) = rapidr_value::layout::form_frame(rp_comp_get(&f, "borderstyle").to_i64());
            (mx - x - fw / 2, my - y - (fh - fw / 2) - i64::from(menu_offset(&f)))
        }
        None => (mx, my),
    }
}

// --------------------------------------- the facade: drawn by the host --

/// A QCANVAS's methods that aren't drawing (the shared model draws): its
/// Repaint (OnPaint), Show, Hide.
pub fn canvas_method(name: &str, method: &str, _args: &[Value]) -> Value {
    match method {
        "paint" | "refresh" | "update" | "repaint" => {
            invalidate();
            crate::object::rp_fire_event(name, "onpaint");
        }
        "show" => gui_show(name),
        "hide" => gui_hide(name),
        _ => eprintln!("[WARN] Canvas.{method}() not implemented"),
    }
    v_null()
}

/// A QIMAGE's methods the shared model leaves to the runtime (the
/// surfaces lane's): a plot's picture (LoadFromPlot), Clear, and a file
/// the model couldn't read (it reads BMP, PNG, JPEG, ICO and SVG).
pub fn image_method(name: &str, method: &str, args: &[Value]) -> Value {
    match method {
        "loadfromfile" | "load" => {
            let file = args.first().map(Value::to_string_val).unwrap_or_default();
            eprintln!("[WARN] RImage: could not load '{file}'");
        }
        "loadfromplot" => {
            #[cfg(feature = "datascience")]
            {
                let plot = args.first().map(Value::to_string_val).unwrap_or_default();
                // (the chart's pixels, and drawn again at the screen's scale for the screen)
                if let Some((w, h)) = rapidr_ui_render::chart::load_into_picture(name, &plot) {
                    if rp_comp_get(name, "stretch").to_i64() == 0 && rp_comp_get(name, "autosize").to_bool() {
                        rp_comp_set(name, "width", v_int(w));
                        rp_comp_set(name, "height", v_int(h));
                    }
                }
            }
            #[cfg(not(feature = "datascience"))]
            {
                let _ = args;
                eprintln!("[WARN] datascience not compiled — loadfromplot unavailable");
            }
        }
        // (the picture goes: nothing shows)
        "clear" | "cls" => {
            rapidr_value::objects::with_picture(name, |b| {
                b.resize(0, 0);
                b.alpha = None;
                b.invalidate_display();
            });
        }
        _ => eprintln!("[WARN] RImage.{method}() not implemented"),
    }
    invalidate();
    v_null()
}

/// A QTREEVIEW's methods that need the drawn tree: the lists lane's.
pub fn tree_method(name: &str, method: &str, args: &[Value]) -> Value {
    match method {
        "getitemat" => return v_int(lists::tree_item_at(name, args.first().map_or(0, Value::to_i64), args.get(1).map_or(0, Value::to_i64))),
        "show" => gui_show(name),
        "hide" => gui_hide(name),
        _ => eprintln!("[WARN] TreeView.{method}() not implemented"),
    }
    v_null()
}

// ---------------------------------------------- the IDE's components --
//
// (Stage 10) RDESIGNSURFACE and RCODEEDITOR keep their state in the shared
// models (rapidr_value::objects::design, a TextEdit in code mode), which the
// kernel's components draw and drive (components/design.rs, codeedit.rs):
// only a design surface's Show / Hide is left here.

pub fn design_surface_method(name: &str, method: &str, _args: &[Value]) -> Value {
    match method {
        "show" => gui_show(name),
        "hide" => gui_hide(name),
        _ => eprintln!("[WARN] DesignSurface.{method}() not implemented"),
    }
    v_null()
}

/// `$THEME name` (platform.rs: the kernel draws the classic look).
pub fn set_theme(theme: &str) {
    st(|s| s.theme = theme.to_lowercase());
    platform::theme(theme);
}

/// What the kernel host doesn't do yet (said once per feature).
fn pending(what: &str) {
    thread_local! {
        static WARNED: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
    }
    if WARNED.with(|w| w.borrow_mut().insert(what.to_string())) {
        eprintln!("[rapidr] the UI kernel host doesn't do {what} yet");
    }
}

// ------------------------------------------------ the facade: platform --

pub fn screen_size() -> (i64, i64) {
    screen()
}

pub fn work_area() -> (i64, i64) {
    ensure_host();
    // (while the host pumps — a tracking tick's handler — as it last said)
    match with_kern(|k| k.host.work_area()) {
        Some(a) => {
            st(|s| s.work_area = Some(a));
            a
        }
        None => st(|s| s.work_area).unwrap_or(rapidr_ui_host_winit::HEADLESS_SCREEN),
    }
}

pub fn mouse() -> (i64, i64) {
    // (while the host pumps — a tracking tick's handler — the platform's
    // answer, which needs no host)
    with_kern(|k| k.host.mouse())
        .or_else(|| (held() && !headless_known()).then(rapidr_ui_host_winit::platform::global_mouse).flatten().map(|(x, y)| (x.round() as i64, y.round() as i64)))
        .unwrap_or((0, 0))
}

/// The host is the headless one (whose mouse stays at (0, 0)).
fn headless_known() -> bool {
    Capture::from_env().is_some() && std::env::var_os("RAPIDR_CAPTURE_WINDOWS").is_none()
}

pub fn monitors() -> i64 {
    ensure_host();
    match with_kern(|k| k.host.monitors()) {
        Some(m) => {
            st(|s| s.monitors = Some(m));
            m
        }
        None => st(|s| s.monitors).unwrap_or(1),
    }
}

/// `Application.Minimize`: every shown window.
pub fn minimize() {
    for f in forms::shown_forms() {
        push_op(WindowOp::Minimize(f));
    }
    if started() {
        pump(Some(Duration::ZERO));
    }
}

/// MSGBOX: the text and OK; 0.
pub fn message_box(text: &str) -> Value {
    // (the dialogs lane's: SHOWMESSAGE's box, titled Application.Title, as
    // the web's)
    let title = crate::globals::get("application", "title").map(|t| t.to_string_val()).unwrap_or_default();
    gui_choice(&title, text, &["OK"], None, false, |_| v_int(0))
}

/// The program ends: the windows' pending commands run (closed forms'
/// windows go while everything is alive). The host is never dropped.
pub fn before_exit() {
    if started() {
        pump(Some(Duration::ZERO));
    }
}

fn end(code: i32) -> ! {
    before_exit();
    std::process::exit(code)
}

// ----------------------------------------------- test hooks (§2.3) --
//
// The script is `rapidr_ui_app::script`'s; its input goes through the
// kernel's routing here, as the user's would.

/// The form a component is on, and where it is in that window's inside
/// (logical; the in-window menu bar included), from the kernel's tree.
fn place_of(comp: &str) -> Option<(String, (i64, i64))> {
    with_kern(|k| rapidr_ui_app::desktop::place_of(Rt, &mut k.desk, &RtStore, comp)).flatten()
}

/// The test's end (after `rapidr_ui_app::script` printed `RAPIDR_TEST_DUMP`):
/// `RAPIDR_TEST_A11Y`, then every shown window saved as `<prefix>-<n>.bmp`
/// (bottom to top, at its scale, by the CPU renderer), and the program
/// ends.
fn capture_and_end(prefix: &str) -> ! {
    let a11y = std::env::var("RAPIDR_TEST_A11Y").ok().filter(|p| !p.is_empty());
    let shots = with_kern(|k| {
        sync_desk(&mut k.desk);
        let Kern { desk, .. } = k;
        let order = desk.stacking();
        let mut trees = Vec::new();
        let mut shots = Vec::new();
        for f in &order {
            if a11y.is_some() {
                trees.extend(desk.access_json(&RtStore, f));
            }
            if let Some(px) = rapidr_ui_host_winit::capture(desk, &RtStore, f) {
                let title = desk.forms.get(f).map(|w| w.spec.title.clone()).unwrap_or_default();
                shots.push((title, px));
            }
        }
        (trees, shots)
    });
    if let Some((trees, shots)) = shots {
        if let Some(path) = a11y {
            if let Err(e) = std::fs::write(&path, format!("[{}]\n", trees.join(",\n"))) {
                eprintln!("[rapidr] can't write {path}: {e}");
            }
        }
        for (n, (title, px)) in shots.into_iter().enumerate() {
            let path = format!("{prefix}-{}.bmp", n + 1);
            match std::fs::write(&path, rapidr_value::objects::codec::encode_bmp(&px)) {
                Ok(()) => eprintln!("[rapidr] captured window '{title}' to {path}"),
                Err(e) => eprintln!("[rapidr] can't write {path}: {e}"),
            }
        }
    }
    end(0)
}

// ------------------------------------------------ rapidr_ui_app's host --

/// The desktop host as `rapidr_ui_app` asks it (the program is
/// `program.rs`'s).
impl Windows for Rt {
    fn start(self) {
        ensure_host();
    }
    fn started(self) -> bool {
        started()
    }
    fn flush(self) {
        pump(Some(Duration::ZERO));
    }
    fn dispatch_pending(self) {
        dispatch_pending();
    }
    fn headless(self) -> bool {
        headless()
    }
    fn forced_scale(self) -> Option<f64> {
        forced_scale()
    }
    fn window_scale(self, form: &str) -> Option<f64> {
        with_kern(|k| k.desk.forms.get(form).map(|f| f.scale).unwrap_or_else(|| k.host.default_scale()))
    }
    fn screen(self) -> (i64, i64) {
        screen()
    }
    fn work_area(self) -> (i64, i64) {
        work_area()
    }
    fn menu_in_window(self) -> bool {
        menu_in_window()
    }
    fn stacking(self) -> Vec<String> {
        with_kern(|k| k.desk.stacking()).unwrap_or_default()
    }
    fn place_of(self, comp: &str) -> Option<(String, (i64, i64))> {
        place_of(comp)
    }
    fn system_resized(self, form: &str, (iw, ih): (i64, i64), (left, top): (i64, i64)) {
        with_kern(|k| {
            if let Some(f) = k.desk.forms.get_mut(form) {
                f.ui.sync(&RtStore);
            }
            k.desk.resized(form, iw, ih);
            k.desk.moved(form, left, top);
        });
    }
    fn open_popup(self, form: &str, menu: &str, x: i64, y: i64, program: bool) {
        menus::open_at(form, menu, x, y, program);
    }
    fn popup_open(self, form: &str) -> bool {
        menus::popup_open(form)
    }
    fn open_dialog(self, id: &str, title: &str, size: (i64, i64)) {
        dialogs::open_window(id, title, size);
    }
    fn close_dialog(self, id: &str) {
        dialogs::close_window(id);
    }
    fn dialog_resized(self, id: &str, size: (i64, i64)) {
        dialogs::resized(id, size);
    }
    fn beep(self, icon: Option<rapidr_value::dialogs::MsgIcon>) {
        dialogs::beep(icon);
    }
    fn font_families(self) -> Vec<String> {
        dialogs::font_families()
    }
    fn ask_files(self, id: u64, form: Option<&str>, req: &rapidr_ui_app::file_dialog::Request) {
        dialogs::ask_files(id, form, req);
    }
    fn files_answer(self, id: u64) -> Option<Vec<String>> {
        dialogs::files_answer(id)
    }
    fn script_input(self, input: ScriptInput) {
        match input {
            // (timers during native menu tracking: the next pump held, as a
            // menu the user keeps open would hold it)
            ScriptInput::Hold(ms) => {
                with_kern(|k| k.host.hold(Duration::from_millis(ms.max(0) as u64)));
            }
            // (keys, the mouse, a double click, a component's step, the
            // resize: through the kernel's routing, as the user's —
            // rapidr_ui_app::desktop, shared with the web host)
            input => {
                with_kern(|k| rapidr_ui_app::desktop::script_input(Rt, &mut k.desk, &RtStore, input));
            }
        }
    }
    fn capture_and_end(self, prefix: &str) {
        capture_and_end(prefix)
    }
}

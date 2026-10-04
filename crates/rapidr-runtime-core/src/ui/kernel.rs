//! The UI kernel as the runtime's desktop host (`RAPIDR_HOST=kernel`;
//! docs/desktop-host-plan.md §1.3–§1.5): the facade's functions for the
//! kernel (`rapidr_ui_kernel`) and its winit or headless host
//! (`rapidr_ui_host_winit`).
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
//! one step each, and its handlers run after the pump returns.
//!
//! What the host needs from the program goes into queues the next pump
//! takes (window commands, "something changed"); what the facade is asked
//! (Visible, Scale, Handle …) is answered from state kept here, never from
//! the host, which is borrowed while it pumps.

use std::cell::{Cell, RefCell};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};

use rapidr_ui_host_winit::{Desktop, Host, HostEvent, Icon, Source, WindowSpec};
use rapidr_ui_kernel::{Clipboard, KernelEvent, Mods};
use rapidr_value::input::{Button, Mouse};

use super::kernel_store::{flag, RtStore};
use super::testhooks::{self, Action, Capture, TestEvent};
use crate::object::{form_of, get_children_of, rp_comp_get, rp_comp_set, rp_comp_type, rp_fire_event, rp_fire_event_1, rp_fire_event_2, rp_fire_event_args, rp_fire_event_then, store_prop};
use crate::value::{v_int, v_null, Value};

// The buttons and menus lane's part: a pick's OnClick, Popup, AutoPopup,
// RAPIDR_DUMP_MENUS.
mod menus;

// ------------------------------------------------------------------ state --

/// The host and the kernel's forms: borrowed while the host pumps.
struct Kern {
    host: Box<dyn Host>,
    desk: Desktop,
}

/// A window command for the next pump (the host may be pumping when the
/// program asks: a store hook inside a callback).
enum WinOp {
    Show(String),
    Hide(String),
    Title(String, String),
    Size(String, (i64, i64)),
    Position(String, (i64, i64)),
    Border(String, bool),
    Icon(String, Option<Icon>),
    Minimize(String),
    /// A pop-up menu shown by the host (form, menu, x, y in its inside).
    Popup(String, String, i64, i64),
}

/// A wait the bytecode VM serves itself ([`gui_set_cooperative_waits`]).
enum Wait {
    /// `Form.ShowModal`: until the form is closed.
    Form(String),
    /// The program's main event loop: until no window is left.
    App,
}

/// A GUI test's run (`RAPIDR_CAPTURE`; ui::testhooks).
struct Script {
    capture: Capture,
    events: VecDeque<TestEvent>,
    /// When the next step may run.
    next: Instant,
    /// The resize and splitter drag done (they come before the events).
    started: bool,
    /// The events done: the next step captures.
    finished: bool,
}

#[derive(Default)]
struct State {
    /// Forms whose kernel side (and window) was made.
    built: HashSet<String>,
    /// Forms shown now.
    shown: HashSet<String>,
    /// Built forms whose first OnPaint waits for their window to show.
    first_paint: HashSet<String>,
    /// Windows a `Visible = True` shows once the program waits.
    pending_shows: Vec<String>,
    /// The forms shown modally now, innermost last.
    modal: Vec<String>,
    /// Each shown form's screen scale (Form.Scale).
    scales: HashMap<String, f64>,
    ops: Vec<WinOp>,
    /// QTIMERs the program made, those ticking, and when they're due.
    timers: Vec<String>,
    scheduled: HashSet<String>,
    heap: BinaryHeap<Reverse<(Instant, u64, String)>>,
    timer_gen: u64,
    cooperative: bool,
    waits: Vec<Wait>,
    wait_started: bool,
    script: Option<Script>,
    theme: String,
}

thread_local! {
    /// Made once, never dropped: winit windows dropped during the main
    /// thread's TLS teardown abort the process (Stage 0), so the host lives
    /// until the process ends.
    static KERN: Cell<Option<&'static RefCell<Kern>>> = const { Cell::new(None) };
    static ST: RefCell<State> = RefCell::new(State::default());
    /// Something to draw again: (paint, the component tree changed).
    static NOTIFY: Cell<(bool, bool)> = const { Cell::new((false, false)) };
    /// Inside a window change the runtime makes (Left / Top following a
    /// move): not the program's.
    static APPLYING: Cell<u32> = const { Cell::new(0) };
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

fn is_form(name: &str) -> bool {
    matches!(rp_comp_type(name).as_str(), "RFORM" | "RFORMMDI")
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
    let host = rapidr_ui_host_winit::new_host(headless, forced_scale());
    let mut desk = Desktop::new(Box::new(RtClipboard));
    if let Some(c) = capture {
        // (only the test's own events drive it)
        desk.ignore_user = true;
        let next = Instant::now() + Duration::from_secs_f64(c.delay.max(0.0));
        st(|s| s.script = Some(Script { events: c.events.clone().into(), capture: c, next, started: false, finished: false }));
    }
    let k: &'static RefCell<Kern> = Box::leak(Box::new(RefCell::new(Kern { host, desk })));
    KERN.with(|c| c.set(Some(k)));
}

fn started() -> bool {
    kern().is_some()
}

// ------------------------------------------------------------- the pump --

/// What the program changed, into the kernel's forms: window commands,
/// new forms' kernel sides, trees rebuilt, layouts read again.
fn sync_desk(k: &mut Kern) {
    menus::dump_if_changed();
    let store = RtStore;
    let (paint, structure) = NOTIFY.with(|n| n.replace((false, false)));
    let ops = st(|s| std::mem::take(&mut s.ops));
    let modal = st(|s| s.modal.clone());
    let desk = &mut k.desk;
    for op in ops {
        match op {
            WinOp::Show(f) => {
                if !desk.forms.contains_key(&f) {
                    desk.ensure_form(&store, &f, menu_in_window(), spec_of(&f));
                }
                desk.show(&f);
            }
            WinOp::Hide(f) => desk.hide(&f),
            WinOp::Title(f, t) => {
                if let Some(w) = desk.form(&f) {
                    w.spec.title = t;
                    desk.cmds.push(rapidr_ui_host_winit::HostCmd::Title(f));
                }
            }
            WinOp::Size(f, size) => {
                if let Some(w) = desk.form(&f) {
                    if w.spec.size != size {
                        w.spec.size = size;
                        desk.cmds.push(rapidr_ui_host_winit::HostCmd::Size(f));
                    }
                }
            }
            WinOp::Position(f, p) => {
                if let Some(w) = desk.form(&f) {
                    w.spec.position = Some(p);
                    desk.cmds.push(rapidr_ui_host_winit::HostCmd::Position(f));
                }
            }
            WinOp::Border(f, b) => {
                if let Some(w) = desk.form(&f) {
                    w.spec.border = b;
                    desk.cmds.push(rapidr_ui_host_winit::HostCmd::Border(f));
                }
            }
            WinOp::Icon(f, i) => {
                if let Some(w) = desk.form(&f) {
                    w.spec.icon = i;
                    desk.cmds.push(rapidr_ui_host_winit::HostCmd::Icon(f));
                }
            }
            WinOp::Minimize(f) => desk.cmds.push(rapidr_ui_host_winit::HostCmd::Minimize(f)),
            WinOp::Popup(form, menu, x, y) => desk.cmds.push(rapidr_ui_host_winit::HostCmd::Popup { form, menu, x, y }),
        }
    }
    desk.modal = modal;
    for (id, f) in desk.forms.iter_mut() {
        f.ui.modal = desk.modal.contains(id);
        if structure {
            f.ui.rebuild(&store);
        } else if paint {
            f.ui.sync(&store);
            f.ui.dirty = true;
        }
    }
}

/// The host's turn: up to `timeout` (`None`: until something happens).
/// Program code fired meanwhile waits ([`crate::object::rp_in_host_callback`]).
fn pump(timeout: Option<Duration>) {
    let Some(k) = kern() else {
        if let Some(t) = timeout {
            std::thread::sleep(t);
        }
        return;
    };
    crate::object::rp_in_host_callback(|| {
        let Ok(mut k) = k.try_borrow_mut() else { return };
        sync_desk(&mut k);
        let Kern { host, desk } = &mut *k;
        host.pump(timeout, desk, &RtStore);
    });
}

/// One step of the innermost wait (see the module's doc).
pub fn step(max_wait: Option<Duration>) {
    ensure_host();
    show_pending();
    let now = Instant::now();
    let mut t = max_wait;
    let mut at_most = |d: Duration| t = Some(t.map_or(d, |t| t.min(d)));
    if let Some(at) = st(|s| s.heap.peek().map(|Reverse((at, _, _))| *at)) {
        at_most(at.saturating_duration_since(now));
    }
    if let Some(at) = st(|s| s.script.as_ref().map(|s| s.next)) {
        at_most(at.saturating_duration_since(now));
    }
    if with_kern(|k| !k.desk.events.is_empty()).unwrap_or(false) {
        at_most(Duration::ZERO);
    }
    pump(t);
    crate::object::rp_run_deferred();
    dispatch_pending();
    fire_due_timers();
    script_step();
}

/// The events the host queued, handled (each to completion; a handler may
/// step again).
fn dispatch_pending() {
    for e in with_kern(|k| std::mem::take(&mut k.desk.events)).unwrap_or_default() {
        if let HostEvent::Kernel(_, ev) = e {
            dispatch(ev);
        }
    }
}

/// A kernel event, as FLTK's callbacks fire it (gui.rs).
fn dispatch(ev: KernelEvent) {
    match ev {
        KernelEvent::Click(id) => rp_fire_event(&id, "onclick"),
        KernelEvent::Change(id) => rp_fire_event(&id, "onchange"),
        KernelEvent::KeyDown { chain, vk, shift, text } => {
            // (a key pressed in the program's windows is INKEY$'s too)
            if let Some(k) = rapidr_value::console::inkey_of(vk, &text) {
                rapidr_value::console::push_key(k);
            }
            for name in rapidr_value::input::key_targets(&chain, |f| rp_comp_get(f, "keypreview").to_bool()) {
                rp_fire_event_2(name, "onkeydown", v_int(vk), v_int(shift));
            }
        }
        KernelEvent::KeyPress { chain, key } => {
            for name in rapidr_value::input::key_targets(&chain, |f| rp_comp_get(f, "keypreview").to_bool()) {
                rp_fire_event_1(name, "onkeypress", v_int(key));
            }
        }
        KernelEvent::KeyUp { chain, vk, shift } => {
            for name in rapidr_value::input::key_targets(&chain, |f| rp_comp_get(f, "keypreview").to_bool()) {
                rp_fire_event_2(name, "onkeyup", v_int(vk), v_int(shift));
            }
        }
        KernelEvent::Mouse { id, kind, button, x, y, shift } => mouse_event(&id, kind, button, x, y, shift),
        KernelEvent::Close(f) => gui_close(&f),
        KernelEvent::Resized(f, w, h) => {
            let menu = i64::from(menu_offset(&f));
            form_resized(&f, w, h + menu);
        }
        KernelEvent::Moved(f, x, y) => {
            APPLYING.with(|a| a.set(a.get() + 1));
            crate::layout::quietly(|| {
                rp_comp_set(&f, "left", v_int(x));
                rp_comp_set(&f, "top", v_int(y));
            });
            APPLYING.with(|a| a.set(a.get() - 1));
        }
        KernelEvent::ScaleChanged(f, scale) => scale_changed(&f, scale),
        KernelEvent::MenuPick(item) => menus::picked(&item),
        KernelEvent::Set { id, prop, value } => {
            rp_comp_set(&id, &prop, v_int(value));
            invalidate();
        }
        KernelEvent::Container(c) => container_event(c),
    }
}

/// A container's action (containers lane): what gui.rs's scroll bars,
/// splitter and MDI frame handlers do.
fn container_event(c: rapidr_ui_kernel::components::form::Container) {
    use rapidr_ui_kernel::components::form::Container;
    match c {
        Container::Scrolled { id, dx, dy } => crate::scroll::user_scrolled(&id, (dx, dy)),
        Container::SplitBegin(id) => {
            crate::layout::splitter_begin(&id);
        }
        Container::SplitMove(delta) => crate::layout::splitter_move(delta),
        Container::SplitEnd => crate::layout::splitter_end(),
        Container::Mdi { form, component, action } => crate::mdi::user(&form, &component, action),
    }
}

/// `name`'s mouse event (gui.rs's `mouse_event`: QIMAGE fires its own).
fn mouse_event(name: &str, kind: Mouse, button: Button, x: i64, y: i64, shift: i64) {
    if kind == Mouse::Down && button == Button::Right && menus::auto_popup(name, x, y) {
        return;
    }
    if rp_comp_type(name).eq_ignore_ascii_case("RIMAGE") {
        return;
    }
    rp_fire_event_args(name, kind.event(), &kind.args(button, x, y, shift));
}

/// The user resized a form's window to `w` × `h` (its inside, the
/// in-window menu included): its Width / Height follow (within its
/// Constraints), its aligned and anchored children are laid out again,
/// OnResize and OnPaint fire (gui.rs's `form_resized`).
fn form_resized(form: &str, w: i64, h: i64) {
    let (fw, fh) = rapidr_value::layout::form_frame(rp_comp_get(form, "borderstyle").to_i64());
    let asked = (w + fw, h + fh);
    let (w, h) = crate::layout::constraints_of(form).size(asked.0, asked.1);
    let same = rp_comp_get(form, "width").to_i64() == w && rp_comp_get(form, "height").to_i64() == h;
    if same {
        // (dragged outside them: the window goes back)
        if (w, h) != asked {
            gui_apply_geometry(form);
        }
        return;
    }
    crate::layout::quietly(|| {
        rp_comp_set(form, "width", v_int(w));
        rp_comp_set(form, "height", v_int(h));
    });
    crate::layout::client_changed(form);
    crate::scroll::update(form);
    gui_apply_geometry(form);
    rp_fire_event(form, "onresize");
    rp_fire_event(form, "onpaint");
}

/// A form's window moved to a screen with another scale: told
/// (OnScaleChanged) and drawn again at it (OnPaint).
fn scale_changed(form: &str, scale: f64) {
    if forced_scale().is_some() {
        return;
    }
    let changed = st(|s| match s.scales.insert(lower(form), scale) {
        Some(old) => (old - scale).abs() > f64::EPSILON,
        None => false,
    });
    if changed {
        rapidr_value::objects::bitmap::set_display_scale(scale);
        rp_fire_event(form, "onscalechanged");
        fire_first_paint(form);
    }
}

// --------------------------------------------------------------- timers --

/// A QTIMER the program made (generated programs call it).
pub fn gui_register_timer(name: &str) {
    let name = lower(name);
    st(|s| {
        if !s.timers.contains(&name) {
            s.timers.push(name);
        }
    });
}

fn start_timers() {
    for t in st(|s| s.timers.clone()) {
        schedule_timer(&t);
    }
}

fn timer_interval(name: &str) -> Duration {
    let ms = rp_comp_get(name, "interval").to_i64();
    Duration::from_millis(if ms > 0 { ms as u64 } else { 1000 })
}

/// Starts timer `name` ticking if it's enabled and isn't already (it
/// fires while the program waits: a modal form, the main loop, DOEVENTS).
fn schedule_timer(name: &str) {
    let name = lower(name);
    if rp_comp_get(&name, "enabled").to_i64() == 0 || !st(|s| s.scheduled.insert(name.clone())) {
        return;
    }
    let at = Instant::now() + timer_interval(&name);
    st(|s| {
        s.timer_gen += 1;
        let g = s.timer_gen;
        s.heap.push(Reverse((at, g, name)));
    });
}

/// A timer's Enabled or Interval changed: it ticks if it's enabled now.
pub fn gui_timer_changed(name: &str) {
    if started() {
        schedule_timer(name);
    }
}

/// The timers due, each fired then armed again from now with its Interval
/// as it is then; one disabled meanwhile stops.
fn fire_due_timers() {
    loop {
        let now = Instant::now();
        let due = st(|s| match s.heap.peek() {
            Some(Reverse((at, _, _))) if *at <= now => s.heap.pop().map(|Reverse((_, _, n))| n),
            _ => None,
        });
        let Some(name) = due else { break };
        if rp_comp_get(&name, "enabled").to_i64() == 0 {
            st(|s| s.scheduled.remove(&name));
            continue;
        }
        rp_fire_event(&name, "ontimer");
        let at = Instant::now() + timer_interval(&name);
        st(|s| {
            s.timer_gen += 1;
            let g = s.timer_gen;
            s.heap.push(Reverse((at, g, name)));
        });
    }
}

// ---------------------------------------------------------------- forms --

/// Whether a QMAINMENU is a bar inside its form's window: everywhere but
/// macOS (its system menu bar), unless `RAPIDR_MENU=window` (gui.rs's).
fn menu_in_window() -> bool {
    !cfg!(target_os = "macos") || std::env::var("RAPIDR_MENU").is_ok_and(|v| v.eq_ignore_ascii_case("window"))
}

/// The height of a form's in-window main menu (0 without one, and on
/// macOS).
pub fn menu_offset(form: &str) -> i32 {
    let has_menu = get_children_of(form).iter().any(|(_, t)| t == "RMAINMENU");
    if has_menu && menu_in_window() {
        rapidr_value::layout::MAIN_MENU_HEIGHT as i32
    } else {
        0
    }
}

/// A form's window inside: Width / Height less the frame the window
/// system draws (the in-window menu included).
fn form_window_size(name: &str) -> (i64, i64) {
    let (fw, fh) = rapidr_value::layout::form_frame(rp_comp_get(name, "borderstyle").to_i64());
    ((rp_comp_get(name, "width").to_i64() - fw).clamp(1, 100_000), (rp_comp_get(name, "height").to_i64() - fh).clamp(1, 100_000))
}

/// A form's icon: its IcoHandle / Icon, else the application's.
fn icon_of(name: &str) -> Option<Icon> {
    let own = ["icohandle", "icon"].into_iter().map(|p| rp_comp_get(name, p)).find(rapidr_value::objects::has_icon);
    let (w, h, rgba, _) = own.or_else(rapidr_value::globals::application_icon).and_then(|v| rapidr_value::objects::icon_pixels(&v))?;
    Some(Icon { width: w as u32, height: h as u32, rgba })
}

fn spec_of(name: &str) -> WindowSpec {
    WindowSpec {
        title: rp_comp_get(name, "caption").to_string_val(),
        size: form_window_size(name),
        position: Some((rp_comp_get(name, "left").to_i64(), rp_comp_get(name, "top").to_i64())),
        border: rp_comp_get(name, "borderstyle").to_i64() != 0,
        icon: icon_of(name),
    }
}

/// A form's kernel side, made the first time (OnLoad once, the first
/// OnPaint waiting for its window to show).
fn build_form(name: &str) {
    let name = lower(name);
    if !st(|s| s.built.insert(name.clone())) {
        return;
    }
    rp_fire_event(&name, "onload");
    st(|s| s.first_paint.insert(name));
}

/// The form's window shown (made the first time), before its OnShow: one
/// pump so the window exists.
fn show_window(name: &str) {
    let name = lower(name);
    st(|s| {
        s.shown.insert(name.clone());
        s.ops.push(WinOp::Show(name));
    });
    pump(Some(Duration::ZERO));
}

fn hide_window(name: &str) {
    let name = lower(name);
    st(|s| {
        if s.shown.remove(&name) {
            s.ops.push(WinOp::Hide(name));
        }
    });
}

/// A form's window shown: drawn at its screen's scale from now on, then
/// (the first time) its OnPaint — as Windows' WM_PAINT comes once a window
/// shows, after OnShow (gui.rs's `after_show`).
fn after_show(name: &str) {
    let name = lower(name);
    let host_scale = with_kern(|k| k.desk.forms.get(&name).map(|f| f.scale).unwrap_or_else(|| k.host.default_scale()));
    let scale = forced_scale().or(host_scale).unwrap_or(1.0);
    rapidr_value::objects::bitmap::set_display_scale(scale);
    st(|s| s.scales.insert(name.clone(), scale));
    if st(|s| s.first_paint.remove(&name)) {
        fire_first_paint(&name);
    }
}

fn fire_first_paint(parent: &str) {
    rp_fire_event(parent, "onpaint");
    for (child, type_name) in get_children_of(parent) {
        if type_name.eq_ignore_ascii_case("RCANVAS") {
            rp_fire_event(&child, "onpaint");
        } else {
            fire_first_paint(&child);
        }
    }
}

/// The program waits: the windows it made visible show (unless it hid
/// them again meanwhile).
fn show_pending() {
    for name in st(|s| std::mem::take(&mut s.pending_shows)) {
        if window_shown(&name) != Some(true) && rp_comp_get(&name, "visible").to_bool() {
            gui_show(&name);
        }
    }
}

fn form_shown(name: &str) -> bool {
    st(|s| s.shown.contains(&lower(name)))
}

fn any_shown() -> bool {
    st(|s| !s.shown.is_empty())
}

/// Something drawn changed: painted again at the next pump.
fn invalidate() {
    NOTIFY.with(|n| n.set((true, n.get().1)));
}

/// Components added, removed or moved between parents.
fn restructure() {
    NOTIFY.with(|n| n.set((n.get().0, true)));
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
pub fn tree_refresh(_name: &str) {
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
/// group come up (gui.rs's, host-neutral: rapidr_value::toggle_group).
pub fn toggle_down_set(name: &str) {
    if !is_toggle_button(name) {
        return;
    }
    let name = lower(name);
    let down = rp_comp_get(&name, "down").to_bool();
    let mut changes = rapidr_value::toggle_group::set_down(&name, down, &toggle_members(&name));
    changes.push((name, down));
    toggle_apply(changes);
}

fn is_toggle_button(name: &str) -> bool {
    matches!(rp_comp_type(name).as_str(), "RCOOLBTN" | "ROVALBTN")
}

/// The toggle buttons sharing `name`'s parent.
fn toggle_members(name: &str) -> Vec<rapidr_value::toggle_group::Member> {
    let parent = rp_comp_get(name, "parent").to_string_val();
    get_children_of(&parent)
        .into_iter()
        .filter(|(_, t)| matches!(t.as_str(), "RCOOLBTN" | "ROVALBTN"))
        .map(|(n, _)| rapidr_value::toggle_group::Member { group: rp_comp_get(&n, "groupindex").to_i64(), down: rp_comp_get(&n, "down").to_bool(), name: n })
        .collect()
}

/// The new Down values, stored (and drawn).
fn toggle_apply(changes: Vec<(String, bool)>) {
    for (n, down) in changes {
        store_prop(&n, "down", v_int(if down { -1 } else { 0 }));
    }
    invalidate();
}

/// The user pressed a QCOOLBTN / QOVALBTN (its group decides what's down).
fn toggle_press(name: &str) {
    let allow_all_up = rp_comp_get(name, "allowallup").to_bool();
    toggle_apply(rapidr_value::toggle_group::press(name, allow_all_up, &toggle_members(name)));
}
pub fn schedule_menu_sync() {
    invalidate();
}

/// A caption: a form's is its window's title.
pub fn gui_set_caption(name: &str, text: &str) {
    if is_form(name) {
        st(|s| s.ops.push(WinOp::Title(lower(name), text.to_string())));
    }
    invalidate();
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

/// `Visible`: a built form's window shows or hides (no OnShow, as FLTK's
/// `show()`); a component is read from the store when painted.
pub fn gui_set_visible(name: &str, visible: bool) {
    if is_form(name) && window_shown(name).is_some() {
        if visible {
            show_window(name);
            after_show(name);
        } else {
            hide_window(name);
        }
        return;
    }
    invalidate();
}

/// Left / Top / Width / Height: a form's window takes its new size.
pub fn gui_apply_geometry(name: &str) {
    if is_form(name) && window_shown(name).is_some() {
        st(|s| s.ops.push(WinOp::Size(lower(name), form_window_size(name))));
    }
    invalidate();
}

// ------------------------------------------------- the facade: windows --

/// Shows a form without waiting (OnShow when it wasn't showing).
pub fn gui_show(name: &str) {
    ensure_host();
    if !is_form(name) {
        // (a component shown: drawn again; its Visible says)
        store_prop(&lower(name), "visible", crate::value::v_bool(true));
        return invalidate();
    }
    let was_built = window_shown(name).is_some();
    if was_built && form_shown(name) {
        // (already showing: on top)
        st(|s| s.ops.push(WinOp::Show(lower(name))));
        return;
    }
    build_form(name);
    show_window(name);
    rp_fire_event(name, "onshow");
    after_show(name);
}

/// `Form.Visible = True`: its Show; a form not built yet (its own CREATE)
/// shows once the program waits.
pub fn gui_show_visible(name: &str) {
    if window_shown(name).is_some() {
        gui_show(name);
        return;
    }
    ensure_host();
    st(|s| s.pending_shows.push(name.to_string()));
}

/// Hides a form's window (no OnClose).
pub fn gui_hide(name: &str) {
    if is_form(name) {
        hide_window(name);
    } else {
        store_prop(&lower(name), "visible", crate::value::v_bool(false));
        invalidate();
    }
}

/// `Form.Close` and the window's close box: OnClose's `Action` (it starts
/// as `caHide`) decides whether the form goes, stays or is minimized.
pub fn gui_close(name: &str) {
    use rapidr_value::events::{CloseAction, CA_HIDE};
    if !is_form(name) || window_shown(name).is_none() {
        return gui_hide(name);
    }
    let name = lower(name);
    rp_fire_event_then(&name.clone(), "onclose", &[v_int(CA_HIDE)], move |a| match CloseAction::of(&a[0]) {
        CloseAction::Stay => {}
        CloseAction::Minimize => st(|s| s.ops.push(WinOp::Minimize(name.clone()))),
        CloseAction::Close => hide_window(&name),
    });
}

/// The screen's size (logical pixels).
fn screen() -> (i64, i64) {
    ensure_host();
    // (the winit host knows its monitor after its first pump)
    if with_kern(|k| k.host.headless()) == Some(false) && !st(|s| !s.built.is_empty()) {
        pump(Some(Duration::ZERO));
    }
    with_kern(|k| k.host.screen()).unwrap_or(rapidr_ui_host_winit::HEADLESS_SCREEN)
}

/// A form's position centred on the screen.
fn centered(name: &str) -> (i64, i64) {
    let (sw, sh) = screen();
    let (w, h) = form_window_size(name);
    ((sw - w) / 2, (sh - h) / 2)
}

/// `Form.Center`: on the screen's middle (when shown; ShowModal centres a
/// form asked to be before it showed).
pub fn gui_center(name: &str) {
    rp_comp_set(name, "_center", v_int(1));
    if window_shown(name).is_none() {
        return;
    }
    let (x, y) = centered(name);
    st(|s| s.ops.push(WinOp::Position(lower(name), (x, y))));
    APPLYING.with(|a| a.set(a.get() + 1));
    crate::layout::quietly(|| {
        rp_comp_set(name, "left", v_int(x));
        rp_comp_set(name, "top", v_int(y));
    });
    APPLYING.with(|a| a.set(a.get() - 1));
}

/// `Form.Left` / `Form.Top` set by the program: the window moves there.
pub fn gui_move_form(name: &str) {
    if APPLYING.with(Cell::get) > 0 || window_shown(name).is_none() {
        return;
    }
    let p = (rp_comp_get(name, "left").to_i64(), rp_comp_get(name, "top").to_i64());
    st(|s| s.ops.push(WinOp::Position(lower(name), p)));
}

/// `Form.BorderStyle`: bsNone (0) takes away the window's frame.
pub fn gui_set_form_border(name: &str) {
    if window_shown(name).is_some() {
        st(|s| s.ops.push(WinOp::Border(lower(name), rp_comp_get(name, "borderstyle").to_i64() != 0)));
    }
    gui_apply_geometry(name);
}

pub fn gui_apply_icon(name: &str) {
    if is_form(name) && window_shown(name).is_some() {
        let icon = icon_of(name);
        st(|s| s.ops.push(WinOp::Icon(lower(name), icon)));
    }
}

/// `Application.Icon` changed: every form without its own.
pub fn gui_apply_icons() {
    for f in st(|s| s.built.iter().cloned().collect::<Vec<_>>()) {
        gui_apply_icon(&f);
    }
}

pub fn gui_menu_popup(name: &str, x: i32, y: i32) {
    menus::popup(name, x, y);
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
    store_prop(&name, "modalresult", v_int(0));
    st(|s| s.modal.push(name.clone()));
    build_form(&name);
    if rp_comp_get(&name, "_center").to_i64() != 0 {
        let p = centered(&name);
        st(|s| s.ops.push(WinOp::Position(name.clone(), p)));
    }
    if form_shown(&name) {
        st(|s| s.ops.push(WinOp::Show(name.clone())));
        pump(Some(Duration::ZERO));
    } else {
        show_window(&name);
    }
    rp_fire_event(&name, "onshow");
    after_show(&name);
    start_timers();
    if st(|s| s.cooperative) {
        st(|s| {
            s.waits.push(Wait::Form(name));
            s.wait_started = true;
        });
        return 0;
    }
    show_pending();
    while form_shown(&name) {
        step(None);
    }
    // (as FLTK's: the timers stop with the modal form)
    crate::object::rp_stop_all_timers();
    modal_ended(&name)
}

/// Whether `name` is shown modally now (setting its ModalResult closes it).
pub fn is_modal(name: &str) -> bool {
    st(|s| s.modal.contains(&lower(name)))
}

/// A modal form closed: what its ShowModal returns.
fn modal_ended(name: &str) -> i64 {
    st(|s| s.modal.retain(|f| f != name));
    rapidr_value::events::modal_result(rp_comp_get(name, "modalresult").to_i64())
}

/// `DOEVENTS`: pending events, timers and redraws get their turn.
pub fn gui_doevents() {
    if !started() {
        return;
    }
    start_timers();
    show_pending();
    step(Some(Duration::ZERO));
}

/// INPUT$'s wait with windows: events are served until a key reaches
/// INKEY$'s queue. `None` without a window shown; `Some(false)` when the
/// last window closed first.
pub fn gui_wait_key() -> Option<bool> {
    if !started() || !any_shown() {
        return None;
    }
    start_timers();
    while !rapidr_value::console::key_waiting() {
        show_pending();
        if !any_shown() {
            return Some(false);
        }
        step(None);
    }
    Some(true)
}

/// For the bytecode VM: `ShowModal` returns at once and leaves its wait to
/// the VM, which steps with [`gui_pump_wait`].
pub fn gui_set_cooperative_waits(on: bool) {
    st(|s| s.cooperative = on);
}

pub fn gui_take_wait_started() -> bool {
    st(|s| std::mem::replace(&mut s.wait_started, false))
}

/// Starts waiting for the program's windows (after the main program).
pub fn gui_begin_app_wait() {
    ensure_host();
    st(|s| s.waits.push(Wait::App));
}

/// One step of the innermost wait: `None` while it goes on, `Some` when
/// it's over (a ShowModal's: its ModalResult).
pub fn gui_pump_wait() -> Option<Value> {
    show_pending();
    let done = st(|s| match s.waits.last() {
        None => true,
        Some(Wait::Form(name)) => !s.shown.contains(name),
        Some(Wait::App) => s.shown.is_empty(),
    });
    if done {
        let finished = st(|s| s.waits.pop());
        if let Some(Wait::Form(form)) = finished {
            crate::object::rp_stop_all_timers();
            return Some(v_int(modal_ended(&form)));
        }
        return Some(v_null());
    }
    step(None);
    None
}

/// The program's windows until none is left.
pub fn run_gui_event_loop() {
    ensure_host();
    show_pending();
    while any_shown() {
        step(None);
    }
}

/// MESSAGEBOX / MESSAGEDLG / SHOWMESSAGE: the kernel-drawn dialog is the
/// dialogs lane's (Stage 8); until then the text is printed and the first
/// button taken.
pub fn gui_choice(title: &str, text: &str, labels: &[&str]) -> Option<usize> {
    pending("MESSAGEBOX");
    println!("[{title}] {text}");
    (!labels.is_empty()).then_some(0)
}

pub fn gui_dialog_execute(_name: &str, _comp_type: &str) -> Value {
    pending("dialogs (QOPENDIALOG …)");
    v_int(0)
}

// ------------------------------------------------- the facade: queries --

/// Whether a form's window shows (`None` before it's built).
pub fn window_shown(name: &str) -> Option<bool> {
    let n = lower(name);
    st(|s| s.built.contains(&n).then(|| s.shown.contains(&n)))
}

pub fn form_window_exists(name: &str) -> bool {
    st(|s| s.built.contains(&lower(name)))
}

/// Form.Scale: its screen's scale (Screen.Scale before it shows).
pub fn form_scale(name: &str) -> f64 {
    st(|s| s.scales.get(&lower(name)).copied()).unwrap_or_else(|| forced_scale().unwrap_or_else(rapidr_value::objects::bitmap::exact_scale))
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
            rp_fire_event(name, "onpaint");
        }
        "show" => gui_show(name),
        "hide" => gui_hide(name),
        _ => eprintln!("[WARN] Canvas.{method}() not implemented"),
    }
    v_null()
}

/// A QIMAGE's pictures from files and plots: the surfaces lane's (Stage 6).
pub fn image_method(_name: &str, method: &str, _args: &[Value]) -> Value {
    pending(&format!("QIMAGE.{method} (pictures from files)"));
    v_null()
}

/// A QTREEVIEW's methods that need the drawn tree: the lists lane's.
pub fn tree_method(name: &str, method: &str, _args: &[Value]) -> Value {
    match method {
        "getitemat" => return v_int(-1),
        "show" => gui_show(name),
        "hide" => gui_hide(name),
        _ => eprintln!("[WARN] TreeView.{method}() not implemented"),
    }
    v_null()
}

// ---------------------------------------------- the IDE's components --

pub fn design_surface_get(_name: &str, _prop: &str) -> Option<Value> {
    None
}
pub fn design_surface_set(_name: &str, _prop: &str, _val: &Value) -> bool {
    false
}
pub fn design_surface_method(_name: &str, _method: &str, _args: &[Value]) -> Value {
    pending("RDESIGNSURFACE");
    v_null()
}
pub fn code_editor_method(_name: &str, _method: &str, _args: &[Value]) -> Value {
    pending("RCODEEDITOR");
    v_null()
}

/// `$THEME name`: the kernel's themes are Stage 9's.
pub fn set_theme(theme: &str) {
    st(|s| s.theme = theme.to_lowercase());
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
    with_kern(|k| k.host.work_area()).unwrap_or(rapidr_ui_host_winit::HEADLESS_SCREEN)
}

pub fn mouse() -> (i64, i64) {
    with_kern(|k| k.host.mouse()).unwrap_or((0, 0))
}

pub fn monitors() -> i64 {
    ensure_host();
    with_kern(|k| k.host.monitors()).unwrap_or(1)
}

/// `Application.Minimize`: every shown window.
pub fn minimize() {
    for f in st(|s| s.shown.iter().cloned().collect::<Vec<_>>()) {
        st(|s| s.ops.push(WinOp::Minimize(f)));
    }
    if started() {
        pump(Some(Duration::ZERO));
    }
}

/// MSGBOX: the text and OK (the dialogs lane's; printed until then).
pub fn message_box(text: &str) {
    gui_choice("", text, &["OK"]);
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

/// Whether `name` shows: visible up to its form, whose window shows.
fn shown_up(name: &str) -> bool {
    let mut cur = lower(name);
    for _ in 0..64 {
        if is_form(&cur) {
            return form_shown(&cur);
        }
        if !flag(&cur, "visible", true) {
            return false;
        }
        let parent = rp_comp_get(&cur, "parent").to_string_val();
        if parent.is_empty() {
            return false;
        }
        cur = lower(&parent);
    }
    false
}

/// The form a component is on, and where it is in that window's inside
/// (logical; the in-window menu bar included), from the kernel's tree.
fn place_of(comp: &str) -> Option<(String, (i64, i64))> {
    let form = form_of(comp)?;
    let comp = lower(comp);
    with_kern(|k| {
        let f = k.desk.forms.get_mut(&form)?;
        f.ui.sync(&RtStore);
        if comp == form {
            return Some((form.clone(), (0, f.ui.menu_offset)));
        }
        let n = f.ui.node(&comp)?;
        Some((form.clone(), (n.abs.0, n.abs.1)))
    })
    .flatten()
}

/// `comp.__key_N`: the component focused, the key pressed and released
/// through the kernel's input (as the user's would be).
fn test_key(comp: &str, vk: i64) {
    let Some(form) = form_of(comp) else { return };
    let comp = lower(comp);
    let text = rapidr_value::input::text_of_vk(vk);
    with_kern(|k| {
        if let Some(f) = k.desk.forms.get_mut(&form) {
            f.ui.sync(&RtStore);
            f.ui.focus_id(&RtStore, &comp);
        }
        k.desk.key_down(&RtStore, &form, vk, &text, Mods::NONE, Source::Script);
        k.desk.key_up(&form, vk, Mods::NONE, Source::Script);
    });
}

/// `comp.__mousedown_x_y` …: the mouse at (x, y) in the component, through
/// the kernel's routing (hit test, capture), as the user's would be.
fn test_mouse(comp: &str, kind: Mouse, x: i64, y: i64) {
    let Some((form, (ox, oy))) = place_of(comp) else { return };
    let (x, y) = ((ox + x) as f64 + 0.5, (oy + y) as f64 + 0.5);
    with_kern(|k| match kind {
        Mouse::Down => k.desk.mouse_down(&RtStore, &form, (x, y), Button::Left, Mods::NONE, Source::Script),
        Mouse::Move => k.desk.mouse_move(&RtStore, &form, x, y, Mods::NONE, Source::Script),
        Mouse::Up => k.desk.mouse_up(&RtStore, &form, (x, y), Button::Left, Mods::NONE, Source::Script),
    });
}

/// `RAPIDR_TEST_RESIZE=w,h`: the frontmost form resized (Width, Height) as
/// a user dragging its border would.
fn test_resize(w: i64, h: i64) {
    let Some(form) = with_kern(|k| k.desk.stacking().last().cloned()).flatten() else { return };
    let (fw, fh) = rapidr_value::layout::form_frame(rp_comp_get(&form, "borderstyle").to_i64());
    let (iw, ih) = (w - fw, h - fh);
    with_kern(|k| {
        if let Some(f) = k.desk.forms.get_mut(&form) {
            f.ui.sync(&RtStore);
        }
        k.desk.resized(&form, iw, ih);
    });
    st(|s| s.ops.push(WinOp::Size(form, (iw, ih))));
}

fn run_test_event(e: TestEvent) {
    let comp = e.comp_lower();
    match e.action {
        Action::Fire(ref event) => {
            // (a toggle button's click goes through its group, as its press does)
            if event == "onclick" && is_toggle_button(&comp) {
                toggle_press(&comp);
            }
            rp_fire_event(&e.comp, event)
        }
        Action::Key(vk) => test_key(&comp, vk),
        Action::Mouse(kind, x, y) => test_mouse(&comp, kind, x, y),
        Action::Close => gui_close(&e.comp),
        Action::Ignored => {}
        Action::Item(_) | Action::Node(_) | Action::Toggle(_) | Action::Cell(..) | Action::Edit | Action::Enter | Action::Escape => {
            pending("the list / tree / grid test actions (__item_, __node_, __cell_ …)");
        }
    }
}

/// The test's next step, when it's due and the handlers fired so far have
/// run: the resize and splitter drag, then one event per step, then (a
/// moment after the last) the dump, the accessibility trees, the captures,
/// and the end.
fn script_step() {
    let now = Instant::now();
    let due = st(|s| s.script.as_ref().is_some_and(|sc| sc.next <= now));
    if !due || crate::object::rp_host_callback_active() {
        return;
    }
    let (started, finished) = st(|s| s.script.as_ref().map(|sc| (sc.started, sc.finished))).unwrap_or((true, true));
    if finished {
        capture_and_end();
    }
    // (busy until this step's handlers have run: a handler's ShowModal steps
    // again, and the next event waits for it, as FLTK's hook's timeout does)
    st(|s| {
        if let Some(sc) = s.script.as_mut() {
            sc.next = now + Duration::from_secs(86_400);
        }
    });
    if !started {
        let (split, resize) = st(|s| {
            let sc = s.script.as_mut().expect("a script");
            sc.started = true;
            (sc.capture.split.clone(), sc.capture.resize)
        });
        if let Some((name, delta)) = split {
            if crate::layout::splitter_begin(&name) {
                crate::layout::splitter_move(delta / 2);
                crate::layout::splitter_move(delta);
                crate::layout::splitter_end();
            }
        }
        if let Some((w, h)) = resize {
            test_resize(i64::from(w), i64::from(h));
            dispatch_pending();
        }
    }
    let next = st(|s| s.script.as_mut().and_then(|sc| sc.events.pop_front()));
    match next {
        Some(e) => {
            run_test_event(e);
            dispatch_pending();
            st(|s| {
                if let Some(sc) = s.script.as_mut() {
                    sc.next = Instant::now() + Duration::from_millis(50);
                }
            });
        }
        None => st(|s| {
            if let Some(sc) = s.script.as_mut() {
                sc.finished = true;
                sc.next = Instant::now() + Duration::from_millis(300);
            }
        }),
    }
}

/// `RAPIDR_TEST_DUMP`, `RAPIDR_TEST_A11Y`, then every shown window saved as
/// `<prefix>-<n>.bmp` (bottom to top, at its scale, by the CPU renderer),
/// and the program ends.
fn capture_and_end() -> ! {
    testhooks::print_dump(shown_up, |comp, prop| rp_comp_get(comp, prop).to_string_val());
    let prefix = st(|s| s.script.as_ref().map(|sc| sc.capture.prefix.clone())).unwrap_or_default();
    let a11y = std::env::var("RAPIDR_TEST_A11Y").ok().filter(|p| !p.is_empty());
    let shots = with_kern(|k| {
        sync_desk(k);
        let Kern { desk, .. } = k;
        let order = desk.stacking();
        let mut trees = Vec::new();
        let mut shots = Vec::new();
        for f in &order {
            if a11y.is_some() {
                let Desktop { forms, text, .. } = &mut *desk;
                if let Some(w) = forms.get_mut(f) {
                    trees.push(w.ui.access_tree(&RtStore, text).to_json());
                }
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

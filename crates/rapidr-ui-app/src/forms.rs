//! The program's forms and their windows, host-neutrally (moved from
//! runtime-core's `ui/kernel.rs`): a form's kernel side made once (OnLoad,
//! its first OnPaint waiting for the window), shown (OnShow, its screen's
//! scale), hidden, closed (OnClose's Action), the forms shown modally, a
//! user's resize and move, WindowState (simulated where the host has no
//! system), and the facade's window calls — Visible, Left … Height, Caption,
//! Center, BorderStyle, the icon — as window commands for the host
//! ([`WindowOp`]).

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};

use rapidr_value::{v_bool, v_int};

use crate::windows::{invalidate, push_op, Icon, WindowOp};
use crate::{timers, Program, Windows};

#[derive(Default)]
struct Forms {
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
    /// Modal forms the program hid (Visible = 0, Hide): their ShowModal
    /// still waits — VCL's, so RapidQ's: only Close (ModalResult) ends it
    /// (examples/forms/titlebtn.bas hides its form into the system tray and
    /// shows it again).
    hidden_modal: HashSet<String>,
    /// Each shown form's screen scale (Form.Scale).
    scales: HashMap<String, f64>,
    /// (the WindowState lane's) The bounds a maximized form goes back to
    /// (the headless host's maximize: `simulate_state`).
    normal_bounds: HashMap<String, rapidr_value::window_state::Bounds>,
}

thread_local! {
    static FORMS: RefCell<Forms> = RefCell::new(Forms::default());
    /// Inside a window change the runtime makes (Left / Top following a
    /// move): not the program's.
    static APPLYING: Cell<u32> = const { Cell::new(0) };
}

fn st<R>(f: impl FnOnce(&mut Forms) -> R) -> R {
    FORMS.with(|s| f(&mut s.borrow_mut()))
}

fn lower(name: &str) -> String {
    name.to_lowercase()
}

pub fn is_form<P: Program>(p: P, name: &str) -> bool {
    matches!(p.type_of(name).as_str(), "RFORM" | "RFORMMDI")
}

/// Runs `f` as a change the runtime makes to a window's place (Left / Top
/// stored after a move), not the program's: [`move_form`] ignores it.
fn applying(f: impl FnOnce()) {
    APPLYING.with(|a| a.set(a.get() + 1));
    f();
    APPLYING.with(|a| a.set(a.get() - 1));
}

// ---------------------------------------------------------------- state --

/// Whether form `name`'s window shows now.
pub fn form_shown(name: &str) -> bool {
    st(|s| s.shown.contains(&lower(name)))
}

/// Whether a modal form's ShowModal still waits: it's shown, or the
/// program only hid it.
pub fn modal_waits(name: &str) -> bool {
    let name = lower(name);
    st(|s| s.shown.contains(&name) || s.hidden_modal.contains(&name))
}

pub fn any_shown() -> bool {
    st(|s| !s.shown.is_empty())
}

/// The forms shown now (in no order).
pub fn shown_forms() -> Vec<String> {
    st(|s| s.shown.iter().cloned().collect())
}

/// The forms whose kernel side was made (in no order).
pub fn built_forms() -> Vec<String> {
    st(|s| s.built.iter().cloned().collect())
}

/// Whether any form was made yet.
pub fn any_built() -> bool {
    st(|s| !s.built.is_empty())
}

/// Whether a form's window shows (`None` before it's built).
pub fn window_shown(name: &str) -> Option<bool> {
    let n = lower(name);
    st(|s| s.built.contains(&n).then(|| s.shown.contains(&n)))
}

pub fn form_window_exists(name: &str) -> bool {
    st(|s| s.built.contains(&lower(name)))
}

/// Form.Scale: its screen's scale (Screen.Scale before it shows).
pub fn form_scale<R: Windows>(rt: R, name: &str) -> f64 {
    st(|s| s.scales.get(&lower(name)).copied()).unwrap_or_else(|| rt.forced_scale().unwrap_or_else(rapidr_value::objects::bitmap::exact_scale))
}

// ------------------------------------------------------------ the modal --
//
// The forms shown modally, innermost last — the program's (ShowModal) and
// the host's own (a kernel-drawn dialog, the form under a file dialog's
// sheet): input to any other window is dropped.

/// Whether `name` is shown modally now (setting its ModalResult closes it).
pub fn is_modal(name: &str) -> bool {
    st(|s| s.modal.contains(&lower(name)))
}

/// The forms shown modally, innermost last.
pub fn modal_forms() -> Vec<String> {
    st(|s| s.modal.clone())
}

/// The innermost form shown modally.
pub fn innermost_modal() -> Option<String> {
    st(|s| s.modal.last().cloned())
}

/// `id` is shown modally from now on (the innermost).
pub fn push_modal(id: &str) {
    st(|s| s.modal.push(id.to_string()));
}

/// `id` is no longer shown modally (every entry of it).
pub fn remove_modal(id: &str) {
    st(|s| s.modal.retain(|m| m != id));
}

/// `id`'s innermost entry taken off the modal list (it may be on it twice:
/// a form under a sheet that is modal itself).
pub fn remove_last_modal(id: &str) {
    st(|s| {
        if let Some(i) = s.modal.iter().rposition(|m| m == id) {
            s.modal.remove(i);
        }
    });
}

/// A modal form closed: what its ShowModal returns.
pub fn modal_ended<P: Program>(p: P, name: &str) -> i64 {
    remove_modal(name);
    rapidr_value::events::modal_result(p.get(name, "modalresult").to_i64())
}

// ----------------------------------------------------- the window's side --

/// The height of a form's in-window main menu (0 without one, and on
/// macOS).
pub fn menu_offset<R: Program + Windows>(rt: R, form: &str) -> i32 {
    let has_menu = rt.children(form).iter().any(|(_, t)| t == "RMAINMENU");
    if has_menu && rt.menu_in_window() {
        rapidr_value::layout::MAIN_MENU_HEIGHT as i32
    } else {
        0
    }
}

/// A form's window inside: Width / Height less the frame the window
/// system draws (the in-window menu included).
pub fn form_window_size<P: Program>(p: P, name: &str) -> (i64, i64) {
    let (fw, fh) = rapidr_value::layout::form_frame(p.get(name, "borderstyle").to_i64());
    ((p.get(name, "width").to_i64() - fw).clamp(1, 100_000), (p.get(name, "height").to_i64() - fh).clamp(1, 100_000))
}

/// A form's icon: its IcoHandle / Icon, else the application's.
pub fn icon_of<P: Program>(p: P, name: &str) -> Option<Icon> {
    let own = ["icohandle", "icon"].into_iter().map(|prop| p.get(name, prop)).find(rapidr_value::objects::has_icon);
    let (w, h, rgba, _) = own.or_else(rapidr_value::globals::application_icon).and_then(|v| rapidr_value::objects::icon_pixels(&v))?;
    Some(Icon { width: w as u32, height: h as u32, rgba })
}

/// A form's position centred on the screen: its outer Width × Height, as
/// Left / Top are (RC.EXE: a 300 × 200 form on a 1920 × 1012 screen shows
/// at 810, 406).
pub fn centered<R: Program + Windows>(rt: R, name: &str) -> (i64, i64) {
    let (sw, sh) = rt.screen();
    let (w, h) = (rt.get(name, "width").to_i64(), rt.get(name, "height").to_i64());
    ((sw - w) / 2, (sh - h) / 2)
}

/// A form asked to be centred (`Center`) shown the first time: its Left /
/// Top set to the screen's middle now, so the window opens there. RapidQ
/// centres a form when it shows, not when `Center` runs (RC.EXE: Left / Top
/// read 0 until `Show`, then the centred place).
fn place_centered<R: Program + Windows>(rt: R, name: &str) {
    if rt.get(name, "_center").to_i64() == 0 {
        return;
    }
    let (x, y) = centered(rt, name);
    applying(|| {
        rt.quietly(&mut || {
            rt.set(name, "left", v_int(x));
            rt.set(name, "top", v_int(y));
        })
    });
}

// ----------------------------------------------------- show and hide --

/// A form's kernel side, made the first time (OnLoad once, the first
/// OnPaint waiting for its window to show).
pub fn build_form<P: Program>(p: P, name: &str) {
    let name = lower(name);
    if !st(|s| s.built.insert(name.clone())) {
        return;
    }
    p.fire(&name, "onload");
    p.form_built(&name);
    st(|s| s.first_paint.insert(name));
}

/// The form's window shown (made the first time), before its OnShow: the
/// host carries it out now, so the window exists.
pub fn show_window<R: Program + Windows>(rt: R, name: &str) {
    let name = lower(name);
    st(|s| {
        s.shown.insert(name.clone());
        s.hidden_modal.remove(&name);
    });
    push_op(WindowOp::Show(name.clone()));
    rt.flush();
    // (the WindowState lane's: shown maximized as asked — the system did it
    // with the window; the headless host's form takes the work area now)
    let state = rapidr_value::window_state::of(rt.get(&name, "windowstate").to_i64());
    if state == rapidr_value::window_state::WS_MAXIMIZED && rt.headless() && !st(|s| s.normal_bounds.contains_key(&name)) {
        simulate_state(rt, &name, rapidr_value::window_state::WS_NORMAL, state);
    }
    // (the DirectX lane's: a QDXSCREEN's FullScreen)
    if rt.form_fullscreen(&name) {
        fullscreen(rt, &name);
    }
}

/// (the DirectX lane's) A form whose QDXSCREEN is FullScreen: its window
/// covers the screen without a frame (RapidQ's DirectDraw exclusive mode,
/// without its display mode change) — the system's borderless full screen,
/// or on the headless host the form taking the screen itself, as a user's
/// drag would (Left / Top / Width / Height, its layout, OnResize).
pub fn fullscreen<R: Program + Windows>(rt: R, name: &str) {
    push_op(WindowOp::Fullscreen(name.to_string()));
    rt.flush();
    if rt.headless() {
        let (sw, sh) = rt.screen();
        let (fw, fh) = rapidr_value::layout::form_frame(rt.get(name, "borderstyle").to_i64());
        let (iw, ih) = ((sw - fw).max(1), (sh - fh).max(1));
        rt.system_resized(name, (iw, ih), (0, 0));
        push_op(WindowOp::Size(name.to_string(), (iw, ih)));
    }
    rt.dispatch_pending();
}

pub fn hide_window(name: &str) {
    let name = lower(name);
    if st(|s| s.shown.remove(&name)) {
        push_op(WindowOp::Hide(name));
    }
}

/// A form's window shown: drawn at its screen's scale from now on, then
/// (the first time) its OnPaint — as Windows' WM_PAINT comes once a window
/// shows, after OnShow.
pub fn after_show<R: Program + Windows>(rt: R, name: &str) {
    let name = lower(name);
    let host_scale = rt.window_scale(&name);
    let scale = rt.forced_scale().or(host_scale).unwrap_or(1.0);
    rapidr_value::objects::bitmap::set_display_scale(scale);
    st(|s| s.scales.insert(name.clone(), scale));
    if st(|s| s.first_paint.remove(&name)) {
        fire_first_paint(rt, &name);
    }
}

/// OnPaint for `parent` and the canvases on it, depth first.
pub fn fire_first_paint<P: Program>(p: P, parent: &str) {
    p.fire(parent, "onpaint");
    for (child, type_name) in p.children(parent) {
        if type_name.eq_ignore_ascii_case("RCANVAS") {
            p.fire(&child, "onpaint");
        } else {
            fire_first_paint(p, &child);
        }
    }
}

/// The program waits: the windows it made visible show (unless it hid
/// them again meanwhile).
pub fn show_pending<R: Program + Windows>(rt: R) {
    for name in st(|s| std::mem::take(&mut s.pending_shows)) {
        if window_shown(&name) != Some(true) && rt.get(&name, "visible").to_bool() {
            show(rt, &name);
        }
    }
}

/// Shows a form without waiting (OnShow when it wasn't showing).
pub fn show<R: Program + Windows>(rt: R, name: &str) {
    rt.start();
    if !is_form(rt, name) {
        // (a component shown: drawn again; its Visible says)
        rt.store(&lower(name), "visible", v_bool(true));
        return invalidate();
    }
    let was_built = window_shown(name).is_some();
    if was_built && form_shown(name) {
        // (already showing: on top)
        push_op(WindowOp::Show(lower(name)));
        return;
    }
    if !was_built {
        place_centered(rt, name);
    }
    build_form(rt, name);
    show_window(rt, name);
    rt.fire(name, "onshow");
    after_show(rt, name);
}

/// `Form.Visible = True`: its Show; a form not built yet (its own CREATE)
/// shows once the program waits.
pub fn show_visible<R: Program + Windows>(rt: R, name: &str) {
    if window_shown(name).is_some() {
        show(rt, name);
        return;
    }
    rt.start();
    st(|s| s.pending_shows.push(name.to_string()));
}

/// Hides a form's window (no OnClose).
pub fn hide<P: Program>(p: P, name: &str) {
    if is_form(p, name) {
        hide_modal(name);
        hide_window(name);
    } else {
        p.store(&lower(name), "visible", v_bool(false));
        invalidate();
    }
}

/// `Form.Close` and the window's close box: OnClose's `Action` (it starts
/// as `caHide`) decides whether the form goes, stays or is minimized.
pub fn close<P: Program>(p: P, name: &str) {
    use rapidr_value::events::{CloseAction, CA_HIDE};
    // (a modal form the program hid: Close ends its ShowModal)
    st(|s| s.hidden_modal.remove(&lower(name)));
    if !is_form(p, name) || window_shown(name).is_none() {
        hide_window(name);
        if !is_form(p, name) {
            hide(p, name);
        }
        return;
    }
    let name = lower(name);
    p.fire_then(
        &name.clone(),
        "onclose",
        &[v_int(CA_HIDE)],
        Box::new(move |a| match CloseAction::of(&a[0]) {
            CloseAction::Stay => {}
            CloseAction::Minimize => push_op(WindowOp::Minimize(name.clone())),
            CloseAction::Close => hide_window(&name),
        }),
    );
}

/// `Visible`: a built form's window shows or hides (no OnShow: Show fires
/// that); a component is read from the store when painted.
pub fn set_visible<R: Program + Windows>(rt: R, name: &str, visible: bool) {
    if is_form(rt, name) && window_shown(name).is_some() {
        if visible {
            show_window(rt, name);
            after_show(rt, name);
        } else {
            hide_modal(name);
            hide_window(name);
        }
        return;
    }
    invalidate();
}

/// A modal form the program hides keeps its ShowModal waiting.
fn hide_modal(name: &str) {
    let name = lower(name);
    st(|s| {
        if s.modal.contains(&name) && s.shown.contains(&name) {
            s.hidden_modal.insert(name);
        }
    });
}

/// `Form.ShowModal`, up to its wait: ModalResult 0, on the modal list,
/// built, centred if asked, shown (OnShow, its first OnPaint), the timers
/// started. The wait is the runtime's (a native build's loop, the VM's
/// [`crate::waits::Wait::Form`]).
pub fn begin_modal<R: Program + Windows>(rt: R, name: &str) {
    let name = lower(name);
    rt.store(&name, "modalresult", v_int(0));
    push_modal(&name);
    if window_shown(&name).is_none() {
        place_centered(rt, &name);
    } else if rt.get(&name, "_center").to_i64() != 0 {
        let p = centered(rt, &name);
        push_op(WindowOp::Position(name.clone(), p));
    }
    build_form(rt, &name);
    if form_shown(&name) {
        push_op(WindowOp::Show(name.clone()));
        rt.flush();
    } else {
        show_window(rt, &name);
    }
    rt.fire(&name, "onshow");
    after_show(rt, &name);
    timers::start_all(rt);
}

// ------------------------------------------------------------ geometry --

/// Left / Top / Width / Height: a form's window takes its new size.
pub fn apply_geometry<P: Program>(p: P, name: &str) {
    if is_form(p, name) && window_shown(name).is_some() {
        push_op(WindowOp::Size(lower(name), form_window_size(p, name)));
    }
    invalidate();
}

/// `Form.Center`: on the screen's middle (when shown; ShowModal centres a
/// form asked to be before it showed).
pub fn center<R: Program + Windows>(rt: R, name: &str) {
    rt.set(name, "_center", v_int(1));
    if window_shown(name).is_none() {
        return;
    }
    let (x, y) = centered(rt, name);
    push_op(WindowOp::Position(lower(name), (x, y)));
    applying(|| {
        rt.quietly(&mut || {
            rt.set(name, "left", v_int(x));
            rt.set(name, "top", v_int(y));
        })
    });
}

/// `Form.Left` / `Form.Top` set by the program: the window moves there.
pub fn move_form<P: Program>(p: P, name: &str) {
    if APPLYING.with(Cell::get) > 0 || window_shown(name).is_none() {
        return;
    }
    let at = (p.get(name, "left").to_i64(), p.get(name, "top").to_i64());
    push_op(WindowOp::Position(lower(name), at));
}

/// `Form.BorderStyle`: bsNone (0) takes away the window's frame.
pub fn set_form_border<P: Program>(p: P, name: &str) {
    if window_shown(name).is_some() {
        push_op(WindowOp::Border(lower(name), p.get(name, "borderstyle").to_i64() != 0));
    }
    apply_geometry(p, name);
}

pub fn apply_icon<P: Program>(p: P, name: &str) {
    if is_form(p, name) && window_shown(name).is_some() {
        let icon = icon_of(p, name);
        push_op(WindowOp::Icon(lower(name), icon));
    }
}

/// `Application.Icon` changed: every form without its own.
pub fn apply_icons<P: Program>(p: P) {
    for f in built_forms() {
        apply_icon(p, &f);
    }
}

/// A caption: a form's is its window's title.
pub fn set_caption<P: Program>(p: P, name: &str, text: &str) {
    if is_form(p, name) {
        push_op(WindowOp::Title(lower(name), text.to_string()));
    }
    invalidate();
}

/// The user resized a form's window to `w` × `h` (its inside, the
/// in-window menu included): its Width / Height follow (within its
/// Constraints), its aligned and anchored children are laid out again,
/// OnResize and OnPaint fire.
pub fn form_resized<P: Program>(p: P, form: &str, w: i64, h: i64) {
    let (fw, fh) = rapidr_value::layout::form_frame(p.get(form, "borderstyle").to_i64());
    let asked = (w + fw, h + fh);
    let (w, h) = p.constraints(form).size(asked.0, asked.1);
    let same = p.get(form, "width").to_i64() == w && p.get(form, "height").to_i64() == h;
    if same {
        // (dragged outside them: the window goes back)
        if (w, h) != asked {
            apply_geometry(p, form);
        }
        return;
    }
    p.quietly(&mut || {
        p.set(form, "width", v_int(w));
        p.set(form, "height", v_int(h));
    });
    p.client_changed(form);
    apply_geometry(p, form);
    p.fire(form, "onresize");
    p.fire(form, "onpaint");
}

/// The user (or the system) moved a form's window: its Left / Top follow,
/// quietly.
pub fn form_moved<P: Program>(p: P, form: &str, x: i64, y: i64) {
    applying(|| {
        p.quietly(&mut || {
            p.set(form, "left", v_int(x));
            p.set(form, "top", v_int(y));
        })
    });
}

/// A form's window moved to a screen with another scale: told
/// (OnScaleChanged) and drawn again at it (OnPaint).
pub fn scale_changed<R: Program + Windows>(rt: R, form: &str, scale: f64) {
    if rt.forced_scale().is_some() {
        return;
    }
    let changed = st(|s| match s.scales.insert(lower(form), scale) {
        Some(old) => (old - scale).abs() > f64::EPSILON,
        None => false,
    });
    if changed {
        rapidr_value::objects::bitmap::set_display_scale(scale);
        rt.fire(form, "onscalechanged");
        fire_first_paint(rt, form);
    }
}

// --------------------------------------------------------- WindowState --

/// (the WindowState lane's) `Form.WindowState` set (it was `from`): its
/// window maximized, minimized or restored by the system, whose word comes
/// back as Resized / Moved (Left … Height follow, OnResize) and the window's
/// state (`Desktop::window_state`). Where the host has no system (the
/// desktop's headless host) the form takes the work area itself
/// ([`simulate_state`]). A form not shown yet takes its state when it
/// shows.
pub fn set_window_state<R: Program + Windows>(rt: R, name: &str, from: i64) {
    let name = lower(name);
    if !is_form(rt, &name) || !form_shown(&name) {
        return;
    }
    let to = rapidr_value::window_state::of(rt.get(&name, "windowstate").to_i64());
    push_op(WindowOp::State(name.clone(), to));
    if rt.headless() {
        rt.flush();
        simulate_state(rt, &name, from, to);
    } else {
        // (the system's answer, as soon as it comes)
        rt.flush();
        rt.dispatch_pending();
    }
}

/// The headless host's maximize and restore (`rapidr_value::window_state::
/// change`): the form moved to the work area (its bounds kept to come back
/// to) or back, as a user's drag would — Left / Top / Width / Height follow,
/// its layout, OnResize.
pub fn simulate_state<R: Program + Windows>(rt: R, name: &str, from: i64, to: i64) {
    let get = |p: &str| rt.get(name, p).to_i64();
    let current = (get("left"), get("top"), get("width"), get("height"));
    let (ww, wh) = rt.work_area();
    let saved = st(|s| s.normal_bounds.get(name).copied());
    let (bounds, keep) = rapidr_value::window_state::change(from, to, current, saved, (0, 0, ww, wh));
    st(|s| match keep {
        Some(b) => {
            s.normal_bounds.insert(name.to_string(), b);
        }
        None => {
            s.normal_bounds.remove(name);
        }
    });
    let Some((left, top, w, h)) = bounds else { return };
    let (fw, fh) = rapidr_value::layout::form_frame(get("borderstyle"));
    let (iw, ih) = ((w - fw).max(1), (h - fh).max(1));
    rt.system_resized(name, (iw, ih), (left, top));
    push_op(WindowOp::Size(name.to_string(), (iw, ih)));
    rt.dispatch_pending();
}

// ----------------------------------------------------- toggle buttons --

pub fn is_toggle_button<P: Program>(p: P, name: &str) -> bool {
    matches!(p.type_of(name).as_str(), "RCOOLBTN" | "ROVALBTN")
}

/// The program set a QCOOLBTN's / QOVALBTN's Down: the others of its
/// group come up (host-neutral: rapidr_value::toggle_group).
pub fn toggle_down_set<P: Program>(p: P, name: &str) {
    if !is_toggle_button(p, name) {
        return;
    }
    let name = lower(name);
    let down = p.get(&name, "down").to_bool();
    let mut changes = rapidr_value::toggle_group::set_down(&name, down, &toggle_members(p, &name));
    changes.push((name, down));
    toggle_apply(p, changes);
}

/// The toggle buttons sharing `name`'s parent.
fn toggle_members<P: Program>(p: P, name: &str) -> Vec<rapidr_value::toggle_group::Member> {
    let parent = p.get(name, "parent").to_string_val();
    p.children(&parent)
        .into_iter()
        .filter(|(_, t)| matches!(t.as_str(), "RCOOLBTN" | "ROVALBTN"))
        .map(|(n, _)| rapidr_value::toggle_group::Member { group: p.get(&n, "groupindex").to_i64(), down: p.get(&n, "down").to_bool(), name: n })
        .collect()
}

/// The new Down values, stored (and drawn).
fn toggle_apply<P: Program>(p: P, changes: Vec<(String, bool)>) {
    for (n, down) in changes {
        p.store(&n, "down", v_int(if down { -1 } else { 0 }));
    }
    invalidate();
}

/// The user pressed a QCOOLBTN / QOVALBTN (its group decides what's down).
pub fn toggle_press<P: Program>(p: P, name: &str) {
    let allow_all_up = p.get(name, "allowallup").to_bool();
    toggle_apply(p, rapidr_value::toggle_group::press(name, allow_all_up, &toggle_members(p, name)));
}

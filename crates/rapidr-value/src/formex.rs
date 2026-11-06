//! RFormEx — a form with more (docs/rformex-design.md): files dropped one
//! by one (OnDrag, DragZone, DropPoint), its own system tray icon
//! (AddTrayIcon, TRAYICO's pictures, OnTrayClick …), the window see-through
//! (Transparent, TransparentClient), on top (fsStayOnTop), OnMinimise,
//! OnGetFocus / OnLostFocus, and the window's messages (OnMessage). At run
//! time it is an RFORM ([`register`]ed here, as `crate::mdi` registers an
//! RFormMDI); this is the model both runtimes share — which forms are
//! RFormExes, which behave as Rapidq2.inc's QFormEx ([`is_classic`]: the
//! program wrote QFormEx), the drop zone, OnMessage's messages, the tray
//! icon's clicks as events, the wheel the classic way. The glue that fires
//! events and asks the windows is `rapidr_ui_app::formex`.

use std::cell::RefCell;
use std::collections::HashMap;

use crate::{v_int, Value};

/// The property the compilers set first on a form written QFormEx
/// (`rapidr_ast::formex::CLASSIC_PROPERTY`).
pub const CLASSIC: &str = "__classic";

/// The form's own tray icon's uID (its hWnd is the form's Handle).
pub const TRAY_UID: i64 = 0x5246;
/// The message its clicks come as (the include's WM_TRAYICON).
pub const WM_TRAYICON: i64 = 0x590;

/// Windows' messages RFormEx turns into its own events: OnMessage doesn't
/// hear them (the include's own split) — WM_DROPFILES, WM_ACTIVATE,
/// WM_KILLFOCUS, WM_SYSCOMMAND, WM_MOUSEWHEEL and the tray icon's.
pub const OWN_EVENT_MESSAGES: [i64; 6] = [0x0233, 0x0006, 0x0008, 0x0112, 0x020A, WM_TRAYICON];

/// Windows' WM_SIZE and WM_MOVE (what OnMessage hears off Windows too).
pub const WM_SIZE: i64 = 0x0005;
pub const WM_MOVE: i64 = 0x0003;
/// WM_SYSCOMMAND and its SC_MINIMIZE (a classic OnMinimise, before).
pub const WM_SYSCOMMAND: i64 = 0x0112;
pub const SC_MINIMIZE: i64 = 0xF020;

/// fsStayOnTop (FormStyle).
pub const FS_STAY_ON_TOP: i64 = 3;

#[derive(Default)]
struct FormEx {
    /// Written QFormEx: the include's behaviour.
    classic: bool,
    /// The tray icon is shown.
    tray: bool,
    /// The animation's picture now (0-based), when it runs.
    frame: usize,
    /// Where the last drop landed (client pixels).
    drop: (i64, i64),
    /// The program set Transparent (a classic form's window stays as it is
    /// until then).
    alpha_set: bool,
    /// (classic, OnMinimise bound) Windows' SC_MINIMIZE waits for the
    /// handler: the window minimises after it.
    hold_minimise: bool,
    /// The window is the active one (OnGetFocus / OnLostFocus once each).
    active: bool,
}

thread_local! {
    static FORMS: RefCell<HashMap<String, FormEx>> = RefCell::new(HashMap::new());
}

fn with<R>(form: &str, f: impl FnOnce(&mut FormEx) -> R) -> Option<R> {
    FORMS.with(|m| m.borrow_mut().get_mut(&form.to_ascii_lowercase()).map(f))
}

/// Form `form` is an RFormEx (the runtimes' create of an RFORMEX).
pub fn register(form: &str) {
    FORMS.with(|m| {
        m.borrow_mut().entry(form.to_ascii_lowercase()).or_default();
    });
}

/// Whether form `form` is an RFormEx (no allocation: every read of a
/// form's property may ask).
pub fn is(form: &str) -> bool {
    FORMS.with(|m| {
        let m = m.borrow();
        !m.is_empty() && m.keys().any(|k| k.eq_ignore_ascii_case(form))
    })
}

/// The program wrote QFormEx for it: it behaves as the include's form.
pub fn set_classic(form: &str) {
    with(form, |f| f.classic = true);
}

/// Whether it behaves as the include's form (written QFormEx).
pub fn is_classic(form: &str) -> bool {
    with(form, |f| f.classic).unwrap_or(false)
}

/// (a classic form whose OnMinimise is bound) Its window's minimise waits
/// for OnMinimise: before, as the include's.
pub fn hold_minimise(form: &str) {
    with(form, |f| f.hold_minimise = true);
}

/// Whether form `form`'s minimise waits for its OnMinimise (on Windows,
/// where the host can hold the system's SC_MINIMIZE).
pub fn holds_minimise(form: &str) -> bool {
    with(form, |f| f.hold_minimise).unwrap_or(false)
}

/// Whether the host keeps Windows message `msg` (wParam `wparam`) from the
/// window of form `form` until the program has heard it: a classic form's
/// SC_MINIMIZE (its OnMinimise first, then the window minimises).
pub fn holds_message(form: &str, msg: i64, wparam: i64) -> bool {
    msg == WM_SYSCOMMAND && (wparam & 0xFFF0) == SC_MINIMIZE && holds_minimise(form)
}

/// The window became (or stopped being) the active one: whether that is
/// news (OnGetFocus / OnLostFocus fire once for each change).
pub fn set_active(form: &str, active: bool) -> bool {
    with(form, |f| std::mem::replace(&mut f.active, active) != active).unwrap_or(false)
}

/// The forms that are RFormExes (lower case).
pub fn all() -> Vec<String> {
    FORMS.with(|m| m.borrow().keys().cloned().collect())
}

/// Whether OnMessage hears Windows message `msg` (on Windows; elsewhere
/// only the messages the runtimes make for it).
pub fn message_heard(msg: i64) -> bool {
    !OWN_EVENT_MESSAGES.contains(&msg)
}

/// Whether a drop at (x, y) (client pixels) counts for a form whose
/// DragZone is (left, top, right, bottom): inside it (right and bottom
/// outside), or anywhere when the zone is empty — except for a classic
/// form, whose empty zone (as it starts) takes none, as the include's.
pub fn in_zone(zone: (i64, i64, i64, i64), classic: bool, (x, y): (i64, i64)) -> bool {
    let (l, t, r, b) = zone;
    if r <= l || b <= t {
        return !classic;
    }
    x >= l && y >= t && x < r && y < b
}

thread_local! {
    /// Where the files being dropped landed, by form (client pixels): the
    /// host says so as it reports the drop.
    static DROP_AT: RefCell<HashMap<String, (i64, i64)>> = RefCell::new(HashMap::new());
}

/// The host: the files dropped on form `form` landed at `at` (any form;
/// only an RFormEx reads it).
pub fn note_drop_at(form: &str, at: (i64, i64)) {
    DROP_AT.with(|d| d.borrow_mut().insert(form.to_ascii_lowercase(), at));
}

/// Where the last files dropped on form `form` landed ((0, 0) when no host
/// said).
pub fn drop_at(form: &str) -> (i64, i64) {
    DROP_AT.with(|d| d.borrow().get(&form.to_ascii_lowercase()).copied()).unwrap_or((0, 0))
}

/// The last drop's point, kept for DropPoint.
pub fn set_drop_point(form: &str, at: (i64, i64)) {
    with(form, |f| f.drop = at);
}

/// DropPoint.X / DropPoint.Y.
pub fn drop_point(form: &str) -> (i64, i64) {
    with(form, |f| f.drop).unwrap_or((0, 0))
}

/// The tray icon shown or taken away (the model's note of it).
pub fn set_tray_shown(form: &str, shown: bool) {
    with(form, |f| {
        f.tray = shown;
        if !shown {
            f.frame = 0;
        }
    });
}

/// Whether its tray icon is shown (TrayIconShown).
pub fn tray_shown(form: &str) -> bool {
    with(form, |f| f.tray).unwrap_or(false)
}

/// The animation's next picture of `count` (0-based), stepped.
pub fn next_frame(form: &str, count: usize) -> usize {
    with(form, |f| {
        f.frame = if count == 0 { 0 } else { (f.frame + 1) % count };
        f.frame
    })
    .unwrap_or(0)
}

/// The program set Transparent.
pub fn note_alpha_set(form: &str) {
    with(form, |f| f.alpha_set = true);
}

/// Whether the window's opacity is the program's to say yet: always for
/// an RFormEx, once Transparent was set for a classic one.
pub fn alpha_active(form: &str) -> bool {
    with(form, |f| !f.classic || f.alpha_set).unwrap_or(false)
}

/// What Transparent reads before the program sets it: 255 (opaque) for an
/// RFormEx, 0 for a classic one (the include's field).
pub fn transparent_default(form: &str) -> i64 {
    if is_classic(form) {
        0
    } else {
        255
    }
}

/// The events a mouse message on the form's tray icon fires, in order:
/// OnTrayClick on the right button's release (the include's), OnTrayDblClick
/// on the left button's double click, RapidR's OnTrayMouseUp(Button) on any
/// release (mbLeft 0, mbRight 1, mbMiddle 2).
pub fn tray_events(mouse: i64) -> Vec<(&'static str, Vec<Value>)> {
    use crate::tray::{WM_LBUTTONDBLCLK, WM_LBUTTONUP, WM_MBUTTONUP, WM_RBUTTONUP};
    match mouse {
        WM_RBUTTONUP => vec![("ontrayclick", vec![]), ("ontraymouseup", vec![v_int(1)])],
        WM_LBUTTONDBLCLK => vec![("ontraydblclick", vec![])],
        WM_LBUTTONUP => vec![("ontraymouseup", vec![v_int(0)])],
        WM_MBUTTONUP => vec![("ontraymouseup", vec![v_int(2)])],
        _ => vec![],
    }
}

/// A classic form's OnMouseWheel arguments, as the include passed them:
/// (1 or -1 — the sign of the turn, away from the person positive; the
/// screen's x as Windows' low word; the screen's y as the include worked
/// it out from Windows' lParam; Windows' MK_ key flags — MK_SHIFT 4,
/// MK_CONTROL 8 — from RapidQ's Shift state `shift`, ssShift / ssCtrl).
pub fn classic_wheel(notches_up: i64, screen: (i64, i64), shift: i64) -> [i64; 4] {
    use crate::input::{SS_CTRL, SS_SHIFT};
    let rotation = notches_up.signum();
    let lparam = ((screen.0 & 0xFFFF) | ((screen.1 & 0xFFFF) << 16)) as i32;
    let x = i64::from(lparam) & 0xFFFF;
    let y = (f64::from(lparam) / 65535.0).round() as i64;
    let mut mk = 0;
    for (ss, flag) in [(SS_SHIFT, 4), (SS_CTRL, 8)] {
        if shift & ss != 0 {
            mk |= flag;
        }
    }
    [rotation, x, y, mk]
}

/// The WM_SIZE OnMessage hears off Windows: (msg, wParam — 0 restored, 1
/// minimised, 2 maximised —, lParam — the client width + height × 65536).
pub fn wm_size(state: i64, w: i64, h: i64) -> [i64; 3] {
    [WM_SIZE, state.clamp(0, 2), i64::from(((w & 0xFFFF) | ((h & 0xFFFF) << 16)) as i32)]
}

/// The WM_MOVE OnMessage hears off Windows: (msg, 0, lParam — the client
/// area's left + top × 65536 on the screen, 16 bits each).
pub fn wm_move(x: i64, y: i64) -> [i64; 3] {
    [WM_MOVE, 0, i64::from(((x & 0xFFFF) | ((y & 0xFFFF) << 16)) as i32)]
}

/// A Transparent value the program may set (0 to 255; others ignored, as
/// the include's).
pub fn alpha_valid(v: i64) -> bool {
    (0..=255).contains(&v)
}

/// FormStyle's "on top" (only fsStayOnTop; fsNormal and the others not).
pub fn on_top(form_style: i64) -> bool {
    form_style == FS_STAY_ON_TOP
}

/// The form is gone: its record too.
pub fn forget(form: &str) {
    FORMS.with(|m| m.borrow_mut().remove(&form.to_ascii_lowercase()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classic_and_rapidr_forms() {
        register("fx1");
        register("fx2");
        set_classic("fx2");
        assert!(is("FX1") && is("fx2") && !is("nope"));
        assert!(!is_classic("fx1") && is_classic("fx2"));
        assert_eq!((transparent_default("fx1"), transparent_default("fx2")), (255, 0));
        assert!(alpha_active("fx1") && !alpha_active("fx2"));
        note_alpha_set("fx2");
        assert!(alpha_active("fx2"));
        forget("fx1");
        forget("fx2");
        assert!(!is("fx1"));
    }

    #[test]
    fn the_drop_zone() {
        // (empty: the whole form for an RFormEx, nothing for a classic one)
        assert!(in_zone((0, 0, 0, 0), false, (10, 10)));
        assert!(!in_zone((0, 0, 0, 0), true, (10, 10)));
        // (right and bottom outside)
        let z = (10, 20, 110, 70);
        for classic in [false, true] {
            assert!(in_zone(z, classic, (10, 20)));
            assert!(in_zone(z, classic, (109, 69)));
            assert!(!in_zone(z, classic, (110, 30)));
            assert!(!in_zone(z, classic, (50, 70)));
            assert!(!in_zone(z, classic, (9, 30)));
        }
    }

    #[test]
    fn on_message_leaves_out_its_own_events() {
        for m in [0x0233, 0x0006, 0x0008, 0x0112, 0x020A, 0x590] {
            assert!(!message_heard(m), "{m:#x}");
        }
        // (MM_MIM_DATA, MM_MIM_LONGDATA, WM_USER + 1, WM_SIZE, WM_SETFOCUS)
        for m in [0x3C3, 0x3C4, 0x401, 0x0005, 0x0007] {
            assert!(message_heard(m), "{m:#x}");
        }
    }

    #[test]
    fn tray_clicks_as_events() {
        let names = |m| tray_events(m).into_iter().map(|(n, a)| (n, a.first().map(Value::to_i64))).collect::<Vec<_>>();
        assert_eq!(names(0x205), vec![("ontrayclick", None), ("ontraymouseup", Some(1))]);
        assert_eq!(names(0x203), vec![("ontraydblclick", None)]);
        assert_eq!(names(0x202), vec![("ontraymouseup", Some(0))]);
        assert_eq!(names(0x208), vec![("ontraymouseup", Some(2))]);
        assert!(names(0x201).is_empty() && names(0x204).is_empty());
    }

    #[test]
    fn the_classic_wheel() {
        use crate::input::{SS_ALT, SS_CTRL, SS_SHIFT};
        // (up two notches at (300, 200) on the screen, Shift held: one turn)
        assert_eq!(classic_wheel(2, (300, 200), SS_SHIFT), [1, 300, 200, 4]);
        assert_eq!(classic_wheel(-1, (5, 7), SS_CTRL | SS_ALT), [-1, 5, 7, 8]);
    }

    #[test]
    fn messages_made_off_windows() {
        assert_eq!(wm_size(0, 640, 480), [5, 0, 640 | (480 << 16)]);
        assert_eq!(wm_size(2, 1, 1), [5, 2, 0x1_0001]);
        assert_eq!(wm_move(100, 50), [3, 0, 100 | (50 << 16)]);
        // (a left edge off the screen: 16 bits, as Windows)
        assert_eq!(wm_move(-8, 0)[2], 0xFFF8);
    }

    #[test]
    fn the_animation_takes_turns() {
        register("fxa");
        assert_eq!((next_frame("fxa", 3), next_frame("fxa", 3), next_frame("fxa", 3)), (1, 2, 0));
        set_tray_shown("fxa", true);
        assert!(tray_shown("fxa"));
        set_tray_shown("fxa", false);
        assert_eq!(next_frame("fxa", 2), 1);
        forget("fxa");
    }
}

//! The platform on the kernel host (the platform lane, plan Stage 9): the
//! window frames BorderStyle and BorderIcons ask for, Screen.Cursor for
//! the host's pointer (`rapidr_ui_host_winit::platform`), `$THEME` and
//! `Application.Theme` (the kernel's themes, `rapidr_value::theme`).
//! Screen.Width / Height, the work area, the monitors and the mouse are the
//! host's answers (`Host::screen` …), asked by kernel.rs's facade.

use rapidr_ui_host_winit::{Desktop, Frame, HostCmd};

use crate::ui::program::Rt;

/// Form `name`'s frame: its BorderStyle and BorderIcons (all three when
/// never set; `rapidr_ui_app::desktop`'s rule, the web host's too).
pub(super) fn frame(name: &str) -> Frame {
    rapidr_ui_app::desktop::frame(Rt, name)
}

/// Before each pump: Screen.Cursor for the pointer, and the frames of the
/// program's forms whose BorderStyle / BorderIcons changed since their
/// window was made (no store hook tells the host about BorderIcons).
pub(super) fn sync(desk: &mut Desktop) {
    system_theme();
    desk.screen_cursor = crate::globals::screen_cursor();
    let mut changed = Vec::new();
    for (id, f) in desk.forms.iter_mut() {
        if rapidr_ui_kernel::dialogs::is_dialog(id) {
            continue;
        }
        let now = frame(id);
        if f.spec.frame != now {
            f.spec.frame = now;
            changed.push(id.clone());
        }
    }
    desk.cmds.extend(changed.into_iter().map(HostCmd::Border));
}

/// `$THEME name` and `Application.Theme = name` (`rapidr_value::theme::
/// choose`): the kernel draws with that theme from now on, every form drawn
/// again. `rapidr` (and `auto`) is RapidR's look as the system is — light,
/// dark or high contrast — and as it becomes; a name no theme has draws
/// RapidR's look too, said once.
pub(super) fn theme(name: &str) {
    use rapidr_value::theme::{self, Choice};
    let n = name.trim().to_lowercase();
    match theme::choose(&n) {
        Choice::Theme(t) => theme::set(t),
        choice => {
            if choice == Choice::Unknown {
                super::pending(&format!("$THEME {n} (it draws RapidR's look)"));
            }
            let (dark, contrast) = system_look();
            theme::follow_system(dark, contrast);
        }
    }
    rapidr_ui_app::windows::invalidate();
}

/// How the system looks (dark, high contrast): macOS' appearance and
/// Increase Contrast, Windows' app mode and high contrast, Linux's GTK
/// theme — asked of the system before the window system is up too (RapidR
/// Studio sets `Application.Theme = "rapidr"` before its first window: it
/// opens in the system's look, not light until the first pump). A GUI
/// test's headless host says light, so captures don't depend on the
/// machine, unless the test says otherwise (`RAPIDR_TEST_SYSTEM_LOOK`: a
/// look, a file, or `system` — the real system's; a `__look_` step:
/// rapidr_ui_app's testhooks).
pub(crate) fn system_look() -> (bool, bool) {
    if let Some(look) = rapidr_ui_app::testhooks::system_look() {
        return look;
    }
    if super::headless_look() && !rapidr_ui_app::testhooks::real_system_look() {
        (false, false)
    } else {
        rapidr_ui_host_winit::platform::system_look()
    }
}

/// As the program starts: RapidR's look (a program that names no theme)
/// is the system's from the first thing drawn or read — the system asked
/// when a theme is first needed (`rapidr_value::theme::set_system_probe`),
/// then twice a second ([`system_theme`]).
pub(crate) fn start() {
    rapidr_value::theme::set_system_probe(system_look);
    rapidr_value::theme::set_motion_probe(reduced_motion);
}

/// Whether the user asked the system for reduced motion (a GUI test's
/// headless host says no, so captures don't depend on the machine, unless
/// `RAPIDR_REDUCED_MOTION` says it).
fn reduced_motion() -> bool {
    if super::headless_look() && !rapidr_ui_app::testhooks::real_system_look() {
        return matches!(std::env::var("RAPIDR_REDUCED_MOTION").as_deref(), Ok("1"));
    }
    rapidr_ui_host_winit::platform::reduced_motion()
}

/// A program that names no theme (or `$THEME rapidr`): RapidR's look as
/// the system is, asked before each pump — the windows drawn again only
/// when the system's look changed (the user switched to dark …). Asked at
/// most twice a second: the answer is a settings read.
fn system_theme() {
    use std::cell::Cell;
    use std::time::{Duration, Instant};
    thread_local!(static ASKED: Cell<Option<Instant>> = const { Cell::new(None) });
    if !rapidr_value::theme::wants_system() {
        return;
    }
    let now = Instant::now();
    if ASKED.with(|a| a.get().is_some_and(|t| now.duration_since(t) < Duration::from_millis(500))) {
        return;
    }
    ASKED.with(|a| a.set(Some(now)));
    let before = rapidr_value::theme::generation();
    let (dark, contrast) = system_look();
    rapidr_value::theme::system_answer(dark, contrast);
    if rapidr_value::theme::generation() != before {
        rapidr_ui_app::windows::invalidate();
    }
}

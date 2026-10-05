//! The platform on the kernel host (the platform lane, plan Stage 9): the
//! window frames BorderStyle and BorderIcons ask for, Screen.Cursor for
//! the host's pointer (`rapidr_ui_host_winit::platform`), `$THEME` and
//! `Application.Theme` (the kernel's themes, `rapidr_value::theme`).
//! Screen.Width / Height, the work area, the monitors and the mouse are the
//! host's answers (`Host::screen` …), asked by kernel.rs's facade.

use rapidr_ui_host_winit::platform::{frame_of, BI_DEFAULT};
use rapidr_ui_host_winit::{Desktop, Frame, HostCmd};

use crate::object::rp_comp_get;
use crate::value::Value;

/// Form `name`'s frame: its BorderStyle and BorderIcons (all three when
/// never set).
pub(super) fn frame(name: &str) -> Frame {
    let icons = match rp_comp_get(name, "bordericons") {
        Value::Null => BI_DEFAULT,
        v => v.to_i64(),
    };
    frame_of(rp_comp_get(name, "borderstyle").to_i64(), icons)
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
/// choose`: classic — RapidQ's, and every name the classic look answered
/// to — modern, dark, highcontrast and the older looks' names): the kernel
/// draws with that theme from now on, every form drawn again. `auto` is
/// the system's look (dark, high contrast) as the host says, or the modern
/// one; a name no theme has draws the classic look, said once.
pub(super) fn theme(name: &str) {
    use rapidr_value::theme::{self, Choice};
    let n = name.trim().to_lowercase();
    let chosen = match theme::choose(&n) {
        Choice::Theme(t) => t,
        Choice::Auto => {
            let (dark, contrast) = system_look();
            theme::auto(dark, contrast)
        }
        Choice::Unknown => {
            super::pending(&format!("$THEME {n} (it draws the classic theme)"));
            &theme::CLASSIC
        }
    };
    theme::set(chosen);
    super::invalidate_all();
}

/// How the system looks (dark, high contrast); a GUI test's headless host
/// says light, so captures don't depend on the machine.
fn system_look() -> (bool, bool) {
    if super::headless() {
        (false, false)
    } else {
        rapidr_ui_host_winit::platform::system_look()
    }
}

/// `RAPIDR_THEME=auto` (a program that names no theme): the system's look,
/// once, before anything is drawn.
fn system_theme() {
    if rapidr_value::theme::wants_system() {
        let (dark, contrast) = system_look();
        rapidr_value::theme::system_answer(dark, contrast);
        super::invalidate_all();
    }
}

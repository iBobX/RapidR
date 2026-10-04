//! The platform on the kernel host (the platform lane, plan Stage 9): the
//! window frames BorderStyle and BorderIcons ask for, Screen.Cursor for
//! the host's pointer (`rapidr_ui_host_winit::platform`), `$THEME`.
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

/// `$THEME name`: the kernel draws Windows' classic look (RapidQ's); the
/// FLTK host's other looks (fltk-theme's) have no kernel counterpart yet.
pub(super) fn theme(name: &str) {
    let n = name.trim().to_lowercase();
    if !matches!(n.as_str(), "" | "classic" | "system" | "light" | "windows" | "win95" | "win98" | "win2k") {
        super::pending(&format!("$THEME {n} (it draws the classic theme)"));
    }
}

//! Menus on the kernel host (the buttons and menus lane): the pop-up menu
//! `QPOPUPMENU.Popup(X, Y)` and AutoPopup open — the host's own context
//! menu or the kernel's. A picked item's OnClick, what comes before the menu
//! shows (OnPopup, where) and `RAPIDR_DUMP_MENUS` are
//! `rapidr_ui_app::menus`' (shared with the web).
//!
//! The kernel draws the in-window bar and its menus and the pop-up menus
//! where the host has no native ones (`rapidr_ui_kernel::components::
//! menubar` / `popupmenu`); the winit host shows macOS' menu bar and
//! macOS' / Windows' context menus (`rapidr_ui_host_winit::menu`). Either
//! way a pick comes back as `KernelEvent::MenuPick` after the pump — menu
//! tracking holds the pump; only the program's timers run inside it, in the
//! host's tracking ticks (`tracking_tick`).

use std::time::Duration;

use rapidr_ui_app::forms::form_shown;
use rapidr_ui_app::waits::{self, Wait};
use rapidr_ui_app::windows::{invalidate, push_op};
use rapidr_ui_app::WindowOp;

use super::{dispatch_pending, pump, step, with_kern, Kern, RtStore};

/// Pop-up menu `name` open at (x, y) of `form`'s window's inside
/// (`program`: the program's Popup, not an AutoPopup).
pub(super) fn open_at(form: &str, name: &str, x: i64, y: i64, program: bool) {
    super::ensure_host();
    // (timers during native menu tracking: a tracking tick's handler — the
    // system's loop holds the pump; this menu shows once it's over)
    if super::held() {
        let (form, name) = (form.to_string(), name.to_string());
        super::after_held(Box::new(move || open_at(&form, &name, x, y, false)));
        return;
    }
    // (under a test's script nobody can pick from the system's menu, which
    // would hold the pump: the kernel draws it)
    let native = with_kern(|k| k.host.native_menus() && !k.desk.ignore_user).unwrap_or(false);
    // (the interpreter's Popup is a wait it serves itself, between
    // instructions: then the menu's tracking ticks can run its handlers)
    if native && program && waits::cooperative() {
        push_op(WindowOp::Popup(form.to_string(), name.to_string(), x, y));
        waits::start(Wait::Popup);
        return;
    }
    if native {
        // (the host's context menu, inside the next pump; its pick comes
        // back as an event)
        push_op(WindowOp::Popup(form.to_string(), name.to_string(), x, y));
        pump(Some(Duration::ZERO));
        pump(Some(Duration::ZERO));
        dispatch_pending();
        return;
    }
    let opened = with_kern(|k: &mut Kern| {
        let f = k.desk.forms.get_mut(form)?;
        f.ui.sync(&RtStore);
        Some(f.ui.open_popup(name, x, y))
    })
    .flatten()
    .unwrap_or(false);
    invalidate();
    if !opened || with_kern(|k| k.host.headless()).unwrap_or(true) {
        return;
    }
    let open = || with_kern(|k| k.desk.forms.get(form).is_some_and(|f| f.ui.popup_open().is_some())).unwrap_or(false);
    while open() && form_shown(form) {
        step(None);
    }
}

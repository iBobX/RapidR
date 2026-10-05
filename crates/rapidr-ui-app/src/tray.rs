//! The system tray's clicks as the program hears them (`rapidr_value::
//! tray`): each mouse message Windows would send the icon's window, fired
//! as that form's WndProc event (`OnWndProc`, rapidr_ast::tray_calls) with
//! (hWnd, uMsg, wParam, lParam) — uMsg the icon's uCallbackMessage, wParam
//! its uID, lParam the mouse message. The hosts report clicks; a GUI test's
//! `form.__tray_N` sends message N to the form's first icon.

use rapidr_value::{tray, v_int};

use crate::Program;

/// Icon `key`'s mouse messages `mouse`, to its form's WndProc.
pub fn deliver(rt: impl Program, key: (i64, i64), mouse: &[i64]) {
    for [hwnd, msg, wparam, lparam] in tray::clicked(key, mouse) {
        let Some(form) = rapidr_value::handles::name_of(hwnd) else { continue };
        rt.fire_args(&form, "onwndproc", &[v_int(hwnd), v_int(msg), v_int(wparam), v_int(lparam)]);
    }
}

/// A test's `form.__tray_N`: message N from the first icon of form `comp`.
pub fn test_message(rt: impl Program, comp: &str, message: i64) {
    let hwnd = rapidr_value::handles::handle_of(comp);
    if let Some(icon) = tray::icons().into_iter().find(|i| i.hwnd == hwnd) {
        deliver(rt, icon.key(), &[message]);
    }
}

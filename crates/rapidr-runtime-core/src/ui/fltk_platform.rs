//! FLTK's answers for the screen, the mouse and the plain message box
//! (`Screen`, `Application.Minimize`, MSGBOX).

use fltk::prelude::{WidgetExt, WindowExt};

pub fn screen_size() -> (i64, i64) {
    let (w, h) = fltk::app::screen_size();
    (w as i64, h as i64)
}

pub fn work_area() -> (i64, i64) {
    let (_, _, w, h) = fltk::app::screen_work_area(0);
    (w as i64, h as i64)
}

pub fn mouse() -> (i64, i64) {
    let (x, y) = fltk::app::get_mouse();
    (x as i64, y as i64)
}

pub fn monitors() -> i64 {
    fltk::app::screen_count().max(1) as i64
}

/// Every shown top-level window iconized.
pub fn minimize() {
    if let Some(windows) = fltk::app::windows() {
        for mut w in windows {
            if w.shown() && w.parent().is_none() {
                w.iconize();
            }
        }
    }
}

/// (the dialogs lane's: SHOWMESSAGE's box, titled Application.Title, as the
/// kernel's and the web's — FLTK's stock message had its own look)
pub fn message_box(text: &str) {
    let title = crate::globals::get("application", "title").map(|t| t.to_string_val()).unwrap_or_default();
    crate::gui::gui_choice(&title, text, &["OK"], None, false);
}

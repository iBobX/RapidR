//! RapidQ's global objects on the desktop (rapidr_value::globals): FLTK
//! answers for the screen and the mouse, the system clipboard (arboard)
//! for the clipboard.

use rapidr_value::globals::Platform;
use rapidr_value::Value;
use std::cell::RefCell;

struct Desktop;

thread_local! {
    /// The system clipboard, kept open (on X11 the text lives as long as
    /// its owner); without one, the program's own clipboard.
    #[cfg(feature = "gui")]
    /// (`RAPIDR_TEST_CLIPBOARD`: tests leave the user's clipboard alone)
    static SYSTEM: RefCell<Option<arboard::Clipboard>> =
        RefCell::new(std::env::var_os("RAPIDR_TEST_CLIPBOARD").is_none().then(|| arboard::Clipboard::new().ok()).flatten());
    static OWN: RefCell<String> = const { RefCell::new(String::new()) };
    /// Screen.Cursor (crDefault = 0: each component's own).
    static CURSOR: std::cell::Cell<i64> = const { std::cell::Cell::new(0) };
}

impl Platform for Desktop {
    fn set_icon(&self) {
        #[cfg(feature = "gui")]
        crate::gui::gui_apply_icons();
    }

    fn screen_size(&self) -> (i64, i64) {
        #[cfg(feature = "gui")]
        {
            let (w, h) = fltk::app::screen_size();
            (w as i64, h as i64)
        }
        #[cfg(not(feature = "gui"))]
        (0, 0)
    }

    fn work_area(&self) -> (i64, i64) {
        #[cfg(feature = "gui")]
        {
            let (_, _, w, h) = fltk::app::screen_work_area(0);
            (w as i64, h as i64)
        }
        #[cfg(not(feature = "gui"))]
        (0, 0)
    }

    fn mouse(&self) -> (i64, i64) {
        #[cfg(feature = "gui")]
        {
            let (x, y) = fltk::app::get_mouse();
            (x as i64, y as i64)
        }
        #[cfg(not(feature = "gui"))]
        (0, 0)
    }

    fn monitors(&self) -> i64 {
        #[cfg(feature = "gui")]
        return fltk::app::screen_count().max(1) as i64;
        #[cfg(not(feature = "gui"))]
        1
    }

    fn clipboard_text(&self) -> String {
        #[cfg(feature = "gui")]
        if let Some(text) = SYSTEM.with(|s| s.borrow_mut().as_mut().map(|c| c.get_text().unwrap_or_default())) {
            return text;
        }
        OWN.with(|o| o.borrow().clone())
    }

    fn set_clipboard_text(&self, text: &str) {
        #[cfg(feature = "gui")]
        if SYSTEM.with(|s| {
            s.borrow_mut().as_mut().map(|c| if text.is_empty() { c.clear().is_ok() } else { c.set_text(text).is_ok() })
        }) == Some(true)
        {
            return;
        }
        OWN.with(|o| *o.borrow_mut() = text.to_string());
    }

    fn exe_path(&self) -> String {
        std::env::current_exe().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default()
    }

    fn terminate(&self) {
        crate::builtins::rp_end();
    }

    fn minimize(&self) {
        #[cfg(feature = "gui")]
        if let Some(windows) = fltk::app::windows() {
            use fltk::prelude::{WidgetExt, WindowExt};
            for mut w in windows {
                if w.shown() && w.parent().is_none() {
                    w.iconize();
                }
            }
        }
    }

    fn set_cursor(&self, cursor: i64) {
        CURSOR.with(|c| c.set(cursor));
    }
}

/// Screen.Cursor: the pointer over every form, or 0 (crDefault).
pub fn screen_cursor() -> i64 {
    CURSOR.with(std::cell::Cell::get)
}

/// Whether `name` is a global object rather than a component the program
/// made with that name.
fn is_global(name: &str) -> bool {
    rapidr_value::globals::global(name).is_some() && !matches!(crate::object::rp_comp_type(name).as_str(), t if !t.is_empty() && t != "RUDT")
}

pub fn get(name: &str, prop: &str) -> Option<Value> {
    if !is_global(name) {
        return None;
    }
    rapidr_value::globals::get(&Desktop, name, prop)
}

pub fn set(name: &str, prop: &str, value: &Value) -> bool {
    is_global(name) && rapidr_value::globals::set(&Desktop, name, prop, value)
}

pub fn call(name: &str, method: &str, args: &[Value]) -> Option<Value> {
    if !is_global(name) {
        return None;
    }
    rapidr_value::globals::call(&Desktop, name, method, args)
}

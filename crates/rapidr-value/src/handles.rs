//! `Component.Handle`: a number that stands for a component (RapidQ gives
//! the window's handle). Stable for the component's life and never 0, the
//! same scheme in every runtime; `name_of` finds the component again (a
//! QFORMMDI's `AddChild(Edit.Handle, …)`).

use std::cell::RefCell;
use std::collections::HashMap;

#[derive(Default)]
struct Handles {
    by_name: HashMap<String, i64>,
    by_handle: HashMap<i64, String>,
}

thread_local! {
    static HANDLES: RefCell<Handles> = RefCell::new(Handles::default());
}

/// Handles start here and go up in steps of 4 (as window handles do).
const FIRST: i64 = 0x0001_0004;

/// The handle of component `name` (made on first use).
pub fn handle_of(name: &str) -> i64 {
    let key = name.to_ascii_lowercase();
    HANDLES.with(|h| {
        let mut h = h.borrow_mut();
        if let Some(&n) = h.by_name.get(&key) {
            return n;
        }
        let n = FIRST + 4 * h.by_name.len() as i64;
        h.by_name.insert(key.clone(), n);
        h.by_handle.insert(n, key);
        n
    })
}

/// The window system's own handle for component `name` (a form's HWND on
/// Windows, registered by the host when the window is made): what `Handle`
/// reads from then on, and what `name_of` maps back, so Windows API calls
/// get the real window (docs/windows-dll-calls.md §3).
pub fn set_native(name: &str, handle: i64) {
    let key = name.to_ascii_lowercase();
    HANDLES.with(|h| {
        let mut h = h.borrow_mut();
        h.by_name.insert(key.clone(), handle);
        h.by_handle.insert(handle, key);
    });
}

/// The component (lowercase name) a handle stands for.
pub fn name_of(handle: i64) -> Option<String> {
    HANDLES.with(|h| h.borrow().by_handle.get(&handle).cloned())
}

thread_local! {
    static ICONS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// Icon handles start here, apart from components' (RC.EXE's
/// `Application.Icon` reads as an HICON: a number a QNOTIFYICONDATA's hIcon
/// takes).
const FIRST_ICON: i64 = 0x0B00_0004;

/// The handle `Application.Icon` reads as for icon `source` (a file, a
/// `$RESOURCE`'s handle as text; "" the application's own): stable, never 0.
pub fn icon_handle(source: &str) -> i64 {
    ICONS.with(|i| {
        let mut i = i.borrow_mut();
        let n = match i.iter().position(|s| s == source) {
            Some(n) => n,
            None => {
                i.push(source.to_string());
                i.len() - 1
            }
        };
        FIRST_ICON + 4 * n as i64
    })
}

/// The icon a handle stands for ("" the application's own).
pub fn icon_source(handle: i64) -> Option<String> {
    let n = handle.checked_sub(FIRST_ICON).filter(|d| d % 4 == 0)? / 4;
    ICONS.with(|i| i.borrow().get(usize::try_from(n).ok()?).cloned())
}

#[cfg(test)]
mod tests {
    #[test]
    fn icon_handles_are_numbers_for_icons() {
        let app = icon_handle("");
        assert_ne!(app, 0);
        assert_eq!(icon_handle(""), app);
        let f = icon_handle("app.ico");
        assert_ne!(f, app);
        assert_eq!(icon_source(f).as_deref(), Some("app.ico"));
        assert_eq!(icon_source(0), None);
        assert_eq!(icon_source(f + 1), None);
    }

    use super::*;

    #[test]
    fn handles_are_stable_and_reversible() {
        let a = handle_of("Edit(1)");
        assert_eq!(handle_of("edit(1)"), a);
        assert_ne!(handle_of("edit(2)"), a);
        assert_eq!(name_of(a).as_deref(), Some("edit(1)"));
        assert_eq!(name_of(0), None);
    }
}

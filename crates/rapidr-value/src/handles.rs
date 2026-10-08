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
    /// Own handles made so far (a form's later gets its window's instead).
    made: usize,
}

thread_local! {
    static HANDLES: RefCell<Handles> = RefCell::new(Handles::default());
}

/// The `n`th of RapidR's own handles (a component's, an icon's) in the
/// series whose low word starts at `low`: a number that is never a real
/// window, icon or cursor of the system. Windows' USER handles are an index
/// into the session's handle table (the low word) with a reuse count (the
/// high word), and that table holds at most 65,536 entries for the whole
/// session, so an index near 0xFFFF names nothing — a DLL call given a
/// control's `Handle` (which has no window of its own: RapidR draws its
/// controls) fails the Windows way instead of reaching another program's
/// window. The high word counts 1 to 0x7FFF (a positive LONG); past that the
/// low word steps down by 4.
fn own_handle(low: i64, n: usize) -> i64 {
    let (band, k) = ((n / 0x7FFF) as i64, (n % 0x7FFF) as i64);
    ((k + 1) << 16) | (low - 4 * band)
}

/// Which `n` [`own_handle`] made `h` in the series starting at `low`.
fn own_index(low: i64, h: i64) -> Option<usize> {
    let (hi, lo) = (h >> 16, h & 0xFFFF);
    if !(1..=0x7FFF).contains(&hi) || lo > low || (low - lo) % 4 != 0 || (low - lo) / 4 >= BANDS {
        return None;
    }
    usize::try_from((low - lo) / 4 * 0x7FFF + hi - 1).ok()
}

/// Components' handles: low words 0xFFFC, 0xFFF8, … (16 series).
const COMPONENTS: i64 = 0xFFFC;
/// Icons' handles: low words 0xFFBC, 0xFFB8, … (apart from components').
const ICONS_LOW: i64 = 0xFFBC;
const BANDS: i64 = 16;

/// The handle of component `name` (made on first use).
pub fn handle_of(name: &str) -> i64 {
    let key = name.to_ascii_lowercase();
    HANDLES.with(|h| {
        let mut h = h.borrow_mut();
        if let Some(&n) = h.by_name.get(&key) {
            return n;
        }
        let n = own_handle(COMPONENTS, h.made);
        h.made += 1;
        h.by_name.insert(key.clone(), n);
        h.by_handle.insert(n, key);
        n
    })
}

/// Whether `h` is one of RapidR's own component handles (not a window of
/// the system's).
pub fn is_own_handle(h: i64) -> bool {
    own_index(COMPONENTS, h).is_some()
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

// Icon handles are a series of their own (RC.EXE's `Application.Icon`
// reads as an HICON: a number a QNOTIFYICONDATA's hIcon takes) — never a
// real icon of the system either (`own_handle`).

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
        own_handle(ICONS_LOW, n)
    })
}

/// The icon a handle stands for ("" the application's own).
pub fn icon_source(handle: i64) -> Option<String> {
    let n = own_index(ICONS_LOW, handle)?;
    ICONS.with(|i| i.borrow().get(n).cloned())
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
        assert!(is_own_handle(a) && !is_own_handle(0) && !is_own_handle(0x0001_0010));
    }

    /// RapidR's own handles are never a USER handle Windows could have
    /// handed out (the table index in the low word stays near 0xFFFF), are
    /// positive LONGs, and series never meet.
    #[test]
    fn own_handles_are_no_windows() {
        for n in [0usize, 1, 2, 0x7FFE, 0x7FFF, 0x8000, 16 * 0x7FFF - 1] {
            let h = own_handle(COMPONENTS, n);
            assert!(h > 0 && h <= i32::MAX as i64, "{h:#x}");
            assert!((h & 0xFFFF) >= 0xFFC0, "{h:#x}");
            assert_eq!(own_index(COMPONENTS, h), Some(n));
            assert_eq!(own_index(ICONS_LOW, h), None);
            let i = own_handle(ICONS_LOW, n);
            assert!((i & 0xFFFF) >= 0xFF80 && (i & 0xFFFF) < 0xFFC0, "{i:#x}");
            assert_eq!(own_index(ICONS_LOW, i), Some(n));
            assert_eq!(own_index(COMPONENTS, i), None);
        }
        assert_eq!(own_handle(COMPONENTS, 0), 0x0001_FFFC);
        assert_eq!(own_index(COMPONENTS, 0x0001_0010), None);
    }
}

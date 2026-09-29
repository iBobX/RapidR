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

/// The component (lowercase name) a handle stands for.
pub fn name_of(handle: i64) -> Option<String> {
    HANDLES.with(|h| h.borrow().by_handle.get(&handle).cloned())
}

#[cfg(test)]
mod tests {
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

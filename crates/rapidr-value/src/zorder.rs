//! (RapidR's) A component's place in the stack of its siblings:
//! `BringToFront` puts it over every sibling, `SendToBack` under them. The
//! UI kernel draws a parent's children in this order and gives the mouse to
//! the topmost one where several overlap, so a panel brought to the front
//! covers the controls under it and takes their clicks; sent back, it is
//! under them. Tab (without TabOrder) and a screen reader's reading order
//! follow the same stack, as a window's controls do on Windows.
//!
//! Classic programs never call these (RapidQ's components have no
//! z-order members: there, siblings stack in the order they were made),
//! so the order of anything they make is unchanged: a component that was
//! never moved keeps its creation place, and every moved one stands above
//! (brought to front) or below (sent back) all of those. Both runtimes keep
//! a component's place in its properties ([`KEY`], a number no program
//! reads) and the kernel sorts each parent's children by it
//! ([`stacked`]); one rule for the desktop, native builds, the interpreter
//! and the web.

use std::sync::atomic::{AtomicI64, Ordering};

/// The property a component's place in the stack is kept in (its
/// runtime's store; set without a property's effects, read by the
/// kernel). Unset (0): its creation place.
pub const KEY: &str = "__zorder";

/// How many moves were made: each one's place is past all before it.
static MOVES: AtomicI64 = AtomicI64::new(0);

/// `Some(true)` for BringToFront, `Some(false)` for SendToBack (a
/// method's name in lower case), `None` for any other.
pub fn method(lower: &str) -> Option<bool> {
    match lower {
        "bringtofront" => Some(true),
        "sendtoback" => Some(false),
        _ => None,
    }
}

/// Whether a component of RapidR type `type_name` has a place among its
/// siblings: every component the kernel places — not a form, a window of
/// its own.
pub fn stacks(type_name: &str) -> bool {
    !matches!(type_name.to_ascii_uppercase().as_str(), "RFORM" | "RFORMMDI" | "RFORMEX" | "")
}

/// The place a component brought to the front (`front`) or sent to the
/// back gets: above (below) every place given before.
pub fn next(front: bool) -> i64 {
    let n = MOVES.fetch_add(1, Ordering::Relaxed) + 1;
    if front {
        n
    } else {
        -n
    }
}

/// One parent's `children` (id, type) in their creation order, put in the
/// order they stack — the first drawn first, the last on top: by their
/// places (`place_of`: a component's [`KEY`], 0 when it has none), equal
/// places in creation order. Nothing moved: as they come.
pub fn stacked(children: Vec<(String, String)>, place_of: impl Fn(&str) -> i64) -> Vec<(String, String)> {
    if MOVES.load(Ordering::Relaxed) == 0 || children.len() < 2 {
        return children;
    }
    let mut keyed: Vec<(i64, (String, String))> = children.into_iter().map(|c| (place_of(&c.0), c)).collect();
    if keyed.iter().all(|(k, _)| *k == 0) {
        return keyed.into_iter().map(|(_, c)| c).collect();
    }
    // (a stable sort: equal places keep their creation order)
    keyed.sort_by_key(|(k, _)| *k);
    keyed.into_iter().map(|(_, c)| c).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn ids(v: &[(String, String)]) -> Vec<&str> {
        v.iter().map(|(i, _)| i.as_str()).collect()
    }

    #[test]
    fn brought_to_front_last_sent_back_first_the_rest_as_made() {
        let kids: Vec<(String, String)> = ["a", "b", "c", "d"].iter().map(|s| (s.to_string(), "RPANEL".to_string())).collect();
        let mut places: HashMap<&str, i64> = HashMap::new();
        // (nothing moved: as made)
        assert_eq!(ids(&stacked(kids.clone(), |_| 0)), ["a", "b", "c", "d"]);
        places.insert("a", next(true));
        assert_eq!(ids(&stacked(kids.clone(), |id| places.get(id).copied().unwrap_or(0))), ["b", "c", "d", "a"]);
        places.insert("c", next(true));
        assert_eq!(ids(&stacked(kids.clone(), |id| places.get(id).copied().unwrap_or(0))), ["b", "d", "a", "c"]);
        // (the one brought to front, sent back: under everything)
        places.insert("c", next(false));
        assert_eq!(ids(&stacked(kids.clone(), |id| places.get(id).copied().unwrap_or(0))), ["c", "b", "d", "a"]);
        places.insert("d", next(false));
        assert_eq!(ids(&stacked(kids.clone(), |id| places.get(id).copied().unwrap_or(0))), ["d", "c", "b", "a"]);
        assert_eq!((method("bringtofront"), method("sendtoback"), method("show")), (Some(true), Some(false), None));
        assert!(stacks("RPANEL") && stacks("rbutton") && !stacks("RFORM") && !stacks("RFORMMDI"));
    }
}

//! What the panels' lists share: the metrics every panel lays its rows out
//! with (one look across the IDE), and where the keyboard moves a list's
//! focused row.

/// A row's height (logical pixels): the inspector's, the toolbox's, the
/// project tree's, the palette's commands.
pub const ROW: i64 = 22;
/// A search box's strip at a panel's top (the box with its margins).
pub const SEARCH: i64 = 32;
/// A tab strip (the inspector's Properties / Events, the console's pages).
pub const TABS: i64 = 28;
/// How far a tree's levels step in.
pub const INDENT: i64 = 16;

/// Windows' virtual keys the lists use.
pub mod vk {
    pub const BACK: i64 = 8;
    pub const TAB: i64 = 9;
    pub const ENTER: i64 = 13;
    pub const ESCAPE: i64 = 27;
    pub const SPACE: i64 = 32;
    pub const PAGE_UP: i64 = 33;
    pub const PAGE_DOWN: i64 = 34;
    pub const END: i64 = 35;
    pub const HOME: i64 = 36;
    pub const LEFT: i64 = 37;
    pub const UP: i64 = 38;
    pub const RIGHT: i64 = 39;
    pub const DOWN: i64 = 40;
    pub const DELETE: i64 = 46;
    pub const F2: i64 = 113;
    pub const F4: i64 = 115;
    pub const APPS: i64 = 93;
}

/// Where a key moves a list's focus among `count` rows, from `at`, with
/// `page` rows to a page: Up / Down, Page Up / Page Down, Home / End.
/// `None`: not a key that moves it (or no rows).
pub fn nav(key: i64, at: Option<usize>, count: usize, page: usize) -> Option<usize> {
    if count == 0 {
        return None;
    }
    let last = count - 1;
    let page = page.max(1);
    Some(match (key, at) {
        (vk::UP, None) | (vk::DOWN, None) | (vk::HOME, _) => 0,
        (vk::END, _) => last,
        (vk::UP, Some(i)) => i.saturating_sub(1),
        (vk::DOWN, Some(i)) => (i + 1).min(last),
        (vk::PAGE_UP, i) => i.unwrap_or(0).saturating_sub(page),
        (vk::PAGE_DOWN, i) => (i.unwrap_or(0) + page).min(last),
        _ => return None,
    })
}

/// The first row from `from` (wrapping) whose label starts with `typed`
/// (any case): typing a name's first letters in a list goes to it.
pub fn type_ahead<'a>(labels: impl IntoIterator<Item = &'a str>, from: usize, typed: &str) -> Option<usize> {
    let labels: Vec<&str> = labels.into_iter().collect();
    let n = labels.len();
    let t = typed.to_lowercase();
    (0..n).map(|k| (from + k) % n).find(|&i| labels[i].to_lowercase().starts_with(&t))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_move_the_focus_within_the_rows() {
        assert_eq!(nav(vk::DOWN, None, 5, 3), Some(0));
        assert_eq!(nav(vk::DOWN, Some(4), 5, 3), Some(4));
        assert_eq!(nav(vk::UP, Some(0), 5, 3), Some(0));
        assert_eq!(nav(vk::PAGE_DOWN, Some(1), 5, 3), Some(4));
        assert_eq!(nav(vk::END, Some(1), 5, 3), Some(4));
        assert_eq!(nav(vk::HOME, Some(3), 5, 3), Some(0));
        assert_eq!(nav(vk::LEFT, Some(3), 5, 3), None);
        assert_eq!(nav(vk::DOWN, None, 0, 3), None);
    }

    #[test]
    fn typing_goes_to_a_name() {
        let l = ["Caption", "Color", "Cursor", "Enabled"];
        assert_eq!(type_ahead(l, 0, "cu"), Some(2));
        assert_eq!(type_ahead(l, 3, "c"), Some(0));
        assert_eq!(type_ahead(l, 0, "x"), None);
    }
}

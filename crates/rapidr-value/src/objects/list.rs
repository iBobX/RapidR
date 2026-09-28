//! QLISTBOX's and QCOMBOBOX's items (RapidQ manual, Appendix A): the
//! strings, the selected item (and, for a MultiSelect list box, which items
//! are selected), Sorted, TopIndex and a combo box's edit text. The runtimes
//! draw it (`with_list`); the program changes it through the properties and
//! methods below, the same on the desktop and the web.
//!
//! * `AddItems a, b, …`, `InsertItem i, s`, `DelItems i, …`, `Clear`,
//!   `Item(i)` read / `Item(i) = s`, `ItemCount`, `ItemIndex`, `Sorted`,
//!   `MultiSelect`, `Selected(i)` read / write, `SelCount`, `TopIndex`,
//!   `Text` (a combo box's edit text; a list box's items as CRLF-terminated
//!   lines), `LoadFromFile` / `SaveToFile` (one item per line, through
//!   [`ItemList::to_text`] / [`ItemList::load_text`]);
//! * RapidR's older names: `AddItem`, `DeleteItem` / `RemoveItem`,
//!   `Items(i)`, `Count` / `ListCount`, `ListIndex`, and `Items` read /
//!   written as newline-separated text.
//!
//! Indexes out of range read as "" / 0 and are ignored when written, as
//! RapidQ doesn't stop the program for them. A list holds at most
//! [`MAX_ITEMS`] items.

use crate::{v_int, v_str, Value};

pub const MAX_ITEMS: usize = 1_000_000;

#[derive(Clone, Debug, Default)]
pub struct ItemList {
    pub items: Vec<String>,
    /// For a MultiSelect list box, one flag per item.
    pub selected: Vec<bool>,
    pub item_index: i64,
    pub sorted: bool,
    pub multi_select: bool,
    pub top_index: i64,
    /// QCOMBOBOX: true. Its Text is its edit text.
    pub combo: bool,
    pub text: String,
}

fn index(v: Option<&Value>) -> Option<usize> {
    usize::try_from(v?.to_i64()).ok()
}

fn flag(on: bool) -> Value {
    v_int(if on { -1 } else { 0 })
}

impl ItemList {
    pub fn new(combo: bool) -> ItemList {
        ItemList { item_index: -1, combo, ..Default::default() }
    }

    pub fn is_selected(&self, i: usize) -> bool {
        if self.multi_select {
            self.selected.get(i).copied().unwrap_or(false)
        } else {
            self.item_index == i as i64
        }
    }

    /// Selects item `i` alone (-1: nothing); a combo box shows it as its
    /// text.
    pub fn select(&mut self, i: i64) {
        let i = if (0..self.items.len() as i64).contains(&i) { i } else { -1 };
        self.item_index = i;
        self.selected.iter_mut().for_each(|s| *s = false);
        if i >= 0 {
            if let Some(s) = self.selected.get_mut(i as usize) {
                *s = true;
            }
            if self.combo {
                self.text = self.items[i as usize].clone();
            }
        }
    }

    /// The user changed a MultiSelect list box's selection: the items
    /// selected now, and the one clicked (ItemIndex, as Delphi's focused
    /// item).
    pub fn set_selection(&mut self, focus: i64, selected: &[bool]) {
        for (i, s) in self.selected.iter_mut().enumerate() {
            *s = selected.get(i).copied().unwrap_or(false);
        }
        self.item_index = if (0..self.items.len() as i64).contains(&focus) { focus } else { -1 };
    }

    /// Adds `s` (in order when Sorted); returns where it went.
    fn add(&mut self, s: String) -> usize {
        if self.items.len() >= MAX_ITEMS {
            return self.items.len();
        }
        let at = if self.sorted { self.items.partition_point(|x| x.to_lowercase() <= s.to_lowercase()) } else { self.items.len() };
        self.insert(at, s);
        at
    }

    fn insert(&mut self, at: usize, s: String) {
        if self.items.len() >= MAX_ITEMS {
            return;
        }
        let at = at.min(self.items.len());
        self.items.insert(at, s);
        self.selected.insert(at, false);
        if self.item_index >= at as i64 {
            self.item_index += 1;
        }
    }

    fn remove(&mut self, at: usize) {
        if at >= self.items.len() {
            return;
        }
        self.items.remove(at);
        self.selected.remove(at);
        match self.item_index.cmp(&(at as i64)) {
            std::cmp::Ordering::Equal => self.item_index = -1,
            std::cmp::Ordering::Greater => self.item_index -= 1,
            std::cmp::Ordering::Less => {}
        }
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.selected.clear();
        self.item_index = -1;
        self.top_index = 0;
    }

    fn sort(&mut self) {
        let current = usize::try_from(self.item_index).ok().and_then(|i| self.items.get(i).cloned());
        let mut paired: Vec<(String, bool)> = self.items.drain(..).zip(self.selected.drain(..)).collect();
        paired.sort_by_key(|(s, _)| s.to_lowercase());
        (self.items, self.selected) = paired.into_iter().unzip();
        self.item_index = current.and_then(|c| self.items.iter().position(|s| *s == c)).map_or(-1, |i| i as i64);
    }

    /// Replaces the items with the lines of `text`.
    pub fn load_text(&mut self, text: &str) {
        self.clear();
        for line in text.lines() {
            self.add(line.to_string());
        }
    }

    /// The items as CRLF-terminated lines.
    pub fn to_text(&self) -> String {
        self.items.iter().map(|s| format!("{s}\r\n")).collect()
    }

    pub fn get(&self, prop: &str) -> Option<Value> {
        Some(match prop {
            "itemcount" | "count" | "listcount" => v_int(self.items.len() as i64),
            "itemindex" | "listindex" => v_int(self.item_index),
            "selcount" => v_int((0..self.items.len()).filter(|&i| self.is_selected(i)).count() as i64),
            "sorted" => flag(self.sorted),
            "multiselect" => flag(self.multi_select),
            "topindex" => v_int(self.top_index),
            "text" if self.combo => v_str(&self.text),
            "text" => v_str(&self.to_text()),
            "items" => v_str(&self.items.join("\n")),
            _ => return None,
        })
    }

    pub fn set(&mut self, prop: &str, val: &Value) -> bool {
        match prop {
            "itemindex" | "listindex" => self.select(val.to_i64()),
            "sorted" => {
                self.sorted = val.to_bool();
                if self.sorted {
                    self.sort();
                }
            }
            "multiselect" => {
                self.multi_select = val.to_bool();
                if !self.multi_select {
                    let i = self.item_index;
                    self.select(i);
                }
            }
            "topindex" => self.top_index = val.to_i64().clamp(0, self.items.len().saturating_sub(1) as i64),
            "text" if self.combo => {
                self.text = val.to_string_val();
                self.item_index = self.items.iter().position(|s| *s == self.text).map_or(-1, |i| i as i64);
            }
            "text" | "items" => self.load_text(&val.to_string_val()),
            _ => return false,
        }
        true
    }

    pub fn call(&mut self, method: &str, args: &[Value]) -> Option<Value> {
        let text = |i: usize| args.get(i).map(|v| v.to_string_val()).unwrap_or_default();
        match method {
            "additems" | "additem" | "addstring" => {
                for a in args {
                    self.add(a.to_string_val());
                }
            }
            "insertitem" => {
                let at = index(args.first()).unwrap_or(usize::MAX);
                self.insert(at, text(1));
            }
            "delitems" | "deleteitem" | "deleteitems" | "removeitem" => {
                // Indexes as they were before any was deleted.
                let mut gone: Vec<usize> = args.iter().filter_map(|a| index(Some(a))).collect();
                gone.sort_unstable();
                gone.dedup();
                for &i in gone.iter().rev() {
                    self.remove(i);
                }
            }
            "clear" => self.clear(),
            "item" | "items" => {
                let i = index(args.first());
                if args.len() >= 2 {
                    if let Some(s) = i.and_then(|i| self.items.get_mut(i)) {
                        *s = text(1);
                    }
                } else {
                    return Some(v_str(i.and_then(|i| self.items.get(i)).map_or("", |s| s.as_str())));
                }
            }
            "selected" => {
                let Some(i) = index(args.first()).filter(|&i| i < self.items.len()) else {
                    return Some(if args.len() >= 2 { Value::Null } else { v_int(0) });
                };
                if args.len() >= 2 {
                    let on = args[1].to_bool();
                    if self.multi_select {
                        // ItemIndex (the focused item) stays, as in Delphi.
                        self.selected[i] = on;
                    } else if on {
                        self.select(i as i64);
                    } else if self.item_index == i as i64 {
                        self.select(-1);
                    }
                } else {
                    return Some(flag(self.is_selected(i)));
                }
            }
            "sort" => self.sort(),
            // RapidR: Find(s) — the index of the first item equal to s, or -1.
            "find" | "indexof" => {
                let s = text(0);
                return Some(v_int(self.items.iter().position(|x| *x == s).map_or(-1, |i| i as i64)));
            }
            "repaint" | "refresh" => {}
            _ => return None,
        }
        Some(Value::Null)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(x: &str) -> Value {
        v_str(x)
    }

    #[test]
    fn user_multi_selection() {
        let mut l = ItemList::new(false);
        l.call("additems", &[s("a"), s("b"), s("c")]);
        l.set("multiselect", &v_int(1));
        l.set_selection(2, &[true, false, true]);
        assert_eq!(l.get("selcount").unwrap().to_i64(), 2);
        assert_eq!(l.item_index, 2);
        assert!(l.is_selected(0) && !l.is_selected(1));
        // Fewer flags than items: the rest aren't selected.
        l.set_selection(0, &[true]);
        assert_eq!(l.get("selcount").unwrap().to_i64(), 1);
    }

    #[test]
    fn add_items_keeps_every_item() {
        let mut l = ItemList::new(false);
        l.call("additems", &[s("one"), s("two"), s("three")]);
        l.call("additems", &[s("four")]);
        assert_eq!(l.get("itemcount").unwrap().to_i64(), 4);
        assert_eq!(l.call("item", &[v_int(0)]).unwrap().to_string_val(), "one");
        l.call("item", &[v_int(1), s("TWO")]);
        assert_eq!(l.items, ["one", "TWO", "three", "four"]);
        assert_eq!(l.get("text").unwrap().to_string_val(), "one\r\nTWO\r\nthree\r\nfour\r\n");
    }

    #[test]
    fn index_follows_inserts_and_deletes() {
        let mut l = ItemList::new(false);
        l.call("additems", &[s("a"), s("b"), s("c"), s("d")]);
        l.set("itemindex", &v_int(2));
        l.call("insertitem", &[v_int(0), s("first")]);
        assert_eq!(l.get("itemindex").unwrap().to_i64(), 3);
        l.call("delitems", &[v_int(0), v_int(1)]);
        assert_eq!(l.items, ["b", "c", "d"]);
        assert_eq!(l.get("itemindex").unwrap().to_i64(), 1);
        l.call("delitems", &[v_int(1)]);
        assert_eq!(l.get("itemindex").unwrap().to_i64(), -1);
        l.set("itemindex", &v_int(99));
        assert_eq!(l.get("itemindex").unwrap().to_i64(), -1);
    }

    #[test]
    fn sorted_and_multi_select() {
        let mut l = ItemList::new(false);
        l.call("additems", &[s("pear"), s("Apple"), s("fig")]);
        l.set("itemindex", &v_int(0));
        l.set("sorted", &v_int(-1));
        assert_eq!(l.items, ["Apple", "fig", "pear"]);
        assert_eq!(l.get("itemindex").unwrap().to_i64(), 2, "the selected item stays selected");
        l.call("additems", &[s("banana")]);
        assert_eq!(l.items, ["Apple", "banana", "fig", "pear"]);
        l.set("multiselect", &v_int(-1));
        assert_eq!(l.get("selcount").unwrap().to_i64(), 1, "pear stays selected");
        l.call("selected", &[v_int(0), v_int(-1)]);
        l.call("selected", &[v_int(2), v_int(-1)]);
        assert_eq!(l.get("selcount").unwrap().to_i64(), 3);
        assert_eq!(l.call("selected", &[v_int(2)]).unwrap().to_i64(), -1);
        assert_eq!(l.call("selected", &[v_int(1)]).unwrap().to_i64(), 0);
    }

    #[test]
    fn combo_text() {
        let mut c = ItemList::new(true);
        c.call("additems", &[s("red"), s("green")]);
        c.set("itemindex", &v_int(1));
        assert_eq!(c.get("text").unwrap().to_string_val(), "green");
        c.set("text", &s("red"));
        assert_eq!(c.get("itemindex").unwrap().to_i64(), 0);
        c.set("text", &s("blue"));
        assert_eq!(c.get("itemindex").unwrap().to_i64(), -1);
        assert_eq!(c.get("text").unwrap().to_string_val(), "blue");
    }

    #[test]
    fn legacy_names_and_out_of_range() {
        let mut l = ItemList::new(false);
        l.set("items", &s("x\ny"));
        assert_eq!(l.get("count").unwrap().to_i64(), 2);
        l.call("additem", &[s("z")]);
        l.call("removeitem", &[v_int(0)]);
        assert_eq!(l.get("items").unwrap().to_string_val(), "y\nz");
        assert_eq!(l.call("item", &[v_int(-1)]).unwrap().to_string_val(), "");
        assert!(l.call("item", &[v_int(9), s("q")]).is_some());
        assert_eq!(l.call("find", &[s("z")]).unwrap().to_i64(), 1);
        assert!(l.call("bogus", &[]).is_none());
    }
}

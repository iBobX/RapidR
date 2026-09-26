//! QLISTVIEW's data (RapidQ manual, Appendix A): columns, items with their
//! sub-items, and the selection. The runtimes draw it (`with_listview`); the
//! program changes it through the methods and indexed sub-objects below,
//! the same on the desktop and the web.
//!
//! * `AddColumns "Name", "Size"`, `ClearColumns`, `Column(i).Caption` /
//!   `.Width`, `ColumnsCount`;
//! * `AddItems "a", "b"`, `InsertItem i, s`, `DelItems i, …`, `Clear`,
//!   `SwapItem i, j`, `Item(i).Caption` / `.Checked` / `.Selected` /
//!   `.ImageIndex` / `.StateIndex` / `.Index`, `ItemCount`, `ItemIndex`,
//!   `SelCount`;
//! * `AddSubItem i, s`, `InsertSubItem i, j, s`, `DelSubItem i, j`,
//!   `SubItem(i, j)` (sub-item `j` of item `i`, 0 = the first one after the
//!   caption).
//!
//! Indexes out of range read as "" / 0 and are ignored when written, as
//! RapidQ does not stop the program for them.

use crate::{v_int, v_str, Value};

/// The width a column gets unless the program sets one (Windows' default).
pub const DEFAULT_COLUMN_WIDTH: i64 = 50;

#[derive(Clone, Debug, Default)]
pub struct Column {
    pub caption: String,
    pub width: i64,
}

#[derive(Clone, Debug, Default)]
pub struct Item {
    pub caption: String,
    pub sub_items: Vec<String>,
    pub checked: bool,
    pub selected: bool,
    pub image_index: i64,
    pub state_index: i64,
}

#[derive(Clone, Debug)]
pub struct ListView {
    pub columns: Vec<Column>,
    pub items: Vec<Item>,
    pub item_index: i64,
}

impl Default for ListView {
    fn default() -> Self {
        Self { columns: Vec::new(), items: Vec::new(), item_index: -1 }
    }
}

fn index(v: Option<&Value>) -> Option<usize> {
    let i = v?.to_i64();
    usize::try_from(i).ok()
}

impl ListView {
    fn item(&self, i: Option<usize>) -> Option<&Item> {
        self.items.get(i?)
    }

    fn item_mut(&mut self, i: Option<usize>) -> Option<&mut Item> {
        self.items.get_mut(i?)
    }

    /// Selects item `i` alone (-1: nothing).
    pub fn select(&mut self, i: i64) {
        let i = if i >= 0 && (i as usize) < self.items.len() { i } else { -1 };
        self.item_index = i;
        for (n, item) in self.items.iter_mut().enumerate() {
            item.selected = n as i64 == i;
        }
    }

    /// Keeps ItemIndex on an existing item after items were removed.
    fn fix_index(&mut self) {
        if self.item_index >= self.items.len() as i64 {
            self.item_index = -1;
        }
    }

    pub fn get(&self, prop: &str) -> Option<Value> {
        Some(match prop {
            "itemcount" | "count" => v_int(self.items.len() as i64),
            "columnscount" | "columncount" => v_int(self.columns.len() as i64),
            "itemindex" => v_int(self.item_index),
            "selcount" => v_int(self.items.iter().filter(|i| i.selected).count() as i64),
            _ => return None,
        })
    }

    pub fn set(&mut self, prop: &str, val: &Value) -> bool {
        match prop {
            "itemindex" => self.select(val.to_i64()),
            _ => return false,
        }
        true
    }

    pub fn call(&mut self, method: &str, args: &[Value]) -> Option<Value> {
        let text = |i: usize| args.get(i).map(|v| v.to_string_val()).unwrap_or_default();
        match method {
            "addcolumns" => {
                self.columns.extend(args.iter().map(|a| Column { caption: a.to_string_val(), width: DEFAULT_COLUMN_WIDTH }));
            }
            "clearcolumns" => self.columns.clear(),
            "additems" => {
                self.items.extend(args.iter().map(|a| Item { caption: a.to_string_val(), ..Default::default() }));
            }
            "insertitem" => {
                let at = index(args.first()).unwrap_or(usize::MAX).min(self.items.len());
                self.items.insert(at, Item { caption: text(1), ..Default::default() });
                if self.item_index >= at as i64 {
                    self.item_index += 1;
                }
            }
            "delitems" => {
                let mut gone: Vec<usize> = args.iter().filter_map(|a| index(Some(a))).filter(|&i| i < self.items.len()).collect();
                gone.sort_unstable();
                gone.dedup();
                for &i in gone.iter().rev() {
                    self.items.remove(i);
                }
                self.fix_index();
            }
            "clear" => {
                self.items.clear();
                self.item_index = -1;
            }
            "swapitem" => {
                if let (Some(a), Some(b)) = (index(args.first()), index(args.get(1))) {
                    if a < self.items.len() && b < self.items.len() {
                        self.items.swap(a, b);
                    }
                }
            }
            "addsubitem" => {
                let s = text(1);
                if let Some(item) = self.item_mut(index(args.first())) {
                    item.sub_items.push(s);
                }
            }
            "insertsubitem" => {
                let s = text(2);
                let at = index(args.get(1)).unwrap_or(usize::MAX);
                if let Some(item) = self.item_mut(index(args.first())) {
                    let at = at.min(item.sub_items.len());
                    item.sub_items.insert(at, s);
                }
            }
            "delsubitem" => {
                let at = index(args.get(1));
                if let (Some(item), Some(at)) = (self.item_mut(index(args.first())), at) {
                    if at < item.sub_items.len() {
                        item.sub_items.remove(at);
                    }
                }
            }
            "subitem" => {
                let at = index(args.get(1));
                let item = self.item(index(args.first()));
                return Some(v_str(item.zip(at).and_then(|(item, at)| item.sub_items.get(at)).map_or("", |s| s.as_str())));
            }
            "subitem=" => {
                let s = text(2);
                let at = index(args.get(1));
                if let (Some(item), Some(at)) = (self.item_mut(index(args.first())), at) {
                    // Writing past the end fills the gap, like setting a cell.
                    if at < 256 {
                        if item.sub_items.len() <= at {
                            item.sub_items.resize(at + 1, String::new());
                        }
                        item.sub_items[at] = s;
                    }
                }
            }
            "selected" => return Some(v_int(self.item(index(args.first())).map_or(0, |i| if i.selected { -1 } else { 0 }))),
            "selected=" => {
                let on = args.get(1).is_some_and(|v| v.to_bool());
                if let Some(item) = self.item_mut(index(args.first())) {
                    item.selected = on;
                }
            }
            _ => return self.indexed(method, args),
        }
        Some(Value::Null)
    }

    /// `Item(i).Caption`, `Column(i).Width = w`, … (compiled as the methods
    /// `item.caption` / `column.width=` with the index first).
    fn indexed(&mut self, method: &str, args: &[Value]) -> Option<Value> {
        let (sub, member) = method.split_once('.')?;
        let setter = member.ends_with('=');
        let member = member.trim_end_matches('=');
        let i = index(args.first());
        let val = args.get(1).cloned().unwrap_or(Value::Null);
        match sub {
            "item" => {
                if setter {
                    if member == "selected" {
                        let on = val.to_bool();
                        if let Some(i) = i.filter(|&i| i < self.items.len()) {
                            if on {
                                self.select(i as i64);
                            } else {
                                self.items[i].selected = false;
                                if self.item_index == i as i64 {
                                    self.item_index = -1;
                                }
                            }
                        }
                        return Some(Value::Null);
                    }
                    let item = self.item_mut(i);
                    match (member, item) {
                        ("caption", Some(item)) => item.caption = val.to_string_val(),
                        ("checked", Some(item)) => item.checked = val.to_bool(),
                        ("imageindex", Some(item)) => item.image_index = val.to_i64(),
                        ("stateindex", Some(item)) => item.state_index = val.to_i64(),
                        ("caption" | "checked" | "imageindex" | "stateindex", None) => {}
                        _ => return None,
                    }
                    return Some(Value::Null);
                }
                let item = self.item(i);
                Some(match member {
                    "caption" => v_str(item.map_or("", |i| i.caption.as_str())),
                    "checked" => v_int(item.map_or(0, |i| if i.checked { -1 } else { 0 })),
                    "selected" => v_int(item.map_or(0, |i| if i.selected { -1 } else { 0 })),
                    "imageindex" => v_int(item.map_or(0, |i| i.image_index)),
                    "stateindex" => v_int(item.map_or(0, |i| i.state_index)),
                    "index" => v_int(if item.is_some() { i.unwrap_or(0) as i64 } else { -1 }),
                    _ => return None,
                })
            }
            "column" => {
                if setter {
                    let column = i.and_then(|i| self.columns.get_mut(i));
                    match (member, column) {
                        ("caption", Some(c)) => c.caption = val.to_string_val(),
                        ("width", Some(c)) => c.width = val.to_i64().clamp(0, 10_000),
                        ("caption" | "width", None) => {}
                        _ => return None,
                    }
                    return Some(Value::Null);
                }
                let column = i.and_then(|i| self.columns.get(i));
                Some(match member {
                    "caption" => v_str(column.map_or("", |c| c.caption.as_str())),
                    "width" => v_int(column.map_or(0, |c| c.width)),
                    _ => return None,
                })
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(x: &str) -> Value {
        v_str(x)
    }

    #[test]
    fn columns_items_and_sub_items() {
        let mut lv = ListView::default();
        lv.call("addcolumns", &[s("Name"), s("Size")]);
        lv.call("column.width=", &[v_int(0), v_int(120)]);
        lv.call("additems", &[s("a.txt"), s("b.txt")]);
        lv.call("addsubitem", &[v_int(0), s("10")]);
        lv.call("addsubitem", &[v_int(1), s("20")]);
        lv.call("insertitem", &[v_int(0), s("first")]);
        assert_eq!(lv.get("itemcount").unwrap().to_i64(), 3);
        assert_eq!(lv.get("columnscount").unwrap().to_i64(), 2);
        assert_eq!(lv.call("item.caption", &[v_int(1)]).unwrap().to_string_val(), "a.txt");
        assert_eq!(lv.call("subitem", &[v_int(2), v_int(0)]).unwrap().to_string_val(), "20");
        assert_eq!(lv.call("column.width", &[v_int(0)]).unwrap().to_i64(), 120);
        assert_eq!(lv.call("column.width", &[v_int(1)]).unwrap().to_i64(), DEFAULT_COLUMN_WIDTH);
        lv.call("swapitem", &[v_int(1), v_int(2)]);
        assert_eq!(lv.call("item.caption", &[v_int(1)]).unwrap().to_string_val(), "b.txt");
        lv.call("delitems", &[v_int(0), v_int(0), v_int(99)]);
        assert_eq!(lv.get("itemcount").unwrap().to_i64(), 2);
    }

    #[test]
    fn selection_follows_item_index() {
        let mut lv = ListView::default();
        lv.call("additems", &[s("a"), s("b"), s("c")]);
        lv.set("itemindex", &v_int(1));
        assert_eq!(lv.call("item.selected", &[v_int(1)]).unwrap().to_i64(), -1);
        assert_eq!(lv.get("selcount").unwrap().to_i64(), 1);
        lv.call("delitems", &[v_int(2), v_int(1)]);
        assert_eq!(lv.get("itemindex").unwrap().to_i64(), -1);
        lv.set("itemindex", &v_int(7));
        assert_eq!(lv.get("itemindex").unwrap().to_i64(), -1);
    }

    #[test]
    fn out_of_range_is_harmless() {
        let mut lv = ListView::default();
        assert_eq!(lv.call("item.caption", &[v_int(-5)]).unwrap().to_string_val(), "");
        assert!(lv.call("item.caption=", &[v_int(3), s("x")]).is_some());
        assert!(lv.call("addsubitem", &[v_int(0), s("x")]).is_some());
        assert!(lv.call("subitem=", &[v_int(0), v_int(1_000_000), s("x")]).is_some());
        assert_eq!(lv.call("subitem", &[v_int(0), v_int(-1)]).unwrap().to_string_val(), "");
        assert!(lv.call("item.bogus", &[v_int(0)]).is_none());
    }
}

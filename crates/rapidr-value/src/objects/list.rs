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
//! Owner drawing (QLISTBOX `Style = lbOwnerDrawFixed` / `lbOwnerDrawVariable`):
//! the runtimes fire `OnDrawItem(Index, State, Rect)` for each item after the
//! list changed ([`ItemList::owner_draw_needed`], [`ItemList::owner_draw_items`]);
//! `FillRect`, `TextOut`, `Draw`, … on the list are kept per item
//! ([`CellDraw`]) and painted ([`ItemList::render_item`]) into a bitmap the
//! runtimes show, so every platform draws the same pixels. Every item is
//! `ItemHeight` tall, except with `lbOwnerDrawVariable`: the runtimes then
//! first fire `OnMeasureItem(Index, Height)` for each item after the items
//! changed ([`ItemList::measure_needed`]), and the `Height` each handler
//! answers ([`ItemList::measured`]) is that item's; OnDrawItem waits until
//! all have answered. `State` is 0 for the selected item and 1 for the
//! others, as RapidQ programs test it.
//!
//! Indexes out of range read as "" / 0 and are ignored when written, as
//! RapidQ doesn't stop the program for them. A list holds at most
//! [`MAX_ITEMS`] items.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use super::bitmap::Bitmap;
use super::filelist::{FileSource, FT_NORMAL};
use super::font::Font;
use super::grid::CellDraw;
use crate::{v_int, v_str, Value};

pub const MAX_ITEMS: usize = 1_000_000;

/// `Style` values (RAPIDQ.INC's `lbStandard`, `lbOwnerDrawFixed`, `lbOwnerDrawVariable`).
pub const LB_OWNER_FIXED: i64 = 1;
pub const LB_OWNER_VARIABLE: i64 = 2;
/// QCOMBOBOX's owner-draw `Style`s (`csOwnerDrawFixed`, `csOwnerDrawVariable`).
pub const CS_OWNER_FIXED: i64 = 3;
pub const CS_OWNER_VARIABLE: i64 = 4;

/// Most items OnDrawItem is fired for after a change, and the height of an
/// item without an `ItemHeight`.
pub const MAX_OWNER_DRAWN: usize = 5_000;
pub const DEFAULT_ITEM_HEIGHT: i64 = 16;

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
    /// QFILELISTBOX: the directory its items come from.
    pub files: Option<FileSource>,
    /// A QSTRINGLIST (no widget): the same items, with RapidQ's own method
    /// names (`Add`, `Insert`, `Delete`, `Get`, `Strings`).
    pub plain: bool,
    /// `Style` and `ItemHeight` (owner drawing, see the module docs).
    pub style: i64,
    pub item_height: i64,
    /// A QLISTBOX's `Columns`: items flow down each column, then into the
    /// next; that many columns show (0: one column).
    pub columns: i64,
    /// The size the runtime shows the items in (for multi-column layout
    /// and for finding the item a drawing lands on).
    view: (i64, i64),
    /// What OnDrawItem drew, per item, relative to the item's top left.
    pub owner_drawing: HashMap<usize, Vec<CellDraw>>,
    drawn_state: Option<u64>,
    /// lbOwnerDrawVariable: each item's height as OnMeasureItem answered
    /// (0: `ItemHeight`); the items it was asked for, and the answers
    /// still to come for the current round (`measure_round`).
    item_heights: Vec<i64>,
    measured_state: Option<u64>,
    measure_round: u32,
    measures_pending: usize,
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

    /// A QSTRINGLIST.
    pub fn new_string_list() -> ItemList {
        ItemList { item_index: -1, plain: true, ..Default::default() }
    }

    /// A QFILELISTBOX: the files of the current directory.
    pub fn new_file_list() -> ItemList {
        let mut l = ItemList { item_index: -1, files: Some(FileSource::default()), ..Default::default() };
        l.reload_files();
        l
    }

    /// A QFILELISTBOX's items read again from its directory.
    pub fn reload_files(&mut self) {
        let Some(src) = &self.files else { return };
        let items = src.list();
        self.selected = vec![false; items.len()];
        self.items = items;
        self.item_index = -1;
        self.top_index = 0;
    }

    /// QFILELISTBOX properties and methods (`None`: not one of them).
    fn file_member(&mut self, name: &str, args: &[Value]) -> Option<Value> {
        let src = self.files.as_mut()?;
        let arg = |i: usize| args.get(i).map(|v| v.to_string_val()).unwrap_or_default();
        match name {
            "directory" if args.is_empty() => return Some(v_str(&src.directory)),
            "directory" => src.directory = arg(0),
            "mask" if args.is_empty() => return Some(v_str(&src.mask)),
            "mask" => src.mask = arg(0),
            // RapidR has no drive letters off Windows: the directory's own.
            "drive" if args.is_empty() => {
                let d = src.directory.chars().next().filter(|_| src.directory.get(1..2) == Some(":"));
                return Some(v_str(&d.map(String::from).unwrap_or_default()));
            }
            "filename" if args.is_empty() => {
                let item = usize::try_from(self.item_index).ok().and_then(|i| self.items.get(i));
                let path = item.map(|i| src.path_of(i)).unwrap_or_default();
                return Some(v_str(&path));
            }
            "addfiletypes" | "delfiletypes" => {
                for t in args.iter().map(|a| a.to_i64()).filter(|t| (0..=FT_NORMAL as i64).contains(t)) {
                    if name == "addfiletypes" {
                        src.types |= 1 << t;
                    } else {
                        src.types &= !(1 << t);
                    }
                }
            }
            "update" => {}
            _ => return None,
        }
        self.reload_files();
        Some(Value::Null)
    }

    /// Whether the items are drawn by OnDrawItem: a list box's `Style`
    /// lbOwnerDrawFixed / lbOwnerDrawVariable, a combo box's
    /// csOwnerDrawFixed / csOwnerDrawVariable.
    pub fn owner_drawn(&self) -> bool {
        if self.combo {
            matches!(self.style, CS_OWNER_FIXED | CS_OWNER_VARIABLE)
        } else {
            !self.plain && matches!(self.style, LB_OWNER_FIXED | LB_OWNER_VARIABLE)
        }
    }

    /// A multi-column list box (`Columns`).
    pub fn multi_column(&self) -> bool {
        !self.combo && !self.plain && self.columns > 0
    }

    /// Whether the runtime shows the items as pictures from this model
    /// (owner-drawn, or in columns) rather than as a plain list.
    pub fn custom_drawn(&self) -> bool {
        self.owner_drawn() || self.multi_column()
    }

    /// Height of an item, in pixels.
    pub fn row_height(&self) -> i64 {
        if self.item_height > 0 { self.item_height.min(2_000) } else { DEFAULT_ITEM_HEIGHT }
    }

    /// Whether items have their own heights (`lbOwnerDrawVariable`,
    /// `csOwnerDrawVariable`; not in columns).
    pub fn variable(&self) -> bool {
        !self.multi_column() && self.style == if self.combo { CS_OWNER_VARIABLE } else { LB_OWNER_VARIABLE }
    }

    /// Height of item `i`: what OnMeasureItem answered for it, else
    /// `ItemHeight`.
    pub fn item_h(&self, i: usize) -> i64 {
        match self.item_heights.get(i) {
            Some(&h) if h > 0 && self.variable() => h,
            _ => self.row_height(),
        }
    }

    /// Top of each item in the list's content, and the bottom of the last.
    fn tops(&self) -> Vec<i64> {
        let mut tops = Vec::with_capacity(self.items.len() + 1);
        let mut y = 0;
        tops.push(0);
        for i in 0..self.items.len() {
            y += self.item_h(i);
            tops.push(y);
        }
        tops
    }

    /// OnMeasureItem: when the items (or Style, ItemHeight) of a
    /// `lbOwnerDrawVariable` list changed, (round, index, Height) for each
    /// item to ask about, Height starting as `ItemHeight`; otherwise none.
    pub fn measure_needed(&mut self) -> Vec<(u32, usize, i64)> {
        if !self.variable() {
            return Vec::new();
        }
        let mut h = std::collections::hash_map::DefaultHasher::new();
        (&self.items, self.style, self.item_height).hash(&mut h);
        let state = h.finish();
        if self.measured_state == Some(state) {
            return Vec::new();
        }
        self.measured_state = Some(state);
        self.measure_round = self.measure_round.wrapping_add(1);
        let n = self.items.len().min(MAX_OWNER_DRAWN);
        self.item_heights = vec![0; n];
        self.measures_pending = n;
        (0..n).map(|i| (self.measure_round, i, self.row_height())).collect()
    }

    /// OnMeasureItem of `round` answered `height` for item `i`: `true` when
    /// it was the last answer to come (the runtime then shows the items and
    /// fires OnDrawItem).
    pub fn measured(&mut self, round: u32, i: usize, height: i64) -> bool {
        if round != self.measure_round {
            return false;
        }
        if let Some(h) = self.item_heights.get_mut(i) {
            *h = height.clamp(0, 2_000);
        }
        self.measures_pending = self.measures_pending.saturating_sub(1);
        self.measures_pending == 0
    }

    /// Whether OnMeasureItem answers are still to come (OnDrawItem waits).
    pub fn measuring(&self) -> bool {
        self.measures_pending > 0
    }

    /// Whether the list changed (items, selection, style, heights) since
    /// OnDrawItem was last fired: if so its drawing is dropped and it must
    /// be fired again. Drawing on the list isn't a change.
    pub fn owner_draw_needed(&mut self) -> bool {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        (&self.items, &self.selected, self.item_index, self.style, self.item_height, &self.item_heights, self.columns, self.view).hash(&mut h);
        let state = h.finish();
        if self.drawn_state == Some(state) {
            return false;
        }
        self.drawn_state = Some(state);
        self.owner_drawing.clear();
        true
    }

    /// Items in columns: (items per column, column width) for the view.
    pub fn column_layout(&self) -> (i64, i64) {
        let (w, h) = self.view;
        ((h / self.row_height()).max(1), (w / self.columns.max(1)).max(1))
    }

    /// Each item's rect in the list's content (Left, Top, Right, Bottom):
    /// stacked by their heights, or in columns; up to [`MAX_OWNER_DRAWN`].
    pub fn item_rects(&self) -> Vec<(i64, i64, i64, i64)> {
        let n = self.items.len().min(MAX_OWNER_DRAWN);
        if self.multi_column() {
            let (per, cw) = self.column_layout();
            let h = self.row_height();
            return (0..n as i64).map(|i| ((i / per) * cw, (i % per) * h, (i / per + 1) * cw, (i % per + 1) * h)).collect();
        }
        let tops = self.tops();
        (0..n).map(|i| (0, tops[i], self.view.0, tops[i + 1])).collect()
    }

    /// The item at (x, y) in the content (a click, a drawing).
    pub fn item_at(&self, x: i64, y: i64) -> Option<usize> {
        self.item_rects().iter().position(|&(l, t, r, b)| x >= l && x < r.max(l + 1) && y >= t && y < b)
    }

    /// The runtime shows the items `width` × `height` (a list box's
    /// client area; a combo box's drop-down width).
    pub fn set_view(&mut self, width: i64, height: i64) {
        self.view = (width.max(1), height.max(1));
    }

    /// Each item's OnDrawItem arguments: (index, State, Rect), the rect in
    /// the list's content ([`Self::item_rects`], in the view the runtime
    /// set first with [`Self::set_view`]), up to [`MAX_OWNER_DRAWN`] items.
    pub fn owner_draw_items(&self) -> Vec<(usize, i64, (i64, i64, i64, i64))> {
        self.item_rects().into_iter().enumerate().map(|(i, r)| (i, if self.is_selected(i) { 0 } else { 1 }, r)).collect()
    }

    /// Keeps a drawing whose anchor is (x, y) on the item there, with
    /// `make` given that item's top left (to make it relative).
    pub fn record(&mut self, x: i64, y: i64, make: impl FnOnce(i64, i64) -> CellDraw) {
        // (in one column, anything left or right of the items is its row's)
        let x = if self.multi_column() { x } else { 0 };
        let Some(i) = self.item_at(x, y) else { return };
        let (l, t, _, _) = self.item_rects()[i];
        let list = self.owner_drawing.entry(i).or_default();
        if list.len() < 1_000 {
            list.push(make(l, t));
        }
    }

    /// The owner-drawing methods (except `Draw`, which needs the source
    /// image: rapidr_value::objects).
    fn draw(&mut self, method: &str, args: &[Value]) -> bool {
        let n = |i: usize| args.get(i).map_or(0, Value::to_i64);
        let c = |i: usize| n(i) as u32 & 0xFFFFFF;
        let optional = |i: usize| args.get(i).map(Value::to_i64).filter(|v| *v >= 0).map(|v| v as u32 & 0xFFFFFF);
        match method {
            "line" => self.record(n(0), n(1), |l, t| CellDraw::Line(n(0) - l, n(1) - t, n(2) - l, n(3) - t, c(4))),
            "rectangle" => self.record(n(0), n(1), |l, t| CellDraw::Rect(n(0) - l, n(1) - t, n(2) - l, n(3) - t, c(4))),
            "fillrect" => self.record(n(0), n(1), |l, t| CellDraw::Fill(n(0) - l, n(1) - t, n(2) - l, n(3) - t, c(4))),
            "circle" => self.record(n(0), n(1), |l, t| CellDraw::Ellipse(n(0) - l, n(1) - t, n(2) - l, n(3) - t, c(4), optional(5))),
            "pset" => self.record(n(0), n(1), |l, t| CellDraw::Pixel(n(0) - l, n(1) - t, c(2))),
            // TextOut(x, y, text, color, background (-1: transparent)).
            "textout" => {
                let text = args.get(2).map(|v| v.to_string_val()).unwrap_or_default();
                self.record(n(0), n(1), |l, t| CellDraw::Text(n(0) - l, n(1) - t, text, c(3), optional(4)))
            }
            _ => return false,
        }
        true
    }

    /// Item `i` as a `width` × ItemHeight bitmap: what OnDrawItem drew on
    /// it, text in `font`; an item it didn't draw on is drawn plainly
    /// (selected: white on blue).
    pub fn render_item(&self, i: usize, width: i64, font: &Font) -> Bitmap {
        let mut b = Bitmap::default();
        let height = self.item_h(i);
        b.resize(width.clamp(1, 10_000), height);
        match self.owner_drawing.get(&i) {
            Some(ops) => ops.iter().for_each(|op| op.paint(&mut b, font)),
            None => {
                let selected = self.is_selected(i);
                if selected {
                    b.fill_rect(0, 0, width, height, 0xD77800);
                }
                let color = if selected { 0xFFFFFF } else { 0 };
                let text = self.items.get(i).map_or("", String::as_str);
                super::text::text_out(&mut b, 2, 0, text, font, color, None);
            }
        }
        b
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
        if self.files.is_some() && matches!(prop, "directory" | "mask" | "drive" | "filename") {
            return self.clone().file_member(prop, &[]);
        }
        Some(match prop {
            "itemcount" | "count" | "listcount" => v_int(self.items.len() as i64),
            "itemindex" | "listindex" => v_int(self.item_index),
            "selcount" => v_int((0..self.items.len()).filter(|&i| self.is_selected(i)).count() as i64),
            "sorted" => flag(self.sorted),
            "multiselect" => flag(self.multi_select),
            "topindex" => v_int(self.top_index),
            "style" if !self.combo => v_int(self.style),
            "itemheight" => v_int(self.row_height()),
            "columns" if !self.combo => v_int(self.columns),
            "text" if self.combo => v_str(&self.text),
            "text" => v_str(&self.to_text()),
            "items" => v_str(&self.items.join("\n")),
            _ => return None,
        })
    }

    pub fn set(&mut self, prop: &str, val: &Value) -> bool {
        if self.files.is_some() && matches!(prop, "directory" | "mask") {
            return self.file_member(prop, std::slice::from_ref(val)).is_some();
        }
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
            "style" if !self.combo => self.style = val.to_i64(),
            // (a combo box's is kept by the runtime too: it builds the
            // widget from it)
            "style" => {
                self.style = val.to_i64();
                return false;
            }
            "itemheight" => self.item_height = val.to_i64().clamp(0, 2_000),
            "columns" if !self.combo => self.columns = val.to_i64().clamp(0, 1_000),
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
        if self.files.is_some() && matches!(method, "addfiletypes" | "delfiletypes" | "update") {
            return self.file_member(method, args);
        }
        let text = |i: usize| args.get(i).map(|v| v.to_string_val()).unwrap_or_default();
        if self.owner_drawn() && self.draw(method, args) {
            return Some(Value::Null);
        }
        // QSTRINGLIST's names for the same operations.
        let method = if self.plain {
            match method {
                "add" => "additems",
                "insert" => "insertitem",
                "delete" | "remove" => "delitems",
                "get" | "strings" | "string" => "item",
                other => other,
            }
        } else {
            method
        };
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

    #[test]
    fn owner_drawing() {
        let mut l = ItemList::new(false);
        l.call("additems", &[s("a"), s("b"), s("c")]);
        l.set("style", &v_int(LB_OWNER_VARIABLE));
        l.set("itemheight", &v_int(20));
        assert!(l.owner_drawn());
        l.select(1);
        l.set_view(100, 120);
        assert!(l.owner_draw_needed());
        assert!(!l.owner_draw_needed());
        // State 0 is the selected item; each Rect is the item's slot.
        assert_eq!(l.owner_draw_items(), vec![(0, 1, (0, 0, 100, 20)), (1, 0, (0, 20, 100, 40)), (2, 1, (0, 40, 100, 60))]);
        // Drawing at content y = 25 lands on item 1, relative to its top.
        l.call("fillrect", &[v_int(0), v_int(20), v_int(100), v_int(40), v_int(0x00FF00)]);
        l.call("textout", &[v_int(4), v_int(25), s("hi"), v_int(0), v_int(-1)]);
        assert_eq!(l.owner_drawing.get(&1).map(Vec::len), Some(2));
        assert!(l.owner_drawing.get(&0).is_none());
        let b = l.render_item(1, 100, &Font::default());
        assert_eq!((b.img.width, b.img.height), (100, 20));
        assert_eq!((b.pixel(50, 2), b.pixel(99, 19)), (Some(0x00FF00), Some(0x00FF00)));
        assert!((0..100).any(|x| (0..20).any(|y| b.pixel(x, y) == Some(0))), "the text was drawn");
        // An item nothing was drawn on: plain (unselected: white).
        assert_eq!(l.render_item(0, 100, &Font::default()).pixel(90, 10), Some(0xFFFFFF));
        // Drawing isn't a change; selecting is.
        assert!(!l.owner_draw_needed());
        l.select(2);
        assert!(l.owner_draw_needed());
        assert!(l.owner_drawing.is_empty());
    }

    #[test]
    fn measured_items() {
        let mut l = ItemList::new(false);
        l.call("additems", &[s("a"), s("b"), s("c")]);
        l.set("itemheight", &v_int(20));
        // Fixed: no OnMeasureItem.
        l.set("style", &v_int(LB_OWNER_FIXED));
        assert!(l.measure_needed().is_empty());
        l.set("style", &v_int(LB_OWNER_VARIABLE));
        let asks = l.measure_needed();
        assert_eq!(asks.iter().map(|a| (a.1, a.2)).collect::<Vec<_>>(), vec![(0, 20), (1, 20), (2, 20)]);
        assert!(l.measure_needed().is_empty() && l.measuring());
        let round = asks[0].0;
        assert!(!l.measured(round, 0, 30));
        assert!(!l.measured(round, 1, 0)); // 0: ItemHeight
        assert!(l.measured(round, 2, 50) && !l.measuring());
        l.set_view(80, 200);
        assert_eq!(l.owner_draw_items(), vec![(0, 1, (0, 0, 80, 30)), (1, 1, (0, 30, 80, 50)), (2, 1, (0, 50, 80, 100))]);
        assert_eq!(l.render_item(2, 80, &Font::default()).img.height, 50);
        // Drawing at y = 60 lands on item 2 (its top is 50).
        l.call("fillrect", &[v_int(0), v_int(60), v_int(10), v_int(70), v_int(0xFF)]);
        assert_eq!(l.owner_drawing.get(&2).map(Vec::len), Some(1));
        // The items changed: asked again; an old round's answer is ignored.
        l.call("additems", &[s("d")]);
        let asks = l.measure_needed();
        assert_eq!(asks.len(), 4);
        assert!(!l.measured(round, 0, 99));
        assert_eq!(l.item_h(0), 20);
    }

    #[test]
    fn columns() {
        let mut l = ItemList::new(false);
        l.call("additems", &(0..7).map(|i| s(&format!("i{i}"))).collect::<Vec<_>>());
        l.set("itemheight", &v_int(20));
        l.set("columns", &v_int(2));
        assert!(l.custom_drawn() && !l.owner_drawn());
        // 65 px high: 3 items a column; 2 columns show in 200 px.
        l.set_view(200, 65);
        assert_eq!(l.column_layout(), (3, 100));
        assert_eq!(l.item_rects()[4], (100, 20, 200, 40));
        assert_eq!(l.item_rects()[6], (200, 0, 300, 20));
        assert_eq!(l.item_at(150, 25), Some(4));
        assert_eq!(l.item_at(250, 30), None);
        // Owner-drawn in columns: a drawing lands on its item, made relative.
        l.set("style", &v_int(LB_OWNER_FIXED));
        l.call("fillrect", &[v_int(110), v_int(22), v_int(120), v_int(30), v_int(0xFF)]);
        assert!(matches!(l.owner_drawing.get(&4).map(Vec::as_slice), Some([CellDraw::Fill(10, 2, 20, 10, 0xFF)])));
    }

    #[test]
    fn owner_drawn_combo() {
        let mut c = ItemList::new(true);
        c.call("additems", &[s("a"), s("b")]);
        // (a combo's Style is the runtime's too)
        assert!(!c.set("style", &v_int(CS_OWNER_VARIABLE)));
        assert!(c.owner_drawn() && c.variable());
        c.set("itemheight", &v_int(18));
        assert_eq!(c.get("itemheight").unwrap().to_i64(), 18);
        c.set_view(90, 300);
        assert_eq!(c.measure_needed().len(), 2);
        assert!(c.call("textout", &[v_int(2), v_int(20), s("x"), v_int(0), v_int(-1)]).is_some());
        assert_eq!(c.owner_drawing.get(&1).map(Vec::len), Some(1));
    }
}

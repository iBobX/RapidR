//! QLISTVIEW (RapidQ manual, Appendix A): columns, items with their
//! sub-items, the selection — and the control itself, the same on every
//! runtime: this model lays the items out in RapidQ's four views, paints
//! them (onto a bitmap the runtime shows, at the screen's scale) and turns
//! the mouse and the keyboard into what Windows' list view does.
//!
//! * `AddColumns "Name", "Size"`, `ClearColumns`, `Column(i).Caption` /
//!   `.Width`, `ColumnsCount`;
//! * `AddItems "a", "b"`, `InsertItem i, s`, `DelItems i, …`, `Clear`,
//!   `SwapItem i, j`, `Item(i).Caption` / `.Checked` / `.Selected` /
//!   `.ImageIndex` / `.StateIndex` / `.Index`, `ItemCount`, `ItemIndex`,
//!   `SelCount`, `Selected(i)`;
//! * `AddSubItem i, s`, `InsertSubItem i, j, s`, `DelSubItem i, j`,
//!   `SubItem(i, j)` (sub-item `j` of item `i`, 0 = the first one after the
//!   caption);
//! * `ViewStyle` (vsIcon 0, vsSmallIcon 1, vsList 2, vsReport 3),
//!   `LargeImages` / `SmallImages` / `StateImages`, `CheckBoxes`,
//!   `MultiSelect`, `RowSelect`, `GridLines`, `HotTrack`, `HideSelection`,
//!   `ColumnClick`, `ShowColumnHeaders`, `SortType` (stText 2: by caption),
//!   `ReadOnly`, `BorderStyle`.
//!
//! What the user does: a click selects (Ctrl adds or removes, Shift a
//! range, with MultiSelect), a check box toggles, the arrows / Home / End /
//! Page Up / Page Down move, Space checks; the header's buttons fire
//! OnColumnClick and its edges resize the columns; scroll bars and the
//! wheel scroll. OnChange (Index, Change: ctState 2) follows each item
//! whose selection or check the user changed, OnClick a click, OnDblClick a
//! double click.
//!
//! Indexes out of range read as "" / 0 and are ignored when written, as
//! RapidQ does not stop the program for them.

use super::bitmap::Bitmap;
use super::font::Font;
use super::header::{self, Header};
use super::text::{text_out, text_size};
use crate::{v_int, v_str, Value};

/// The width a column gets unless the program sets one (Windows' default).
pub const DEFAULT_COLUMN_WIDTH: i64 = 50;

pub const VS_ICON: i64 = 0;
pub const VS_SMALL_ICON: i64 = 1;
pub const VS_LIST: i64 = 2;
pub const VS_REPORT: i64 = 3;
/// SortType stText: kept in order of their captions (stBoth too).
pub const ST_TEXT: i64 = 2;
pub const ST_BOTH: i64 = 3;
/// OnChange's Change: ctText, ctImage, ctState.
pub const CT_TEXT: i64 = 0;
pub const CT_IMAGE: i64 = 1;
pub const CT_STATE: i64 = 2;

/// Scroll bars' thickness (Windows' 17 pixels).
const BAR: i64 = 17;
/// A check box's size, and the slot it takes (a state image's).
const CHECK: i64 = 13;
const CHECK_SLOT: i64 = 16;

/// The colours a list view draws in (&HBBGGRR, as RapidQ's and its
/// bitmap's): the current theme's (`crate::theme`).
#[derive(Clone, Copy, Debug)]
struct Colors {
    window: u32,
    highlight: u32,
    highlight_text: u32,
    inactive: u32,
    hot: u32,
    grid: u32,
    border: u32,
    bar_track: u32,
    bar_thumb: u32,
    bar_thumb_held: u32,
    bar_arrow: u32,
    check_ink: u32,
    text: u32,
}

impl Colors {
    fn now() -> Colors {
        use crate::theme::bgr;
        let t = crate::theme::current();
        Colors {
            window: bgr(t.window),
            highlight: bgr(t.highlight),
            highlight_text: bgr(t.highlight_text),
            inactive: bgr(t.unfocused),
            hot: bgr(t.view_hot),
            grid: bgr(t.view_grid),
            border: bgr(t.view_border),
            // (the classic look's bars sit on the face; a fluent one's on its
            // quiet track)
            bar_track: bgr(if t.fluent() { t.track } else { t.face }),
            bar_thumb: bgr(t.view_thumb),
            bar_thumb_held: bgr(t.view_thumb_held),
            bar_arrow: bgr(t.view_arrow),
            check_ink: bgr(t.view_check),
            text: bgr(t.text),
        }
    }
}

// (the classic look's, which the tests read back)
#[cfg(test)]
const WINDOW: u32 = 0xFFFFFF;
#[cfg(test)]
const HIGHLIGHT: u32 = 0xD77800;
#[cfg(test)]
const INACTIVE: u32 = 0xF0F0F0;
#[cfg(test)]
const BORDER: u32 = 0x908782;
#[cfg(test)]
const CHECK_INK: u32 = 0x333333;

#[derive(Clone, Debug, Default)]
pub struct Column {
    pub caption: String,
    pub width: i64,
}

#[derive(Clone, Debug)]
pub struct Item {
    pub caption: String,
    pub sub_items: Vec<String>,
    pub checked: bool,
    pub selected: bool,
    pub image_index: i64,
    /// -1: no state image.
    pub state_index: i64,
}

impl Default for Item {
    fn default() -> Self {
        Self { caption: String::new(), sub_items: Vec::new(), checked: false, selected: false, image_index: 0, state_index: -1 }
    }
}

/// What the program hears of what the user did.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Click,
    DblClick,
    /// OnColumnClick (Column).
    ColumnClick(usize),
    /// OnChange (Index, Change).
    Change(usize, i64),
    /// (for the runtime) Item `i`'s caption is to be edited now (F2) …
    Edit(usize),
    /// … or in a moment (a click on the selected item), unless another
    /// click came first ([`ListView::clicks`] changed).
    EditSoon(usize),
}

/// What the runtime tells the model before it lays the items out: the
/// control's size and font, and its image lists' image sizes.
#[derive(Clone, Debug, Default)]
pub struct View {
    pub width: i64,
    pub height: i64,
    pub font: Font,
    pub small: Option<(i64, i64)>,
    pub large: Option<(i64, i64)>,
    pub state: Option<(i64, i64)>,
}

/// The images to paint with (image `i` of each list).
#[derive(Default)]
pub struct Images {
    pub small: Vec<Bitmap>,
    pub large: Vec<Bitmap>,
    pub state: Vec<Bitmap>,
}

type Rect = (i64, i64, i64, i64);

fn inside((l, t, r, b): Rect, x: i64, y: i64) -> bool {
    x >= l && x < r && y >= t && y < b
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Part {
    Less,
    More,
    PageLess,
    PageMore,
    Thumb,
}

/// A scroll bar: where it is, and what it scrolls.
#[derive(Clone, Copy, Debug)]
struct Bar {
    vertical: bool,
    rect: Rect,
    content: i64,
    view: i64,
    pos: i64,
}

impl Bar {
    fn len(&self) -> i64 {
        if self.vertical { self.rect.3 - self.rect.1 } else { self.rect.2 - self.rect.0 }
    }

    /// The thumb's start and end along the bar.
    fn thumb(&self) -> (i64, i64) {
        let track = (self.len() - 2 * BAR).max(0);
        let size = (track * self.view / self.content.max(1)).clamp(track.min(10), track);
        let room = (self.content - self.view).max(1);
        let at = BAR + (track - size) * self.pos.clamp(0, room) / room;
        (at, at + size)
    }

    fn part(&self, x: i64, y: i64) -> Part {
        let along = if self.vertical { y - self.rect.1 } else { x - self.rect.0 };
        let (t0, t1) = self.thumb();
        if along < BAR {
            Part::Less
        } else if along >= self.len() - BAR {
            Part::More
        } else if along < t0 {
            Part::PageLess
        } else if along >= t1 {
            Part::PageMore
        } else {
            Part::Thumb
        }
    }
}

/// Where everything is, for the current size, font, view and scroll.
#[derive(Clone, Debug)]
struct Layout {
    /// The border's width (0 with BorderStyle bsNone).
    inset: i64,
    header_h: i64,
    row_h: i64,
    /// An item's cell (list, small icon and icon views).
    cell: (i64, i64),
    /// Items across a line (icon views) or down a column (list view).
    per_line: i64,
    /// Where the items show.
    view: Rect,
    content: (i64, i64),
    vbar: Option<Bar>,
    hbar: Option<Bar>,
    text_h: i64,
}

/// An item's parts on the screen.
#[derive(Clone, Debug, Default)]
struct Parts {
    /// The whole row / cell.
    cell: Rect,
    check: Option<Rect>,
    state: Option<Rect>,
    icon: Option<Rect>,
    label: Rect,
    /// The caption's lines (the icon view wraps it).
    lines: Vec<String>,
}

#[derive(Clone, Debug)]
enum Drag {
    Header,
    Thumb { vertical: bool, from: i64, start: i64 },
    Bar,
}

#[derive(Clone, Debug)]
pub struct ListView {
    pub columns: Vec<Column>,
    pub items: Vec<Item>,
    /// The first selected item (-1: none), as ItemIndex reads.
    pub item_index: i64,
    pub view_style: i64,
    pub check_boxes: bool,
    pub grid_lines: bool,
    pub row_select: bool,
    pub multi_select: bool,
    pub hide_selection: bool,
    pub hot_track: bool,
    pub column_click: bool,
    pub show_column_headers: bool,
    pub read_only: bool,
    pub sort_type: i64,
    pub border_style: i64,
    pub large_images: String,
    pub small_images: String,
    pub state_images: String,
    /// The item with the keyboard's focus.
    pub focus: Option<usize>,
    /// Where a Shift range starts.
    anchor: Option<usize>,
    /// The item under the mouse (HotTrack).
    pub hot: Option<usize>,
    pub scroll: (i64, i64),
    /// Whether the control has the keyboard's focus.
    pub focused: bool,
    view: View,
    header: Header,
    drag: Option<Drag>,
    /// The mouse went down on the items (a click follows its release).
    pressed_items: bool,
    /// A double click was just handled: its release is no click.
    after_double: bool,
    /// A click on the selected item: its caption is edited after the
    /// release (unless it becomes a double click).
    pending_edit: Option<usize>,
    /// Clicks so far (a delayed edit checks no other came).
    pub clicks: u64,
}

impl Default for ListView {
    fn default() -> Self {
        Self {
            columns: Vec::new(),
            items: Vec::new(),
            item_index: -1,
            view_style: VS_ICON,
            check_boxes: false,
            grid_lines: false,
            row_select: false,
            multi_select: false,
            hide_selection: false,
            hot_track: false,
            column_click: true,
            show_column_headers: true,
            read_only: false,
            sort_type: 0,
            border_style: 1,
            large_images: String::new(),
            small_images: String::new(),
            state_images: String::new(),
            focus: None,
            anchor: None,
            hot: None,
            scroll: (0, 0),
            focused: false,
            view: View { width: 0, height: 0, ..View::default() },
            header: Header::default(),
            drag: None,
            pressed_items: false,
            after_double: false,
            pending_edit: None,
            clicks: 0,
        }
    }
}

fn index(v: Option<&Value>) -> Option<usize> {
    let i = v?.to_i64();
    usize::try_from(i).ok()
}

fn flag(b: bool) -> Value {
    v_int(if b { -1 } else { 0 })
}

/// `text` cut to `max` pixels with "..." (as Windows shows a caption too
/// long for its column).
fn fit(text: &str, font: &Font, max: i64) -> String {
    if text_size(text, font).0 <= max {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let (mut lo, mut hi) = (0, chars.len());
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        let s: String = chars[..mid].iter().collect::<String>() + "...";
        if text_size(&s, font).0 <= max {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    if lo == 0 {
        return String::new();
    }
    chars[..lo].iter().collect::<String>() + "..."
}

/// `text` broken into at most `max_lines` lines of `max` pixels (the icon
/// view's captions), the last cut with "...".
fn wrap(text: &str, font: &Font, max: i64, max_lines: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in text.split(' ') {
        let candidate = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
        if text_size(&candidate, font).0 <= max || line.is_empty() {
            line = candidate;
        } else {
            lines.push(std::mem::take(&mut line));
            line = word.to_string();
        }
    }
    lines.push(line);
    if lines.len() > max_lines {
        let rest = lines[max_lines - 1..].join(" ");
        lines.truncate(max_lines - 1);
        lines.push(rest);
    }
    lines.into_iter().map(|l| fit(&l, font, max)).collect()
}

impl ListView {
    fn item(&self, i: Option<usize>) -> Option<&Item> {
        self.items.get(i?)
    }

    fn item_mut(&mut self, i: Option<usize>) -> Option<&mut Item> {
        self.items.get_mut(i?)
    }

    /// ItemIndex: the first selected item.
    fn sync_index(&mut self) {
        self.item_index = self.items.iter().position(|i| i.selected).map_or(-1, |i| i as i64);
    }

    /// Selects item `i` alone (-1: nothing); the focus goes with it.
    pub fn select(&mut self, i: i64) {
        let i = if i >= 0 && (i as usize) < self.items.len() { i } else { -1 };
        for (n, item) in self.items.iter_mut().enumerate() {
            item.selected = n as i64 == i;
        }
        if i >= 0 {
            self.focus = Some(i as usize);
            self.anchor = Some(i as usize);
        }
        self.sync_index();
    }

    /// Item `i` (de)selected by the program (`Item(i).Selected`,
    /// `Selected(i)`): alone unless MultiSelect.
    fn set_selected(&mut self, i: usize, on: bool) {
        if i >= self.items.len() {
            return;
        }
        if on && !self.multi_select {
            self.select(i as i64);
            return;
        }
        self.items[i].selected = on;
        self.sync_index();
    }

    /// The focus, anchor and hot item after items moved: `map` tells where
    /// an item went (None: deleted).
    fn remap(&mut self, map: impl Fn(usize) -> Option<usize>) {
        self.focus = self.focus.and_then(&map);
        self.anchor = self.anchor.and_then(&map);
        self.hot = self.hot.and_then(&map);
    }

    /// Keeps the items in order of their captions (SortType stText /
    /// stBoth), each one's selection and the focus with it.
    fn resort(&mut self) {
        if !matches!(self.sort_type, ST_TEXT | ST_BOTH) || self.items.len() < 2 {
            return;
        }
        let mut order: Vec<usize> = (0..self.items.len()).collect();
        order.sort_by_cached_key(|&i| self.items[i].caption.to_lowercase());
        if order.iter().enumerate().all(|(n, &i)| n == i) {
            return;
        }
        let mut place = vec![0; order.len()];
        for (n, &i) in order.iter().enumerate() {
            place[i] = n;
        }
        let old = std::mem::take(&mut self.items);
        let mut slots: Vec<Option<Item>> = old.into_iter().map(Some).collect();
        self.items = order.iter().map(|&i| slots[i].take().unwrap_or_default()).collect();
        self.remap(|i| place.get(i).copied());
        self.sync_index();
    }

    fn insert_at(&mut self, at: usize, captions: Vec<String>) {
        let at = at.min(self.items.len());
        let n = captions.len();
        self.items.splice(at..at, captions.into_iter().map(|caption| Item { caption, ..Item::default() }));
        self.remap(|i| Some(if i >= at { i + n } else { i }));
        self.sync_index();
        self.resort();
    }

    fn delete_at(&mut self, i: usize) {
        if i >= self.items.len() {
            return;
        }
        self.items.remove(i);
        self.remap(|k| match k.cmp(&i) {
            std::cmp::Ordering::Less => Some(k),
            std::cmp::Ordering::Equal => None,
            std::cmp::Ordering::Greater => Some(k - 1),
        });
        self.sync_index();
    }

    pub fn get(&self, prop: &str) -> Option<Value> {
        Some(match prop {
            "itemcount" | "count" => v_int(self.items.len() as i64),
            "columnscount" | "columncount" => v_int(self.columns.len() as i64),
            "itemindex" => v_int(self.item_index),
            "selcount" => v_int(self.items.iter().filter(|i| i.selected).count() as i64),
            "viewstyle" => v_int(self.view_style),
            "checkboxes" => flag(self.check_boxes),
            "gridlines" => flag(self.grid_lines),
            "rowselect" => flag(self.row_select),
            "multiselect" => flag(self.multi_select),
            "hideselection" => flag(self.hide_selection),
            "hottrack" => flag(self.hot_track),
            "columnclick" => flag(self.column_click),
            "showcolumnheaders" => flag(self.show_column_headers),
            "readonly" => flag(self.read_only),
            "sorttype" => v_int(self.sort_type),
            "borderstyle" => v_int(self.border_style),
            "largeimages" => v_str(&self.large_images),
            "smallimages" => v_str(&self.small_images),
            "stateimages" => v_str(&self.state_images),
            _ => return None,
        })
    }

    pub fn set(&mut self, prop: &str, val: &Value) -> bool {
        match prop {
            "itemindex" => self.select(val.to_i64()),
            "viewstyle" => {
                self.view_style = val.to_i64().clamp(VS_ICON, VS_REPORT);
                self.scroll = (0, 0);
            }
            "checkboxes" => self.check_boxes = val.to_bool(),
            "gridlines" => self.grid_lines = val.to_bool(),
            "rowselect" => self.row_select = val.to_bool(),
            "multiselect" => {
                self.multi_select = val.to_bool();
                // (one selected item stays, the first)
                if !self.multi_select && self.items.iter().filter(|i| i.selected).count() > 1 {
                    let first = self.item_index;
                    self.select(first);
                }
            }
            "hideselection" => self.hide_selection = val.to_bool(),
            "hottrack" => {
                self.hot_track = val.to_bool();
                self.hot = None;
            }
            "columnclick" => self.column_click = val.to_bool(),
            "showcolumnheaders" => self.show_column_headers = val.to_bool(),
            "readonly" => self.read_only = val.to_bool(),
            "sorttype" => {
                self.sort_type = val.to_i64();
                self.resort();
            }
            "borderstyle" => self.border_style = val.to_i64(),
            "largeimages" => self.large_images = val.to_string_val(),
            "smallimages" => self.small_images = val.to_string_val(),
            "stateimages" => self.state_images = val.to_string_val(),
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
            // RapidR's canvas names: a list view has no surface to draw on,
            // so nothing is drawn (as a list box that isn't owner-drawn).
            "rect" | "setpixel" | "ellipse" | "drawtext" => {}
            // (AddItem / DeleteItem: RapidR's other names, as a list box's)
            "additems" | "additem" => {
                let at = self.items.len();
                self.insert_at(at, args.iter().map(|a| a.to_string_val()).collect());
            }
            "insertitem" => {
                let at = index(args.first()).unwrap_or(usize::MAX);
                self.insert_at(at, vec![text(1)]);
            }
            "delitems" | "deleteitem" => {
                let mut gone: Vec<usize> = args.iter().filter_map(|a| index(Some(a))).filter(|&i| i < self.items.len()).collect();
                gone.sort_unstable();
                gone.dedup();
                for &i in gone.iter().rev() {
                    self.delete_at(i);
                }
            }
            "clear" => {
                self.items.clear();
                self.item_index = -1;
                self.remap(|_| None);
                self.scroll = (0, 0);
            }
            "swapitem" => {
                if let (Some(a), Some(b)) = (index(args.first()), index(args.get(1))) {
                    if a < self.items.len() && b < self.items.len() {
                        self.items.swap(a, b);
                        self.remap(|i| Some(if i == a { b } else if i == b { a } else { i }));
                        self.sync_index();
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
            "selected" => return Some(flag(self.item(index(args.first())).is_some_and(|i| i.selected))),
            "selected=" => {
                let on = args.get(1).is_some_and(|v| v.to_bool());
                if let Some(i) = index(args.first()) {
                    self.set_selected(i, on);
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
                        if let Some(i) = i {
                            self.set_selected(i, val.to_bool());
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
                    if member == "caption" {
                        self.resort();
                    }
                    return Some(Value::Null);
                }
                let item = self.item(i);
                Some(match member {
                    "caption" => v_str(item.map_or("", |i| i.caption.as_str())),
                    "checked" => flag(item.is_some_and(|i| i.checked)),
                    "selected" => flag(item.is_some_and(|i| i.selected)),
                    "imageindex" => v_int(item.map_or(0, |i| i.image_index)),
                    "stateindex" => v_int(item.map_or(-1, |i| i.state_index)),
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

    // ------------------------------------------------------------------
    // Layout
    // ------------------------------------------------------------------

    /// The control's size, font and image sizes (the runtime, before it
    /// paints or passes the mouse on).
    pub fn set_view(&mut self, view: View) {
        self.view = view;
    }

    /// The header shown (report view with columns, ShowColumnHeaders).
    fn has_header(&self) -> bool {
        self.view_style == VS_REPORT && self.show_column_headers && !self.columns.is_empty()
    }

    /// The small image slot (small icon, list and report views).
    fn small_size(&self) -> Option<(i64, i64)> {
        self.view.small
    }

    /// The state image slot: a check box, or StateImages' image.
    fn state_slot(&self) -> Option<(i64, i64)> {
        if self.check_boxes {
            Some((CHECK_SLOT, CHECK_SLOT))
        } else {
            self.view.state
        }
    }

    fn layout(&self) -> Layout {
        let font = &self.view.font;
        let text_h = text_size("Ag", font).1.max(1);
        let inset = if self.border_style == 0 { 0 } else { 1 };
        let (w, h) = (self.view.width.max(0), self.view.height.max(0));
        let header_h = if self.has_header() { text_h + 6 } else { 0 };
        let small = self.small_size();
        let slot_h = small.map_or(0, |s| s.1).max(self.state_slot().map_or(0, |s| s.1));
        let row_h = (text_h + 3).max(slot_h + 1);
        let n = self.items.len() as i64;
        // The widest caption with its images (list and small icon views).
        let item_w = |it: &Item| {
            let mut x = 4;
            if let Some(s) = self.state_slot() {
                x += s.0;
            }
            if let Some(s) = small {
                x += s.0 + 2;
            }
            x + text_size(&it.caption, font).0 + 6
        };
        let large = self.view.large.unwrap_or((32, 32));
        let cell = match self.view_style {
            VS_ICON => ((large.0 + 43).max(75), large.1 + 8 + 2 * text_h + 4),
            VS_REPORT => (0, row_h),
            _ => (self.items.iter().map(item_w).max().unwrap_or(60).clamp(20, 400), row_h),
        };
        let area = (inset, inset + header_h, w - inset, h - inset);
        // Content size for a viewport `vw` × `vh`.
        let measure = |vw: i64, vh: i64| -> ((i64, i64), i64) {
            match self.view_style {
                VS_REPORT => ((self.columns.iter().map(|c| c.width.max(0)).sum(), n * row_h), 1),
                VS_LIST => {
                    let rows = (vh / cell.1).max(1);
                    let cols = (n + rows - 1) / rows;
                    ((cols * cell.0, rows.min(n.max(1)) * cell.1), rows)
                }
                _ => {
                    let per = (vw / cell.0).max(1);
                    let rows = (n + per - 1) / per;
                    ((per.min(n.max(1)) * cell.0, rows * cell.1), per)
                }
            }
        };
        let (mut vw, mut vh) = (area.2 - area.0, area.3 - area.1);
        let (mut need_v, mut need_h) = (false, false);
        let mut result = measure(vw, vh);
        for _ in 0..3 {
            let (content, _) = result;
            let v = self.view_style != VS_LIST && content.1 > vh;
            let hz = matches!(self.view_style, VS_REPORT | VS_LIST) && content.0 > vw;
            if v == need_v && hz == need_h {
                break;
            }
            (need_v, need_h) = (v, hz);
            vw = area.2 - area.0 - if need_v { BAR } else { 0 };
            vh = area.3 - area.1 - if need_h { BAR } else { 0 };
            result = measure(vw.max(0), vh.max(0));
        }
        let ((cw, ch), per_line) = result;
        let view = (area.0, area.1, area.0 + vw.max(0), area.1 + vh.max(0));
        let scroll_x = self.scroll.0.clamp(0, (cw - vw).max(0));
        let scroll_y = self.scroll.1.clamp(0, (ch - vh).max(0));
        // (the vertical bar reaches up beside the header)
        let vbar = need_v.then_some(Bar { vertical: true, rect: (view.2, inset, view.2 + BAR, view.3), content: ch, view: vh, pos: scroll_y });
        let hbar = need_h.then_some(Bar { vertical: false, rect: (view.0, view.3, view.2, view.3 + BAR), content: cw, view: vw, pos: scroll_x });
        Layout { inset, header_h, row_h, cell, per_line, view, content: (cw, ch), vbar, hbar, text_h }
    }

    /// The scroll position the layout allows.
    fn scroll_of(&self, l: &Layout) -> (i64, i64) {
        (self.scroll.0.clamp(0, (l.content.0 - (l.view.2 - l.view.0)).max(0)), self.scroll.1.clamp(0, (l.content.1 - (l.view.3 - l.view.1)).max(0)))
    }

    fn parts(&self, l: &Layout, i: usize) -> Parts {
        let font = &self.view.font;
        let (sx, sy) = self.scroll_of(l);
        let item = &self.items[i];
        let i = i as i64;
        let state_slot = self.state_slot();
        let small = self.small_size();
        // A row / cell with the images left of the caption.
        let beside = |cell: Rect, max_right: i64| -> Parts {
            let mut x = cell.0 + 2;
            let mid = |h: i64| cell.1 + (cell.3 - cell.1 - h) / 2;
            let mut p = Parts { cell, ..Parts::default() };
            if let Some((sw, sh)) = state_slot {
                if self.check_boxes {
                    p.check = Some((x + (CHECK_SLOT - CHECK) / 2, mid(CHECK), x + (CHECK_SLOT - CHECK) / 2 + CHECK, mid(CHECK) + CHECK));
                } else if item.state_index >= 0 {
                    p.state = Some((x, mid(sh), x + sw, mid(sh) + sh));
                }
                x += sw;
            }
            if let Some((iw, ih)) = small {
                p.icon = Some((x, mid(ih), x + iw, mid(ih) + ih));
                x += iw + 2;
            }
            let caption = fit(&item.caption, font, (max_right - x - 4).max(0));
            let tw = text_size(&caption, font).0;
            p.label = (x, cell.1, (x + tw + 4).min(max_right.max(x)), cell.3);
            p.lines = vec![caption];
            p
        };
        match self.view_style {
            VS_REPORT => {
                let top = l.view.1 + i * l.row_h - sy;
                let width: i64 = self.columns.iter().map(|c| c.width.max(0)).sum();
                let left = l.view.0 - sx;
                let first = self.columns.first().map_or(0, |c| c.width.max(0));
                let mut p = beside((left, top, left + width, top + l.row_h), left + first);
                p.cell = (left, top, left + width.max(first), top + l.row_h);
                p
            }
            VS_LIST => {
                let (col, row) = (i / l.per_line, i % l.per_line);
                let (x, y) = (l.view.0 + col * l.cell.0 - sx, l.view.1 + row * l.cell.1 - sy);
                beside((x, y, x + l.cell.0, y + l.cell.1), x + l.cell.0)
            }
            VS_SMALL_ICON => {
                let (row, col) = (i / l.per_line, i % l.per_line);
                let (x, y) = (l.view.0 + col * l.cell.0 - sx, l.view.1 + row * l.cell.1 - sy);
                beside((x, y, x + l.cell.0, y + l.cell.1), x + l.cell.0)
            }
            _ => {
                let (row, col) = (i / l.per_line, i % l.per_line);
                let (x, y) = (l.view.0 + col * l.cell.0 - sx, l.view.1 + row * l.cell.1 - sy);
                let (lw, lh) = self.view.large.unwrap_or((32, 32));
                let ix = x + (l.cell.0 - lw) / 2;
                let icon = (ix, y + 4, ix + lw, y + 4 + lh);
                let mut p = Parts { cell: (x, y, x + l.cell.0, y + l.cell.1), icon: Some(icon), ..Parts::default() };
                if let Some((sw, sh)) = state_slot {
                    if self.check_boxes {
                        p.check = Some((ix - CHECK - 2, icon.3 - CHECK, ix - 2, icon.3));
                    } else if item.state_index >= 0 {
                        p.state = Some((ix - sw - 1, icon.3 - sh, ix - 1, icon.3));
                    }
                }
                let lines = wrap(&item.caption, font, l.cell.0 - 6, 2);
                let tw = lines.iter().map(|s| text_size(s, font).0).max().unwrap_or(0);
                let top = icon.3 + 2;
                p.label = (x + (l.cell.0 - tw) / 2 - 2, top, x + (l.cell.0 + tw) / 2 + 2, top + lines.len() as i64 * l.text_h + 2);
                p.lines = lines;
                p
            }
        }
    }

    /// Items that may show (the rest are scrolled away).
    fn shown(&self, l: &Layout) -> std::ops::Range<usize> {
        let n = self.items.len();
        if self.view_style != VS_REPORT {
            return 0..n;
        }
        let (_, sy) = self.scroll_of(l);
        let first = (sy / l.row_h.max(1)).max(0) as usize;
        let last = ((sy + l.view.3 - l.view.1) / l.row_h.max(1) + 1).max(0) as usize;
        first.min(n)..(last + 1).min(n)
    }

    /// Whether a click at (x, y) is on item `i` (Windows' hit test: the
    /// caption, its images and check box; the whole row with RowSelect).
    fn on_item(&self, p: &Parts, x: i64, y: i64) -> bool {
        if self.view_style == VS_REPORT && self.row_select {
            return inside(p.cell, x, y);
        }
        let left = p.check.or(p.state).or(p.icon).map_or(p.label.0, |r| r.0.min(p.label.0));
        if self.view_style == VS_ICON {
            return inside(p.label, x, y) || p.icon.is_some_and(|r| inside(r, x, y)) || p.check.is_some_and(|r| inside(r, x, y));
        }
        inside((left, p.cell.1, p.label.2, p.cell.3), x, y)
    }

    /// The item at (x, y), and whether it's on its check box.
    fn item_at(&self, l: &Layout, x: i64, y: i64) -> Option<(usize, bool)> {
        if !inside(l.view, x, y) {
            return None;
        }
        self.shown(l).find_map(|i| {
            let p = self.parts(l, i);
            if p.check.is_some_and(|r| inside(r, x, y)) {
                return Some((i, true));
            }
            self.on_item(&p, x, y).then_some((i, false))
        })
    }

    /// GetItemAt-like: the item at (x, y) of the control (-1: none).
    pub fn item_index_at(&self, x: i64, y: i64) -> i64 {
        let l = self.layout();
        self.item_at(&l, x, y).map_or(-1, |(i, _)| i as i64)
    }

    // ------------------------------------------------------------------
    // Painting
    // ------------------------------------------------------------------

    /// The control as the screen shows it (`background`: its Color),
    /// `images` its image lists'.
    pub fn paint(&mut self, background: u32, images: &Images) -> Bitmap {
        self.sync_header();
        let l = self.layout();
        self.scroll = self.scroll_of(&l);
        let (w, h) = (self.view.width.max(1), self.view.height.max(1));
        let mut b = Bitmap::default();
        b.resize(w, h);
        b.fill_rect(0, 0, w, h, background);
        let font = self.view.font.clone();
        let colors = Colors::now();
        let c = colors;
        // (a font the program didn't colour: the theme's text)
        let text = match crate::objects::color_bgr(font.color) {
            0 => c.text,
            ink => ink,
        };
        let active = self.focused || !self.hide_selection;
        // Grid lines (report view): under the items, over the whole area.
        if self.grid_lines && self.view_style == VS_REPORT {
            let (sx, sy) = self.scroll;
            let mut y = l.view.1 - sy % l.row_h.max(1) + l.row_h - 1;
            while y < l.view.3 {
                b.line(l.view.0, y, l.view.2 - 1, y, c.grid);
                y += l.row_h.max(1);
            }
            let mut x = l.view.0 - sx;
            for c in &self.columns {
                x += c.width.max(0);
                if x > l.view.0 && x <= l.view.2 {
                    b.line(x - 1, l.view.1, x - 1, l.view.3 - 1, colors.grid);
                }
            }
        }
        for i in self.shown(&l) {
            let p = self.parts(&l, i);
            if p.cell.3 < l.view.1 || p.cell.1 > l.view.3 || p.cell.2 < l.view.0 || p.cell.0 > l.view.2 {
                continue;
            }
            let item = &self.items[i];
            let selected = item.selected && active;
            let fill = if !item.selected {
                (self.hot_track && self.hot == Some(i)).then_some(c.hot)
            } else if !active {
                None
            } else if self.focused {
                Some(c.highlight)
            } else {
                Some(c.inactive)
            };
            let ink = if selected && self.focused { c.highlight_text } else { text };
            let whole_row = self.view_style == VS_REPORT && self.row_select;
            if let Some(fill) = fill {
                let r = if whole_row { p.cell } else { p.label };
                b.fill_rect(r.0, r.1, r.2, r.3, fill);
            }
            // (a focused item that isn't selected: a dotted frame)
            if self.focused && self.focus == Some(i) && !item.selected {
                let r = if whole_row { p.cell } else { p.label };
                dotted(&mut b, r, text);
            }
            if let Some(r) = p.check {
                check_box(&mut b, r, item.checked, &c);
            }
            if let (Some(r), Some(img)) = (p.state, usize::try_from(item.state_index).ok().and_then(|k| images.state.get(k))) {
                b.draw(r.0, r.1, img);
            }
            if let Some(r) = p.icon {
                let list = if self.view_style == VS_ICON { &images.large } else { &images.small };
                if let Some(img) = usize::try_from(item.image_index).ok().and_then(|k| list.get(k)) {
                    b.draw(r.0, r.1, img);
                }
            }
            let ty = |line: i64| if self.view_style == VS_ICON { p.label.1 + 1 + line * l.text_h } else { p.cell.1 + (p.cell.3 - p.cell.1 - l.text_h) / 2 };
            for (n, line) in p.lines.iter().enumerate() {
                let tx = if self.view_style == VS_ICON { p.cell.0 + (p.cell.2 - p.cell.0 - text_size(line, &font).0) / 2 } else { p.label.0 + 2 };
                text_out(&mut b, tx, ty(n as i64), line, &font, ink, None);
            }
            // Report view: the sub-items in their columns.
            if self.view_style == VS_REPORT {
                let mut x = p.cell.0 + self.columns.first().map_or(0, |c| c.width.max(0));
                for (k, c) in self.columns.iter().enumerate().skip(1) {
                    let cw = c.width.max(0);
                    if let Some(s) = item.sub_items.get(k - 1).filter(|s| !s.is_empty()) {
                        let s = fit(s, &font, cw - 12);
                        let ink = if selected && whole_row && self.focused { colors.highlight_text } else { text };
                        text_out(&mut b, x + 6, ty(0), &s, &font, ink, None);
                    }
                    x += cw;
                }
            }
        }
        // The header over the rows scrolled under it.
        if l.header_h > 0 {
            let mut hb = Bitmap::default();
            hb.resize((l.view.2 - l.view.0).max(1), l.header_h);
            self.header.paint_scrolled(&mut hb, &font, self.scroll.0);
            b.draw(l.view.0, l.inset, &hb);
        }
        for bar in [l.vbar, l.hbar].into_iter().flatten() {
            let held = matches!(self.drag, Some(Drag::Thumb { vertical, .. }) if vertical == bar.vertical);
            scroll_bar(&mut b, &bar, held, &c);
        }
        if let (Some(v), Some(hz)) = (l.vbar, l.hbar) {
            b.fill_rect(v.rect.0, hz.rect.1, v.rect.2, hz.rect.3, c.bar_track);
        }
        if l.inset > 0 {
            b.rectangle(0, 0, w, h, c.border);
        }
        b
    }

    /// The header's sections from the columns (captions, widths, clickable).
    fn sync_header(&mut self) {
        if self.header.sections.len() != self.columns.len() {
            self.header.call("clear", &[]);
            let captions: Vec<Value> = self.columns.iter().map(|c| v_str(&c.caption)).collect();
            self.header.call("addsections", &captions);
        }
        for (s, c) in self.header.sections.iter_mut().zip(&self.columns) {
            s.caption.clone_from(&c.caption);
            s.width = c.width.max(0);
            s.allow_click = self.column_click;
        }
    }

    // ------------------------------------------------------------------
    // The mouse and the keyboard
    // ------------------------------------------------------------------

    /// Sets item `i`'s selection, noting a change.
    fn mark(&mut self, i: usize, on: bool, events: &mut Vec<Event>) {
        if self.items[i].selected != on {
            self.items[i].selected = on;
            events.push(Event::Change(i, CT_STATE));
        }
    }

    /// The user picks item `i` (a click or a key): Ctrl toggles it or only
    /// moves the focus, Shift selects the range from the anchor (with
    /// MultiSelect); else it's selected alone.
    fn pick(&mut self, i: usize, shift: bool, ctrl: bool, toggle: bool, events: &mut Vec<Event>) {
        let multi = self.multi_select;
        if multi && shift {
            let a = self.anchor.unwrap_or(i);
            let (lo, hi) = (a.min(i), a.max(i));
            for k in 0..self.items.len() {
                let on = (lo..=hi).contains(&k) || (ctrl && self.items[k].selected);
                self.mark(k, on, events);
            }
        } else if multi && ctrl {
            if toggle {
                let on = !self.items[i].selected;
                self.mark(i, on, events);
            }
            self.anchor = Some(i);
        } else {
            for k in 0..self.items.len() {
                self.mark(k, k == i, events);
            }
            self.anchor = Some(i);
        }
        self.focus = Some(i);
        self.sync_index();
    }

    /// The mouse went down at (x, y) with the left button (`double`: the
    /// second of a double click).
    pub fn mouse_down(&mut self, x: i64, y: i64, shift: bool, ctrl: bool, double: bool) -> Vec<Event> {
        self.sync_header();
        let l = self.layout();
        self.scroll = self.scroll_of(&l);
        let mut events = Vec::new();
        self.pressed_items = false;
        self.after_double = false;
        self.pending_edit = None;
        self.clicks += 1;
        for bar in [l.vbar, l.hbar].into_iter().flatten() {
            if inside(bar.rect, x, y) {
                self.bar_press(&l, &bar, x, y);
                return events;
            }
        }
        if l.header_h > 0 && inside((l.view.0, l.inset, l.view.2, l.inset + l.header_h), x, y) {
            let actions = self.header.press(x - l.view.0 + self.scroll.0);
            if self.header.pressed.is_some() || !actions.is_empty() {
                self.drag = Some(Drag::Header);
            }
            return events;
        }
        if !inside(l.view, x, y) {
            return events;
        }
        self.pressed_items = true;
        match self.item_at(&l, x, y) {
            Some((i, true)) => {
                self.items[i].checked = !self.items[i].checked;
                events.push(Event::Change(i, CT_STATE));
            }
            Some((i, false)) => {
                // A click on the item already selected alone: its caption
                // is edited (ReadOnly False), as in Windows.
                let alone = self.items[i].selected && self.items.iter().filter(|it| it.selected).count() == 1;
                if alone && !double && !shift && !ctrl && !self.read_only && self.focused {
                    self.pending_edit = Some(i);
                }
                // (a double click doesn't undo what its first click did)
                if !double {
                    self.pick(i, shift, ctrl, true, &mut events);
                }
            }
            None if !ctrl => {
                for k in 0..self.items.len() {
                    self.mark(k, false, &mut events);
                }
                self.sync_index();
            }
            None => {}
        }
        if double {
            events.push(Event::DblClick);
            self.after_double = true;
        }
        events
    }

    /// The mouse moved to (x, y) (a button held or not); `true` when the
    /// control looks different.
    pub fn mouse_move(&mut self, x: i64, y: i64) -> bool {
        let l = self.layout();
        match self.drag.clone() {
            Some(Drag::Header) => {
                let before: Vec<i64> = self.header.sections.iter().map(|s| s.width).collect();
                self.header.drag_to(x - l.view.0 + self.scroll.0);
                for (c, s) in self.columns.iter_mut().zip(&self.header.sections) {
                    c.width = s.width;
                }
                return before != self.header.sections.iter().map(|s| s.width).collect::<Vec<_>>();
            }
            Some(Drag::Thumb { vertical, from, start }) => {
                let Some(bar) = (if vertical { l.vbar } else { l.hbar }) else { return false };
                let track = (bar.len() - 2 * BAR).max(1);
                let (t0, t1) = bar.thumb();
                let room = (track - (t1 - t0)).max(1);
                let moved = (if vertical { y } else { x }) - from;
                let pos = start + moved * (bar.content - bar.view).max(0) / room;
                let before = self.scroll;
                if vertical { self.scroll.1 = pos } else { self.scroll.0 = pos }
                self.scroll = self.scroll_of(&l);
                return before != self.scroll;
            }
            Some(Drag::Bar) => return false,
            None => {}
        }
        if !self.hot_track {
            return false;
        }
        let hot = self.item_at(&l, x, y).map(|(i, _)| i);
        std::mem::replace(&mut self.hot, hot) != hot
    }

    /// The mouse went up at (x, y).
    pub fn mouse_up(&mut self, x: i64, _y: i64) -> Vec<Event> {
        let l = self.layout();
        let mut events = Vec::new();
        match self.drag.take() {
            Some(Drag::Header) => {
                for a in self.header.release(x - l.view.0 + self.scroll.0) {
                    if let header::Action::Click(i) = a {
                        if self.column_click {
                            events.push(Event::ColumnClick(i));
                        }
                    }
                }
                return events;
            }
            Some(_) => return events,
            None => {}
        }
        if std::mem::take(&mut self.pressed_items) && !std::mem::take(&mut self.after_double) {
            events.push(Event::Click);
            if let Some(i) = self.pending_edit.take() {
                events.push(Event::EditSoon(i));
            }
        }
        events
    }

    /// Where item `i`'s caption is edited (in the control), if it shows.
    pub fn editor_rect(&mut self, i: usize) -> Option<(i64, i64, i64, i64)> {
        if i >= self.items.len() {
            return None;
        }
        self.show_item(i);
        let l = self.layout();
        let p = self.parts(&l, i);
        let right = match self.view_style {
            VS_REPORT => p.cell.0 + self.columns.first().map_or(0, |c| c.width.max(0)),
            VS_ICON => p.cell.2,
            _ => p.cell.2.max(p.label.0 + 80),
        };
        let (left, top) = if self.view_style == VS_ICON { (p.cell.0 + 2, p.label.1) } else { (p.label.0, p.cell.1) };
        let height = l.text_h + 4;
        Some((left, top, (right - 1).max(left + 40).min(l.view.2), top + height))
    }

    /// The user edited item `i`'s caption to `text`.
    pub fn edited(&mut self, i: usize, text: String) -> Vec<Event> {
        let Some(item) = self.items.get_mut(i) else { return Vec::new() };
        if item.caption == text {
            return Vec::new();
        }
        item.caption = text;
        self.resort();
        // (sorted, it may have moved: its index is now where it is)
        let at = if matches!(self.sort_type, ST_TEXT | ST_BOTH) { self.focus.unwrap_or(i) } else { i };
        vec![Event::Change(at, CT_TEXT)]
    }

    /// Whether the mouse is held on the control (the runtime passes its
    /// moves and release on wherever they are).
    pub fn held(&self) -> bool {
        self.drag.is_some() || self.pressed_items
    }

    /// The mouse left the control.
    pub fn mouse_leave(&mut self) -> bool {
        self.hot.take().is_some()
    }

    /// Whether (x, y) is on a header section's edge (the resize cursor).
    pub fn on_grip(&self, x: i64, y: i64) -> bool {
        let l = self.layout();
        if matches!(self.drag, Some(Drag::Header)) {
            return self.header.dragging();
        }
        l.header_h > 0 && inside((l.view.0, l.inset, l.view.2, l.inset + l.header_h), x, y) && self.header.on_grip(x - l.view.0 + self.scroll.0)
    }

    fn bar_press(&mut self, l: &Layout, bar: &Bar, x: i64, y: i64) {
        let line = if bar.vertical {
            if self.view_style == VS_REPORT { l.row_h } else { l.cell.1 }
        } else if self.view_style == VS_LIST {
            l.cell.0
        } else {
            16
        };
        let page = (bar.view - line).max(line);
        let step = match bar.part(x, y) {
            Part::Less => -line,
            Part::More => line,
            Part::PageLess => -page,
            Part::PageMore => page,
            Part::Thumb => {
                self.drag = Some(Drag::Thumb { vertical: bar.vertical, from: if bar.vertical { y } else { x }, start: bar.pos });
                return;
            }
        };
        self.drag = Some(Drag::Bar);
        if bar.vertical { self.scroll.1 += step } else { self.scroll.0 += step }
        self.scroll = self.scroll_of(l);
    }

    /// The mouse wheel turned `notches` (up: negative); `true` when it
    /// scrolled.
    pub fn wheel(&mut self, notches: i64) -> bool {
        let l = self.layout();
        let before = self.scroll;
        if self.view_style == VS_LIST {
            self.scroll.0 += notches * l.cell.0;
        } else {
            let line = if self.view_style == VS_REPORT { l.row_h } else { l.cell.1 / 2 };
            self.scroll.1 += notches * 3 * line;
        }
        self.scroll = self.scroll_of(&l);
        before != self.scroll
    }

    /// Scrolls so item `i` shows.
    fn show_item(&mut self, i: usize) {
        let l = self.layout();
        let p = self.parts(&l, i);
        let (vw, vh) = (l.view.2 - l.view.0, l.view.3 - l.view.1);
        if p.cell.1 < l.view.1 {
            self.scroll.1 -= l.view.1 - p.cell.1;
        } else if p.cell.3 > l.view.1 + vh {
            self.scroll.1 += p.cell.3 - (l.view.1 + vh);
        }
        if self.view_style != VS_REPORT {
            if p.cell.0 < l.view.0 {
                self.scroll.0 -= l.view.0 - p.cell.0;
            } else if p.cell.2 > l.view.0 + vw {
                self.scroll.0 += p.cell.2 - (l.view.0 + vw);
            }
        }
        self.scroll = self.scroll_of(&l);
    }

    /// A key went down (`vk`: its virtual key code); the events it caused,
    /// and whether the control looks different.
    pub fn key_down(&mut self, vk: i64, shift: bool, ctrl: bool) -> (Vec<Event>, bool) {
        let n = self.items.len();
        let mut events = Vec::new();
        if n == 0 {
            return (events, false);
        }
        let l = self.layout();
        // (nothing focused yet: the arrows start at the first item)
        let fresh = self.focus.is_none_or(|f| f >= n);
        let at = self.focus.filter(|&f| f < n).unwrap_or(0) as i64;
        let per = l.per_line.max(1);
        let visible = match self.view_style {
            VS_REPORT => ((l.view.3 - l.view.1) / l.row_h.max(1)).max(1),
            VS_LIST => per,
            _ => ((l.view.3 - l.view.1) / l.cell.1.max(1)).max(1) * per,
        };
        let (across, down) = match self.view_style {
            VS_REPORT => (0, 1),
            VS_LIST => (per, 1),
            _ => (1, per),
        };
        let target = match vk {
            38 => at - down,
            40 => at + down,
            37 if across > 0 => at - across,
            39 if across > 0 => at + across,
            33 => at - visible,
            34 => at + visible,
            36 => 0,
            35 => n as i64 - 1,
            32 => {
                if self.check_boxes && !ctrl {
                    let i = at as usize;
                    self.items[i].checked = !self.items[i].checked;
                    events.push(Event::Change(i, CT_STATE));
                } else {
                    self.pick(at as usize, shift, ctrl, true, &mut events);
                }
                return (events, true);
            }
            // F2: the focused item's caption edited (ReadOnly False).
            113 => {
                if !self.read_only {
                    self.show_item(at as usize);
                    events.push(Event::Edit(at as usize));
                }
                return (events, true);
            }
            // Ctrl+A: all of them (MultiSelect).
            65 if ctrl && self.multi_select => {
                for k in 0..n {
                    self.mark(k, true, &mut events);
                }
                self.sync_index();
                return (events, true);
            }
            _ => return (events, false),
        };
        let target = if fresh && matches!(vk, 37..=40) { 0 } else { target.clamp(0, n as i64 - 1) as usize };
        // (Ctrl moves only the focus when several can be selected)
        self.pick(target, shift, ctrl, false, &mut events);
        self.show_item(target);
        (events, true)
    }
}

/// A dotted frame (a focused item that isn't selected).
fn dotted(b: &mut Bitmap, (l, t, r, bot): Rect, c: u32) {
    for x in (l..r).step_by(2) {
        b.pset(x, t, c);
        b.pset(x, bot - 1, c);
    }
    for y in (t..bot).step_by(2) {
        b.pset(l, y, c);
        b.pset(r - 1, y, c);
    }
}

fn check_box(b: &mut Bitmap, (l, t, r, bot): Rect, checked: bool, c: &Colors) {
    b.fill_rect(l, t, r, bot, c.window);
    b.rectangle(l, t, r, bot, c.check_ink);
    if checked {
        for d in 0..2 {
            b.line(l + 3, t + 6 + d, l + 5, t + 8 + d, c.check_ink);
            b.line(l + 5, t + 8 + d, l + 9, t + 3 + d, c.check_ink);
        }
    }
}

fn scroll_bar(b: &mut Bitmap, bar: &Bar, held: bool, c: &Colors) {
    let (l, t, r, bot) = bar.rect;
    b.fill_rect(l, t, r, bot, c.bar_track);
    let (t0, t1) = bar.thumb();
    let thumb = if held { c.bar_thumb_held } else { c.bar_thumb };
    let arrow = c.bar_arrow;
    // The arrows: small triangles in the end buttons.
    let tri = |b: &mut Bitmap, cx: i64, cy: i64, dir: (i64, i64)| {
        for k in 0..4 {
            match dir {
                (0, -1) => b.line(cx - k, cy - 2 + k, cx + k, cy - 2 + k, arrow),
                (0, 1) => b.line(cx - k, cy + 2 - k, cx + k, cy + 2 - k, arrow),
                (-1, 0) => b.line(cx - 2 + k, cy - k, cx - 2 + k, cy + k, arrow),
                _ => b.line(cx + 2 - k, cy - k, cx + 2 - k, cy + k, arrow),
            }
        }
    };
    if bar.vertical {
        b.fill_rect(l + 2, t + t0, r - 2, t + t1, thumb);
        tri(b, l + BAR / 2, t + BAR / 2, (0, -1));
        tri(b, l + BAR / 2, bot - BAR / 2 - 1, (0, 1));
    } else {
        b.fill_rect(l + t0, t + 2, l + t1, bot - 2, thumb);
        tri(b, l + BAR / 2, t + BAR / 2, (-1, 0));
        tri(b, r - BAR / 2 - 1, t + BAR / 2, (1, 0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(x: &str) -> Value {
        v_str(x)
    }

    fn report() -> ListView {
        let mut lv = ListView::default();
        lv.set("viewstyle", &v_int(VS_REPORT));
        lv.call("addcolumns", &[s("Name"), s("Size")]);
        lv.call("column.width=", &[v_int(0), v_int(120)]);
        lv.call("additems", &[s("a.txt"), s("b.txt"), s("c.txt")]);
        lv.set_view(View { width: 300, height: 200, font: Font::default(), ..View::default() });
        lv
    }

    /// The middle of item `i`'s caption.
    fn at(lv: &ListView, i: usize) -> (i64, i64) {
        let l = lv.layout();
        let p = lv.parts(&l, i);
        (p.label.0 + 3, (p.label.1 + p.label.3) / 2)
    }

    #[test]
    fn default_metrics() {
        let lv = report();
        let l = lv.layout();
        eprintln!("METRICS header_h={} row_h={} text_h={} view={:?}", l.header_h, l.row_h, l.text_h, l.view);
        assert_eq!((l.header_h, l.row_h), (l.text_h + 6, l.text_h + 3));
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
        assert_eq!(lv.call("item.stateindex", &[v_int(0)]).unwrap().to_i64(), -1);
    }

    #[test]
    fn sorted_by_caption() {
        let mut lv = ListView::default();
        lv.call("additems", &[s("pear"), s("Apple"), s("fig")]);
        lv.set("itemindex", &v_int(0));
        lv.set("sorttype", &v_int(ST_TEXT));
        let names: Vec<String> = lv.items.iter().map(|i| i.caption.clone()).collect();
        assert_eq!(names, vec!["Apple", "fig", "pear"]);
        // The selection went with its item; new items take their place.
        assert_eq!(lv.item_index, 2);
        lv.call("additems", &[s("banana")]);
        assert_eq!(lv.items[1].caption, "banana");
        assert_eq!(lv.item_index, 3);
        lv.call("item.caption=", &[v_int(0), s("zucchini")]);
        assert_eq!(lv.items[3].caption, "zucchini");
    }

    #[test]
    fn clicks_select_and_multi_select() {
        let mut lv = report();
        let (x, y) = at(&lv, 1);
        assert_eq!(lv.mouse_down(x, y, false, false, false), vec![Event::Change(1, CT_STATE)]);
        assert_eq!(lv.mouse_up(x, y), vec![Event::Click]);
        assert_eq!(lv.item_index, 1);
        // Without MultiSelect, Ctrl doesn't add.
        let (x2, y2) = at(&lv, 2);
        lv.mouse_down(x2, y2, false, true, false);
        assert_eq!(lv.get("selcount").unwrap().to_i64(), 1);
        lv.set("multiselect", &v_int(-1));
        let (x0, y0) = at(&lv, 0);
        lv.mouse_down(x0, y0, false, true, false);
        assert_eq!(lv.get("selcount").unwrap().to_i64(), 2);
        assert_eq!(lv.item_index, 0);
        // Shift: the range from the anchor (item 0).
        lv.mouse_down(x2, y2, true, false, false);
        assert_eq!(lv.get("selcount").unwrap().to_i64(), 3);
        // A click on nothing: nothing selected.
        let l = lv.layout();
        lv.mouse_down(l.view.2 - 5, l.view.3 - 5, false, false, false);
        assert_eq!((lv.get("selcount").unwrap().to_i64(), lv.item_index), (0, -1));
        // A double click: OnDblClick, and no click after it.
        let ev = lv.mouse_down(x, y, false, false, true);
        assert_eq!(ev.last(), Some(&Event::DblClick));
        assert!(lv.mouse_up(x, y).is_empty());
    }

    #[test]
    fn row_select_and_sub_item_area() {
        let mut lv = report();
        // Past the caption, in the second column: nothing (Windows).
        let (_, y) = at(&lv, 1);
        lv.mouse_down(150, y, false, false, false);
        assert_eq!(lv.item_index, -1);
        lv.set("rowselect", &v_int(-1));
        lv.mouse_down(150, y, false, false, false);
        assert_eq!(lv.item_index, 1);
    }

    #[test]
    fn check_boxes_and_keys() {
        let mut lv = report();
        lv.set("checkboxes", &v_int(-1));
        let l = lv.layout();
        let c = lv.parts(&l, 2).check.unwrap();
        assert_eq!(lv.mouse_down(c.0 + 3, c.1 + 3, false, false, false), vec![Event::Change(2, CT_STATE)]);
        assert!(lv.items[2].checked);
        assert_eq!(lv.item_index, -1, "a check box doesn't select");
        let (ev, _) = lv.key_down(40, false, false);
        assert_eq!(ev, vec![Event::Change(0, CT_STATE)]);
        lv.key_down(35, false, false);
        assert_eq!(lv.item_index, 2);
        lv.key_down(32, false, false);
        assert!(!lv.items[2].checked);
        lv.key_down(36, false, false);
        assert_eq!(lv.item_index, 0);
    }

    #[test]
    fn captions_edited() {
        let mut lv = report();
        lv.focused = true;
        let (x, y) = at(&lv, 1);
        lv.mouse_down(x, y, false, false, false);
        assert_eq!(lv.mouse_up(x, y), vec![Event::Click], "the first click selects");
        lv.mouse_down(x, y, false, false, false);
        assert_eq!(lv.mouse_up(x, y), vec![Event::Click, Event::EditSoon(1)]);
        assert!(lv.editor_rect(1).is_some());
        assert_eq!(lv.edited(1, "renamed".into()), vec![Event::Change(1, CT_TEXT)]);
        assert_eq!(lv.items[1].caption, "renamed");
        assert_eq!(lv.key_down(113, false, false).0, vec![Event::Edit(1)]);
        lv.set("readonly", &v_int(-1));
        assert!(lv.key_down(113, false, false).0.is_empty());
        lv.mouse_down(x, y, false, false, false);
        assert_eq!(lv.mouse_up(x, y), vec![Event::Click]);
    }

    #[test]
    fn header_clicks_and_resizes() {
        let mut lv = report();
        let l = lv.layout();
        let hy = l.inset + l.header_h / 2;
        lv.mouse_down(40, hy, false, false, false);
        assert_eq!(lv.mouse_up(40, hy), vec![Event::ColumnClick(0)]);
        lv.mouse_down(150, hy, false, false, false);
        assert_eq!(lv.mouse_up(150, hy), vec![Event::ColumnClick(1)]);
        // Column 0's edge (x = 1 + 120) dragged 30 to the right.
        let edge = l.view.0 + 120;
        assert!(lv.on_grip(edge, hy));
        lv.mouse_down(edge, hy, false, false, false);
        assert!(lv.mouse_move(edge + 30, hy));
        assert!(lv.mouse_up(edge + 30, hy).is_empty());
        assert_eq!(lv.columns[0].width, 150);
        lv.set("columnclick", &v_int(0));
        lv.mouse_down(40, hy, false, false, false);
        assert!(lv.mouse_up(40, hy).is_empty());
    }

    #[test]
    fn scrolls_and_views() {
        let mut lv = report();
        for k in 0..40 {
            lv.call("additems", &[s(&format!("item {k}"))]);
        }
        let l = lv.layout();
        assert!(l.vbar.is_some() && l.hbar.is_none());
        assert!(lv.wheel(1));
        assert_eq!(lv.scroll.1, 3 * l.row_h);
        lv.key_down(35, false, false);
        let l = lv.layout();
        assert_eq!(lv.scroll.1, l.content.1 - (l.view.3 - l.view.1));
        // The icon view: a grid of cells, no sideways bar.
        lv.set("viewstyle", &v_int(VS_ICON));
        let l = lv.layout();
        assert!(l.per_line >= 3 && l.hbar.is_none());
        let p0 = lv.parts(&l, 0);
        let p1 = lv.parts(&l, 1);
        assert_eq!(p0.cell.1, p1.cell.1);
        assert!(p1.cell.0 > p0.cell.0);
        // The list view: columns, a sideways bar.
        for k in 40..120 {
            lv.call("additems", &[s(&format!("item {k}"))]);
        }
        lv.set("viewstyle", &v_int(VS_LIST));
        let l = lv.layout();
        assert!(l.hbar.is_some() && l.vbar.is_none());
        let p1 = lv.parts(&l, 1);
        let p0 = lv.parts(&l, 0);
        assert_eq!(p0.cell.0, p1.cell.0);
        assert!(p1.cell.1 > p0.cell.1);
    }

    #[test]
    fn paints_selection_and_check_marks() {
        let mut lv = report();
        lv.set("checkboxes", &v_int(-1));
        lv.call("item.checked=", &[v_int(0), v_int(-1)]);
        lv.set("itemindex", &v_int(1));
        lv.focused = true;
        let b = lv.paint(WINDOW, &Images::default());
        let l = lv.layout();
        let p = lv.parts(&l, 1);
        assert_eq!(b.pixel(p.label.0 + 1, p.label.1 + 1), Some(HIGHLIGHT));
        let c = lv.parts(&l, 0).check.unwrap();
        assert_eq!(b.pixel(c.0, c.1), Some(CHECK_INK));
        assert_eq!(b.pixel(c.0 + 4, c.1 + 7), Some(CHECK_INK), "the check mark");
        assert_eq!(b.pixel(0, 0), Some(BORDER));
        // Not focused: gray; HideSelection: none.
        lv.focused = false;
        let b = lv.paint(WINDOW, &Images::default());
        assert_eq!(b.pixel(p.label.0 + 1, p.label.1 + 1), Some(INACTIVE));
        lv.set("hideselection", &v_int(-1));
        let b = lv.paint(WINDOW, &Images::default());
        assert_eq!(b.pixel(p.label.0 + 1, p.label.1 + 1), Some(WINDOW));
    }
}

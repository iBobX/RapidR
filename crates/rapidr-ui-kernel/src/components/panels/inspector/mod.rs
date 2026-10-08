//! RPROPERTYINSPECTOR drawn and driven by the kernel; its model is
//! `rapidr_value::panels::inspector` (the web draws the same).
//!
//! The mouse: a row's name selects it (its expander opens it), its value
//! starts typing (a Boolean's box toggles, the button drops a list or asks
//! for the program's editor "…"), the line between the columns drags, the
//! reset glyph puts a changed value back; the anchors editor's pins and
//! the colour picker's swatches are clicked. The keyboard: Up / Down, Page
//! Up / Down, Home / End; Left / Right close and open (an enum's: the
//! previous / next constant); Enter or F2 types a value (Enter keeps it,
//! Escape drops it, Tab goes on to the next row's, as Delphi's); Space
//! toggles; Alt+Down (F4) drops a list; Delete puts a value back;
//! Ctrl+F searches; letters go to a property by its name.

pub mod draw;
pub mod geo;

use std::cell::RefCell;
use std::collections::HashMap;

use rapidr_value::objects::a11y::{part_id, AccessNode, Action, Role, PART_EDITOR, PART_ITEM, PART_ROW, PART_TAB};
use rapidr_value::objects::ops::Rect;
use rapidr_value::panels::inspector::values::{self, Kind};
use rapidr_value::panels::inspector::{self as model, Hover, Inspector as Model, Row, RowKind, User as Act};
use rapidr_value::panels::rows::{self as nav, vk, ROW};
use rapidr_value::panels::User;

use self::geo::{Geo, Part};
use super::common::{self, look};
use super::send;
use crate::a11y::AccessValue;
use crate::components::edit::{self, Source};
use crate::components::list::{self, InPlace};
use crate::components::{combo, ComponentKind, Cx, Ime, KeyIn, MouseIn, MouseKind, MouseOut};
use crate::input::{Clipboard, KernelEvent};
use crate::paint::Painter;
use crate::store::Store;

pub struct Inspector;

/// The search box's edit (its key among the rows').
const SEARCH: &str = "?search";
/// The anchors' flags, pin by pin.
const FLAGS: [&str; 4] = ["akLeft", "akTop", "akRight", "akBottom"];
/// The pins in the keyboard's order: top, left, right, bottom.
const PIN_ORDER: [usize; 4] = [1, 0, 2, 3];
/// Accessibility parts past the rows'.
const PART_TABS: usize = 2_000_000;
const PART_SEARCH: usize = 3_000_000;
const PART_VIEW: usize = 3_000_001;
const PART_PINS: usize = 4_000_000;
const PART_SWATCHES: usize = 5_000_000;
const PART_EDIT: usize = 6_000_000;

thread_local! {
    /// Each inspector's in-place edit: the row it types into (SEARCH: the
    /// search box).
    static EDITING: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
    /// Type-ahead: what was typed lately, and when.
    static TYPED: RefCell<HashMap<String, (String, crate::tick::Instant)>> = RefCell::new(HashMap::new());
}

/// What the input and drawing work from: the model, its rows and the
/// places of things now.
struct V {
    m: Model,
    rows: Vec<Row>,
    g: Geo,
    pos: i64,
    /// The rows' width (beside the scroll bar).
    cw: i64,
    /// The names' column's.
    nw: i64,
}

fn classic() -> bool {
    look(rapidr_value::theme::current()).classic
}

fn view(cx: &Cx, classic: bool) -> V {
    let m = model::with(cx.id, Clone::clone).unwrap_or_default();
    let rows = m.rows();
    let g = geo::geo(&m, cx.width(), cx.height(), if classic { 2 } else { 0 });
    // (the scroller told the rows' height now: input may come before the
    // next drawing, and must find what it will show)
    let (pos, cw, _) = list::vscroll(cx.id, g.list.2, g.list.3, geo::content(&rows), ROW);
    let nw = geo::name_width(&m, cw);
    V { m, rows, g, pos, cw, nw }
}

impl V {
    fn row_rect(&self, i: usize) -> Rect {
        geo::row_rect(&self.g, &self.rows, i, self.pos, self.cw)
    }

    fn row_at(&self, x: i64, y: i64) -> Option<(usize, Rect)> {
        let (lx, ly, _, lh) = self.g.list;
        if x < lx || x >= lx + self.cw || y < ly || y >= ly + lh {
            return None;
        }
        let mut top = ly - self.pos;
        for (i, r) in self.rows.iter().enumerate() {
            if y >= top && y < top + r.height {
                return Some((i, (lx, top, self.cw, r.height)));
            }
            top += r.height;
        }
        None
    }

    /// Row `i`'s place in the content, with its opened parts below it (as
    /// much as the view shows): what scrolling to it shows.
    fn span(&self, i: usize) -> (i64, i64) {
        let top: i64 = self.rows[..i].iter().map(|r| r.height).sum();
        let level = self.rows[i].level;
        let parts: i64 = self.rows[i + 1..].iter().take_while(|r| r.level > level && self.rows[i].kind != RowKind::Category).map(|r| r.height).sum();
        (top, top + (self.rows[i].height + parts).min(self.g.list.3.max(ROW)))
    }

    fn index(&self, key: &str) -> Option<usize> {
        self.rows.iter().position(|r| r.key == key)
    }

    fn selected(&self) -> Option<usize> {
        self.m.selected_in(&self.rows)
    }

    /// The search box's editor rectangle (its text area inside a frame).
    fn search_rect(&self) -> Rect {
        let (x, y, w, h) = self.g.search;
        let s = (h - 8).clamp(10, 16);
        let area = (x + 10 + s, y + 1, (w - 14 - s).max(0), (h - 2).max(0));
        (area.0 - 3, area.1 - 1, area.2 + 6, area.3 + 2)
    }

    /// Row `i`'s value editor rectangle.
    fn value_rect(&self, i: usize) -> Rect {
        let rr = self.row_rect(i);
        let r = &self.rows[i];
        if r.kind == RowKind::Picker {
            return geo::picker(rr).field;
        }
        let parts = geo::parts(&self.m, r, rr, self.nw);
        let (vx, y, vw, h) = parts.value;
        let right = parts.button.map_or(vx + vw - 2, |b| b.0 - 2);
        (vx + 3, y + 1, (right - vx - 3).max(8), h - 2)
    }
}

/// The key of the row (or search box) inspector `id` types into, if it
/// does.
fn editing_key(id: &str) -> Option<String> {
    list::editing(id)?;
    EDITING.with(|e| e.borrow().get(&id.to_lowercase()).cloned())
}

/// The program's model changed by the user's input (pure UI state).
fn ui(cx: &Cx, f: impl FnOnce(&mut Model)) {
    model::with_mut(cx.id, f);
}

/// Selects row `key` (scrolled into view): OnSelect when it changed.
fn select(cx: &mut Cx, key: &str) {
    reveal(cx, key);
    let changed = model::with_mut(cx.id, |m| {
        let rows = m.rows();
        let name = |k: Option<&str>| k.and_then(|k| rows.iter().find(|r| r.key == k)).map(|r| r.prop_name(&m.snap.props, &m.snap.events));
        // (OnSelect when another property is selected: not for its parts'
        // editors — the anchors' pins, a colour's picker)
        let changed = name(m.ui.selected.as_deref()) != name(Some(key));
        if m.ui.selected.as_deref() != Some(key) {
            m.ui.error = None;
        }
        m.ui.selected = Some(key.to_string());
        changed
    });
    if changed {
        send(cx, User::Inspector(Act::Select(key.to_string())));
    }
}

/// Scrolls row `key` into the list's view.
fn reveal(cx: &Cx, key: &str) {
    let v = view(cx, classic());
    if let Some(i) = v.index(key) {
        let (top, bottom) = v.span(i);
        list::scroll_into_view(cx.id, top, bottom, v.g.list.3);
    }
}

/// Checks `text` for row `key`, then sends it to the program; `false`
/// (and the row says why) when it can't be taken.
fn commit(cx: &mut Cx, key: &str, text: &str) -> bool {
    let checked = model::with(cx.id, |m| m.interpret(key, text).map(|_| ())).unwrap_or(Err(String::new()));
    match checked {
        Ok(()) => {
            ui(cx, |m| m.ui.error = None);
            send(cx, User::Inspector(Act::Commit { key: key.to_string(), text: text.to_string() }));
            true
        }
        Err(e) => {
            ui(cx, |m| m.ui.error = Some((key.to_string(), e)));
            false
        }
    }
}

/// Starts typing into row `key`'s value (or the search box): whether it
/// can be typed.
fn begin(cx: &mut Cx, v: &V, key: &str) -> bool {
    if key == SEARCH {
        list::begin_edit(cx.id, InPlace { target: (0, 0), text: v.m.filter.clone(), rect: None });
        EDITING.with(|e| e.borrow_mut().insert(cx.id.to_lowercase(), SEARCH.into()));
        return true;
    }
    if v.m.read_only {
        return false;
    }
    let Some(i) = v.index(key) else { return false };
    let r = &v.rows[i];
    let typed = match r.kind {
        RowKind::Category | RowKind::Pins => false,
        RowKind::Picker | RowKind::Event | RowKind::Line(_) | RowKind::AddLine => true,
        _ => geo::row_kind(&v.m, r).is_some_and(|k| k.typed()),
    };
    if !typed {
        return false;
    }
    let text = match r.kind {
        RowKind::AddLine => String::new(),
        RowKind::Line(_) => r.text.clone(),
        RowKind::Picker => v.m.value_text(r.key.split('#').next().unwrap_or("")).unwrap_or_default(),
        _ if r.mixed => String::new(),
        _ => r.text.clone(),
    };
    list::begin_edit(cx.id, InPlace { target: (1, i), text, rect: None });
    EDITING.with(|e| e.borrow_mut().insert(cx.id.to_lowercase(), key.to_string()));
    ui(cx, |m| m.ui.error = None);
    true
}

/// Ends the edit going on: kept (checked first: a refused value keeps the
/// editor open) or dropped. Whether it ended.
fn finish(cx: &mut Cx, keep: bool) -> bool {
    let Some(key) = editing_key(cx.id) else { return true };
    let text = list::editing(cx.id).map(|e| e.text).unwrap_or_default();
    if key == SEARCH {
        list::end_edit(cx.id);
        EDITING.with(|e| e.borrow_mut().remove(&cx.id.to_lowercase()));
        if !keep {
            ui(cx, |m| m.filter.clear());
        }
        return true;
    }
    if keep {
        // (an unchanged value isn't a change)
        let same = model::with(cx.id, |m| {
            let rows = m.rows();
            rows.iter().find(|r| r.key == key).is_some_and(|r| !r.mixed && r.text == text && !matches!(r.kind, RowKind::AddLine | RowKind::Picker))
        })
        .unwrap_or(false);
        if !same && !commit(cx, &key, &text) {
            return false;
        }
    } else {
        ui(cx, |m| m.ui.error = None);
    }
    list::end_edit(cx.id);
    EDITING.with(|e| e.borrow_mut().remove(&cx.id.to_lowercase()));
    true
}

/// The edit's rectangle (in the component), if one goes on.
fn edit_rect(cx: &Cx) -> Option<Rect> {
    let key = editing_key(cx.id)?;
    let v = view(cx, classic());
    if key == SEARCH {
        return Some(v.search_rect());
    }
    Some(v.value_rect(v.index(&key)?))
}

/// A Boolean row's value turned over.
fn toggle(cx: &mut Cx, r: &Row) {
    if r.mixed || r.text != "True" {
        commit(cx, &r.key, "True");
    } else {
        commit(cx, &r.key, "False");
    }
}

/// Anchor pin `k` of the Anchors row `r` turned over.
fn toggle_pin(cx: &mut Cx, v: &V, r: &Row, k: usize) {
    let Some(p) = v.m.snap.props.get(r.prop) else { return };
    let on = p.value.as_ref().is_some_and(|x| x.to_i64() & (1 << k) != 0);
    commit(cx, &format!("{}.{}", p.key(), FLAGS[k].to_ascii_lowercase()), if on { "False" } else { "True" });
}

/// The colour picker row `r`'s swatch `k` picked.
fn pick_swatch(cx: &mut Cx, r: &Row, k: usize) {
    let base = r.key.split('#').next().unwrap_or("").to_string();
    commit(cx, &base, geo::swatch_name(k));
}

/// An enum row's next (or previous) constant.
fn cycle(cx: &mut Cx, v: &V, r: &Row, step: i64) -> bool {
    let Some(Kind::Enum(values)) = geo::row_kind(&v.m, r) else { return false };
    if v.m.read_only || values.is_empty() {
        return false;
    }
    let at = values.iter().position(|n| n.eq_ignore_ascii_case(&r.text)).map_or(0, |i| i as i64 + step);
    let n = values.len() as i64;
    commit(cx, &r.key, &values[at.rem_euclid(n) as usize]);
    true
}

/// Row `i`'s list dropped (or its picker opened, or the program asked for
/// its editor).
fn drop(cx: &mut Cx, v: &V, i: usize) {
    let r = &v.rows[i];
    let Some(kind) = geo::row_kind(&v.m, r) else { return };
    if v.m.read_only {
        return;
    }
    if matches!(kind, Kind::Color) && r.kind == RowKind::Prop {
        let (key, open) = (r.key.clone(), !r.expanded);
        ui(cx, |m| m.set_open(&key, open));
        return;
    }
    if r.kind == RowKind::Event || kind.drops() {
        let rr = v.row_rect(i);
        let parts = geo::parts(&v.m, r, rr, v.nw);
        let (vx, y, vw, h) = parts.value;
        let anchor = (cx.rect.0 + vx, cx.rect.1 + y, vw, h);
        send(cx, User::Inspector(Act::Drop { key: r.key.clone(), anchor }));
        return;
    }
    if kind.ellipsis() {
        let name = r.prop_name(&v.m.snap.props, &v.m.snap.events);
        send(cx, User::Inspector(Act::EditorRequest(name)));
    }
}

/// What Enter (or a double click) does on row `i`.
fn activate(cx: &mut Cx, v: &V, i: usize) {
    let r = v.rows[i].clone();
    match r.kind {
        RowKind::Category => {
            let open = !r.expanded;
            ui(cx, |m| m.set_open(&r.key, open));
        }
        RowKind::Pins => toggle_pin(cx, v, &r, v.m.ui.pin),
        RowKind::Picker => pick_swatch(cx, &r, v.m.ui.swatch),
        RowKind::Event if r.text.is_empty() => send(cx, User::Inspector(Act::EventDblClick(r.name.clone()))),
        _ => {
            let kind = geo::row_kind(&v.m, &r);
            match kind {
                Some(Kind::Bool) => toggle(cx, &r),
                Some(Kind::Font | Kind::Strings | Kind::Columns) if r.kind == RowKind::Prop => {
                    let open = !r.expanded;
                    ui(cx, |m| m.set_open(&r.key, open));
                }
                _ => {
                    begin(cx, v, &r.key);
                }
            }
        }
    }
}

/// Moves the selection to row `to` (from `from`, for the editors' parts
/// entered from above or below).
fn go(cx: &mut Cx, v: &V, to: usize, from: Option<usize>) {
    let r = &v.rows[to];
    let down = from.is_none_or(|f| f < to);
    match r.kind {
        RowKind::Pins => ui(cx, |m| m.ui.pin = if down { 1 } else { 3 }),
        RowKind::Picker => {
            let cur = v.m.snap.props.get(r.prop).and_then(|p| p.value.as_ref()).map(|x| x.to_i64());
            let at = (0..geo::swatch_count()).find(|&k| values::color_value(geo::swatch_name(k)) == cur);
            ui(cx, |m| m.ui.swatch = at.unwrap_or(if down { 0 } else { geo::swatch_count() - 1 }));
        }
        _ => {}
    }
    let key = r.key.clone();
    select(cx, &key);
}

impl Inspector {
    /// A key on the rows (no edit going on).
    fn row_key(cx: &mut Cx, v: &V, k: &KeyIn) -> bool {
        let count = v.rows.len();
        let at = v.selected();
        let page = ((v.g.list.3 / ROW) - 1).max(1) as usize;
        let cur = at.map(|i| v.rows[i].clone());
        // (inside the anchors editor and the colour picker, the arrows
        // move among their parts first)
        if let Some(r) = &cur {
            match (r.kind, k.vk) {
                (RowKind::Pins, vk::UP | vk::DOWN | vk::LEFT | vk::RIGHT) => {
                    let o = PIN_ORDER.iter().position(|&p| p == v.m.ui.pin).unwrap_or(0);
                    let next = match k.vk {
                        vk::UP | vk::LEFT => o.checked_sub(1),
                        _ => (o + 1 < 4).then_some(o + 1),
                    };
                    match next {
                        Some(n) => {
                            ui(cx, |m| m.ui.pin = PIN_ORDER[n]);
                            return true;
                        }
                        None if matches!(k.vk, vk::LEFT | vk::RIGHT) => return true,
                        None => {}
                    }
                }
                (RowKind::Picker, vk::UP | vk::DOWN | vk::LEFT | vk::RIGHT) => {
                    let n = geo::swatch_count() as i64;
                    let s = v.m.ui.swatch as i64;
                    let cols = geo::COLUMNS as i64;
                    // (the standard grid's two rows, then the system's three)
                    let next = match k.vk {
                        vk::LEFT => (s - 1).max(0),
                        vk::RIGHT => (s + 1).min(n - 1),
                        vk::UP => s - cols,
                        _ => s + cols,
                    };
                    if (0..n).contains(&next) {
                        ui(cx, |m| m.ui.swatch = next as usize);
                        return true;
                    }
                    if matches!(k.vk, vk::LEFT | vk::RIGHT) {
                        return true;
                    }
                }
                (RowKind::Pins, vk::SPACE | vk::ENTER) => {
                    toggle_pin(cx, v, r, v.m.ui.pin);
                    return true;
                }
                (RowKind::Picker, vk::SPACE | vk::ENTER) => {
                    pick_swatch(cx, r, v.m.ui.swatch);
                    return true;
                }
                _ => {}
            }
        }
        // (Alt+Down / Up drops a list, as a combo box's)
        if k.mods.alt && matches!(k.vk, vk::UP | vk::DOWN) {
            if let Some(i) = at {
                drop(cx, v, i);
            }
            return true;
        }
        if let Some(to) = nav::nav(k.vk, at, count, page) {
            forget_typed(cx);
            if Some(to) != at {
                go(cx, v, to, at);
            }
            return true;
        }
        let Some(i) = at else {
            return Self::type_ahead(cx, v, k, None);
        };
        let r = v.rows[i].clone();
        let kind = geo::row_kind(&v.m, &r);
        let plus = k.text == "+" || k.vk == 107;
        let minus = k.text == "-" || k.vk == 109;
        match k.vk {
            vk::F4 => drop(cx, v, i),
            vk::ENTER if k.mods.ctrl && kind.as_ref().is_some_and(Kind::ellipsis) => {
                send(cx, User::Inspector(Act::EditorRequest(r.prop_name(&v.m.snap.props, &v.m.snap.events))));
            }
            vk::ENTER => activate(cx, v, i),
            vk::F2 => {
                begin(cx, v, &r.key);
            }
            vk::SPACE => match (r.kind, &kind) {
                (RowKind::Category, _) => activate(cx, v, i),
                (_, Some(Kind::Bool)) if !v.m.read_only => toggle(cx, &r),
                _ => return Self::type_ahead(cx, v, k, Some(i)),
            },
            vk::LEFT | vk::RIGHT if matches!(kind, Some(Kind::Enum(_))) && r.kind != RowKind::Category => {
                cycle(cx, v, &r, if k.vk == vk::RIGHT { 1 } else { -1 });
            }
            _ if (plus || minus) && matches!(kind, Some(Kind::Enum(_))) => {
                cycle(cx, v, &r, if plus { 1 } else { -1 });
            }
            vk::LEFT => {
                if r.expandable && r.expanded {
                    ui(cx, |m| m.set_open(&r.key, false));
                } else if let Some(parent) = (0..i).rev().find(|&j| v.rows[j].level < r.level || (r.level == 0 && v.rows[j].kind == RowKind::Category && !v.m.alphabetic && r.kind != RowKind::Category)) {
                    go(cx, v, parent, Some(i));
                }
            }
            vk::RIGHT => {
                if r.expandable && !r.expanded {
                    ui(cx, |m| m.set_open(&r.key, true));
                } else if r.expandable && i + 1 < count {
                    go(cx, v, i + 1, Some(i));
                }
            }
            _ if plus && r.expandable => ui(cx, |m| m.set_open(&r.key, true)),
            _ if minus && r.expandable => ui(cx, |m| m.set_open(&r.key, false)),
            vk::DELETE | vk::BACK => match r.kind {
                RowKind::Line(n) if !v.m.read_only => {
                    let base = r.key.split('#').next().unwrap_or("").to_string();
                    let text = v.m.snap.props.get(r.prop).and_then(|p| p.value.as_ref()).map(|x| x.to_string_val()).unwrap_or_default();
                    let mut lines = values::lines(&text);
                    if n < lines.len() {
                        lines.remove(n);
                    }
                    commit(cx, &base, &lines.join("\r\n"));
                }
                _ if geo::resettable(&r) && !r.is_default && !r.mixed && !v.m.read_only => send(cx, User::Inspector(Act::Reset(r.key.clone()))),
                _ => {}
            },
            vk::ESCAPE if r.kind == RowKind::Picker => {
                let base = r.key.split('#').next().unwrap_or("").to_string();
                ui(cx, |m| m.set_open(&base, false));
                select(cx, &base);
            }
            _ => return Self::type_ahead(cx, v, k, Some(i)),
        }
        forget_typed(cx);
        true
    }

    /// Letters typed on the rows: the next row whose name starts with
    /// them (what was typed within the last second counts).
    fn type_ahead(cx: &mut Cx, v: &V, k: &KeyIn, at: Option<usize>) -> bool {
        let c = k.text;
        if c.is_empty() || k.mods.ctrl || k.mods.alt || k.mods.command || c.chars().any(|ch| ch.is_control() || ch == ' ') {
            return false;
        }
        let now = crate::tick::now();
        let typed = TYPED.with(|t| {
            let mut t = t.borrow_mut();
            let e = t.entry(cx.id.to_lowercase()).or_insert((String::new(), now));
            if now.duration_since(e.1) > std::time::Duration::from_millis(1000) {
                e.0.clear();
            }
            e.0.push_str(c);
            e.1 = now;
            e.0.clone()
        });
        let from = match at {
            Some(i) if typed.chars().count() > 1 => i,
            Some(i) => i + 1,
            None => 0,
        };
        if v.rows.is_empty() {
            return true;
        }
        if let Some(j) = nav::type_ahead(v.rows.iter().map(|r| r.name.as_str()), from % v.rows.len(), &typed) {
            go(cx, v, j, at);
        }
        true
    }

    /// The keyboard while a value (or the search) is typed.
    fn edit_key(cx: &mut Cx, v: &V, key: &str, k: &KeyIn, clip: &mut dyn Clipboard) -> bool {
        if key == SEARCH {
            let rect = v.search_rect();
            match k.vk {
                vk::TAB => return false,
                vk::ENTER | vk::DOWN => {
                    finish(cx, true);
                    let v = view(cx, classic());
                    if v.selected().is_none() {
                        if let Some(first) = v.rows.iter().position(|r| r.kind != RowKind::Category).or((!v.rows.is_empty()).then_some(0)) {
                            go(cx, &v, first, None);
                        }
                    }
                }
                vk::ESCAPE => {
                    finish(cx, false);
                }
                _ => {
                    list::edit_key(cx, k, clip, rect);
                    let text = list::editing(cx.id).map(|e| e.text).unwrap_or_default();
                    ui(cx, |m| m.filter = text);
                }
            }
            return true;
        }
        let Some(i) = v.index(key) else {
            finish(cx, false);
            return true;
        };
        let r = v.rows[i].clone();
        let kind = geo::row_kind(&v.m, &r);
        match k.vk {
            vk::TAB => {
                if finish(cx, true) {
                    // (Delphi's: on to the next (Shift: previous) row that types)
                    let step: i64 = if k.mods.shift { -1 } else { 1 };
                    let mut j = i as i64 + step;
                    while j >= 0 && (j as usize) < v.rows.len() {
                        let next = &v.rows[j as usize];
                        if next.kind != RowKind::Category && geo::row_kind(&v.m, next).is_some_and(|k| k.typed()) {
                            let key = next.key.clone();
                            select(cx, &key);
                            begin(cx, v, &key);
                            break;
                        }
                        j += step;
                    }
                }
            }
            vk::UP | vk::DOWN if k.mods.alt => {
                finish(cx, false);
                drop(cx, v, i);
            }
            vk::UP | vk::DOWN if matches!(kind, Some(Kind::Int | Kind::Float)) => {
                let text = list::editing(cx.id).map(|e| e.text).unwrap_or_default();
                let step = if k.vk == vk::UP { 1.0 } else { -1.0 };
                let next = match kind {
                    Some(Kind::Int) => values::parse_int(&text).map(|n| (n + step as i64).to_string()),
                    _ => text.trim().parse::<f64>().ok().map(|f| values::float_text(f + step)),
                };
                if let Some(n) = next {
                    list::set_edit_text(cx.id, &n);
                }
            }
            _ => match list::edit_key(cx, k, clip, v.value_rect(i)) {
                Some(Some(keep)) => {
                    finish(cx, keep);
                }
                Some(None) if model::with(cx.id, |m| m.ui.error.is_some()).unwrap_or(false) => ui(cx, |m| m.ui.error = None),
                Some(None) => {}
                None => {}
            },
        }
        true
    }

    /// A press at (x, y).
    fn press(cx: &mut Cx, m: &MouseIn, x: i64, y: i64) -> MouseOut {
        forget_typed(cx);
        let v = view(cx, classic());
        // (the tabs, the search box, the view's button)
        if let Some(t) = v.g.tabs {
            if common::inside(t, x, y) {
                if let Some(i) = tab_at(cx, &v, x) {
                    finish(cx, true);
                    ui(cx, |m| {
                        m.events_page = i == 1;
                        m.ui.error = None;
                    });
                }
                return MouseOut::default();
            }
        }
        if common::inside(v.g.search, x, y) {
            if editing_key(cx.id).as_deref() != Some(SEARCH) {
                finish(cx, true);
                begin(cx, &v, SEARCH);
            }
            list::editor_mouse(cx, m, v.search_rect());
            return MouseOut::default();
        }
        if common::inside(v.g.view_button, x, y) {
            if !v.m.on_events() {
                ui(cx, |m| m.alphabetic = !m.alphabetic);
            }
            return MouseOut::default();
        }
        // (the line between the columns)
        let (lx, ly, _, lh) = v.g.list;
        if (x - (lx + v.nw)).abs() <= 3 && y >= ly && y < ly + lh && v.row_at(x, y).is_some_and(|(i, _)| !matches!(v.rows[i].kind, RowKind::Category | RowKind::Pins | RowKind::Picker)) {
            finish(cx, true);
            ui(cx, |mm| mm.ui.drag = Some((x, v.nw)));
            return MouseOut::default();
        }
        let Some((i, rr)) = v.row_at(x, y) else { return MouseOut::default() };
        let r = v.rows[i].clone();
        let part = geo::part_at(&v.m, &r, rr, v.nw, x, y);
        // (a press elsewhere ends an edit, kept — refused: it stays)
        if editing_key(cx.id).is_some() && !finish(cx, true) {
            return MouseOut::default();
        }
        let was = v.selected() == Some(i);
        select(cx, &r.key);
        if m.double() && part != Part::Button {
            match r.kind {
                RowKind::Event => send(cx, User::Inspector(Act::EventDblClick(r.name.clone()))),
                RowKind::Category => {}
                _ if matches!(geo::row_kind(&v.m, &r), Some(Kind::Enum(_))) => {
                    list::end_edit(cx.id);
                    cycle(cx, &v, &r, 1);
                }
                _ if r.expandable && part != Part::Value => ui(cx, |mm| mm.set_open(&r.key, !r.expanded)),
                _ => {}
            }
            return MouseOut::default();
        }
        match part {
            Part::Expander => ui(cx, |mm| mm.set_open(&r.key, !r.expanded)),
            Part::Reset => send(cx, User::Inspector(Act::Reset(r.key.clone()))),
            Part::Check if !v.m.read_only => toggle(cx, &r),
            Part::Button => drop(cx, &v, i),
            Part::Pin(k) if !v.m.read_only => {
                ui(cx, |mm| mm.ui.pin = k);
                toggle_pin(cx, &v, &r, k);
            }
            Part::Swatch(k) if !v.m.read_only => {
                ui(cx, |mm| mm.ui.swatch = k);
                pick_swatch(cx, &r, k);
            }
            Part::Field => {
                begin(cx, &v, &r.key);
            }
            Part::Value => match r.kind {
                // (an event's SUB: double click makes one; a press selects)
                RowKind::Event if !was => {}
                _ => {
                    let typed = begin(cx, &v, &r.key);
                    if typed {
                        let rect = view(cx, classic()).value_rect(i);
                        list::editor_mouse(cx, m, rect);
                    } else if matches!(geo::row_kind(&v.m, &r), Some(Kind::Bool)) && was && !v.m.read_only {
                        toggle(cx, &r);
                    }
                }
            },
            _ => {}
        }
        MouseOut::default()
    }
}

/// Type-ahead starts over (a click, a key that isn't a letter).
fn forget_typed(cx: &Cx) {
    TYPED.with(|t| t.borrow_mut().remove(&cx.id.to_lowercase()));
}

/// A list the runtime dropped for it shows its current value lit.
fn sync_drop(cx: &Cx) {
    if let Some(i) = model::with(cx.id, |m| m.ui.drop_hot).flatten() {
        if combo::is_dropped(cx.id) {
            combo::set_hot(cx.id, i);
            ui(cx, |m| m.ui.drop_hot = None);
        }
    }
}

/// The tab at x, if the strip has one there.
fn tab_at(cx: &Cx, v: &V, x: i64) -> Option<usize> {
    let t = v.g.tabs?;
    let mut at = t.0 + 4;
    for (i, title) in ["Properties", "Events"].iter().enumerate() {
        let (tw, _) = rapidr_value::objects::text::text_size(title, &cx.font);
        let w = tw + 20;
        if x >= at && x < at + w {
            return Some(i);
        }
        at += w + 2;
    }
    None
}

impl ComponentKind for Inspector {
    fn name(&self) -> &'static str {
        "RPROPERTYINSPECTOR"
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let l = look(p.theme());
        let (w, h) = (cx.width(), cx.height());
        common::ground(p, w, h, &l);
        // (the program's requests: EditValue's editor, a list's lit item, a
        // row to show)
        let request = model::with_mut(cx.id, |m| m.ui.edit_request.take());
        let mut v = view(cx, l.classic);
        if let Some(key) = request {
            begin(cx, &v, &key);
        }
        sync_drop(cx);
        if v.m.ui.reveal {
            if let Some(i) = v.selected() {
                let (top, bottom) = v.span(i);
                list::scroll_into_view(cx.id, top, bottom, v.g.list.3);
            }
            ui(cx, |m| m.ui.reveal = false);
        }
        let (lx, ly, lw, lh) = v.g.list;
        let (pos, cw, bar) = list::vscroll(cx.id, lw, lh, geo::content(&v.rows), ROW);
        v.pos = pos;
        v.cw = cw;
        v.nw = geo::name_width(&v.m, cw);
        let editing = editing_key(cx.id);
        if editing.is_none() {
            list::drop_editor(cx);
        }
        let searching = editing.as_deref() == Some(SEARCH);
        let value_edit = editing.as_deref().filter(|k| *k != SEARCH);
        let font = cx.font.clone();
        let empty = crate::store::string(cx.store, cx.id, "emptytext");
        let c = draw::Ctx { id: cx.id, l: &l, font: &font, focused: cx.state.focused, editing: value_edit, searching, enabled: cx.state.enabled, empty: &empty };
        draw::header(p, &v.g, &v.m, &c);
        if let Some(t) = v.g.tabs {
            let hover = match v.m.ui.hover {
                Some(Hover::Tab(i)) => Some(i),
                _ => None,
            };
            common::tabs(p, t, &l, &["Properties", "Events"], usize::from(v.m.on_events()), hover, &font);
        }
        draw::strip(p, &v.g, &v.m, &c);
        if searching {
            let rect = v.search_rect();
            edit::paint_line(cx, p, list::editor_area(rect), Source::InPlace);
        }
        // (the rows)
        p.fill((lx, ly, lw, lh), l.body);
        if v.rows.is_empty() {
            draw::empty(p, (lx, ly, cw, lh), &v.m, &c);
        }
        let selected = v.selected();
        p.clipped((lx, ly, cw, lh), |p| {
            let mut top = ly - pos;
            for (i, r) in v.rows.iter().enumerate() {
                let rr = (lx, top, cw, r.height);
                top += r.height;
                if rr.1 + rr.3 <= ly || rr.1 >= ly + lh {
                    continue;
                }
                draw::row(p, &v.m, r, i, rr, v.nw, selected == Some(i), &c);
            }
        });
        if let Some(key) = value_edit {
            if let Some(i) = v.index(key) {
                let rect = v.value_rect(i);
                let error = v.m.ui.error.as_ref().is_some_and(|(k, _)| k == key);
                let (ex, ey, ew, eh) = rect;
                p.clipped((lx, ly, cw, lh), |p| {
                    if l.classic {
                        p.fill(rect, l.field);
                        p.frame(rect, if error { l.error } else { l.text });
                    } else {
                        p.round((ex - 1, ey, ew + 2, eh), 4.0, Some(l.field), Some(if error { l.error } else { l.field_focus }), if l.contrast { 2.0 } else { 1.5 });
                    }
                });
                if ey >= ly && ey + eh <= ly + lh {
                    edit::paint_line(cx, p, list::editor_area(rect), Source::InPlace);
                }
            }
        }
        p.at((lx, ly), |p| p.ops(bar));
        if let Some(f) = v.g.footer {
            draw::footer(p, f, &v.m, &v.rows, &c);
        }
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        let (x, y) = (m.x.floor() as i64, m.y.floor() as i64);
        let v = view(cx, classic());
        let (lx, ly, lw, lh) = v.g.list;
        // (the scroll bar)
        let inner = MouseIn { x: m.x - lx as f64, y: m.y - ly as f64, ..*m };
        if list::bar_mouse(cx, &inner, lw, lh) {
            return MouseOut::default();
        }
        // (the editor takes its own presses and drags)
        if let Some(rect) = edit_rect(cx) {
            if list::editor_mouse(cx, m, rect) {
                return MouseOut::default();
            }
        }
        match m.kind {
            MouseKind::Down => Self::press(cx, m, x, y),
            MouseKind::Move => {
                if let Some((from, width)) = v.m.ui.drag {
                    let nw = (width + x - from).clamp(40, (v.cw - 40).max(40));
                    ui(cx, |mm| mm.name_width = nw);
                    return MouseOut::default();
                }
                let hover = if let Some(t) = v.g.tabs.filter(|t| common::inside(*t, x, y)) {
                    let _ = t;
                    tab_at(cx, &v, x).map(Hover::Tab)
                } else if common::inside(v.g.view_button, x, y) {
                    Some(Hover::ViewButton)
                } else if common::inside(v.g.search, x, y) {
                    Some(Hover::Search)
                } else if (x - (lx + v.nw)).abs() <= 3 && y >= ly && y < ly + lh {
                    Some(Hover::Divider)
                } else {
                    v.row_at(x, y).map(|(i, rr)| draw::hover_of(geo::part_at(&v.m, &v.rows[i], rr, v.nw, x, y), i))
                };
                if hover != v.m.ui.hover {
                    ui(cx, |mm| mm.ui.hover = hover);
                }
                MouseOut::default()
            }
            MouseKind::Up => {
                if v.m.ui.drag.is_some() {
                    ui(cx, |mm| mm.ui.drag = None);
                }
                MouseOut::default()
            }
            MouseKind::Leave => {
                if v.m.ui.hover.is_some() {
                    ui(cx, |mm| mm.ui.hover = None);
                }
                MouseOut::default()
            }
        }
    }

    fn key(&self, cx: &mut Cx, k: &KeyIn, clip: &mut dyn Clipboard) -> bool {
        sync_drop(cx);
        // (a list it dropped: Up / Down light, Enter picks, Escape closes)
        if let Some((hot, n)) = combo::hot_item(cx.id) {
            match k.vk {
                vk::UP | vk::DOWN if !k.mods.alt => {
                    let at = hot.as_ref().map(|h| h.0 as i64).unwrap_or(-1);
                    let next = if k.vk == vk::DOWN { at + 1 } else { at - 1 };
                    combo::set_hot(cx.id, next.clamp(0, n as i64 - 1) as usize);
                    return true;
                }
                vk::ENTER | vk::SPACE => {
                    combo::close();
                    if let Some((_, item)) = hot {
                        send(cx, User::Picked(item));
                    }
                    return true;
                }
                vk::ESCAPE | vk::F4 => {
                    combo::close();
                    return true;
                }
                vk::UP | vk::DOWN => {
                    combo::close();
                    return true;
                }
                _ => combo::close(),
            }
        }
        let v = view(cx, classic());
        if let Some(key) = editing_key(cx.id) {
            return Self::edit_key(cx, &v, &key, k, clip);
        }
        if (k.mods.ctrl || k.mods.command) && k.vk == 70 {
            begin(cx, &v, SEARCH);
            return true;
        }
        if k.mods.command || (k.mods.ctrl && k.vk != vk::ENTER) {
            return false;
        }
        Self::row_key(cx, &v, k)
    }

    fn wheel(&self, cx: &mut Cx, _dx: f64, dy: f64, _mods: crate::input::Mods) -> bool {
        let v = view(cx, classic());
        list::vscroll_wheel(cx.id, dy, v.g.list.2, v.g.list.3)
    }

    fn tick(&self, cx: &mut Cx) {
        list::bar_tick(cx);
    }

    fn ime(&self, cx: &mut Cx, ime: &Ime) -> bool {
        edit_rect(cx).is_some_and(|r| list::editor_ime(cx, ime, r))
    }

    fn ime_area(&self, cx: &mut Cx) -> Option<Rect> {
        list::editor_ime_area(cx, edit_rect(cx)?)
    }

    fn wants_ime(&self, _store: &dyn Store, id: &str) -> bool {
        list::editing(id).is_some()
    }

    fn context_menu(&self, cx: &mut Cx) -> Option<edit::MenuState> {
        list::editor_menu(cx, edit_rect(cx)?)
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        let mut n = crate::components::shared_describe(cx, self.name());
        n.role = Role::Grid;
        let v = view(cx, classic());
        if n.name.is_empty() {
            n.name = match v.m.snap.objects.as_slice() {
                [] => "Properties".into(),
                [(name, ty)] => format!("Properties of {name} ({ty})"),
                more => format!("Properties of {} components", more.len()),
            };
        }
        n.children.clear();
        let (ax, ay) = (cx.rect.0, cx.rect.1);
        let at = |r: Rect| (ax + r.0, ay + r.1, r.2, r.3);
        let editing = editing_key(cx.id);
        let focused = cx.state.focused;
        if let Some(t) = v.g.tabs {
            let mut list = AccessNode::new(part_id(cx.id, PART_TAB, PART_TABS + 9), Role::TabList);
            list.name = "Pages".into();
            list.bounds = at(t);
            let mut x = t.0 + 4;
            for (i, title) in ["Properties", "Events"].iter().enumerate() {
                let (tw, _) = rapidr_value::objects::text::text_size(title, &cx.font);
                let mut tab = AccessNode::new(part_id(cx.id, PART_TAB, PART_TABS + i), Role::Tab);
                tab.name = title.to_string();
                tab.states.selected = Some(usize::from(v.m.on_events()) == i);
                tab.actions = vec![Action::Click];
                tab.bounds = at((x, t.1, tw + 20, t.3));
                x += tw + 22;
                list.children.push(tab);
            }
            n.children.push(list);
        }
        let mut search = AccessNode::new(part_id(cx.id, PART_ITEM, PART_SEARCH), Role::TextInput);
        search.name = if v.m.on_events() { "Search events".into() } else { "Search properties".into() };
        search.value = Some(match &editing {
            Some(k) if k == SEARCH => list::editing(cx.id).map(|e| e.text).unwrap_or_default(),
            _ => v.m.filter.clone(),
        });
        search.states.focused = focused && editing.as_deref() == Some(SEARCH);
        search.actions = vec![Action::Focus, Action::SetValue];
        search.bounds = at(v.g.search);
        n.children.push(search);
        let mut view_button = AccessNode::new(part_id(cx.id, PART_ITEM, PART_VIEW), Role::Button);
        view_button.name = "Sort A to Z".into();
        view_button.states.checked = Some(v.m.alphabetic);
        view_button.states.disabled = v.m.on_events();
        view_button.actions = vec![Action::Click];
        view_button.bounds = at(v.g.view_button);
        n.children.push(view_button);
        let selected = v.selected();
        let mut top = v.g.list.1 - v.pos;
        for (i, r) in v.rows.iter().enumerate() {
            let rr = (v.g.list.0, top, v.cw, r.height);
            top += r.height;
            let mut row = AccessNode::new(part_id(cx.id, PART_ROW, i), Role::Row);
            row.name = match r.kind {
                RowKind::Pins => "Anchors editor".into(),
                RowKind::Picker => "Colour picker".into(),
                RowKind::AddLine => r.name.clone(),
                _ => r.prop_name(&v.m.snap.props, &v.m.snap.events),
            };
            if !matches!(r.kind, RowKind::Category | RowKind::Pins | RowKind::Picker) {
                row.value = Some(if r.mixed { String::new() } else { r.text.clone() });
            }
            // (a category 1, its rows 2, their parts 3; A–Z: rows 1, parts 2)
            row.level = Some(r.level + 1);
            row.states.selected = Some(selected == Some(i));
            row.states.focused = focused && selected == Some(i) && editing.is_none();
            row.states.read_only = v.m.read_only || geo::row_kind(&v.m, r).is_none_or(|k| !k.typed() && !matches!(k, Kind::Bool));
            if r.expandable {
                row.states.expanded = Some(r.expanded);
            }
            row.description = match r.kind {
                RowKind::Category => format!("{} properties", r.count),
                _ if r.mixed => "values differ".into(),
                _ if r.ext => "RapidR extension".into(),
                _ if !r.is_default && geo::resettable(r) => "changed".into(),
                _ => String::new(),
            };
            row.actions = vec![Action::Click, Action::Focus];
            if !row.states.read_only {
                row.actions.push(Action::SetValue);
            }
            if r.expandable {
                row.actions.push(if r.expanded { Action::Collapse } else { Action::Expand });
            }
            row.bounds = at(rr);
            match r.kind {
                RowKind::Pins => {
                    let (_, _, struts) = geo::pins(rr);
                    let mask = v.m.snap.props.get(r.prop).and_then(|p| p.value.as_ref()).map_or(0, |x| x.to_i64());
                    for (k, s) in struts.iter().enumerate() {
                        let mut pin = AccessNode::new(part_id(cx.id, PART_ITEM, PART_PINS + i * 4 + k), Role::CheckBox);
                        pin.name = format!("Anchor {}", ["left", "top", "right", "bottom"][k]);
                        pin.states.checked = Some(mask & (1 << k) != 0);
                        pin.states.focused = focused && selected == Some(i) && v.m.ui.pin == k && editing.is_none();
                        pin.actions = vec![Action::Click];
                        pin.bounds = at(*s);
                        row.children.push(pin);
                    }
                }
                RowKind::Picker => {
                    let pk = geo::picker(rr);
                    let cur = v.m.snap.props.get(r.prop).and_then(|p| p.value.as_ref()).map(|x| x.to_i64());
                    for (k, s) in pk.swatches.iter().enumerate() {
                        let name = geo::swatch_name(k);
                        let mut sw = AccessNode::new(part_id(cx.id, PART_ITEM, PART_SWATCHES + i * 64 + k), Role::ListBoxOption);
                        sw.name = name.to_string();
                        sw.states.selected = Some(values::color_value(name) == cur);
                        sw.states.focused = focused && selected == Some(i) && v.m.ui.swatch == k && editing.is_none();
                        sw.actions = vec![Action::Click];
                        sw.bounds = at(*s);
                        row.children.push(sw);
                    }
                }
                _ => {}
            }
            if editing.as_deref() == Some(r.key.as_str()) {
                let mut ed = AccessNode::new(part_id(cx.id, PART_EDITOR, PART_EDIT), Role::TextInput);
                ed.name = row.name.clone();
                ed.value = list::editing(cx.id).map(|e| e.text);
                ed.states.focused = focused;
                ed.bounds = at(v.value_rect(i));
                row.children.push(ed);
            }
            n.children.push(row);
        }
        n
    }

    fn access(&self, cx: &mut Cx, action: Action, part: Option<usize>, value: Option<&AccessValue>) -> bool {
        let Some(part) = part else { return false };
        let v = view(cx, classic());
        let text = match value {
            Some(AccessValue::Text(t)) => t.clone(),
            Some(AccessValue::Number(n)) => values::float_text(*n),
            None => String::new(),
        };
        match part {
            p if (PART_TABS..PART_TABS + 2).contains(&p) && action == Action::Click => {
                ui(cx, |m| m.events_page = p == PART_TABS + 1);
                true
            }
            PART_SEARCH => match action {
                Action::SetValue => {
                    ui(cx, |m| m.filter = text);
                    true
                }
                Action::Focus | Action::Click => begin(cx, &v, SEARCH),
                _ => false,
            },
            PART_VIEW if action == Action::Click => {
                ui(cx, |m| m.alphabetic = !m.alphabetic);
                true
            }
            p if (PART_SWATCHES..PART_EDIT).contains(&p) && action == Action::Click => {
                let (i, k) = ((p - PART_SWATCHES) / 64, (p - PART_SWATCHES) % 64);
                let Some(r) = v.rows.get(i).cloned() else { return false };
                select(cx, &r.key);
                ui(cx, |m| m.ui.swatch = k);
                pick_swatch(cx, &r, k);
                true
            }
            p if (PART_PINS..PART_SWATCHES).contains(&p) && action == Action::Click => {
                let (i, k) = ((p - PART_PINS) / 4, (p - PART_PINS) % 4);
                let Some(r) = v.rows.get(i).cloned() else { return false };
                select(cx, &r.key);
                ui(cx, |m| m.ui.pin = k);
                toggle_pin(cx, &v, &r, k);
                true
            }
            i if i < v.rows.len() => {
                let r = v.rows[i].clone();
                match action {
                    Action::Click | Action::Focus => {
                        go(cx, &v, i, v.selected());
                        true
                    }
                    Action::Expand | Action::Collapse if r.expandable => {
                        ui(cx, |m| m.set_open(&r.key, action == Action::Expand));
                        true
                    }
                    Action::SetValue => {
                        select(cx, &r.key);
                        commit(cx, &r.key, &text)
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    /// `__edit` (F2 on the selected row), `__enter` ("Renamed" typed, then
    /// Enter), `__escape` (the edit dropped).
    fn test_action(&self, cx: &mut Cx, action: &str) -> bool {
        let key = |vk: i64| KeyIn { vk, text: "", mods: crate::input::Mods::NONE };
        let mut clip = crate::input::MemClipboard::default();
        match action {
            "__edit" => {
                self.key(cx, &key(vk::F2), &mut clip);
            }
            "__enter" | "__escape" => {
                if list::editing(cx.id).is_some() {
                    if action == "__enter" {
                        list::set_edit_text(cx.id, "Renamed");
                    }
                    self.key(cx, &key(if action == "__enter" { vk::ENTER } else { vk::ESCAPE }), &mut clip);
                }
            }
            _ => return false,
        }
        true
    }
}

/// The focus left it while its in-place editor was open
/// (`super::focus_left`): a value typed is kept (sent to the program), the
/// search's text stays.
pub fn focus_left(id: &str, ed: InPlace) -> Vec<KernelEvent> {
    let key = EDITING.with(|e| e.borrow_mut().remove(&id.to_lowercase()));
    match key {
        Some(k) if k != SEARCH => {
            let ok = model::with(id, |m| m.interpret(&k, &ed.text).is_ok()).unwrap_or(false);
            let same = model::with(id, |m| m.value_text(&k).as_deref() == Some(ed.text.as_str())).unwrap_or(false);
            if ok && !same {
                vec![KernelEvent::Container(crate::components::form::Container::Panel { id: id.to_lowercase(), action: User::Inspector(Act::Commit { key: k, text: ed.text }) })]
            } else {
                Vec::new()
            }
        }
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests;

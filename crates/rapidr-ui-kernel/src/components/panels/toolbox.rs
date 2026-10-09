//! RTOOLBOX drawn and driven by the kernel; its model is
//! `rapidr_value::panels::toolbox` (the web draws the same).
//!
//! At the top a search box (an in-place editor over it while the user
//! types: the filter follows each key); under it the groups by purpose
//! (Standard, Additional, Dialogs …) as headings with a chevron, the
//! components under RapidR's names with their icons — or, while searching, the matches flat with their
//! group dimmed beside them, best first. The keyboard: in the list the
//! arrows, Home / End, Page Up / Down move, Left / Right close and open
//! groups (or go to the parent / first child), Enter picks (OnPick) or
//! turns a group over, letters go to a name (type-ahead), Backspace and
//! Ctrl+F go to the search box; in the search box Down goes to the list,
//! Enter picks the best match, Escape clears it (then leaves it). The
//! mouse: a click selects (OnSelect) or turns a group over, a double click
//! picks, a press dragged 5 pixels starts a drag (OnDragStart) whose ghost
//! follows the mouse, and its release over the form fires OnDragDrop on
//! what is under it.

use rapidr_value::input::Button;
use rapidr_value::objects::a11y::{part_id, AccessNode, Action, Role, PART_ITEM};
use rapidr_value::objects::font::Font;
use rapidr_value::objects::ops::{Place, Rect};
use rapidr_value::objects::text::text_size;
use rapidr_value::panels::rows::{self as rows_nav, vk, ROW, SEARCH};
use rapidr_value::panels::toolbox::{self as model, Drag, Key, Row, Toolbox as Model, User};
use rapidr_value::panels::User as PanelUser;

use super::common::{self, inside, look, Look};
use crate::a11y::AccessValue;
use crate::components::edit::{self, Source};
use crate::components::list::{self, begin_edit, begin_edit_typed, editing, end_edit, set_edit_text, InPlace};
use crate::components::{ComponentKind, Cx, Ime, KeyIn, MouseIn, MouseKind, MouseOut};
use crate::input::{Clipboard, KernelEvent};
use crate::paint::Painter;
use crate::store::Store;

pub struct Toolbox;

/// How far the mouse goes with an item pressed before it is a drag.
const DRAG_START: i64 = 5;

/// The accessibility parts' indexes: the search box, the groups (100 +
/// their index in the groups' table), the items (10 000 + their index in
/// the catalog).
const PART_SEARCH: usize = 1;
const PART_GROUPS: usize = 100;
const PART_ITEMS: usize = 10_000;

/// The search box pressed (`NodeUi::part`): the mouse is its editor's
/// until let go.
const BOX_PART: usize = usize::MAX;

fn send(cx: &mut Cx, action: User) {
    super::send(cx, PanelUser::Toolbox(action));
}

/// Where its parts are in a `w` × `h` toolbox.
struct Geo {
    /// The search strip across the top, the box in it.
    strip: Rect,
    sbox: Rect,
    /// The list under it.
    list: Rect,
}

fn geo(w: i64, h: i64, l: &Look) -> Geo {
    let i = common::inset(l);
    let strip = (i, i, (w - 2 * i).max(0), SEARCH);
    let sbox = (i + 6, i + 5, (w - 2 * i - 12).max(0), SEARCH - 10);
    let list = (i, i + SEARCH, (w - 2 * i).max(0), (h - 2 * i - SEARCH).max(0));
    Geo { strip, sbox, list }
}

/// The search box's clear button (shown with a filter).
fn clear_rect(sbox: Rect) -> Rect {
    let (x, y, w, h) = sbox;
    (x + w - h, y, h, h)
}

/// The text's own area in the search box (left of the clear button).
fn text_area(sbox: Rect) -> Rect {
    let (x, y, w, h) = sbox;
    let s = (h - 8).clamp(10, 16);
    (x + 10 + s, y + 1, (w - 14 - s - h).max(0), (h - 2).max(0))
}

/// The in-place editor's box whose text area is `area` (list.rs' rule:
/// 3 pixels in, 1 down).
fn editor_box(area: Rect) -> Rect {
    (area.0 - 3, area.1 - 1, area.2 + 6, area.3 + 2)
}

fn model<R>(id: &str, f: impl FnOnce(&Model) -> R) -> R {
    model::with_mut(id, |m| f(m))
}

fn model_mut<R>(id: &str, f: impl FnOnce(&mut Model) -> R) -> R {
    model::with_mut(id, f)
}

/// The search box is being typed in.
fn searching(id: &str) -> bool {
    editing(id).is_some()
}

/// The scroll position and the rows' width (the bar beside them), and the
/// bar's ops, for `count` rows.
fn scroll(id: &str, list: Rect, count: usize) -> (i64, i64, Vec<rapidr_value::objects::ops::Op>) {
    list::vscroll(id, list.2, list.3, count as i64 * ROW, ROW)
}

/// The row at (x, y) of the toolbox.
fn row_at(id: &str, g: &Geo, rows: &[Row], x: i64, y: i64) -> Option<usize> {
    let (pos, cw, _) = scroll(id, g.list, rows.len());
    if !inside((g.list.0, g.list.1, cw, g.list.3), x, y) {
        return None;
    }
    let i = ((y - g.list.1 + pos) / ROW) as usize;
    (i < rows.len()).then_some(i)
}

/// Row `i` of `count` scrolled into view (the bar brought up to date
/// first: a scroll asked for before is where it starts from).
fn reveal(id: &str, g: &Geo, i: usize, count: usize) {
    scroll(id, g.list, count);
    list::scroll_into_view(id, i as i64 * ROW, (i as i64 + 1) * ROW, g.list.3);
}

/// Opens the search box's editor (the caret after the text when `typed`).
fn start_search(id: &str, typed: bool) {
    if searching(id) {
        return;
    }
    let text = model(id, |m| m.filter.clone());
    let ed = InPlace { target: (0, 0), text, rect: None };
    if typed {
        begin_edit_typed(id, ed);
    } else {
        begin_edit(id, ed);
    }
}

/// The search box's editor closes, the filter kept.
fn stop_search(cx: &mut Cx) {
    if let Some(ed) = end_edit(cx.id) {
        model_mut(cx.id, |m| m.set_filter(&ed.text));
    }
    cx.ui.edit = None;
}

/// The filter changed (typed, cleared): the list at its top.
fn filter_changed(cx: &mut Cx, text: &str) {
    let g = geo(cx.width(), cx.height(), &look(rapidr_value::theme::current()));
    let changed = model_mut(cx.id, |m| {
        let was = m.filter.clone();
        m.set_filter(text);
        was != m.filter
    });
    if changed {
        let (key, rows) = model(cx.id, |m| (m.cursor.clone(), m.rows()));
        list::vscroll_at(cx.id, g.list.2, g.list.3, rows.len() as i64 * ROW, ROW, Some(0));
        if let Some(i) = key.and_then(|k| rows.iter().position(|r| r.key == k)) {
            reveal(cx.id, &g, i, rows.len());
        }
    }
}

/// The keyboard's row moved to `key`: an item is selected too (OnSelect).
fn go_to(cx: &mut Cx, key: Key) {
    let g = geo(cx.width(), cx.height(), &look(rapidr_value::theme::current()));
    let rows = model_mut(cx.id, |m| {
        m.cursor = Some(key.clone());
        m.rows()
    });
    if let Some(i) = rows.iter().position(|r| r.key == key) {
        reveal(cx.id, &g, i, rows.len());
    }
    // (the runtime selects it: OnSelect when it changed)
    if key.is_item() {
        send(cx, User::Select(key));
    }
}

/// A group turned over (`open`: `None`), the keyboard on it.
fn toggle(cx: &mut Cx, group: &str, open: Option<bool>) {
    model_mut(cx.id, |m| {
        m.set_open(group, open);
        m.cursor = Some(Key::Group(group.to_string()));
    });
}

/// An item picked (Enter, a double click): OnPick.
fn pick(cx: &mut Cx, key: Key) {
    model_mut(cx.id, |m| {
        m.selected = Some(key.clone());
        m.cursor = Some(key.clone());
    });
    send(cx, User::Pick(key));
}

/// A component's or template's row: its icon, its name (marked letters
/// while searching), its group dimmed at the right while searching.
fn paint_item(p: &mut Painter, l: &Look, r: &Row, rect: Rect, ink: u32, dim: u32, font: &Font) {
    let (x, y, w, h) = rect;
    let flat = r.level == 1;
    let ix = if flat { x + 10 } else { x + 32 };
    common::icon(p, &r.icon, ix, y + (h - 16) / 2, 16, None, false);
    let tx = ix + 22;
    let mut right = x + w - 8;
    if flat && !r.group.is_empty() {
        let (gw, _) = text_size(&r.group, font);
        let gw = gw.min((w / 3).max(0));
        right -= gw;
        let g = common::elide(&r.group, font, gw);
        p.text((right, y, gw, h), &g, font, dim, Place::Left);
        right -= 8;
    }
    common::marked_text(p, (tx, y, (right - tx).max(0), h), &r.text, &r.marks, font, ink, if ink == l.text { l.mark } else { ink });
}

/// A group's row: its chevron (or classic ±), its icon, its title (bold
/// for a top group, on the section's band), its count at the right.
fn paint_group(p: &mut Painter, l: &Look, r: &Row, rect: Rect, ink: u32, band: bool, font: &Font) {
    let (x, y, w, h) = rect;
    let top = r.level == 1;
    if band {
        p.fill(rect, l.section);
        if l.classic {
            p.fill((x, y + h - 1, w, 1), l.line);
        }
    }
    let c = if top { x + 10 } else { x + 16 };
    let mid = y + h / 2;
    let open = r.open.unwrap_or(true);
    if l.classic {
        crate::components::tree::expander(p, c, mid, 9, open);
    } else {
        p.chevron(c as f64 + 0.5, mid as f64 + 0.5, 7.0, open, if ink == l.text { l.dim } else { ink });
    }
    let ix = c + 8;
    common::icon(p, &r.icon, ix, y + (h - 16) / 2, 16, None, false);
    let tx = ix + 22;
    let bold = Font { styles: font.styles | u8::from(top), ..font.clone() };
    let count = r.count.to_string();
    let (cw, _) = text_size(&count, font);
    let tw = (x + w - 10 - cw - 8 - tx).max(0);
    p.text((tx, y, tw, h), &common::elide(&r.text, &bold, tw), &bold, if top && band { l.section_text } else { ink }, Place::Left);
    p.text((x + w - 10 - cw, y, cw, h), &count, font, if ink == l.text || ink == l.section_text { l.dim } else { ink }, Place::Left);
}

impl Toolbox {
    /// The rows, the keyboard's row and the selected one now.
    fn state(id: &str) -> (Vec<Row>, Option<Key>, Option<Key>, Option<Key>) {
        model(id, |m| (m.rows(), m.cursor.clone(), m.selected.clone(), m.hover.clone()))
    }

    /// A key in the list (not the search box): whether it was the list's.
    fn list_key(&self, cx: &mut Cx, k: &KeyIn, clip: &mut dyn Clipboard) -> bool {
        let ctrl = k.mods.ctrl || k.mods.command;
        let (rows, cursor, _, _) = Self::state(cx.id);
        let at = cursor.as_ref().and_then(|c| rows.iter().position(|r| &r.key == c));
        let g = geo(cx.width(), cx.height(), &look(rapidr_value::theme::current()));
        if k.vk != 0 && !matches!(k.vk, 16..=18) && k.text.trim().is_empty() {
            model_mut(cx.id, |m| m.typed.clear());
        }
        match k.vk {
            vk::UP | vk::DOWN | vk::HOME | vk::END | vk::PAGE_UP | vk::PAGE_DOWN if !ctrl && !k.mods.alt => {
                let page = (g.list.3 / ROW).max(1) as usize;
                if let Some(i) = rows_nav::nav(k.vk, at, rows.len(), page) {
                    go_to(cx, rows[i].key.clone());
                }
                true
            }
            vk::LEFT | vk::RIGHT if !ctrl && !k.mods.alt => {
                let Some(r) = at.map(|i| rows[i].clone()) else { return true };
                match (&r.key, r.open, k.vk) {
                    (Key::Group(g), Some(true), vk::LEFT) => toggle(cx, g, Some(false)),
                    (Key::Group(g), Some(false), vk::RIGHT) => toggle(cx, g, Some(true)),
                    (Key::Group(_), Some(true), vk::RIGHT) => {
                        if let Some(next) = rows.get(at.unwrap_or(0) + 1) {
                            go_to(cx, next.key.clone());
                        }
                    }
                    (key, _, vk::LEFT) => {
                        if let Some(parent) = model(cx.id, |m| m.parent_of(key)) {
                            go_to(cx, parent);
                        }
                    }
                    _ => {}
                }
                true
            }
            vk::ENTER if !ctrl && !k.mods.alt => {
                match at.map(|i| rows[i].key.clone()) {
                    Some(Key::Group(g)) => toggle(cx, &g, None),
                    Some(item) => pick(cx, item),
                    None => return false,
                }
                true
            }
            vk::ESCAPE => {
                if model(cx.id, |m| m.drag.as_ref().is_some_and(|d| d.started)) {
                    model_mut(cx.id, |m| m.drag = None);
                    send(cx, User::DragEnd);
                    return true;
                }
                if model(cx.id, |m| m.filter.is_empty()) {
                    return false;
                }
                filter_changed(cx, "");
                true
            }
            // (Ctrl+F: the search box)
            70 if ctrl && !k.mods.alt => {
                start_search(cx.id, false);
                true
            }
            // (Backspace: the search box, its last letter gone)
            vk::BACK if !ctrl && !k.mods.alt => {
                start_search(cx.id, true);
                self.search_key(cx, k, clip);
                true
            }
            vk::SPACE if !ctrl && !k.mods.alt && model(cx.id, |m| m.typed.is_empty()) => {
                if let Some(Key::Group(g)) = at.map(|i| rows[i].key.clone()) {
                    toggle(cx, &g, None);
                }
                true
            }
            _ if !k.text.is_empty() && !ctrl && !k.mods.alt && k.text.chars().all(|c| !c.is_control()) => {
                if let Some(key) = model_mut(cx.id, |m| m.type_ahead(k.text)) {
                    go_to(cx, key);
                }
                true
            }
            _ => false,
        }
    }

    /// A key in the search box: whether it was its.
    fn search_key(&self, cx: &mut Cx, k: &KeyIn, clip: &mut dyn Clipboard) -> bool {
        let ctrl = k.mods.ctrl || k.mods.command;
        let g = geo(cx.width(), cx.height(), &look(rapidr_value::theme::current()));
        match k.vk {
            vk::DOWN if !ctrl && !k.mods.alt => {
                stop_search(cx);
                let (rows, cursor, _, _) = Self::state(cx.id);
                let key = cursor.filter(|c| rows.iter().any(|r| &r.key == c)).or_else(|| rows.first().map(|r| r.key.clone()));
                if let Some(key) = key {
                    go_to(cx, key);
                }
                true
            }
            vk::ENTER if !ctrl && !k.mods.alt => {
                let (rows, cursor, _, _) = Self::state(cx.id);
                let key = cursor.filter(|c| c.is_item() && rows.iter().any(|r| &r.key == c)).or_else(|| rows.iter().find(|r| r.key.is_item()).map(|r| r.key.clone()));
                if let Some(key) = key {
                    pick(cx, key);
                }
                true
            }
            vk::ESCAPE => {
                if model(cx.id, |m| m.filter.is_empty()) {
                    stop_search(cx);
                } else {
                    set_edit_text(cx.id, "");
                    filter_changed(cx, "");
                }
                true
            }
            vk::TAB => false,
            _ => {
                let r = editor_box(text_area(g.sbox));
                if list::edit_key(cx, k, clip, r).is_none() {
                    return false;
                }
                if let Some(ed) = editing(cx.id) {
                    filter_changed(cx, &ed.text);
                }
                true
            }
        }
    }

    /// The accessibility parts' index of each row.
    fn part_of(id: &str, key: &Key) -> Option<usize> {
        match key {
            Key::Group(g) => model::groups().iter().position(|x| x.id == g).map(|i| PART_GROUPS + i),
            item => model(id, |m| m.catalog().iter().position(|(k, _)| k == item)).map(|i| PART_ITEMS + i),
        }
    }

    /// The row an accessibility part names.
    fn key_of_part(id: &str, part: usize) -> Option<Key> {
        if (PART_GROUPS..PART_ITEMS).contains(&part) {
            return model::groups().get(part - PART_GROUPS).map(|g| Key::Group(g.id.to_string()));
        }
        let i = part.checked_sub(PART_ITEMS)?;
        model(id, |m| m.catalog().get(i).map(|(k, _)| k.clone()))
    }
}

impl ComponentKind for Toolbox {
    fn name(&self) -> &'static str {
        "RTOOLBOX"
    }

    /// (tooltip.rs) The item under the mouse: its card — its name, what it
    /// is, RapidQ's or RapidR's, where it runs.
    fn tip_at(&self, store: &dyn Store, id: &str, x: f64, y: f64) -> Option<String> {
        let (w, h) = (crate::store::int(store, id, "width", 0), crate::store::int(store, id, "height", 0));
        let g = geo(w, h, &look(rapidr_value::theme::current()));
        let (rows, ..) = Self::state(id);
        let i = row_at(id, &g, &rows, x.floor() as i64, y.floor() as i64)?;
        let card = model::card(&rows[i].key);
        (!card.is_empty()).then_some(card)
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let l = look(p.theme());
        let (w, h) = (cx.width(), cx.height());
        let g = geo(w, h, &l);
        common::ground(p, w, h, &l);
        // (the search strip)
        p.fill(g.strip, l.chrome);
        common::hline(p, g.strip.0, g.strip.1 + g.strip.3 - 1, g.strip.2, l.line);
        let typing = searching(cx.id);
        let filter = model(cx.id, |m| m.filter.clone());
        let shown_text = if typing { String::new() } else { filter.clone() };
        let empty = if typing { editing(cx.id).is_none_or(|e| e.text.is_empty()) } else { filter.is_empty() };
        let font = cx.font.clone();
        common::search_box(p, g.sbox, &l, &shown_text, if empty { "Search components" } else { "" }, typing && cx.state.focused, &font);
        if !filter.is_empty() || typing && !empty {
            let c = clear_rect(g.sbox);
            common::icon(p, "close", c.0 + (c.2 - 12) / 2, c.1 + (c.3 - 12) / 2, 12, Some(l.dim), false);
        }
        if typing {
            let area = text_area(g.sbox);
            p.clipped(area, |p| edit::paint_line(cx, p, area, Source::InPlace));
        } else {
            list::drop_editor(cx);
        }
        // (the rows)
        let (rows, cursor, selected, hover) = Self::state(cx.id);
        // (the keyboard's item is the one highlighted — the best match while
        // searching — else the selected one)
        let selected = cursor.clone().filter(Key::is_item).or(selected);
        let (pos, cw, bar) = scroll(cx.id, g.list, rows.len());
        let list_focus = cx.state.focused && !typing;
        let enabled = cx.state.enabled;
        p.clipped((g.list.0, g.list.1, cw, g.list.3), |p| {
            if rows.is_empty() {
                let msg = if filter.trim().is_empty() { "No components" } else { "No components match" };
                p.text((g.list.0, g.list.1 + 8, cw, ROW), msg, &font, l.dim, Place::Center);
            }
            let first = (pos / ROW) as usize;
            let last = ((pos + g.list.3) / ROW + 1) as usize;
            for (i, r) in rows.iter().enumerate().take(last.min(rows.len())).skip(first) {
                let rect = (g.list.0, g.list.1 + i as i64 * ROW - pos, cw, ROW);
                let is_sel = selected.as_ref() == Some(&r.key);
                let is_cursor = cursor.as_ref() == Some(&r.key);
                let is_hover = hover.as_ref() == Some(&r.key) && !is_sel;
                let band = matches!(r.key, Key::Group(_)) && r.level == 1;
                if band {
                    paint_group(p, &l, r, rect, l.text, true, &font);
                    if is_cursor && list_focus {
                        common::row_focus(p, rect, &l);
                    }
                    continue;
                }
                // (the keyboard's row is the selection when it's an item; a
                // group under it gets the focus ring)
                let ink = common::row(p, rect, &l, is_sel, cx.state.focused, is_hover);
                let ink = if enabled { ink } else { l.disabled };
                let dim = if is_sel && cx.state.focused { ink } else { l.dim };
                match r.key {
                    Key::Group(_) => paint_group(p, &l, r, rect, ink, false, &font),
                    _ => paint_item(p, &l, r, rect, ink, dim, &font),
                }
                if is_cursor && list_focus && !is_sel {
                    common::row_focus(p, rect, &l);
                }
            }
        });
        p.at((g.list.0, g.list.1), |p| p.ops(bar));
    }

    /// A drag's ghost — the component's icon and name on a raised card —
    /// following the mouse over the form (the toolbox is drawn over its
    /// neighbours while it lasts).
    fn paint_over(&self, store: &dyn Store, id: &str, _w: i64, _h: i64, p: &mut Painter) {
        let Some((drag, text, icon)) = model::with(id, |m| m.drag.clone().filter(|d| d.started).map(|d| (d.clone(), m.text_of(&d.key), m.icon_of(&d.key)))).flatten() else { return };
        let l = look(p.theme());
        let font = store.font(id);
        let (tw, _) = text_size(&text, &font);
        let (cw, ch) = (tw + 42, 26);
        let (x, y) = (drag.at.0 + 12, drag.at.1 + 8);
        let card = (x, y, cw, ch);
        if l.classic {
            p.fill(card, l.chrome);
            p.raised_edge(card);
        } else {
            // (a soft shadow under a raised card)
            let under = crate::paint::behind(store, id);
            for k in 1..=3 {
                let c = common::mix(l.shadow, under, 0.5 + 0.15 * k as f64);
                p.ring((x - k, y - k + 1, cw + 2 * k, ch + 2 * k), 6.0 + k as f64, c, 1.0);
            }
            p.round(card, 6.0, Some(l.body), Some(if l.contrast { l.text } else { l.accent }), if l.contrast { 2.0 } else { 1.5 });
        }
        common::icon(p, &icon, x + 8, y + (ch - 16) / 2, 16, None, false);
        p.text((x + 30, y, tw + 8, ch), &text, &font, l.text, Place::Left);
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        let l = look(rapidr_value::theme::current());
        let (w, h) = (cx.width(), cx.height());
        let g = geo(w, h, &l);
        let (x, y) = (m.x.floor() as i64, m.y.floor() as i64);
        let (rows, ..) = Self::state(cx.id);
        let id = cx.id.to_string();
        // (the scroll bar's)
        let inner = MouseIn { x: m.x - g.list.0 as f64, y: m.y - g.list.1 as f64, ..*m };
        if model(&id, |mm| mm.drag.is_none()) && list::bar_mouse(cx, &inner, g.list.2, g.list.3) {
            return MouseOut::default();
        }
        // (the search box's editor: a press in it, the drag after it)
        let area = text_area(g.sbox);
        let in_box = match m.kind {
            MouseKind::Down => inside(editor_box(area), x, y),
            MouseKind::Move | MouseKind::Up => m.captured && cx.ui.part == Some(BOX_PART),
            MouseKind::Leave => false,
        };
        if in_box && searching(&id) && list::editor_mouse(cx, m, editor_box(area)) {
            cx.ui.part = if m.kind == MouseKind::Up { None } else { Some(BOX_PART) };
            return MouseOut { press: false, focus: Some(true) };
        }
        match m.kind {
            MouseKind::Move | MouseKind::Leave if !m.captured => {
                let hov = if m.kind == MouseKind::Leave { None } else { row_at(&id, &g, &rows, x, y).map(|i| rows[i].key.clone()) };
                // (the row under the mouse changed: drawn again)
                if model_mut(&id, |mm| std::mem::replace(&mut mm.hover, hov.clone()) != hov) {
                    send(cx, User::Redraw);
                }
            }
            MouseKind::Down if m.button == Button::Left => {
                model_mut(&id, |mm| mm.typed.clear());
                if inside(g.strip, x, y) {
                    let filtered = !model(&id, |mm| mm.filter.is_empty()) || editing(&id).is_some_and(|e| !e.text.is_empty());
                    if filtered && inside(clear_rect(g.sbox), x, y) {
                        if searching(&id) {
                            set_edit_text(&id, "");
                        }
                        filter_changed(cx, "");
                        start_search(&id, false);
                    } else if inside(g.sbox, x, y) {
                        start_search(&id, false);
                        if inside(editor_box(area), x, y) && list::editor_mouse(cx, m, editor_box(area)) {
                            cx.ui.part = Some(BOX_PART);
                        }
                    }
                    return MouseOut { press: false, focus: Some(true) };
                }
                if searching(&id) {
                    stop_search(cx);
                }
                let Some(i) = row_at(&id, &g, &rows, x, y) else { return MouseOut::default() };
                let key = rows[i].key.clone();
                match &key {
                    Key::Group(gr) => toggle(cx, gr, None),
                    _ => {
                        go_to(cx, key.clone());
                        if m.double() {
                            pick(cx, key);
                        } else {
                            model_mut(&id, |mm| mm.drag = Some(Drag { key, from: (x, y), at: (x, y), started: false }));
                        }
                    }
                }
            }
            MouseKind::Move if m.captured => {
                let start = model_mut(&id, |mm| {
                    let d = mm.drag.as_mut()?;
                    d.at = (x, y);
                    if !d.started && ((x - d.from.0).abs() > DRAG_START || (y - d.from.1).abs() > DRAG_START) {
                        d.started = true;
                        mm.hover = None;
                        return Some(d.key.clone());
                    }
                    None
                });
                if let Some(key) = start {
                    send(cx, User::DragStart(key));
                }
            }
            MouseKind::Up => {
                if let Some(d) = model_mut(&id, |mm| mm.drag.take()) {
                    if d.started {
                        // (let go over the toolbox itself: nothing dropped)
                        if inside((0, 0, w, h), x, y) {
                            send(cx, User::DragEnd);
                        } else {
                            send(cx, User::Drop(d.key, x, y));
                        }
                    }
                }
            }
            _ => {}
        }
        MouseOut::default()
    }

    fn wheel(&self, cx: &mut Cx, _dx: f64, dy: f64, _mods: crate::input::Mods) -> bool {
        let g = geo(cx.width(), cx.height(), &look(rapidr_value::theme::current()));
        list::vscroll_wheel(cx.id, dy, g.list.2, g.list.3)
    }

    fn tick(&self, cx: &mut Cx) {
        list::bar_tick(cx);
    }

    fn key(&self, cx: &mut Cx, k: &KeyIn, clip: &mut dyn Clipboard) -> bool {
        if searching(cx.id) {
            self.search_key(cx, k, clip)
        } else {
            self.list_key(cx, k, clip)
        }
    }

    fn ime(&self, cx: &mut Cx, ime: &Ime) -> bool {
        let g = geo(cx.width(), cx.height(), &look(rapidr_value::theme::current()));
        let took = list::editor_ime(cx, ime, editor_box(text_area(g.sbox)));
        if took {
            if let Some(ed) = editing(cx.id) {
                filter_changed(cx, &ed.text);
            }
        }
        took
    }

    fn ime_area(&self, cx: &mut Cx) -> Option<Rect> {
        let g = geo(cx.width(), cx.height(), &look(rapidr_value::theme::current()));
        list::editor_ime_area(cx, editor_box(text_area(g.sbox)))
    }

    fn wants_ime(&self, _store: &dyn Store, id: &str) -> bool {
        searching(id)
    }

    fn context_menu(&self, cx: &mut Cx) -> Option<crate::components::edit::MenuState> {
        let g = geo(cx.width(), cx.height(), &look(rapidr_value::theme::current()));
        list::editor_menu(cx, editor_box(text_area(g.sbox)))
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        let mut n = crate::components::shared_describe(cx, self.name());
        n.role = Role::Tree;
        if n.name.is_empty() {
            n.name = "Toolbox".into();
        }
        let l = look(rapidr_value::theme::current());
        let g = geo(cx.width(), cx.height(), &l);
        let (x0, y0) = (cx.rect.0, cx.rect.1);
        let typing = searching(cx.id);
        // (the search box)
        let mut s = AccessNode::new(part_id(cx.id, PART_ITEM, PART_SEARCH), Role::TextInput);
        s.name = "Search components".into();
        s.value = Some(editing(cx.id).map(|e| e.text).unwrap_or_else(|| model(cx.id, |m| m.filter.clone())));
        s.bounds = (x0 + g.sbox.0, y0 + g.sbox.1, g.sbox.2, g.sbox.3);
        s.states.focused = typing && cx.state.focused;
        s.actions = vec![Action::Focus, Action::SetValue];
        n.children.push(s);
        // (the rows, a group's inside it)
        let (rows, cursor, selected, _) = Self::state(cx.id);
        let selected = cursor.clone().filter(Key::is_item).or(selected);
        let (pos, cw, _) = scroll(cx.id, g.list, rows.len());
        let list_focus = cx.state.focused && !typing;
        let mut stack: Vec<AccessNode> = Vec::new();
        let mut levels: Vec<usize> = Vec::new();
        let close = |stack: &mut Vec<AccessNode>, levels: &mut Vec<usize>, out: &mut Vec<AccessNode>, down_to: usize| {
            while levels.last().is_some_and(|&lv| lv >= down_to) {
                let done = stack.pop().unwrap_or_else(|| AccessNode::new(0, Role::TreeItem));
                levels.pop();
                match stack.last_mut() {
                    Some(parent) => parent.children.push(done),
                    None => out.push(done),
                }
            }
        };
        let mut out = Vec::new();
        for (i, r) in rows.iter().enumerate() {
            let Some(part) = Self::part_of(cx.id, &r.key) else { continue };
            let mut item = AccessNode::new(part_id(cx.id, PART_ITEM, part), Role::TreeItem);
            item.level = Some(r.level);
            item.bounds = (x0 + g.list.0, y0 + g.list.1 + i as i64 * ROW - pos, cw, ROW);
            match &r.key {
                Key::Group(_) => {
                    item.name = r.text.clone();
                    item.description = format!("{} components", r.count);
                    item.states.expanded = r.open;
                    item.actions = vec![Action::Click, Action::Focus, if r.open == Some(true) { Action::Collapse } else { Action::Expand }];
                }
                key => {
                    let title = match key {
                        Key::Component(t) => model(cx.id, |m| m.name_of(t)) + " — " + &title_words(t),
                        _ => format!("{} — template", r.text),
                    };
                    item.name = title;
                    if let Key::Component(t) = key {
                        item.description = model::description(t);
                    }
                    item.states.selected = Some(selected.as_ref() == Some(key));
                    item.actions = vec![Action::Click, Action::Focus];
                }
            }
            item.states.focused = list_focus && cursor.as_ref() == Some(&r.key);
            close(&mut stack, &mut levels, &mut out, r.level);
            if matches!(r.key, Key::Group(_)) {
                stack.push(item);
                levels.push(r.level);
            } else {
                match stack.last_mut() {
                    Some(parent) => parent.children.push(item),
                    None => out.push(item),
                }
            }
        }
        close(&mut stack, &mut levels, &mut out, 0);
        n.children.extend(out);
        n
    }

    fn access(&self, cx: &mut Cx, action: Action, part: Option<usize>, value: Option<&AccessValue>) -> bool {
        let Some(part) = part else { return false };
        if part == PART_SEARCH {
            match action {
                Action::Focus => start_search(cx.id, false),
                Action::SetValue => {
                    let text = match value {
                        Some(AccessValue::Text(t)) => t.clone(),
                        Some(AccessValue::Number(n)) => n.to_string(),
                        None => String::new(),
                    };
                    if searching(cx.id) {
                        set_edit_text(cx.id, &text);
                    }
                    filter_changed(cx, &text);
                }
                _ => return false,
            }
            return true;
        }
        let Some(key) = Self::key_of_part(cx.id, part) else { return false };
        match (&key, action) {
            (Key::Group(g), Action::Expand | Action::Collapse) => toggle(cx, g, Some(action == Action::Expand)),
            (Key::Group(g), Action::Click) => toggle(cx, g, None),
            (_, Action::Focus) => {
                if searching(cx.id) {
                    stop_search(cx);
                }
                go_to(cx, key);
            }
            (_, Action::Click) => {
                go_to(cx, key.clone());
                pick(cx, key);
            }
            _ => return false,
        }
        true
    }
}

/// A component's title in words (its icon's: "Button").
fn title_words(type_name: &str) -> String {
    rapidr_icons::component(type_name).map(|i| i.title.to_string()).unwrap_or_else(|| model::written_name(type_name))
}

/// A child's z-order: a toolbox dragging a component over its neighbours
/// (its ghost drawn over them).
pub fn stacked(_parent: &str, mut children: Vec<(String, String)>) -> Vec<(String, String)> {
    let dragging = |(c, t): &(String, String)| t.eq_ignore_ascii_case("RTOOLBOX") && model::with(c, |m| m.drag.as_ref().is_some_and(|d| d.started)).unwrap_or(false);
    if let Some(i) = children.iter().position(dragging) {
        let c = children.remove(i);
        children.push(c);
    }
    children
}

/// The focus left it while its search box was open (`super::focus_left`):
/// the filter stays.
pub fn focus_left(id: &str, ed: InPlace) -> Vec<KernelEvent> {
    model::with_mut(id, |m| m.set_filter(&ed.text));
    Vec::new()
}

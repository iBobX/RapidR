//! QCOMBOBOX: the shared list model (`ItemList`, `combo: true`) holds the
//! items, ItemIndex and Text; this draws Windows' classic combo box — a
//! sunken white box showing the Text (or, owner-drawn — Style
//! csOwnerDrawFixed / csOwnerDrawVariable — the selected item as
//! OnDrawItem drew it) and a drop-down button — and its drop-down list.
//!
//! The drop-down is drawn by the kernel over the form, under the box (above
//! it when there's no room below), on top of every component, and takes
//! the mouse first while open (plan §1.5 "Popups": an in-window overlay; a
//! borderless top-level window is the host's later refinement). A pick sets
//! ItemIndex and Text and fires OnChange, as FLTK's Choice and the owner-
//! drawn combo do; Up / Down pick the item before / after; Alt+Down, F4 or
//! a click on the box open the list, Escape closes it.

use std::cell::RefCell;

use rapidr_value::input::Button;
use rapidr_value::objects::a11y::{node_id, part_id, AccessNode, Action, Role};
use rapidr_value::objects::ops::{Place, Rect};
use rapidr_value::objects::{with_list, with_list_mut};

use super::list::{picture_of, view_size};
use super::{ComponentKind, Cx, KeyIn, MouseIn, MouseKind, MouseOut};
use crate::a11y::AccessValue;
use crate::input::Clipboard;
use crate::paint::{Painter, DARK, FACE, GRAY_TEXT, HIGHLIGHT, HIGHLIGHT_TEXT, LIGHT, SHADOW};
use crate::store::Store;
use crate::text::{bgr_to_rgb, TextSystem};
use crate::tree::FormUi;

/// The most rows the drop-down shows (Windows' default DropDownCount).
const DROP_ROWS: usize = 8;

/// The open drop-down: (form, combo id, the row under the mouse).
#[derive(Clone, Debug, PartialEq)]
struct Dropped {
    form: String,
    id: String,
    hot: Option<usize>,
    /// The first row shown (scrolled with the wheel or the keys).
    top: usize,
    /// Another component's list (a grid's gcsList column): its items and
    /// the cell it drops from (absolute); `None`: the combo's own.
    items: Option<Vec<String>>,
    anchor: Option<Rect>,
}

thread_local! {
    static DROPPED: RefCell<Option<Dropped>> = const { RefCell::new(None) };
}

fn dropped() -> Option<Dropped> {
    DROPPED.with(|d| d.borrow().clone())
}

/// Closes any open drop-down.
pub fn close() {
    DROPPED.with(|d| *d.borrow_mut() = None);
}

/// Whether combo `id`'s list is down.
pub fn is_dropped(id: &str) -> bool {
    dropped().is_some_and(|d| d.id.eq_ignore_ascii_case(id))
}

fn open(form: &str, id: &str) {
    let top = with_list(id, |l| usize::try_from(l.item_index).unwrap_or(0).saturating_sub(DROP_ROWS - 1)).unwrap_or(0);
    DROPPED.with(|d| *d.borrow_mut() = Some(Dropped { form: form.to_lowercase(), id: id.to_lowercase(), hot: None, top, items: None, anchor: None }));
}

/// Drops a list of `items` from `anchor` (a cell, absolute in form `form`'s
/// client area) for component `id`: a pick is `ListAction::GridStore` (a
/// grid's gcsList column, once OnListDropDown answered its items).
pub fn open_list(form: &str, id: &str, items: Vec<String>, anchor: Rect) {
    if items.is_empty() {
        return;
    }
    DROPPED.with(|d| *d.borrow_mut() = Some(Dropped { form: form.to_lowercase(), id: id.to_lowercase(), hot: None, top: 0, items: Some(items), anchor: Some(anchor) }));
}

/// The drop-down's button: as wide as a scroll bar, inside the frame.
fn button_rect(w: i64, h: i64) -> Rect {
    let bw = rapidr_value::scrollbars::BAR.min(w - 4).max(0);
    (w - 2 - bw, 2, bw, (h - 4).max(0))
}

/// Each row's height in the drop-down (owner-drawn: its own).
fn row_heights(id: &str, font_h: i64) -> Vec<i64> {
    with_list(id, |l| {
        (0..l.items.len().min(rapidr_value::objects::list::MAX_OWNER_DRAWN))
            .map(|i| if l.owner_drawn() { l.item_h(i) } else if l.item_height > 0 { l.row_height() } else { font_h + 3 })
            .collect()
    })
    .unwrap_or_default()
}

/// Where the open list is in the client area (the combo at `abs`) and its
/// rows (index, top within it, height).
/// The rows of an open list: (index, top within it, height).
type Rows = Vec<(usize, i64, i64)>;

fn layout(f: &FormUi, d: &Dropped, store: &dyn Store) -> Option<(Rect, Rows)> {
    let (x, y, w, h) = match d.anchor {
        Some(a) => a,
        None => f.node(&d.id)?.abs,
    };
    let font = store.font(&d.id);
    let heights = match &d.items {
        Some(items) => vec![font.pixel_size() + 3; items.len()],
        None => row_heights(&d.id, font.pixel_size()),
    };
    if heights.is_empty() {
        return None;
    }
    let mut rows = Vec::new();
    let mut at = 1;
    for (i, &rh) in heights.iter().enumerate().skip(d.top).take(DROP_ROWS) {
        rows.push((i, at, rh));
        at += rh;
    }
    let lh = at + 1;
    let ch = f.client.1 + f.menu_offset;
    // (below the box, else above it if it fits there better)
    let below = y + h;
    let top = if below + lh > ch && y - lh >= f.menu_offset { y - lh } else { below };
    Some(((x, top, w, lh), rows))
}

/// The wheel while form `f`'s drop-down is open: it scrolls (three rows a
/// notch); whether it took it. (The text lane's wheel routing.)
pub fn popup_wheel(f: &mut FormUi, store: &dyn Store, _x: f64, _y: f64, dy: f64) -> bool {
    let Some(d) = dropped().filter(|d| d.form == f.form) else { return false };
    let count = match &d.items {
        Some(items) => items.len(),
        None => row_heights(&d.id, store.font(&d.id).pixel_size()).len(),
    };
    let rows = (dy * 3.0).round() as i64;
    let max = count.saturating_sub(DROP_ROWS) as i64;
    let top = (d.top as i64 + rows).clamp(0, max.max(0)) as usize;
    DROPPED.with(|dd| {
        if let Some(dd) = dd.borrow_mut().as_mut() {
            dd.top = top;
        }
    });
    f.dirty = true;
    true
}

/// Draws the open drop-down over everything (FormUi::paint's last step).
pub fn paint_popup(f: &FormUi, store: &dyn Store, _ts: &mut TextSystem, p: &mut Painter) {
    let Some(d) = dropped().filter(|d| d.form == f.form) else { return };
    let Some(((x, y, w, h), rows)) = layout(f, &d, store) else { return };
    let font = store.font(&d.id);
    let l = match &d.items {
        Some(items) => {
            let mut l = rapidr_value::objects::list::ItemList::new(true);
            l.items = items.clone();
            l
        }
        None => match with_list(&d.id, |l| l.clone()) {
            Some(l) => l,
            None => return,
        },
    };
    p.at((x, y), |p| {
        p.fill((0, 0, w, h), 0xFFFFFF);
        p.edge((0, 0, w, h), &[0x000000], &[0x000000]);
        p.clipped((1, 1, w - 2, h - 2), |p| {
            for (i, top, rh) in rows {
                let hot = d.hot.map_or(l.item_index == i as i64, |hi| hi == i);
                if l.owner_drawn() {
                    let shown = l.render_item(i, w - 8, &font).display_rgba();
                    p.picture(&format!("{}#drop{i}", d.id), 0, picture_of(shown), (1, top, w - 8, rh));
                    if hot {
                        p.focus((1, top, w - 2, rh));
                    }
                    continue;
                }
                if hot {
                    p.fill((1, top, w - 2, rh), HIGHLIGHT);
                }
                let text = l.items[i].replace(['\n', '\r', '\t'], " ");
                p.text((3, top, w - 6, rh), &text, &font, if hot { HIGHLIGHT_TEXT } else { bgr_to_rgb(font.color) }, Place::Left);
            }
        });
    });
}

/// The row of the open list at (x, y) of the client area: `Some(None)`
/// inside it but on no row, `None` outside it.
fn row_at(f: &FormUi, d: &Dropped, store: &dyn Store, x: f64, y: f64) -> Option<Option<usize>> {
    let ((lx, ly, lw, lh), rows) = layout(f, d, store)?;
    let (x, y) = (x.floor() as i64 - lx, y.floor() as i64 - ly);
    if x < 0 || y < 0 || x >= lw || y >= lh {
        return None;
    }
    Some(rows.iter().find(|(_, t, h)| y >= *t && y < t + h).map(|r| r.0))
}

/// The mouse pressed while a list is down (FormUi::mouse_down's first
/// step): a row picks it, anywhere else closes the list. Whether the press
/// was the drop-down's (a press on the combo's own box closes the list
/// too, as Windows' does).
pub fn popup_mouse_down(f: &mut FormUi, store: &dyn Store, x: f64, y: f64) -> bool {
    let Some(d) = dropped() else { return false };
    if d.form != f.form {
        return false;
    }
    close();
    f.dirty = true;
    match row_at(f, &d, store, x, y) {
        Some(Some(i)) => {
            match d.items.as_ref().and_then(|items| items.get(i)) {
                Some(item) => f.events.push(crate::input::KernelEvent::List(d.id.clone(), super::list::ListAction::GridStore(item.clone()))),
                None => pick(f, &d.id, i),
            }
            true
        }
        Some(None) => true,
        // (outside: the press closes it; on the combo itself it's taken)
        None => f.node(&d.id).is_some_and(|n| {
            let (nx, ny, nw, nh) = n.abs;
            x >= nx as f64 && y >= ny as f64 && x < (nx + nw) as f64 && y < (ny + nh) as f64
        }),
    }
}

/// The mouse moved while a list is down: the row under it lights.
pub fn popup_mouse_move(f: &mut FormUi, store: &dyn Store, x: f64, y: f64) {
    let Some(d) = dropped() else { return };
    if d.form != f.form {
        return;
    }
    if let Some(Some(i)) = row_at(f, &d, store, x, y) {
        if d.hot != Some(i) {
            DROPPED.with(|dd| {
                if let Some(dd) = dd.borrow_mut().as_mut() {
                    dd.hot = Some(i);
                }
            });
            f.dirty = true;
        }
    }
}

/// Item `i` picked from the list: ItemIndex, Text, OnChange.
fn pick(f: &mut FormUi, id: &str, i: usize) {
    with_list_mut(id, |l| l.select(i as i64));
    f.events.push(crate::input::KernelEvent::Change(id.to_lowercase()));
}

pub struct ComboBox;

/// Whether combo `id`'s box is an edit (Style csDropDown 0 / csSimple 1,
/// not owner-drawn): the text lane's editor shows and edits its Text.
fn editable(store: &dyn Store, id: &str) -> bool {
    with_list(id, |l| !l.owner_drawn()).unwrap_or(false) && store.get(id, "style").to_i64() < 2
}

/// An editable box's text area (inside the frame, left of the button).
fn text_area(w: i64, h: i64) -> Rect {
    let (bx, _, _, _) = button_rect(w, h);
    (4, 2, (bx - 5).max(0), (h - 4).max(0))
}

impl ComboBox {
    /// Picks item `i` (the keys, a screen reader): ItemIndex, OnChange.
    fn choose(cx: &mut Cx, i: i64) {
        let changed = with_list_mut(cx.id, |l| {
            let before = l.item_index;
            l.select(i);
            l.item_index != before
        });
        if changed.unwrap_or(false) {
            cx.change();
        }
    }
}

impl ComponentKind for ComboBox {
    fn name(&self) -> &'static str {
        "RCOMBOBOX"
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let (w, h) = (cx.width(), cx.height());
        let s = cx.state;
        p.fill((0, 0, w, h), if s.enabled { 0xFFFFFF } else { FACE });
        p.edge((0, 0, w, h), &[SHADOW, DARK], &[LIGHT, FACE]);
        let (bx, by, bw, bh) = button_rect(w, h);
        let Some(mut l) = with_list(cx.id, |l| l.clone()) else { return };
        let editable = !l.owner_drawn() && cx.store.get(cx.id, "style").to_i64() < 2;
        let area = (2, 2, (bx - 2).max(0), h - 4);
        let font = cx.font.clone();
        if l.owner_drawn() {
            let (vw, vh) = view_size(&l, w, h);
            l.set_view(vw, vh);
            with_list_mut(cx.id, |m| m.set_view(vw, vh));
            if let Ok(i) = usize::try_from(l.item_index) {
                if i < l.items.len() {
                    let ih = l.item_h(i);
                    let shown = l.render_item(i, area.2, &font).display_rgba();
                    p.clipped(area, |p| p.picture(&format!("{}#box", cx.id), 0, picture_of(shown), (2, 2 + (area.3 - ih) / 2, area.2, ih)));
                }
            }
        } else if editable {
            // (the text lane's editor over the list's Text)
            super::edit::paint_line(cx, p, text_area(w, h), super::edit::Source::Combo);
        } else {
            let text = if l.item_index < 0 { l.text.clone() } else { l.items.get(l.item_index as usize).cloned().unwrap_or_default() };
            // (a list-only combo with the focus shows its text selected)
            let selected = s.focused && !editable;
            if selected {
                p.fill((area.0 + 1, area.1 + 1, area.2 - 2, area.3 - 2), HIGHLIGHT);
            }
            let color = if !s.enabled { GRAY_TEXT } else if selected { HIGHLIGHT_TEXT } else { bgr_to_rgb(font.color) };
            p.clipped(area, |p| p.text((area.0 + 2, area.1, area.2 - 2, area.3), &text.replace(['\n', '\r', '\t'], " "), &font, color, Place::Left));
            if selected {
                p.focus((area.0 + 1, area.1 + 1, area.2 - 2, area.3 - 2));
            }
        }
        if s.focused && l.owner_drawn() {
            p.focus((area.0 + 1, area.1 + 1, area.2 - 2, area.3 - 2));
        }
        // (the button: raised, pushed while the list is down)
        let down = is_dropped(cx.id);
        p.fill((bx, by, bw, bh), FACE);
        if down {
            p.edge((bx, by, bw, bh), &[SHADOW], &[SHADOW]);
        } else {
            p.edge((bx, by, bw, bh), &[FACE, LIGHT], &[DARK, SHADOW]);
        }
        let d = f64::from(u8::from(down));
        let (cxm, cym) = (bx as f64 + bw as f64 / 2.0 + d, by as f64 + bh as f64 / 2.0 + d);
        let arrow = if s.enabled { 0x000000 } else { GRAY_TEXT };
        p.op(rapidr_value::objects::ops::Op::Arrow { points: [(cxm - 4.0, cym - 2.0), (cxm + 4.0, cym - 2.0), (cxm, cym + 2.0)], color: arrow });
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        let (w, h) = (cx.width(), cx.height());
        if editable(cx.store, cx.id) {
            // (an editable box: a press on the text places the caret; only
            // the button drops the list)
            let on_text = m.x < button_rect(w, h).0 as f64;
            let held_text = cx.ui.edit.as_ref().is_some_and(|_| m.kind != MouseKind::Down && m.captured && !is_dropped(cx.id));
            if (m.kind == MouseKind::Down && on_text) || held_text {
                super::edit::mouse_line(cx, m, text_area(w, h), super::edit::Source::Combo);
                return MouseOut::default();
            }
        }
        if m.kind == MouseKind::Down && m.button == Button::Left {
            DROP_REQUEST.with(|r| *r.borrow_mut() = Some(cx.id.to_string()));
        }
        MouseOut::default()
    }

    fn ime(&self, cx: &mut Cx, ime: &super::Ime) -> bool {
        if !editable(cx.store, cx.id) {
            return false;
        }
        let (w, h) = (cx.width(), cx.height());
        let area = text_area(w, h);
        let spec = super::edit::Spec::line(cx, area, super::edit::Source::Combo);
        super::edit::ime_box(cx, &spec, ime, |e| {
            let s = f64::from(e.ed.scale());
            ((area.2 as f64 * s).max(1.0), (area.3 as f64 * s).max(1.0))
        });
        true
    }

    fn ime_area(&self, cx: &mut Cx) -> Option<Rect> {
        if !editable(cx.store, cx.id) {
            return None;
        }
        let (w, h) = (cx.width(), cx.height());
        Some(super::edit::ime_area_line(cx, text_area(w, h), super::edit::Source::Combo))
    }

    fn wants_ime(&self, store: &dyn Store, id: &str) -> bool {
        editable(store, id)
    }

    fn context_menu(&self, cx: &mut Cx) -> Option<super::edit::MenuState> {
        if !editable(cx.store, cx.id) {
            return None;
        }
        let (w, h) = (cx.width(), cx.height());
        Some(super::edit::menu_line(cx, text_area(w, h), super::edit::Source::Combo))
    }

    fn key(&self, cx: &mut Cx, k: &KeyIn, clip: &mut dyn Clipboard) -> bool {
        // (an editable box: what isn't the list's goes to the editor)
        let list_key = matches!(k.vk, 38 | 40 | 115) || (is_dropped(cx.id) && matches!(k.vk, 13 | 27));
        if editable(cx.store, cx.id) && !list_key {
            let (w, h) = (cx.width(), cx.height());
            return super::edit::key_line(cx, k, clip, text_area(w, h), super::edit::Source::Combo);
        }
        if k.mods.command {
            return false;
        }
        let Some((index, count)) = with_list(cx.id, |l| (l.item_index, l.items.len() as i64)) else { return false };
        if is_dropped(cx.id) {
            match k.vk {
                27 => close(),
                13 => {
                    let hot = dropped().and_then(|d| d.hot);
                    close();
                    if let Some(i) = hot {
                        Self::choose(cx, i as i64);
                    }
                }
                38 | 40 => {
                    let next = (index + if k.vk == 40 { 1 } else { -1 }).clamp(0, (count - 1).max(0));
                    if count > 0 {
                        Self::choose(cx, next);
                    }
                }
                _ => return false,
            }
            return true;
        }
        if (k.vk == 40 && k.mods.alt) || k.vk == 115 {
            DROP_REQUEST.with(|r| *r.borrow_mut() = Some(cx.id.to_string()));
            return true;
        }
        if k.mods.alt {
            return false;
        }
        let next = match k.vk {
            40 => index + 1,
            38 => (index - 1).max(0),
            36 => 0,
            35 => count - 1,
            _ => return false,
        };
        let next = next.clamp(0, (count - 1).max(0));
        if count > 0 && next != index {
            Self::choose(cx, next);
        }
        true
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        let mut n = AccessNode::new(node_id(cx.id), Role::ComboBox);
        let (text, items, index) = with_list(cx.id, |l| (l.text.clone(), l.items.clone(), l.item_index)).unwrap_or_default();
        n.value = Some(text);
        n.states.expanded = Some(is_dropped(cx.id));
        n.actions = vec![Action::Focus, Action::Expand, Action::Collapse];
        n.bounds = cx.rect;
        for (i, item) in items.into_iter().enumerate().take(1_000) {
            let mut o = AccessNode::new(part_id(cx.id, 1, i), Role::ListBoxOption);
            o.name = item;
            o.states.selected = Some(index == i as i64);
            o.actions = vec![Action::Click];
            n.children.push(o);
        }
        n
    }

    fn access(&self, cx: &mut Cx, action: Action, part: Option<usize>, _value: Option<&AccessValue>) -> bool {
        match (action, part) {
            (Action::Click, Some(i)) => {
                close();
                Self::choose(cx, i as i64);
                true
            }
            (Action::Expand, None) => {
                DROP_REQUEST.with(|r| *r.borrow_mut() = Some(cx.id.to_string()));
                true
            }
            (Action::Collapse, None) => {
                close();
                true
            }
            _ => false,
        }
    }

    /// `__item_i`: the list dropped and item `i` clicked in it (picked
    /// directly when it isn't among the rows shown).
    fn test_action(&self, cx: &mut Cx, action: &str) -> bool {
        let Some(i) = action.strip_prefix("__item_").and_then(|s| s.parse::<usize>().ok()) else { return false };
        let count = with_list(cx.id, |l| l.items.len()).unwrap_or(0);
        if i < count {
            TEST_PICK.with(|t| *t.borrow_mut() = Some((cx.id.to_string(), i)));
        }
        true
    }
}

thread_local! {
    /// A combo asked to drop its list (FormUi opens it: the combo's Cx
    /// doesn't know its form).
    static DROP_REQUEST: RefCell<Option<String>> = const { RefCell::new(None) };
    /// A test's pick through the list, done by [`after_input`].
    static TEST_PICK: RefCell<Option<(String, usize)>> = const { RefCell::new(None) };
}

/// After the kernel routed input into form `f`: a combo's request to drop
/// its list opens it, and a test's pick clicks the row (through
/// [`popup_mouse_down`], as the user's click would go).
pub fn after_input(f: &mut FormUi, store: &dyn Store) {
    if let Some(id) = DROP_REQUEST.with(|r| r.borrow_mut().take()) {
        if is_dropped(&id) {
            close();
        } else if f.node(&id).is_some() {
            open(&f.form, &id);
        }
        f.dirty = true;
    }
    if let Some((id, i)) = TEST_PICK.with(|t| t.borrow_mut().take()) {
        if f.node(&id).is_none() {
            return;
        }
        open(&f.form, &id);
        // (the row brought into the shown ones)
        DROPPED.with(|d| {
            if let Some(d) = d.borrow_mut().as_mut() {
                if i < d.top || i >= d.top + DROP_ROWS {
                    d.top = i.saturating_sub(DROP_ROWS - 1);
                }
            }
        });
        let d = dropped().expect("just opened");
        match layout(f, &d, store).and_then(|(r, rows)| rows.iter().find(|row| row.0 == i).map(|row| (r, row.1, row.2))) {
            Some(((lx, ly, lw, _), top, rh)) => {
                let (x, y) = ((lx + lw / 2) as f64 + 0.5, (ly + top + rh / 2) as f64 + 0.5);
                popup_mouse_down(f, store, x, y);
            }
            None => {
                close();
                pick(f, &id, i);
            }
        }
        f.dirty = true;
    }
}

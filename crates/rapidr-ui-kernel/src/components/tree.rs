//! QTREEVIEW (and QOUTLINE, a tree of lines): the shared model
//! (`rapidr_value::objects::tree::TreeView`) holds the nodes, lays out
//! their rows (`TreeView::rows`: buttons, lines, levels) and finds what a
//! click lands on; this draws Windows' classic tree view (a sunken white
//! box, dotted lines, ± buttons, the nodes' icons from Images /
//! StateImages, the selected node's text white on blue) and routes the
//! mouse and keys.
//!
//! What the user does is the program's to allow (OnChanging, OnExpanding /
//! OnCollapsing, OnEditing, OnEdited answer back), so a click queues a
//! [`ListAction`] that runtime-core carries out after the pump: a click on
//! a node asks to select it, then fires OnClick; on its button OnClick,
//! then asks to expand or collapse it; F2 — or a click on the selected
//! node, after Windows' double-click time unless a double click comes —
//! asks to edit the selected node's text, and its editor's Enter asks
//! OnEdited.

use std::cell::RefCell;
use std::collections::HashMap;

use rapidr_value::objects::a11y::{part_id, AccessNode, Action, PART_ITEM};
use rapidr_value::objects::ops::{Place, Rect};
use rapidr_value::objects::tree::{Hit, Row, BUTTON, ROW_HEIGHT};

/// A tree's rows' height (its ItemHeight, or 16).
fn row_h(id: &str) -> i64 {
    with_tree(id, |t| t.row_height()).unwrap_or(ROW_HEIGHT)
}
use rapidr_value::objects::with_tree;

use super::list::{act, background, bar_mouse, bar_tick, begin_edit, drop_editor, edit_key, editing, editor_ime, editor_ime_area, editor_menu, editor_mouse, end_edit, fire, paint_editor, picture_of, set_edit_text, sunken, vscroll_at, vscroll_state, InPlace, ListAction};
use rapidr_value::input::Cursor;
use super::{ComponentKind, Cx, Ime, KeyIn, MouseIn, MouseKind, MouseOut};
use crate::a11y::AccessValue;
use crate::input::Clipboard;
use crate::paint::{ink, Painter};
use crate::store::{self, Store};

thread_local! {
    /// (the input lane's) A click on the selected node of a focused tree:
    /// its edit asked for when the double-click time is up (its `tick`),
    /// unless a press or a key comes first — Windows' tree view, the web's
    /// 0.5 s timer.
    static EDIT_SOON: RefCell<HashMap<String, usize>> = RefCell::new(HashMap::new());
}

/// (the input lane's) A pending edit-after-a-pause forgotten.
fn cancel_edit_soon(cx: &mut Cx) {
    if EDIT_SOON.with(|e| e.borrow_mut().remove(cx.id)).is_some() {
        cx.ui.wake = None;
    }
}

pub struct Tree;

/// A node's icon (its state image beside its image), as the screen shows
/// it, and its logical size.
fn icon(cx: &Cx, node: usize, selected: bool) -> Option<(crate::display::Picture, i64, i64)> {
    let images = store::string(cx.store, cx.id, "images");
    let states = store::string(cx.store, cx.id, "stateimages");
    if images.is_empty() && states.is_empty() {
        return None;
    }
    let (image, sel, state) = with_tree(cx.id, |t| t.nodes.get(node).map(|n| (n.image_index, n.selected_index, n.state_index))).flatten()?;
    let px = rapidr_value::objects::tree_icon(&images, &states, if selected { sel } else { image }, state)?;
    let scale = px.3.max(1) as i64;
    let (w, h) = (px.0 as i64 / scale, px.1 as i64 / scale);
    Some((picture_of(px), w, h))
}

/// The rows as drawn now (inside the frame; TopIndex's row first).
fn rows(cx: &Cx) -> Vec<Row> {
    let h = cx.height() - 4;
    with_tree(cx.id, |t| t.rows(t.row_height(), h)).unwrap_or_default()
}

/// Where node `n`'s text box is (in the component), with its icon's room.
fn text_left(cx: &Cx, row: &Row) -> i64 {
    2 + row.left + icon(cx, row.node, row.selected).map_or(0, |(_, w, _)| w + 2)
}

/// A vertical dotted line at x from y0 to y1 (both included), and a
/// horizontal one: Windows' tree lines (every other pixel, on even sums).
// (the lines: the theme's, dotted every other pixel as Windows draws them)
fn dotted_v(p: &mut Painter, x: i64, y0: i64, y1: i64) {
    let lines = p.theme().lines;
    let mut y = y0 + (x + y0).rem_euclid(2);
    while y <= y1 {
        p.fill((x, y, 1, 1), lines);
        y += 2;
    }
}

fn dotted_h(p: &mut Painter, x0: i64, x1: i64, y: i64) {
    let lines = p.theme().lines;
    let mut x = x0 + (x0 + y).rem_euclid(2);
    while x <= x1 {
        p.fill((x, y, 1, 1), lines);
        x += 2;
    }
}

/// A node's expand / collapse button centred on (c, mid): Windows' boxed
/// plus and minus (classic); a fluent theme's chevron (down: expanded).
pub fn expander(p: &mut Painter, c: i64, mid: i64, side: i64, expanded: bool) {
    let t = p.theme();
    if t.fluent() {
        p.chevron(c as f64 + 0.5, mid as f64 + 0.5, 7.0, expanded, t.border_strong);
        return;
    }
    let (bx, by) = (c - side / 2, mid - side / 2);
    p.fill((bx, by, side, side), t.window);
    p.frame((bx, by, side, side), t.shadow);
    p.fill((bx + 2, mid, side - 4, 1), t.text);
    if !expanded {
        p.fill((c, by + 2, 1, side - 4), t.text);
    }
}

impl Tree {
    /// The node's row in the component (its rect), if shown.
    fn row_rect(cx: &Cx, n: usize) -> Option<(Row, Rect)> {
        let row = rows(cx).into_iter().find(|r| r.node == n)?;
        let r = (2, 2 + row.top, cx.width() - 4, row.height);
        Some((row, r))
    }

    /// An edit going on ends (a click elsewhere keeps it, as Windows does).
    fn finish_edit(cx: &mut Cx, keep: bool) {
        if let Some(ed) = end_edit(cx.id) {
            cx.ui.edit = None;
            if keep {
                act(cx, ListAction::TreeEdited(ed.target.0, ed.text));
            }
        }
    }

    /// The bar moved: TopIndex follows, a row at a time.
    fn follow_bar(cx: &Cx) {
        let (pos, _, _) = vscroll_state(cx.id);
        with_tree(cx.id, |t| {
            let rows = t.visible_rows();
            if let Some(&r) = rows.get((pos / t.row_height()) as usize) {
                t.top_index = r as i64;
            }
        });
    }

    /// (the input lane's) The edit's box in the component — over the node's
    /// text (its icon's right), to the rows' right edge (at least 40 wide)
    /// — while one goes on and its row shows.
    fn edit_rect(cx: &Cx) -> Option<Rect> {
        let ed = editing(cx.id)?;
        let (row, (_, y, _, rh)) = Self::row_rect(cx, ed.target.0)?;
        let x = text_left(cx, &row);
        let (_, bar, _) = vscroll_state(cx.id);
        let cw = cx.width() - 4 - if bar { rapidr_value::scrollbars::BAR } else { 0 };
        Some((x, y, (cw - (x - 2) - 3).max(40), rh))
    }

    /// A press on node `n` (not on its button): asks to select it, then
    /// OnClick (a double click's second press: OnDblClick too, as the
    /// web's tree fires them).
    fn press(cx: &mut Cx, n: usize, double: bool) {
        let selected = with_tree(cx.id, |t| t.item_index == n as i64).unwrap_or(false);
        if !selected {
            act(cx, ListAction::TreeSelect(n));
        }
        cx.click();
        if double {
            fire(cx, "ondblclick", Vec::new());
        }
    }

    /// A press on node `n`'s button: OnClick, then asks to expand or
    /// collapse it.
    fn toggle(cx: &mut Cx, n: usize) {
        let open = !with_tree(cx.id, |t| t.nodes.get(n).is_some_and(|x| x.expanded)).unwrap_or(true);
        cx.click();
        act(cx, ListAction::TreeToggle(n, open));
    }
}

impl ComponentKind for Tree {
    fn name(&self) -> &'static str {
        "RTREEVIEW"
    }

    fn field(&self) -> bool {
        true
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        drop_editor(cx);
        let (w, h) = (cx.width(), cx.height());
        let back = background(cx);
        sunken(p, w, h, back);
        let Some((count, top_row, hide)) = with_tree(cx.id, |t| {
            let rows = t.visible_rows();
            let top = rows.iter().position(|&r| r as i64 >= t.top_index).unwrap_or(0) as i64;
            (rows.len() as i64, top, t.hide_selection)
        }) else {
            return;
        };
        // (the bar follows TopIndex)
        let (_, cw, bar) = vscroll_at(cx.id, w - 4, h - 4, count * row_h(cx.id), row_h(cx.id), Some(top_row * row_h(cx.id)));
        let rows = rows(cx);
        let font = cx.font.clone();
        let focused = cx.state.focused;
        let enabled = cx.state.enabled;
        let (show_lines, rows_all) = (with_tree(cx.id, |t| t.show_lines).unwrap_or(true), rows.clone());
        p.at((2, 2), |p| {
            p.clipped((0, 0, cw, h - 4), |p| {
                for row in &rows_all {
                    let mid = row.top + row.height / 2;
                    // (RapidR's look: the selection across the row, under
                    // its lines, icon and text)
                    if p.fluent() && row.selected && (focused || !hide) {
                        super::list::selected_row(p, (0, row.top, cw, row.height), focused);
                    }
                    if show_lines {
                        for &x in &row.through {
                            dotted_v(p, x, row.top, row.top + row.height - 1);
                        }
                        if let Some(c) = row.center {
                            // (up to the parent or the row above, on down to the next sibling)
                            let first = row.top == 0 && rows_all.first().is_some_and(|f| f.node == row.node) && row.node == 0;
                            let y0 = if first { mid } else { row.top };
                            let y1 = if row.more { row.top + row.height - 1 } else { mid };
                            dotted_v(p, c, y0, y1);
                            dotted_h(p, c, row.left - 3, mid);
                        }
                    }
                    if let (Some(c), Some(expanded)) = (row.center, row.button) {
                        expander(p, c, mid, BUTTON, expanded);
                    }
                    let mut x = row.left;
                    if let Some((pic, iw, ih)) = icon(cx, row.node, row.selected) {
                        p.picture(&format!("{}#icon{}", cx.id, row.node), 0, pic, (x, row.top + (row.height - ih) / 2, iw, ih));
                        x += iw + 2;
                    }
                    let text = with_tree(cx.id, |t| t.nodes.get(row.node).map(|n| n.text.clone())).flatten().unwrap_or_default();
                    let (tw, _) = rapidr_value::objects::text::text_size(&text, &font);
                    let tr = (x, row.top + 1, tw + 4, row.height - 2);
                    // (the selection: blue with the focus, grey without; none
                    // without the focus and with HideSelection)
                    let shown = row.selected && (focused || !hide);
                    let t = p.theme();
                    let color = if !enabled {
                        t.gray_text
                    } else if shown && focused {
                        t.highlight_text
                    } else if shown && t.fluent() {
                        t.text
                    } else {
                        ink(cx.store, cx.id, &font, true, if shown { t.unfocused } else { back })
                    };
                    if shown && !t.fluent() {
                        p.fill(tr, if focused { t.highlight } else { t.unfocused });
                    }
                    p.text((x + 2, row.top, tw + 2, row.height), &text, &font, color, Place::Left);
                    if shown && focused && !t.fluent() {
                        p.focus(tr);
                    }
                }
            });
            p.ops(bar);
        });
        // (the node's editor over its row, in the rows' area)
        if let Some(r) = Self::edit_rect(cx) {
            p.clipped((2, 2, cw, h - 4), |p| paint_editor(cx, p, r));
        }
    }

    fn wheel(&self, cx: &mut Cx, _dx: f64, dy: f64, _mods: crate::input::Mods) -> bool {
        if !super::list::vscroll_wheel(cx.id, dy, cx.width() - 4, cx.height() - 4) {
            return false;
        }
        // (TopIndex follows the bar, a row at a time)
        let (pos, _, _) = vscroll_state(cx.id);
        with_tree(cx.id, |t| {
            if let Some(&r) = t.visible_rows().get((pos / t.row_height()) as usize) {
                t.top_index = r as i64;
            }
        });
        true
    }

    /// A label being edited: the I-beam.
    fn pointer(&self, cx: &mut Cx, x: i64, y: i64) -> Cursor {
        Self::edit_rect(cx).map_or(Cursor::Default, |r| crate::components::list::editor_pointer(cx, r, x, y))
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        let (w, h) = (cx.width(), cx.height());
        let inner = MouseIn { x: m.x - 2.0, y: m.y - 2.0, ..*m };
        // (the input lane's: any press forgets a pending edit)
        if m.kind == MouseKind::Down {
            cancel_edit_soon(cx);
        }
        if bar_mouse(cx, &inner, w - 4, h - 4) {
            Self::follow_bar(cx);
            return MouseOut::default();
        }
        // (the edit's box: its editor's)
        if let Some(r) = Self::edit_rect(cx) {
            if editor_mouse(cx, m, r) {
                return MouseOut::default();
            }
        }
        if m.kind != MouseKind::Down {
            return MouseOut::default();
        }
        // (the input lane's: a press forgets a pending edit — a double
        // click's second press too)
        cancel_edit_soon(cx);
        let (x, y) = (m.x.floor() as i64 - 2, m.y.floor() as i64 - 2);
        let hit = with_tree(cx.id, |t| t.hit(x, y, t.row_height())).flatten();
        // (a press elsewhere ends an edit, keeping it)
        if editing(cx.id).is_some() {
            Self::finish_edit(cx, true);
        }
        match hit {
            Some(Hit::Button(n)) => Self::toggle(cx, n),
            Some(Hit::Row(n)) => {
                // (the selected node of a focused tree clicked: its edit
                // after a pause)
                let selected = with_tree(cx.id, |t| t.item_index == n as i64 && !t.read_only).unwrap_or(false);
                Self::press(cx, n, m.double());
                if selected && cx.state.focused && !m.double() {
                    EDIT_SOON.with(|e| e.borrow_mut().insert(cx.id.to_string(), n));
                    cx.ui.wake = Some(crate::tick::now() + crate::tick::DOUBLE_CLICK);
                }
            }
            // (no node: the selection stays, as Windows')
            None => {}
        }
        MouseOut::default()
    }

    fn key(&self, cx: &mut Cx, k: &KeyIn, clip: &mut dyn Clipboard) -> bool {
        // (while editing, the keys are the editor's — its row scrolled away
        // or not)
        if editing(cx.id).is_some() {
            let r = Self::edit_rect(cx).unwrap_or((2, 2, cx.width() - 4, row_h(cx.id)));
            if let Some(end) = edit_key(cx, k, clip, r) {
                if let Some(keep) = end {
                    Self::finish_edit(cx, keep);
                }
                return true;
            }
        }
        // (the input lane's: a key forgets a pending edit)
        cancel_edit_soon(cx);
        if k.mods.alt || k.mods.command {
            return false;
        }
        let Some((rows, index, read_only)) = with_tree(cx.id, |t| (t.visible_rows(), t.item_index, t.read_only)) else { return false };
        let at = rows.iter().position(|&r| r as i64 == index);
        let expanded = usize::try_from(index).ok().and_then(|i| with_tree(cx.id, |t| t.nodes.get(i).map(|n| (n.expanded, t.has_children(i), t.parent(i)))).flatten());
        match k.vk {
            // F2: the selected node's text edited
            113 => {
                if let (Ok(i), false) = (usize::try_from(index), read_only) {
                    act(cx, ListAction::TreeEdit(i));
                }
            }
            38 | 40 | 36 | 35 | 33 | 34 => {
                let page = ((cx.height() - 4) / row_h(cx.id)).max(1) as usize;
                let next = match (k.vk, at) {
                    (_, None) => rows.first().copied(),
                    (40, Some(a)) => rows.get(a + 1).copied(),
                    (38, Some(a)) => a.checked_sub(1).and_then(|a| rows.get(a).copied()),
                    (36, _) => rows.first().copied(),
                    (35, _) => rows.last().copied(),
                    (34, Some(a)) => rows.get((a + page).min(rows.len().saturating_sub(1))).copied(),
                    (33, Some(a)) => rows.get(a.saturating_sub(page)).copied(),
                    _ => None,
                };
                if let Some(n) = next.filter(|&n| n as i64 != index) {
                    act(cx, ListAction::TreeSelect(n));
                }
            }
            // Right: expand (or to the first child); Left: collapse (or to the parent)
            39 => {
                if let (Ok(i), Some((false, true, _))) = (usize::try_from(index), expanded) {
                    act(cx, ListAction::TreeToggle(i, true));
                } else if let (Some(a), Some((true, true, _))) = (at, expanded) {
                    if let Some(&n) = rows.get(a + 1) {
                        act(cx, ListAction::TreeSelect(n));
                    }
                }
            }
            37 => match (usize::try_from(index), expanded) {
                (Ok(i), Some((true, true, _))) => act(cx, ListAction::TreeToggle(i, false)),
                (Ok(_), Some((_, _, Some(parent)))) => act(cx, ListAction::TreeSelect(parent)),
                _ => {}
            },
            _ => return false,
        }
        true
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        let mut n = super::shared_describe(cx, "RTREEVIEW");
        let (x0, y0) = (cx.rect.0, cx.rect.1);
        for row in rows(cx) {
            if let Some(item) = n.children.iter_mut().find(|c| c.id == part_id(cx.id, PART_ITEM, row.node)) {
                item.bounds = (x0 + 2, y0 + 2 + row.top, cx.rect.2 - 4, row.height);
            }
        }
        n
    }

    fn access(&self, cx: &mut Cx, action: Action, part: Option<usize>, _value: Option<&AccessValue>) -> bool {
        let Some(i) = part else { return false };
        match action {
            Action::Click => Self::press(cx, i, false),
            Action::Expand | Action::Collapse => {
                cx.click();
                act(cx, ListAction::TreeToggle(i, action == Action::Expand));
            }
            _ => return false,
        }
        true
    }

    /// (the input lane's) A held bar's repeat; or the double-click time is
    /// up after a click on the selected node: its edit asked for
    /// (OnEditing), if it's still the selected one.
    fn tick(&self, cx: &mut Cx) {
        if bar_tick(cx) {
            Self::follow_bar(cx);
            return;
        }
        let Some(n) = EDIT_SOON.with(|e| e.borrow_mut().remove(cx.id)) else { return };
        if with_tree(cx.id, |t| t.item_index == n as i64).unwrap_or(false) && editing(cx.id).is_none() {
            act(cx, ListAction::TreeEdit(n));
        }
    }

    // (the input lane's: the edit's input methods and context menu)
    fn ime(&self, cx: &mut Cx, ime: &Ime) -> bool {
        Self::edit_rect(cx).is_some_and(|r| editor_ime(cx, ime, r))
    }

    fn ime_area(&self, cx: &mut Cx) -> Option<Rect> {
        editor_ime_area(cx, Self::edit_rect(cx)?)
    }

    fn wants_ime(&self, _store: &dyn Store, id: &str) -> bool {
        editing(id).is_some()
    }

    fn context_menu(&self, cx: &mut Cx) -> Option<super::edit::MenuState> {
        editor_menu(cx, Self::edit_rect(cx)?)
    }

    /// `__node_i` (a click on node i's text), `__toggle_i` (on its
    /// button), `__edit` (F2), `__enter` ("Renamed" typed, then Enter),
    /// `__escape` (the edit dropped).
    fn test_action(&self, cx: &mut Cx, action: &str) -> bool {
        let click_at = |cx: &mut Cx, this: &Tree, x: i64, y: i64| {
            let at = MouseIn { kind: MouseKind::Down, x: x as f64 + 0.5, y: y as f64 + 0.5, button: rapidr_value::input::Button::Left, mods: crate::input::Mods::NONE, inside: true, captured: true, clicks: 1 };
            this.mouse(cx, &at);
            this.mouse(cx, &MouseIn { kind: MouseKind::Up, ..at });
        };
        if let Some(n) = action.strip_prefix("__node_").and_then(|s| s.parse::<usize>().ok()) {
            // (a test's click is never a double click: `click_at`'s count is 1)
            match Self::row_rect(cx, n) {
                Some((row, (_, y, _, rh))) => {
                    let x = text_left(cx, &row) + 4;
                    click_at(cx, self, x, y + rh / 2);
                }
                None => Self::press(cx, n, false),
            }
            return true;
        }
        if let Some(n) = action.strip_prefix("__toggle_").and_then(|s| s.parse::<usize>().ok()) {
            match Self::row_rect(cx, n) {
                Some((Row { center: Some(c), button: Some(_), .. }, (_, y, _, rh))) => click_at(cx, self, 2 + c, y + rh / 2),
                _ => Self::toggle(cx, n),
            }
            return true;
        }
        let key = |vk: i64| KeyIn { vk, text: "", mods: crate::input::Mods::NONE };
        match action {
            "__edit" => {
                let mut clip = crate::input::MemClipboard::default();
                self.key(cx, &key(113), &mut clip);
            }
            "__enter" | "__escape" => {
                if editing(cx.id).is_some() {
                    if action == "__enter" {
                        set_edit_text(cx.id, "Renamed");
                    }
                    let mut clip = crate::input::MemClipboard::default();
                    self.key(cx, &key(if action == "__enter" { 13 } else { 27 }), &mut clip);
                }
            }
            _ => return false,
        }
        true
    }
}

/// Opens node `n`'s editor with `text` (runtime-core, once OnEditing
/// allowed it).
pub fn open_editor(id: &str, n: usize, text: &str) {
    begin_edit(id, InPlace { target: (n, 0), text: text.to_string(), rect: None });
}

//! QSTRINGGRID: the shared model (`rapidr_value::objects::grid::StringGrid`)
//! holds the cells, sizes, options and the selection, and keeps what
//! OnDrawCell drew per cell; this draws Delphi's grid — a sunken white
//! box, the fixed rows and columns as raised grey cells, the 1-pixel lines
//! between cells, the selected cells white on blue, a gcsList column's
//! selected cell with its drop-down button, an ellipsis column's "…"
//! button — with what OnDrawCell drew replayed over each cell (as FLTK's
//! `grid_replay`); scrolled by TopRow / LeftCol under the fixed ones.
//!
//! What the user does: a click selects a cell (OnSelectCell (Col, Row,
//! CanSelect) answers back: runtime-core) and fires OnClick; Shift+click
//! or a drag selects a range (goRangeSelect); a fixed row's cell dragged
//! moves its column (goColMoving), a fixed column's its row
//! (goRowMoving); a fixed row's cell border dragged sizes the column
//! (goColSizing); the arrows move the selection; with goEditing, Enter,
//! F2, typing or a double click edit the cell in place, and Enter stores
//! it (OnSetEditText, OnChange).
//!
//! VisibleRowCount / VisibleColCount count what fits the control's inside
//! (`StringGrid::view`, set here as FLTK's table sets it).

use std::cell::RefCell;
use std::collections::HashMap;

use rapidr_value::objects::a11y::{node_id, part_id, AccessNode, Action, Role};
use rapidr_value::objects::grid::{StringGrid, GCS_ELLIPSIS, GO_ALWAYS_SHOW_EDITOR, GO_COL_MOVING, GO_COL_SIZING, GO_FIXED_HORZ_LINE, GO_FIXED_VERT_LINE, GO_HORZ_LINE, GO_ROW_MOVING, GO_ROW_SIZING, GO_VERT_LINE};
use rapidr_value::objects::ops::{lift, Op, Place, Rect};
use rapidr_value::objects::{with_grid, with_grid_mut};
use rapidr_value::scrollbars::{Child, Scroller};

use super::list::{act, begin_edit, edit_key, editing, end_edit, fire, paint_edit, replay, set_edit_text, sunken, InPlace, ListAction};
use super::{ComponentKind, Cx, KeyIn, MouseIn, MouseKind, MouseOut};
use crate::a11y::AccessValue;
use crate::input::Clipboard;
use crate::paint::{Painter, DARK, FACE, HIGHLIGHT, HIGHLIGHT_TEXT, LIGHT, SHADOW};
use crate::text::bgr_to_rgb;

/// The lines between cells: silver among the cells, grey among the fixed.
const LINE: u32 = 0xC0C0C0;
const FIXED_LINE: u32 = 0x808080;

/// What the mouse is doing on a grid.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Drag {
    /// Selecting a range from a press on a cell.
    Range,
    /// A column (true) or row moved from index `from`.
    Move(bool, usize),
    /// Column `col` sized from x, its width then.
    Size(usize, i64, i64),
}

thread_local! {
    static DRAGS: RefCell<HashMap<String, Drag>> = RefCell::new(HashMap::new());
    static SCROLLERS: RefCell<HashMap<String, Scroller>> = RefCell::new(HashMap::new());
    /// The selected cell each grid last showed (a new one is scrolled
    /// into view).
    static SHOWN: RefCell<HashMap<String, (i64, i64)>> = RefCell::new(HashMap::new());
}

pub struct Grid;

/// The columns (or rows) shown, as (index, start, size) along the grid's
/// inside: the fixed ones, then the others from `first` (LeftCol / TopRow).
fn spans(sizes: &[i64], fixed: usize, first: i64, room: i64) -> Vec<(usize, i64, i64)> {
    let mut out = Vec::new();
    let mut at = 0;
    let first = usize::try_from(first).unwrap_or(0).max(fixed);
    for i in (0..fixed.min(sizes.len())).chain(first..sizes.len()) {
        if at >= room {
            break;
        }
        let s = sizes[i].clamp(0, 10_000);
        out.push((i, at, s));
        at += s + 1;
    }
    out
}

/// Where everything is, for a grid `w` × `h` (its frame included).
struct Layout {
    cols: Vec<(usize, i64, i64)>,
    rows: Vec<(usize, i64, i64)>,
    /// The cells' area inside the frame and the bars.
    inner: (i64, i64),
    bars: Vec<Op>,
}

/// The scroll bars over the non-fixed cells: their ranges in pixels, the
/// positions from TopRow / LeftCol.
fn layout(id: &str, g: &StringGrid, w: i64, h: i64) -> Layout {
    let (iw, ih) = (w - 4, h - 4);
    let total = |sizes: &[i64]| sizes.iter().map(|s| (*s).clamp(0, 10_000) + 1).sum::<i64>();
    let before = |sizes: &[i64], fixed: usize, first: i64| sizes.iter().take(usize::try_from(first).unwrap_or(0)).skip(fixed).map(|s| (*s).clamp(0, 10_000) + 1).sum::<i64>();
    let (fc, fr) = (g.fixed_cols(), g.fixed_rows());
    let (bars, (cw, ch)) = SCROLLERS.with(|s| {
        let mut s = s.borrow_mut();
        let sc = s.entry(id.to_lowercase()).or_default();
        sc.vert.tracking = true;
        sc.horz.tracking = true;
        sc.vert.increment = g.default_row_height.max(1);
        sc.horz.increment = g.default_col_width.max(1);
        sc.vert.position = before(&g.row_heights, fr, g.top_row);
        sc.horz.position = before(&g.col_widths, fc, g.left_col);
        let content = Child { left: -sc.horz.position, top: -sc.vert.position, width: total(&g.col_widths), height: total(&g.row_heights), align: rapidr_value::layout::Align::None, visible: true };
        sc.update(iw, ih, &[content]);
        (lift(sc.ops(iw, ih)), sc.client(iw, ih))
    });
    Layout { cols: spans(&g.col_widths, fc, g.left_col, cw), rows: spans(&g.row_heights, fr, g.top_row, ch), inner: (cw, ch), bars }
}

/// The cell at (x, y) of the component, with its rectangle there.
fn cell_at(l: &Layout, x: i64, y: i64) -> Option<((usize, usize), Rect)> {
    let (x, y) = (x - 2, y - 2);
    if x < 0 || y < 0 || x >= l.inner.0 || y >= l.inner.1 {
        return None;
    }
    let &(c, cx0, cw) = l.cols.iter().find(|&&(_, s, sz)| x >= s && x < s + sz + 1)?;
    let &(r, ry0, rh) = l.rows.iter().find(|&&(_, s, sz)| y >= s && y < s + sz + 1)?;
    Some(((c, r), (2 + cx0, 2 + ry0, cw, rh)))
}

/// Cell (c, r)'s rectangle in the component, if shown.
fn cell_rect(l: &Layout, c: usize, r: usize) -> Option<Rect> {
    let &(_, x, w) = l.cols.iter().find(|s| s.0 == c)?;
    let &(_, y, h) = l.rows.iter().find(|s| s.0 == r)?;
    Some((2 + x, 2 + y, w, h))
}

/// Whether (c, r) shows an ellipsis button.
fn has_ellipsis(g: &StringGrid, c: usize, r: usize) -> bool {
    let fixed = r < g.fixed_rows() || c < g.fixed_cols();
    !fixed && (g.column_style(c) == GCS_ELLIPSIS || g.cell(c, r) == "...")
}

/// TopRow / LeftCol moved so cell (c, r) shows (Delphi scrolls the
/// selection into view).
fn scroll_to_cell(g: &mut StringGrid, c: i64, r: i64, l: &Layout) {
    let (fc, fr) = (g.fixed_cols() as i64, g.fixed_rows() as i64);
    if r >= fr && r < g.top_row {
        g.top_row = r;
    } else if r >= fr && !l.rows.iter().any(|&(i, s, sz)| i as i64 == r && s + sz <= l.inner.1) {
        // (down: the first row that leaves it whole at the bottom)
        let mut top = r;
        let mut room = l.inner.1 - l.rows.iter().filter(|x| (x.0 as i64) < fr).map(|x| x.2 + 1).sum::<i64>();
        while top > fr {
            room -= g.row_heights.get(top as usize).copied().unwrap_or(0) + 1;
            if room - g.row_heights.get(top as usize - 1).copied().unwrap_or(0) - 1 < 0 {
                break;
            }
            top -= 1;
        }
        g.top_row = top.max(fr);
    }
    if c >= fc && c < g.left_col {
        g.left_col = c;
    } else if c >= fc && !l.cols.iter().any(|&(i, s, sz)| i as i64 == c && s + sz <= l.inner.0) {
        let mut left = c;
        let mut room = l.inner.0 - l.cols.iter().filter(|x| (x.0 as i64) < fc).map(|x| x.2 + 1).sum::<i64>();
        while left > fc {
            room -= g.col_widths.get(left as usize).copied().unwrap_or(0) + 1;
            if room - g.col_widths.get(left as usize - 1).copied().unwrap_or(0) - 1 < 0 {
                break;
            }
            left -= 1;
        }
        g.left_col = left.max(fc);
    }
}

impl Grid {
    /// Starts editing the selected cell (with `initial` text, or its own).
    fn start_edit(cx: &mut Cx, initial: Option<String>) {
        let Some((c, r, text, editable)) = with_grid(cx.id, |g| (g.col, g.row, g.cell(g.col.max(0) as usize, g.row.max(0) as usize).to_string(), g.editable())) else { return };
        if !editable || c < 0 || r < 0 {
            return;
        }
        begin_edit(cx.id, InPlace { target: (c as usize, r as usize), text: initial.unwrap_or(text), rect: None });
    }

    /// An edit going on ends: kept, the cell gets it (OnSetEditText,
    /// OnChange: runtime-core).
    fn finish_edit(cx: &mut Cx, keep: bool) {
        if let Some(ed) = end_edit(cx.id) {
            if keep {
                act(cx, ListAction::GridStore(ed.text));
            }
        }
    }
}

impl ComponentKind for Grid {
    fn name(&self) -> &'static str {
        "RSTRINGGRID"
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let (w, h) = (cx.width(), cx.height());
        sunken(p, w, h, 0xFFFFFF);
        // (VisibleRowCount / VisibleColCount: the inside, as FLTK's table)
        let fresh = with_grid_mut(cx.id, |g| {
            g.view = (w - 4, h - 4);
            let sel = (g.col, g.row);
            SHOWN.with(|s| s.borrow_mut().insert(cx.id.to_string(), sel)) != Some(sel)
        });
        if fresh == Some(true) {
            with_grid_mut(cx.id, |g| {
                let l = layout(cx.id, g, w, h);
                let (c, r) = (g.col, g.row);
                scroll_to_cell(g, c, r, &l);
            });
        }
        let Some(g) = with_grid(cx.id, |g| g.clone()) else { return };
        let l = layout(cx.id, &g, w, h);
        let font = cx.font.clone();
        let text_color = bgr_to_rgb(font.color);
        let edit = editing(cx.id);
        let (fc, fr) = (g.fixed_cols(), g.fixed_rows());
        p.at((2, 2), |p| {
            p.clipped((0, 0, l.inner.0, l.inner.1), |p| {
                // (the lines: what the cells leave between them)
                let right = l.cols.last().map_or(0, |c| c.1 + c.2 + 1);
                let bottom = l.rows.last().map_or(0, |r| r.1 + r.2 + 1);
                for &(r, y, rh) in &l.rows {
                    for &(c, x, cw) in &l.cols {
                        let fixed = r < fr || c < fc;
                        let (vl, hl) = if fixed { (g.has_option(GO_FIXED_VERT_LINE), g.has_option(GO_FIXED_HORZ_LINE)) } else { (g.has_option(GO_VERT_LINE), g.has_option(GO_HORZ_LINE)) };
                        let line = if fixed { FIXED_LINE } else { LINE };
                        if vl {
                            p.fill((x + cw, y, 1, rh + 1), line);
                        }
                        if hl {
                            p.fill((x, y + rh, cw + 1, 1), line);
                        }
                    }
                }
                let _ = (right, bottom);
                for &(r, y, rh) in &l.rows {
                    for &(c, x, cw) in &l.cols {
                        let rect = (x, y, cw, rh);
                        let fixed = r < fr || c < fc;
                        let selected = g.is_selected(c, r);
                        let text = g.cell(c, r).to_string();
                        let ellipsis = has_ellipsis(&g, c, r);
                        let list = (g.col, g.row) == (c as i64, r as i64) && g.list_items(c, r).is_some();
                        let button = if ellipsis || list { rh.min(cw) } else { 0 };
                        p.clipped(rect, |p| {
                            if fixed {
                                p.fill(rect, FACE);
                                p.edge(rect, &[LIGHT], &[SHADOW]);
                            } else {
                                p.fill(rect, if selected { HIGHLIGHT } else { 0xFFFFFF });
                            }
                            if !(ellipsis && text == "...") {
                                let color = if selected { HIGHLIGHT_TEXT } else { text_color };
                                p.clipped((x, y, (cw - button).max(0), rh), |p| p.text((x + 2, y + 2, (cw - 4 - button).max(0), (rh - 2).max(0)), &text, &font, color, Place::TopLeft));
                            }
                            if button > 0 {
                                let b = (x + cw - button, y, button, rh);
                                p.fill(b, FACE);
                                p.edge(b, &[LIGHT], &[DARK, SHADOW]);
                                if ellipsis {
                                    p.text(b, "...", &font, 0x000000, Place::Center);
                                } else {
                                    let (mx, my) = (b.0 as f64 + b.2 as f64 / 2.0, b.1 as f64 + b.3 as f64 / 2.0);
                                    p.op(Op::Arrow { points: [(mx - 4.0, my - 2.0), (mx + 4.0, my - 2.0), (mx, my + 2.0)], color: 0x000000 });
                                }
                            }
                            if let Some(ops) = g.owner_drawing.get(&(c, r)) {
                                p.at((x, y), |p| replay(p, ops, &font, &format!("{}#cell{c},{r}", cx.id)));
                            }
                        });
                        if let Some(ed) = edit.as_ref().filter(|e| e.target == (c, r)) {
                            paint_edit(p, rect, &ed.text, &font, cx.state.caret_on);
                        }
                    }
                }
            });
            p.ops(l.bars.clone());
        });
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        let (w, h) = (cx.width(), cx.height());
        let Some(g) = with_grid(cx.id, |g| g.clone()) else { return MouseOut::default() };
        let l = layout(cx.id, &g, w, h);
        let (x, y) = (m.x.floor() as i64, m.y.floor() as i64);
        // (the scroll bars first: TopRow / LeftCol follow, a row / column at a time)
        let on_bars = SCROLLERS.with(|s| {
            let mut s = s.borrow_mut();
            let Some(sc) = s.get_mut(cx.id) else { return false };
            let (bx, by) = (x - 2, y - 2);
            let took = match m.kind {
                MouseKind::Down => sc.mouse_down(bx, by, w - 4, h - 4).is_some(),
                MouseKind::Move if sc.pressed.is_some() => {
                    sc.mouse_drag(bx, by, w - 4, h - 4);
                    true
                }
                MouseKind::Up if sc.pressed.is_some() => {
                    sc.mouse_up(w - 4, h - 4);
                    true
                }
                _ => false,
            };
            if took {
                let (vp, hp) = (sc.vert.position, sc.horz.position);
                with_grid_mut(cx.id, |g| {
                    let pick = |sizes: &[i64], fixed: usize, pos: i64| {
                        let mut at = 0;
                        for (i, s) in sizes.iter().enumerate().skip(fixed) {
                            if at + (*s).clamp(0, 10_000) / 2 >= pos {
                                return i as i64;
                            }
                            at += (*s).clamp(0, 10_000) + 1;
                        }
                        sizes.len().saturating_sub(1) as i64
                    };
                    g.top_row = pick(&g.row_heights, g.fixed_rows(), vp);
                    g.left_col = pick(&g.col_widths, g.fixed_cols(), hp);
                });
            }
            took
        });
        if on_bars {
            return MouseOut::default();
        }
        let id = cx.id.to_string();
        match m.kind {
            MouseKind::Down => {
                if editing(&id).is_some() {
                    Self::finish_edit(cx, true);
                }
                let Some(((c, r), (rx, ry, rw, rh))) = cell_at(&l, x, y) else { return MouseOut::default() };
                let (fc, fr) = (g.fixed_cols(), g.fixed_rows());
                // (a fixed row's cell border: the column sized)
                if r < fr && g.has_option(GO_COL_SIZING) {
                    if let Some(&(sc, sx, sw)) = l.cols.iter().find(|&&(_, s, sz)| (x - 2 - (s + sz)).abs() <= 2) {
                        DRAGS.with(|d| d.borrow_mut().insert(id.clone(), Drag::Size(sc, x, sw)));
                        let _ = sx;
                        return MouseOut::default();
                    }
                }
                let _ = GO_ROW_SIZING;
                if r < fr && c >= fc && g.has_option(GO_COL_MOVING) {
                    DRAGS.with(|d| d.borrow_mut().insert(id.clone(), Drag::Move(true, c)));
                    return MouseOut::default();
                }
                if c < fc && r >= fr && g.has_option(GO_ROW_MOVING) {
                    DRAGS.with(|d| d.borrow_mut().insert(id.clone(), Drag::Move(false, r)));
                    return MouseOut::default();
                }
                if r < fr || c < fc {
                    return MouseOut::default();
                }
                // (the focused gcsList cell's drop-down button)
                if (g.col, g.row) == (c as i64, r as i64) && g.list_items(c, r).is_some() && x >= rx + rw - rh.min(rw) {
                    let (ax, ay) = (cx.rect.0 + rx, cx.rect.1 + ry);
                    act(cx, ListAction::GridListDrop(c as i64, r as i64, (ax, ay, rw, rh)));
                    return MouseOut::default();
                }
                let on_button = has_ellipsis(&g, c, r) && x >= rx + rw - rh.min(rw);
                if m.mods.shift {
                    act(cx, ListAction::GridSelect(c as i64, r as i64, true));
                    return MouseOut::default();
                }
                act(cx, ListAction::GridSelect(c as i64, r as i64, false));
                DRAGS.with(|d| d.borrow_mut().insert(id.clone(), Drag::Range));
                if on_button {
                    fire(cx, "onellipsisclick", vec![rapidr_value::v_int(c as i64), rapidr_value::v_int(r as i64)]);
                    fire(cx, "ondblclick", Vec::new());
                    return MouseOut::default();
                }
                cx.click();
                if m.double() {
                    fire(cx, "ondblclick", Vec::new());
                    Self::start_edit_at(cx, c, r);
                } else if g.has_option(GO_ALWAYS_SHOW_EDITOR) {
                    Self::start_edit_at(cx, c, r);
                }
            }
            MouseKind::Move if m.captured => match DRAGS.with(|d| d.borrow().get(&id).copied()) {
                Some(Drag::Range) => {
                    if let Some(((c, r), _)) = cell_at(&l, x, y) {
                        if (c as i64, r as i64) != (g.col, g.row) && r >= g.fixed_rows() && c >= g.fixed_cols() {
                            act(cx, ListAction::GridSelect(c as i64, r as i64, true));
                        }
                    }
                }
                Some(Drag::Size(c, from, width)) => {
                    with_grid_mut(&id, |g| {
                        if let Some(cw) = g.col_widths.get_mut(c) {
                            *cw = (width + x - from).max(0);
                        }
                    });
                }
                _ => {}
            },
            MouseKind::Up => match DRAGS.with(|d| d.borrow_mut().remove(&id)) {
                Some(Drag::Move(cols, from)) => {
                    if let Some(((c, r), _)) = cell_at(&l, x, y) {
                        let to = if cols { c } else { r };
                        with_grid_mut(&id, |g| if cols { g.move_col(from, to) } else { g.move_row(from, to) });
                    }
                }
                Some(Drag::Size(..)) | Some(Drag::Range) | None => {}
            },
            _ => {}
        }
        MouseOut::default()
    }

    fn key(&self, cx: &mut Cx, k: &KeyIn, _clip: &mut dyn Clipboard) -> bool {
        if let Some(end) = edit_key(cx.id, k) {
            if let Some(keep) = end {
                Self::finish_edit(cx, keep);
            }
            return true;
        }
        if k.mods.alt || k.mods.command {
            return false;
        }
        let Some((c, r)) = with_grid(cx.id, |g| (g.col, g.row)) else { return false };
        let moved = match k.vk {
            38 => Some((c, r - 1)),
            40 => Some((c, r + 1)),
            37 => Some((c - 1, r)),
            39 => Some((c + 1, r)),
            _ => None,
        };
        if let Some((nc, nr)) = moved {
            act(cx, ListAction::GridSelect(nc, nr, k.mods.shift));
            return true;
        }
        if matches!(k.vk, 13 | 113) {
            Self::start_edit(cx, None);
            return true;
        }
        if !k.text.is_empty() && k.text.chars().all(|ch| !ch.is_control()) && !k.mods.ctrl {
            let editable = with_grid(cx.id, |g| g.editable()).unwrap_or(false);
            if editable {
                Self::start_edit(cx, Some(k.text.to_string()));
                return true;
            }
        }
        false
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        let mut n = AccessNode::new(node_id(cx.id), Role::Grid);
        n.actions = vec![Action::Focus];
        n.bounds = cx.rect;
        let (w, h) = (cx.width(), cx.height());
        let Some(g) = with_grid(cx.id, |g| g.clone()) else { return n };
        let l = layout(cx.id, &g, w, h);
        for &(r, _, _) in &l.rows {
            let mut row = AccessNode::new(part_id(cx.id, 1, r), Role::Row);
            for &(c, _, _) in &l.cols {
                let mut cell = AccessNode::new(part_id(cx.id, 2, r * 10_000 + c), Role::Cell);
                cell.value = Some(g.cell(c, r).to_string());
                cell.states.selected = Some(g.is_selected(c, r));
                if let Some((x, y, cw, ch)) = cell_rect(&l, c, r) {
                    cell.bounds = (cx.rect.0 + x, cx.rect.1 + y, cw, ch);
                }
                row.children.push(cell);
            }
            n.children.push(row);
        }
        n
    }

    fn access(&self, _cx: &mut Cx, _action: Action, _part: Option<usize>, _value: Option<&AccessValue>) -> bool {
        false
    }

    /// `__cell_c_r`: cell (c, r) selected as FLTK's hook does (OnSelectCell,
    /// no click); `__edit` (F2), `__enter` ("Renamed", Enter), `__escape`.
    fn test_action(&self, cx: &mut Cx, action: &str) -> bool {
        if let Some(rest) = action.strip_prefix("__cell_") {
            let mut it = rest.split('_').map(|s| s.parse::<i64>().ok());
            if let (Some(Some(c)), Some(Some(r))) = (it.next(), it.next()) {
                act(cx, ListAction::GridSelect(c, r, false));
                return true;
            }
            return false;
        }
        let key = |vk: i64| KeyIn { vk, text: "", mods: crate::input::Mods::NONE };
        let mut clip = crate::input::MemClipboard::default();
        match action {
            "__edit" => {
                self.key(cx, &key(113), &mut clip);
            }
            "__enter" | "__escape" => {
                if editing(cx.id).is_some() {
                    if action == "__enter" {
                        set_edit_text(cx.id, "Renamed");
                    }
                    self.key(cx, &key(if action == "__enter" { 13 } else { 27 }), &mut clip);
                }
            }
            _ => return false,
        }
        true
    }
}

impl Grid {
    /// An edit of cell (c, r), once it's the selected one (the click's
    /// selection is the program's to allow: the edit opens on the cell
    /// clicked when editing is on).
    fn start_edit_at(cx: &mut Cx, c: usize, r: usize) {
        let Some((text, editable)) = with_grid(cx.id, |g| (g.cell(c, r).to_string(), g.editable())) else { return };
        if editable {
            begin_edit(cx.id, InPlace { target: (c, r), text, rect: None });
        }
    }
}

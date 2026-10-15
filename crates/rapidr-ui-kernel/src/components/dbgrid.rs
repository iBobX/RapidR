//! RDBGRID (I7, docs/data-science-plan.md §4.4): a grid drawn straight
//! from a data set's frame — an RDataSource's, an RDataFrame's, an
//! RCSV's (`rapidr_value::datascience::link` resolves it and keeps the
//! current row; `frame` has the table). **Virtual rows**: only the rows on
//! screen are drawn, so a million rows cost nothing beyond the frame and
//! scroll at the frame rate.
//!
//! Drawn: the field's box, a heading row (the columns' names, the sorted
//! one with its arrow), a gutter with each row's number in the data (from
//! 0, as `Row` counts), the cells — numbers right-aligned, a missing value
//! dimmed as `null` —, the current row highlighted, the scroll bars.
//!
//! What the user does: a click on a row makes it the current row (bound
//! edits follow; OnCellClick(Row, Col)); a click on a heading sorts the
//! grid's view by that column, again descending, a third time back to the
//! data's order (OnTitleClick(Col)) — the data stays as it is; the arrows,
//! Page Up / Down, Home / End move the current row; the wheel and the bars
//! scroll. A screen reader hears a grid: the headings, then the rows on
//! screen, each named "Row 12 of 1000000", the current one selected.
//!
//! On a design surface (RapidR Studio's designer) it shows the data the
//! designer read for the designed form (`link::design_begin`), or what to
//! set when it has none.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use rapidr_value::datascience::{frame, link};
use rapidr_value::objects::a11y::{part_id, AccessNode, Action, Role, PART_CELL, PART_ROW};
use rapidr_value::objects::font::Font;
use rapidr_value::objects::ops::{Op, Place};
use rapidr_value::scrollbars::{Child, Scroller};
use rapidr_value::v_int;

use super::list::{sunken, whole_notches};
use super::{ComponentKind, Cx, KeyIn, MouseIn, MouseKind, MouseOut};
use crate::a11y::AccessValue;
use crate::input::{Clipboard, KernelEvent};
use crate::paint::{ink, Painter};
use crate::text::TextSystem;

pub struct DbGrid;

/// The columns' widths for a data set's version and a font.
struct Widths {
    key: (String, u64, String, i64),
    cols: Vec<i64>,
    gutter: i64,
}

thread_local! {
    static SCROLLERS: RefCell<HashMap<String, Scroller>> = RefCell::new(HashMap::new());
    static WIDTHS: RefCell<HashMap<String, Widths>> = RefCell::new(HashMap::new());
    /// Where a grid's bar is held (a held arrow or track repeats).
    static HELD: RefCell<HashMap<String, (i64, i64)>> = RefCell::new(HashMap::new());
    /// The current row and sort each grid last showed (a new one is
    /// scrolled into view).
    static SHOWN: RefCell<HashMap<String, (usize, Option<(usize, bool)>)>> = RefCell::new(HashMap::new());
}

/// Most rows looked at to size a column (the widest text among them).
const SAMPLE_ROWS: usize = 200;
const MIN_COL: i64 = 44;
const MAX_COL: i64 = 320;

/// A row's height for `font` (the heading's too).
fn row_height(font: &Font) -> i64 {
    (font.pixel_size() + 9).max(20)
}

/// The columns' widths and the gutter's, measured once per data version:
/// the heading and the widest of the first rows' texts, kept in bounds.
fn widths(ts: &mut TextSystem, id: &str, d: &link::GridData, font: &Font) -> (Vec<i64>, i64) {
    let key = (d.dataset.clone(), d.version, font.name.clone(), font.pixel_size());
    if let Some(w) = WIDTHS.with(|w| w.borrow().get(id).filter(|w| w.key == key).map(|w| (w.cols.clone(), w.gutter))) {
        return w;
    }
    let (names, samples): (Vec<String>, Vec<String>) = frame::with_frame(&d.dataset, |f| {
        let n = f.height().min(SAMPLE_ROWS);
        let names = f.columns.clone();
        let samples = (0..f.width())
            .map(|c| (0..n).filter_map(|r| f.cell(r, c).map(|s| s.into_owned())).max_by_key(|s| s.chars().count()).unwrap_or_default())
            .collect();
        (names, samples)
    });
    let mut bold = font.clone();
    bold.styles |= 1;
    let cols: Vec<i64> = names
        .iter()
        .zip(&samples)
        .map(|(name, sample)| {
            let head = ts.measure(name, &bold).0 as i64 + 22;
            let cell = ts.measure(sample, font).0 as i64 + 14;
            head.max(cell).clamp(MIN_COL, MAX_COL)
        })
        .collect();
    let gutter = ts.measure(&d.rows.saturating_sub(1).max(9).to_string(), font).0 as i64 + 14;
    WIDTHS.with(|w| w.borrow_mut().insert(id.to_string(), Widths { key, cols: cols.clone(), gutter }));
    (cols, gutter)
}

/// Where everything is, for a grid `w` × `h` (its frame included).
struct Layout {
    row_h: i64,
    cols: Vec<i64>,
    gutter: i64,
    /// The cells' area inside the frame, less the bars.
    inner: (i64, i64),
    /// How many data rows fit under the heading.
    page: usize,
    /// The first view row shown, and how far the columns are scrolled.
    top: usize,
    left: i64,
    bars: Vec<Op>,
}

fn layout(cx: &mut Cx, d: &link::GridData) -> Layout {
    let (w, h) = (cx.width(), cx.height());
    let font = cx.font.clone();
    let row_h = row_height(&font);
    let (cols, gutter) = widths(cx.text, cx.id, d, &font);
    let total_w = gutter + cols.iter().map(|c| c + 1).sum::<i64>();
    let total_h = row_h + 1 + d.rows as i64 * row_h;
    let (iw, ih) = (w - 4, h - 4);
    let (bars, (cw, ch), top, left) = SCROLLERS.with(|s| {
        let mut s = s.borrow_mut();
        let sc = s.entry(cx.id.to_string()).or_default();
        sc.vert.tracking = true;
        sc.horz.tracking = true;
        sc.vert.increment = row_h;
        sc.horz.increment = 24;
        sc.vert.position = d.top as i64 * row_h;
        let content = Child { left: -sc.horz.position, top: -sc.vert.position, width: total_w, height: total_h, align: rapidr_value::layout::Align::None, visible: true };
        sc.update(iw, ih, &[content]);
        let top = (sc.vert.position / row_h.max(1)) as usize;
        (crate::paint::bar_ops(sc, iw, ih), sc.client(iw, ih), top, sc.horz.position)
    });
    if top != d.top {
        link::set_grid_top(cx.id, top);
    }
    let page = ((ch - row_h - 1).max(0) / row_h.max(1)) as usize;
    Layout { row_h, cols, gutter, inner: (cw, ch), page: page.max(1), top, left, bars }
}

/// The data row shown at view row `v` (the view sorted, or the data's order).
fn data_row(order: &Option<(Rc<Vec<usize>>, Rc<Vec<usize>>)>, v: usize) -> usize {
    order.as_ref().and_then(|(o, _)| o.get(v).copied()).unwrap_or(v)
}

/// Data row `r`'s place in the view.
fn view_row(order: &Option<(Rc<Vec<usize>>, Rc<Vec<usize>>)>, r: usize) -> usize {
    order.as_ref().and_then(|(_, p)| p.get(r).copied()).unwrap_or(r)
}

/// The column at x of the cells' area (after the gutter, scrolled).
fn column_at(l: &Layout, x: i64) -> Option<usize> {
    let mut at = l.gutter - l.left;
    for (c, w) in l.cols.iter().enumerate() {
        if x >= at && x < at + w + 1 {
            return Some(c);
        }
        at += w + 1;
    }
    None
}

impl DbGrid {
    /// The current row moved to view row `v` (kept in the data): shown,
    /// and the program told (Row).
    fn go_to_view(cx: &mut Cx, d: &link::GridData, l: &Layout, order: &Option<(Rc<Vec<usize>>, Rc<Vec<usize>>)>, v: i64) {
        if d.rows == 0 {
            return;
        }
        let v = v.clamp(0, d.rows as i64 - 1) as usize;
        // (scrolled so it shows)
        let top = if v < l.top { v } else if v >= l.top + l.page { v + 1 - l.page } else { l.top };
        if top != l.top {
            link::set_grid_top(cx.id, top);
        }
        let r = data_row(order, v);
        link::set_row(cx.id, r);
        cx.events.push(KernelEvent::Set { id: cx.id.to_string(), prop: "row".into(), value: r as i64 });
    }
}

impl ComponentKind for DbGrid {
    fn name(&self) -> &'static str {
        "RDBGRID"
    }

    fn field(&self) -> bool {
        true
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let (w, h) = (cx.width(), cx.height());
        let t = p.theme();
        let background = super::list::background(cx);
        sunken(p, w, h, background);
        let font = cx.font.clone();
        let Some(d) = link::grid(cx.id) else {
            // (the designer: what to set)
            if super::design::designed(cx.id) {
                p.text((8, 4, w - 16, h - 8), "Set DataSource to see the rows here", &font, t.gray_text, Place::Center);
            }
            return;
        };
        let order = link::grid_order(cx.id, &d);
        let mut l = layout(cx, &d);
        // (a current row the program moved, or a new view: scrolled so it shows)
        let at = view_row(&order, d.row);
        let seen = SHOWN.with(|s| s.borrow_mut().insert(cx.id.to_string(), (d.row, d.sort)));
        if seen != Some((d.row, d.sort)) && d.rows > 0 && (at < l.top || at >= l.top + l.page) {
            let top = if at < l.top { at } else { at + 1 - l.page };
            link::set_grid_top(cx.id, top);
            if let Some(d2) = link::grid(cx.id) {
                l = layout(cx, &d2);
            }
        }
        let text_color = ink(cx.store, cx.id, &font, cx.state.enabled, background);
        let mut bold = font.clone();
        bold.styles |= 1;
        let (cw, ch) = l.inner;
        let numeric: Vec<bool> = frame::with_frame(&d.dataset, |f| (0..f.width()).map(|c| f.dtype(c).numeric()).collect());
        let names: Vec<String> = frame::with_frame(&d.dataset, |f| f.columns.clone());
        // (selected: a tint of the accent, grey without the focus)
        let (sel_fill, sel_ink) = if !t.fluent() {
            (t.highlight, t.highlight_text)
        } else if cx.state.focused {
            (t.selected, t.selected_text)
        } else {
            (t.unfocused, t.text)
        };
        let line_h = (cx.text.measure("Ag", &font).1 as i64).max(1);
        p.at((2, 2), |p| {
            p.clipped((0, 0, cw, ch), |p| {
                let rh = l.row_h;
                // (the rows on screen)
                let last = (l.top + l.page + 1).min(d.rows);
                for (i, v) in (l.top..last).enumerate() {
                    let r = data_row(&order, v);
                    let y = rh + 1 + i as i64 * rh;
                    let current = r == d.row;
                    if current {
                        p.fill((0, y, cw, rh), sel_fill);
                    }
                    let ink_row = if current { sel_ink } else { text_color };
                    // (its number)
                    p.fill((0, y, l.gutter, rh), if current { sel_fill } else { t.face });
                    p.text((0, y + (rh - line_h) / 2, l.gutter - 6, line_h), &r.to_string(), &font, if current { sel_ink } else { t.gray_text }, Place::TopRight);
                    let mut x = l.gutter - l.left;
                    frame::with_frame(&d.dataset, |f| {
                        for (c, colw) in l.cols.iter().enumerate() {
                            if x + colw >= l.gutter && x < cw {
                                let cell = f.cell(r, c);
                                let (text, color) = match &cell {
                                    Some(s) => (s.as_ref(), ink_row),
                                    None => ("null", if current { sel_ink } else { t.gray_text }),
                                };
                                let rect = (x + 5, y + (rh - line_h) / 2, colw - 10, line_h);
                                p.clipped((x.max(l.gutter), y, (x + colw).min(cw) - x.max(l.gutter), rh), |p| {
                                    p.text(rect, text, &font, color, if numeric.get(c).copied().unwrap_or(false) && cell.is_some() { Place::TopRight } else { Place::TopLeft });
                                });
                            }
                            x += colw + 1;
                        }
                    });
                    p.fill((0, y + rh - 1, cw, 1), t.grid_lines);
                }
                // (the columns' lines, down to the last row)
                let bottom = (rh + 1 + (last - l.top) as i64 * rh).min(ch);
                let mut x = l.gutter - l.left;
                for colw in &l.cols {
                    x += colw + 1;
                    if x > l.gutter && x <= cw {
                        p.fill((x - 1, rh, 1, bottom - rh), t.grid_lines);
                    }
                }
                // (the heading, over the rows)
                p.fill((0, 0, cw, rh + 1), t.face);
                p.fill((0, rh, cw, 1), t.fixed_lines);
                p.fill((l.gutter - 1, 0, 1, bottom), t.fixed_lines);
                let mut x = l.gutter - l.left;
                for (c, colw) in l.cols.iter().enumerate() {
                    if x + colw >= l.gutter && x < cw {
                        let sorted = d.sort.filter(|(s, _)| *s == c);
                        p.clipped((x.max(l.gutter), 0, (x + colw).min(cw) - x.max(l.gutter), rh), |p| {
                            let room = if sorted.is_some() { colw - 22 } else { colw - 10 };
                            p.text((x + 5, (rh - line_h) / 2, room, line_h), names.get(c).map_or("", String::as_str), &bold, t.text, Place::TopLeft);
                            if let Some((_, desc)) = sorted {
                                let (mx, my) = ((x + colw - 11) as f64, (rh / 2) as f64);
                                let points = if desc { [(mx - 4.0, my - 2.0), (mx + 4.0, my - 2.0), (mx, my + 3.0)] } else { [(mx - 4.0, my + 2.0), (mx + 4.0, my + 2.0), (mx, my - 3.0)] };
                                p.op(Op::Arrow { points, color: t.accent });
                            }
                        });
                        p.fill((x + colw, 0, 1, rh), t.fixed_lines);
                    }
                    x += colw + 1;
                }
                // (the gutter's heading: the rows' count)
                p.fill((0, 0, l.gutter - 1, rh), t.face);
            });
            p.ops(l.bars.iter().cloned());
        });
        if cx.state.focused {
            p.items_focus(w, h);
        }
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        let Some(d) = link::grid(cx.id) else { return MouseOut::default() };
        let (w, h) = (cx.width(), cx.height());
        let (x, y) = (m.x.floor() as i64 - 2, m.y.floor() as i64 - 2);
        // (the scroll bars first)
        let row_h = row_height(&cx.font);
        let on_bars = SCROLLERS.with(|s| {
            let mut s = s.borrow_mut();
            let Some(sc) = s.get_mut(cx.id) else { return false };
            let took = match m.kind {
                MouseKind::Down => sc.mouse_down(x, y, w - 4, h - 4).is_some(),
                MouseKind::Move if sc.pressed.is_some() => {
                    sc.mouse_drag(x, y, w - 4, h - 4);
                    true
                }
                MouseKind::Up if sc.pressed.is_some() => {
                    sc.mouse_up(w - 4, h - 4);
                    true
                }
                _ => false,
            };
            if took {
                link::set_grid_top(cx.id, (sc.vert.position / row_h.max(1)) as usize);
                HELD.with(|held| {
                    let mut held = held.borrow_mut();
                    if m.kind == MouseKind::Up {
                        held.remove(cx.id);
                    } else {
                        held.insert(cx.id.to_string(), (x, y));
                    }
                });
            }
            took
        });
        if on_bars {
            match m.kind {
                MouseKind::Down => cx.ui.wake = Some(crate::tick::now() + crate::tick::REPEAT_DELAY),
                MouseKind::Up => cx.ui.wake = None,
                _ => {}
            }
            cx.events.push(KernelEvent::Set { id: cx.id.to_string(), prop: "toprow".into(), value: SCROLLERS.with(|s| s.borrow().get(cx.id).map_or(0, |sc| sc.vert.position / row_h.max(1))) });
            return MouseOut::default();
        }
        if m.kind != MouseKind::Down {
            return MouseOut::default();
        }
        let l = layout(cx, &d);
        if x < 0 || y < 0 || x >= l.inner.0 || y >= l.inner.1 {
            return MouseOut::default();
        }
        let col = if x >= l.gutter { column_at(&l, x) } else { None };
        if y <= l.row_h {
            // (a heading: the view sorted by it)
            if let Some(c) = col {
                link::grid_title_click(cx.id, c);
                cx.events.push(KernelEvent::Fire { id: cx.id.to_string(), event: "ontitleclick".into(), args: vec![v_int(c as i64)] });
            }
            return MouseOut::default();
        }
        let v = l.top + ((y - l.row_h - 1) / l.row_h) as usize;
        if v < d.rows {
            let order = link::grid_order(cx.id, &d);
            Self::go_to_view(cx, &d, &l, &order, v as i64);
            let r = data_row(&order, v);
            cx.events.push(KernelEvent::Fire { id: cx.id.to_string(), event: "oncellclick".into(), args: vec![v_int(r as i64), v_int(col.map_or(-1, |c| c as i64))] });
        }
        MouseOut::default()
    }

    fn key(&self, cx: &mut Cx, k: &KeyIn, _clip: &mut dyn Clipboard) -> bool {
        let Some(d) = link::grid(cx.id) else { return false };
        let l = layout(cx, &d);
        let order = link::grid_order(cx.id, &d);
        let now = view_row(&order, d.row) as i64;
        let page = l.page as i64;
        let to = match k.vk {
            38 => now - 1,
            40 => now + 1,
            33 => now - page,
            34 => now + page,
            36 => 0,
            35 => d.rows as i64 - 1,
            _ => return false,
        };
        Self::go_to_view(cx, &d, &l, &order, to);
        true
    }

    fn wheel(&self, cx: &mut Cx, dx: f64, dy: f64, mods: crate::input::Mods) -> bool {
        let (w, h) = (cx.width(), cx.height());
        let horizontal = dy == 0.0 || mods.shift;
        let notches = whole_notches(cx.id, if dy == 0.0 { dx } else { dy });
        let row_h = row_height(&cx.font);
        let top = SCROLLERS.with(|s| {
            let mut s = s.borrow_mut();
            let sc = s.get_mut(cx.id)?;
            if !(sc.vert.shown || sc.horz.shown) {
                return None;
            }
            if notches != 0 {
                sc.wheel(notches, horizontal, w - 4, h - 4);
            }
            Some(sc.vert.position / row_h.max(1))
        });
        match top {
            Some(t) => {
                link::set_grid_top(cx.id, t as usize);
                true
            }
            None => false,
        }
    }

    fn tick(&self, cx: &mut Cx) {
        let Some((x, y)) = HELD.with(|held| held.borrow().get(cx.id).copied()) else { return };
        let (w, h) = (cx.width(), cx.height());
        let row_h = row_height(&cx.font);
        SCROLLERS.with(|s| {
            if let Some(sc) = s.borrow_mut().get_mut(cx.id) {
                sc.repeat(x, y, w - 4, h - 4);
                link::set_grid_top(cx.id, (sc.vert.position / row_h.max(1)) as usize);
            }
        });
        cx.ui.wake = Some(crate::tick::now() + crate::tick::REPEAT);
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        let mut n = super::shared_describe(cx, "RDBGRID");
        let Some(d) = link::grid(cx.id) else { return n };
        let l = layout(cx, &d);
        let order = link::grid_order(cx.id, &d);
        let (names, rows): (Vec<String>, Vec<(usize, Vec<String>)>) = frame::with_frame(&d.dataset, |f| {
            let last = (l.top + l.page + 1).min(d.rows);
            let rows = (l.top..last).map(|v| {
                let r = data_row(&order, v);
                (r, (0..f.width()).map(|c| f.cell(r, c).map_or_else(|| "null".to_string(), |s| s.into_owned())).collect())
            });
            (f.columns.clone(), rows.collect())
        });
        n.description = format!("{} rows, {} columns", d.rows, names.len());
        let (x0, y0) = (cx.rect.0 + 2, cx.rect.1 + 2);
        let mut head = AccessNode::new(part_id(cx.id, PART_ROW, 0), Role::Row);
        head.name = "Headings".into();
        head.bounds = (x0, y0, l.inner.0, l.row_h);
        for (c, name) in names.iter().enumerate() {
            let mut cell = AccessNode::new(part_id(cx.id, PART_CELL, c), Role::Cell);
            cell.name = name.clone();
            head.children.push(cell);
        }
        n.children.push(head);
        for (i, (r, cells)) in rows.into_iter().enumerate() {
            let mut row = AccessNode::new(part_id(cx.id, PART_ROW, r + 1), Role::Row);
            row.name = format!("Row {r} of {}", d.rows);
            row.value = Some(names.iter().zip(&cells).map(|(n, v)| format!("{n} {v}")).collect::<Vec<_>>().join(", "));
            row.states.selected = Some(r == d.row);
            row.bounds = (x0, y0 + l.row_h + 1 + i as i64 * l.row_h, l.inner.0, l.row_h);
            row.actions.push(Action::Click);
            n.children.push(row);
        }
        n
    }

    fn access(&self, cx: &mut Cx, action: Action, part: Option<usize>, _value: Option<&AccessValue>) -> bool {
        // (a row clicked by a screen reader: the current row)
        let (Action::Click, Some(p)) = (action, part) else { return false };
        let Some(d) = link::grid(cx.id) else { return false };
        let r = p;
        if r == 0 || r > d.rows {
            return false;
        }
        let l = layout(cx, &d);
        let order = link::grid_order(cx.id, &d);
        Self::go_to_view(cx, &d, &l, &order, view_row(&order, r - 1) as i64);
        true
    }

    /// (tests) `__cell_C_R`: the cell in column C of view row R clicked (the
    /// current row, OnCellClick); R = -1: heading C clicked (the view
    /// sorted, OnTitleClick).
    fn test_action(&self, cx: &mut Cx, action: &str) -> bool {
        let Some((c, r)) = action.strip_prefix("__cell_").and_then(|rc| rc.split_once('_')).and_then(|(c, r)| Some((c.parse::<i64>().ok()?, r.parse::<i64>().ok()?))) else { return false };
        let Some(d) = link::grid(cx.id) else { return false };
        if r < 0 {
            link::grid_title_click(cx.id, c.max(0) as usize);
            cx.events.push(KernelEvent::Fire { id: cx.id.to_string(), event: "ontitleclick".into(), args: vec![v_int(c)] });
            return true;
        }
        let l = layout(cx, &d);
        let order = link::grid_order(cx.id, &d);
        Self::go_to_view(cx, &d, &l, &order, r);
        let row = data_row(&order, (r.max(0) as usize).min(d.rows.saturating_sub(1)));
        cx.events.push(KernelEvent::Fire { id: cx.id.to_string(), event: "oncellclick".into(), args: vec![v_int(row as i64), v_int(c)] });
        true
    }

    fn tip_at(&self, _store: &dyn crate::store::Store, _id: &str, _x: f64, _y: f64) -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    use rapidr_value::datascience::{frame, link};
    use rapidr_value::{v_int, v_str};

    use crate::{FormUi, Item, MemStore, Op, TextSystem};

    fn texts(list: &crate::DisplayList) -> Vec<String> {
        list.items.iter().filter_map(|i| if let Item::Op { op: Op::Text { text, .. }, .. } = i { Some(text.clone()) } else { None }).collect()
    }

    fn form(id: &str) -> MemStore {
        let mut s = MemStore::new();
        s.add("f", "RFORM", None);
        s.set("f", "width", v_int(500)).set("f", "height", v_int(400));
        s.add(id, "RDBGRID", Some("f"));
        s.set(id, "left", v_int(10)).set(id, "top", v_int(10)).set(id, "width", v_int(400)).set(id, "height", v_int(200));
        s
    }

    #[test]
    fn draws_only_the_rows_on_screen_of_a_million() {
        let mut csv = String::from("id,name,score\n");
        for i in 0..1_000_000 {
            csv.push_str(&format!("{i},p{i},{}\n", i % 97));
        }
        frame::set_frame("kg_df", frame::parse_csv(&csv));
        link::create("kg_df", "RDATAFRAME");
        // (the store makes the grid's model, as a runtime does)
        let s = form("kg_grid");
        link::set("kg_grid", "datasource", &v_str("kg_df"));
        let mut ts = TextSystem::new();
        let mut f = FormUi::build(&s, "f", true);
        let start = std::time::Instant::now();
        let list = f.paint(&s, &mut ts, 1.0);
        let t = texts(&list);
        assert!(t.contains(&"name".to_string()) && t.contains(&"p3".to_string()), "{t:?} {:?} {}", link::grid("kg_grid"), list.items.len());
        assert!(!t.contains(&"p500".to_string()));
        // (a few dozen texts, not a million)
        assert!(t.len() < 200, "{}", t.len());
        assert!(start.elapsed().as_millis() < 2000);
        // (row 41 made current: scrolled into view, highlighted)
        link::set_row("kg_grid", 41);
        let list = f.paint(&s, &mut ts, 1.0);
        assert!(texts(&list).contains(&"p41".to_string()));
        // (a heading clicked: sorted by score, the data as it was)
        link::grid_title_click("kg_grid", 2);
        let d = link::grid("kg_grid").unwrap();
        assert_eq!(d.sort, Some((2, false)));
        assert_eq!(frame::frame("kg_df").cell(0, 1).as_deref(), Some("p0"));
    }
}

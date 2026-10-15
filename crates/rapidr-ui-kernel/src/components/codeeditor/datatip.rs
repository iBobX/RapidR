//! A debugger's data tip (`ShowDataTip`, docs/manual/debugging.md): the
//! value under a resting mouse as a tree that opens to any depth — a
//! triangle per value with children, its name, its value (in the colour of
//! its kind, a changed one marked), its type at the right; a table's rows
//! (an RDataFrame's) as a grid under its columns' names. Over a row, its
//! actions: copy the value, add it to the watches, see a string's whole
//! text. It lives in the window's popup layer, over every component, and
//! stays while the mouse is over it (the wheel scrolls a long one).
//!
//! The program fills it: a triangle clicked is OnDataTipExpand(Id, Open),
//! an action OnDataTipAction(Id, Action) — `copy`, `watch`, `view` — and
//! the program shows the tree again (its children fetched).

use rapidr_value::code_scheme::Scheme;
use rapidr_value::objects::codeedit::{CodeEditor as Model, DataTip, TipRow};
use rapidr_value::objects::font::Font;
use rapidr_value::objects::ops::{Op, Place, Rect};
use rapidr_value::objects::text::{text_size, CODE_FACE};
use rapidr_value::objects::{with_code, with_code_mut};
use rapidr_value::{v_int, v_str};

use super::paint::{caret_x, shown_at};
use super::CodeUi;
use crate::input::KernelEvent;
use crate::paint::Painter;
use crate::tree::FormUi;

/// A row's height, and how many show at once.
pub const ROW_H: i64 = 20;
pub const VISIBLE_ROWS: usize = 16;
/// A level's indent; the triangle's column.
const INDENT: i64 = 14;
const TRIANGLE: i64 = 16;
/// The actions over the row under the mouse: their icons and names.
const ACTIONS: [(&str, &str); 3] = [("actions/copy", "copy"), ("actions/watch", "watch"), ("actions/search", "view")];
const ICON: i64 = 16;
/// The widest a cell of a table, a value, a type show (then "…").
const CELL_MAX: i64 = 150;
const VALUE_MAX: i64 = 380;
const KIND_MAX: i64 = 130;

fn code_font(px: i64) -> Font {
    Font { name: CODE_FACE.into(), size: -px.max(8), ..Font::default() }
}

fn ui_font() -> Font {
    Font { name: "Segoe UI".into(), size: -12, ..Font::default() }
}

/// Where the tip sits and how its columns fall (absolute logical pixels).
#[derive(Debug, Clone)]
pub struct Layout {
    pub rect: Rect,
    /// Rows shown at once.
    pub rows: usize,
    /// The names' column (after the indent and the triangle) and the
    /// values' (from the box's left).
    pub value_x: i64,
    pub kind_w: i64,
    /// A table's columns' widths (its header and rows), and where its
    /// cells start.
    pub cells: Vec<i64>,
    pub cells_x: i64,
    /// The word's row: top and bottom (the corridor to the tip).
    pub word: (i64, i64, i64),
}

/// The tip's layout for editor `c` (its view `ui`, at `abs` in the
/// window, `client`: the window's size).
pub fn layout(c: &Model, ui: &CodeUi, abs: (i64, i64), scale: f64, client: (i64, i64)) -> Option<Layout> {
    let t = c.datatip.as_ref()?;
    let sh = shown_at(ui, c, t.start)?;
    // (the word's left edge: caret_x is the window's already, as the
    // completion list's anchor takes it)
    let ax = (caret_x(ui, sh, t.start) / scale).round() as i64;
    let top = abs.1 + sh.top.round() as i64;
    let bottom = top + ui.metrics.line;
    let cf = code_font(ui.font.pixel_size().max(8));
    let uf = ui_font();
    let mut name_w = 0;
    let mut value_w = 0;
    let mut kind_w = 0;
    let mut cells: Vec<i64> = Vec::new();
    let mut label_w = 0;
    let mut table_indent = 0;
    for r in &t.rows {
        let indent = r.depth as i64 * INDENT + TRIANGLE;
        if r.table {
            table_indent = table_indent.max(indent);
            label_w = label_w.max(text_size(&r.name, &cf).0 + 10);
            for (k, cell) in r.cells.iter().enumerate() {
                let f = if r.header { Font { styles: 1, ..cf.clone() } } else { cf.clone() };
                let w = (text_size(cell, &f).0 + 14).min(CELL_MAX);
                if k >= cells.len() {
                    cells.push(w);
                } else {
                    cells[k] = cells[k].max(w);
                }
            }
        } else {
            name_w = name_w.max(indent + text_size(&r.name, &cf).0);
            value_w = value_w.max(text_size(&r.value, &cf).0.min(VALUE_MAX));
            kind_w = kind_w.max(text_size(&r.kind, &uf).0.min(KIND_MAX));
        }
    }
    let actions_w = ACTIONS.len() as i64 * (ICON + 6) + 6;
    let tree_w = 8 + name_w + text_size(" = ", &cf).0 + value_w + 16 + kind_w.max(actions_w) + 8;
    let table_w = 8 + table_indent + label_w + cells.iter().sum::<i64>() + 8 + actions_w;
    let w = tree_w.max(table_w).clamp(220, 760).min(client.0 - 8).max(160);
    let rows = t.rows.len().min(VISIBLE_ROWS);
    let h = rows as i64 * ROW_H + 8;
    // (under the word, as an editor's data tip is; above it when there's
    // no room below)
    let y = if bottom + 2 + h <= client.1 || top - h - 2 < 0 { bottom + 2 } else { top - h - 2 };
    let x = (ax - 8).clamp(0, (client.0 - w).max(0));
    let value_x = 8 + name_w + text_size(" = ", &cf).0;
    Some(Layout { rect: (x, y, w, h), rows, value_x, kind_w, cells, cells_x: 8 + table_indent + label_w, word: (ax, top, bottom) })
}

/// Whether absolute point (`x`, `y`) is over the tip, or on the way to it
/// from its word (the band between the word's row and the tip): a mouse
/// going there keeps it.
pub fn holds(l: &Layout, x: f64, y: f64) -> bool {
    let (rx, ry, rw, rh) = l.rect;
    let over = x >= rx as f64 && x < (rx + rw) as f64 && y >= ry as f64 && y < (ry + rh) as f64;
    let (top, bottom) = (l.word.1.min(ry), l.word.2.max(ry + rh));
    let corridor = x >= rx as f64 && x < (rx + rw) as f64 && y >= top as f64 && y < bottom as f64;
    over || corridor
}

/// The row at absolute `y` (none outside the tip's rows).
fn row_at(t: &DataTip, l: &Layout, y: f64) -> Option<usize> {
    let k = ((y - l.rect.1 as f64 - 4.0) / ROW_H as f64).floor();
    (k >= 0.0 && (k as usize) < l.rows).then(|| t.top + k as usize).filter(|&i| i < t.rows.len())
}

/// The action whose icon is at absolute `x` on the row under the mouse.
fn action_at(l: &Layout, row: &TipRow, x: f64) -> Option<&'static str> {
    action_rects(l, row).into_iter().find(|(r, _, _)| x >= r.0 as f64 && x < (r.0 + r.2) as f64).map(|(_, _, a)| a)
}

/// The actions a row offers, right to left from the box's right edge:
/// copy and watch for every value, the text viewer for a string.
fn action_rects(l: &Layout, row: &TipRow) -> Vec<(Rect, &'static str, &'static str)> {
    let mut out = Vec::new();
    // (a row that isn't a value — a table's header, "… more", one still
    // coming — has none)
    if row.header || row.kind.is_empty() {
        return out;
    }
    let mut x = l.rect.0 + l.rect.2 - 6;
    for (icon, action) in ACTIONS.iter().rev() {
        if *action == "view" && row.kind != "String" {
            continue;
        }
        x -= ICON + 6;
        out.push(((x, 0, ICON + 4, ROW_H), *icon, *action));
    }
    out.reverse();
    out
}

/// `s` cut to `width` pixels in `font` ("…" at its end when cut).
fn fit(s: &str, font: &Font, width: i64) -> String {
    if text_size(s, font).0 <= width {
        return s.to_string();
    }
    let mut chars: Vec<char> = s.chars().collect();
    while !chars.is_empty() {
        chars.pop();
        let t: String = chars.iter().collect::<String>() + "…";
        if text_size(&t, font).0 <= width {
            return t;
        }
    }
    String::new()
}

/// A value's colour by its kind: strings, numbers, truth values and the
/// rest (types, components) as the code shows them.
fn value_color(sc: &Scheme, kind: &str) -> u32 {
    match kind {
        "String" | "str" => sc.token(["string"]).color,
        "Integer" | "Double" | "i64" | "f64" => sc.token(["number", "constant"]).color,
        "Boolean" | "bool" => sc.token(["keyword", "constant"]).color,
        "Empty" | "" => sc.popup_detail,
        _ => sc.token(["type", "function"]).color,
    }
}

/// Draws the tip.
pub fn paint(c: &Model, ui: &CodeUi, abs: (i64, i64), scale: f64, client: (i64, i64), p: &mut Painter) {
    let Some(t) = c.datatip.as_ref() else { return };
    let Some(l) = layout(c, ui, abs, scale, client) else { return };
    let sc = rapidr_value::code_scheme::resolve(&c.opts.color_scheme);
    let (x, y, w, h) = l.rect;
    super::popup::frame(p, sc, l.rect);
    let cf = code_font(ui.font.pixel_size().max(8));
    let bold = Font { styles: 1, ..cf.clone() };
    let uf = ui_font();
    let th = text_size("Ag", &cf).1;
    let eq_w = text_size(" = ", &cf).0;
    p.clipped((x + 1, y + 1, w - 2, h - 2), |p| {
        for (k, r) in t.rows.iter().enumerate().skip(t.top).take(l.rows) {
            let ry = y + 4 + (k - t.top) as i64 * ROW_H;
            let ty = ry + (ROW_H - th) / 2;
            let hot = t.hot == Some(k);
            if hot && !r.header {
                p.op(Op::Round { rect: (x + 3, ry, w - 6, ROW_H), radius: 3.0, fill: Some(super::paint::blend(sc.popup_selected, sc.popup, 0.18)), stroke: None, width: 1.0 });
            }
            let indent = x + 8 + r.depth as i64 * INDENT;
            if r.expandable {
                p.chevron((indent + TRIANGLE / 2 - 2) as f64, (ry + ROW_H / 2) as f64, 7.0, r.open, sc.popup_detail);
            }
            if r.table {
                // (a table's row: its index, then its cells in columns)
                let mut cx = x + l.cells_x;
                if r.header {
                    p.fill((x + 6, ry + ROW_H - 1, w - 12, 1), sc.popup_border);
                } else {
                    p.text((indent + TRIANGLE, ty, l.cells_x - (indent - x) - TRIANGLE, th), &r.name, &cf, sc.popup_detail, Place::TopLeft);
                }
                for (i, cell) in r.cells.iter().enumerate() {
                    let cw = l.cells.get(i).copied().unwrap_or(60);
                    let f = if r.header { &bold } else { &cf };
                    let shown = fit(cell, f, cw - 12);
                    let color = if r.header { sc.popup_text } else if cell.parse::<f64>().is_ok() { value_color(sc, "Double") } else { sc.popup_text };
                    p.text((cx + 4, ty, cw - 8, th), &shown, f, color, Place::TopLeft);
                    if i > 0 {
                        p.fill((cx, ry + 3, 1, ROW_H - 6), sc.popup_border);
                    }
                    cx += cw;
                }
            } else {
                let name_x = indent + TRIANGLE;
                p.text((name_x, ty, (x + l.value_x - name_x).max(10), th), &r.name, &cf, sc.popup_text, Place::TopLeft);
                p.text((x + l.value_x - eq_w, ty, eq_w + 2, th), " = ", &cf, sc.popup_detail, Place::TopLeft);
                // (the actions take the type's place on the row under the mouse)
                let right = if hot { action_rects(&l, r).first().map_or(x + w - 8, |a| a.0 .0 - 6) } else { x + w - 12 - l.kind_w - 10 };
                let vw = (right - (x + l.value_x)).max(20);
                let shown = fit(&r.value, if r.changed { &bold } else { &cf }, vw);
                let color = if r.changed { sc.mark_changed } else { value_color(sc, &r.kind) };
                p.text((x + l.value_x, ty, vw + 2, th), &shown, if r.changed { &bold } else { &cf }, color, Place::TopLeft);
                if !hot && !r.kind.is_empty() {
                    let kind = fit(&r.kind, &uf, l.kind_w);
                    let kw = text_size(&kind, &uf).0;
                    p.text((x + w - 12 - kw, ty + 1, kw + 4, th), &kind, &uf, sc.popup_detail, Place::TopLeft);
                }
            }
            if hot && !r.header {
                for ((ax, _, aw, _), icon, _) in action_rects(&l, r) {
                    p.icon(icon, (ax + 2, ry + (ROW_H - ICON) / 2, ICON, ICON), Some(sc.popup_detail), false);
                    let _ = aw;
                }
            }
        }
        // (a scroll indicator)
        if t.rows.len() > l.rows {
            let track = h - 8;
            let bar = ((l.rows as f64 / t.rows.len() as f64) * track as f64).max(16.0) as i64;
            let by = y + 4 + ((t.top as f64 / (t.rows.len() - l.rows) as f64) * (track - bar) as f64) as i64;
            p.op(Op::Round { rect: (x + w - 6, by, 3, bar), radius: 1.5, fill: Some(sc.minimap_slider), stroke: None, width: 1.0 });
        }
    });
}

/// The code editors of form `f` with a tip showing: (node, layout).
fn tips(f: &FormUi) -> Vec<(usize, Layout)> {
    let client = (f.client.0, f.client.1 + f.menu_offset);
    f.nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.shown)
        .filter_map(|(i, n)| {
            let ui = n.ui.code.as_ref()?;
            with_code(&n.id, |c| layout(c, ui, (n.abs.0, n.abs.1), f.scale, client)).flatten().map(|l| (i, l))
        })
        .collect()
}

/// A press at (x, y) of the form: on a tip, its triangle opens or closes
/// the row (OnDataTipExpand), an action icon does its action
/// (OnDataTipAction); anywhere on the tip, nothing else happens (true).
pub fn mouse_down(f: &mut FormUi, x: f64, y: f64, double: bool) -> bool {
    for (i, l) in tips(f) {
        let (rx, ry, rw, rh) = l.rect;
        if !(x >= rx as f64 && x < (rx + rw) as f64 && y >= ry as f64 && y < (ry + rh) as f64) {
            continue;
        }
        let id = f.nodes[i].id.clone();
        let fire = with_code(&id, |c| {
            let t = c.datatip.as_ref()?;
            let k = row_at(t, &l, y)?;
            let r = &t.rows[k];
            if r.header {
                return None;
            }
            if let Some(action) = action_at(&l, r, x) {
                return Some(("ondatatipaction", vec![v_str(&r.id), v_str(action)]));
            }
            let indent = rx + 8 + r.depth as i64 * INDENT;
            let on_triangle = x >= indent as f64 - 2.0 && x < (indent + TRIANGLE + 2) as f64;
            (r.expandable && (on_triangle || double)).then(|| ("ondatatipexpand", vec![v_str(&r.id), v_int(i64::from(!r.open))]))
        })
        .flatten();
        if let Some((event, args)) = fire {
            f.events.push(KernelEvent::Fire { id, event: event.to_string(), args });
        }
        f.dirty = true;
        return true;
    }
    false
}

/// The mouse moved to (x, y) of the form: the row under it marked on a
/// tip — true when the mouse is over one or on its way there from its
/// word (the tip keeps it: the editor below doesn't see the move).
pub fn mouse_move(f: &mut FormUi, x: f64, y: f64) -> bool {
    let mut held = false;
    for (i, l) in tips(f) {
        let (rx, ry, rw, rh) = l.rect;
        let over = x >= rx as f64 && x < (rx + rw) as f64 && y >= ry as f64 && y < (ry + rh) as f64;
        let id = f.nodes[i].id.clone();
        held |= holds(&l, x, y);
        let changed = with_code_mut(&id, |c| {
            let Some(t) = c.datatip.as_mut() else { return false };
            let hot = if over { row_at(t, &l, y) } else { None };
            std::mem::replace(&mut t.hot, hot) != hot
        })
        .unwrap_or(false);
        if changed {
            f.dirty = true;
        }
    }
    held
}

/// Whether (x, y) of the form keeps editor node `i`'s tip (over it, or on
/// the way there).
pub fn keeps(f: &FormUi, i: usize, x: f64, y: f64) -> bool {
    tips(f).iter().any(|(n, l)| *n == i && holds(l, x, y))
}

/// The wheel over a tip scrolls it.
pub fn wheel(f: &mut FormUi, x: f64, y: f64, dy: f64) -> bool {
    for (i, l) in tips(f) {
        let (rx, ry, rw, rh) = l.rect;
        if !(x >= rx as f64 && x < (rx + rw) as f64 && y >= ry as f64 && y < (ry + rh) as f64) {
            continue;
        }
        let id = f.nodes[i].id.clone();
        with_code_mut(&id, |c| {
            if let Some(t) = c.datatip.as_mut() {
                let step = if dy > 0.0 { 3 } else { -3 };
                t.top = (t.top as i64 + step).clamp(0, t.rows.len().saturating_sub(l.rows) as i64) as usize;
            }
        });
        f.dirty = true;
        return true;
    }
    false
}

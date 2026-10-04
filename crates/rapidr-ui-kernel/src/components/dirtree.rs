//! QDIRTREE: the shared model (`rapidr_value::objects::dirtree::DirTree`)
//! walks the directories (the open ones' subdirectories, depth first) and
//! keeps the selected one; this draws them as Windows' directory outline —
//! a sunken white box, each directory a row indented by its depth, a ±
//! box where it has (or may have) subdirectories, a folder, its name, the
//! selected one white on blue, scrolled into view when it changes — and
//! routes the mouse and keys: a click selects a directory (OnChange), a
//! click on its box or a double click opens or closes it, the arrows move
//! the selection.

use std::cell::RefCell;
use std::collections::HashMap;

use rapidr_value::objects::a11y::{node_id, part_id, AccessNode, Action, Role};
use rapidr_value::objects::dirtree::Row;
use rapidr_value::objects::ops::Place;
use rapidr_value::objects::with_dirtree;

use super::list::{background, bar_mouse, bar_tick, scroll_into_view, sunken, vscroll, vscroll_state};
use super::{ComponentKind, Cx, KeyIn, MouseIn, MouseKind, MouseOut};
use crate::a11y::AccessValue;
use crate::input::Clipboard;
use crate::paint::{Painter, GRAY_TEXT, HIGHLIGHT, HIGHLIGHT_TEXT, SHADOW};
use crate::text::bgr_to_rgb;

/// A row's height and a level's indent.
const ROW: i64 = 18;
const INDENT: i64 = 16;
/// The folder's yellow and its outline.
const FOLDER: u32 = 0xF0C850;
const FOLDER_EDGE: u32 = 0x9C7A1C;

thread_local! {
    /// The directory each tree last showed selected (a new one is
    /// scrolled into view).
    static SHOWN: RefCell<HashMap<String, std::path::PathBuf>> = RefCell::new(HashMap::new());
}

pub struct DirTreeBox;

fn rows(id: &str) -> (Vec<Row>, Option<usize>) {
    with_dirtree(id, |t| {
        let rows = t.rows();
        let sel = rows.iter().position(|r| r.path == t.directory);
        (rows, sel)
    })
    .unwrap_or_default()
}

/// The row at y (component coordinates).
fn row_at(id: &str, y: f64, count: usize) -> Option<usize> {
    let (pos, _, _) = vscroll_state(id);
    let y = y.floor() as i64 - 2 + pos;
    (y >= 0).then_some((y / ROW) as usize).filter(|&i| i < count)
}

impl DirTreeBox {
    /// Row `i` clicked: selected (OnChange when the directory changed).
    fn select(cx: &mut Cx, i: usize) {
        if with_dirtree(cx.id, |t| t.click(i)).unwrap_or(false) {
            cx.change();
        }
    }

    /// Row `i` opened or closed.
    fn toggle(cx: &mut Cx, i: usize) {
        with_dirtree(cx.id, |t| t.toggle(i));
        cx.ui.dragging = false;
    }
}

impl ComponentKind for DirTreeBox {
    fn name(&self) -> &'static str {
        "RDIRTREE"
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let (w, h) = (cx.width(), cx.height());
        sunken(p, w, h, background(cx));
        let (rows, sel) = rows(cx.id);
        // (a newly selected directory scrolled into the middle)
        let dir = with_dirtree(cx.id, |t| t.directory.clone());
        let fresh = SHOWN.with(|s| {
            let mut s = s.borrow_mut();
            let fresh = s.get(cx.id) != dir.as_ref();
            if let Some(d) = dir {
                s.insert(cx.id.to_string(), d);
            }
            fresh
        });
        if let (true, Some(i)) = (fresh, sel) {
            let mid = (i as i64 * ROW - (h - 4) / 2 + ROW / 2).max(0);
            scroll_into_view(cx.id, mid, mid + h - 4, h - 4);
        }
        let (pos, cw, bar) = vscroll(cx.id, w - 4, h - 4, rows.len() as i64 * ROW, ROW);
        let font = cx.font.clone();
        let (focused, enabled) = (cx.state.focused, cx.state.enabled);
        p.at((2, 2), |p| {
            p.clipped((0, 0, cw, h - 4), |p| {
                let first = (pos / ROW).max(0) as usize;
                for (i, row) in rows.iter().enumerate().skip(first) {
                    let top = i as i64 * ROW - pos;
                    if top >= h - 4 {
                        break;
                    }
                    let x = 2 + row.depth as i64 * INDENT;
                    let mid = top + ROW / 2;
                    if row.has_children {
                        let (bx, by) = (x, mid - 4);
                        p.fill((bx, by, 9, 9), 0xFFFFFF);
                        p.edge((bx, by, 9, 9), &[SHADOW], &[SHADOW]);
                        p.fill((bx + 2, mid, 5, 1), 0x000000);
                        if !row.expanded {
                            p.fill((bx + 4, by + 2, 1, 5), 0x000000);
                        }
                    }
                    // (a folder: its tab and its body)
                    let fx = x + 12;
                    p.fill((fx, mid - 5, 6, 2), FOLDER);
                    p.fill((fx, mid - 3, 14, 9), FOLDER);
                    p.edge((fx, mid - 3, 14, 9), &[FOLDER_EDGE], &[FOLDER_EDGE]);
                    let tx = fx + 18;
                    let (tw, _) = rapidr_value::objects::text::text_size(&row.name, &font);
                    let selected = sel == Some(i);
                    if selected {
                        p.fill((tx, top + 1, tw + 4, ROW - 2), if focused { HIGHLIGHT } else { 0xD0D0D0 });
                    }
                    let color = if !enabled {
                        GRAY_TEXT
                    } else if selected && focused {
                        HIGHLIGHT_TEXT
                    } else {
                        bgr_to_rgb(font.color)
                    };
                    p.text((tx + 2, top, tw + 2, ROW), &row.name, &font, color, Place::Left);
                    if selected && focused {
                        p.focus((tx, top + 1, tw + 4, ROW - 2));
                    }
                }
            });
            p.ops(bar);
        });
    }

    fn wheel(&self, cx: &mut Cx, _dx: f64, dy: f64, _mods: crate::input::Mods) -> bool {
        super::list::vscroll_wheel(cx.id, dy, cx.width() - 4, cx.height() - 4)
    }

    /// (the input lane's) A held bar's repeat.
    fn tick(&self, cx: &mut Cx) {
        bar_tick(cx);
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        let (w, h) = (cx.width(), cx.height());
        let inner = MouseIn { x: m.x - 2.0, y: m.y - 2.0, ..*m };
        if bar_mouse(cx, &inner, w - 4, h - 4) || m.kind != MouseKind::Down {
            return MouseOut::default();
        }
        let (rows, _) = rows(cx.id);
        let Some(i) = row_at(cx.id, m.y, rows.len()) else { return MouseOut::default() };
        let x = m.x.floor() as i64 - 2 - 2 - rows[i].depth as i64 * INDENT;
        if rows[i].has_children && (0..10).contains(&x) {
            Self::toggle(cx, i);
            return MouseOut::default();
        }
        // (a double click's second press opens or closes it)
        let dbl = m.double();
        if dbl {
            Self::toggle(cx, i);
        }
        Self::select(cx, i);
        MouseOut::default()
    }

    fn key(&self, cx: &mut Cx, k: &KeyIn, _clip: &mut dyn Clipboard) -> bool {
        if k.mods.alt || k.mods.command {
            return false;
        }
        let (rows, sel) = rows(cx.id);
        let Some(at) = sel else { return false };
        let next = match k.vk {
            40 => (at + 1).min(rows.len().saturating_sub(1)),
            38 => at.saturating_sub(1),
            36 => 0,
            35 => rows.len().saturating_sub(1),
            39 | 37 | 13 => {
                let open = rows[at].expanded;
                if (k.vk == 39 && !open) || (k.vk == 37 && open) || k.vk == 13 {
                    Self::toggle(cx, at);
                }
                return true;
            }
            _ => return false,
        };
        if next != at {
            Self::select(cx, next);
            let (pos, _, _) = vscroll_state(cx.id);
            let top = next as i64 * ROW;
            let _ = pos;
            scroll_into_view(cx.id, top, top + ROW, cx.height() - 4);
        }
        true
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        let mut n = AccessNode::new(node_id(cx.id), Role::Tree);
        n.actions = vec![Action::Focus];
        n.bounds = cx.rect;
        let (rows, sel) = rows(cx.id);
        for (i, row) in rows.iter().enumerate().take(1_000) {
            let mut item = AccessNode::new(part_id(cx.id, 1, i), Role::TreeItem);
            item.name = row.name.clone();
            item.states.selected = Some(sel == Some(i));
            item.states.expanded = row.has_children.then_some(row.expanded);
            item.actions = vec![Action::Click];
            n.children.push(item);
        }
        n
    }

    fn access(&self, cx: &mut Cx, action: Action, part: Option<usize>, _value: Option<&AccessValue>) -> bool {
        let Some(i) = part else { return false };
        match action {
            Action::Click => Self::select(cx, i),
            Action::Expand | Action::Collapse => Self::toggle(cx, i),
            _ => return false,
        }
        true
    }

    /// `__item_i` / `__node_i`: a click on row `i`.
    fn test_action(&self, cx: &mut Cx, action: &str) -> bool {
        let Some(i) = action.strip_prefix("__item_").or_else(|| action.strip_prefix("__node_")).and_then(|s| s.parse::<usize>().ok()) else { return false };
        Self::select(cx, i);
        true
    }
}

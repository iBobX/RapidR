//! RPROJECTTREE's look: Xcode's navigator and VS Code's explorer in the
//! fluent themes (soft rounded rows, chevrons, the accent's mark on the
//! selected one, counts in pills), Delphi's project manager in the classic
//! one (± boxes, a full-row highlight) — every colour from the theme
//! (`common::look`).

use rapidr_value::objects::font::Font;
use rapidr_value::objects::ops::{Place, Rect};
use rapidr_value::objects::text::text_size;
use rapidr_value::panels::project_tree::{model, with, with_mut, DropAt, Node, NodeKind, ProjectTree as Model};
use rapidr_value::panels::rows::ROW;

use super::super::common::{self, look, mix, Look};
use super::{geo, row_x, strip_buttons, Geo, ProjectTree};
use crate::components::list;
use crate::components::Cx;
use crate::paint::Painter;

fn bold(font: &Font) -> Font {
    Font { styles: font.styles | 1, ..font.clone() }
}

fn smaller(font: &Font) -> Font {
    Font { size: (font.size - 1).max(6), ..font.clone() }
}

/// A node's icon (rapidr_icons: the project's, a kind's, a folder's, a
/// file's by extension — a form's own —, a component type's).
pub(crate) fn icon_of(m: &Model, n: &Node) -> &'static str {
    let kind = |k: &str| rapidr_icons::project_kind(k).map_or("files/file", |i| i.id);
    match &n.kind {
        NodeKind::Project => kind("project"),
        NodeKind::Group(k) => kind(k.as_str()),
        NodeKind::Folder(..) => kind(if n.expanded { "folder-open" } else { "folder" }),
        NodeKind::File(p) => match m.file(p).map(|f| f.kind.as_str()) {
            Some("form") => kind("form"),
            _ => rapidr_icons::file(p).id,
        },
        NodeKind::Component { ty, .. } => rapidr_icons::component(ty).map_or_else(|| kind("component"), |i| i.id),
    }
}

pub(super) fn paint(cx: &mut Cx, p: &mut Painter) {
    let l = look(p.theme());
    let (w, h) = (cx.width(), cx.height());
    common::ground(p, w, h, &l);
    let font = cx.font.clone();
    let Some(m) = with(cx.id, Clone::clone) else { return };
    let g = geo(cx, &l, m.confirm.is_some());
    if !m.loaded {
        // (EmptyText: what to do, the program's — "No project" else)
        let empty = crate::store::string(cx.store, cx.id, "emptytext");
        let text = if empty.trim().is_empty() { "No project".to_string() } else { empty };
        common::wrapped_center(p, (g.area.0 + 12, g.area.1 + 12, (g.area.2 - 24).max(0), g.area.3), &text, &font, l.dim);
        return;
    }
    let rows = m.rows();
    let content = rows.len() as i64 * ROW + 2 * g.pad;
    list::vscroll(cx.id, g.area.2, g.area.3, content, ROW);
    if m.reveal {
        if let Some(i) = rows.iter().position(|r| r.key == m.selected) {
            let top = g.pad + i as i64 * ROW;
            list::scroll_into_view(cx.id, top - g.pad, top + ROW + g.pad, g.area.3);
        }
        with_mut(cx.id, |m| m.reveal = false);
    }
    let (pos, cw, bar) = list::vscroll(cx.id, g.area.2, g.area.3, content, ROW);
    let focused = cx.state.focused;
    let enabled = cx.state.enabled;
    let drag = m.drag.as_ref().filter(|d| d.active);
    let editing = list::editing(cx.id).and(m.editing.clone());
    let area = (g.area.0, g.area.1, cw, g.area.3);
    p.clipped(area, |p| {
        for (i, n) in rows.iter().enumerate() {
            let y = g.area.1 + g.pad + i as i64 * ROW - pos;
            if y + ROW <= g.area.1 || y >= g.area.1 + g.area.3 {
                continue;
            }
            let r = (g.area.0, y, cw, ROW);
            let selected = n.key == m.selected;
            let hover = drag.is_none() && m.hover.as_deref() == Some(n.key.as_str());
            let into = drag.is_some_and(|d| match (&d.target, &n.kind) {
                (Some(DropAt::Into(f)), NodeKind::Group(k)) => f.is_empty() && m.file(&d.path).is_some_and(|x| x.kind == *k),
                (Some(DropAt::Into(f)), NodeKind::Folder(k, path)) => f == path && m.file(&d.path).is_some_and(|x| x.kind == *k),
                _ => false,
            });
            let ink = if into {
                drop_into(p, r, &l);
                l.text
            } else {
                common::row(p, r, &l, selected, focused, hover)
            };
            let ink = if enabled { ink } else { l.disabled };
            row(p, &g, &l, &m, n, r, ink, selected && focused, &font, editing.as_deref() == Some(n.key.as_str()));
            if l.classic && selected && focused && editing.is_none() && m.confirm.is_none() {
                common::row_focus(p, r, &l);
            }
        }
        // (the tree has the keyboard and nothing is selected: the first row's ring)
        if focused && m.selected.is_empty() && !rows.is_empty() && m.confirm.is_none() {
            common::row_focus(p, (g.area.0, g.area.1 + g.pad - pos, cw, ROW), &l);
        }
        if let Some(d) = drag {
            if let Some(at) = &d.target {
                insertion(p, &g, &l, &rows, at, pos, cw);
            }
        }
    });
    p.at((g.area.0, g.area.1), |p| p.ops(bar));
    // (the name's editor over its row, what's wrong with it under it)
    if let Some(r) = ProjectTree::edit_rect(cx, &g) {
        p.clipped(area, |p| {
            if l.classic {
                p.fill(r, l.field);
                p.frame(r, l.text);
            } else {
                p.round(r, 3.0, Some(l.field), Some(l.field_focus), if l.contrast { 2.0 } else { 1.5 });
            }
        });
        p.clipped(area, |p| crate::components::edit::paint_line(cx, p, list::editor_area(r), crate::components::edit::Source::InPlace));
        if let Some(why) = &m.rename_error {
            error_tip(p, &l, r, area, why, &font);
        }
    }
    if let (Some(c), Some(strip)) = (&m.confirm, g.strip) {
        confirm_strip(p, &l, strip, c, focused, &font);
    }
    if let Some(d) = drag {
        ghost(p, &l, &m, &d.path, d.at, (0, 0, w, h), &font);
    }
}

/// A folder or group a dragged file would go into, lit up.
fn drop_into(p: &mut Painter, r: Rect, l: &Look) {
    let (x, y, w, h) = r;
    if l.classic {
        p.fill(r, l.inactive);
        p.frame(r, l.selected);
    } else {
        p.round((x + 2, y + 1, w - 4, h - 2), 4.0, Some(mix(l.accent, l.body, if l.contrast { 0.0 } else { 0.82 })), Some(l.accent), if l.contrast { 2.0 } else { 1.5 });
    }
}

/// A row's chevron, icon, name and what follows it.
#[allow(clippy::too_many_arguments)]
fn row(p: &mut Painter, g: &Geo, l: &Look, m: &Model, n: &Node, r: Rect, ink: u32, strong: bool, font: &Font, editing: bool) {
    let (x, y, w, h) = r;
    let mid = y + h / 2;
    let (chev, ix, tx) = row_x(g, n.level);
    if n.expandable {
        if l.classic {
            crate::components::tree::expander(p, chev + 9, mid, 9, n.expanded);
        } else {
            p.chevron((chev + 9) as f64 + 0.5, mid as f64 + 0.5, 7.0, n.expanded, if strong { ink } else { l.dim });
        }
    }
    let selected = n.key == m.selected;
    common::icon(p, icon_of(m, n), ix, y + (h - 16) / 2, 16, selected.then_some(ink), false);
    if editing {
        return;
    }
    let right = x + w - 8;
    let heading = matches!(n.kind, NodeKind::Project | NodeKind::Group(_)) || n.main;
    let f = if heading { bold(font) } else { font.clone() };
    let dim = if strong || l.contrast { ink } else { l.dim };
    // (what goes at the right: a group's count, the main file's mark)
    let mut room = right - tx;
    if let Some(count) = n.count {
        let small = smaller(font);
        let s = count.to_string();
        let (cw, _) = text_size(&s, &small);
        let pw = (cw + 12).max(22);
        let pill = (right - pw, mid - 8, pw, 16);
        if !l.classic {
            // (high contrast: a framed lozenge — a full circle's edge
            // rasterizes a pixel differently here and there)
            let radius = if l.contrast { 4.0 } else { 8.0 };
            p.round(pill, radius, Some(if selected { mix(ink, l.selected, 0.85) } else { l.section }), if l.contrast { Some(l.border) } else { None }, 1.0);
        }
        p.text(pill, &s, &small, dim, Place::Center);
        room = pill.0 - 6 - tx;
    } else if n.main {
        let small = smaller(font);
        let (mw, _) = text_size("main", &small);
        p.text((right - mw, y, mw, h), "main", &small, dim, Place::Left);
        room = right - mw - 8 - tx;
    }
    match &n.kind {
        NodeKind::Component { name, ty, .. } => {
            let (nw, _) = text_size(name, &f);
            let shown = common::elide(name, &f, room);
            p.text((tx, y, room.max(0), h), &shown, &f, ink, Place::Left);
            if nw + 12 < room {
                let rest = format!(": {ty}");
                let left = room - nw;
                p.text((tx + nw, y, left, h), &common::elide(&rest, font, left), font, dim, Place::Left);
            }
        }
        NodeKind::File(path) if dirty(m, path) => {
            // (changes not saved: a dot after the name, as editors' tabs)
            let shown = common::elide(&n.label, &f, room - 14);
            let (lw, _) = text_size(&shown, &f);
            p.text((tx, y, room.max(0), h), &shown, &f, ink, Place::Left);
            p.round((tx + lw + 6, mid - 3, 6, 6), 3.0, Some(if selected { ink } else { l.accent }), None, 1.0);
        }
        _ => {
            p.text((tx, y, room.max(0), h), &common::elide(&n.label, &f, room), &f, ink, Place::Left);
        }
    }
}

/// Whether file `path` has changes not saved (FileModified).
fn dirty(m: &rapidr_value::panels::project_tree::ProjectTree, path: &str) -> bool {
    m.dirty.contains(&path.replace('\\', "/").to_lowercase())
}

/// Where a dragged file would go among the files: a line in the accent
/// with a ring at its start, at the gap.
fn insertion(p: &mut Painter, g: &Geo, l: &Look, rows: &[Node], at: &DropAt, pos: i64, cw: i64) {
    let (anchor, after) = match at {
        DropAt::Before(a) => (a, false),
        DropAt::After(a) => (a, true),
        DropAt::Into(_) => return,
    };
    let Some(i) = rows.iter().position(|r| r.key == *anchor) else { return };
    // (after a node: after what's open under it)
    let mut j = i;
    if after {
        while j + 1 < rows.len() && rows[j + 1].level > rows[i].level {
            j += 1;
        }
    }
    let y = g.area.1 + g.pad + (if after { j + 1 } else { i }) as i64 * ROW - pos;
    let (_, ix, _) = row_x(g, rows[i].level);
    let right = g.area.0 + cw - 6;
    if l.classic {
        p.fill((ix, y - 1, right - ix, 2), l.text);
        p.fill((ix, y - 3, 2, 6), l.text);
    } else {
        p.fill((ix + 3, y - 1, right - ix - 3, 2), l.accent);
        p.round((ix - 4, y - 4, 8, 8), 4.0, Some(l.body), Some(l.accent), 2.0);
    }
}

/// What's wrong with the name typed, under its editor (above it near the
/// bottom).
fn error_tip(p: &mut Painter, l: &Look, ed: Rect, area: Rect, why: &str, font: &Font) {
    let (tw, _) = text_size(why, font);
    let w = (tw + 16).min(area.0 + area.2 - ed.0 - 2).max(40);
    let h = 22;
    let below = ed.1 + ed.3 + 2;
    let y = if below + h <= area.1 + area.3 { below } else { ed.1 - h - 2 };
    let r = (ed.0, y, w, h);
    if l.classic {
        p.fill(r, l.body);
        p.frame(r, l.error);
    } else {
        p.round(r, 4.0, Some(if l.contrast { l.body } else { mix(l.error, l.body, 0.88) }), Some(l.error), if l.contrast { 2.0 } else { 1.0 });
    }
    p.text((r.0 + 8, y, w - 12, h), &common::elide(why, font, w - 12), font, l.text, Place::Left);
}

/// The strip asking whether a file goes: a warning, the question, [Cancel]
/// [Remove] (Remove the default; the one with the keyboard ringed).
fn confirm_strip(p: &mut Painter, l: &Look, strip: Rect, c: &rapidr_value::panels::project_tree::Confirm, focused: bool, font: &Font) {
    let (x, y, w, h) = strip;
    p.fill(strip, l.chrome);
    if l.classic {
        p.fill((x, y, w, 1), l.border);
        p.fill((x, y + 1, w, 1), l.body);
    } else {
        common::hline(p, x, y, w, l.line);
    }
    common::icon(p, "warning", x + 10, y + 11, 16, None, false);
    let name = model::name_of(&c.path);
    let text = format!("Remove {name} from the project?");
    let start = "Remove ".chars().count();
    let marks: Vec<usize> = (start..start + name.chars().count()).collect();
    common::marked_text(p, (x + 32, y + 6, w - 42, 26), &text, &marks, font, l.text, l.text);
    let _ = h;
    for (b, rect) in strip_buttons(strip, font).into_iter().enumerate() {
        let title = if b == 0 { "Remove" } else { "Cancel" };
        let (pressed, hover, has) = (c.pressed == Some(b) && c.hover == Some(b), c.hover == Some(b), focused && c.focus == b);
        button(p, l, rect, title, b == 0, pressed, hover, has, font);
    }
}

#[allow(clippy::too_many_arguments)]
fn button(p: &mut Painter, l: &Look, r: Rect, title: &str, primary: bool, pressed: bool, hover: bool, focus: bool, font: &Font) {
    let (x, y, w, h) = r;
    if l.classic {
        p.fill(r, l.chrome);
        if pressed {
            p.frame(r, l.text);
            p.frame((x + 1, y + 1, w - 2, h - 2), l.border);
        } else if focus || primary {
            // (the default button's dark frame)
            p.frame(r, l.text);
            p.button_edge((x + 1, y + 1, w - 2, h - 2));
        } else {
            p.button_edge(r);
        }
        let shift = i64::from(pressed);
        p.text((x + shift, y + shift, w, h), title, font, l.text, Place::Center);
        if focus {
            p.focus((x + 4, y + 4, w - 8, h - 8));
        }
        return;
    }
    let (fill, stroke, ink) = if primary {
        let base = l.accent;
        let fill = if pressed { mix(base, l.text, 0.18) } else if hover && !l.contrast { mix(base, l.body, 0.12) } else { base };
        (fill, None, l.accent_text)
    } else {
        let fill = if pressed { mix(l.hover, l.text, 0.08) } else if hover { l.hover } else { l.field };
        (fill, Some(l.field_border), l.text)
    };
    p.round(r, 5.0, Some(fill), stroke, 1.0);
    let f = if primary { bold(font) } else { font.clone() };
    p.text(r, title, &f, ink, Place::Center);
    if focus {
        p.ring((x - 2, y - 2, w + 4, h + 4), 7.0, l.focus, if l.contrast { 2.0 } else { 1.5 });
    }
}

/// What's dragged, beside the pointer.
fn ghost(p: &mut Painter, l: &Look, m: &Model, path: &str, at: (i64, i64), clip: Rect, font: &Font) {
    let name = model::name_of(path);
    let (tw, _) = text_size(name, font);
    let (w, h) = (tw + 36, ROW);
    let (x, y) = (at.0 + 12, at.1 + 4);
    let node = Node { kind: NodeKind::File(path.to_string()), key: path.to_string(), label: name.to_string(), level: 0, expandable: false, expanded: false, main: false, count: None, parent: None };
    p.clipped(clip, |p| {
        if l.classic {
            p.fill((x, y, w, h), l.field);
            p.frame((x, y, w, h), l.text);
        } else {
            p.round((x + 1, y + 2, w, h), 5.0, Some(mix(l.border, l.body, 0.5)), None, 1.0);
            p.round((x, y, w, h), 5.0, Some(l.field), Some(if l.contrast { l.border } else { l.accent }), 1.0);
        }
        common::icon(p, icon_of(m, &node), x + 6, y + (h - 16) / 2, 16, None, false);
        p.text((x + 26, y, w - 28, h), name, font, l.text, Place::Left);
    });
}

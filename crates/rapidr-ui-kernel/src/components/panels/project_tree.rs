//! RPROJECTTREE drawn and driven by the kernel; its model is
//! `rapidr_value::panels::project_tree` (the web draws the same).
//!
//! The rows ([`ProjectTree::rows`]): the project, its groups by kind with
//! their counts, folders, files (the main one in bold), each form's
//! components; a chevron opens a node (classic: Windows' ± box), each has
//! its icon. A click selects (OnSelect), a double click or Enter opens
//! (OnOpen), F2 renames in place (the kernel's line editor over the name,
//! the name without its extension selected; a name that can't be is said
//! under it), Delete asks with a strip at the bottom ([Remove] [Cancel])
//! before OnDelete, a file dragged more than 5 pixels shows where it would
//! go — a line between files, or a folder or group lit up — and goes there
//! when let go (OnMove); Alt+Up / Alt+Down move it with the keyboard.

mod draw;

use std::cell::RefCell;
use std::collections::HashMap;

use rapidr_value::input::{Button, Cursor};
use rapidr_value::objects::a11y::{part_id, AccessNode, Action, Role, PART_ITEM};
use rapidr_value::objects::ops::Rect;
use rapidr_value::panels::project_tree::{model, with, with_mut, Confirm, Drag, DropAt, Node, NodeKind, ProjectTree as Model, User};
use rapidr_value::panels::rows::{self, vk, ROW};
use rapidr_value::panels::User as PanelUser;

use super::common::{self, look, Look};
use crate::a11y::AccessValue;
use crate::components::form::Container;
use crate::components::list::{self, InPlace};
use crate::components::{ComponentKind, Cx, Ime, KeyIn, MouseIn, MouseKind, MouseOut};
use crate::input::{Clipboard, KernelEvent};
use crate::paint::Painter;
use crate::store::Store;

pub struct ProjectTree;

/// How far the mouse goes before a press on a file is a drag.
const DRAG_START: i64 = 5;
/// The delete confirmation strip's height.
const STRIP: i64 = 66;
/// Accessibility parts past the rows: the strip, its buttons, the editor.
const PART_STRIP: usize = 1_000_000;
const PART_REMOVE: usize = 1_000_001;
const PART_CANCEL: usize = 1_000_002;
const PART_EDITOR: usize = 1_000_010;
/// A type-ahead's letters run together within this long.
const TYPE_AHEAD: std::time::Duration = std::time::Duration::from_millis(1000);

thread_local! {
    /// Each tree's type-ahead: what was typed and when.
    static TYPED: RefCell<HashMap<String, (String, crate::tick::Instant)>> = RefCell::new(HashMap::new());
}

fn send(cx: &mut Cx, action: User) {
    super::send(cx, PanelUser::ProjectTree(action));
}

/// Where things are, in the component's pixels.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Geo {
    /// The rows' area (the scroll bar inside it, at its right).
    pub area: Rect,
    /// Room above the first row and below the last.
    pub pad: i64,
    /// The delete confirmation strip, when it shows.
    pub strip: Option<Rect>,
}

fn geo(cx: &Cx, l: &Look, confirming: bool) -> Geo {
    let i = common::inset(l);
    let (w, h) = (cx.width(), cx.height());
    let strip_h = if confirming { STRIP.min((h - 2 * i).max(0)) } else { 0 };
    let area = (i, i, (w - 2 * i).max(0), (h - 2 * i - strip_h).max(0));
    let strip = confirming.then_some((i, i + area.3, area.2, strip_h));
    Geo { area, pad: if l.classic { 2 } else { 4 }, strip }
}

/// The model's rows and the scroll position (pixels).
fn rows_now(cx: &Cx) -> Vec<Node> {
    with(cx.id, Model::rows).unwrap_or_default()
}

fn scrolled(cx: &Cx) -> i64 {
    list::vscroll_state(cx.id).0
}

/// Row `i`'s rectangle in the component (as scrolled), the scroll bar's
/// room left out.
fn row_rect(cx: &Cx, g: &Geo, i: usize) -> Rect {
    let (_, bar, _) = list::vscroll_state(cx.id);
    let w = g.area.2 - if bar { rapidr_value::scrollbars::BAR } else { 0 };
    (g.area.0, g.area.1 + g.pad + i as i64 * ROW - scrolled(cx), w, ROW)
}

/// Where a row's chevron, icon and text start.
pub(crate) fn row_x(g: &Geo, level: usize) -> (i64, i64, i64) {
    let base = g.area.0 + 4 + level as i64 * rows::INDENT;
    (base, base + 18, base + 18 + 16 + 6)
}

/// The row at (x, y) of the component.
fn row_at(cx: &Cx, g: &Geo, count: usize, y: i64) -> Option<usize> {
    if y < g.area.1 || y >= g.area.1 + g.area.3 {
        return None;
    }
    let at = y - g.area.1 - g.pad + scrolled(cx);
    (at >= 0).then_some((at / ROW) as usize).filter(|&i| i < count)
}

/// The strip's buttons: Remove, Cancel.
pub(crate) fn strip_buttons(strip: Rect, font: &rapidr_value::objects::font::Font) -> [Rect; 2] {
    let (x, y, w, h) = strip;
    let bw = |t: &str| (rapidr_value::objects::text::text_size(t, font).0 + 28).max(72);
    let (rw, cw) = (bw("Remove"), bw("Cancel"));
    let by = y + h - 10 - 24;
    let remove = (x + w - 10 - rw, by, rw, 24);
    let cancel = (remove.0 - 8 - cw, by, cw, 24);
    [remove, cancel]
}

impl ProjectTree {
    /// The selection moved to row key `key` (the program hears it).
    fn select(cx: &mut Cx, key: &str) {
        let changed = with_mut(cx.id, |m| {
            m.reveal = true;
            if m.selected == key {
                return false;
            }
            m.selected = key.to_string();
            true
        });
        if changed {
            send(cx, User::Select(key.to_string()));
        }
    }

    /// Node `n` opened by the user: a file or component is opened (OnOpen),
    /// the others open or close.
    fn open(cx: &mut Cx, n: &Node) {
        match n.kind {
            NodeKind::File(_) | NodeKind::Component { .. } => send(cx, User::Open(n.key.clone())),
            _ if n.expandable => with_mut(cx.id, |m| m.set_open(&n.key, !n.expanded)),
            _ => {}
        }
    }

    /// The editor opened for a file NewFile added or Rename asked for.
    fn sync_edit(cx: &mut Cx) {
        let Some(path) = with_mut(cx.id, |m| m.edit_request.take()) else { return };
        Self::begin_rename(cx, &path);
    }

    /// File `path`'s name edited in place (its stem selected).
    fn begin_rename(cx: &mut Cx, path: &str) {
        let name = model::name_of(path).to_string();
        let stem = match name.rfind('.') {
            Some(i) if i > 0 => name[..i].chars().count(),
            _ => name.chars().count(),
        };
        with_mut(cx.id, |m| {
            m.editing = Some(path.to_string());
            m.rename_error = None;
            m.confirm = None;
            m.open_above(path);
            m.selected = path.to_string();
            m.reveal = true;
        });
        list::begin_edit_selecting(cx.id, InPlace { target: (0, 0), text: name, rect: None }, (0, stem));
    }

    /// The edit's box in the component (over the name, to the rows' right)
    /// while its row shows.
    fn edit_rect(cx: &Cx, g: &Geo) -> Option<Rect> {
        list::editing(cx.id)?;
        let path = with(cx.id, |m| m.editing.clone()).flatten()?;
        let rows = rows_now(cx);
        let i = rows.iter().position(|r| r.key == path)?;
        let (x, y, w, _) = row_rect(cx, g, i);
        let (_, _, tx) = row_x(g, rows[i].level);
        Some((tx - 4, y + 1, (x + w - 4 - (tx - 4)).max(48), ROW - 2))
    }

    /// The edit ends: kept (Enter, the focus leaving) or dropped (Escape).
    /// Whether it ended (a name that can't be keeps it open, saying why).
    fn finish_edit(cx: &mut Cx, keep: bool) -> bool {
        let Some(ed) = list::editing(cx.id) else { return true };
        let Some(path) = with(cx.id, |m| m.editing.clone()).flatten() else {
            list::end_edit(cx.id);
            return true;
        };
        if keep {
            if let Some(Err(why)) = with(cx.id, |m| m.validate(&path, &ed.text)) {
                with_mut(cx.id, |m| m.rename_error = Some(why));
                return false;
            }
        }
        list::end_edit(cx.id);
        cx.ui.edit = None;
        let events = ended(cx.id, &path, keep.then_some(ed.text.as_str()));
        cx.events.extend(events);
        true
    }

    /// The confirmation strip's button `b` (0 Remove, 1 Cancel) pressed.
    fn confirm_button(cx: &mut Cx, b: usize) {
        let Some(c) = with_mut(cx.id, |m| m.confirm.take()) else { return };
        if b == 0 {
            send(cx, User::Delete(c.path));
        }
    }

    /// Asks to delete the selected file (the strip).
    fn ask_delete(cx: &mut Cx) {
        with_mut(cx.id, |m| {
            if let Some(f) = m.file(&m.selected.clone()).map(|f| f.path.clone()) {
                m.confirm = Some(Confirm { path: f, ..Confirm::default() });
                m.reveal = true;
            }
        });
    }

    /// Where a file dragged to (x, y) would go.
    fn drop_at(cx: &Cx, g: &Geo, path: &str, x: i64, y: i64) -> Option<DropAt> {
        let _ = x;
        let rows = rows_now(cx);
        let i = row_at(cx, g, rows.len(), y)?;
        let kind = with(cx.id, |m| m.file(path).map(|f| f.kind)).flatten()?;
        let (_, ry, _, rh) = row_rect(cx, g, i);
        let at = match &rows[i].kind {
            NodeKind::Group(k) if *k == kind => DropAt::Into(String::new()),
            NodeKind::Folder(k, p) if *k == kind => DropAt::Into(p.clone()),
            NodeKind::File(p) if p != path => {
                if y < ry + rh / 2 {
                    DropAt::Before(p.clone())
                } else {
                    DropAt::After(p.clone())
                }
            }
            _ => return None,
        };
        with(cx.id, |m| m.drop_result(path, &at).is_some()).unwrap_or(false).then_some(at)
    }

    /// A press on row `i` (not its chevron).
    fn press_row(cx: &mut Cx, rows: &[Node], i: usize, at: (i64, i64), double: bool) {
        let n = &rows[i];
        Self::select(cx, &n.key);
        if double {
            Self::open(cx, n);
            return;
        }
        if let NodeKind::File(p) = &n.kind {
            with_mut(cx.id, |m| m.drag = Some(Drag { path: p.clone(), from: at, at, active: false, target: None }));
        }
    }

    fn key_rows(cx: &mut Cx, k: &KeyIn) -> bool {
        let rows = rows_now(cx);
        let Some((selected, confirming)) = with(cx.id, |m| (m.selected.clone(), m.confirm.is_some())) else { return false };
        if confirming {
            return Self::key_strip(cx, k);
        }
        if k.mods.ctrl || k.mods.command {
            return false;
        }
        let at = rows.iter().position(|r| r.key == selected);
        let l = look(rapidr_value::theme::current());
        let g = geo(cx, &l, false);
        let page = (g.area.3 / ROW).max(1) as usize;
        if k.mods.alt {
            // (Alt+Up / Alt+Down: the file moved among its folder's)
            let down = match k.vk {
                vk::UP => false,
                vk::DOWN => true,
                _ => return false,
            };
            if let Some((new, index)) = with(cx.id, |m| m.step(&selected, down)).flatten() {
                send(cx, User::Move(selected, new, index));
            }
            return true;
        }
        match k.vk {
            vk::UP | vk::DOWN | vk::HOME | vk::END | vk::PAGE_UP | vk::PAGE_DOWN => {
                if let Some(to) = rows::nav(k.vk, at, rows.len(), page) {
                    Self::select(cx, &rows[to].key);
                }
            }
            vk::RIGHT => {
                if let Some(i) = at {
                    let n = &rows[i];
                    if n.expandable && !n.expanded {
                        with_mut(cx.id, |m| m.set_open(&n.key, true));
                    } else if n.expandable && i + 1 < rows.len() {
                        Self::select(cx, &rows[i + 1].key);
                    }
                }
            }
            vk::LEFT => {
                if let Some(i) = at {
                    let n = &rows[i];
                    if n.expandable && n.expanded {
                        with_mut(cx.id, |m| m.set_open(&n.key, false));
                    } else if let Some(p) = n.parent {
                        Self::select(cx, &rows[p].key);
                    }
                }
            }
            vk::ENTER => {
                if let Some(i) = at {
                    Self::open(cx, &rows[i]);
                }
            }
            vk::F2 => {
                if let Some(NodeKind::File(p)) = at.map(|i| rows[i].kind.clone()) {
                    Self::begin_rename(cx, &p);
                }
            }
            vk::DELETE => Self::ask_delete(cx),
            vk::ESCAPE | vk::TAB => return false,
            _ => {
                let t = k.text;
                if t.is_empty() || t.chars().any(char::is_control) {
                    return false;
                }
                let now = crate::tick::now();
                let typed = TYPED.with(|m| {
                    let mut m = m.borrow_mut();
                    let e = m.entry(cx.id.to_string()).or_insert_with(|| (String::new(), now));
                    if now.duration_since(e.1) > TYPE_AHEAD {
                        e.0.clear();
                    }
                    e.0.push_str(t);
                    e.1 = now;
                    e.0.clone()
                });
                // (one letter: the next row starting with it; more: the
                // row they start, from here)
                let from = match at {
                    Some(i) if typed.chars().count() == 1 => i + 1,
                    Some(i) => i,
                    None => 0,
                };
                if let Some(to) = rows::type_ahead(rows.iter().map(|r| r.label.as_str()), from % rows.len().max(1), &typed) {
                    Self::select(cx, &rows[to].key);
                }
            }
        }
        true
    }

    /// The keyboard while the strip asks: Left / Right / Tab between the
    /// buttons, Enter or Space presses one, Escape cancels.
    fn key_strip(cx: &mut Cx, k: &KeyIn) -> bool {
        let focus = with(cx.id, |m| m.confirm.as_ref().map(|c| c.focus)).flatten().unwrap_or(0);
        match k.vk {
            vk::LEFT | vk::RIGHT | vk::TAB => with_mut(cx.id, |m| {
                if let Some(c) = m.confirm.as_mut() {
                    c.focus = 1 - c.focus;
                }
            }),
            vk::ENTER | vk::SPACE => Self::confirm_button(cx, focus),
            vk::ESCAPE => Self::confirm_button(cx, 1),
            _ => {}
        }
        true
    }

    fn mouse_strip(cx: &mut Cx, m: &MouseIn, strip: Rect) -> bool {
        let (x, y) = (m.x.floor() as i64, m.y.floor() as i64);
        let buttons = strip_buttons(strip, &cx.font);
        let on = buttons.iter().position(|b| common::inside(*b, x, y));
        match m.kind {
            MouseKind::Down => {
                if !common::inside(strip, x, y) {
                    return false;
                }
                with_mut(cx.id, |md| {
                    if let Some(c) = md.confirm.as_mut() {
                        c.pressed = on;
                        if let Some(b) = on {
                            c.focus = b;
                        }
                    }
                });
                true
            }
            MouseKind::Up => {
                let pressed = with_mut(cx.id, |md| md.confirm.as_mut().and_then(|c| c.pressed.take()));
                if pressed.is_some() && pressed == on {
                    Self::confirm_button(cx, on.unwrap_or(1));
                }
                pressed.is_some() || common::inside(strip, x, y)
            }
            MouseKind::Move => {
                let changed = with_mut(cx.id, |md| {
                    md.confirm.as_mut().is_some_and(|c| {
                        let h = if common::inside(strip, x, y) { on } else { None };
                        std::mem::replace(&mut c.hover, h) != h
                    })
                });
                if changed {
                    cx.ui.wake = Some(crate::tick::now());
                }
                common::inside(strip, x, y)
            }
            MouseKind::Leave => {
                with_mut(cx.id, |md| {
                    if let Some(c) = md.confirm.as_mut() {
                        c.hover = None;
                    }
                });
                false
            }
        }
    }
}

/// What a name's edit ending does: OnRename for a file renamed (`typed`
/// its new name; `None`: dropped), OnNewFile for one NewFile added.
fn ended(id: &str, path: &str, typed: Option<&str>) -> Vec<KernelEvent> {
    let (new_file, new) = with_mut(id, |m| {
        m.editing = None;
        m.rename_error = None;
        let new = typed.and_then(|t| m.validate(path, t).ok()).unwrap_or_else(|| path.to_string());
        (m.new_file.as_deref() == Some(path), new)
    });
    let action = if new_file {
        User::Named(path.to_string(), new)
    } else if new != path {
        User::Rename(path.to_string(), new)
    } else {
        return Vec::new();
    };
    vec![KernelEvent::Container(Container::Panel { id: id.to_string(), action: PanelUser::ProjectTree(action) })]
}

impl ComponentKind for ProjectTree {
    fn name(&self) -> &'static str {
        "RPROJECTTREE"
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        Self::sync_edit(cx);
        list::drop_editor(cx);
        draw::paint(cx, p);
    }

    fn wheel(&self, cx: &mut Cx, _dx: f64, dy: f64, _mods: crate::input::Mods) -> bool {
        let l = look(rapidr_value::theme::current());
        let confirming = with(cx.id, |m| m.confirm.is_some()).unwrap_or(false);
        let g = geo(cx, &l, confirming);
        list::vscroll_wheel(cx.id, dy, g.area.2, g.area.3)
    }

    /// A file being dragged onto a folder: the closed hand; a name being
    /// edited: the I-beam.
    fn pointer(&self, cx: &mut Cx, x: i64, y: i64) -> Cursor {
        if with(cx.id, |md| md.drag.as_ref().is_some_and(|d| d.active)) == Some(true) {
            return Cursor::Grabbing;
        }
        let Some(confirming) = with(cx.id, |md| md.confirm.is_some()) else { return Cursor::Default };
        let g = geo(cx, &look(rapidr_value::theme::current()), confirming);
        Self::edit_rect(cx, &g).map_or(Cursor::Default, |r| list::editor_pointer(cx, r, x, y))
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        Self::sync_edit(cx);
        let l = look(rapidr_value::theme::current());
        let Some(confirming) = with(cx.id, |md| md.confirm.is_some()) else { return MouseOut::default() };
        let g = geo(cx, &l, confirming);
        // (the strip's buttons)
        if let Some(strip) = g.strip {
            if Self::mouse_strip(cx, m, strip) {
                return MouseOut::default();
            }
        }
        // (the scroll bar, in the rows' area)
        let inner = MouseIn { x: m.x - g.area.0 as f64, y: m.y - g.area.1 as f64, ..*m };
        if with(cx.id, |md| md.drag.as_ref().is_none_or(|d| !d.active)).unwrap_or(true) && list::bar_mouse(cx, &inner, g.area.2, g.area.3) {
            return MouseOut::default();
        }
        // (the name's editor)
        if let Some(r) = Self::edit_rect(cx, &g) {
            if list::editor_mouse(cx, m, r) {
                return MouseOut::default();
            }
        }
        let (x, y) = (m.x.floor() as i64, m.y.floor() as i64);
        let rows = rows_now(cx);
        match m.kind {
            MouseKind::Down if m.button == Button::Left => {
                // (a press elsewhere ends an edit, keeping it if it can be)
                if list::editing(cx.id).is_some() && !Self::finish_edit(cx, true) {
                    Self::finish_edit(cx, false);
                }
                let Some(i) = row_at(cx, &g, rows.len(), y) else { return MouseOut::default() };
                let n = &rows[i];
                let (chev, _, _) = row_x(&g, n.level);
                if n.expandable && x >= chev && x < chev + 18 && !m.double() {
                    with_mut(cx.id, |md| md.set_open(&n.key, !n.expanded));
                } else {
                    Self::press_row(cx, &rows, i, (x, y), m.double());
                }
            }
            MouseKind::Down => {}
            MouseKind::Move if m.captured => {
                let Some(d) = with(cx.id, |md| md.drag.clone()).flatten() else { return MouseOut::default() };
                let active = d.active || (x - d.from.0).abs() > DRAG_START || (y - d.from.1).abs() > DRAG_START;
                let target = if active { Self::drop_at(cx, &g, &d.path, x, y) } else { None };
                with_mut(cx.id, |md| {
                    md.hover = None;
                    md.drag = Some(Drag { at: (x, y), active, target, ..d });
                });
                // (near the top or bottom while dragging: the rows scroll)
                if active {
                    let edge = ROW / 2;
                    if y < g.area.1 + edge {
                        list::vscroll_wheel(cx.id, -1.0 / 3.0, g.area.2, g.area.3);
                    } else if y > g.area.1 + g.area.3 - edge {
                        list::vscroll_wheel(cx.id, 1.0 / 3.0, g.area.2, g.area.3);
                    }
                }
            }
            MouseKind::Move => {
                let h = row_at(cx, &g, rows.len(), y).map(|i| rows[i].key.clone());
                if with_mut(cx.id, |md| std::mem::replace(&mut md.hover, h.clone()) != h) {
                    cx.ui.wake = Some(crate::tick::now());
                }
            }
            MouseKind::Up => {
                if let Some(d) = with_mut(cx.id, |md| md.drag.take()) {
                    if let (true, Some(at)) = (d.active, d.target) {
                        if let Some((new, index)) = with(cx.id, |md| md.drop_result(&d.path, &at)).flatten() {
                            send(cx, User::Move(d.path, new, index));
                        }
                    }
                }
            }
            MouseKind::Leave => {
                if with_mut(cx.id, |md| md.hover.take().is_some()) {
                    cx.ui.wake = Some(crate::tick::now());
                }
            }
        }
        MouseOut::default()
    }

    fn key(&self, cx: &mut Cx, k: &KeyIn, clip: &mut dyn Clipboard) -> bool {
        Self::sync_edit(cx);
        if list::editing(cx.id).is_some() {
            let l = look(rapidr_value::theme::current());
            let g = geo(cx, &l, false);
            let r = Self::edit_rect(cx, &g).unwrap_or((g.area.0, g.area.1, g.area.2, ROW));
            if let Some(end) = list::edit_key(cx, k, clip, r) {
                match end {
                    Some(keep) => {
                        Self::finish_edit(cx, keep);
                    }
                    // (typing: what was wrong is said again on Enter)
                    None => with_mut(cx.id, |m| m.rename_error = None),
                }
                return true;
            }
        }
        Self::key_rows(cx, k)
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        Self::sync_edit(cx);
        let mut n = crate::components::shared_describe(cx, self.name());
        n.role = Role::Tree;
        let Some(m) = with(cx.id, Clone::clone) else {
            if n.name.is_empty() {
                n.name = "Project".into();
            }
            return n;
        };
        if n.name.is_empty() {
            n.name = if m.loaded { format!("Project {}", m.project.name) } else { "Project".into() };
        }
        let l = look(rapidr_value::theme::current());
        let g = geo(cx, &l, m.confirm.is_some());
        let (ox, oy) = (cx.rect.0, cx.rect.1);
        let focused = cx.state.focused;
        let editing = list::editing(cx.id);
        let rows = m.rows();
        for (i, r) in rows.iter().enumerate() {
            let mut item = AccessNode::new(part_id(cx.id, PART_ITEM, i), Role::TreeItem);
            item.name = match &r.kind {
                NodeKind::Component { name, ty, .. } => format!("{name}, {ty}"),
                _ => r.label.clone(),
            };
            item.description = match &r.kind {
                NodeKind::Project => "project".into(),
                NodeKind::Group(_) => format!("{} file{}", r.count.unwrap_or(0), if r.count == Some(1) { "" } else { "s" }),
                NodeKind::Folder(..) => "folder".into(),
                NodeKind::File(p) => m.file(p).map(|f| if r.main { format!("{}, the main file", f.kind.as_str()) } else { f.kind.as_str().to_string() }).unwrap_or_default(),
                NodeKind::Component { .. } => "component".into(),
            };
            item.level = Some(r.level + 1);
            item.states.selected = Some(r.key == m.selected);
            item.states.focused = r.key == m.selected && m.confirm.is_none() && editing.is_none();
            if r.expandable {
                item.states.expanded = Some(r.expanded);
                item.actions.push(if r.expanded { Action::Collapse } else { Action::Expand });
            }
            item.actions.extend([Action::Click, Action::Focus, Action::ScrollIntoView]);
            let (x, y, w, h) = row_rect(cx, &g, i);
            item.bounds = (ox + x, oy + y, w, h);
            n.children.push(item);
        }
        if let (Some(ed), Some(r)) = (&editing, Self::edit_rect(cx, &g)) {
            let mut e = AccessNode::new(part_id(cx.id, PART_ITEM, PART_EDITOR), Role::TextInput);
            e.name = "File name".into();
            e.value = Some(ed.text.clone());
            e.description = m.rename_error.clone().unwrap_or_default();
            e.states.focused = focused;
            e.actions = vec![Action::SetValue, Action::Focus];
            e.bounds = (ox + r.0, oy + r.1, r.2, r.3);
            n.children.push(e);
        }
        if let (Some(c), Some(strip)) = (&m.confirm, g.strip) {
            let mut s = AccessNode::new(part_id(cx.id, PART_ITEM, PART_STRIP), Role::Group);
            s.name = format!("Remove {} from the project?", model::name_of(&c.path));
            s.bounds = (ox + strip.0, oy + strip.1, strip.2, strip.3);
            for (b, (rect, (title, part))) in strip_buttons(strip, &cx.font).into_iter().zip([("Remove", PART_REMOVE), ("Cancel", PART_CANCEL)]).enumerate() {
                let mut btn = AccessNode::new(part_id(cx.id, PART_ITEM, part), Role::Button);
                btn.name = title.into();
                btn.states.focused = focused && c.focus == b;
                btn.actions = vec![Action::Click, Action::Focus];
                btn.bounds = (ox + rect.0, oy + rect.1, rect.2, rect.3);
                s.children.push(btn);
            }
            n.children.push(s);
        }
        n
    }

    fn access(&self, cx: &mut Cx, action: Action, part: Option<usize>, value: Option<&AccessValue>) -> bool {
        let Some(part) = part else { return false };
        match (part, action) {
            (PART_REMOVE, Action::Click) => Self::confirm_button(cx, 0),
            (PART_CANCEL, Action::Click) => Self::confirm_button(cx, 1),
            (PART_REMOVE | PART_CANCEL, Action::Focus) => with_mut(cx.id, |m| {
                if let Some(c) = m.confirm.as_mut() {
                    c.focus = usize::from(part == PART_CANCEL);
                }
            }),
            (PART_EDITOR, Action::SetValue) => match value {
                Some(AccessValue::Text(t)) => list::set_edit_text(cx.id, t),
                _ => return false,
            },
            (PART_EDITOR, Action::Focus) => {}
            (i, a) if i < PART_STRIP => {
                let rows = rows_now(cx);
                let Some(n) = rows.get(i).cloned() else { return false };
                match a {
                    Action::Click | Action::Focus => Self::select(cx, &n.key),
                    Action::Expand | Action::Collapse if n.expandable => with_mut(cx.id, |m| m.set_open(&n.key, a == Action::Expand)),
                    Action::ScrollIntoView => with_mut(cx.id, |m| m.reveal = true),
                    _ => return false,
                }
            }
            _ => return false,
        }
        true
    }

    fn tick(&self, cx: &mut Cx) {
        list::bar_tick(cx);
    }

    fn ime(&self, cx: &mut Cx, ime: &Ime) -> bool {
        let l = look(rapidr_value::theme::current());
        let g = geo(cx, &l, false);
        Self::edit_rect(cx, &g).is_some_and(|r| list::editor_ime(cx, ime, r))
    }

    fn ime_area(&self, cx: &mut Cx) -> Option<Rect> {
        let l = look(rapidr_value::theme::current());
        let g = geo(cx, &l, false);
        list::editor_ime_area(cx, Self::edit_rect(cx, &g)?)
    }

    fn wants_ime(&self, _store: &dyn Store, id: &str) -> bool {
        list::editing(id).is_some()
    }

    fn context_menu(&self, cx: &mut Cx) -> Option<crate::components::edit::MenuState> {
        let l = look(rapidr_value::theme::current());
        let g = geo(cx, &l, false);
        list::editor_menu(cx, Self::edit_rect(cx, &g)?)
    }

    /// `__edit` (F2), `__enter` ("Renamed" typed, then Enter), `__escape`.
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

/// The focus left it while its name editor was open (`super::focus_left`):
/// the name kept if it can be, else the old one.
pub fn focus_left(id: &str, ed: InPlace) -> Vec<KernelEvent> {
    let Some(path) = with(id, |m| m.editing.clone()).flatten() else { return Vec::new() };
    let ok = with(id, |m| m.validate(&path, &ed.text).is_ok()).unwrap_or(false);
    ended(id, &path, ok.then_some(ed.text.as_str()))
}

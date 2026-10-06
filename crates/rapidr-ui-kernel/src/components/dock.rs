//! RDOCKMANAGER (RapidR Studio's docking; `rapidr_value::dock`, the web's
//! too) drawn and driven by the kernel. Three kinds:
//!
//! - **RDOCKMANAGER**: the ground between the groups, the splitters (a
//!   drag resizes the neighbours), the auto-hide strips (a click slides
//!   the pane out or in), and over everything the open flyout's shadow
//!   and, while a pane is dragged (or moved with the keyboard), the
//!   docking compass and the outline of where it would go.
//! - **RDOCKGROUP**: a group's header — its pane's icon and title, or
//!   tabs — and its buttons (auto-hide, close; a floating one's dock), its
//!   border; a press on a tab or the header shows that pane and makes it
//!   active, a drag takes it to the compass, a double click floats it (a
//!   floating one's docks it again).
//! - **RDOCKDOCS**: the document area: the workspace behind its MDI
//!   windows, or the tabbed documents' tabs (a click shows one, its close
//!   button or a middle click closes it).
//!
//! What the user does goes to runtime-core as [`Container::Dock`], which
//! the shared runtime glue (`rapidr_value::dock::runtime`) carries out
//! with the program's events. The keyboard ([`key`]): F6 / Shift+F6 go
//! from area to area (the groups' shown panes, the active document),
//! Ctrl+Tab / Ctrl+Shift+Tab from document to document, Ctrl+Shift+M
//! moves the active pane with the compass (arrows choose, the same arrow
//! again the outer edge, Tab the next area, Enter docks, F floats, Escape
//! stops), Escape in a slid-out pane slides it in.

use rapidr_value::dock::access::{self, AccessPart};
use rapidr_value::dock::geometry::{self, DocHit, Group, GroupHit, Hit, Slot};
use rapidr_value::dock::manager::{self, Drag, Part, SplitDrag, User};
use rapidr_value::dock::{look, Axis, Target};
use rapidr_value::input::Button;
use rapidr_value::objects::a11y::{AccessNode, Action};

use super::form::Container;
use super::{ComponentKind, Cx, MouseIn, MouseKind, MouseOut};
use crate::a11y::AccessValue;
use crate::input::{KernelEvent, Mods};
use crate::paint::Painter;
use crate::store::{self, Store};
use crate::text::TextSystem;
use crate::tree::FormUi;

pub struct DockManager;
pub struct DockGroup;
pub struct DockDocs;

/// How far the mouse goes before a press on a tab is a drag.
const DRAG_START: i64 = 5;

fn send(cx: &mut Cx, dock: &str, action: User) {
    cx.events.push(KernelEvent::Container(Container::Dock { id: dock.to_string(), action }));
}

/// The dock manager's model laid out at this size and font (the kernel
/// paints before the runtime may have placed a resize).
fn laid_out(cx: &Cx) -> bool {
    let id = cx.id.to_string();
    let (w, h) = (cx.width(), cx.height());
    manager::exists(&id) && {
        manager::with_mut(&id, |m| {
            m.resize((w, h), &cx.font);
        });
        true
    }
}

/// A child's z-order: a dock manager's slid-out pane over its groups.
pub fn stacked(parent: &str, mut children: Vec<(String, String)>) -> Vec<(String, String)> {
    if !manager::exists(parent) {
        return children;
    }
    let fly = rapidr_value::dock::runtime::group_name(parent, Slot::Flyout);
    if let Some(i) = children.iter().position(|(c, _)| c.eq_ignore_ascii_case(&fly)) {
        let f = children.remove(i);
        children.push(f);
    }
    children
}

impl ComponentKind for DockManager {
    fn name(&self) -> &'static str {
        "RDOCKMANAGER"
    }

    fn focusable(&self, _store: &dyn Store, _id: &str) -> bool {
        false
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        if !laid_out(cx) {
            p.fill((0, 0, cx.width(), cx.height()), look::palette(p.theme()).ground);
            return;
        }
        let theme = p.theme();
        let ops = manager::with_mut(cx.id, |m| {
            let g = m.geometry().clone();
            look::manager_ops(m, &g, theme, &cx.font)
        });
        p.ops(ops);
    }

    fn paint_over(&self, store: &dyn Store, id: &str, _w: i64, _h: i64, p: &mut Painter) {
        if !manager::exists(id) {
            return;
        }
        let font = store.font(id);
        let theme = p.theme();
        let ops = manager::with_mut(id, |m| {
            let g = m.geometry().clone();
            look::overlay_ops(m, &g, theme, &font)
        });
        p.ops(ops);
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        let out = MouseOut { press: false, focus: Some(false) };
        if !laid_out(cx) {
            return out;
        }
        let id = cx.id.to_string();
        let (x, y) = (m.x.floor() as i64, m.y.floor() as i64);
        let hit = manager::with_mut(&id, |d| d.geometry().hit(x, y));
        match m.kind {
            MouseKind::Move if !m.captured => manager::with_mut(&id, |d| d.ui.hover = hit.clone().map(Part::Manager)),
            MouseKind::Leave => manager::with_mut(&id, |d| {
                if matches!(d.ui.hover, Some(Part::Manager(_))) {
                    d.ui.hover = None;
                }
            }),
            MouseKind::Down if m.button == Button::Left => match hit {
                Some(Hit::Splitter(i)) => manager::with_mut(&id, |d| {
                    let g = d.geometry().clone();
                    let s = &g.splitters[i];
                    let start = g.extents.iter().find(|(p, _)| *p == s.path).map(|(_, e)| e.clone()).unwrap_or_default();
                    let from = if s.axis == Axis::Row { x } else { y };
                    d.ui.split = Some(SplitDrag { splitter: i, start, from });
                }),
                Some(Hit::Strip(p)) => {
                    let open = manager::with(&id, |d| d.flyout.as_deref() == Some(p.as_str())).unwrap_or(false);
                    send(cx, &id, User::Flyout(if open { None } else { Some(p) }));
                }
                None => {
                    // (a press on the ground: the flyout slides in)
                    if manager::with(&id, |d| d.flyout.is_some()).unwrap_or(false) {
                        send(cx, &id, User::Flyout(None));
                    }
                }
            },
            MouseKind::Move if m.captured => {
                let step = manager::with_mut(&id, |d| {
                    let s = d.ui.split.clone()?;
                    let axis = d.geometry().splitters.get(s.splitter)?.axis;
                    let delta = if axis == Axis::Row { x } else { y } - s.from;
                    d.split_extents(s.splitter, &s.start, delta).map(|e| (s.splitter, e))
                });
                if let Some((splitter, extents)) = step {
                    send(cx, &id, User::Split { splitter, extents, done: false });
                }
            }
            MouseKind::Up => {
                let done = manager::with_mut(&id, |d| {
                    let s = d.ui.split.take()?;
                    let axis = d.geometry().splitters.get(s.splitter)?.axis;
                    let delta = if axis == Axis::Row { x } else { y } - s.from;
                    d.split_extents(s.splitter, &s.start, delta).map(|e| (s.splitter, e))
                });
                if let Some((splitter, extents)) = done {
                    send(cx, &id, User::Split { splitter, extents, done: true });
                }
            }
            _ => {}
        }
        out
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        if !laid_out(cx) {
            return super::shared_describe(cx, self.name());
        }
        let at = (cx.rect.0, cx.rect.1);
        manager::with_mut(cx.id, |m| {
            let g = m.geometry().clone();
            access::describe_manager(cx.id, m, &g, at)
        })
    }

    fn access(&self, cx: &mut Cx, action: Action, part: Option<usize>, _value: Option<&AccessValue>) -> bool {
        let id = cx.id.to_string();
        let Some(part) = part.and_then(access::decode) else { return false };
        match (part, action) {
            (AccessPart::Splitter(i), Action::Increment | Action::Decrement) => {
                let step = if action == Action::Increment { 8 } else { -8 };
                let ext = manager::with_mut(&id, |d| {
                    let g = d.geometry().clone();
                    let s = g.splitters.get(i)?;
                    let start = g.extents.iter().find(|(p, _)| *p == s.path)?.1.clone();
                    d.split_extents(i, &start, step)
                });
                if let Some(extents) = ext {
                    send(cx, &id, User::Split { splitter: i, extents, done: true });
                }
                true
            }
            (AccessPart::Strip(i), Action::Click) => {
                let p = manager::with_mut(&id, |d| d.geometry().strip_tabs.get(i).map(|t| t.pane.clone())).map(|p| {
                    let open = manager::with(&id, |d| d.flyout.as_deref() == Some(p.as_str())).unwrap_or(false);
                    if open {
                        None
                    } else {
                        Some(p)
                    }
                });
                if let Some(p) = p {
                    send(cx, &id, User::Flyout(p));
                }
                true
            }
            _ => false,
        }
    }
}

// ------------------------------------------------------------- groups --

/// A group component's dock manager and slot (`__dock`, `__slot`).
fn slot_of(store: &dyn Store, id: &str) -> Option<(String, Slot)> {
    let dock = store::string(store, id, "__dock").to_ascii_lowercase();
    let slot = Slot::parse(&store::string(store, id, "__slot"))?;
    manager::exists(&dock).then_some((dock, slot))
}

/// A group's geometry now (its own size: a floating one's is its window's).
fn group_geo(dock: &str, slot: Slot, w: i64, h: i64, font: &rapidr_value::objects::font::Font) -> Option<Group> {
    manager::with_mut(dock, |m| match slot {
        Slot::Float(i) => {
            let f = m.layout.floating.get(i)?.clone();
            Some(geometry::group(slot, &f.panes, f.active, (0, 0, w, h), Vec::new(), &m.titles(), font))
        }
        s => m.geometry().slot(s).cloned(),
    })
}

/// Whether a group holds the active pane.
fn holds_active(dock: &str, gr: &Group) -> bool {
    manager::with(dock, |m| m.active_pane.as_ref().is_some_and(|a| gr.panes.contains(a))).unwrap_or(false)
}

impl ComponentKind for DockGroup {
    fn name(&self) -> &'static str {
        "RDOCKGROUP"
    }

    fn focusable(&self, _store: &dyn Store, _id: &str) -> bool {
        false
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let theme = p.theme();
        let Some((dock, slot)) = slot_of(cx.store, cx.id) else {
            p.fill((0, 0, cx.width(), cx.height()), theme.face);
            return;
        };
        let font = cx.store.font(&dock);
        let Some(gr) = group_geo(&dock, slot, cx.width(), cx.height(), &font) else { return };
        let active = holds_active(&dock, &gr);
        let ops = manager::with_mut(&dock, |m| look::group_ops(m, &gr, theme, &font, active));
        p.ops(ops);
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        let out = MouseOut { press: false, focus: Some(false) };
        let Some((dock, slot)) = slot_of(cx.store, cx.id) else { return out };
        let font = cx.store.font(&dock);
        let Some(gr) = group_geo(&dock, slot, cx.width(), cx.height(), &font) else { return out };
        let (x, y) = (m.x.floor() as i64, m.y.floor() as i64);
        let hit = gr.hit(x, y);
        // (the dock manager's coordinates: a docked group's rectangle is in them)
        let at = (gr.rect.0 + x, gr.rect.1 + y);
        let floating = matches!(slot, Slot::Float(_));
        match m.kind {
            MouseKind::Move if !m.captured => manager::with_mut(&dock, |d| {
                let h = hit.filter(|h| *h != GroupHit::Content).map(|h| Part::Group(slot, h));
                if h != d.ui.hover {
                    d.ui.hover = h;
                }
            }),
            MouseKind::Leave => manager::with_mut(&dock, |d| {
                if matches!(d.ui.hover, Some(Part::Group(s, _)) if s == slot) {
                    d.ui.hover = None;
                }
            }),
            MouseKind::Down if m.button == Button::Left => match hit {
                Some(GroupHit::Button(b)) => manager::with_mut(&dock, |d| d.ui.pressed = Some(Part::Group(slot, GroupHit::Button(b)))),
                Some(GroupHit::Tab(_)) | Some(GroupHit::Header) => {
                    let pane = match hit {
                        Some(GroupHit::Tab(i)) => gr.panes[i].clone(),
                        _ => gr.panes[gr.active].clone(),
                    };
                    if m.double() && hit == Some(GroupHit::Header) {
                        // (a double click on the title: a docked group floats, a floating one docks)
                        if floating {
                            send(cx, &dock, User::Button(slot, geometry::Button::Dock));
                        } else {
                            send(cx, &dock, User::Float(pane, at.0 - 40, at.1 - 12));
                        }
                        return out;
                    }
                    send(cx, &dock, User::Select(pane.clone()));
                    if !floating {
                        manager::with_mut(&dock, |d| d.ui.drag = Some(Drag { pane, from: at, at, started: false, target: None }));
                    }
                }
                _ => {}
            },
            // (a middle click on a tab closes it)
            MouseKind::Up if m.button == Button::Middle && m.inside => {
                if let Some(GroupHit::Tab(i)) = hit {
                    let pane = gr.panes[i].clone();
                    manager::with_mut(&dock, |d| d.ui.hover = None);
                    send(cx, &dock, User::Button(slot, geometry::Button::Close));
                    let _ = pane;
                }
            }
            MouseKind::Move if m.captured => manager::with_mut(&dock, |d| {
                let Some(mut drag) = d.ui.drag.clone() else { return };
                drag.at = at;
                if !drag.started && ((at.0 - drag.from.0).abs() > DRAG_START || (at.1 - drag.from.1).abs() > DRAG_START) {
                    drag.started = true;
                    d.ui.hover = None;
                }
                if drag.started {
                    let g = d.geometry().clone();
                    drag.target = g.compass(&drag.pane, Some(at), None).into_iter().find(|(_, r, _)| at.0 >= r.0 && at.1 >= r.1 && at.0 < r.0 + r.2 && at.1 < r.1 + r.3).map(|(_, _, t)| t);
                }
                d.ui.drag = Some(drag);
            }),
            MouseKind::Up => {
                let drag = manager::with_mut(&dock, |d| d.ui.drag.take());
                let pressed = manager::with_mut(&dock, |d| d.ui.pressed.take());
                match drag {
                    Some(Drag { pane, started: true, target: Some(t), .. }) => send(cx, &dock, User::Drop(pane, t)),
                    Some(Drag { pane, started: true, target: None, at, .. }) => send(cx, &dock, User::Float(pane, at.0 - 40, at.1 - 12)),
                    _ => {
                        if let (Some(Part::Group(_, GroupHit::Button(b))), Some(GroupHit::Button(now))) = (pressed, hit) {
                            if b == now && m.inside {
                                send(cx, &dock, User::Button(slot, b));
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        out
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        let Some((dock, slot)) = slot_of(cx.store, cx.id) else { return super::shared_describe(cx, self.name()) };
        let font = cx.store.font(&dock);
        let Some(gr) = group_geo(&dock, slot, cx.width(), cx.height(), &font) else { return super::shared_describe(cx, self.name()) };
        let active = holds_active(&dock, &gr);
        manager::with(&dock, |m| access::describe_group(cx.id, m, &gr, (cx.rect.0, cx.rect.1), active)).unwrap_or_else(|| super::shared_describe(cx, self.name()))
    }

    fn access(&self, cx: &mut Cx, action: Action, part: Option<usize>, _value: Option<&AccessValue>) -> bool {
        let Some((dock, slot)) = slot_of(cx.store, cx.id) else { return false };
        let font = cx.store.font(&dock);
        let Some(gr) = group_geo(&dock, slot, cx.width(), cx.height(), &font) else { return false };
        match (part.and_then(access::decode), action) {
            (Some(AccessPart::Tab(i)), Action::Click) if i < gr.panes.len() => send(cx, &dock, User::Select(gr.panes[i].clone())),
            (Some(AccessPart::Button(b)), Action::Click) => send(cx, &dock, User::Button(slot, b)),
            _ => return false,
        }
        true
    }
}

// ---------------------------------------------------------- documents --

fn docs_dock(store: &dyn Store, id: &str) -> Option<String> {
    let dock = store::string(store, id, "__dock").to_ascii_lowercase();
    manager::exists(&dock).then_some(dock)
}

impl ComponentKind for DockDocs {
    fn name(&self) -> &'static str {
        "RDOCKDOCS"
    }

    fn focusable(&self, _store: &dyn Store, _id: &str) -> bool {
        false
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let theme = p.theme();
        let Some(dock) = docs_dock(cx.store, cx.id) else {
            p.fill((0, 0, cx.width(), cx.height()), look::palette(theme).workspace);
            return;
        };
        let font = cx.store.font(&dock);
        let ops = manager::with_mut(&dock, |m| {
            let d = m.geometry().documents.clone()?;
            Some(look::documents_ops(m, &d, theme, &font))
        });
        p.ops(ops.unwrap_or_default());
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        let out = MouseOut { press: false, focus: Some(false) };
        let Some(dock) = docs_dock(cx.store, cx.id) else { return out };
        let (x, y) = (m.x.floor() as i64, m.y.floor() as i64);
        let Some(d) = manager::with_mut(&dock, |mm| mm.geometry().documents.clone()) else { return out };
        let hit = d.hit(x, y);
        match m.kind {
            MouseKind::Move if !m.captured => manager::with_mut(&dock, |mm| {
                let h = hit.map(Part::Doc);
                let ours = matches!(mm.ui.hover, Some(Part::Doc(_)) | None);
                if ours {
                    mm.ui.hover = h;
                }
            }),
            MouseKind::Leave => manager::with_mut(&dock, |mm| {
                if matches!(mm.ui.hover, Some(Part::Doc(_))) {
                    mm.ui.hover = None;
                }
            }),
            MouseKind::Down if m.button == Button::Left => match hit {
                Some(DocHit::Tab(i)) => send(cx, &dock, User::Select(d.tabs[i].pane.clone())),
                Some(DocHit::Close(i)) => manager::with_mut(&dock, |mm| mm.ui.pressed = Some(Part::Doc(DocHit::Close(i)))),
                None => {}
            },
            MouseKind::Up => {
                let pressed = manager::with_mut(&dock, |mm| mm.ui.pressed.take());
                match (m.button, pressed, hit) {
                    (Button::Left, Some(Part::Doc(DocHit::Close(a))), Some(DocHit::Close(b))) if a == b && m.inside => send(cx, &dock, User::CloseDocument(d.tabs[a].pane.clone())),
                    (Button::Middle, _, Some(DocHit::Tab(i) | DocHit::Close(i))) if m.inside => send(cx, &dock, User::CloseDocument(d.tabs[i].pane.clone())),
                    _ => {}
                }
            }
            _ => {}
        }
        out
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        let Some(dock) = docs_dock(cx.store, cx.id) else { return super::shared_describe(cx, self.name()) };
        let at = (cx.rect.0, cx.rect.1);
        manager::with_mut(&dock, |m| {
            let d = m.geometry().documents.clone()?;
            Some(access::describe_documents(cx.id, m, &d, at))
        })
        .unwrap_or_else(|| super::shared_describe(cx, self.name()))
    }

    fn access(&self, cx: &mut Cx, action: Action, part: Option<usize>, _value: Option<&AccessValue>) -> bool {
        let Some(dock) = docs_dock(cx.store, cx.id) else { return false };
        let Some(d) = manager::with_mut(&dock, |m| m.geometry().documents.clone()) else { return false };
        match (part.and_then(access::decode), action) {
            (Some(AccessPart::Tab(i)), Action::Click) if i < d.tabs.len() => send(cx, &dock, User::Select(d.tabs[i].pane.clone())),
            (Some(AccessPart::Close(i)), Action::Click) if i < d.tabs.len() => send(cx, &dock, User::CloseDocument(d.tabs[i].pane.clone())),
            _ => return false,
        }
        true
    }
}

// ------------------------------------------------------- the keyboard --

impl FormUi {
    /// The dock manager node `i` is in (itself, through a group or the
    /// document area), and the pane holding it.
    fn dock_around(&self, store: &dyn Store, i: usize) -> Option<(String, Option<String>)> {
        let mut at = Some(i);
        let mut pane: Option<String> = None;
        let mut last: Option<String> = None;
        while let Some(n) = at {
            let node = &self.nodes[n];
            match node.type_name.as_str() {
                "RDOCKMANAGER" => return Some((node.id.clone(), pane)),
                "RDOCKGROUP" | "RDOCKDOCS" => {
                    if pane.is_none() {
                        pane = last.clone();
                    }
                    let dock = store::string(store, &node.id, "__dock").to_ascii_lowercase();
                    if manager::exists(&dock) {
                        // (an MDI window's frame holds the document)
                        if let Some(p) = &pane {
                            if let Some(c) = p.split("__mdi__").nth(1) {
                                pane = Some(c.to_string());
                            }
                        }
                        return Some((dock, pane));
                    }
                }
                _ => {}
            }
            last = Some(node.id.clone());
            at = node.parent;
        }
        None
    }

    /// The first node in Tab order inside component `pane` (itself first).
    fn first_focus_in(&self, store: &dyn Store, pane: &str) -> Option<usize> {
        let root = self.index_of(pane)?;
        let inside = |mut i: usize| loop {
            if i == root {
                return true;
            }
            match self.nodes[i].parent {
                Some(p) => i = p,
                None => return false,
            }
        };
        self.tab_order(store).into_iter().find(|&i| inside(i)).or_else(|| (0..self.nodes.len()).find(|&i| inside(i) && self.can_focus(store, i)))
    }
}

/// A key for the dock managers of form `f` (before the focused component
/// gets it): whether it was theirs.
pub fn key(f: &mut FormUi, store: &dyn Store, _ts: &mut TextSystem, vk: i64, mods: Mods) -> bool {
    let around = match f.focus {
        Some(i) => f.dock_around(store, i),
        None => None,
    };
    // (not inside one: the form's first)
    let (dock, pane) = match around {
        Some(a) => a,
        None => match f.nodes.iter().find(|n| n.type_name == "RDOCKMANAGER" && n.shown && manager::exists(&n.id)) {
            Some(n) => (n.id.clone(), None),
            None => return false,
        },
    };
    if !manager::exists(&dock) {
        return false;
    }
    let ctrl = mods.ctrl || mods.command;
    let push = |f: &mut FormUi, action: User| f.events.push(KernelEvent::Container(Container::Dock { id: dock.clone(), action }));
    // (the keyboard's move takes every key while it lasts)
    if manager::with(&dock, |m| m.ui.moving.is_some()).unwrap_or(false) {
        let (took, action) = manager::with_mut(&dock, |m| m.move_key(vk, mods.shift));
        f.dirty = true;
        if let Some(a) = action {
            push(f, a);
        }
        return took;
    }
    match vk {
        // F6 / Shift+F6: the next / previous area
        117 if !ctrl && !mods.alt => {
            let areas = manager::with_mut(&dock, |m| m.areas_for_keys());
            if areas.is_empty() {
                return false;
            }
            let n = areas.len();
            let now = pane.as_ref().and_then(|p| areas.iter().position(|a| a == p));
            let next = match now {
                Some(k) if mods.shift => (k + n - 1) % n,
                Some(k) => (k + 1) % n,
                None if mods.shift => n - 1,
                None => 0,
            };
            let target = areas[next].clone();
            if let Some(i) = f.first_focus_in(store, &target) {
                f.set_focus(Some(i));
            }
            push(f, User::Activate(target));
            f.dirty = true;
            true
        }
        // Ctrl+Tab / Ctrl+Shift+Tab: the next / previous document
        9 if ctrl && !mods.alt => {
            if manager::with(&dock, |m| m.layout.documents.len() < 2).unwrap_or(true) {
                return false;
            }
            push(f, User::NextDocument(mods.shift));
            true
        }
        // Ctrl+Shift+M: move the active pane with the compass
        77 if ctrl && mods.shift && !mods.alt => {
            let p = pane.or_else(|| manager::with(&dock, |m| m.active_pane.clone()).flatten());
            let Some(p) = p else { return false };
            let ok = manager::with_mut(&dock, |m| m.begin_move(&p));
            f.dirty = true;
            ok
        }
        // Escape in a slid-out pane: it slides in
        27 if !ctrl => {
            let open = manager::with(&dock, |m| m.flyout.clone()).flatten();
            if open.is_some() && open == pane {
                push(f, User::Flyout(None));
                return true;
            }
            false
        }
        _ => false,
    }
}

/// The drop target under the dock manager's point `at` while `pane` is
/// dragged (a test hook's and the kernel's).
pub fn target_at(dock: &str, pane: &str, at: (i64, i64)) -> Option<Target> {
    manager::with_mut(dock, |d| {
        let g = d.geometry().clone();
        g.compass(pane, Some(at), None).into_iter().find(|(_, r, _)| at.0 >= r.0 && at.1 >= r.1 && at.0 < r.0 + r.2 && at.1 < r.1 + r.3).map(|(_, _, t)| t)
    })
}

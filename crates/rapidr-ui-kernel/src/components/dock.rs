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
use rapidr_value::dock::manager::{self, DocDrag, DocSplitDrag, Drag, Part, SplitDrag, User};
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

    /// (tooltip.rs) A tab's whole title, a header button's name.
    fn tip_at(&self, store: &dyn Store, id: &str, x: f64, y: f64) -> Option<String> {
        let (dock, slot) = slot_of(store, id)?;
        let font = store.font(&dock);
        let (w, h) = (store::int(store, id, "width", 0), store::int(store, id, "height", 0));
        let gr = group_geo(&dock, slot, w, h, &font)?;
        let titles = manager::with(&dock, |m| { use rapidr_value::dock::geometry::Titles; let t = m.titles(); gr.panes.iter().map(|p| t.title(p)).collect::<Vec<_>>() })?;
        match gr.hit(x.floor() as i64, y.floor() as i64)? {
            GroupHit::Button(rapidr_value::dock::geometry::Button::Close) => Some("Close".into()),
            GroupHit::Button(rapidr_value::dock::geometry::Button::Pin) => Some(if slot == Slot::Flyout { "Dock" } else { "Auto Hide" }.into()),
            GroupHit::Button(rapidr_value::dock::geometry::Button::Dock) => Some("Dock".into()),
            GroupHit::Tab(i) => titles.get(i).cloned(),
            GroupHit::Header => titles.get(gr.active).cloned(),
            GroupHit::Content => None,
        }
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

    /// (tooltip.rs) A tab's whole title, its close button, a view switch's
    /// segment.
    fn tip_at(&self, store: &dyn Store, id: &str, x: f64, y: f64) -> Option<String> {
        let dock = docs_dock(store, id)?;
        let d = manager::with_mut(&dock, |m| m.geometry().documents.clone())?;
        let titles = |p: &str| manager::with(&dock, |m| { use rapidr_value::dock::geometry::Titles; m.titles().title(p) }).unwrap_or_default();
        match d.hit(x.floor() as i64, y.floor() as i64)? {
            DocHit::Tab(g, i) => Some(titles(&d.groups[g].tabs[i].pane)),
            DocHit::Close(g, i) => Some(format!("Close {}", titles(&d.groups[g].tabs[i].pane))),
            DocHit::View(g, k) => d.groups[g].switch.get(k).map(|(_, c, _)| c.clone()),
            _ => None,
        }
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

    /// (over the documents' components) Where a dragged tab would go.
    fn paint_over(&self, store: &dyn Store, id: &str, _w: i64, _h: i64, p: &mut Painter) {
        let Some(dock) = docs_dock(store, id) else { return };
        let theme = p.theme();
        let ops = manager::with(&dock, |m| look::documents_overlay_ops(m, theme)).unwrap_or_default();
        p.ops(ops);
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
                Some(DocHit::Tab(g, i)) => {
                    let pane = d.groups[g].tabs[i].pane.clone();
                    send(cx, &dock, User::Select(pane.clone()));
                    manager::with_mut(&dock, |mm| mm.ui.doc_drag = Some(DocDrag { pane, from: (x, y), at: (x, y), started: false, target: None, preview: None, bar: None }));
                }
                Some(DocHit::Close(g, i)) => manager::with_mut(&dock, |mm| mm.ui.pressed = Some(Part::Doc(DocHit::Close(g, i)))),
                Some(DocHit::View(g, k)) => {
                    if let (Some(doc), Some((v, _, _))) = (d.groups[g].shown().cloned(), d.groups[g].switch.get(k)) {
                        send(cx, &dock, User::View(doc, *v));
                    }
                }
                Some(DocHit::Splitter(i)) => {
                    let sp = &d.splitters[i];
                    let start = d.extents.iter().find(|(p, _)| *p == sp.path).map(|(_, e)| e.clone()).unwrap_or_default();
                    let from = if sp.axis == Axis::Row { x } else { y };
                    manager::with_mut(&dock, |mm| mm.ui.doc_split = Some(DocSplitDrag { splitter: i, views_of: None, start, from }));
                }
                Some(DocHit::ViewSplitter(g)) => {
                    let c = d.groups[g].content;
                    manager::with_mut(&dock, |mm| mm.ui.doc_split = Some(DocSplitDrag { splitter: 0, views_of: Some(g), start: vec![c.0, c.2], from: x }));
                }
                Some(DocHit::Strip(_)) | None => {}
            },
            // (a double click on a strip's empty part: nothing; on a tab: the
            // tab stays — VS Code pins it, we have no preview tabs)
            MouseKind::Move if m.captured => {
                if let Some(step) = doc_split_step(&dock, &d, x, y, false) {
                    send(cx, &dock, step);
                    return out;
                }
                manager::with_mut(&dock, |mm| {
                    let Some(mut drag) = mm.ui.doc_drag.clone() else { return };
                    drag.at = (x, y);
                    if !drag.started && ((x - drag.from.0).abs() > DRAG_START || (y - drag.from.1).abs() > DRAG_START) {
                        drag.started = true;
                        mm.ui.hover = None;
                    }
                    if drag.started {
                        match d.drop_at(&drag.pane, x, y) {
                            Some((t, preview, bar)) => {
                                drag.target = Some(t);
                                drag.preview = preview;
                                drag.bar = bar;
                            }
                            None => {
                                drag.target = None;
                                drag.preview = None;
                                drag.bar = None;
                            }
                        }
                    }
                    mm.ui.doc_drag = Some(drag);
                });
            }
            MouseKind::Up => {
                if let Some(step) = doc_split_step(&dock, &d, x, y, true) {
                    manager::with_mut(&dock, |mm| mm.ui.doc_split = None);
                    send(cx, &dock, step);
                    return out;
                }
                manager::with_mut(&dock, |mm| mm.ui.doc_split = None);
                let drag = manager::with_mut(&dock, |mm| mm.ui.doc_drag.take());
                if let Some(DocDrag { pane, started: true, target: Some(t), .. }) = drag {
                    send(cx, &dock, User::MoveDocument(pane, t));
                    return out;
                }
                let pressed = manager::with_mut(&dock, |mm| mm.ui.pressed.take());
                match (m.button, pressed, hit) {
                    (Button::Left, Some(Part::Doc(DocHit::Close(a, i))), Some(DocHit::Close(b, k))) if (a, i) == (b, k) && m.inside => send(cx, &dock, User::CloseDocument(d.groups[a].tabs[i].pane.clone())),
                    (Button::Middle, _, Some(DocHit::Tab(g, i) | DocHit::Close(g, i))) if m.inside => send(cx, &dock, User::CloseDocument(d.groups[g].tabs[i].pane.clone())),
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
        let tabs: Vec<String> = d.groups.iter().flat_map(|g| g.tabs.iter().map(|t| t.pane.clone())).collect();
        match (part.and_then(access::decode), action) {
            (Some(AccessPart::Tab(i)), Action::Click) if i < tabs.len() => send(cx, &dock, User::Select(tabs[i].clone())),
            (Some(AccessPart::Close(i)), Action::Click) if i < tabs.len() => send(cx, &dock, User::CloseDocument(tabs[i].clone())),
            (Some(AccessPart::View(g, k)), Action::Click) => {
                let Some(gr) = d.groups.get(g) else { return false };
                let (Some(doc), Some((v, _, _))) = (gr.shown().cloned(), gr.switch.get(k)) else { return false };
                send(cx, &dock, User::View(doc, *v));
            }
            (Some(AccessPart::DocSplitter(i)), Action::Increment | Action::Decrement) => {
                let Some(sp) = d.splitters.get(i) else { return false };
                let Some((_, start)) = d.extents.iter().find(|(p, _)| *p == sp.path) else { return false };
                let step = if action == Action::Increment { 8 } else { -8 };
                let extents = doc_extents(start, sp.index, step);
                send(cx, &dock, User::DocSplit { path: sp.path.clone(), extents, done: true });
            }
            _ => return false,
        }
        true
    }
}

/// A split's extents with splitter `index` moved `delta` pixels, both
/// neighbours kept at their least.
fn doc_extents(start: &[i64], index: usize, delta: i64) -> Vec<i64> {
    let mut ext = start.to_vec();
    let (a, b) = (index, index + 1);
    if b >= ext.len() {
        return ext;
    }
    let min = geometry::MIN_EXTENT;
    let delta = delta.clamp(-(ext[a] - min).max(0), (ext[b] - min).max(0));
    ext[a] += delta;
    ext[b] -= delta;
    ext
}

/// A held splitter of the document area at (x, y): the user's step (a
/// group split's new extents, or a document's views' share).
fn doc_split_step(dock: &str, d: &geometry::Documents, x: i64, y: i64, done: bool) -> Option<User> {
    let s = manager::with(dock, |m| m.ui.doc_split.clone()).flatten()?;
    match s.views_of {
        Some(g) => {
            let gr = d.groups.get(g)?;
            let doc = gr.shown()?.clone();
            let (cx0, cw) = (s.start[0], s.start[1].max(1));
            let ratio = ((x - cx0) * 1000 / cw).clamp(100, 900);
            Some(User::ViewRatio(doc, ratio, done))
        }
        None => {
            let sp = d.splitters.get(s.splitter)?;
            let delta = if sp.axis == Axis::Row { x } else { y } - s.from;
            Some(User::DocSplit { path: sp.path.clone(), extents: doc_extents(&s.start, sp.index, delta), done })
        }
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

/// The resize pointer over a dock manager's splitter or its document
/// area's (`Some(true)`: one standing between left and right, ↔), at
/// (x, y) of component `id` of kind `kind`.
pub fn splitter_cursor(store: &dyn Store, kind: &str, id: &str, x: i64, y: i64) -> Option<bool> {
    let grab = |r: &rapidr_value::dock::Rect| x >= r.0 - 1 && y >= r.1 - 1 && x < r.0 + r.2 + 1 && y < r.1 + r.3 + 1;
    match kind {
        "RDOCKMANAGER" => {
            if !manager::exists(id) {
                return None;
            }
            manager::with_mut(id, |m| {
                if let Some(s) = m.ui.split.clone() {
                    return m.geometry().splitters.get(s.splitter).map(|s| s.axis == Axis::Row);
                }
                m.geometry().splitters.iter().find(|s| grab(&s.rect)).map(|s| s.axis == Axis::Row)
            })
        }
        "RDOCKDOCS" => {
            let dock = docs_dock(store, id)?;
            manager::with_mut(&dock, |m| {
                let d = m.geometry().documents.clone()?;
                if let Some(s) = &m.ui.doc_split {
                    return match s.views_of {
                        Some(_) => Some(true),
                        None => d.splitters.get(s.splitter).map(|s| s.axis == Axis::Row),
                    };
                }
                if d.groups.iter().any(|g| g.view_splitter.as_ref().is_some_and(grab)) {
                    return Some(true);
                }
                d.splitters.iter().find(|s| grab(&s.rect)).map(|s| s.axis == Axis::Row)
            })
        }
        _ => None,
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

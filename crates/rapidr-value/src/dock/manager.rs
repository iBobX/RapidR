//! One dock manager: its panes (title, icon, where each goes back to),
//! the layout, which pane is active, the auto-hide flyout open, the size
//! and font it was laid out at, and the user's transient state (what's
//! under the mouse, a drag with its compass target, a splitter held, the
//! keyboard's "move pane"). Kept per component in a thread-local table,
//! read by the kernel (to draw it, to hit-test) and changed by the runtime
//! glue (`super::runtime`) — the model of the desktop and the web alike.

use std::cell::RefCell;
use std::collections::HashMap;

use super::geometry::{self, Geometry, Guide, Slot, Titles};
use super::{Anchor, Axis, DocTarget, DocumentMode, Layout, Node, Place, Rect, Side, Target, Where};
use crate::objects::font::Font;
use crate::Value;

/// A pane: a component shown in a group (a tool pane) or the document
/// area (a document).
#[derive(Clone, Debug, PartialEq)]
pub struct PaneInfo {
    /// Its component's name, lowercase.
    pub name: String,
    /// Its name as Pane(i) and the events say it: lowercase, as both
    /// runtimes' component registries keep names (a native build's
    /// arguments arrive lowercase).
    pub given: String,
    pub title: String,
    pub icon: String,
    /// Where it was when it last left the layout (hidden, auto-hidden,
    /// floated), to go back to.
    pub place: Option<Place>,
    /// Its extent as an auto-hide flyout.
    pub extent: i64,
    /// Its floating window's size.
    pub float_size: (i64, i64),
    /// (a document) Its views — (component, caption) — as AddView gave
    /// them: the tab strip's Design | Code switch; none: its own component.
    pub views: Vec<(String, String)>,
    /// (a document) The view shown.
    pub view: DocView,
    /// (a document) Side by side: the first view's share, per mille.
    pub view_ratio: i64,
    /// (a document) Changed and not saved: a dot on its tab.
    pub modified: bool,
}

/// What a document with views shows: one of them, or the first two side
/// by side.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DocView {
    #[default]
    First,
    One(usize),
    Split,
}

impl DocView {
    /// The view's index (side by side: `None`).
    pub fn index(self) -> Option<usize> {
        match self {
            DocView::First => Some(0),
            DocView::One(i) => Some(i),
            DocView::Split => None,
        }
    }
}

impl PaneInfo {
    fn new(name: &str, title: &str, icon: &str) -> PaneInfo {
        PaneInfo { name: name.to_string(), given: name.to_string(), title: title.to_string(), icon: icon.to_string(), place: None, extent: super::DEFAULT_SIDE, float_size: super::DEFAULT_FLOAT, views: Vec::new(), view: DocView::First, view_ratio: 500, modified: false }
    }

    /// The view shown, by caption ("Split": side by side).
    pub fn view_caption(&self) -> String {
        match self.view.index() {
            None => "Split".into(),
            Some(i) => self.views.get(i).map(|v| v.1.clone()).unwrap_or_default(),
        }
    }

    /// The components shown for it now: its own, else the view's (two
    /// side by side).
    pub fn shown_components(&self) -> Vec<String> {
        if self.views.is_empty() {
            return vec![self.name.clone()];
        }
        match self.view.index() {
            None if self.views.len() >= 2 => vec![self.views[0].0.clone(), self.views[1].0.clone()],
            None => vec![self.views[0].0.clone()],
            Some(i) => vec![self.views.get(i).unwrap_or(&self.views[0]).0.clone()],
        }
    }
}

/// What's under the mouse (or pressed) on the dock manager's parts.
#[derive(Clone, Debug, PartialEq)]
pub enum Part {
    Group(Slot, geometry::GroupHit),
    Doc(geometry::DocHit),
    Manager(geometry::Hit),
}

/// A pane dragged by its tab or header: from where (the dock manager's
/// coordinates), to where now, whether it has moved far enough to be a
/// drag, and the compass button under the mouse.
#[derive(Clone, Debug, PartialEq)]
pub struct Drag {
    pub pane: String,
    pub from: (i64, i64),
    pub at: (i64, i64),
    pub started: bool,
    pub target: Option<Target>,
}

/// A splitter held: its index in the geometry, the split's extents then,
/// the mouse along the axis then.
#[derive(Clone, Debug, PartialEq)]
pub struct SplitDrag {
    pub splitter: usize,
    pub start: Vec<i64>,
    pub from: i64,
}

/// A document's tab dragged (the document area's coordinates): to
/// another place in its strip, another group's, or a side of a group (a
/// new group there) — `preview` the outline of where it goes, `bar` the
/// insertion mark between tabs.
#[derive(Clone, Debug, PartialEq)]
pub struct DocDrag {
    pub pane: String,
    pub from: (i64, i64),
    pub at: (i64, i64),
    pub started: bool,
    pub target: Option<DocTarget>,
    pub preview: Option<Rect>,
    pub bar: Option<Rect>,
}

/// A splitter of the document area held: between groups (`group`:
/// `None`; its index in the geometry) or between a document's two views
/// side by side (its group's index).
#[derive(Clone, Debug, PartialEq)]
pub struct DocSplitDrag {
    pub splitter: usize,
    pub views_of: Option<usize>,
    pub start: Vec<i64>,
    pub from: i64,
}

/// The keyboard's "move pane": the pane, the area whose compass is shown
/// and the button chosen.
#[derive(Clone, Debug, PartialEq)]
pub struct KeyMove {
    pub pane: String,
    pub area: usize,
    pub guide: Guide,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ui {
    pub hover: Option<Part>,
    pub pressed: Option<Part>,
    pub drag: Option<Drag>,
    pub split: Option<SplitDrag>,
    pub moving: Option<KeyMove>,
    pub doc_drag: Option<DocDrag>,
    pub doc_split: Option<DocSplitDrag>,
}

/// An event for the program (OnPaneChange (Name), OnDocumentActivate
/// (Name), OnLayoutChange).
#[derive(Clone, Debug, PartialEq)]
pub struct Event {
    pub name: &'static str,
    pub args: Vec<Value>,
}

impl Event {
    fn pane(name: &'static str, pane: &str) -> Event {
        Event { name, args: vec![Value::String(pane.to_string())] }
    }
}

/// What the user did, from the kernel (`Container::Dock`), for the
/// runtime glue to carry out with the program's events.
#[derive(Clone, Debug, PartialEq)]
pub enum User {
    /// A tab or header clicked: its pane shown in its group and active.
    Select(String),
    /// A header button.
    Button(Slot, geometry::Button),
    /// A document tab's close button (OnDocumentClose may keep it).
    CloseDocument(String),
    /// Dropped on a compass button (or chosen with the keyboard).
    Drop(String, Target),
    /// Dropped away from the compass: floats with its window's corner at
    /// (x, y) of the dock manager.
    Float(String, i64, i64),
    /// A splitter dragged: the split's new extents; `done` when let go.
    Split { splitter: usize, extents: Vec<i64>, done: bool },
    /// The auto-hide flyout slid out for a pane, or in.
    Flyout(Option<String>),
    /// The focus moved into a pane (F6): it's the active pane.
    Activate(String),
    /// Ctrl+Tab / Ctrl+Shift+Tab: the next / previous document.
    NextDocument(bool),
    /// A document's tab dropped elsewhere among the documents.
    MoveDocument(String, DocTarget),
    /// A splitter between document groups dragged (the split's path, its
    /// children's new extents); `done` when let go.
    DocSplit { path: Vec<usize>, extents: Vec<i64>, done: bool },
    /// A document's view chosen on its tab strip's switch.
    View(String, DocView),
    /// The splitter between a document's two views dragged: the first's
    /// share per mille; `done` when let go.
    ViewRatio(String, i64, bool),
}

/// What a method, a property or the user's action did, for the runtime:
/// a value (a FUNCTION's), the events to fire, documents to close (each
/// after OnDocumentClose), a pane whose component takes the focus.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Outcome {
    pub value: Option<Value>,
    pub events: Vec<Event>,
    pub closing: Vec<String>,
    pub focus: Option<String>,
    /// The layout changed (OnLayoutChange fires once, last).
    pub changed: bool,
    /// A pane to float: the runtime places its window (the dock manager's
    /// coordinates; `None`: centred on it).
    pub floating: Vec<(String, Option<(i64, i64)>)>,
}

#[derive(Clone, Debug)]
pub struct Manager {
    pub layout: Layout,
    pub panes: Vec<PaneInfo>,
    /// The panes as added (where each went): ResetLayout puts them back.
    initial: Vec<(String, Target)>,
    initial_mode: DocumentMode,
    pub active_pane: Option<String>,
    pub flyout: Option<String>,
    pub ui: Ui,
    /// Its size and font when last laid out (the geometry's).
    pub size: (i64, i64),
    pub font: Font,
    geometry: Option<Geometry>,
}

impl Default for Manager {
    fn default() -> Self {
        Manager {
            layout: Layout::default(),
            panes: Vec::new(),
            initial: Vec::new(),
            initial_mode: DocumentMode::Mdi,
            active_pane: None,
            flyout: None,
            ui: Ui::default(),
            size: (0, 0),
            font: Font::default(),
            geometry: None,
        }
    }
}

thread_local! {
    static DOCKS: RefCell<HashMap<String, Manager>> = RefCell::new(HashMap::new());
}

/// Reads dock manager `id` (`None`: no such one yet).
pub fn with<R>(id: &str, f: impl FnOnce(&Manager) -> R) -> Option<R> {
    DOCKS.with(|d| d.borrow().get(&id.to_ascii_lowercase()).map(f))
}

/// Changes dock manager `id` (made on first use).
pub fn with_mut<R>(id: &str, f: impl FnOnce(&mut Manager) -> R) -> R {
    DOCKS.with(|d| f(d.borrow_mut().entry(id.to_ascii_lowercase()).or_default()))
}

pub fn exists(id: &str) -> bool {
    DOCKS.with(|d| d.borrow().contains_key(&id.to_ascii_lowercase()))
}

/// The document area component of dock manager `dock`.
pub fn docs_name(dock: &str) -> String {
    format!("{}__docs", dock.to_ascii_lowercase())
}

/// Whether `name` is a dock manager's document area (its MDI client): the
/// dock manager it belongs to.
pub fn is_docs_area(name: &str) -> Option<String> {
    let dock = name.to_ascii_lowercase().strip_suffix("__docs")?.to_string();
    exists(&dock).then_some(dock)
}

struct T<'a>(&'a [PaneInfo]);

impl Titles for T<'_> {
    fn title(&self, pane: &str) -> String {
        self.0.iter().find(|p| p.name == pane).map_or_else(|| pane.to_string(), |p| p.title.clone())
    }
    fn icon(&self, pane: &str) -> String {
        self.0.iter().find(|p| p.name == pane).map(|p| p.icon.clone()).unwrap_or_default()
    }
    fn views(&self, pane: &str) -> Vec<String> {
        self.0.iter().find(|p| p.name == pane).map(|p| p.views.iter().map(|v| v.1.clone()).collect()).unwrap_or_default()
    }
    fn view(&self, pane: &str) -> DocView {
        self.0.iter().find(|p| p.name == pane).map(|p| p.view).unwrap_or_default()
    }
    fn view_ratio(&self, pane: &str) -> i64 {
        self.0.iter().find(|p| p.name == pane).map_or(500, |p| p.view_ratio)
    }
    fn modified(&self, pane: &str) -> bool {
        self.0.iter().find(|p| p.name == pane).is_some_and(|p| p.modified)
    }
}

/// Where `AddPane` / `DockPane` put a pane: `left`, `right`, `top`,
/// `bottom` (an outer edge), `documents`, `float`, `tab:<pane>` (into its
/// group), `<side>:<pane>` (beside its group), `<side>:documents`,
/// `autohide:<side>`.
pub fn parse_where(s: &str) -> Option<Target> {
    let s = s.trim().to_ascii_lowercase();
    let (head, rest) = match s.split_once(':') {
        Some((h, r)) => (h.trim().to_string(), Some(r.trim().to_string())),
        None => (s.clone(), None),
    };
    let anchor = |r: &str| if r == "documents" { Anchor::Documents } else { Anchor::Pane(r.to_string()) };
    Some(match (head.as_str(), rest) {
        ("documents" | "document", None) => Target::Into(Anchor::Documents),
        ("float" | "floating", None) => Target::Float((0, 0, 0, 0)),
        ("tab", Some(r)) => Target::Into(anchor(&r)),
        ("autohide", Some(r)) => Target::AutoHide(Side::parse(&r)?),
        (side, None) => Target::Edge(Side::parse(side)?),
        (side, Some(r)) => Target::Beside(anchor(&r), Side::parse(side)?),
    })
}

impl Manager {
    pub fn titles(&self) -> impl Titles + '_ {
        T(&self.panes)
    }

    pub fn pane(&self, name: &str) -> Option<&PaneInfo> {
        self.panes.iter().find(|p| p.name.eq_ignore_ascii_case(name))
    }

    fn pane_mut(&mut self, name: &str) -> Option<&mut PaneInfo> {
        self.panes.iter_mut().find(|p| p.name.eq_ignore_ascii_case(name))
    }

    /// The geometry for the size and font last laid out at.
    pub fn geometry(&mut self) -> &Geometry {
        if self.geometry.is_none() {
            let flyout = self.flyout.as_ref().map(|p| (p.as_str(), self.pane(p).map_or(super::DEFAULT_SIDE, |i| i.extent)));
            let g = geometry::compute(&self.layout, self.size, &T(&self.panes), &self.font, flyout);
            self.geometry = Some(g);
        }
        self.geometry.as_ref().unwrap()
    }

    /// The geometry, read only (computed now if it isn't cached).
    pub fn geometry_now(&self) -> Geometry {
        match &self.geometry {
            Some(g) => g.clone(),
            None => {
                let flyout = self.flyout.as_ref().map(|p| (p.as_str(), self.pane(p).map_or(super::DEFAULT_SIDE, |i| i.extent)));
                geometry::compute(&self.layout, self.size, &T(&self.panes), &self.font, flyout)
            }
        }
    }

    /// Something changed what the geometry says.
    pub fn touch(&mut self) {
        self.geometry = None;
    }

    /// Laid out at a new size or font: whether it changed.
    pub fn resize(&mut self, size: (i64, i64), font: &Font) -> bool {
        if self.size == size && self.font == *font {
            return false;
        }
        self.size = size;
        self.font = font.clone();
        self.touch();
        true
    }

    fn extent_fn(&mut self) -> impl Fn(&[usize], Axis) -> i64 {
        let g = self.geometry().clone();
        move |path: &[usize], axis| g.extent(path, axis)
    }

    /// Panes not placed anywhere (hidden).
    pub fn hidden(&self) -> Vec<String> {
        let placed = self.layout.placed();
        self.panes.iter().filter(|p| !placed.contains(&p.name)).map(|p| p.name.clone()).collect()
    }

    /// A pane's state: `docked`, `tabbed` (docked, in a group of more),
    /// `autohide`, `floating`, `document`, `hidden` ("" for none).
    pub fn state_of(&self, pane: &str) -> &'static str {
        let pane = pane.to_ascii_lowercase();
        if self.pane(&pane).is_none() {
            return "";
        }
        match self.layout.find(&pane) {
            Some(Where::Docked(path, _)) => match self.layout.root.at(&path) {
                Some(Node::Tabs { panes, .. }) if panes.len() > 1 => "tabbed",
                _ => "docked",
            },
            Some(Where::AutoHide(..)) => "autohide",
            Some(Where::Floating(..)) => "floating",
            Some(Where::Document(_)) => "document",
            None => "hidden",
        }
    }

    /// The side a docked pane is on (left of the documents: left …), for
    /// auto-hiding it.
    fn side_of(&mut self, pane: &str) -> Side {
        let g = self.geometry().clone();
        let (Some(gr), Some(d)) = (g.group_of(pane), g.documents.as_ref()) else { return Side::Left };
        let (gx, gy, gw, gh) = gr.rect;
        let (dx, dy, dw, dh) = d.rect;
        if gx + gw <= dx {
            Side::Left
        } else if gx >= dx + dw {
            Side::Right
        } else if gy + gh <= dy {
            Side::Top
        } else if gy >= dy + dh {
            Side::Bottom
        } else {
            Side::Left
        }
    }

    // ------------------------------------------------------- operations --

    /// Puts a pane (not placed now) at `target`; whether it went.
    fn put(&mut self, pane: &str, target: &Target, size: i64) -> bool {
        let extent = self.extent_fn();
        let ok = self.layout.insert(pane, target, size, &extent);
        if !ok {
            // (its anchor's gone: its default edge)
            let side = match target {
                Target::Beside(_, s) => *s,
                _ => Side::Left,
            };
            let extent = self.extent_fn();
            return self.layout.insert(pane, &Target::Edge(side), size, &extent) && self.done();
        }
        self.done()
    }

    fn done(&mut self) -> bool {
        self.touch();
        true
    }

    /// Takes a pane out of the layout, remembering where it was.
    fn take(&mut self, pane: &str) -> Option<Place> {
        let extent = self.extent_fn();
        let place = self.layout.remove(pane, &extent)?;
        if self.flyout.as_deref() == Some(pane) {
            self.flyout = None;
        }
        if let Some(p) = self.pane_mut(pane) {
            p.place = Some(place.clone());
        }
        self.touch();
        Some(place)
    }

    /// AddPane(Component, Title, Where, Icon).
    pub fn add_pane(&mut self, component: &str, title: &str, at: &str, icon: &str) -> Outcome {
        let name = component.to_ascii_lowercase();
        let mut out = Outcome { value: Some(Value::Integer(0)), ..Default::default() };
        if name.is_empty() || self.pane(&name).is_some() {
            return out;
        }
        let Some(target) = parse_where(at) else { return out };
        self.panes.push(PaneInfo::new(&name, title, icon));
        self.initial.push((name.clone(), target.clone()));
        if self.initial.len() == 1 {
            self.initial_mode = self.layout.mode;
        }
        out.value = Some(Value::Integer(-1));
        self.place_at(&name, target, &mut out);
        out
    }

    /// Puts pane `name` (not placed now) at `target` with its events.
    fn place_at(&mut self, name: &str, target: Target, out: &mut Outcome) {
        match target {
            Target::Float(_) => out.floating.push((name.to_string(), None)),
            Target::Into(Anchor::Documents) => {
                self.put(name, &target, 0);
                out.events.push(Event::pane("ondocumentactivate", name));
            }
            t => {
                self.put(name, &t, 0);
            }
        }
        out.events.push(Event::pane("onpanechange", name));
        out.changed = true;
    }

    /// ShowPane: back where it was (or shown in its group, the flyout
    /// slid out for an auto-hidden one).
    pub fn show_pane(&mut self, name: &str) -> Outcome {
        let name = name.to_ascii_lowercase();
        let mut out = Outcome::default();
        let Some(info) = self.pane(&name).cloned() else { return out };
        match self.layout.find(&name) {
            Some(Where::AutoHide(..)) => {
                self.flyout = Some(name.clone());
                self.active_pane = Some(name.clone());
                self.touch();
            }
            Some(_) => {
                if self.layout.select(&name) {
                    self.touch();
                    out.events.push(Event::pane(if self.layout.documents.contains(&name) { "ondocumentactivate" } else { "onpanechange" }, &name));
                }
            }
            None => {
                let target = self.restore_target(&info);
                self.place_at(&name, target, &mut out);
                // (back at its index among the tabs it was with)
                if let Some(Place::Tab(_, i)) = &info.place {
                    self.layout.move_tab(&name, *i);
                }
            }
        }
        out
    }

    /// Where a pane not placed goes back to.
    fn restore_target(&self, info: &PaneInfo) -> Target {
        let docked = |p: &str| self.layout.is_docked(p);
        info.place.as_ref().and_then(|pl| pl.target(&docked)).map(|(t, _)| t).unwrap_or_else(|| {
            self.initial.iter().find(|(p, _)| *p == info.name).map(|(_, t)| t.clone()).unwrap_or(Target::Edge(Side::Left))
        })
    }

    /// The extent a restore target remembers.
    fn restore_size(&self, info: &PaneInfo) -> i64 {
        match &info.place {
            Some(Place::Beside(_, _, s)) | Some(Place::Edge(_, s)) => *s,
            _ => 0,
        }
    }

    /// HidePane.
    pub fn hide_pane(&mut self, name: &str) -> Outcome {
        let name = name.to_ascii_lowercase();
        let mut out = Outcome::default();
        if self.pane(&name).is_none() || self.layout.find(&name).is_none() {
            return out;
        }
        let was_doc = self.layout.documents.contains(&name);
        self.take(&name);
        if self.active_pane.as_deref() == Some(&name) {
            self.active_pane = None;
        }
        out.events.push(Event::pane("onpanechange", &name));
        if was_doc {
            if let Some(a) = self.active_document() {
                out.events.push(Event::pane("ondocumentactivate", &a));
            }
        }
        out.changed = true;
        out
    }

    /// FloatPane (`at`: where its window goes, in the dock manager's
    /// coordinates; `None`: centred).
    pub fn float_pane(&mut self, name: &str, at: Option<(i64, i64)>) -> Outcome {
        let name = name.to_ascii_lowercase();
        let mut out = Outcome::default();
        if self.pane(&name).is_none() || matches!(self.layout.find(&name), Some(Where::Floating(..))) {
            return out;
        }
        self.take(&name);
        out.floating.push((name.clone(), at));
        out.events.push(Event::pane("onpanechange", &name));
        out.changed = true;
        out
    }

    /// The runtime made a floating window for `name` at `rect` (screen).
    pub fn floated(&mut self, name: &str, rect: Rect) {
        let extent = self.extent_fn();
        self.layout.insert(name, &Target::Float(rect), 0, &extent);
        if let Some(p) = self.pane_mut(name) {
            p.float_size = (rect.2, rect.3);
        }
        self.touch();
    }

    /// AutoHide(Name, On).
    pub fn auto_hide(&mut self, name: &str, on: bool) -> Outcome {
        let name = name.to_ascii_lowercase();
        let mut out = Outcome::default();
        if self.pane(&name).is_none() {
            return out;
        }
        let state = self.layout.find(&name);
        match (on, state) {
            (true, Some(Where::Docked(..))) => {
                let side = self.side_of(&name);
                let extent = self.geometry().group_of(&name).map(|g| if side.axis() == Axis::Row { g.rect.2 } else { g.rect.3 });
                self.take(&name);
                if let Some(e) = extent {
                    if let Some(p) = self.pane_mut(&name) {
                        p.extent = e;
                    }
                }
                self.layout.autohide[side.index()].push(name.clone());
                self.touch();
            }
            (false, Some(Where::AutoHide(..))) => {
                let place = self.pane(&name).and_then(|p| p.place.clone());
                self.take(&name);
                // (back where it was before it hid, not on the strip)
                if let Some(p) = self.pane_mut(&name) {
                    p.place = place;
                }
                let info = self.pane(&name).cloned().unwrap();
                let target = self.restore_target(&info);
                let size = self.restore_size(&info);
                if !matches!(target, Target::AutoHide(_)) {
                    self.put(&name, &target, size);
                } else {
                    self.put(&name, &Target::Edge(Side::Left), 0);
                }
            }
            _ => return out,
        }
        out.events.push(Event::pane("onpanechange", &name));
        out.changed = true;
        out
    }

    /// DockPane(Name, Where): moved to `target`.
    pub fn dock_pane(&mut self, name: &str, target: Target) -> Outcome {
        let name = name.to_ascii_lowercase();
        let mut out = Outcome::default();
        if self.pane(&name).is_none() {
            return out;
        }
        // (onto itself: nothing)
        if let Target::Into(Anchor::Pane(p)) | Target::Beside(Anchor::Pane(p), _) = &target {
            if *p == name && self.layout.find(&name).is_some() {
                let alone = matches!(self.layout.find(&name), Some(Where::Docked(path, _)) if matches!(self.layout.root.at(&path), Some(Node::Tabs { panes, .. }) if panes.len() == 1));
                if alone || matches!(target, Target::Into(_)) {
                    return out;
                }
            }
        }
        let was_doc = self.layout.documents.contains(&name);
        // (beside its own group: the group's other pane anchors it)
        let target = match target {
            Target::Beside(Anchor::Pane(p), side) if p == name => match self.layout.find(&name) {
                Some(Where::Docked(path, i)) => match self.layout.root.at(&path) {
                    Some(Node::Tabs { panes, .. }) => Target::Beside(Anchor::Pane(panes[if i == 0 { 1 } else { 0 }].clone()), side),
                    _ => return out,
                },
                _ => return out,
            },
            t => t,
        };
        self.take(&name);
        self.place_at(&name, target, &mut out);
        if was_doc && !self.layout.documents.contains(&name) {
            if let Some(a) = self.active_document() {
                out.events.push(Event::pane("ondocumentactivate", &a));
            }
        }
        out
    }

    /// FocusPane: shown and active; its component takes the focus.
    pub fn focus_pane(&mut self, name: &str) -> Outcome {
        let name = name.to_ascii_lowercase();
        // (one of a document's views: that view shown — still side by side
        // when it's one of the two — its document active, it focused)
        if let Some(doc) = self.document_of_view(&name) {
            let (idx, split) = match self.pane(&doc) {
                Some(i) => (i.views.iter().position(|v| v.0 == name), i.view == DocView::Split),
                None => (None, false),
            };
            if let Some(idx) = idx {
                let mut out = if split && idx < 2 {
                    let mut o = Outcome::default();
                    if self.layout.select(&doc) {
                        o.events.push(Event::pane("ondocumentactivate", &doc));
                    }
                    self.touch();
                    o
                } else {
                    self.set_view(&doc, if idx == 0 { DocView::First } else { DocView::One(idx) })
                };
                out.focus = Some(name);
                return out;
            }
        }
        let mut out = self.show_pane(&name);
        if self.pane(&name).is_none() {
            return out;
        }
        if self.layout.documents.contains(&name) {
            self.layout.select(&name);
        } else if self.active_pane.as_deref() != Some(&name) {
            self.active_pane = Some(name.clone());
            if !out.events.iter().any(|e| e.name == "onpanechange") {
                out.events.push(Event::pane("onpanechange", &name));
            }
        }
        self.touch();
        out.focus = Some(name);
        out
    }

    /// ClosePane: a document closes (asking the program), a tool pane
    /// hides.
    pub fn close_pane(&mut self, name: &str) -> Outcome {
        let name = name.to_ascii_lowercase();
        if self.layout.documents.contains(&name) {
            return Outcome { closing: vec![name], ..Default::default() };
        }
        self.hide_pane(&name)
    }

    /// A document goes (after OnDocumentClose let it).
    pub fn close_document(&mut self, name: &str) -> Outcome {
        let mut out = Outcome::default();
        if !self.layout.documents.iter().any(|d| d == name) {
            return out;
        }
        let was_active = self.active_document().as_deref() == Some(name);
        // (the event names it as the program did: it's gone after this)
        let given = self.pane(name).map_or_else(|| name.to_string(), |p| p.given.clone());
        let extent = self.extent_fn();
        self.layout.remove(name, &extent);
        self.panes.retain(|p| p.name != name);
        self.initial.retain(|(p, _)| p != name);
        self.touch();
        if was_active {
            if let Some(a) = self.active_document() {
                out.events.push(Event::pane("ondocumentactivate", &a));
            }
        }
        out.events.push(Event::pane("onpanechange", &given));
        out.changed = true;
        out
    }

    pub fn active_document(&self) -> Option<String> {
        self.layout.active_document.and_then(|i| self.layout.documents.get(i).cloned())
    }

    /// ActiveDocument = name.
    pub fn activate_document(&mut self, name: &str) -> Outcome {
        let name = name.to_ascii_lowercase();
        let mut out = Outcome::default();
        if self.layout.documents.contains(&name) && self.layout.select(&name) {
            self.touch();
            out.events.push(Event::pane("ondocumentactivate", &name));
        }
        out
    }

    /// The next (previous) document.
    pub fn next_document(&mut self, back: bool) -> Outcome {
        let n = self.layout.documents.len();
        if n < 2 {
            return Outcome::default();
        }
        let a = self.layout.active_document.unwrap_or(0);
        let next = if back { (a + n - 1) % n } else { (a + 1) % n };
        let name = self.layout.documents[next].clone();
        let mut out = self.activate_document(&name);
        out.focus = Some(name);
        out
    }

    /// SaveLayout: the layout's text, then each document's view when it
    /// isn't its first, or its side-by-side share when it was moved
    /// (`view <document> <first | n | split> <per mille>`).
    pub fn save(&self) -> String {
        let mut out = self.layout.save(&self.hidden());
        for d in &self.layout.documents {
            if let Some(p) = self.pane(d).filter(|p| !p.views.is_empty() && (p.view.index() != Some(0) || p.view_ratio != 500)) {
                let v = match p.view.index() {
                    None => "split".to_string(),
                    Some(0) => "first".to_string(),
                    Some(i) => i.to_string(),
                };
                out.push_str(&format!("view {d} {v} {}\n", p.view_ratio));
            }
        }
        out
    }

    /// LoadLayout: whether the text was a layout. The documents' views it
    /// names are shown (OnDocumentView for each that changes).
    pub fn load(&mut self, text: &str) -> Outcome {
        let known: Vec<String> = self.panes.iter().map(|p| p.name.clone()).collect();
        let views: Vec<Vec<String>> = text.lines().filter(|l| l.starts_with("view ")).map(|l| l.split_whitespace().map(str::to_string).collect()).collect();
        let rest: String = text.lines().filter(|l| !l.starts_with("view ")).map(|l| format!("{l}\n")).collect();
        let Ok((mut layout, _hidden)) = Layout::load(&rest, &|p| known.iter().any(|k| k == p)) else {
            return Outcome { value: Some(Value::Integer(0)), ..Default::default() };
        };
        let mut events = Vec::new();
        for w in views.iter().filter(|w| w.len() >= 3) {
            let doc = w[1].to_ascii_lowercase();
            let Some(info) = self.pane_mut(&doc) else { continue };
            let v = match w[2].as_str() {
                "split" if info.views.len() >= 2 => DocView::Split,
                "first" => DocView::First,
                n => match n.parse::<usize>() {
                    Ok(i) if i < info.views.len() => if i == 0 { DocView::First } else { DocView::One(i) },
                    _ => continue,
                },
            };
            if let Some(r) = w.get(3).and_then(|r| r.parse::<i64>().ok()) {
                info.view_ratio = r.clamp(100, 900);
            }
            if info.view.index() != v.index() {
                info.view = v;
                let caption = info.view_caption();
                events.push(Event { name: "ondocumentview", args: vec![Value::String(doc.clone()), Value::String(caption)] });
            }
        }
        // (documents it doesn't list stay open, after the ones it lists)
        for d in &self.layout.documents {
            if !layout.documents.contains(d) && layout.find(d).is_none() {
                layout.add_document(d);
            }
        }
        if layout.active_document.is_none() && !layout.documents.is_empty() {
            layout.select_document(0);
        }
        self.layout = layout;
        self.flyout = None;
        self.ui = Ui::default();
        self.touch();
        Outcome { value: Some(Value::Integer(-1)), changed: true, events, ..Default::default() }
    }

    /// ResetLayout: the panes where AddPane put them.
    pub fn reset(&mut self) -> Outcome {
        let docs = self.layout.documents.clone();
        let active = self.active_document();
        let floats: Vec<String> = self.initial.iter().filter(|(_, t)| matches!(t, Target::Float(_))).map(|(p, _)| p.clone()).collect();
        self.layout = Layout { mode: self.initial_mode, ..Layout::default() };
        self.flyout = None;
        self.ui = Ui::default();
        self.touch();
        let initial = self.initial.clone();
        let mut out = Outcome { changed: true, ..Default::default() };
        for (p, t) in initial {
            match t {
                Target::Float(_) => {}
                Target::Into(Anchor::Documents) => {}
                t => {
                    self.put(&p, &t, 0);
                }
            }
        }
        for d in docs {
            self.put(&d, &Target::Into(Anchor::Documents), 0);
        }
        if let Some(a) = active {
            self.layout.select(&a);
        }
        out.floating = floats.into_iter().map(|p| (p, None)).collect();
        out
    }

    // --------------------------------------------------- the user's acts --

    /// A user's action (the kernel's), as the model's.
    pub fn user(&mut self, action: User) -> Outcome {
        match action {
            User::Select(p) => {
                let mut out = Outcome::default();
                let doc = self.layout.documents.contains(&p);
                let changed = self.layout.select(&p);
                if doc {
                    if changed {
                        out.events.push(Event::pane("ondocumentactivate", &p));
                    }
                } else {
                    let was = self.active_pane.replace(p.clone());
                    if changed || was.as_deref() != Some(&p) {
                        out.events.push(Event::pane("onpanechange", &p));
                    }
                    // (another pane chosen: the flyout slides in)
                    if self.flyout.as_ref().is_some_and(|f| *f != p) {
                        self.flyout = None;
                    }
                }
                self.touch();
                out.focus = Some(p);
                out
            }
            User::Activate(p) => {
                let mut out = Outcome::default();
                if self.layout.documents.contains(&p) {
                    if self.layout.select(&p) {
                        out.events.push(Event::pane("ondocumentactivate", &p));
                    }
                } else if self.active_pane.as_deref() != Some(&p) {
                    self.active_pane = Some(p.clone());
                    out.events.push(Event::pane("onpanechange", &p));
                }
                self.touch();
                out
            }
            User::Button(slot, b) => {
                let pane = match slot {
                    Slot::Flyout => self.flyout.clone(),
                    Slot::Docked(i) => self.geometry().groups.get(i).map(|g| g.panes[g.active].clone()),
                    Slot::Float(i) => self.layout.floating.get(i).map(|f| f.panes[f.active].clone()),
                };
                let Some(pane) = pane else { return Outcome::default() };
                match (slot, b) {
                    (_, geometry::Button::Close) => self.hide_pane(&pane),
                    (Slot::Flyout, geometry::Button::Pin) => self.auto_hide(&pane, false),
                    (_, geometry::Button::Pin) => {
                        // (the whole group goes to the strip, as Visual Studio's pin)
                        let group: Vec<String> = self.geometry().group_of(&pane).map(|g| g.panes.clone()).unwrap_or_default();
                        let mut out = Outcome::default();
                        for p in group {
                            let o = self.auto_hide(&p, true);
                            out.events.extend(o.events);
                            out.changed |= o.changed;
                        }
                        out
                    }
                    (Slot::Float(i), geometry::Button::Dock) => {
                        let group: Vec<String> = self.layout.floating.get(i).map(|f| f.panes.clone()).unwrap_or_default();
                        self.dock_back(group)
                    }
                    _ => Outcome::default(),
                }
            }
            User::CloseDocument(p) => Outcome { closing: vec![p], ..Default::default() },
            User::Drop(p, t) => self.dock_pane(&p, t),
            User::Float(p, x, y) => self.float_pane(&p, Some((x, y))),
            User::Split { splitter, extents, done } => {
                let Some(s) = self.geometry().splitters.get(splitter).cloned() else { return Outcome::default() };
                if let Some(Node::Split { sizes, .. }) = self.layout.root.at_mut(&s.path) {
                    if sizes.len() == extents.len() {
                        *sizes = extents;
                    }
                }
                self.touch();
                Outcome { changed: done, ..Default::default() }
            }
            User::Flyout(p) => {
                if self.flyout != p {
                    self.flyout = p;
                    self.touch();
                }
                let focus = self.flyout.clone();
                if let Some(f) = &focus {
                    self.active_pane = Some(f.clone());
                }
                Outcome { focus, ..Default::default() }
            }
            User::NextDocument(back) => self.next_document(back),
            User::MoveDocument(p, t) => {
                let mut out = Outcome::default();
                let was = self.active_document();
                if self.layout.move_document(&p, &t) {
                    self.touch();
                    if was.as_deref() != Some(p.as_str()) {
                        out.events.push(Event::pane("ondocumentactivate", &p));
                    }
                    out.changed = true;
                    out.focus = Some(p);
                }
                out
            }
            User::DocSplit { path, extents, done } => {
                self.layout.set_doc_weights(&path, &extents);
                self.touch();
                Outcome { changed: done, ..Default::default() }
            }
            User::View(p, v) => self.set_view(&p, v),
            User::ViewRatio(p, r, done) => {
                if let Some(i) = self.pane_mut(&p) {
                    i.view_ratio = r.clamp(100, 900);
                }
                self.touch();
                Outcome { changed: done, ..Default::default() }
            }
        }
    }

    // ------------------------------------------------- document views --

    /// AddView(Document, Component, Caption): `component` is one of the
    /// document's views (the document's own component too, when given).
    pub fn add_view(&mut self, doc: &str, component: &str, caption: &str) -> Outcome {
        let (doc, comp) = (doc.to_ascii_lowercase(), component.to_ascii_lowercase());
        let Some(info) = self.pane_mut(&doc) else { return Outcome::default() };
        match info.views.iter_mut().find(|v| v.0 == comp) {
            Some(v) => v.1 = caption.to_string(),
            None => info.views.push((comp, caption.to_string())),
        }
        self.touch();
        Outcome { changed: true, ..Default::default() }
    }

    /// The view `doc` shows (`v`): OnDocumentView (Name, View) when it
    /// changed; the shown view's component takes the focus.
    pub fn set_view(&mut self, doc: &str, v: DocView) -> Outcome {
        let doc = doc.to_ascii_lowercase();
        let mut out = Outcome::default();
        let Some(info) = self.pane_mut(&doc) else { return out };
        let n = info.views.len();
        let v = match v {
            DocView::One(i) if i >= n => return out,
            DocView::Split if n < 2 => return out,
            v => v,
        };
        let same = info.view.index() == v.index();
        info.view = v;
        let caption = info.view_caption();
        let focus = info.shown_components().last().cloned();
        self.touch();
        if !same {
            out.events.push(Event { name: "ondocumentview", args: vec![Value::String(doc.clone()), Value::String(caption)] });
            out.changed = true;
        }
        // (its document shown and active too)
        if self.layout.documents.contains(&doc) && self.layout.select(&doc) {
            out.events.push(Event::pane("ondocumentactivate", &doc));
        }
        out.focus = focus;
        out
    }

    /// A view by its caption ("Split" / "Both": side by side).
    pub fn view_named(&self, doc: &str, caption: &str) -> Option<DocView> {
        let info = self.pane(doc)?;
        let c = caption.trim();
        if c.eq_ignore_ascii_case("split") || c.eq_ignore_ascii_case("both") {
            return Some(DocView::Split);
        }
        info.views.iter().position(|v| v.1.eq_ignore_ascii_case(c)).map(|i| if i == 0 { DocView::First } else { DocView::One(i) })
    }

    /// SplitDocument(Name, Side): a new group on that side of its group,
    /// the document in it.
    pub fn split_document(&mut self, doc: &str, side: Side) -> Outcome {
        let doc = doc.to_ascii_lowercase();
        if !self.layout.documents.contains(&doc) {
            return Outcome::default();
        }
        self.user(User::MoveDocument(doc.clone(), DocTarget::Split { anchor: doc, side }))
    }

    /// The document whose views include `component` (or that is it).
    pub fn document_of_view(&self, component: &str) -> Option<String> {
        let c = component.to_ascii_lowercase();
        self.panes.iter().find(|p| self.layout.documents.contains(&p.name) && (p.name == c || p.views.iter().any(|v| v.0 == c))).map(|p| p.name.clone())
    }

    /// A floating group's panes back where they were docked.
    fn dock_back(&mut self, group: Vec<String>) -> Outcome {
        let mut out = Outcome::default();
        let mut first: Option<String> = None;
        for p in group {
            let before = self.pane(&p).and_then(|i| i.place.clone());
            self.take(&p);
            let target = match &first {
                // (the others tab in with the first)
                Some(f) => Target::Into(Anchor::Pane(f.clone())),
                None => {
                    let info = PaneInfo { place: before, ..self.pane(&p).cloned().unwrap() };
                    match self.restore_target(&info) {
                        Target::Float(_) => Target::Edge(Side::Left),
                        t => t,
                    }
                }
            };
            let size = self.pane(&p).map(|i| self.restore_size(i)).unwrap_or(0);
            self.put(&p, &target, size);
            first.get_or_insert(p.clone());
            out.events.push(Event::pane("onpanechange", &p));
        }
        out.changed = true;
        out
    }

    /// A floating window closed by its close box: its panes are hidden.
    pub fn float_closed(&mut self, index: usize) -> Outcome {
        let panes = self.layout.floating.get(index).map(|f| f.panes.clone()).unwrap_or_default();
        let mut out = Outcome::default();
        for p in panes {
            let o = self.hide_pane(&p);
            out.events.extend(o.events);
            out.changed |= o.changed;
        }
        out
    }

    /// A floating window moved or resized by the user.
    pub fn float_moved(&mut self, index: usize, rect: Rect) {
        if let Some(f) = self.layout.floating.get_mut(index) {
            f.rect = rect;
        }
    }

    // ------------------------------------------------- the keyboard's move --

    /// The "move pane" command: the compass over its own area to begin.
    pub fn begin_move(&mut self, name: &str) -> bool {
        let name = name.to_ascii_lowercase();
        if self.pane(&name).is_none() || self.layout.documents.contains(&name) {
            return false;
        }
        let g = self.geometry().clone();
        let areas = g.areas();
        let own = g.group_of(&name).and_then(|gr| areas.iter().position(|(_, r)| *r == gr.rect));
        // (its own lone group has no compass: the documents' to begin)
        let alone = g.group_of(&name).is_some_and(|gr| gr.single());
        let area = match own {
            Some(a) if !alone => a,
            _ => areas.iter().position(|(a, _)| *a == Anchor::Documents).unwrap_or(0),
        };
        let guide = if alone || own.is_none() { Guide::Center } else { Guide::Arm(Side::Right) };
        self.ui.moving = Some(KeyMove { pane: name, area, guide });
        true
    }

    /// The keyboard's move's target now.
    pub fn move_target(&self) -> Option<Target> {
        let m = self.ui.moving.as_ref()?;
        let g = self.geometry_now();
        g.compass(&m.pane, None, Some(m.area)).into_iter().find(|(gd, _, _)| *gd == m.guide).map(|(_, _, t)| t)
    }

    /// A key while moving: arrows choose the compass's button (the same
    /// arrow again: the outer edge), Tab the next area, Enter docks, F
    /// floats, Escape stops. Whether it took the key, and what to do.
    pub fn move_key(&mut self, vk: i64, shift: bool) -> (bool, Option<User>) {
        let Some(m) = self.ui.moving.clone() else { return (false, None) };
        let g = self.geometry_now();
        let n = g.areas().len().max(1);
        let arrow = match vk {
            37 => Some(Side::Left),
            38 => Some(Side::Top),
            39 => Some(Side::Right),
            40 => Some(Side::Bottom),
            _ => None,
        };
        let valid = |area: usize, guide: Guide| g.compass(&m.pane, None, Some(area)).iter().any(|(gd, _, _)| *gd == guide);
        let mut next = m.clone();
        match vk {
            27 => {
                self.ui.moving = None;
                return (true, None);
            }
            13 => {
                self.ui.moving = None;
                return (true, self.target_for(&m, &g).map(|t| User::Drop(m.pane.clone(), t)));
            }
            70 => {
                self.ui.moving = None;
                let (w, h) = self.size;
                return (true, Some(User::Float(m.pane.clone(), w / 4, h / 4)));
            }
            9 | 117 => {
                // (Tab / F6: the next area with a compass)
                for k in 1..=n {
                    let a = if shift { (m.area + n * 2 - k) % n } else { (m.area + k) % n };
                    let guide = if valid(a, m.guide) { m.guide } else { Guide::Center };
                    if valid(a, guide) || valid(a, Guide::Arm(Side::Left)) {
                        next.area = a;
                        next.guide = if valid(a, guide) { guide } else { Guide::Arm(Side::Left) };
                        break;
                    }
                }
            }
            32 | 36 => {
                if valid(m.area, Guide::Center) {
                    next.guide = Guide::Center;
                }
            }
            _ => match arrow {
                Some(side) => {
                    next.guide = match m.guide {
                        Guide::Arm(s) if s == side => Guide::Outer(side),
                        Guide::Outer(s) if s == side => Guide::Outer(side),
                        _ if valid(m.area, Guide::Arm(side)) => Guide::Arm(side),
                        _ => Guide::Outer(side),
                    };
                }
                None => return (true, None),
            },
        }
        self.ui.moving = Some(next);
        (true, None)
    }

    fn target_for(&self, m: &KeyMove, g: &Geometry) -> Option<Target> {
        g.compass(&m.pane, None, Some(m.area)).into_iter().find(|(gd, _, _)| *gd == m.guide).map(|(_, _, t)| t)
    }

    /// The areas F6 visits in order: each docked group (its active pane),
    /// then the documents (the active one); the open flyout first.
    pub fn areas_for_keys(&mut self) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(f) = &self.flyout {
            out.push(f.clone());
        }
        let g = self.geometry().clone();
        for gr in &g.groups {
            out.push(gr.panes[gr.active].clone());
        }
        if let Some(d) = self.active_document() {
            out.push(d);
        }
        out
    }

    /// Splitter `i` dragged `delta` pixels from where it was held: the
    /// split's new extents.
    pub fn split_extents(&mut self, i: usize, start: &[i64], delta: i64) -> Option<Vec<i64>> {
        let s = self.geometry().splitters.get(i)?.clone();
        let Some(Node::Split { children, .. }) = self.layout.root.at(&s.path) else { return None };
        let flex = Node::flex(children);
        let mut ext = start.to_vec();
        let (a, b) = (s.index, s.index + 1);
        let min = geometry::MIN_EXTENT;
        // (what the two sides can give)
        let delta = delta.clamp(-(ext[a] - min).max(0), (ext[b] - min).max(0));
        ext[a] += delta;
        ext[b] -= delta;
        let _ = flex;
        Some(ext)
    }
}

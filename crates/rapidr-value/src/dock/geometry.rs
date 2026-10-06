//! Where everything of a dock manager goes for a size: the auto-hide
//! strips on its edges and their tabs, each docked group (its header,
//! tabs, buttons, the place of its pane's component), the splitters
//! between them, the document area (its tabs in tabbed mode), an open
//! auto-hide flyout; the docking compass while a pane is dragged; and what
//! is under a point. Theme-free: every theme draws the same places, so the
//! components sit where they sit whatever the look (logical pixels).

use super::{Anchor, Axis, DocumentMode, Layout, Node, Rect, Side, Target};
use crate::objects::font::Font;
use crate::objects::text::text_size;

/// Between two docked neighbours: the splitter you drag.
pub const GAP: i64 = 4;
/// The least a docked group is along a split.
pub const MIN_EXTENT: i64 = 60;
/// A group's header (its title or tabs, its buttons).
pub const HEADER: i64 = 26;
/// The tabbed document area's tab strip.
pub const DOC_TABS: i64 = 30;
/// An auto-hide strip's thickness.
pub const STRIP: i64 = 24;
/// A header button's side.
pub const BUTTON: i64 = 20;
/// An icon's side.
pub const ICON: i64 = 16;
/// A compass button's side.
pub const GUIDE: i64 = 32;

/// Which group: a docked one (by its order in [`Geometry::groups`]), a
/// floating window's, or the open auto-hide flyout.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Slot {
    Docked(usize),
    Float(usize),
    Flyout,
}

impl Slot {
    /// Its component's name after the dock manager's (`<dock>__g0` …).
    pub fn suffix(self) -> String {
        match self {
            Slot::Docked(i) => format!("g{i}"),
            Slot::Float(i) => format!("fg{i}"),
            Slot::Flyout => "fly".into(),
        }
    }

    pub fn parse(s: &str) -> Option<Slot> {
        let s = s.to_ascii_lowercase();
        if s == "fly" {
            return Some(Slot::Flyout);
        }
        if let Some(n) = s.strip_prefix("fg") {
            return n.parse().ok().map(Slot::Float);
        }
        s.strip_prefix('g')?.parse().ok().map(Slot::Docked)
    }
}

/// A group header's buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Button {
    /// Auto-hide on (a docked group's pin) or off (the flyout's).
    Pin,
    /// Back into the layout (a floating group's).
    Dock,
    /// Hides the active pane.
    Close,
}

impl Button {
    pub fn name(self) -> &'static str {
        match self {
            Button::Pin => "Auto Hide",
            Button::Dock => "Dock",
            Button::Close => "Close",
        }
    }
}

/// A tab (a group's, the documents'): its pane, its place (relative to
/// its group / the document area) and its close button's.
#[derive(Clone, Debug, PartialEq)]
pub struct Tab {
    pub pane: String,
    pub rect: Rect,
    pub close: Option<Rect>,
}

/// A group of panes: where it is (in the dock manager's coordinates; a
/// floating one's in its window's), and in its own coordinates its
/// header's tabs (one tab shows as a title), buttons and the place of the
/// active pane's component.
#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    pub slot: Slot,
    pub panes: Vec<String>,
    pub active: usize,
    pub rect: Rect,
    pub tabs: Vec<Tab>,
    pub buttons: Vec<(Button, Rect)>,
    pub content: Rect,
    /// The path to its node (docked groups).
    pub path: Vec<usize>,
}

impl Group {
    /// One pane: its header is a title, not tabs.
    pub fn single(&self) -> bool {
        self.panes.len() == 1
    }
}

/// The document area: where it is, its tabs (tabbed mode) and where the
/// documents go (both relative to it).
#[derive(Clone, Debug, PartialEq)]
pub struct Documents {
    pub rect: Rect,
    pub tabs: Vec<Tab>,
    pub content: Rect,
    pub path: Vec<usize>,
}

/// A splitter: between children `index` and `index + 1` of the split at
/// `path`.
#[derive(Clone, Debug, PartialEq)]
pub struct Splitter {
    pub path: Vec<usize>,
    pub index: usize,
    pub axis: Axis,
    pub rect: Rect,
}

/// An auto-hidden pane's tab on its strip.
#[derive(Clone, Debug, PartialEq)]
pub struct StripTab {
    pub pane: String,
    pub side: Side,
    pub rect: Rect,
}

/// What a pane looks like in a tab: its title and icon name.
pub trait Titles {
    fn title(&self, pane: &str) -> String;
    fn icon(&self, pane: &str) -> String;
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Geometry {
    pub size: (i64, i64),
    /// The docked layout's area (inside the strips).
    pub inner: Rect,
    pub groups: Vec<Group>,
    pub documents: Option<Documents>,
    pub splitters: Vec<Splitter>,
    /// The strips (by side) and their tabs.
    pub strips: Vec<(Side, Rect)>,
    pub strip_tabs: Vec<StripTab>,
    pub flyout: Option<Group>,
    /// Each split's children's extents as laid out (by the split's path).
    pub extents: Vec<(Vec<usize>, Vec<i64>)>,
    /// Every node's place (by path).
    pub nodes: Vec<(Vec<usize>, Rect)>,
}

/// A tab's width for `title` (with an icon, a close button).
fn tab_width(title: &str, font: &Font, icon: bool, close: bool, pad: i64) -> i64 {
    let (w, _) = text_size(title, font);
    w + 2 * pad + if icon { ICON + 6 } else { 0 } + if close { BUTTON } else { 0 }
}

/// A group's header and content in its own coordinates, `w` × `h`.
pub fn group(slot: Slot, panes: &[String], active: usize, rect: Rect, path: Vec<usize>, titles: &dyn Titles, font: &Font) -> Group {
    let (w, h) = (rect.2, rect.3);
    let mut buttons = Vec::new();
    let kinds: &[Button] = match slot {
        Slot::Docked(_) => &[Button::Close, Button::Pin],
        Slot::Flyout => &[Button::Close, Button::Pin],
        Slot::Float(_) => &[Button::Close, Button::Dock],
    };
    let mut x = w - 4;
    for &b in kinds {
        x -= BUTTON;
        buttons.push((b, (x, (HEADER - BUTTON) / 2, BUTTON, BUTTON)));
        x -= 2;
    }
    buttons.reverse();
    let room = (x - 4).max(0);
    let mut tabs = Vec::new();
    if panes.len() == 1 {
        tabs.push(Tab { pane: panes[0].clone(), rect: (0, 0, room + 4, HEADER), close: None });
    } else {
        let widths: Vec<i64> = panes.iter().map(|p| tab_width(&titles.title(p), font, !titles.icon(p).is_empty(), false, 10)).collect();
        let total: i64 = widths.iter().sum();
        // (too many to fit: each gives up room in proportion, none under 40)
        let fit = |wd: i64| if total > room && total > 0 { (wd * room / total).max(40) } else { wd };
        let mut x = 4;
        for (p, wd) in panes.iter().zip(widths) {
            let wd = fit(wd);
            tabs.push(Tab { pane: p.clone(), rect: (x, 0, wd, HEADER), close: None });
            x += wd;
        }
    }
    // (flush with the group's edges, over the header's bottom line: the
    // pane's own border is the group's)
    let content = (0, HEADER - 1, w, (h - HEADER + 1).max(0));
    Group { slot, panes: panes.to_vec(), active: active.min(panes.len().saturating_sub(1)), rect, tabs, buttons, content, path }
}

/// The document area's tabs (tabbed mode) and content, `rect` its place.
fn documents(layout: &Layout, rect: Rect, path: Vec<usize>, titles: &dyn Titles, font: &Font) -> Documents {
    let (w, h) = (rect.2, rect.3);
    if layout.mode == DocumentMode::Mdi || layout.documents.is_empty() {
        return Documents { rect, tabs: Vec::new(), content: (0, 0, w, h), path };
    }
    let mut x = 0;
    let mut tabs = Vec::new();
    for p in &layout.documents {
        let wd = tab_width(&titles.title(p), font, !titles.icon(p).is_empty(), true, 12).min(260);
        let close = (x + wd - BUTTON - 6, (DOC_TABS - BUTTON) / 2 + 1, BUTTON - 2, BUTTON - 2);
        tabs.push(Tab { pane: p.clone(), rect: (x, 0, wd, DOC_TABS), close: Some(close) });
        x += wd + 1;
    }
    Documents { rect, tabs, content: (0, DOC_TABS, w, (h - DOC_TABS).max(0)), path }
}

/// Extents along a split for `avail` pixels: each child its size (at
/// least [`MIN_EXTENT`]), the flex child the rest; all of them shrunk
/// alike when there isn't room.
pub fn split_extents(children: &[Node], sizes: &[i64], avail: i64) -> Vec<i64> {
    let n = children.len();
    if n == 0 {
        return Vec::new();
    }
    let flex = Node::flex(children);
    let mut ext: Vec<i64> = sizes.iter().map(|&s| s.max(MIN_EXTENT)).collect();
    ext.resize(n, MIN_EXTENT);
    let fixed: i64 = ext.iter().enumerate().filter(|&(i, _)| i != flex).map(|(_, e)| e).sum();
    let left_for_flex = avail - fixed;
    if left_for_flex >= MIN_EXTENT || avail <= 0 {
        ext[flex] = left_for_flex.max(0);
    } else {
        // (the fixed ones shrink so the flex keeps its least)
        let room = (avail - MIN_EXTENT.min(avail / n as i64)).max(0);
        let mut given = 0;
        for (i, e) in ext.iter_mut().enumerate() {
            if i != flex {
                *e = if fixed > 0 { *e * room / fixed } else { 0 };
                given += *e;
            }
        }
        ext[flex] = (avail - given).max(0);
    }
    ext
}

struct Builder<'a> {
    layout: &'a Layout,
    titles: &'a dyn Titles,
    font: &'a Font,
    g: Geometry,
}

impl Builder<'_> {
    fn place(&mut self, node: &Node, rect: Rect, path: Vec<usize>) {
        self.g.nodes.push((path.clone(), rect));
        match node {
            Node::Tabs { panes, active } => {
                let slot = Slot::Docked(self.g.groups.len());
                let gr = group(slot, panes, *active, rect, path, self.titles, self.font);
                self.g.groups.push(gr);
            }
            Node::Documents => self.g.documents = Some(documents(self.layout, rect, path, self.titles, self.font)),
            Node::Split { axis, children, sizes } => {
                let (x, y, w, h) = rect;
                let along = if *axis == Axis::Row { w } else { h };
                let avail = along - GAP * (children.len() as i64 - 1);
                let ext = split_extents(children, sizes, avail);
                self.g.extents.push((path.clone(), ext.clone()));
                let mut at = 0;
                for (i, (c, e)) in children.iter().zip(&ext).enumerate() {
                    let r = if *axis == Axis::Row { (x + at, y, *e, h) } else { (x, y + at, w, *e) };
                    let mut p = path.clone();
                    p.push(i);
                    self.place(c, r, p);
                    at += e;
                    if i + 1 < children.len() {
                        let s = if *axis == Axis::Row { (x + at, y, GAP, h) } else { (x, y + at, w, GAP) };
                        self.g.splitters.push(Splitter { path: path.clone(), index: i, axis: *axis, rect: s });
                        at += GAP;
                    }
                }
            }
        }
    }
}

/// Everything's place for a `size` dock manager. `flyout`: the auto-hidden
/// pane slid out, and its extent.
pub fn compute(layout: &Layout, size: (i64, i64), titles: &dyn Titles, font: &Font, flyout: Option<(&str, i64)>) -> Geometry {
    let (w, h) = size;
    let mut g = Geometry { size, ..Default::default() };
    let has = |s: Side| !layout.autohide[s.index()].is_empty();
    let (l, r, t, b) = (has(Side::Left), has(Side::Right), has(Side::Top), has(Side::Bottom));
    let left = if l { STRIP } else { 0 };
    let right = if r { STRIP } else { 0 };
    let top = if t { STRIP } else { 0 };
    let bottom = if b { STRIP } else { 0 };
    let inner = (left, top, (w - left - right).max(0), (h - top - bottom).max(0));
    g.inner = inner;
    for side in Side::ALL {
        if !has(side) {
            continue;
        }
        let strip = match side {
            Side::Left => (0, 0, STRIP, h),
            Side::Right => (w - STRIP, 0, STRIP, h),
            Side::Top => (left, 0, inner.2, STRIP),
            Side::Bottom => (left, h - STRIP, inner.2, STRIP),
        };
        g.strips.push((side, strip));
        let mut at = 6;
        for p in &layout.autohide[side.index()] {
            let len = tab_width(&titles.title(p), font, !titles.icon(p).is_empty(), false, 8);
            let rect = match side {
                Side::Left | Side::Right => (strip.0 + 2, strip.1 + at, STRIP - 4, len),
                Side::Top | Side::Bottom => (strip.0 + at, strip.1 + 2, len, STRIP - 4),
            };
            g.strip_tabs.push(StripTab { pane: p.clone(), side, rect });
            at += len + 4;
        }
    }
    let mut builder = Builder { layout, titles, font, g };
    builder.place(&layout.root, inner, Vec::new());
    let mut g = builder.g;
    if let Some((pane, extent)) = flyout {
        if let Some(Some(side)) = Side::ALL.iter().map(|s| layout.autohide[s.index()].iter().any(|p| p == pane).then_some(*s)).find(Option::is_some) {
            let (x, y, iw, ih) = inner;
            let e = extent.clamp(MIN_EXTENT, if side.axis() == Axis::Row { iw.max(MIN_EXTENT) } else { ih.max(MIN_EXTENT) });
            let rect = match side {
                Side::Left => (x, y, e, ih),
                Side::Right => (x + iw - e, y, e, ih),
                Side::Top => (x, y, iw, e),
                Side::Bottom => (x, y + ih - e, iw, e),
            };
            g.flyout = Some(group(Slot::Flyout, &[pane.to_string()], 0, rect, Vec::new(), titles, font));
        }
    }
    g
}

impl Geometry {
    /// A node's extent along `axis` (0: unknown).
    pub fn extent(&self, path: &[usize], axis: Axis) -> i64 {
        self.nodes.iter().find(|(p, _)| p == path).map_or(0, |(_, r)| if axis == Axis::Row { r.2 } else { r.3 })
    }

    /// The docked group holding `pane`.
    pub fn group_of(&self, pane: &str) -> Option<&Group> {
        self.groups.iter().find(|g| g.panes.iter().any(|p| p == pane))
    }

    pub fn slot(&self, slot: Slot) -> Option<&Group> {
        match slot {
            Slot::Docked(i) => self.groups.get(i),
            Slot::Flyout => self.flyout.as_ref(),
            Slot::Float(_) => None,
        }
    }

    /// The splitter or strip tab at (x, y) of the dock manager.
    pub fn hit(&self, x: i64, y: i64) -> Option<Hit> {
        let inside = |r: &Rect| x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3;
        if let Some(t) = self.strip_tabs.iter().find(|t| inside(&t.rect)) {
            return Some(Hit::Strip(t.pane.clone()));
        }
        // (a splitter: its gap, a pixel more each way to grab)
        self.splitters.iter().position(|s| inside(&(s.rect.0 - 1, s.rect.1 - 1, s.rect.2 + 2, s.rect.3 + 2))).map(Hit::Splitter)
    }

    /// The areas a dragged pane can go into or beside, with the compass
    /// over each: the docked groups, then the document area.
    pub fn areas(&self) -> Vec<(Anchor, Rect)> {
        let mut out: Vec<(Anchor, Rect)> = self.groups.iter().map(|g| (Anchor::Pane(g.panes[0].clone()), g.rect)).collect();
        if let Some(d) = &self.documents {
            out.push((Anchor::Documents, d.rect));
        }
        out
    }

    /// The compass's buttons while `pane` is dragged with the mouse at
    /// (x, y) (`None`: by the keyboard, over area `area`): the outer
    /// guides on the layout's edges, and the cross over the area under
    /// the mouse — each with the target it docks to.
    pub fn compass(&self, pane: &str, at: Option<(i64, i64)>, area: Option<usize>) -> Vec<(Guide, Rect, Target)> {
        let (ix, iy, iw, ih) = self.inner;
        let mut out = Vec::new();
        let (cx, cy) = (ix + iw / 2, iy + ih / 2);
        for side in Side::ALL {
            let r = match side {
                Side::Left => (ix + 8, cy - GUIDE / 2, GUIDE, GUIDE),
                Side::Right => (ix + iw - 8 - GUIDE, cy - GUIDE / 2, GUIDE, GUIDE),
                Side::Top => (cx - GUIDE / 2, iy + 8, GUIDE, GUIDE),
                Side::Bottom => (cx - GUIDE / 2, iy + ih - 8 - GUIDE, GUIDE, GUIDE),
            };
            out.push((Guide::Outer(side), r, Target::Edge(side)));
        }
        let areas = self.areas();
        let over = match (at, area) {
            (_, Some(a)) => areas.get(a).cloned(),
            (Some((x, y)), None) => areas.iter().find(|(_, r)| x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3).cloned(),
            _ => None,
        };
        if let Some((anchor, r)) = over {
            // (its own lone group: nowhere to go in or beside it)
            let own_alone = matches!(&anchor, Anchor::Pane(p) if self.group_of(p).is_some_and(|g| g.single() && g.panes[0] == pane));
            if !own_alone {
                let (mx, my) = (r.0 + r.2 / 2, r.1 + r.3 / 2);
                let step = GUIDE + 4;
                let own_group = matches!(&anchor, Anchor::Pane(p) if self.group_of(p).is_some_and(|g| g.panes.iter().any(|q| q == pane)));
                if !own_group {
                    out.push((Guide::Center, (mx - GUIDE / 2, my - GUIDE / 2, GUIDE, GUIDE), Target::Into(anchor.clone())));
                }
                for side in Side::ALL {
                    let (dx, dy) = match side {
                        Side::Left => (-step, 0),
                        Side::Right => (step, 0),
                        Side::Top => (0, -step),
                        Side::Bottom => (0, step),
                    };
                    out.push((Guide::Arm(side), (mx + dx - GUIDE / 2, my + dy - GUIDE / 2, GUIDE, GUIDE), Target::Beside(anchor.clone(), side)));
                }
            }
        }
        out
    }

    /// Where a target would put a pane of extent `extent` (the outline the
    /// compass shows).
    pub fn preview(&self, target: &Target, extent: i64) -> Option<Rect> {
        let (ix, iy, iw, ih) = self.inner;
        let along = |_side: Side, room: i64| extent.clamp(MIN_EXTENT.min(room), (room / 2).max(1));
        let beside = |r: Rect, side: Side| -> Rect {
            let (x, y, w, h) = r;
            match side {
                Side::Left => (x, y, along(side, w), h),
                Side::Right => (x + w - along(side, w), y, along(side, w), h),
                Side::Top => (x, y, w, along(side, h)),
                Side::Bottom => (x, y + h - along(side, h), w, along(side, h)),
            }
        };
        let area = |a: &Anchor| -> Option<Rect> {
            match a {
                Anchor::Documents => self.documents.as_ref().map(|d| d.rect),
                Anchor::Pane(p) => self.group_of(p).map(|g| g.rect),
            }
        };
        match target {
            Target::Edge(side) => Some(beside((ix, iy, iw, ih), *side)),
            Target::Beside(a, side) => area(a).map(|r| beside(r, *side)),
            Target::Into(a) => area(a),
            _ => None,
        }
    }
}

/// A compass button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Guide {
    Outer(Side),
    Center,
    Arm(Side),
}

/// What's under the mouse on the dock manager itself.
#[derive(Clone, Debug, PartialEq)]
pub enum Hit {
    Splitter(usize),
    Strip(String),
}

/// What's under the mouse on a group (its own coordinates).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupHit {
    Tab(usize),
    Button(Button),
    Header,
    Content,
}

impl Group {
    pub fn hit(&self, x: i64, y: i64) -> Option<GroupHit> {
        let inside = |r: &Rect| x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3;
        if let Some((b, _)) = self.buttons.iter().find(|(_, r)| inside(r)) {
            return Some(GroupHit::Button(*b));
        }
        if y < HEADER {
            if !self.single() {
                if let Some(i) = self.tabs.iter().position(|t| inside(&t.rect)) {
                    return Some(GroupHit::Tab(i));
                }
            }
            return Some(GroupHit::Header);
        }
        (x >= 0 && y >= 0 && x < self.rect.2 && y < self.rect.3).then_some(GroupHit::Content)
    }
}

/// What's under the mouse on the document area (its own coordinates).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocHit {
    Tab(usize),
    Close(usize),
}

impl Documents {
    pub fn hit(&self, x: i64, y: i64) -> Option<DocHit> {
        let inside = |r: &Rect| x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3;
        for (i, t) in self.tabs.iter().enumerate() {
            if t.close.as_ref().is_some_and(inside) {
                return Some(DocHit::Close(i));
            }
            if inside(&t.rect) {
                return Some(DocHit::Tab(i));
            }
        }
        None
    }
}

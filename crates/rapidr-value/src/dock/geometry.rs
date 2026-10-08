//! Where everything of a dock manager goes for a size: the auto-hide
//! strips on its edges and their tabs, each docked group (its header,
//! tabs, buttons, the place of its pane's component), the splitters
//! between them, the document area (its tabs in tabbed mode), an open
//! auto-hide flyout; the docking compass while a pane is dragged; and what
//! is under a point. Theme-free: every theme draws the same places, so the
//! components sit where they sit whatever the look (logical pixels).

use super::manager::DocView;
use super::{Anchor, Axis, DocNode, DocTarget, DocumentMode, Layout, Node, Rect, Side, Target};
use crate::objects::font::Font;
use crate::objects::text::text_size;

/// Between two docked neighbours: the splitter you drag.
pub const GAP: i64 = 4;
/// The least a docked group is along a split.
pub const MIN_EXTENT: i64 = 60;
/// A group's header (its title or tabs, its buttons).
pub const HEADER: i64 = 28;
/// The tabbed document area's tab strip.
pub const DOC_TABS: i64 = 30;
/// A document's view switch (Design | Code | side by side): its height,
/// the side-by-side segment's width.
pub const SWITCH_H: i64 = 22;
pub const SWITCH_SPLIT: i64 = 30;
/// The least a tab shrinks to when a strip is full.
pub const MIN_TAB: i64 = 64;
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

/// The document area: where it is (in the dock manager), and in its own
/// coordinates its groups (tabbed mode) and the splitters between them;
/// MDI: `content` is all of it.
#[derive(Clone, Debug, PartialEq)]
pub struct Documents {
    pub rect: Rect,
    pub groups: Vec<DocGroup>,
    pub splitters: Vec<Splitter>,
    /// Each split's children's extents as laid out (by its path).
    pub extents: Vec<(Vec<usize>, Vec<i64>)>,
    pub content: Rect,
    pub path: Vec<usize>,
}

/// A group of tabbed documents (the document area's coordinates): its
/// strip's tabs, the shown document's view switch, where its view(s) go.
#[derive(Clone, Debug, PartialEq)]
pub struct DocGroup {
    /// Its path in [`Layout::groups`].
    pub path: Vec<usize>,
    pub rect: Rect,
    pub docs: Vec<String>,
    pub active: usize,
    pub tabs: Vec<Tab>,
    /// The switch's segments: (the view, its caption, its place).
    pub switch: Vec<(DocView, String, Rect)>,
    /// Under the strip.
    pub content: Rect,
    /// Where the shown document's component(s) go: one, or two side by
    /// side with the splitter between them.
    pub places: Vec<Rect>,
    pub view_splitter: Option<Rect>,
}

impl DocGroup {
    /// The shown document.
    pub fn shown(&self) -> Option<&String> {
        self.docs.get(self.active)
    }
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

/// What a pane looks like in a tab: its title and icon name; a
/// document's views (captions), the one shown, the side-by-side share, a
/// change not saved.
pub trait Titles {
    fn title(&self, pane: &str) -> String;
    fn icon(&self, pane: &str) -> String;
    fn views(&self, _pane: &str) -> Vec<String> {
        Vec::new()
    }
    fn view(&self, _pane: &str) -> DocView {
        DocView::First
    }
    fn view_ratio(&self, _pane: &str) -> i64 {
        500
    }
    fn modified(&self, _pane: &str) -> bool {
        false
    }
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

/// A tab's width for `title` (with an icon, a close button): measured in
/// the bold face the shown tab is drawn in, so it fits shown or not.
fn tab_width(title: &str, font: &Font, icon: bool, close: bool, pad: i64) -> i64 {
    let (w, _) = text_size(title, &Font { styles: font.styles | 1, ..font.clone() });
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

/// The document area's groups (tabbed mode) and content, `rect` its place.
fn documents(layout: &Layout, rect: Rect, path: Vec<usize>, titles: &dyn Titles, font: &Font) -> Documents {
    let (w, h) = (rect.2, rect.3);
    let mut d = Documents { rect, groups: Vec::new(), splitters: Vec::new(), extents: Vec::new(), content: (0, 0, w, h), path };
    if layout.mode == DocumentMode::Mdi || layout.documents.is_empty() {
        return d;
    }
    place_docs(&layout.groups, (0, 0, w, h), Vec::new(), titles, font, &mut d);
    d
}

/// Extents of a document split's children in proportion to their
/// weights, each at least [`MIN_EXTENT`] while there's room.
pub fn weighted(weights: &[i64], avail: i64) -> Vec<i64> {
    let n = weights.len().max(1) as i64;
    let total: i64 = weights.iter().map(|w| (*w).max(1)).sum::<i64>().max(1);
    let mut ext: Vec<i64> = weights.iter().map(|w| avail.max(0) * (*w).max(1) / total).collect();
    if avail >= MIN_EXTENT * n {
        // (none under the least: the others give it what it lacks)
        for i in 0..ext.len() {
            if ext[i] < MIN_EXTENT {
                let need = MIN_EXTENT - ext[i];
                ext[i] = MIN_EXTENT;
                let big = (0..ext.len()).filter(|&k| k != i).max_by_key(|&k| ext[k]).unwrap_or(i);
                ext[big] -= need;
            }
        }
    }
    // (the rounding's leftover to the last)
    let sum: i64 = ext.iter().sum();
    if let Some(last) = ext.last_mut() {
        *last += avail.max(0) - sum;
    }
    ext
}

fn place_docs(node: &DocNode, rect: Rect, path: Vec<usize>, titles: &dyn Titles, font: &Font, d: &mut Documents) {
    match node {
        DocNode::Group { docs, active } => d.groups.push(doc_group(docs, *active, rect, path, titles, font)),
        DocNode::Split { axis, children, weights } => {
            let (x, y, w, h) = rect;
            let along = if *axis == Axis::Row { w } else { h };
            let avail = along - GAP * (children.len() as i64 - 1);
            let ext = weighted(weights, avail);
            d.extents.push((path.clone(), ext.clone()));
            let mut at = 0;
            for (i, (c, e)) in children.iter().zip(&ext).enumerate() {
                let r = if *axis == Axis::Row { (x + at, y, *e, h) } else { (x, y + at, w, *e) };
                let mut p = path.clone();
                p.push(i);
                place_docs(c, r, p, titles, font, d);
                at += e;
                if i + 1 < children.len() {
                    let s = if *axis == Axis::Row { (x + at, y, GAP, h) } else { (x, y + at, w, GAP) };
                    d.splitters.push(Splitter { path: path.clone(), index: i, axis: *axis, rect: s });
                    at += GAP;
                }
            }
        }
    }
}

/// One group of tabbed documents at `rect` (the document area's
/// coordinates): its tabs (shrunk alike when they don't fit, none under
/// [`MIN_TAB`]), the shown document's view switch at the strip's right,
/// and where its view(s) go.
pub fn doc_group(docs: &[String], active: usize, rect: Rect, path: Vec<usize>, titles: &dyn Titles, font: &Font) -> DocGroup {
    let (gx, gy, gw, gh) = rect;
    let active = active.min(docs.len().saturating_sub(1));
    let shown = docs.get(active).cloned().unwrap_or_default();
    // (the view switch: each view's caption, then side by side)
    let views = titles.views(&shown);
    let mut switch = Vec::new();
    let mut right = gx + gw - 6;
    if views.len() >= 2 {
        let mut segs: Vec<(DocView, String, i64)> = views.iter().enumerate().map(|(i, c)| (if i == 0 { DocView::First } else { DocView::One(i) }, c.clone(), text_size(c, font).0 + 22)).collect();
        segs.push((DocView::Split, "Side by Side".into(), SWITCH_SPLIT));
        let total: i64 = segs.iter().map(|s| s.2).sum();
        let mut x = right - total;
        let y = gy + (DOC_TABS - SWITCH_H) / 2;
        for (v, c, wd) in segs {
            switch.push((v, c, (x, y, wd, SWITCH_H)));
            x += wd;
        }
        right -= total + 8;
    }
    let room = (right - gx).max(0);
    let widths: Vec<i64> = docs.iter().map(|p| tab_width(&titles.title(p), font, !titles.icon(p).is_empty(), true, 12).min(260)).collect();
    let total: i64 = widths.iter().map(|w| w + 1).sum();
    let fit = |wd: i64| if total > room && total > 0 { (wd * room / total).max(MIN_TAB.min(wd)) } else { wd };
    let mut x = gx;
    let mut tabs = Vec::new();
    for (p, wd) in docs.iter().zip(widths) {
        let wd = fit(wd);
        let close = (x + wd - BUTTON - 6, gy + (DOC_TABS - BUTTON) / 2 + 1, BUTTON - 2, BUTTON - 2);
        tabs.push(Tab { pane: p.clone(), rect: (x, gy, wd, DOC_TABS), close: Some(close) });
        x += wd + 1;
    }
    let content = (gx, gy + DOC_TABS, gw, (gh - DOC_TABS).max(0));
    let (cx, cy, cw, ch) = content;
    let (places, view_splitter) = if views.len() >= 2 && titles.view(&shown) == DocView::Split {
        let first = ((cw - GAP) * titles.view_ratio(&shown).clamp(100, 900) / 1000).max(0);
        let second = (cw - GAP - first).max(0);
        (vec![(cx, cy, first, ch), (cx + first + GAP, cy, second, ch)], Some((cx + first, cy, GAP, ch)))
    } else {
        (vec![content], None)
    };
    DocGroup { path, rect, docs: docs.to_vec(), active, tabs, switch, content, places, view_splitter }
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

/// What's under the mouse on the document area (its own coordinates):
/// a group's tab, its close button, a segment of its view switch, the
/// rest of its strip; a splitter between groups; the splitter between a
/// group's two views.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocHit {
    Tab(usize, usize),
    Close(usize, usize),
    View(usize, usize),
    Strip(usize),
    Splitter(usize),
    ViewSplitter(usize),
}

impl Documents {
    pub fn hit(&self, x: i64, y: i64) -> Option<DocHit> {
        let inside = |r: &Rect| x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3;
        let grab = |r: &Rect| inside(&(r.0 - 1, r.1 - 1, r.2 + 2, r.3 + 2));
        if let Some(i) = self.splitters.iter().position(|s| grab(&s.rect)) {
            return Some(DocHit::Splitter(i));
        }
        for (g, gr) in self.groups.iter().enumerate() {
            if gr.view_splitter.as_ref().is_some_and(grab) {
                return Some(DocHit::ViewSplitter(g));
            }
            if let Some(k) = gr.switch.iter().position(|(_, _, r)| inside(r)) {
                return Some(DocHit::View(g, k));
            }
            for (i, t) in gr.tabs.iter().enumerate() {
                if t.close.as_ref().is_some_and(inside) {
                    return Some(DocHit::Close(g, i));
                }
                if inside(&t.rect) {
                    return Some(DocHit::Tab(g, i));
                }
            }
            if inside(&(gr.rect.0, gr.rect.1, gr.rect.2, DOC_TABS)) {
                return Some(DocHit::Strip(g));
            }
        }
        None
    }

    /// The group holding `doc`.
    pub fn group_of(&self, doc: &str) -> Option<usize> {
        self.groups.iter().position(|g| g.docs.iter().any(|d| d == doc))
    }

    /// Where document `doc` dragged to (x, y) would go, with the outline
    /// shown (a group's half for a new group beside it, all of it to join
    /// it) or the insertion bar between tabs: over a strip, among its
    /// tabs; over a group's page, its outer quarters split it, its middle
    /// joins it (VS Code's editor groups).
    pub fn drop_at(&self, doc: &str, x: i64, y: i64) -> Option<(DocTarget, Option<Rect>, Option<Rect>)> {
        let inside = |r: &Rect| x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3;
        let from = self.group_of(doc);
        for (g, gr) in self.groups.iter().enumerate() {
            if !inside(&gr.rect) {
                continue;
            }
            let anchor = gr.docs.first()?.clone();
            let strip = (gr.rect.0, gr.rect.1, gr.rect.2, DOC_TABS);
            if inside(&strip) {
                // (before the first tab whose middle is right of the mouse)
                let index = gr.tabs.iter().position(|t| x < t.rect.0 + t.rect.2 / 2).unwrap_or(gr.tabs.len());
                let bx = match gr.tabs.get(index) {
                    Some(t) => t.rect.0 - 1,
                    None => gr.tabs.last().map_or(gr.rect.0, |t| t.rect.0 + t.rect.2),
                };
                let bar = (bx - 1, gr.rect.1 + 3, 3, DOC_TABS - 6);
                return Some((DocTarget::Into { anchor, index }, None, Some(bar)));
            }
            let (cx, cy, cw, ch) = gr.content;
            let (fx, fy) = ((x - cx) as f64 / cw.max(1) as f64, (y - cy) as f64 / ch.max(1) as f64);
            // (the nearest edge within a quarter, else the middle)
            let edges = [(fx, Side::Left), (1.0 - fx, Side::Right), (fy, Side::Top), (1.0 - fy, Side::Bottom)];
            let (near, side) = edges.iter().cloned().fold((f64::MAX, Side::Left), |a, b| if b.0 < a.0 { b } else { a });
            let alone = from == Some(g) && gr.docs.len() == 1;
            if near < 0.25 && !alone {
                let r = match side {
                    Side::Left => (cx, cy, cw / 2, ch),
                    Side::Right => (cx + cw - cw / 2, cy, cw / 2, ch),
                    Side::Top => (cx, cy, cw, ch / 2),
                    Side::Bottom => (cx, cy + ch - ch / 2, cw, ch / 2),
                };
                return Some((DocTarget::Split { anchor, side }, Some(r), None));
            }
            if from == Some(g) {
                return None;
            }
            return Some((DocTarget::Into { anchor, index: gr.docs.len() }, Some(gr.content), None));
        }
        None
    }
}

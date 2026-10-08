//! RDOCKMANAGER (RapidR Studio's docking, docs/ide-plan.md §4 I1,
//! docs/ide-components.md §3.7): panes docked left, right, top or bottom
//! of a document area, split and tabbed, auto-hidden on an edge or
//! floating in a window of their own; the document area an MDI client
//! (`crate::mdi`, the default) or tabbed documents. This is the model —
//! GUI-free and the same on every runtime:
//!
//! - [`Layout`]: the tree ([`Node`]: splits, tab groups, the documents),
//!   the auto-hidden panes by edge, the floating groups, the documents;
//!   every operation (remove a pane and remember its [`Place`], insert one
//!   at a [`Target`]) and the text [`Layout::save`] / [`Layout::load`]
//!   round-trip.
//! - [`geometry`]: where everything goes for a size (groups, their tabs
//!   and buttons, splitters, auto-hide strips, the docking compass), and
//!   what's under the mouse.
//! - [`look`]: what it looks like in each theme (the kernel's ops).
//! - [`manager`]: one dock manager's state (its panes, the layout, the
//!   user's transient state), its methods, properties and events.
//! - [`runtime`]: applying it through a runtime (placing the panes'
//!   components, the group components the kernel draws, floating windows,
//!   the program's events) — the desktop's and the web's alike.
//! - [`access`]: what a screen reader is told.
//!
//! Pane names are their components' names, lowercase.

pub mod access;
pub mod geometry;
pub mod look;
pub mod manager;
pub mod runtime;
#[cfg(test)]
mod tests;

pub use manager::{is_docs_area, with, with_mut, Manager};

/// A rectangle: left, top, width, height (logical pixels).
pub type Rect = crate::objects::ops::Rect;

/// An edge (of the dock manager, or of a group or the documents).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    Left,
    Top,
    Right,
    Bottom,
}

impl Side {
    pub const ALL: [Side; 4] = [Side::Left, Side::Top, Side::Right, Side::Bottom];

    pub fn name(self) -> &'static str {
        match self {
            Side::Left => "left",
            Side::Top => "top",
            Side::Right => "right",
            Side::Bottom => "bottom",
        }
    }

    pub fn parse(s: &str) -> Option<Side> {
        Side::ALL.into_iter().find(|d| d.name().eq_ignore_ascii_case(s.trim()))
    }

    /// The axis along which panes on this side sit beside the rest.
    pub fn axis(self) -> Axis {
        match self {
            Side::Left | Side::Right => Axis::Row,
            Side::Top | Side::Bottom => Axis::Column,
        }
    }

    /// Whether it comes before (left of, above) what it's beside.
    pub fn before(self) -> bool {
        matches!(self, Side::Left | Side::Top)
    }

    pub fn index(self) -> usize {
        self as usize
    }
}

/// How a split lays its children out: side by side (a row) or one above
/// another (a column).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Axis {
    Row,
    Column,
}

impl Axis {
    fn name(self) -> &'static str {
        match self {
            Axis::Row => "row",
            Axis::Column => "column",
        }
    }
}

/// The docked layout: a tree whose leaves are tab groups and the one
/// document area.
#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    /// Children side by side (a row) or stacked (a column), each with its
    /// extent along the axis in pixels; the child holding the documents
    /// (else the last) takes what the others leave.
    Split { axis: Axis, children: Vec<Node>, sizes: Vec<i64> },
    /// A group of panes, one shown (`active`).
    Tabs { panes: Vec<String>, active: usize },
    /// The document area.
    Documents,
}

impl Node {
    fn tabs(pane: &str) -> Node {
        Node::Tabs { panes: vec![pane.to_string()], active: 0 }
    }

    pub fn has_documents(&self) -> bool {
        match self {
            Node::Documents => true,
            Node::Tabs { .. } => false,
            Node::Split { children, .. } => children.iter().any(Node::has_documents),
        }
    }

    /// The child of a split that takes the room left over.
    pub fn flex(children: &[Node]) -> usize {
        children.iter().position(Node::has_documents).unwrap_or(children.len().saturating_sub(1))
    }

    /// Every pane under it, in order.
    pub fn panes(&self, out: &mut Vec<String>) {
        match self {
            Node::Split { children, .. } => children.iter().for_each(|c| c.panes(out)),
            Node::Tabs { panes, .. } => out.extend(panes.iter().cloned()),
            Node::Documents => {}
        }
    }

    /// The path (child indexes from here) to the group holding `pane`.
    pub fn path_of(&self, pane: &str) -> Option<Vec<usize>> {
        match self {
            Node::Tabs { panes, .. } => panes.iter().any(|p| p == pane).then(Vec::new),
            Node::Documents => None,
            Node::Split { children, .. } => children.iter().enumerate().find_map(|(i, c)| {
                c.path_of(pane).map(|mut p| {
                    p.insert(0, i);
                    p
                })
            }),
        }
    }

    /// The path to the document area.
    pub fn documents_path(&self) -> Option<Vec<usize>> {
        match self {
            Node::Documents => Some(Vec::new()),
            Node::Tabs { .. } => None,
            Node::Split { children, .. } => children.iter().enumerate().find_map(|(i, c)| {
                c.documents_path().map(|mut p| {
                    p.insert(0, i);
                    p
                })
            }),
        }
    }

    pub fn at(&self, path: &[usize]) -> Option<&Node> {
        match (path.split_first(), self) {
            (None, n) => Some(n),
            (Some((&i, rest)), Node::Split { children, .. }) => children.get(i)?.at(rest),
            _ => None,
        }
    }

    pub fn at_mut(&mut self, path: &[usize]) -> Option<&mut Node> {
        match path.split_first() {
            None => Some(self),
            Some((&i, rest)) => match self {
                Node::Split { children, .. } => children.get_mut(i)?.at_mut(rest),
                _ => None,
            },
        }
    }

    /// Empty groups and splits gone, a split of one child replaced by it,
    /// a split inside a split on the same axis merged into it.
    fn normalized(self) -> Option<Node> {
        match self {
            Node::Tabs { panes, .. } if panes.is_empty() => None,
            Node::Tabs { panes, active } => {
                let active = active.min(panes.len() - 1);
                Some(Node::Tabs { panes, active })
            }
            Node::Documents => Some(Node::Documents),
            Node::Split { axis, children, sizes } => {
                let (mut kids, mut extents) = (Vec::new(), Vec::new());
                for (c, s) in children.into_iter().zip(sizes) {
                    match c.normalized() {
                        Some(Node::Split { axis: a, children: cc, sizes: ss }) if a == axis => {
                            kids.extend(cc);
                            extents.extend(ss);
                        }
                        Some(n) => {
                            kids.push(n);
                            extents.push(s);
                        }
                        None => {}
                    }
                }
                match kids.len() {
                    0 => None,
                    1 => kids.pop(),
                    _ => Some(Node::Split { axis, children: kids, sizes: extents }),
                }
            }
        }
    }
}

/// Where a pane is dropped (by the mouse on the compass, the keyboard's
/// move, or the program's `DockPane` / `AddPane`).
#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    /// Along an outer edge of the whole layout.
    Edge(Side),
    /// Beside a group (named by one of its panes) or the documents.
    Beside(Anchor, Side),
    /// Into a group as a tab, or among the documents.
    Into(Anchor),
    /// A floating window at this place (screen coordinates).
    Float(Rect),
    /// Auto-hidden on an edge.
    AutoHide(Side),
}

/// A group (by one of its panes) or the document area.
#[derive(Clone, Debug, PartialEq)]
pub enum Anchor {
    Pane(String),
    Documents,
}

/// Where a pane was, so it can go back (ShowPane after HidePane, docking a
/// floating or auto-hidden pane again).
#[derive(Clone, Debug, PartialEq)]
pub enum Place {
    /// In a group, with this pane, at this index.
    Tab(String, usize),
    /// A group of its own beside this pane's group (or the documents), on
    /// this side of it, this extent.
    Beside(Anchor, Side, i64),
    Edge(Side, i64),
    AutoHide(Side),
    Float(Rect),
    Document,
}

impl Place {
    /// The target that puts the pane back, given which panes are docked
    /// now (`None`: none to go back to: its own default edge).
    pub fn target(&self, docked: &dyn Fn(&str) -> bool) -> Option<(Target, i64)> {
        match self {
            Place::Tab(p, _) if docked(p) => Some((Target::Into(Anchor::Pane(p.clone())), 0)),
            Place::Beside(Anchor::Pane(p), side, size) if docked(p) => Some((Target::Beside(Anchor::Pane(p.clone()), *side), *size)),
            Place::Beside(Anchor::Documents, side, size) => Some((Target::Beside(Anchor::Documents, *side), *size)),
            Place::Edge(side, size) => Some((Target::Edge(*side), *size)),
            Place::AutoHide(side) => Some((Target::AutoHide(*side), 0)),
            Place::Float(r) => Some((Target::Float(*r), 0)),
            Place::Document => Some((Target::Into(Anchor::Documents), 0)),
            _ => None,
        }
    }
}

/// A floating window's group.
#[derive(Clone, Debug, PartialEq)]
pub struct Floating {
    pub panes: Vec<String>,
    pub active: usize,
    /// Its window's place on the screen (left, top) and size.
    pub rect: Rect,
}

/// Where a pane is now.
#[derive(Clone, Debug, PartialEq)]
pub enum Where {
    /// In a docked group (the path to it, its index there).
    Docked(Vec<usize>, usize),
    AutoHide(Side, usize),
    Floating(usize, usize),
    Document(usize),
}

/// What the documents area shows: MDI child windows (the default) or tabs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocumentMode {
    Mdi,
    Tabs,
}

impl DocumentMode {
    pub fn name(self) -> &'static str {
        match self {
            DocumentMode::Mdi => "mdi",
            DocumentMode::Tabs => "tabs",
        }
    }

    pub fn parse(s: &str) -> Option<DocumentMode> {
        match s.trim().to_ascii_lowercase().as_str() {
            "mdi" => Some(DocumentMode::Mdi),
            "tabs" | "tabbed" => Some(DocumentMode::Tabs),
            _ => None,
        }
    }
}

/// The default extent of a pane docked on a side (pixels).
pub const DEFAULT_SIDE: i64 = 240;
pub const DEFAULT_BOTTOM: i64 = 180;
/// A floating window's default size.
pub const DEFAULT_FLOAT: (i64, i64) = (300, 360);

pub fn default_extent(side: Side) -> i64 {
    if side.axis() == Axis::Row {
        DEFAULT_SIDE
    } else {
        DEFAULT_BOTTOM
    }
}

/// The whole arrangement.
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub root: Node,
    /// Auto-hidden panes by edge ([`Side::index`]).
    pub autohide: [Vec<String>; 4],
    pub floating: Vec<Floating>,
    /// Every document, in the order of [`Layout::groups`] (kept from it:
    /// change the documents through the layout's operations).
    pub documents: Vec<String>,
    /// The active document (an index in `documents`).
    pub active_document: Option<usize>,
    pub mode: DocumentMode,
    /// The tabbed documents' groups (VS Code's editor groups): one group
    /// until a document is dragged to a side (or `SplitDocument`).
    pub groups: DocNode,
}

impl Default for Layout {
    fn default() -> Self {
        Layout { root: Node::Documents, autohide: Default::default(), floating: Vec::new(), documents: Vec::new(), active_document: None, mode: DocumentMode::Mdi, groups: DocNode::default() }
    }
}

/// The tabbed document area's groups: split side by side (a row) or
/// stacked (a column) in proportion to their weights, each a strip of
/// tabs with one document shown.
#[derive(Clone, Debug, PartialEq)]
pub enum DocNode {
    Group { docs: Vec<String>, active: usize },
    Split { axis: Axis, children: Vec<DocNode>, weights: Vec<i64> },
}

impl Default for DocNode {
    fn default() -> Self {
        DocNode::Group { docs: Vec::new(), active: 0 }
    }
}

/// Where a dragged document goes: into the group holding `anchor` at tab
/// `index`, or a new group on `side` of it.
#[derive(Clone, Debug, PartialEq)]
pub enum DocTarget {
    Into { anchor: String, index: usize },
    Split { anchor: String, side: Side },
}

impl DocNode {
    /// Its documents in order.
    pub fn docs(&self, out: &mut Vec<String>) {
        match self {
            DocNode::Group { docs, .. } => out.extend(docs.iter().cloned()),
            DocNode::Split { children, .. } => children.iter().for_each(|c| c.docs(out)),
        }
    }

    /// The groups in order: (path, documents, shown).
    pub fn groups(&self) -> Vec<(Vec<usize>, Vec<String>, usize)> {
        fn walk(n: &DocNode, path: Vec<usize>, out: &mut Vec<(Vec<usize>, Vec<String>, usize)>) {
            match n {
                DocNode::Group { docs, active } => out.push((path, docs.clone(), *active)),
                DocNode::Split { children, .. } => {
                    for (i, c) in children.iter().enumerate() {
                        let mut p = path.clone();
                        p.push(i);
                        walk(c, p, out);
                    }
                }
            }
        }
        let mut out = Vec::new();
        walk(self, Vec::new(), &mut out);
        out
    }

    /// The path to the group holding `doc`.
    pub fn path_of(&self, doc: &str) -> Option<Vec<usize>> {
        self.groups().into_iter().find(|(_, d, _)| d.iter().any(|x| x == doc)).map(|(p, _, _)| p)
    }

    pub fn at_mut(&mut self, path: &[usize]) -> Option<&mut DocNode> {
        match path.split_first() {
            None => Some(self),
            Some((&i, rest)) => match self {
                DocNode::Split { children, .. } => children.get_mut(i)?.at_mut(rest),
                _ => None,
            },
        }
    }

    pub fn at(&self, path: &[usize]) -> Option<&DocNode> {
        match (path.split_first(), self) {
            (None, n) => Some(n),
            (Some((&i, rest)), DocNode::Split { children, .. }) => children.get(i)?.at(rest),
            _ => None,
        }
    }

    /// Empty groups gone, a split of one child replaced by it, a split in
    /// a split on the same axis merged into it (its weights shared out in
    /// proportion).
    fn normalized(self) -> Option<DocNode> {
        match self {
            DocNode::Group { docs, .. } if docs.is_empty() => None,
            DocNode::Group { docs, active } => {
                let active = active.min(docs.len() - 1);
                Some(DocNode::Group { docs, active })
            }
            DocNode::Split { axis, children, weights } => {
                let (mut kids, mut ws) = (Vec::new(), Vec::new());
                for (c, w) in children.into_iter().zip(weights) {
                    match c.normalized() {
                        Some(DocNode::Split { axis: a, children: cc, weights: ww }) if a == axis => {
                            let total: i64 = ww.iter().sum::<i64>().max(1);
                            for (c2, w2) in cc.into_iter().zip(ww) {
                                kids.push(c2);
                                ws.push((w * w2 / total).max(1));
                            }
                        }
                        Some(n) => {
                            kids.push(n);
                            ws.push(w.max(1));
                        }
                        None => {}
                    }
                }
                match kids.len() {
                    0 => None,
                    1 => kids.pop(),
                    _ => Some(DocNode::Split { axis, children: kids, weights: ws }),
                }
            }
        }
    }

    /// Only the documents `keep` says.
    fn filter(self, keep: &dyn Fn(&str) -> bool) -> DocNode {
        match self {
            DocNode::Group { docs, active } => {
                let shown = docs.get(active).cloned();
                let docs: Vec<String> = docs.into_iter().filter(|d| keep(d)).collect();
                let active = shown.and_then(|s| docs.iter().position(|d| *d == s)).unwrap_or(0);
                DocNode::Group { docs, active }
            }
            DocNode::Split { axis, children, weights } => DocNode::Split { axis, children: children.into_iter().map(|c| c.filter(keep)).collect(), weights },
        }
    }
}

impl Layout {
    /// Where `pane` is (`None`: hidden, or no such pane).
    pub fn find(&self, pane: &str) -> Option<Where> {
        if let Some(path) = self.root.path_of(pane) {
            if let Some(Node::Tabs { panes, .. }) = self.root.at(&path) {
                let i = panes.iter().position(|p| p == pane)?;
                return Some(Where::Docked(path, i));
            }
        }
        for side in Side::ALL {
            if let Some(i) = self.autohide[side.index()].iter().position(|p| p == pane) {
                return Some(Where::AutoHide(side, i));
            }
        }
        for (f, fl) in self.floating.iter().enumerate() {
            if let Some(i) = fl.panes.iter().position(|p| p == pane) {
                return Some(Where::Floating(f, i));
            }
        }
        self.documents.iter().position(|p| p == pane).map(Where::Document)
    }

    pub fn is_docked(&self, pane: &str) -> bool {
        matches!(self.find(pane), Some(Where::Docked(..)))
    }

    /// Every pane placed (docked, auto-hidden, floating, documents).
    pub fn placed(&self) -> Vec<String> {
        let mut out = Vec::new();
        self.root.panes(&mut out);
        for side in &self.autohide {
            out.extend(side.iter().cloned());
        }
        for f in &self.floating {
            out.extend(f.panes.iter().cloned());
        }
        out.extend(self.documents.iter().cloned());
        out
    }

    /// The pane shown in the group holding `pane` (or the floating group).
    pub fn is_active(&self, pane: &str) -> bool {
        match self.find(pane) {
            Some(Where::Docked(path, i)) => matches!(self.root.at(&path), Some(Node::Tabs { active, .. }) if *active == i),
            Some(Where::Floating(f, i)) => self.floating[f].active == i,
            Some(Where::Document(i)) => self.active_document == Some(i),
            Some(Where::AutoHide(..)) => true,
            None => false,
        }
    }

    /// Makes `pane` the one its group shows; whether it changed.
    pub fn select(&mut self, pane: &str) -> bool {
        match self.find(pane) {
            Some(Where::Docked(path, i)) => {
                if let Some(Node::Tabs { active, .. }) = self.root.at_mut(&path) {
                    let changed = *active != i;
                    *active = i;
                    return changed;
                }
                false
            }
            Some(Where::Floating(f, i)) => std::mem::replace(&mut self.floating[f].active, i) != i,
            Some(Where::Document(i)) => {
                let doc = self.documents[i].clone();
                if let Some(path) = self.groups.path_of(&doc) {
                    if let Some(DocNode::Group { docs, active }) = self.groups.at_mut(&path) {
                        if let Some(k) = docs.iter().position(|d| *d == doc) {
                            *active = k;
                        }
                    }
                }
                self.active_document.replace(i) != Some(i)
            }
            _ => false,
        }
    }

    // ------------------------------------------------- document groups --

    /// `documents` and `active_document` again from the groups (the
    /// active document kept by name, else the first group's shown one).
    fn sync_documents(&mut self, was: Option<String>) {
        let mut docs = Vec::new();
        self.groups.docs(&mut docs);
        self.documents = docs;
        self.active_document = match was.and_then(|w| self.documents.iter().position(|d| *d == w)) {
            Some(i) => Some(i),
            None => {
                let shown = self.groups.groups().into_iter().find_map(|(_, d, a)| d.get(a).cloned());
                shown.and_then(|s| self.documents.iter().position(|d| *d == s))
            }
        };
    }

    fn normalize_groups(&mut self) {
        let g = std::mem::take(&mut self.groups);
        self.groups = g.normalized().unwrap_or_default();
    }

    /// The group (its path) a new document goes to: the active
    /// document's, else the first.
    fn active_group_path(&self) -> Vec<usize> {
        self.active_document
            .and_then(|i| self.documents.get(i))
            .and_then(|d| self.groups.path_of(d))
            .unwrap_or_else(|| self.groups.groups().first().map(|g| g.0.clone()).unwrap_or_default())
    }

    /// Adds a document (not placed now) to the active group, shown and
    /// active.
    pub fn add_document(&mut self, doc: &str) {
        let path = self.active_group_path();
        if let Some(DocNode::Group { docs, active }) = self.groups.at_mut(&path) {
            docs.push(doc.to_string());
            *active = docs.len() - 1;
        }
        self.sync_documents(Some(doc.to_string()));
    }

    /// The document groups' count.
    pub fn group_count(&self) -> usize {
        self.groups.groups().len()
    }

    /// Shows document `i` in its group and makes it active.
    pub fn select_document(&mut self, i: usize) {
        if let Some(doc) = self.documents.get(i).cloned() {
            self.select(&doc);
        }
    }

    /// Moves document `doc` to `target` (into a group at a tab index, or
    /// a new group beside one), shown and active; whether it moved.
    pub fn move_document(&mut self, doc: &str, target: &DocTarget) -> bool {
        let Some(from) = self.groups.path_of(doc) else { return false };
        let anchor = match target {
            DocTarget::Into { anchor, .. } | DocTarget::Split { anchor, .. } => anchor.clone(),
        };
        let own: Vec<String> = match self.groups.at(&from) {
            Some(DocNode::Group { docs, .. }) => docs.clone(),
            _ => Vec::new(),
        };
        let Some(to) = self.groups.path_of(&anchor) else { return false };
        let same_group = to == from;
        match target {
            DocTarget::Into { index, .. } => {
                if same_group {
                    if let Some(DocNode::Group { docs, active }) = self.groups.at_mut(&to) {
                        let i = docs.iter().position(|d| d == doc).unwrap_or(0);
                        let d = docs.remove(i);
                        let at = if *index > i { index - 1 } else { *index }.min(docs.len());
                        docs.insert(at, d);
                        *active = at;
                    }
                } else {
                    // (the anchor's group found again once it has left its own)
                    self.take_from_group(doc);
                    let Some(to) = self.groups.path_of(&anchor) else { return false };
                    if let Some(DocNode::Group { docs, active }) = self.groups.at_mut(&to) {
                        let at = (*index).min(docs.len());
                        docs.insert(at, doc.to_string());
                        *active = at;
                    }
                }
            }
            DocTarget::Split { side, .. } => {
                // (beside its own lone group: nowhere to go)
                if same_group && own.len() == 1 {
                    return false;
                }
                // (beside its own group: another of the group anchors it)
                let anchor = if anchor == doc {
                    match own.iter().find(|d| d.as_str() != doc) {
                        Some(o) => o.clone(),
                        None => return false,
                    }
                } else {
                    anchor
                };
                self.take_from_group(doc);
                let Some(to) = self.groups.path_of(&anchor) else { return false };
                let new = DocNode::Group { docs: vec![doc.to_string()], active: 0 };
                let axis = side.axis();
                // (a split on the same axis takes it as a sibling, halving the anchor's weight)
                let mut done = false;
                if let Some((&i, parent)) = to.split_last() {
                    if let Some(DocNode::Split { axis: a, children, weights }) = self.groups.at_mut(parent) {
                        if *a == axis {
                            let half = (weights[i] / 2).max(1);
                            weights[i] -= half;
                            let at = if side.before() { i } else { i + 1 };
                            children.insert(at, new.clone());
                            weights.insert(at, half);
                            done = true;
                        }
                    }
                }
                if !done {
                    let Some(node) = self.groups.at_mut(&to) else { return false };
                    let old = std::mem::take(node);
                    let (children, weights) = if side.before() { (vec![new, old], vec![1000, 1000]) } else { (vec![old, new], vec![1000, 1000]) };
                    *node = DocNode::Split { axis, children, weights };
                }
            }
        }
        self.normalize_groups();
        self.sync_documents(Some(doc.to_string()));
        true
    }

    /// Takes `doc` out of its group (the group showing a neighbour); the
    /// groups normalized.
    fn take_from_group(&mut self, doc: &str) {
        if let Some(path) = self.groups.path_of(doc) {
            if let Some(DocNode::Group { docs, active }) = self.groups.at_mut(&path) {
                if let Some(i) = docs.iter().position(|d| d == doc) {
                    docs.remove(i);
                    if *active > i || *active >= docs.len() {
                        *active = active.saturating_sub(1);
                    }
                }
            }
        }
        self.normalize_groups();
    }

    /// The split at `path` laid out at these extents (its splitter
    /// dragged): its weights.
    pub fn set_doc_weights(&mut self, path: &[usize], extents: &[i64]) {
        if let Some(DocNode::Split { weights, .. }) = self.groups.at_mut(path) {
            if weights.len() == extents.len() {
                *weights = extents.iter().map(|e| (*e).max(1)).collect();
            }
        }
    }

    /// Moves `pane` to index `at` of its group (or floating group), shown.
    pub fn move_tab(&mut self, pane: &str, at: usize) {
        let list = match self.find(pane) {
            Some(Where::Docked(path, _)) => match self.root.at_mut(&path) {
                Some(Node::Tabs { panes, active }) => Some((panes, active)),
                _ => None,
            },
            Some(Where::Floating(f, _)) => {
                let fl = &mut self.floating[f];
                Some((&mut fl.panes, &mut fl.active))
            }
            _ => None,
        };
        if let Some((panes, active)) = list {
            if let Some(i) = panes.iter().position(|p| p == pane) {
                let p = panes.remove(i);
                let at = at.min(panes.len());
                panes.insert(at, p);
                *active = at;
            }
        }
    }

    /// Takes `pane` out of the layout; where it was (to go back to), its
    /// group's extent read through `extent` (the path of a node and the
    /// axis it lies along).
    pub fn remove(&mut self, pane: &str, extent: &dyn Fn(&[usize], Axis) -> i64) -> Option<Place> {
        let place = match self.find(pane)? {
            Where::Docked(path, i) => {
                let len = match self.root.at(&path) {
                    Some(Node::Tabs { panes, .. }) => panes.len(),
                    _ => return None,
                };
                let place = if len > 1 {
                    let Some(Node::Tabs { panes, active }) = self.root.at_mut(&path) else { return None };
                    let other = if i + 1 < panes.len() { panes[i + 1].clone() } else { panes[i - 1].clone() };
                    panes.remove(i);
                    if *active > i || *active >= panes.len() {
                        *active = active.saturating_sub(1);
                    }
                    Place::Tab(other, i)
                } else {
                    let place = self.neighbour_place(&path, extent);
                    if let Some(Node::Tabs { panes, .. }) = self.root.at_mut(&path) {
                        panes.clear();
                    }
                    place
                };
                self.normalize();
                place
            }
            Where::AutoHide(side, i) => {
                self.autohide[side.index()].remove(i);
                Place::AutoHide(side)
            }
            Where::Floating(f, i) => {
                let fl = &mut self.floating[f];
                let rect = fl.rect;
                fl.panes.remove(i);
                if fl.active >= fl.panes.len() {
                    fl.active = fl.panes.len().saturating_sub(1);
                }
                if fl.panes.is_empty() {
                    self.floating.remove(f);
                }
                Place::Float(rect)
            }
            Where::Document(i) => {
                let was = self.active_document.and_then(|a| self.documents.get(a).cloned());
                let doc = self.documents[i].clone();
                // (the active one gone: its group's next shown, else the first group's)
                let keep = if was.as_deref() == Some(doc.as_str()) {
                    let path = self.groups.path_of(&doc);
                    self.take_from_group(&doc);
                    path.and_then(|p| match self.groups.at(&p) {
                        Some(DocNode::Group { docs, active }) => docs.get(*active).cloned(),
                        _ => None,
                    })
                } else {
                    self.take_from_group(&doc);
                    was
                };
                self.sync_documents(keep);
                Place::Document
            }
        };
        Some(place)
    }

    /// The place of the lone group at `path`: beside its neighbour in its
    /// split, else on the edge it's nearest.
    fn neighbour_place(&self, path: &[usize], extent: &dyn Fn(&[usize], Axis) -> i64) -> Place {
        let Some((&i, parent)) = path.split_last() else { return Place::Edge(Side::Left, DEFAULT_SIDE) };
        let Some(Node::Split { axis, children, .. }) = self.root.at(parent) else { return Place::Edge(Side::Left, DEFAULT_SIDE) };
        let size = extent(path, *axis);
        let (side_before, side_after) = match axis {
            Axis::Row => (Side::Left, Side::Right),
            Axis::Column => (Side::Top, Side::Bottom),
        };
        // (the neighbour after it: it was on its near side; else the one before)
        let (n, side) = if i + 1 < children.len() { (i + 1, side_before) } else { (i - 1, side_after) };
        match &children[n] {
            Node::Documents => Place::Beside(Anchor::Documents, side, size),
            Node::Tabs { panes, .. } => Place::Beside(Anchor::Pane(panes[0].clone()), side, size),
            // (beside a split: on the layout's edge at the top level, else
            // beside the split's pane nearest it)
            _ if parent.is_empty() => Place::Edge(side, size),
            split => {
                let mut all = Vec::new();
                split.panes(&mut all);
                let near = if side == side_before { all.first() } else { all.last() };
                match near {
                    Some(p) => Place::Beside(Anchor::Pane(p.clone()), side, size),
                    None => Place::Beside(Anchor::Documents, side, size),
                }
            }
        }
    }

    fn normalize(&mut self) {
        let root = std::mem::replace(&mut self.root, Node::Documents);
        self.root = root.normalized().unwrap_or(Node::Documents);
    }

    /// Puts `pane` (not placed now) at `target`, `size` pixels along the
    /// axis it docks on (0: its default; beside a group, at most half of
    /// it — `extent` reads a node's extent along an axis).
    pub fn insert(&mut self, pane: &str, target: &Target, size: i64, extent: &dyn Fn(&[usize], Axis) -> i64) -> bool {
        if self.find(pane).is_some() {
            return false;
        }
        match target {
            Target::Edge(side) => {
                let size = if size > 0 { size } else { default_extent(*side) };
                let root = std::mem::replace(&mut self.root, Node::Documents);
                self.root = wrap(root, Node::tabs(pane), *side, size, 0);
            }
            Target::Beside(anchor, side) => {
                let path = match anchor {
                    Anchor::Pane(p) => self.root.path_of(p),
                    Anchor::Documents => self.root.documents_path(),
                };
                let Some(path) = path else { return false };
                let room = extent(&path, side.axis());
                let size = if size > 0 { size } else { default_extent(*side) };
                let size = match anchor {
                    // (beside the documents: its own extent; beside a group: half of it at most)
                    Anchor::Documents => size,
                    Anchor::Pane(_) => size.min(((room - geometry::GAP) / 2).max(geometry::MIN_EXTENT)),
                };
                // (a split on the same axis takes it as a sibling)
                if let Some((&i, parent)) = path.split_last() {
                    if let Some(Node::Split { axis, children, sizes }) = self.root.at_mut(parent) {
                        if *axis == side.axis() {
                            let at = if side.before() { i } else { i + 1 };
                            let flex = Node::flex(children);
                            // (the anchor gives up the room unless it's the flex)
                            if flex != i && !matches!(anchor, Anchor::Documents) {
                                sizes[i] = (sizes[i] - size - geometry::GAP).max(geometry::MIN_EXTENT);
                            }
                            children.insert(at, Node::tabs(pane));
                            sizes.insert(at, size);
                            return true;
                        }
                    }
                }
                let Some(node) = self.root.at_mut(&path) else { return false };
                let old = std::mem::replace(node, Node::Documents);
                let rest = (room - size - geometry::GAP).max(geometry::MIN_EXTENT);
                *node = wrap(old, Node::tabs(pane), *side, size, rest);
            }
            Target::Into(Anchor::Pane(p)) => {
                let Some(path) = self.root.path_of(p) else {
                    // (into a floating group)
                    if let Some(Where::Floating(f, _)) = self.find(p) {
                        let fl = &mut self.floating[f];
                        fl.panes.push(pane.to_string());
                        fl.active = fl.panes.len() - 1;
                        return true;
                    }
                    return false;
                };
                if let Some(Node::Tabs { panes, active }) = self.root.at_mut(&path) {
                    panes.push(pane.to_string());
                    *active = panes.len() - 1;
                }
            }
            Target::Into(Anchor::Documents) => self.add_document(pane),
            Target::Float(rect) => self.floating.push(Floating { panes: vec![pane.to_string()], active: 0, rect: *rect }),
            Target::AutoHide(side) => self.autohide[side.index()].push(pane.to_string()),
        }
        true
    }

    // ------------------------------------------------------------ text --

    /// The layout as text ([`Layout::load`] reads it back exactly):
    ///
    /// ```text
    /// rapidr-dock 1
    /// mode mdi
    /// root split row
    ///   240 tabs 0 explorer outline
    ///   600 split column
    ///     400 documents
    ///     180 tabs 1 output problems
    ///   280 tabs 0 properties
    /// autohide left toolbox
    /// float 900 120 300 400 0 search
    /// documents 0 form1 module1
    /// hidden immediate
    /// ```
    pub fn save(&self, hidden: &[String]) -> String {
        let mut out = String::from("rapidr-dock 1\n");
        out.push_str(&format!("mode {}\n", self.mode.name()));
        out.push_str("root ");
        node_text(&self.root, 0, &mut out);
        for side in Side::ALL {
            let panes = &self.autohide[side.index()];
            if !panes.is_empty() {
                out.push_str(&format!("autohide {} {}\n", side.name(), panes.join(" ")));
            }
        }
        for f in &self.floating {
            let (l, t, w, h) = f.rect;
            out.push_str(&format!("float {l} {t} {w} {h} {} {}\n", f.active, f.panes.join(" ")));
        }
        if !self.documents.is_empty() {
            let active = self.active_document.map_or(-1, |a| a as i64);
            out.push_str(&format!("documents {active} {}\n", self.documents.join(" ")));
        }
        // (the document groups, when the documents are split)
        if matches!(self.groups, DocNode::Split { .. }) {
            out.push_str("groups ");
            doc_text(&self.groups, 0, &mut out);
        }
        if !hidden.is_empty() {
            out.push_str(&format!("hidden {}\n", hidden.join(" ")));
        }
        out
    }

    /// Reads [`Layout::save`]'s text; `known` says which panes exist (the
    /// others are dropped). The panes it lists as hidden come back too.
    pub fn load(text: &str, known: &dyn Fn(&str) -> bool) -> Result<(Layout, Vec<String>), String> {
        let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
        let mut it = lines.iter().peekable();
        match it.next().map(|l| l.split_whitespace().collect::<Vec<_>>()) {
            Some(h) if h.first() == Some(&"rapidr-dock") && h.get(1) == Some(&"1") => {}
            _ => return Err("not a RapidR dock layout".into()),
        }
        let mut layout = Layout::default();
        let mut hidden = Vec::new();
        let mut groups: Option<DocNode> = None;
        let mut seen = std::collections::HashSet::new();
        let mut keep = |p: &str| -> Option<String> {
            let p = p.to_ascii_lowercase();
            (known(&p) && seen.insert(p.clone())).then_some(p)
        };
        while let Some(line) = it.next() {
            let words: Vec<&str> = line.split_whitespace().collect();
            match words.first().copied() {
                Some("mode") => layout.mode = words.get(1).and_then(|m| DocumentMode::parse(m)).ok_or("bad mode")?,
                Some("root") => {
                    let first = words[1..].to_vec();
                    layout.root = parse_node(&first, 0, &mut it, &mut keep)?.unwrap_or(Node::Documents);
                }
                Some("autohide") => {
                    let side = words.get(1).and_then(|s| Side::parse(s)).ok_or("bad autohide side")?;
                    layout.autohide[side.index()].extend(words[2..].iter().filter_map(|p| keep(p)));
                }
                Some("float") => {
                    let n = |i: usize| words.get(i).and_then(|w| w.parse::<i64>().ok()).ok_or_else(|| "bad float".to_string());
                    let rect = (n(1)?, n(2)?, n(3)?, n(4)?);
                    let active = n(5)?.max(0) as usize;
                    let panes: Vec<String> = words[6..].iter().filter_map(|p| keep(p)).collect();
                    if !panes.is_empty() {
                        let active = active.min(panes.len() - 1);
                        layout.floating.push(Floating { panes, active, rect });
                    }
                }
                Some("documents") => {
                    let active = words.get(1).and_then(|w| w.parse::<i64>().ok()).ok_or("bad documents")?;
                    layout.documents = words[2..].iter().filter_map(|p| keep(p)).collect();
                    layout.active_document = (active >= 0 && !layout.documents.is_empty()).then(|| (active as usize).min(layout.documents.len() - 1));
                }
                Some("hidden") => hidden.extend(words[1..].iter().filter_map(|p| keep(p))),
                Some("groups") => {
                    let first = words[1..].to_vec();
                    groups = parse_doc(&first, 0, &mut it, known)?;
                }
                _ => return Err(format!("unknown line: {line}")),
            }
        }
        // (the groups hold the documents listed, in their places; the
        // others join the first group)
        let active = layout.active_document.and_then(|a| layout.documents.get(a).cloned());
        let listed = layout.documents.clone();
        let mut g = groups.and_then(|g| g.filter(&|d| listed.iter().any(|l| l == d)).normalized()).unwrap_or_default();
        let mut in_groups = Vec::new();
        g.docs(&mut in_groups);
        let first = g.groups().first().map(|x| x.0.clone()).unwrap_or_default();
        if let Some(DocNode::Group { docs, .. }) = g.at_mut(&first) {
            for d in &listed {
                if !in_groups.contains(d) {
                    docs.push(d.clone());
                }
            }
        }
        layout.groups = g;
        layout.sync_documents(active.clone());
        if let Some(i) = layout.active_document {
            layout.select_document(i);
        }
        // (the documents are always somewhere)
        if !layout.root.has_documents() {
            let root = std::mem::replace(&mut layout.root, Node::Documents);
            layout.root = Node::Split { axis: Axis::Row, children: vec![root, Node::Documents], sizes: vec![DEFAULT_SIDE, 0] };
        }
        layout.normalize();
        Ok((layout, hidden))
    }
}

fn doc_text(node: &DocNode, depth: usize, out: &mut String) {
    match node {
        DocNode::Group { docs, active } => out.push_str(&format!("group {active} {}\n", docs.join(" "))),
        DocNode::Split { axis, children, weights } => {
            out.push_str(&format!("split {}\n", axis.name()));
            for (c, w) in children.iter().zip(weights) {
                out.push_str(&"  ".repeat(depth + 1));
                out.push_str(&format!("{w} "));
                doc_text(c, depth + 1, out);
            }
        }
    }
}

fn parse_doc(words: &[&str], depth: usize, it: &mut Lines, known: &dyn Fn(&str) -> bool) -> Result<Option<DocNode>, String> {
    match words.first().copied() {
        Some("group") => {
            let active = words.get(1).and_then(|w| w.parse::<usize>().ok()).ok_or("bad group")?;
            let docs: Vec<String> = words[2..].iter().map(|p| p.to_ascii_lowercase()).filter(|p| known(p)).collect();
            Ok(Some(DocNode::Group { docs, active }))
        }
        Some("split") => {
            let axis = match words.get(1).copied() {
                Some("row") => Axis::Row,
                Some("column") => Axis::Column,
                _ => return Err("bad split".into()),
            };
            let (mut children, mut weights) = (Vec::new(), Vec::new());
            while let Some(line) = it.peek() {
                if depth_of(line) != depth + 1 {
                    break;
                }
                let line = it.next().unwrap();
                let w: Vec<&str> = line.split_whitespace().collect();
                let weight = w.first().and_then(|s| s.parse::<i64>().ok()).ok_or("bad weight")?;
                if let Some(n) = parse_doc(&w[1..], depth + 1, it, known)? {
                    children.push(n);
                    weights.push(weight.max(1));
                }
            }
            Ok(Some(DocNode::Split { axis, children, weights }))
        }
        _ => Err("bad group node".into()),
    }
}

/// `node` with `pane` beside it on `side` (`size` its extent, `rest` the
/// node's).
fn wrap(node: Node, pane: Node, side: Side, size: i64, rest: i64) -> Node {
    match node {
        Node::Split { axis, mut children, mut sizes } if axis == side.axis() => {
            if side.before() {
                children.insert(0, pane);
                sizes.insert(0, size);
            } else {
                children.push(pane);
                sizes.push(size);
            }
            Node::Split { axis, children, sizes }
        }
        node => {
            let (children, sizes) = if side.before() { (vec![pane, node], vec![size, rest]) } else { (vec![node, pane], vec![rest, size]) };
            Node::Split { axis: side.axis(), children, sizes }
        }
    }
}

fn node_text(node: &Node, depth: usize, out: &mut String) {
    match node {
        Node::Documents => out.push_str("documents\n"),
        Node::Tabs { panes, active } => out.push_str(&format!("tabs {active} {}\n", panes.join(" "))),
        Node::Split { axis, children, sizes } => {
            out.push_str(&format!("split {}\n", axis.name()));
            for (c, s) in children.iter().zip(sizes) {
                out.push_str(&"  ".repeat(depth + 1));
                out.push_str(&format!("{s} "));
                node_text(c, depth + 1, out);
            }
        }
    }
}

type Lines<'a, 'b> = std::iter::Peekable<std::slice::Iter<'b, &'a str>>;

fn depth_of(line: &str) -> usize {
    (line.len() - line.trim_start().len()) / 2
}

/// A node from its words (after its size), its children from the lines
/// below it indented one level more.
fn parse_node(words: &[&str], depth: usize, it: &mut Lines, keep: &mut dyn FnMut(&str) -> Option<String>) -> Result<Option<Node>, String> {
    match words.first().copied() {
        Some("documents") => Ok(Some(Node::Documents)),
        Some("tabs") => {
            let active = words.get(1).and_then(|w| w.parse::<usize>().ok()).ok_or("bad tabs")?;
            let panes: Vec<String> = words[2..].iter().filter_map(|p| keep(p)).collect();
            if panes.is_empty() {
                return Ok(None);
            }
            let active = active.min(panes.len() - 1);
            Ok(Some(Node::Tabs { panes, active }))
        }
        Some("split") => {
            let axis = match words.get(1).copied() {
                Some("row") => Axis::Row,
                Some("column") => Axis::Column,
                _ => return Err("bad split".into()),
            };
            let (mut children, mut sizes) = (Vec::new(), Vec::new());
            while let Some(line) = it.peek() {
                if depth_of(line) != depth + 1 {
                    break;
                }
                let line = it.next().unwrap();
                let w: Vec<&str> = line.split_whitespace().collect();
                let size = w.first().and_then(|s| s.parse::<i64>().ok()).ok_or("bad size")?;
                if let Some(n) = parse_node(&w[1..], depth + 1, it, keep)? {
                    children.push(n);
                    sizes.push(size);
                }
            }
            Ok(match children.len() {
                0 => None,
                1 => children.pop(),
                _ => Some(Node::Split { axis, children, sizes }),
            })
        }
        _ => Err("bad node".into()),
    }
}

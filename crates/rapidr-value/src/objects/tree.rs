//! QTREEVIEW's nodes (RapidQ manual, Appendix A; Delphi's TTreeNodes): the
//! runtimes show them (`with_tree`); the program changes them through the
//! methods and `Item(i)` below, the same on the desktop and the web.
//!
//! Nodes are indexed in depth-first order (a node, then its children, then
//! its next sibling), as RapidQ numbers them: after `AddItems "1", "2"` and
//! `AddChildItems 0, "a", "b"` the nodes are 0 = "1", 1 = "a", 2 = "b",
//! 3 = "2". They're kept in that order, each with its level, so a node's
//! subtree is the run of nodes after it with a deeper level.
//!
//! * `AddItems s, …` (top-level nodes at the end), `AddChildItems i, s, …`
//!   (last children of node i), `InsertItem i, s` (before node i, beside
//!   it), `DelItems i, …` (with their subtrees), `Clear`, `Sort`;
//! * `Expand i, recurse`, `Collapse i, recurse`, `FullExpand`,
//!   `FullCollapse`, `GetItemAt(x, y)` (in rows of the runtime's height);
//! * `Item(i).Text` / `.ImageIndex` / `.SelectedIndex` / `.StateIndex` /
//!   `.HasChildren` / `.Selected` / `.Expanded`, read-only `.Count` /
//!   `.Level` / `.IsVisible` / `.Handle` / `.Index` / `.Parent`;
//! * `ItemCount`, `ItemIndex`, `TopIndex`, `ShowButtons`, `ShowLines`,
//!   `ShowRoot`, `Indent`, `ReadOnly`, `HideSelection`, `SortType`;
//! * `LoadFromFile` / `SaveToFile`: a node per line, indented by a tab per
//!   level ([`TreeView::to_text`] / [`TreeView::load_text`]).
//!
//! What the user does goes through the runtime, which asks the program
//! first (OnChanging, OnExpanding, OnCollapsing, OnEditing, OnEdited answer
//! back) and then changes the nodes here. Indexes out of range read as
//! "" / 0 and are ignored when written, as RapidQ doesn't stop for them.

use crate::{v_int, v_str, Value};

/// Most nodes a tree holds.
pub const MAX_NODES: usize = 1_000_000;

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub text: String,
    pub level: usize,
    pub image_index: i64,
    pub selected_index: i64,
    pub state_index: i64,
    pub expanded: bool,
    /// `HasChildren = True` shows the node's button before it has any
    /// (folder trees fill them in when it's expanded).
    pub has_children: bool,
}

impl Node {
    fn new(text: String, level: usize) -> Node {
        Node { text, level, image_index: 0, selected_index: 0, state_index: -1, expanded: false, has_children: false }
    }
}

#[derive(Clone, Debug)]
pub struct TreeView {
    pub nodes: Vec<Node>,
    pub item_index: i64,
    pub top_index: i64,
    pub show_buttons: bool,
    pub show_lines: bool,
    pub show_root: bool,
    pub indent: i64,
    /// ItemHeight (QOUTLINE's, RapidR's for a tree view): its rows' height;
    /// 0 — never set — is Windows' [`ROW_HEIGHT`].
    pub item_height: i64,
    pub read_only: bool,
    pub hide_selection: bool,
    pub sort_type: i64,
    /// Nodes deleted since the runtime last asked (OnDeletion).
    deleted: Vec<usize>,
    /// Bumped by every change of the nodes (the runtime rebuilds its widget).
    pub version: u64,
}

impl Default for TreeView {
    fn default() -> Self {
        TreeView {
            nodes: Vec::new(),
            item_index: -1,
            top_index: 0,
            show_buttons: true,
            show_lines: true,
            show_root: true,
            indent: 19,
            item_height: 0,
            read_only: false,
            hide_selection: false,
            sort_type: 0,
            deleted: Vec::new(),
            version: 0,
        }
    }
}

fn flag(on: bool) -> Value {
    v_int(on as i64)
}

fn index(v: Option<&Value>) -> Option<usize> {
    usize::try_from(v?.to_i64()).ok()
}

impl TreeView {
    fn changed(&mut self) {
        self.version = self.version.wrapping_add(1);
    }

    /// The end of node `i`'s subtree (exclusive).
    pub fn subtree_end(&self, i: usize) -> usize {
        let level = self.nodes[i].level;
        self.nodes[i + 1..].iter().position(|n| n.level <= level).map_or(self.nodes.len(), |p| i + 1 + p)
    }

    /// Node `i`'s parent (`None` for a top-level node).
    pub fn parent(&self, i: usize) -> Option<usize> {
        let level = self.nodes.get(i)?.level;
        (0..i).rev().find(|&k| self.nodes[k].level < level)
    }

    /// Node `i`'s children.
    pub fn children(&self, i: usize) -> Vec<usize> {
        let (level, end) = (self.nodes[i].level, self.subtree_end(i));
        (i + 1..end).filter(|&k| self.nodes[k].level == level + 1).collect()
    }

    /// Whether node `i` shows a button (it has children, or says so).
    pub fn has_children(&self, i: usize) -> bool {
        self.nodes.get(i + 1).is_some_and(|n| n.level > self.nodes[i].level) || self.nodes[i].has_children
    }

    /// Whether every ancestor of node `i` is expanded (it has a row).
    pub fn is_visible(&self, i: usize) -> bool {
        let mut k = i;
        while let Some(p) = self.parent(k) {
            if !self.nodes[p].expanded {
                return false;
            }
            k = p;
        }
        i < self.nodes.len()
    }

    /// The nodes that have a row, top to bottom.
    pub fn visible_rows(&self) -> Vec<usize> {
        let mut rows = Vec::new();
        let mut i = 0;
        while i < self.nodes.len() {
            rows.push(i);
            i = if self.nodes[i].expanded { i + 1 } else { self.subtree_end(i) };
        }
        rows
    }

    /// `GetItemAt(x, y)`: the node in the row at `y` (rows `row_height`
    /// tall, from `TopIndex`), or -1.
    pub fn item_at(&self, _x: i64, y: i64, row_height: i64) -> i64 {
        if y < 0 || row_height <= 0 {
            return -1;
        }
        let rows = self.visible_rows();
        let top = rows.iter().position(|&r| r as i64 >= self.top_index).unwrap_or(0);
        rows.get(top + (y / row_height) as usize).map_or(-1, |&r| r as i64)
    }

    /// Selects node `i` (-1: none); its ancestors expand to show it.
    pub fn select(&mut self, i: i64) {
        let i = if i >= 0 && (i as usize) < self.nodes.len() { i } else { -1 };
        if i >= 0 {
            let mut k = i as usize;
            while let Some(p) = self.parent(k) {
                self.nodes[p].expanded = true;
                k = p;
            }
        }
        self.item_index = i;
        self.changed();
    }

    /// Expands (or collapses) node `i`, with `recurse` its whole subtree.
    /// Node `i`'s text (the user edited it).
    pub fn set_text(&mut self, i: usize, text: String) {
        if let Some(n) = self.nodes.get_mut(i) {
            if n.text != text {
                n.text = text;
                self.changed();
            }
        }
    }

    pub fn set_expanded(&mut self, i: usize, on: bool, recurse: bool) {
        if i >= self.nodes.len() {
            return;
        }
        let end = if recurse { self.subtree_end(i) } else { i + 1 };
        for n in &mut self.nodes[i..end] {
            n.expanded = on;
        }
        // (a collapsed node's selected descendant: the node is selected)
        if !on && self.item_index > i as i64 && (self.item_index as usize) < self.subtree_end(i) {
            self.item_index = i as i64;
        }
        self.changed();
    }

    fn insert(&mut self, at: usize, texts: impl IntoIterator<Item = String>, level: usize) {
        let new: Vec<Node> = texts.into_iter().take(MAX_NODES.saturating_sub(self.nodes.len())).map(|t| Node::new(t, level)).collect();
        let n = new.len();
        if n == 0 {
            return;
        }
        if self.item_index >= at as i64 {
            self.item_index += n as i64;
        }
        self.nodes.splice(at..at, new);
        self.changed();
    }

    /// Deletes node `i` and its subtree (OnDeletion for each, top first).
    fn delete(&mut self, i: usize) {
        if i >= self.nodes.len() {
            return;
        }
        let end = self.subtree_end(i);
        self.deleted.extend(i..end);
        self.nodes.drain(i..end);
        let (i, n) = (i as i64, (end - i) as i64);
        if self.item_index >= i + n {
            self.item_index -= n;
        } else if self.item_index >= i {
            self.item_index = -1;
        }
        self.changed();
    }

    /// What the widget's items are built from: the nodes' texts and levels
    /// (a change rebuilds them; expanded, selected and images are synced).
    pub fn shape_hash(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        for n in &self.nodes {
            (&n.text, n.level).hash(&mut h);
        }
        h.finish()
    }

    /// What the tree shows, images aside: its shape, which nodes are shown
    /// and the selected one. OnGetImageIndex asks again only when this
    /// changed — never because the program set a node's image.
    pub fn view_hash(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        (self.shape_hash(), self.visible_rows(), self.item_index).hash(&mut h);
        h.finish()
    }

    /// Nodes deleted since last asked, as they were numbered (OnDeletion).
    pub fn take_deleted(&mut self) -> Vec<usize> {
        std::mem::take(&mut self.deleted)
    }

    /// Sorts the nodes by their text, siblings among themselves (subtrees
    /// move with their nodes).
    pub fn sort(&mut self) {
        fn sorted(nodes: &[Node]) -> Vec<Node> {
            if nodes.is_empty() {
                return Vec::new();
            }
            let level = nodes[0].level;
            let mut groups: Vec<Vec<Node>> = Vec::new();
            for n in nodes {
                if n.level == level || groups.is_empty() {
                    groups.push(vec![n.clone()]);
                } else if let Some(g) = groups.last_mut() {
                    g.push(n.clone());
                }
            }
            groups.sort_by(|a, b| a[0].text.to_lowercase().cmp(&b[0].text.to_lowercase()));
            groups.into_iter().flat_map(|g| std::iter::once(g[0].clone()).chain(sorted(&g[1..]))).collect()
        }
        let selected = usize::try_from(self.item_index).ok().and_then(|i| self.nodes.get(i)).cloned();
        self.nodes = sorted(&self.nodes);
        self.item_index = selected.and_then(|s| self.nodes.iter().position(|n| *n == s)).map_or(-1, |i| i as i64);
        self.changed();
    }

    /// The nodes as text: a line per node, a tab per level before it.
    pub fn to_text(&self) -> String {
        self.nodes.iter().map(|n| format!("{}{}\r\n", "\t".repeat(n.level), n.text)).collect()
    }

    /// Replaces the nodes with `text`'s (see [`Self::to_text`]; a line
    /// can't be more than one level deeper than the one before it).
    pub fn load_text(&mut self, text: &str) {
        self.nodes.clear();
        self.item_index = -1;
        let mut prev: Option<usize> = None;
        for line in text.lines().filter(|l| !l.trim().is_empty()).take(MAX_NODES) {
            let tabs = line.len() - line.trim_start_matches('\t').len();
            let level = prev.map_or(0, |p| tabs.min(p + 1));
            self.nodes.push(Node::new(line[tabs..].trim_end_matches('\r').to_string(), level));
            prev = Some(level);
        }
        self.changed();
    }

    pub fn get(&self, prop: &str) -> Option<Value> {
        Some(match prop {
            "itemcount" | "count" | "linecount" => v_int(self.nodes.len() as i64),
            "itemindex" | "row" => v_int(self.item_index),
            "topindex" => v_int(self.top_index),
            "showbuttons" => flag(self.show_buttons),
            "showlines" => flag(self.show_lines),
            "showroot" => flag(self.show_root),
            "indent" => v_int(self.indent),
            "itemheight" if self.item_height > 0 => v_int(self.item_height),
            "readonly" => flag(self.read_only),
            "hideselection" => flag(self.hide_selection),
            "sorttype" => v_int(self.sort_type),
            // RapidR's older name: the selected node's text.
            "selecteditem" => v_str(usize::try_from(self.item_index).ok().and_then(|i| self.nodes.get(i)).map_or("", |n| &n.text)),
            _ => return None,
        })
    }

    pub fn set(&mut self, prop: &str, val: &Value) -> bool {
        match prop {
            "itemindex" | "row" => self.select(val.to_i64()),
            "topindex" => self.top_index = val.to_i64().clamp(0, self.nodes.len().saturating_sub(1) as i64),
            "showbuttons" => self.show_buttons = val.to_bool(),
            "showlines" => self.show_lines = val.to_bool(),
            "showroot" => self.show_root = val.to_bool(),
            "indent" => self.indent = val.to_i64().clamp(0, 1_000),
            "itemheight" => {
                self.item_height = val.to_i64().clamp(0, 400);
                self.version += 1;
            }
            "readonly" => self.read_only = val.to_bool(),
            "hideselection" => self.hide_selection = val.to_bool(),
            "sorttype" => {
                self.sort_type = val.to_i64();
                if self.sort_type != 0 {
                    self.sort();
                }
            }
            _ => return false,
        }
        self.changed();
        true
    }

    pub fn call(&mut self, method: &str, args: &[Value]) -> Option<Value> {
        let texts = |from: usize| args.iter().skip(from).map(|a| a.to_string_val()).collect::<Vec<_>>();
        let recurse = args.get(1).is_some_and(Value::to_bool);
        match method {
            "additems" | "additem" => {
                let at = self.nodes.len();
                self.insert(at, texts(0), 0);
            }
            "addchilditems" => {
                let Some(i) = index(args.first()).filter(|&i| i < self.nodes.len()) else { return Some(Value::Null) };
                let (at, level) = (self.subtree_end(i), self.nodes[i].level + 1);
                self.insert(at, texts(1), level);
            }
            "insertitem" => {
                let (at, level) = match index(args.first()).filter(|&i| i < self.nodes.len()) {
                    Some(i) => (i, self.nodes[i].level),
                    None => (self.nodes.len(), 0),
                };
                self.insert(at, texts(1).into_iter().take(1), level);
            }
            "delitems" | "delitem" => {
                let mut gone: Vec<usize> = args.iter().filter_map(|a| index(Some(a))).collect();
                gone.sort_unstable();
                gone.dedup();
                for i in gone.into_iter().rev() {
                    self.delete(i);
                }
            }
            "clear" => {
                self.deleted.extend(0..self.nodes.len());
                self.nodes.clear();
                self.item_index = -1;
                self.top_index = 0;
                self.changed();
            }
            "expand" => self.set_expanded(index(args.first()).unwrap_or(usize::MAX), true, recurse),
            "collapse" => self.set_expanded(index(args.first()).unwrap_or(usize::MAX), false, recurse),
            "fullexpand" | "fullcollapse" => {
                let on = method == "fullexpand";
                self.nodes.iter_mut().for_each(|n| n.expanded = on);
                if !on {
                    self.select(usize::try_from(self.item_index).ok().map_or(-1, |i| {
                        let mut k = i;
                        while let Some(p) = self.parent(k) {
                            k = p;
                        }
                        k as i64
                    }));
                }
                self.changed();
            }
            "sort" | "alphasort" => self.sort(),
            // RapidR's older names: AddRoot(s), AddChild(parent text, s).
            "addroot" => {
                let at = self.nodes.len();
                self.insert(at, texts(0).into_iter().take(1), 0);
            }
            // QOUTLINE: AddLines "Parent", " Child", "  Grandchild" — each
            // leading space (or tab) a level deeper.
            "addlines" => {
                for line in texts(0) {
                    let depth = line.chars().take_while(|c| matches!(c, ' ' | '\t')).count();
                    let max = self.nodes.last().map_or(0, |n| n.level + 1);
                    let at = self.nodes.len();
                    self.insert(at, [line.trim_start_matches([' ', '\t']).to_string()], depth.min(max));
                }
            }
            // QOUTLINE's AddChild(Index, S) and Insert(Index, S).
            "addchild" if matches!(args.first(), Some(Value::Integer(_) | Value::Double(_))) => {
                return self.call("addchilditems", args);
            }
            "insert" => return self.call("insertitem", args),
            "dellines" => return self.call("delitems", args),
            "addoptions" | "deloptions" => {}
            // QOUTLINE's Item(i): a line's text (`Item(i) = s`: the value
            // comes as a second argument).
            "item" if args.len() >= 2 => return self.call("item=", args),
            "item" => {
                let i = index(args.first()).filter(|&i| i < self.nodes.len());
                return Some(v_str(i.map_or("", |i| &self.nodes[i].text)));
            }
            "item=" => {
                if let Some(i) = index(args.first()).filter(|&i| i < self.nodes.len()) {
                    self.nodes[i].text = args.get(1).map(|v| v.to_string_val()).unwrap_or_default();
                    self.changed();
                }
            }
            "addchild" => {
                let parent = args.first().map(|v| v.to_string_val()).unwrap_or_default();
                match self.nodes.iter().rposition(|n| n.text == parent) {
                    Some(i) => {
                        let (at, level) = (self.subtree_end(i), self.nodes[i].level + 1);
                        self.insert(at, texts(1).into_iter().take(1), level);
                    }
                    None => {
                        let at = self.nodes.len();
                        self.insert(at, texts(1).into_iter().take(1), 0);
                    }
                }
            }
            "addnode" => {
                let at = self.nodes.len();
                self.insert(at, texts(0).into_iter().take(1), 0);
            }
            _ => return self.item(method, args),
        }
        Some(Value::Null)
    }

    /// `Item(i).Text`, `Item(i).Expanded = True`, … (compiled as the
    /// methods `item.text` / `item.expanded=` with the index first).
    fn item(&mut self, method: &str, args: &[Value]) -> Option<Value> {
        let member = method.strip_prefix("item.")?;
        let setter = member.ends_with('=');
        let member = member.trim_end_matches('=');
        let i = index(args.first()).filter(|&i| i < self.nodes.len());
        if setter {
            let val = args.get(1).cloned().unwrap_or(Value::Null);
            let Some(i) = i else {
                return matches!(member, "text" | "imageindex" | "selectedindex" | "stateindex" | "haschildren" | "selected" | "expanded").then_some(Value::Null);
            };
            match member {
                "text" => self.nodes[i].text = val.to_string_val(),
                "imageindex" => self.nodes[i].image_index = val.to_i64(),
                "selectedindex" => self.nodes[i].selected_index = val.to_i64(),
                "stateindex" => self.nodes[i].state_index = val.to_i64(),
                "haschildren" => self.nodes[i].has_children = val.to_bool(),
                "selected" if val.to_bool() => self.select(i as i64),
                "selected" => {
                    if self.item_index == i as i64 {
                        self.item_index = -1;
                    }
                }
                "expanded" => self.set_expanded(i, val.to_bool(), false),
                _ => return None,
            }
            self.changed();
            return Some(Value::Null);
        }
        let node = i.map(|i| &self.nodes[i]);
        Some(match member {
            "text" => v_str(node.map_or("", |n| &n.text)),
            "imageindex" => v_int(node.map_or(0, |n| n.image_index)),
            "selectedindex" => v_int(node.map_or(0, |n| n.selected_index)),
            "stateindex" => v_int(node.map_or(-1, |n| n.state_index)),
            "level" => v_int(node.map_or(0, |n| n.level as i64)),
            "expanded" => flag(node.is_some_and(|n| n.expanded)),
            "count" => v_int(i.map_or(0, |i| self.children(i).len() as i64)),
            "haschildren" => flag(i.is_some_and(|i| self.has_children(i))),
            "selected" => flag(i.is_some_and(|i| self.item_index == i as i64)),
            "isvisible" => flag(i.is_some_and(|i| self.is_visible(i))),
            "index" | "absoluteindex" => v_int(i.map_or(-1, |i| i as i64)),
            "parent" => v_int(i.and_then(|i| self.parent(i)).map_or(-1, |p| p as i64)),
            // A stable number for the node while the tree doesn't change.
            "handle" => v_int(i.map_or(0, |i| i as i64 + 1)),
            _ => return None,
        })
    }
}

// ------------------------------------------------------------- drawing --
//
// The tree as Windows' tree view lays it out, for a runtime that draws it
// itself (RapidR's UI kernel; the web can follow): a row per shown node,
// from TopIndex, each `row_height` tall; a node's level is a column
// `Indent` wide (one more with ShowRoot, whose top-level nodes have
// buttons and lines too); its button (ShowButtons, a node with children)
// and the lines to its siblings (ShowLines) are centred in the column
// before its own; its icons (state image, image) and text follow.

/// A tree's rows' height (GetItemAt's rows, the kernel's drawing): 16, as
/// Windows' tree view shows MS Sans Serif 8 (a line and 3; RapidQ's capture).
pub const ROW_HEIGHT: i64 = 16;
/// A button's box (9 × 9, centred on its column's middle).
pub const BUTTON: i64 = 9;

/// A shown node's row, as drawn.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub node: usize,
    /// Its top in the tree's area (TopIndex's row at 0).
    pub top: i64,
    pub height: i64,
    /// Where its icons, then its text, start.
    pub left: i64,
    /// The middle of the column its button and lines are in (`None`: a
    /// top-level node without ShowRoot has none).
    pub center: Option<i64>,
    /// Its button: `Some(expanded)` with ShowButtons when it has children.
    pub button: Option<bool>,
    /// A sibling follows it (its line goes on down).
    pub more: bool,
    /// For each column left of its own, whether a line passes through
    /// (that level's ancestor has a sibling after it), with the column's
    /// middle.
    pub through: Vec<i64>,
    pub selected: bool,
}

/// What's at a point of the tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    /// Node `n`'s button (expands / collapses it).
    Button(usize),
    /// Node `n`'s row elsewhere (selects it).
    Row(usize),
}

impl TreeView {
    /// The width of a level's column.
    pub fn column_width(&self) -> i64 {
        self.indent.clamp(8, 1_000)
    }

    /// Node `i`'s column (0: the leftmost; ShowRoot moves every node one
    /// to the right, for the top level's buttons and lines).
    fn column(&self, i: usize) -> i64 {
        self.nodes[i].level as i64 + i64::from(self.show_root)
    }

    /// Whether a sibling follows node `i`.
    fn has_next_sibling(&self, i: usize) -> bool {
        let end = self.subtree_end(i);
        self.nodes.get(end).is_some_and(|n| n.level == self.nodes[i].level)
    }

    /// Its rows' height: ItemHeight, or Windows' 16.
    pub fn row_height(&self) -> i64 {
        if self.item_height > 0 { self.item_height } else { ROW_HEIGHT }
    }

    /// The rows shown from TopIndex, `row_height` tall, up to `height`
    /// pixels of them (a row cut at the bottom included).
    pub fn rows(&self, row_height: i64, height: i64) -> Vec<Row> {
        let rh = row_height.max(1);
        let cw = self.column_width();
        let rows = self.visible_rows();
        let first = rows.iter().position(|&r| r as i64 >= self.top_index).unwrap_or(0);
        let mut out = Vec::new();
        for (k, &i) in rows.iter().enumerate().skip(first) {
            let top = (k - first) as i64 * rh;
            if top >= height.max(rh) {
                break;
            }
            let col = self.column(i);
            let center = (col > 0).then(|| (col - 1) * cw + cw / 2);
            // (the ancestors' columns a line passes through)
            let mut through = Vec::new();
            let mut a = self.parent(i);
            while let Some(p) = a {
                let pc = self.column(p);
                if pc > 0 && self.has_next_sibling(p) {
                    through.push((pc - 1) * cw + cw / 2);
                }
                a = self.parent(p);
            }
            through.reverse();
            out.push(Row {
                node: i,
                top,
                height: rh,
                left: col * cw + 2,
                center,
                button: (self.show_buttons && center.is_some() && self.has_children(i)).then(|| self.nodes[i].expanded),
                more: self.has_next_sibling(i),
                through,
                selected: self.item_index == i as i64,
            });
        }
        out
    }

    /// What is at (x, y) of the tree's area, rows `row_height` tall.
    pub fn hit(&self, x: i64, y: i64, row_height: i64) -> Option<Hit> {
        if y < 0 {
            return None;
        }
        let rh = row_height.max(1);
        let row = self.rows(rh, y + rh).into_iter().find(|r| y >= r.top && y < r.top + r.height)?;
        if let (Some(c), Some(_)) = (row.center, row.button) {
            if (x - c).abs() <= BUTTON / 2 + 1 {
                return Some(Hit::Button(row.node));
            }
        }
        Some(Hit::Row(row.node))
    }

    /// Where node `i`'s row is (top, and its button's middle), if shown.
    pub fn row_of(&self, i: usize, row_height: i64, height: i64) -> Option<Row> {
        self.rows(row_height, height).into_iter().find(|r| r.node == i)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(t: &str) -> Value {
        v_str(t)
    }

    fn texts(t: &TreeView) -> Vec<String> {
        t.nodes.iter().map(|n| format!("{}{}", ".".repeat(n.level), n.text)).collect()
    }

    #[test]
    fn nodes_in_rapidq_order() {
        // Three nodes, three children under the first and the second.
        let mut t = TreeView::default();
        t.call("additems", &[s("North"), s("South"), s("East")]);
        t.call("addchilditems", &[v_int(0), s("Hill"), s("Lake"), s("Wood")]);
        t.call("addchilditems", &[v_int(4), s("Port"), s("Bay"), s("Dune")]);
        assert_eq!(texts(&t), ["North", ".Hill", ".Lake", ".Wood", "South", ".Port", ".Bay", ".Dune", "East"]);
        assert_eq!(t.get("itemcount").unwrap().to_i64(), 9);
        assert_eq!(t.call("item.count", &[v_int(4)]).unwrap().to_i64(), 3);
        assert_eq!(t.call("item.level", &[v_int(5)]).unwrap().to_i64(), 1);
        assert_eq!(t.call("item.parent", &[v_int(6)]).unwrap().to_i64(), 4);
        assert_eq!(t.call("item.haschildren", &[v_int(8)]).unwrap().to_i64(), 0);
        // Nothing expanded: the top-level nodes have rows.
        assert_eq!(t.visible_rows(), [0, 4, 8]);
        t.call("fullexpand", &[]);
        assert_eq!(t.visible_rows().len(), 9);
        assert_eq!(t.item_at(5, 3 * 18 + 2, 18), 3);
        t.call("item.text=", &[v_int(2), s("two")]);
        assert_eq!(t.call("item.text", &[v_int(2)]).unwrap().to_string_val(), "two");
    }

    #[test]
    fn insert_delete_select() {
        let mut t = TreeView::default();
        t.call("additems", &[s("a"), s("b")]);
        t.call("addchilditems", &[v_int(1), s("b1"), s("b2")]);
        t.set("itemindex", &v_int(3));
        // Selecting a hidden node expands its parent.
        assert!(t.nodes[1].expanded && t.is_visible(3));
        t.call("insertitem", &[v_int(1), s("a2")]);
        assert_eq!(texts(&t), ["a", "a2", "b", ".b1", ".b2"]);
        assert_eq!(t.get("itemindex").unwrap().to_i64(), 4);
        // Deleting a node takes its subtree (OnDeletion for each).
        t.call("delitems", &[v_int(2)]);
        assert_eq!(texts(&t), ["a", "a2"]);
        assert_eq!(t.take_deleted(), [2, 3, 4]);
        assert_eq!(t.get("itemindex").unwrap().to_i64(), -1);
        // Collapsing a node whose child is selected selects the node.
        t.call("addchilditems", &[v_int(0), s("x")]);
        t.set("itemindex", &v_int(1));
        t.call("collapse", &[v_int(0), v_int(0)]);
        assert_eq!(t.get("itemindex").unwrap().to_i64(), 0);
        // Out of range: ignored.
        assert!(t.call("item.text=", &[v_int(99), s("z")]).is_some());
        assert_eq!(t.call("item.text", &[v_int(99)]).unwrap().to_string_val(), "");
    }

    #[test]
    fn sort_and_text() {
        let mut t = TreeView::default();
        t.call("additems", &[s("pear"), s("Apple")]);
        t.call("addchilditems", &[v_int(0), s("z"), s("b")]);
        t.set("itemindex", &v_int(1));
        t.call("sort", &[]);
        assert_eq!(texts(&t), ["Apple", "pear", ".b", ".z"]);
        assert_eq!(t.nodes[t.get("itemindex").unwrap().to_i64() as usize].text, "z");
        let text = t.to_text();
        assert_eq!(text, "Apple\r\npear\r\n\tb\r\n\tz\r\n");
        let mut u = TreeView::default();
        u.load_text("root\n\t\tdeep\n\tchild\nnext\n");
        assert_eq!(texts(&u), ["root", ".deep", ".child", "next"]);
    }

    #[test]
    fn outline_lines() {
        let mut t = TreeView::default();
        t.call("addlines", &[s("Parent 1"), s(" Child"), s("  Grandchild"), s("Parent 2"), s("   too deep")]);
        assert_eq!(t.nodes.iter().map(|n| n.level).collect::<Vec<_>>(), vec![0, 1, 2, 0, 1]);
        assert_eq!(t.call("item", &[v_int(2)]).unwrap().to_string_val(), "Grandchild");
        t.call("addchild", &[v_int(0), s("Second child")]);
        assert_eq!(texts(&t), vec!["Parent 1", ".Child", "..Grandchild", ".Second child", "Parent 2", ".too deep"]);
        t.call("item=", &[v_int(4), s("P2")]);
        t.set("row", &v_int(4));
        assert_eq!((t.get("linecount").unwrap().to_i64(), t.get("row").unwrap().to_i64(), t.nodes[4].text.as_str()), (6, 4, "P2"));
    }

    #[test]
    fn rapidr_older_names() {
        let mut t = TreeView::default();
        t.call("addroot", &[s("Project1")]);
        t.call("addchild", &[s("Project1"), s("Form1 (Form)")]);
        t.call("addchild", &[s("Form1 (Form)"), s("Button1")]);
        assert_eq!(texts(&t), ["Project1", ".Form1 (Form)", "..Button1"]);
        t.set("itemindex", &v_int(2));
        assert_eq!(t.get("selecteditem").unwrap().to_string_val(), "Button1");
    }

    #[test]
    fn rows_as_drawn() {
        let mut t = TreeView::default();
        t.call("additems", &[s("1"), s("2")]);
        t.call("addchilditems", &[v_int(0), s("a"), s("b")]);
        // (collapsed: two rows; ShowRoot gives the top level a column)
        let rows = t.rows(ROW_HEIGHT, 200);
        assert_eq!(rows.iter().map(|r| r.node).collect::<Vec<_>>(), [0, 3]);
        assert_eq!((rows[0].center, rows[0].button, rows[0].left, rows[0].more), (Some(9), Some(false), 21, true));
        assert_eq!((rows[1].button, rows[1].more), (None, false));
        assert_eq!(t.hit(9, 5, ROW_HEIGHT), Some(Hit::Button(0)));
        assert_eq!(t.hit(40, 5, ROW_HEIGHT), Some(Hit::Row(0)));
        assert_eq!(t.hit(40, 20, ROW_HEIGHT), Some(Hit::Row(3)));
        assert_eq!(t.hit(40, 40, ROW_HEIGHT), None);
        t.set_expanded(0, true, false);
        let rows = t.rows(ROW_HEIGHT, 200);
        assert_eq!(rows.iter().map(|r| (r.node, r.top)).collect::<Vec<_>>(), [(0, 0), (1, 16), (2, 32), (3, 48)]);
        // (a child: its column's middle, the root's line passing through)
        assert_eq!((rows[1].center, rows[1].left, rows[1].through.clone(), rows[2].more), (Some(28), 40, vec![9], false));
        // (cut at the height)
        assert_eq!(t.rows(ROW_HEIGHT, 30).len(), 2);
        t.show_root = false;
        assert_eq!((t.rows(ROW_HEIGHT, 200)[0].center, t.rows(ROW_HEIGHT, 200)[0].button), (None, None));
    }
}

//! The designed form: a tree of components as its CREATE blocks say them.
//!
//! Each [`Node`] is one `CREATE Name AS Type … END CREATE`: its name, its
//! type as written (`QBUTTON`) and as RapidR's (`RBUTTON`), and its body in
//! source order — property assignments ([`Prop`], the value as source
//! text), nested CREATEs ([`Item::Child`]) and statements the designer
//! doesn't own ([`Item::Code`]: `Center`, an IF …, kept as text). The order
//! is the program's: it creates components and sets properties in it, so
//! layout replays it ([`super::layout`]); a parent's children in body order
//! are its creation order, which is the z-order (the last on top) and the
//! default Tab order.

use std::collections::BTreeMap;

pub type NodeId = u32;

/// A property assignment in a CREATE block: `Left = 10`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Prop {
    /// As written (`Left`, `Font.Name`, `Constraints.MinWidth`).
    pub name: String,
    /// The value's source text (`10`, `"OK"`, `akLeft + akRight`).
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Item {
    Prop(Prop),
    Child(NodeId),
    /// A statement the designer shows but doesn't own (its text).
    Code(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub id: NodeId,
    pub name: String,
    /// The type as the source writes it (`QBUTTON`, `RPLOT`): kept as it
    /// is (docs/q-and-r-components.md).
    pub type_written: String,
    /// RapidR's name for it (`RBUTTON`).
    pub canonical: String,
    pub parent: Option<NodeId>,
    pub body: Vec<Item>,
}

/// A property's key: lower case, `Constraints.MinWidth` as `minwidth` (one
/// property, two spellings, as the runtimes store it).
pub fn prop_key(name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    match crate::layout::constraint_alias(&lower) {
        Some(flat) => flat.to_string(),
        None => lower,
    }
}

/// RapidR's name of a type as written (`QBUTTON` → `RBUTTON`, `QGAUGE` →
/// `RPROGRESSBAR`); upper case as written when the registry doesn't know it.
pub fn canonical_type(written: &str) -> String {
    match rapidr_lang::component(written) {
        Some(c) => c.name.to_string(),
        None => written.to_ascii_uppercase(),
    }
}

impl Node {
    pub fn new(id: NodeId, name: &str, type_written: &str) -> Node {
        Node { id, name: name.to_string(), type_written: type_written.to_string(), canonical: canonical_type(type_written), parent: None, body: Vec::new() }
    }

    /// The index in `body` of the last assignment of `name` (the one that
    /// counts: the program runs them in order).
    pub fn prop_index(&self, name: &str) -> Option<usize> {
        let key = prop_key(name);
        self.body.iter().rposition(|i| matches!(i, Item::Prop(p) if prop_key(&p.name) == key))
    }

    /// The value text of property `name` (its last assignment).
    pub fn prop(&self, name: &str) -> Option<&str> {
        self.prop_index(name).and_then(|i| match &self.body[i] {
            Item::Prop(p) => Some(p.value.as_str()),
            _ => None,
        })
    }

    /// Its integer value, when the designer can read one.
    pub fn int(&self, name: &str) -> Option<i64> {
        self.prop(name).and_then(super::value::int)
    }

    /// Its properties in order (every assignment).
    pub fn props(&self) -> impl DoubleEndedIterator<Item = &Prop> {
        self.body.iter().filter_map(|i| match i {
            Item::Prop(p) => Some(p),
            _ => None,
        })
    }

    /// Its children, in creation order.
    pub fn children(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.body.iter().filter_map(|i| match i {
            Item::Child(c) => Some(*c),
            _ => None,
        })
    }

    /// The registry's entry for its type.
    pub fn component(&self) -> Option<&'static rapidr_lang::Component> {
        rapidr_lang::component(&self.canonical)
    }

    /// Whether it shows on the form (else it goes in the component tray:
    /// timers, dialogs, databases …). Unknown types are shown.
    pub fn is_visual(&self) -> bool {
        self.component().is_none_or(|c| c.visual) && !matches!(self.canonical.as_str(), "RMAINMENU" | "RPOPUPMENU" | "RMENUITEM")
    }

    /// Whether components can be dropped into it.
    pub fn is_container(&self) -> bool {
        self.component().is_some_and(|c| c.container)
    }

    pub fn is_form(&self) -> bool {
        self.canonical == "RFORM"
    }
}

/// A component and everything CREATEd inside it, detached from a form (to
/// insert, paste, or restore on undo).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Subtree {
    /// Its id (kept when restored by undo; new ones are given on paste).
    pub id: NodeId,
    pub name: String,
    pub type_written: String,
    pub body: Vec<SubItem>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SubItem {
    Prop(Prop),
    Child(Subtree),
    Code(String),
}

impl Subtree {
    /// A new component with these properties (in order).
    pub fn new(name: &str, type_written: &str, props: &[(&str, String)]) -> Subtree {
        Subtree { id: 0, name: name.to_string(), type_written: type_written.to_string(), body: props.iter().map(|(n, v)| SubItem::Prop(Prop { name: n.to_string(), value: v.clone() })).collect() }
    }

    /// Every component in it, itself first (depth first).
    pub fn all(&self) -> Vec<&Subtree> {
        let mut out = vec![self];
        for i in &self.body {
            if let SubItem::Child(c) = i {
                out.extend(c.all());
            }
        }
        out
    }

    fn all_mut(&mut self, f: &mut dyn FnMut(&mut Subtree)) {
        f(self);
        for i in &mut self.body {
            if let SubItem::Child(c) = i {
                c.all_mut(f);
            }
        }
    }

    /// Its value of property `name` (the last assignment).
    pub fn prop(&self, name: &str) -> Option<&str> {
        let key = prop_key(name);
        self.body.iter().rev().find_map(|i| match i {
            SubItem::Prop(p) if prop_key(&p.name) == key => Some(p.value.as_str()),
            _ => None,
        })
    }

    /// Sets property `name` (its last assignment, else a new one after the
    /// last property).
    pub fn set_prop(&mut self, name: &str, value: String) {
        let key = prop_key(name);
        if let Some(SubItem::Prop(p)) = self.body.iter_mut().rev().find(|i| matches!(i, SubItem::Prop(p) if prop_key(&p.name) == key)) {
            p.value = value;
            return;
        }
        let at = self.body.iter().rposition(|i| matches!(i, SubItem::Prop(_))).map_or(0, |i| i + 1);
        self.body.insert(at, SubItem::Prop(Prop { name: name.to_string(), value }));
    }

    /// Drops every assignment of `name`.
    pub fn remove_prop(&mut self, name: &str) {
        let key = prop_key(name);
        self.body.retain(|i| !matches!(i, SubItem::Prop(p) if prop_key(&p.name) == key));
    }
}

/// A designed form: its components by id, the form itself the root. (Two
/// are equal when their components are: the next free id doesn't count.)
#[derive(Clone, Debug, Eq)]
pub struct FormDesign {
    nodes: BTreeMap<NodeId, Node>,
    root: NodeId,
    next: NodeId,
}

impl PartialEq for FormDesign {
    fn eq(&self, other: &Self) -> bool {
        self.root == other.root && self.nodes == other.nodes
    }
}

impl FormDesign {
    /// A form with nothing in it (`CREATE name AS type_written`).
    pub fn new(name: &str, type_written: &str) -> FormDesign {
        let mut nodes = BTreeMap::new();
        nodes.insert(1, Node::new(1, name, type_written));
        FormDesign { nodes, root: 1, next: 2 }
    }

    /// The form from a whole CREATE tree (as read from source).
    pub fn from_subtree(tree: Subtree) -> FormDesign {
        FormDesign::from_subtree_after(tree, 1)
    }

    /// The same, keeping the tree's ids (non-zero ones) and giving new ones
    /// from `next` on (and after every kept one): a form read again from
    /// its source keeps its components' ids.
    pub fn from_subtree_after(tree: Subtree, next: NodeId) -> FormDesign {
        let max = tree.all().iter().map(|t| t.id).max().unwrap_or(0);
        let mut d = FormDesign { nodes: BTreeMap::new(), root: 0, next: next.max(max + 1) };
        let id = d.attach(None, tree, true);
        d.root = id;
        d
    }

    /// The id the next new component gets.
    pub fn next_id(&self) -> NodeId {
        self.next
    }

    pub fn root(&self) -> NodeId {
        self.root
    }

    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(&id)
    }

    pub(crate) fn node_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(&id)
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Every component, the form first, in creation order (depth first).
    pub fn ids(&self) -> Vec<NodeId> {
        let mut out = Vec::new();
        self.walk(self.root, &mut out);
        out
    }

    fn walk(&self, id: NodeId, out: &mut Vec<NodeId>) {
        out.push(id);
        if let Some(n) = self.node(id) {
            for c in n.children() {
                self.walk(c, out);
            }
        }
    }

    /// A component by name (any case).
    pub fn find(&self, name: &str) -> Option<NodeId> {
        self.nodes.values().find(|n| n.name.eq_ignore_ascii_case(name)).map(|n| n.id)
    }

    pub fn children(&self, id: NodeId) -> Vec<NodeId> {
        self.node(id).map(|n| n.children().collect()).unwrap_or_default()
    }

    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.node(id).and_then(|n| n.parent)
    }

    /// Whether `id` is `ancestor` or inside it.
    pub fn is_within(&self, mut id: NodeId, ancestor: NodeId) -> bool {
        loop {
            if id == ancestor {
                return true;
            }
            match self.parent(id) {
                Some(p) => id = p,
                None => return false,
            }
        }
    }

    /// A name no component has: `base` followed by the smallest number from
    /// 1 (`Button1`, `Button2` …), as Delphi names new components.
    pub fn unique_name(&self, base: &str) -> String {
        let base = base.trim_end_matches(|c: char| c.is_ascii_digit());
        (1..).map(|n| format!("{base}{n}")).find(|n| self.find(n).is_none()).unwrap_or_default()
    }

    /// The name a new component of this type gets (`QBUTTON` → `Button1`).
    pub fn new_name(&self, type_written: &str) -> String {
        let t = type_written.to_ascii_uppercase();
        let short = t.strip_prefix('Q').or_else(|| t.strip_prefix('R')).unwrap_or(&t);
        let mut base: String = short.chars().take(1).collect();
        base.push_str(&short.chars().skip(1).collect::<String>().to_ascii_lowercase());
        self.unique_name(&base)
    }

    /// Puts a detached tree under `parent` at `index` of its body (None: the
    /// root). `keep_ids`: the tree's own ids are used (undo); else new ones
    /// are given. Returns the tree's root id.
    pub(crate) fn attach(&mut self, parent: Option<(NodeId, usize)>, tree: Subtree, keep_ids: bool) -> NodeId {
        let id = if keep_ids && tree.id != 0 && !self.nodes.contains_key(&tree.id) { tree.id } else { self.next };
        self.next = self.next.max(id + 1);
        let mut node = Node::new(id, &tree.name, &tree.type_written);
        node.parent = parent.map(|(p, _)| p);
        self.nodes.insert(id, node);
        let mut body = Vec::with_capacity(tree.body.len());
        for item in tree.body {
            body.push(match item {
                SubItem::Prop(p) => Item::Prop(p),
                SubItem::Code(c) => Item::Code(c),
                SubItem::Child(c) => {
                    let child = self.attach(Some((id, usize::MAX)), c, keep_ids);
                    Item::Child(child)
                }
            });
        }
        if let Some(n) = self.nodes.get_mut(&id) {
            n.body = body;
        }
        if let Some((p, at)) = parent.filter(|(_, at)| *at != usize::MAX) {
            if let Some(pn) = self.nodes.get_mut(&p) {
                let at = at.min(pn.body.len());
                pn.body.insert(at, Item::Child(id));
            }
        }
        id
    }

    /// Takes a component (and what's inside it) out of the form: the tree,
    /// and where it was (parent, index in its body).
    pub(crate) fn detach(&mut self, id: NodeId) -> Option<(Subtree, NodeId, usize)> {
        let parent = self.parent(id)?;
        let at = self.node(parent)?.body.iter().position(|i| *i == Item::Child(id))?;
        let tree = self.subtree(id)?;
        self.node_mut(parent)?.body.remove(at);
        for n in tree.all() {
            self.nodes.remove(&n.id);
        }
        Some((tree, parent, at))
    }

    /// A component and its insides as a detached tree (ids kept).
    pub fn subtree(&self, id: NodeId) -> Option<Subtree> {
        let n = self.node(id)?;
        let body = n
            .body
            .iter()
            .map(|i| match i {
                Item::Prop(p) => SubItem::Prop(p.clone()),
                Item::Code(c) => SubItem::Code(c.clone()),
                Item::Child(c) => SubItem::Child(self.subtree(*c).expect("child exists")),
            })
            .collect();
        Some(Subtree { id, name: n.name.clone(), type_written: n.type_written.clone(), body })
    }

    /// A copy of `tree` with fresh unique names (`Button1` → `Button2`) for
    /// this form, ids cleared and event handlers unbound (`On… =` lines
    /// dropped): what Paste inserts.
    pub fn pasteable(&self, tree: &Subtree, taken: &mut Vec<String>) -> Subtree {
        let mut t = tree.clone();
        t.all_mut(&mut |s| {
            s.id = 0;
            let mut name = s.name.clone();
            if self.find(&name).is_some() || taken.iter().any(|n| n.eq_ignore_ascii_case(&name)) {
                let base = name.trim_end_matches(|c: char| c.is_ascii_digit()).to_string();
                name = (1..).map(|n| format!("{base}{n}")).find(|n| self.find(n).is_none() && !taken.iter().any(|t| t.eq_ignore_ascii_case(n))).unwrap_or_default();
            }
            taken.push(name.clone());
            s.name = name;
            s.body.retain(|i| !matches!(i, SubItem::Prop(p) if p.name.len() > 2 && p.name[..2].eq_ignore_ascii_case("on")));
        });
        t
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(n: &str, v: &str) -> SubItem {
        SubItem::Prop(Prop { name: n.into(), value: v.into() })
    }

    pub(crate) fn sample() -> FormDesign {
        let ok = Subtree { id: 0, name: "Ok".into(), type_written: "QBUTTON".into(), body: vec![p("Caption", "\"OK\""), p("Left", "300"), p("Top", "230")] };
        let ed = Subtree { id: 0, name: "Ed".into(), type_written: "QEDIT".into(), body: vec![p("Left", "10"), p("Width", "200")] };
        let form = Subtree { id: 0, name: "Form".into(), type_written: "QFORM".into(), body: vec![p("Width", "400"), p("Height", "300"), SubItem::Child(ok), SubItem::Code("Center".into()), SubItem::Child(ed)] };
        FormDesign::from_subtree(form)
    }

    #[test]
    fn a_tree_of_create_blocks() {
        let d = sample();
        let ids = d.ids();
        assert_eq!(ids.len(), 3);
        let ok = d.find("OK").unwrap();
        let form = d.node(d.root()).unwrap();
        assert_eq!(form.canonical, "RFORM");
        assert_eq!(d.node(ok).unwrap().canonical, "RBUTTON");
        assert_eq!(d.node(ok).unwrap().int("left"), Some(300));
        assert_eq!(d.children(d.root()).len(), 2);
        assert_eq!(d.parent(ok), Some(d.root()));
        assert!(d.is_within(ok, d.root()));
        assert_eq!(d.unique_name("Ok"), "Ok1");
        assert_eq!(d.new_name("QBUTTON"), "Button1");
        assert_eq!(d.new_name("RPLOT"), "Plot1");
        assert_eq!(prop_key("Constraints.MinWidth"), "minwidth");
    }

    #[test]
    fn detach_and_attach_give_the_same_form() {
        let mut d = sample();
        let before = d.clone();
        let ok = d.find("Ok").unwrap();
        let (tree, parent, at) = d.detach(ok).unwrap();
        assert_eq!(d.find("Ok"), None);
        d.attach(Some((parent, at)), tree, true);
        assert_eq!(d, before);
    }

    #[test]
    fn paste_names_are_unique_and_handlers_unbound() {
        let d = sample();
        let mut t = d.subtree(d.find("Ok").unwrap()).unwrap();
        t.body.push(p("OnClick", "OkClick"));
        let mut taken = Vec::new();
        let copy = d.pasteable(&t, &mut taken);
        assert_eq!(copy.name, "Ok1");
        assert!(copy.prop("OnClick").is_none());
        let again = d.pasteable(&t, &mut taken);
        assert_eq!(again.name, "Ok2");
    }
}

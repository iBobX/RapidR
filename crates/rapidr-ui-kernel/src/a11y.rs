//! A form's accessibility tree (`docs/desktop-host-plan.md` §6): the
//! window (a dialog while modal) and every shown component, nested as on
//! the form, each described by its kind (and its shared model), with
//! stable ids (`a11y::node_id`), its state (focused, disabled) and a name:
//!
//! 1. its AccessibleName (RapidR's), else
//! 2. its Caption (a button's, a label's; `&` dropped), else
//! 3. its Hint, else
//! 4. the nearest QLABEL starting to its left on the same parent, else the
//!    nearest above it that doesn't already name a control to its right.
//!
//! AccessibleDescription (RapidR's) is its description. A screen reader's
//! request comes back through [`FormUi::access_action`] as the same input
//! a user's would be, so OnClick and OnChange fire identically.

use rapidr_value::objects::a11y::{label_above, label_left_of, node_id, AccessNode, Action, Role, PART_TAB};

use crate::store::{self, Store};
use crate::text::TextSystem;
use crate::tree::FormUi;

/// A value a screen reader sets.
#[derive(Clone, Debug, PartialEq)]
pub enum AccessValue {
    Number(f64),
    Text(String),
}

impl FormUi {
    /// The tree now (bounds in the client area's logical pixels, the menu
    /// bar's included).
    pub fn access_tree(&mut self, store: &dyn Store, ts: &mut TextSystem) -> AccessNode {
        self.sync(store);
        let mut root = AccessNode::new(node_id(&self.form), if self.modal { Role::Dialog } else { Role::Window });
        root.name = store::string(store, &self.form, "caption");
        root.description = store::string(store, &self.form, "accessibledescription");
        root.bounds = (0, 0, self.client.0, self.client.1 + self.menu_offset);
        root.states.modal = self.modal;
        root.states.focused = self.focus.is_none();
        // (the in-window menu bar: components/menubar.rs)
        if let Some(bar) = self.describe_menu_bar(store) {
            root.children.push(bar);
        }
        for i in self.roots() {
            if let Some(n) = self.describe(store, ts, i) {
                root.children.push(n);
            }
        }
        root
    }

    fn describe(&mut self, store: &dyn Store, ts: &mut TextSystem, i: usize) -> Option<AccessNode> {
        if !self.nodes[i].shown {
            return None;
        }
        let mut n = self.with_cx(store, ts, i, |k, cx| k.describe(cx)).unwrap_or_else(|| {
            // (a container the kernel only places: a pane)
            let mut n = AccessNode::new(node_id(&self.nodes[i].id), Role::Pane);
            n.bounds = self.nodes[i].abs;
            n
        });
        let id = self.nodes[i].id.clone();
        n.states.focused = self.focus == Some(i);
        n.states.disabled = !self.nodes[i].enabled;
        if self.can_focus(store, i) && !n.actions.contains(&Action::Focus) {
            n.actions.push(Action::Focus);
        }
        let given = store::string(store, &id, "accessiblename");
        if !given.is_empty() {
            n.name = given;
        }
        if n.name.is_empty() {
            n.name = store::string(store, &id, "hint");
        }
        if n.name.is_empty() && !matches!(n.role, Role::Label | Role::Pane) {
            if let Some(l) = self.label_for(i) {
                n.labelled_by = Some(node_id(&self.nodes[l].id));
                let caption = store::string(store, &self.nodes[l].id, "caption");
                n.name = rapidr_value::objects::a11y::mnemonic(&caption).0;
            }
        }
        n.description = store::string(store, &id, "accessibledescription");
        for c in self.children(i) {
            if let Some(child) = self.describe(store, ts, c) {
                n.children.push(child);
            }
        }
        Some(n)
    }

    /// The shown QLABEL on the same parent nearest to node `i`'s left, else
    /// the nearest above it that doesn't already name a control to its
    /// right.
    fn label_for(&self, i: usize) -> Option<usize> {
        let parent = self.nodes[i].parent;
        let labels: Vec<usize> = (0..self.nodes.len()).filter(|&l| l != i && self.nodes[l].parent == parent && self.nodes[l].shown && self.nodes[l].type_name == "RLABEL").collect();
        let rects: Vec<_> = labels.iter().map(|&l| self.nodes[l].abs).collect();
        if let Some(k) = label_left_of(self.nodes[i].abs, &rects) {
            return Some(labels[k]);
        }
        let others: Vec<usize> = (0..self.nodes.len()).filter(|&o| o != i && self.nodes[o].parent == parent && self.nodes[o].shown && self.nodes[o].type_name != "RLABEL").collect();
        let claimed: Vec<usize> = others.iter().filter_map(|&o| label_left_of(self.nodes[o].abs, &rects)).collect();
        let free: Vec<usize> = (0..labels.len()).filter(|k| !claimed.contains(k)).collect();
        let free_rects: Vec<_> = free.iter().map(|&k| rects[k]).collect();
        label_above(self.nodes[i].abs, &free_rects).map(|k| labels[free[k]])
    }

    /// The node (component index, part) an accessibility id names.
    fn access_target(&self, store: &dyn Store, target: u64) -> Option<(usize, Option<usize>)> {
        if let Some(i) = (0..self.nodes.len()).find(|&i| node_id(&self.nodes[i].id) == target) {
            return Some((i, None));
        }
        let kind = target >> 56;
        if kind != PART_TAB {
            return None;
        }
        (0..self.nodes.len()).find_map(|i| {
            let id = &self.nodes[i].id;
            let index = ((target & 0x00FF_FFFF_FFFF_FFFF) ^ node_id(id)) as usize;
            let tabs = rapidr_value::objects::with_tabcontrol(id, |t| t.tabs.len()).unwrap_or(0);
            (index < tabs && rapidr_value::objects::a11y::part_id(id, kind, index) == target && store.type_of(id).eq_ignore_ascii_case("RTABCONTROL")).then_some((i, Some(index)))
        })
    }

    /// A screen reader's (or RAI's) request: `action` on node `target`
    /// (with `value` for SetValue). Done as the user's input would be;
    /// whether it was understood.
    pub fn access_action(&mut self, store: &dyn Store, ts: &mut TextSystem, target: u64, action: Action, value: Option<AccessValue>) -> bool {
        if let Some(done) = self.menu_access(store, target, action) {
            return done;
        }
        let Some((i, part)) = self.access_target(store, target) else { return false };
        if !self.nodes[i].shown || !self.nodes[i].enabled {
            return false;
        }
        self.dirty = true;
        if action == Action::Focus && part.is_none() {
            if self.can_focus(store, i) {
                self.set_focus(Some(i));
                return true;
            }
            return false;
        }
        self.with_cx(store, ts, i, |k, cx| k.access(cx, action, part, value.as_ref())).unwrap_or(false)
    }
}

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
//! AccessibleDescription (RapidR's) is its description. The rules are
//! `rapidr_value::objects::a11y`'s, which the web runtime applies too
//! (`apply_name_rule`, `label_for`). A screen reader's request comes back
//! through [`FormUi::access_action`] as the same input a user's would be, so
//! OnClick and OnChange fire identically.

use rapidr_value::objects::a11y::{apply_name_rule, label_for, node_id, AccessNode, Action, Role};

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
        // (a part the kind says holds the focus — a grid's in-place editor —
        // has it while the component has it)
        let focused = self.focus == Some(i);
        match n.children.iter_mut().find(|c| c.states.focused) {
            Some(part) => {
                part.states.focused = focused;
                n.states.focused = false;
            }
            None => n.states.focused = focused,
        }
        n.states.disabled = !self.nodes[i].enabled;
        if self.can_focus(store, i) && !n.actions.contains(&Action::Focus) {
            n.actions.push(Action::Focus);
        }
        apply_name_rule(&mut n, &|p| store.get(&id, p), || {
            let l = self.label_for(i)?;
            Some((node_id(&self.nodes[l].id), store::string(store, &self.nodes[l].id, "caption")))
        });
        for c in self.children(i) {
            if let Some(child) = self.describe(store, ts, c) {
                n.children.push(child);
            }
        }
        Some(n)
    }

    /// The shown QLABEL on the same parent naming node `i` (`label_for`'s
    /// rule).
    fn label_for(&self, i: usize) -> Option<usize> {
        let parent = self.nodes[i].parent;
        let siblings: Vec<usize> = (0..self.nodes.len()).filter(|&o| self.nodes[o].parent == parent && self.nodes[o].shown).collect();
        let at = siblings.iter().position(|&o| o == i)?;
        let rects: Vec<_> = siblings.iter().map(|&o| (self.nodes[o].abs, self.nodes[o].type_name == "RLABEL")).collect();
        label_for(at, &rects).map(|k| siblings[k])
    }

    /// The node (component index, part) an accessibility id names: a
    /// component, or a part of one (a tab, a list's row, a tree's item: the
    /// index its id mixes in, `a11y::part_id`).
    fn access_target(&mut self, store: &dyn Store, ts: &mut TextSystem, target: u64) -> Option<(usize, Option<usize>)> {
        if let Some(i) = (0..self.nodes.len()).find(|&i| node_id(&self.nodes[i].id) == target) {
            return Some((i, None));
        }
        let index = |id: &str| ((target & 0x00FF_FFFF_FFFF_FFFF) ^ node_id(id)) as usize;
        // (the component whose description has the part)
        let has = |n: &AccessNode| {
            let mut found = false;
            n.walk(&mut |c| found |= c.id == target);
            found
        };
        (0..self.nodes.len()).find_map(|i| {
            if !self.nodes[i].shown || index(&self.nodes[i].id) >= 100_000_000 {
                return None;
            }
            let n = self.with_cx(store, ts, i, |k, cx| k.describe(cx))?;
            has(&n).then(|| (i, Some(index(&self.nodes[i].id))))
        })
    }

    /// A screen reader's (or RAI's) request: `action` on node `target`
    /// (with `value` for SetValue). Done as the user's input would be;
    /// whether it was understood.
    pub fn access_action(&mut self, store: &dyn Store, ts: &mut TextSystem, target: u64, action: Action, value: Option<AccessValue>) -> bool {
        if let Some(done) = self.menu_access(store, target, action) {
            return done;
        }
        let Some((i, part)) = self.access_target(store, ts, target) else { return false };
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

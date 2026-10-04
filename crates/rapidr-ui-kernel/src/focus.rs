//! The keyboard focus and Tab order, as Windows' dialog navigation walks
//! it: depth first through the form's components, each parent's children
//! by TabOrder where the store has one (else in creation order), skipping
//! what's hidden, disabled, can't take the focus (a label) or has
//! TabStop = False. A click focuses a component whatever its TabStop.

use crate::store::{self, Store};
use crate::tree::FormUi;

impl FormUi {
    /// Whether node `i` can have the focus (shown, enabled, a kind that
    /// takes it).
    pub fn can_focus(&self, store: &dyn Store, i: usize) -> bool {
        let n = &self.nodes[i];
        n.shown && n.enabled && n.kind.is_some_and(|k| k.focusable(store, &n.id))
    }

    /// Whether Tab stops on node `i`.
    pub fn tab_stop(&self, store: &dyn Store, i: usize) -> bool {
        self.can_focus(store, i) && store::flag(store, &self.nodes[i].id, "tabstop", true)
    }

    /// The nodes Tab visits, in order.
    pub fn tab_order(&self, store: &dyn Store) -> Vec<usize> {
        let mut out = Vec::new();
        self.walk_tab(store, &self.roots(), &mut out);
        out
    }

    fn walk_tab(&self, store: &dyn Store, siblings: &[usize], out: &mut Vec<usize>) {
        let mut sorted: Vec<(i64, usize)> = siblings
            .iter()
            .enumerate()
            .map(|(k, &i)| {
                let order = match store.get(&self.nodes[i].id, "taborder") {
                    rapidr_value::Value::Null => k as i64,
                    v => v.to_i64(),
                };
                (order, i)
            })
            .collect();
        // (stable: equal TabOrders keep creation order)
        sorted.sort_by_key(|(o, _)| *o);
        for (_, i) in sorted {
            if !self.nodes[i].shown || !self.nodes[i].enabled {
                continue;
            }
            if self.tab_stop(store, i) {
                out.push(i);
            }
            self.walk_tab(store, &self.children(i), out);
        }
    }

    /// Tab / Shift+Tab: the next (previous) component in Tab order.
    pub fn move_focus(&mut self, store: &dyn Store, back: bool) {
        let order = self.tab_order(store);
        if order.is_empty() {
            return;
        }
        let n = order.len();
        let next = match self.focus.and_then(|f| order.iter().position(|&o| o == f)) {
            Some(k) if back => order[(k + n - 1) % n],
            Some(k) => order[(k + 1) % n],
            None if back => order[n - 1],
            None => order[0],
        };
        self.set_focus(Some(next));
    }

    pub fn set_focus(&mut self, i: Option<usize>) {
        if self.focus != i {
            self.dirty = true;
        }
        self.focus = i;
        self.reset_caret();
    }

    /// The focused component's id.
    pub fn focused(&self) -> Option<&str> {
        self.focus.map(|f| self.nodes[f].id.as_str())
    }

    /// `X.SetFocus`: whether `id` took the focus.
    pub fn focus_id(&mut self, store: &dyn Store, id: &str) -> bool {
        match self.index_of(id).filter(|&i| self.can_focus(store, i)) {
            Some(i) => {
                self.set_focus(Some(i));
                true
            }
            None => false,
        }
    }

    /// The component after `i` in Tab order (what a label's mnemonic
    /// focuses).
    pub(crate) fn next_in_order(&self, store: &dyn Store, i: usize) -> Option<usize> {
        // (creation order decides where a component that isn't a tab stop
        // itself, a label, falls)
        self.tab_order(store).into_iter().find(|&o| o > i)
    }
}

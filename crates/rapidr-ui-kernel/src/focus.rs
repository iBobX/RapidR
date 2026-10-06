//! The keyboard focus and Tab order, as Windows' dialog navigation walks
//! it: depth first through the form's components, each parent's children
//! by TabOrder where the store has one (else in creation order), skipping
//! what's hidden, disabled, can't take the focus (a label) or has
//! TabStop = False. A click focuses a component whatever its TabStop.

use crate::store::{self, Store};
use crate::tree::FormUi;

/// A single-line edit (QEDIT) the focus enters selects all its text
/// (VCL's AutoSelect); the program reads it at once (the shared model).
pub(crate) fn select_on_entry(id: &str, type_name: &str) {
    if type_name.eq_ignore_ascii_case("REDIT") {
        rapidr_value::objects::with_textedit_mut(id, |t| {
            if !t.multi {
                t.set("selstart", &rapidr_value::v_int(0));
                t.set("sellength", &rapidr_value::v_int(i64::MAX >> 1));
            }
        });
    }
}

/// A form's nodes as Tab walks them.
struct Walk<'a> {
    ui: &'a FormUi,
    store: &'a dyn Store,
}

impl rapidr_value::objects::a11y::TabTree for Walk<'_> {
    type Id = usize;

    fn children(&self, parent: Option<&usize>) -> Vec<usize> {
        parent.map_or_else(|| self.ui.roots(), |&i| self.ui.children(i))
    }

    fn tab_order_of(&self, &i: &usize) -> Option<i64> {
        match self.store.get(&self.ui.nodes[i].id, "taborder") {
            rapidr_value::Value::Null => None,
            v => Some(v.to_i64()),
        }
    }

    fn active(&self, &i: &usize) -> bool {
        self.ui.nodes[i].shown && self.ui.nodes[i].enabled
    }

    fn stops(&self, &i: &usize) -> bool {
        self.ui.tab_stop(self.store, i)
    }
}

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

    /// The nodes Tab visits, in order (the shared walk, the web's too:
    /// `rapidr_value::objects::a11y::tab_order`).
    pub fn tab_order(&self, store: &dyn Store) -> Vec<usize> {
        rapidr_value::objects::a11y::tab_order(&Walk { ui: self, store })
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

    /// The focus to `i` (Tab, a mnemonic, `SetFocus`, the form showing …): a
    /// QEDIT it enters selects its text (VCL's TEdit.AutoSelect — RC.EXE:
    /// SelStart 0, SelLength the text's length once the form shows with
    /// it focused, and each time the focus comes back, wherever the
    /// program left the selection; not a QRICHEDIT or memo).
    pub fn set_focus(&mut self, i: Option<usize>) {
        let entering = self.focus != i;
        self.focus_to(i);
        if entering {
            if let Some(n) = i {
                select_on_entry(&self.nodes[n].id, &self.nodes[n].type_name);
            }
        }
    }

    /// The focus to `i` by a mouse press: the caret stays where the click
    /// put it (VCL skips AutoSelect while the left button is down).
    pub(crate) fn set_focus_by_click(&mut self, i: Option<usize>) {
        self.focus_to(i);
    }

    fn focus_to(&mut self, i: Option<usize>) {
        if self.focus != i {
            self.dirty = true;
            // (the input lane's: an in-place edit ends, kept, when its
            // component loses the focus — components/list.rs)
            if let Some(old) = self.focus {
                let (id, t) = (self.nodes[old].id.clone(), self.nodes[old].type_name.clone());
                crate::components::list::focus_left(&id, &t, &mut self.events);
            }
            if let Some(new) = i {
                self.entered(new);
            }
        }
        self.focus = i;
        self.reset_caret();
    }

    /// Node `i` got the focus: a list's OnEnter (the VCL's CM_ENTER — RC.EXE
    /// takes `QFILELISTBOX.OnEnter`; its QLISTBOX one stops the program at
    /// the binding, RapidR fires it as the file list's).
    pub(crate) fn entered(&mut self, i: usize) {
        let n = &self.nodes[i];
        if matches!(n.type_name.as_str(), "RLISTBOX" | "RFILELISTBOX") {
            self.events.push(crate::input::KernelEvent::Fire { id: n.id.clone(), event: "onenter".into(), args: Vec::new() });
        }
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
    /// focuses: the next stop after it in Tab's walk, the web's rule too).
    pub(crate) fn next_in_order(&self, store: &dyn Store, i: usize) -> Option<usize> {
        rapidr_value::objects::a11y::next_stop_after(&Walk { ui: self, store }, &i)
    }
}

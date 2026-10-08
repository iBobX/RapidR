//! Hints (`rapidr_value::hints`): what the mouse is over tells the program
//! the application's hint — the long part of that component's Hint, or
//! its parent's, up to the form's ("" over a component without one, or
//! outside the window) — which the form OnHint was bound on last hears
//! when it changes ([`KernelEvent::Hint`]). The tooltip that shows a
//! hint's short part is tooltip.rs's, which asks [`FormUi::hint_tip`]
//! what a component shows.

use rapidr_value::hints::{long_hint, short_hint};
use rapidr_value::Value;

use crate::input::KernelEvent;
use crate::store::{self, Store};
use crate::tree::FormUi;

/// A form's hint state.
#[derive(Default)]
pub struct HintUi {
    /// What the mouse is over, as last told: a component's id, the form's
    /// own id over its open area; `None` outside the window.
    over: Option<String>,
}

impl FormUi {
    /// A component's Hint, else its parent's, … else the form's.
    fn hint_of(&self, store: &dyn Store, from: Option<usize>) -> String {
        let mut at = from;
        while let Some(i) = at {
            let h = store::string(store, &self.nodes[i].id, "hint");
            if !h.is_empty() {
                return h;
            }
            at = self.nodes[i].parent;
        }
        store::string(store, &self.form, "hint")
    }

    /// Whether node `i` (`None`: the form) shows its hint: its ShowHint,
    /// else its parent's (the VCL's ParentShowHint), the form's False when
    /// never set.
    fn shows_hint(&self, store: &dyn Store, i: Option<usize>) -> bool {
        let mut at = i;
        while let Some(n) = at {
            match store.get(&self.nodes[n].id, "showhint") {
                Value::Null => at = self.nodes[n].parent,
                _ => return store::flag(store, &self.nodes[n].id, "showhint", false),
            }
        }
        store::flag(store, &self.form, "showhint", false)
    }

    /// The tooltip over node `i` (`None`: the form's open area; tooltip.rs)
    /// when Application.ShowHint is on: the short part of the hint of the
    /// first of it and its parents that shows hints (the VCL's
    /// GetHintControl), its own Hint else its parent's …; "" when none
    /// does.
    pub(crate) fn hint_tip(&self, store: &dyn Store, i: Option<usize>) -> String {
        if !rapidr_value::globals::hint_setting("showhint").to_bool() {
            return String::new();
        }
        let mut at = i;
        while let Some(n) = at {
            if self.shows_hint(store, Some(n)) {
                break;
            }
            at = self.nodes[n].parent;
        }
        if at.is_none() && !self.shows_hint(store, None) {
            return String::new();
        }
        short_hint(&self.hint_of(store, at)).to_string()
    }

    /// The mouse moved onto `hit` (`inside`: within the window at all):
    /// the application's hint, told when it changes.
    pub(crate) fn hint_hover(&mut self, store: &dyn Store, hit: Option<usize>, inside: bool) {
        let over = inside.then(|| hit.map_or_else(|| self.form.clone(), |i| self.nodes[i].id.clone()));
        if over == self.hint.over {
            return;
        }
        self.hint.over = over;
        let long = if inside { long_hint(&self.hint_of(store, hit)).to_string() } else { String::new() };
        self.events.push(KernelEvent::Hint(long));
    }

    /// The tooltip shown now, if any: its text.
    pub fn hint_shown(&self) -> Option<&str> {
        (self.tip.shown && !self.tip.text.is_empty()).then_some(self.tip.text.as_str())
    }
}

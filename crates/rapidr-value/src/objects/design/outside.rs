//! The tray's components from outside the designed form: what the program
//! creates in top-level CREATE blocks of its own — notepad.bas's
//! `CREATE OpenDialog AS QOPENDIALOG … END CREATE` beside its form, a
//! timer, a pop-up menu. Delphi's form owns its non-visual components;
//! RapidQ's programs often make them apart, so the tray shows both: the
//! form's own first, then these (their API indexes follow the form's
//! components). Selecting one shows its properties in the inspector
//! ([`DesignSurface::with_inspected`]), and a change is written into its
//! own block as the smallest edit, one undo step, like any other.

use super::{DesignEvent, DesignSurface, Designer, Attached};

/// A top-level non-visual component of the program, outside the form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outside {
    /// Which of the source's top-level CREATE blocks it is.
    pub form: usize,
    pub name: String,
    pub type_name: String,
}

/// Whether a top-level block of this type goes in the tray: a component
/// that doesn't show on a form (a dialog, a timer, a pop-up menu, a
/// database …), not a form.
fn trayed(type_written: &str) -> bool {
    let canonical = crate::designer::model::canonical_type(type_written);
    if canonical == "RFORM" || canonical == "RMAINMENU" || canonical == "RMENUITEM" {
        return false;
    }
    match rapidr_lang::component(&canonical) {
        Some(c) => !c.visual || canonical == "RPOPUPMENU",
        None => false,
    }
}

impl DesignSurface {
    /// The tray's outside components read again from the source (the one
    /// selected stays selected while it is there).
    pub(super) fn refresh_outside(&mut self) {
        let was = self.outside_sel.and_then(|k| self.outside.get(k)).map(|o| o.name.clone());
        self.outside = match &self.source {
            Some(Attached { doc, form: Some(designed) }) => doc
                .borrow()
                .forms()
                .into_iter()
                .enumerate()
                .filter(|(k, (_, t))| k != designed && trayed(t))
                .map(|(form, (name, type_name))| Outside { form, name, type_name })
                .collect(),
            _ => Vec::new(),
        };
        self.outside_sel = was.and_then(|n| self.outside.iter().position(|o| o.name.eq_ignore_ascii_case(&n)));
    }

    /// The outside components (the tray's, after the form's own).
    pub fn outside(&self) -> &[Outside] {
        &self.outside
    }

    /// Outside component `k` selected alone (a click on it in the tray).
    pub(super) fn select_outside(&mut self, k: usize) -> bool {
        let Some(o) = self.outside.get(k) else { return false };
        let text = format!("{} ({}), created outside the form", o.name, o.type_name);
        self.designer.selection.clear();
        self.outside_sel = Some(k);
        self.say(text);
        true
    }

    /// The outside component selected, if one is.
    pub fn outside_selected(&self) -> Option<&Outside> {
        self.outside_sel.and_then(|k| self.outside.get(k))
    }

    /// The designer of what the inspector shows: the designed form's (its
    /// selection), or — an outside component selected in the tray — that
    /// component's own block, itself selected. `f` changes it; the change
    /// is written into the source at once (one undo step: OnSourceEdit,
    /// then OnChange), as [`DesignSurface::commit`] writes the form's.
    /// The inspector (S-PANELS) edits through this.
    pub fn with_inspected<R>(&mut self, f: impl FnOnce(&mut Designer) -> R) -> R {
        let outside = self.outside_selected().cloned();
        let attached = self.source.clone();
        match (outside, attached) {
            (Some(o), Some(a)) => {
                let Some(mut d) = a.doc.borrow().designer(o.form) else {
                    return f(&mut self.designer);
                };
                let root = d.design.root();
                d.selection.set(root);
                let r = f(&mut d);
                if d.has_applied() && !self.read_only() {
                    self.step_begins();
                    let (edits, _) = a.doc.borrow_mut().commit(o.form, d);
                    self.outbox.extend(edits.into_iter().map(DesignEvent::SourceEdit));
                    self.outbox.push(DesignEvent::Change);
                    self.reload(false);
                }
                r
            }
            _ => {
                let r = f(&mut self.designer);
                self.commit();
                r
            }
        }
    }

    /// What the inspector shows: (name, type as written) of each selected
    /// component, or the outside one selected.
    pub fn inspected_objects(&self) -> Vec<(String, String)> {
        if let Some(o) = self.outside_selected() {
            return vec![(o.name.clone(), o.type_name.clone())];
        }
        let d = &self.designer.design;
        self.designer.selection.ids().iter().filter_map(|&id| d.node(id)).map(|n| (n.name.clone(), n.type_written.clone())).collect()
    }
}

//! RDESIGNSURFACE as an inspector's subject (`Inspector.Designer =
//! Surface1`): the surface keeps a designer model (`objects::design` on
//! `crate::designer`), so its selection is inspected through that model
//! ([`super::designer_model`]): each change one undoable command, written
//! as the program writes it — the same model its program reads with
//! GetProp and its source comes from.

use super::designer_model::{DesignerModelSubject, Source};

/// Registers RDESIGNSURFACE's subject (done once, at the inspector's first
/// use of a designer).
pub fn register() {
    crate::panels::subject::register_designer("RDESIGNSURFACE", |d| Box::new(DesignerModelSubject { source: Source::Surface(d.to_string()) }));
}

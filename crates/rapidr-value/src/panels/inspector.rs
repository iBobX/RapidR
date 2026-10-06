//! RPROPERTYINSPECTOR's model (docs/ide-components.md §3): the data the kernel draws
//! (`rapidr-ui-kernel`'s `components/panels/inspector.rs`) and the program's
//! members and events through the runtime glue.

use super::runtime::Runtime;
use crate::Value;

/// RPROPERTYINSPECTOR's state.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Inspector {}

crate::panel_models!(Inspector);

/// What the user did to it (from the kernel).
#[derive(Clone, Debug, PartialEq)]
pub enum User {
    /// (nothing yet)
    None,
}

/// Its methods; `None`: not one of its own.
pub fn rt_method<R: Runtime>(_rt: R, _name: &str, _method: &str, _args: &[Value]) -> Option<Value> {
    None
}

/// Its properties its model answers.
pub fn rt_get<R: Runtime>(_rt: R, _name: &str, _prop: &str) -> Option<Value> {
    None
}

/// Its properties its model keeps: whether `prop` was one.
pub fn rt_set<R: Runtime>(_rt: R, _name: &str, _prop: &str, _v: &Value) -> bool {
    false
}

/// What the user did.
pub fn rt_user<R: Runtime>(_rt: R, _name: &str, _action: User) {}

/// Any component's property was set: an inspector showing it follows.
pub fn rt_after_set<R: Runtime>(_rt: R, _name: &str, _prop: &str) {}

/// An item picked from a list it dropped (an enum's values).
pub fn rt_picked<R: Runtime>(_rt: R, _name: &str, _item: String) {}

/// A designer's selection, or a selected object's property, changed
/// (super::subject: the designer calls this): every inspector following
/// designer `designer` reads it again.
pub fn designer_changed(_host: &dyn super::subject::Host, _designer: &str) {}

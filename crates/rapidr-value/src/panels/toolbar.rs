//! RTOOLBAR's model (docs/ide-components.md §3): the data the kernel draws
//! (`rapidr-ui-kernel`'s `components/panels/toolbar.rs`) and the program's
//! members and events through the runtime glue.

use super::runtime::Runtime;
use crate::Value;

/// RTOOLBAR's state.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ToolBar {}

crate::panel_models!(ToolBar);

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

/// An item picked from its overflow menu.
pub fn rt_picked<R: Runtime>(_rt: R, _name: &str, _item: String) {}

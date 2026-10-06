//! RDOCKMANAGER on the desktop: `rapidr_value::dock` does the work (the
//! same as on the web); this is the desktop runtime it works through.

use rapidr_value::dock::manager::User;
use rapidr_value::dock::runtime::{self, Runtime};

use crate::object::{get_children_of, rp_comp_call, rp_comp_get, rp_comp_set, rp_comp_type, rp_create_component, rp_fire_event_args, rp_fire_event_then};
use crate::value::Value;

#[derive(Clone, Copy)]
pub struct Desktop;

impl Runtime for Desktop {
    fn get(self, name: &str, prop: &str) -> Value {
        rp_comp_get(name, prop)
    }
    fn set(self, name: &str, prop: &str, v: Value) {
        rp_comp_set(name, prop, v)
    }
    fn create(self, name: &str, type_name: &str) {
        rp_create_component(name, type_name)
    }
    fn exists(self, name: &str) -> bool {
        !rp_comp_type(name).is_empty()
    }
    fn type_of(self, name: &str) -> String {
        rp_comp_type(name)
    }
    fn fire(self, name: &str, event: &str, args: &[Value]) {
        rp_fire_event_args(name, event, args)
    }
    fn fire_then(self, name: &str, event: &str, args: &[Value], then: Box<dyn FnOnce(&[Value])>) {
        rp_fire_event_then(name, event, args, move |a: &[Value]| then(a))
    }
    fn children(self, name: &str) -> Vec<(String, String)> {
        get_children_of(name)
    }
    fn restructure(self) {
        #[cfg(feature = "gui")]
        crate::ui::stack_widgets(&[]);
    }
    fn focus(self, name: &str) {
        rp_comp_call(name, "setfocus", &[]);
    }
    fn show(self, form: &str) {
        rp_comp_call(form, "show", &[]);
    }
}

/// An RDOCKMANAGER method (`None`: not one of its own).
pub fn method(dock: &str, method: &str, args: &[Value]) -> Option<Value> {
    runtime::rt_method(Desktop, dock, method, args)
}

/// Sets an RDOCKMANAGER property: whether it was one of its own.
pub fn set(dock: &str, prop: &str, val: &Value) -> bool {
    runtime::rt_set(Desktop, dock, prop, val)
}

/// After a property was stored (a dock manager or a floating window
/// resized, moved or closed).
pub fn after_set(name: &str, prop: &str) {
    runtime::rt_after_set(Desktop, name, prop)
}

/// What the user did to a dock manager (the kernel's `Container::Dock`).
pub fn user(dock: &str, action: User) {
    runtime::rt_user(Desktop, dock, action)
}

/// What the user did to a document's MDI window.
pub fn docs_user(docs: &str, component: &str, action: rapidr_value::mdi::Action) {
    runtime::docs_user(Desktop, docs, component, action)
}

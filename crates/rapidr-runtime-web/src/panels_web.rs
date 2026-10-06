//! RapidR Studio's panels in the browser (RPROPERTYINSPECTOR, RTOOLBOX,
//! RPROJECTTREE, ROUTPUTCONSOLE, RTOOLBAR, RCOMMANDPALETTE):
//! `rapidr_value::panels` does the work (the same as on the desktop); this
//! is the web runtime it works through. The UI kernel draws them on the
//! page's canvas as it does on the desktop.

use rapidr_value::objects::ops::Rect;
use rapidr_value::panels::runtime::{self, Runtime, Then};
use rapidr_value::panels::User;

use crate::object_web::{all_components, form_of, get_children_of, rp_comp_call, rp_comp_get, rp_comp_set, rp_comp_type, rp_fire_event_args, rp_fire_event_then};
use crate::value::Value;

#[derive(Clone, Copy)]
pub struct Web;

impl Runtime for Web {
    fn get(self, name: &str, prop: &str) -> Value {
        rp_comp_get(name, prop)
    }
    fn set(self, name: &str, prop: &str, v: Value) {
        rp_comp_set(name, prop, v)
    }
    fn call(self, name: &str, method: &str, args: &[Value]) -> Value {
        rp_comp_call(name, method, args)
    }
    fn exists(self, name: &str) -> bool {
        !rp_comp_type(name).is_empty()
    }
    fn type_of(self, name: &str) -> String {
        rp_comp_type(name)
    }
    fn components(self) -> Vec<(String, String)> {
        all_components()
    }
    fn children(self, name: &str) -> Vec<(String, String)> {
        get_children_of(name)
    }
    fn form_of(self, name: &str) -> Option<String> {
        form_of(name)
    }
    fn fire(self, name: &str, event: &str, args: &[Value]) {
        rp_fire_event_args(name, event, args)
    }
    fn fire_then(self, name: &str, event: &str, args: &[Value], then: Then) {
        rp_fire_event_then(name, event, args, move |a: &[Value]| then(a))
    }
    fn focus(self, name: &str) {
        rp_comp_call(name, "setfocus", &[]);
    }
    fn invalidate(self) {
        crate::kernel_web::redraw();
    }
    fn restructure(self) {
        crate::kernel_web::rebuild();
    }
    fn drop_list(self, form: &str, name: &str, items: Vec<String>, anchor: Rect) {
        rapidr_ui_kernel::components::combo::open_list(form, name, items, anchor);
        crate::kernel_web::redraw();
    }
}

/// A panel's method (`None`: not one of its own).
pub fn method(name: &str, method: &str, args: &[Value]) -> Option<Value> {
    runtime::rt_method(Web, name, method, args)
}

/// A panel's property its model answers.
pub fn get(name: &str, prop: &str) -> Option<Value> {
    runtime::rt_get(Web, name, prop)
}

/// Sets a panel's property in its model: whether it was the model's.
pub fn set(name: &str, prop: &str, val: &Value) -> bool {
    runtime::rt_set(Web, name, prop, val)
}

/// After any component's property was set.
pub fn after_set(name: &str, prop: &str) {
    runtime::rt_after_set(Web, name, prop)
}

/// What the user did to a panel (the kernel's `Container::Panel`).
pub fn user(name: &str, action: User) {
    runtime::rt_user(Web, name, action)
}

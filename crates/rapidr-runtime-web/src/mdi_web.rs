//! QFORMMDI in the browser: `rapidr_value::mdi` does the work (the same as
//! on the desktop); this is the web runtime it works through. Each child's
//! frame is an `RMDICHILD` element (`gui_web::create_mdi_frame`).

use rapidr_value::mdi::{self, Action, Runtime};

use crate::object_web::{get_children_of, rp_comp_get, rp_comp_set, rp_comp_type, rp_create_component, rp_fire_event_args, rp_fire_event_then};
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
    fn create(self, name: &str, type_name: &str) {
        rp_create_component(name, type_name)
    }
    fn exists(self, name: &str) -> bool {
        !rp_comp_type(name).is_empty()
    }
    fn fire(self, name: &str, event: &str, args: &[Value]) {
        rp_fire_event_args(name, event, args)
    }
    fn fire_then(self, name: &str, event: &str, args: &[Value], then: Box<dyn FnOnce()>) {
        rp_fire_event_then(name, event, args, then)
    }
    fn children(self, name: &str) -> Vec<(String, String)> {
        get_children_of(name)
    }
    fn stack(self, names: &[String]) {
        crate::gui_web::stack_elements(names);
    }
    fn client(self, form: &str) -> (i64, i64) {
        crate::layout_web::form_client(&form.to_uppercase())
    }
}

pub fn method(form: &str, method: &str, args: &[Value]) -> Option<Value> {
    mdi::rt_method(Web, form, method, args)
}

pub fn set(form: &str, prop: &str, val: &Value) -> bool {
    mdi::rt_set(Web, form, prop, val)
}

pub fn user(form: &str, component: &str, action: Action) {
    mdi::rt_user(Web, form, component, action)
}

pub fn resized(form: &str) {
    mdi::rt_resized(Web, form)
}

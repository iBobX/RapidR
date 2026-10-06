//! QFORMMDI on the desktop: `rapidr_value::mdi` does the work (the same as
//! on the web); this is the desktop runtime it works through.

use rapidr_value::mdi::{self, Action, Runtime};

use crate::object::{form_client, get_children_of, rp_comp_get, rp_comp_set, rp_comp_type, rp_create_component, rp_fire_event_args, rp_fire_event_then};
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
    fn fire(self, name: &str, event: &str, args: &[Value]) {
        rp_fire_event_args(name, event, args)
    }
    fn fire_then(self, name: &str, event: &str, args: &[Value], then: Box<dyn FnOnce()>) {
        rp_fire_event_then(name, event, args, move |_| then())
    }
    fn children(self, name: &str) -> Vec<(String, String)> {
        get_children_of(name)
    }
    fn stack(self, names: &[String]) {
        #[cfg(feature = "gui")]
        crate::ui::stack_widgets(names);
        #[cfg(not(feature = "gui"))]
        let _ = names;
    }
    fn client(self, form: &str) -> (i64, i64) {
        form_client(form)
    }
}

pub fn method(form: &str, method: &str, args: &[Value]) -> Option<Value> {
    mdi::rt_method(Desktop, form, method, args)
}

pub fn set(form: &str, prop: &str, val: &Value) -> bool {
    mdi::rt_set(Desktop, form, prop, val)
}

pub fn user(form: &str, component: &str, action: Action) {
    // (I1: a dock manager's document area — its documents' events)
    if rapidr_value::dock::is_docs_area(form).is_some() {
        return crate::dock::docs_user(form, component, action);
    }
    mdi::rt_user(Desktop, form, component, action)
}

pub fn resized(form: &str) {
    mdi::rt_resized(Desktop, form)
}

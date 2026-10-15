//! RAI on the web: `rapidr_ai::rai` is the component — the same on the
//! desktop (rapidr-runtime-core's ai.rs); this is its host. A page has no
//! loop that polls: the provider's answer, as it streams in through
//! `fetch`, is delivered at once (the transport's notify), its events
//! fired then. Keys: the tab's session, or the page's storage when the
//! user asked to remember one — never in the built page.

use crate::object_web::rp_fire_event_args;
use crate::value::Value;

#[derive(Clone, Copy)]
struct Web;

impl rapidr_ai::rai::Fire for Web {
    fn fire(self, name: &str, event: &str, args: &[Value]) {
        rp_fire_event_args(name, event, args);
    }
}

/// The answers delivered as they come.
pub fn install() {
    rapidr_ai::http::set_notify(Some(|| {
        rapidr_ai::rai::poll(Web);
    }));
}

pub fn get(name: &str, prop: &str) -> Option<Value> {
    rapidr_ai::rai::get(name, prop)
}

pub fn set(name: &str, prop: &str, v: &Value) -> bool {
    rapidr_ai::rai::set(name, prop, v)
}

pub fn call(name: &str, method: &str, args: &[Value]) -> Option<Value> {
    // (RapidR Studio's project tools, to attach)
    rapidr_studio::ai::install();
    rapidr_ai::rai::call(name, method, args, rapidr_value::panels::runtime::component_arg)
}

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

thread_local! {
    static INSTALLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// The answers delivered as they come. (A test page's environment may name
/// a mock provider on this computer, RAPIDR_AI_TEST_SERVER: a loopback
/// address only — rapidr_ai::provider::set_test_server refuses others.)
pub fn install() {
    if INSTALLED.with(|i| i.replace(true)) {
        return;
    }
    rapidr_ai::http::set_notify(Some(|| {
        rapidr_ai::rai::poll(Web);
    }));
    if let Some(server) = rapidr_ui_app::testhooks::var("RAPIDR_AI_TEST_SERVER") {
        if let Err(e) = rapidr_ai::provider::set_test_server(Some(&server)) {
            web_sys::console::warn_1(&format!("RAPIDR_AI_TEST_SERVER ignored: {e}").into());
        }
    }
}

pub fn get(name: &str, prop: &str) -> Option<Value> {
    rapidr_ai::rai::get(name, prop)
}

pub fn set(name: &str, prop: &str, v: &Value) -> bool {
    // (also in a program that has no other component installing it: a
    // console one)
    install();
    rapidr_ai::rai::set(name, prop, v)
}

pub fn call(name: &str, method: &str, args: &[Value]) -> Option<Value> {
    install();
    // (RapidR Studio's project tools, to attach)
    rapidr_studio::ai::install();
    rapidr_ai::rai::call(name, method, args, rapidr_value::panels::runtime::component_arg)
}

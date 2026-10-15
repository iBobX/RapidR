//! RAI on the desktop (interpreted and native programs, and RapidR Studio):
//! `rapidr_ai::rai` is the component — the same on the web
//! (rapidr-runtime-web's ai_web.rs); this is its host: its events through
//! the runtime's queue, its answers taken between the window loop's turns
//! (every 20 ms while one is coming).

use crate::object::rp_fire_event_args;
use crate::value::Value;

#[derive(Clone, Copy)]
struct Desktop;

impl rapidr_ai::rai::Fire for Desktop {
    fn fire(self, name: &str, event: &str, args: &[Value]) {
        rp_fire_event_args(name, event, args);
    }
}

/// A property of an RAI (`None`: not its own).
pub fn get(name: &str, prop: &str) -> Option<Value> {
    rapidr_ai::rai::get(name, prop)
}

/// Sets a property; whether it was the RAI's own.
pub fn set(name: &str, prop: &str, v: &Value) -> bool {
    rapidr_ai::rai::set(name, prop, v)
}

/// A method (`None`: not the RAI's own).
pub fn call(name: &str, method: &str, args: &[Value]) -> Option<Value> {
    // (RapidR Studio's runtime has the project's tools to attach)
    #[cfg(feature = "studio")]
    rapidr_studio::ai::install();
    rapidr_ai::rai::call(name, method, args, rapidr_value::panels::runtime::component_arg)
}

/// The answers that came, as their events; whether one is still coming.
pub fn poll() -> bool {
    rapidr_ai::rai::poll(Desktop)
}

/// Whether an answer is coming (the window loop then polls often).
pub fn busy() -> bool {
    rapidr_ai::rai::busy()
}

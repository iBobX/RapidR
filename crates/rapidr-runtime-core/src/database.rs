//! RSQLITE and RMYSQL for native builds and the interpreter: the
//! components every runtime shares (rapidr-db), with this runtime's
//! properties, events and messages. Background queries (QueryAsync) run
//! on each connection's worker thread; their rows and events are taken
//! between the window loop's turns ([`poll`], every 16 ms while one runs)
//! and by DOEVENTS.

use crate::object::{rp_comp_get, rp_comp_set, rp_fire_event_args};
use crate::value::Value;

struct Desktop;

impl rapidr_db::Host for Desktop {
    fn set(&self, name: &str, prop: &str, value: Value) {
        rp_comp_set(name, prop, value);
    }

    fn get(&self, name: &str, prop: &str) -> Value {
        rp_comp_get(name, prop)
    }

    fn fire(&self, name: &str, event: &str, args: &[Value]) {
        rp_fire_event_args(name, event, args);
    }

    fn report(&self, message: &str) {
        eprintln!("{message}");
    }
}

pub fn sqlite_method(name: &str, method: &str, args: &[Value]) -> Value {
    rapidr_db::sqlite::method(&Desktop, name, method, args)
}

pub fn mysql_method(name: &str, method: &str, args: &[Value]) -> Value {
    rapidr_db::mysql::method(&Desktop, name, method, args)
}

/// The background queries' rows and events (OnRows, OnDone, OnError), the
/// grids bound to them drawn again. Whether one still runs.
pub fn poll() -> bool {
    if !rapidr_db::background::running() {
        return false;
    }
    let running = rapidr_db::background::poll(&Desktop);
    crate::links::apply();
    running
}

/// Whether a background query runs (the window loop then polls often).
pub fn busy() -> bool {
    rapidr_db::background::running()
}

//! RSQLITE and RMYSQL for native builds and the interpreter: the
//! components every runtime shares (rapidr-db), with this runtime's
//! properties, events and messages.

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

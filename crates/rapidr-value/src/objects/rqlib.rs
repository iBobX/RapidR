//! RapidQ's input / output and media objects, for every runtime: the
//! objects of RapidQ's own libraries and of its manual's Appendix A that
//! talk to the world outside the program — QCGI (cgi.rs) so far. Kept in
//! a store of their own (as the Direct3D scene keeps its own), reached
//! through `objects::{create, get, set, call}`; what they need of a device
//! (a serial port, a sound card, the network) the runtime installs.
//! docs/io-media-plan.md.

use std::cell::RefCell;
use std::collections::HashMap;

use crate::Value;

use super::cgi::Cgi;

/// Their RapidR component names (QCGI is RCGI, …): no window of their own
/// on any runtime.
pub const TYPES: &[&str] = &["RCGI"];

/// Whether `type_name` is one of [`TYPES`].
pub fn is_type(type_name: &str) -> bool {
    TYPES.iter().any(|t| t.eq_ignore_ascii_case(type_name))
}

enum Lib {
    Cgi(Cgi),
}

thread_local! {
    static STORE: RefCell<HashMap<String, Lib>> = RefCell::new(HashMap::new());
}

fn with<R>(id: &str, f: impl FnOnce(&mut Lib) -> R) -> Option<R> {
    STORE.with(|s| s.borrow_mut().get_mut(&id.to_lowercase()).map(f))
}

/// Makes object `id` if `type_name` is one of these objects.
pub fn create(id: &str, type_name: &str) -> bool {
    let lib = match type_name.to_ascii_uppercase().as_str() {
        "RCGI" => Lib::Cgi(Cgi::new()),
        _ => return false,
    };
    STORE.with(|s| s.borrow_mut().insert(id.to_lowercase(), lib));
    true
}

/// Whether `id` is one of these objects.
pub fn exists(id: &str) -> bool {
    STORE.with(|s| s.borrow().contains_key(&id.to_lowercase()))
}

/// Whether `id` is a QCGI.
pub fn is_cgi(id: &str) -> bool {
    with(id, |l| matches!(l, Lib::Cgi(_))) == Some(true)
}

pub fn get(id: &str, prop: &str) -> Option<Value> {
    with(id, |l| match l {
        Lib::Cgi(c) => c.get(prop),
    })?
}

/// `Some(Ok)`: set (or read-only, left as it is); `None`: not one of the
/// object's properties (the runtime keeps it).
pub fn set(id: &str, prop: &str, val: &Value) -> Option<Result<(), String>> {
    with(id, |l| match l {
        Lib::Cgi(c) => c.set(prop, val).map(|_| Ok(())),
    })?
}

pub fn call(id: &str, method: &str, args: &[Value]) -> Option<Result<Value, String>> {
    with(id, |l| match l {
        Lib::Cgi(c) => c.call(method, args).map(Ok),
    })?
}

//! RapidQ's input / output and media objects, for every runtime: the
//! objects of RapidQ's own libraries and of its manual's Appendix A that
//! talk to the world outside the program — QCGI (cgi.rs), QCOMPORT
//! (comport.rs), QDOWNLOAD (download.rs). Kept in a store of their own (as
//! the Direct3D scene keeps its own), reached through
//! `objects::{create, get, set, call}`; what they need of a device (a
//! serial port, a sound card, the network) the runtime installs, and the
//! events a call leaves the runtime fires ([`take_events`]).
//! docs/io-media-plan.md.

use std::cell::RefCell;
use std::collections::HashMap;

use crate::Value;

use super::cgi::Cgi;
use super::comport::ComPort;
use super::download::{Download, Outcome, Request, Shown};

/// Their RapidR component names (QCGI is RCGI, …): no window of their own
/// on any runtime.
pub const TYPES: &[&str] = &["RCGI", "RCOMPORT", "RDOWNLOAD"];

/// Whether `type_name` is one of [`TYPES`].
pub fn is_type(type_name: &str) -> bool {
    TYPES.iter().any(|t| t.eq_ignore_ascii_case(type_name))
}

/// How often the runtime looks for the events nothing in the program
/// causes (QCOMPORT's OnRxChar), while the program handles one.
pub const LOOK_MS: u64 = 50;

enum Lib {
    Cgi(Cgi),
    ComPort(ComPort),
    Download(Download),
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
        "RCOMPORT" => Lib::ComPort(ComPort::default()),
        "RDOWNLOAD" => Lib::Download(Download::default()),
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

/// Whether `id` is a QCOMPORT.
pub fn is_comport(id: &str) -> bool {
    with(id, |l| matches!(l, Lib::ComPort(_))) == Some(true)
}

/// Whether `id` is a QDOWNLOAD.
pub fn is_download(id: &str) -> bool {
    with(id, |l| matches!(l, Lib::Download(_))) == Some(true)
}

pub fn get(id: &str, prop: &str) -> Option<Value> {
    with(id, |l| match l {
        Lib::Cgi(c) => c.get(prop),
        Lib::ComPort(c) => c.get(prop),
        Lib::Download(d) => d.get(prop),
    })?
}

/// `Some(Ok)`: set (or read-only, left as it is); `None`: not one of the
/// object's properties (the runtime keeps it).
pub fn set(id: &str, prop: &str, val: &Value) -> Option<Result<(), String>> {
    with(id, |l| match l {
        Lib::Cgi(c) => c.set(prop, val).map(|_| Ok(())),
        Lib::ComPort(c) => c.set(prop, val).map(Ok),
        Lib::Download(d) => d.set(prop, val).map(Ok),
    })?
}

pub fn call(id: &str, method: &str, args: &[Value]) -> Option<Result<Value, String>> {
    // (QCOMPORT's Write / Read: a stream's bytes, read or written outside
    // the store's borrow)
    let mut pending_write: Option<(String, Vec<u8>)> = None;
    let r = with(id, |l| match l {
        Lib::Cgi(c) => c.call(method, args).map(Ok),
        Lib::ComPort(c) => c
            .call(method, args, &mut |stream, bytes| match bytes {
                None => super::stream_bytes(stream).unwrap_or_default(),
                Some(b) => {
                    pending_write = Some((stream.to_string(), b.to_vec()));
                    Vec::new()
                }
            })
            .map(Ok),
        Lib::Download(d) => d.call(method, args).map(Ok),
    })?;
    if let Some((stream, bytes)) = pending_write {
        super::stream_append(&stream, &bytes);
    }
    r
}

/// The events object `id`'s last calls left, oldest first.
pub fn take_events(id: &str) -> Vec<(&'static str, Vec<Value>)> {
    with(id, |l| match l {
        Lib::ComPort(c) => c.events.drain(..).collect(),
        _ => Vec::new(),
    })
    .unwrap_or_default()
}

/// The events the runtime looks for ([`look`]).
pub fn look_events() -> &'static [&'static str] {
    &super::comport::EVENTS
}

/// The runtime's look at `id`: the events to fire.
pub fn look(id: &str) -> Vec<(&'static str, Vec<Value>)> {
    with(id, |l| match l {
        Lib::ComPort(c) => c.look(),
        _ => Vec::new(),
    })
    .unwrap_or_default()
}

/// QCOMPORT `id`'s Open where ports open later (the browser's Web
/// Serial): its port's name and settings, or `None` (failed already).
pub fn comport_open_begin(id: &str) -> Option<(String, super::comport::Settings)> {
    with(id, |l| match l {
        Lib::ComPort(c) => c.open_begin().map(|s| (c.port.clone(), s)),
        _ => None,
    })?
}

/// QCOMPORT `id`'s Open ended (`comport_open_begin`).
pub fn comport_open_end(id: &str, opened: Result<Box<dyn super::comport::Link>, super::comport::PortError>) {
    with(id, |l| {
        if let Lib::ComPort(c) = l {
            c.open_end(opened);
        }
    });
}

/// QDOWNLOAD `id`'s LeechFile begins (`now`: TIME$): what to fetch, or
/// LeechFile's result already (the library's checks failed).
pub fn download_begin(id: &str, now: &str) -> Result<Request, i64> {
    with(id, |l| match l {
        Lib::Download(d) => d.begin(now),
        _ => Err(0),
    })
    .unwrap_or(Err(0))
}

/// QDOWNLOAD `id`'s transfer so far: what its gauge and label show.
pub fn download_progress(id: &str, received: u64, length: Option<u64>, now: &str) -> Option<Shown> {
    with(id, |l| match l {
        Lib::Download(d) => Some(d.progress(received, length, now)),
        _ => None,
    })?
}

/// QDOWNLOAD `id`'s transfer ended: LeechFile's result, and the file to
/// write (OutDevice 2).
pub fn download_finish(id: &str, outcome: Outcome) -> (i64, Option<(String, Vec<u8>)>) {
    with(id, |l| match l {
        Lib::Download(d) => d.finish(outcome),
        _ => (0, None),
    })
    .unwrap_or((0, None))
}

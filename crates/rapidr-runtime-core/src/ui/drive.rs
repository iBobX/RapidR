//! The program's eyes and hands on the desktop (`rapidr_ui_app::{eyes,
//! hands, link, app_components}`, shared with the web runtime): RapidR
//! Studio's link — the session's `app` requests, which its reader thread
//! hands over ([`push`]) — and the program's own RWindowCapture and
//! RAppHands.
//!
//! Everything runs on the program's thread, in the window loop's turn
//! ([`turn`], from `kernel::step`): requests looked at, the hands' jobs
//! played, what ended answered (the link's reply, a RAppHands' OnDone).
//! While the debugger holds the program ([`turn_paused`], from its paused
//! wait) only looking is served: input is refused with where it is
//! paused. A program busy in its own code (a long loop with no DOEVENTS)
//! answers when it next waits for its events.

use std::collections::VecDeque;
use std::sync::Mutex;

use rapidr_ui_app::{app_components, hands, link};
use serde_json::{json, Value as Json};

use super::program::Rt;
use crate::value::Value;

/// How RapidR Studio is answered: the request's number and the reply
/// (JSON), from the program's thread.
pub type Reply = Box<dyn Fn(u64, String) + Send>;

static INBOX: Mutex<VecDeque<(u64, String)>> = Mutex::new(VecDeque::new());
static REPLY: Mutex<Option<Reply>> = Mutex::new(None);
/// Where the debugger holds the program, in words ("at line 12 of
/// calc.rr"), while it does.
static PAUSED_AT: Mutex<Option<String>> = Mutex::new(None);

/// RapidR Studio's link is up (a session): its replies go to `reply`.
pub fn set_link(reply: Reply) {
    if let Ok(mut r) = REPLY.lock() {
        *r = Some(reply);
    }
}

/// Whether a session's link is up (a headless program under one keeps
/// waiting for its requests rather than ending as nothing is scheduled).
pub fn linked() -> bool {
    REPLY.lock().map(|r| r.is_some()).unwrap_or(false)
}

/// A request from RapidR Studio (any thread: the session's reader); the
/// window loop is woken to serve it.
pub fn push(seq: u64, request: String) {
    if let Ok(mut q) = INBOX.lock() {
        q.push_back((seq, request));
    }
    crate::object::rp_wake();
}

/// The debugger holds the program `at` ("at line 12 of calc.rr"), or let
/// it go on (`None`).
pub fn set_paused(at: Option<String>) {
    if let Ok(mut p) = PAUSED_AT.lock() {
        *p = at;
    }
}

fn paused_at() -> Option<String> {
    PAUSED_AT.lock().ok().and_then(|p| p.clone())
}

fn answer(seq: u64, reply: &Json) {
    if let Ok(r) = REPLY.lock() {
        if let Some(r) = r.as_ref() {
            r(seq, reply.to_string());
        }
    }
}

fn serve(paused: Option<&str>) {
    let inbox: Vec<(u64, String)> = INBOX.lock().map(|mut q| q.drain(..).collect()).unwrap_or_default();
    for (seq, text) in inbox {
        let reply = match serde_json::from_str::<Json>(&text) {
            Ok(j) => link::request(Rt, seq, &j, paused),
            Err(e) => Some(json!({"ok": false, "text": format!("The request isn't JSON: {e}")})),
        };
        if let Some(r) = reply {
            answer(seq, &r);
        }
    }
}

/// The window loop's turn: requests served, the hands' jobs played, what
/// ended delivered.
pub fn turn() {
    let paused = paused_at();
    serve(paused.as_deref());
    if paused.is_none() {
        hands::step(Rt);
    }
    app_components::deliver(Rt, &answer);
    // (demo mode: a scripted tutorial played and recorded — test hooks
    // only, docs/video-pipeline.md)
    rapidr_ui_app::demo::turn(Rt);
}

/// A turn while the debugger holds the program: looking is served (its
/// windows as they are), input refused; nothing of it runs.
pub fn turn_paused() {
    let at = paused_at().unwrap_or_else(|| "where it stopped".into());
    serve(Some(&at));
    let (links, _) = link::ends();
    for (seq, r) in links {
        answer(seq, &r);
    }
}

// ------------------------------------------------- RWindowCapture, RAppHands --

/// RWindowCapture's properties the runtime answers.
pub fn capture_get(name: &str, prop: &str) -> Option<Value> {
    app_components::capture_get(Rt, name, prop)
}

/// RWindowCapture's methods (SaveToFile writes the file system).
pub fn capture_method(name: &str, method: &str, args: &[Value]) -> Option<Value> {
    app_components::capture_method(Rt, name, method, args, &|file, bytes| std::fs::write(file, bytes).map_err(|e| format!("can't write {file}: {e}")))
}

/// RAppHands's properties the runtime answers.
pub fn hands_get(name: &str, prop: &str) -> Option<Value> {
    app_components::hands_get(name, prop)
}

/// RAppHands's methods: the window loop is woken for the job.
pub fn hands_method(name: &str, method: &str, args: &[Value]) -> Option<Value> {
    let v = app_components::hands_method(Rt, name, method, args);
    crate::object::rp_wake();
    v
}

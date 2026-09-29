//! Events whose handler's work the runtime needs afterwards (a QFORMMDI's
//! OnChildClose sets `ChildResult`; OnClose's Action, …): the runtime fires
//! the event, then goes on in a *continuation* once the handler has run.
//!
//! Native builds run a handler at once, so the continuation runs at once.
//! The interpreter's hosts queue a handler (they never call into the VM from
//! the runtime): the continuation travels with it ([`QueuedEvent::then`]),
//! and the VM hands it back ([`run_all`]) when the handler returns — even
//! one that waited for a dialog in between.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use crate::Value;

/// An event for a bytecode handler: the handler (a function index), its
/// arguments, and the continuations to run once it has run.
#[derive(Clone, Debug, PartialEq)]
pub struct QueuedEvent {
    pub handler: u32,
    pub args: Vec<Value>,
    pub then: Vec<u32>,
}

impl QueuedEvent {
    pub fn new(handler: u32, args: Vec<Value>) -> Self {
        QueuedEvent { handler, args, then: Vec::new() }
    }
}

thread_local! {
    static THEN: RefCell<HashMap<u32, Box<dyn FnOnce()>>> = RefCell::new(HashMap::new());
    static NEXT: Cell<u32> = const { Cell::new(1) };
    /// The continuation of the event being fired (`arm`), until a host
    /// queues its handler (`take_armed`).
    static ARMED: Cell<Option<u32>> = const { Cell::new(None) };
}

/// Keeps `f` to run after the handler of the event about to be fired.
pub fn arm(f: Box<dyn FnOnce()>) {
    let id = NEXT.with(|n| {
        let id = n.get();
        n.set(id.wrapping_add(1).max(1));
        id
    });
    THEN.with(|t| t.borrow_mut().insert(id, f));
    ARMED.with(|a| a.set(Some(id)));
}

/// For a host queuing a handler: the continuation to go with it, if any.
pub fn take_armed() -> Vec<u32> {
    ARMED.with(|a| a.take()).into_iter().collect()
}

/// After firing: the continuation, if no host queued it (a native build ran
/// the handler already, or there was none) — the caller runs it now.
pub fn disarm() -> Option<Box<dyn FnOnce()>> {
    let id = ARMED.with(|a| a.take())?;
    THEN.with(|t| t.borrow_mut().remove(&id))
}

/// Runs the continuations of a handler that has returned.
pub fn run_all(then: Vec<u32>) {
    for id in then {
        if let Some(f) = THEN.with(|t| t.borrow_mut().remove(&id)) {
            f();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    #[test]
    fn a_continuation_goes_with_the_queued_event_or_runs_at_once() {
        let log = Rc::new(RefCell::new(Vec::new()));
        // Queued (the interpreter): it goes with the event.
        let l = log.clone();
        arm(Box::new(move || l.borrow_mut().push("queued")));
        let ev = QueuedEvent { then: take_armed(), ..QueuedEvent::new(3, vec![]) };
        assert!(disarm().is_none() && log.borrow().is_empty());
        run_all(ev.then);
        // Not queued (a native build): the caller runs it.
        let l = log.clone();
        arm(Box::new(move || l.borrow_mut().push("now")));
        disarm().unwrap()();
        assert_eq!(*log.borrow(), vec!["queued", "now"]);
    }
}

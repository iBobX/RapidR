//! Event handlers, as both runtimes (desktop and web) keep and call them,
//! and events whose handler's answers the runtime needs afterwards.
//!
//! In RapidQ a handler's parameters come back to the runtime: OnClose's
//! `Action = caNone` keeps the form open, OnSelectCell's `CanSelect = 0`
//! refuses the cell, OnMeasureItem's `Height` sizes an item. The runtime
//! fires such an event with [`arm`]ed *continuation* that gets the
//! arguments as the handler left them.
//!
//! Native builds run a handler at once, so the continuation runs at once,
//! with what the compiled handler wrote back ([`Handler::Out`]). The
//! interpreter's hosts queue a handler (they never call into the VM from
//! the runtime): the continuation travels with it ([`QueuedEvent::then`]),
//! and the VM hands it back ([`run_all`]) with the handler's parameters
//! when it returns — even one that waited for a dialog in between.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use crate::{v_null, v_str, Value};

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

/// A handler bound to a component's event.
#[derive(Clone)]
pub enum Handler {
    Arity0(fn()),
    Arity1(fn(Value)),
    Arity2(fn(Value, Value)),
    Arity3(fn(Value, Value, Value)),
    Arity4(fn(Value, Value, Value, Value)),
    Arity5(fn(Value, Value, Value, Value, Value)),
    /// A compiled handler of `n` parameters that writes them back (native
    /// builds: RapidQ's event parameters are by reference).
    Out(usize, fn(&mut [Value])),
    /// Opaque handler id (a bytecode function index) — run through the
    /// runtime's indirect dispatcher (the interpreter's host).
    Indirect(u32),
    /// A bytecode EVENT handler of a TYPE bound to one instance: invoked
    /// with that instance (`This`) before the event's arguments.
    IndirectThis(u32, Value),
    /// A compiled handler bound to one instance (native EVENT blocks and
    /// `obj(i).OnClick = Handler`); it writes the arguments back.
    Closure(Rc<dyn Fn(&mut Vec<Value>)>),
}

/// What binding a handler changes besides the binding (both runtimes call
/// it when they bind one): an OnStartDrag handler makes its button a drag
/// source (`crate::drag`), a form's OnHint handler hears the program's
/// hints (`crate::hints`).
pub fn bound(name: &str, event: &str) {
    if event.eq_ignore_ascii_case("onstartdrag") {
        crate::drag::set_source(name);
    } else if event.eq_ignore_ascii_case("onhint") {
        crate::hints::set_receiver(name);
    }
}

/// Runs `handler` for `name`'s event with `args`, the firing component
/// last (`Sender`, as in RapidQ's `SUB Button1Click (Sender AS QBUTTON)`;
/// handlers declaring fewer parameters don't get it). Returns the arguments
/// as a compiled handler left them (a queued bytecode handler hands them to
/// its continuation instead).
pub fn call(handler: Handler, name: &str, args: &[Value], indirect: impl Fn(u32, &[Value])) -> Vec<Value> {
    let mut all: Vec<Value> = args.iter().cloned().chain(std::iter::once(v_str(name))).collect();
    let a = |all: &[Value], i: usize| all.get(i).cloned().unwrap_or_else(v_null);
    // A compiled handler may fire events of its own: this one's
    // continuation waits aside until it returns. A host queuing a bytecode
    // handler takes it with the handler (`take_armed`).
    let armed = ARMED.with(|a| a.take());
    let queued = matches!(handler, Handler::Indirect(_) | Handler::IndirectThis(..));
    if queued {
        ARMED.with(|a| a.set(armed));
    }
    match handler {
        Handler::Arity0(f) => f(),
        Handler::Arity1(f) => f(a(&all, 0)),
        Handler::Arity2(f) => f(a(&all, 0), a(&all, 1)),
        Handler::Arity3(f) => f(a(&all, 0), a(&all, 1), a(&all, 2)),
        Handler::Arity4(f) => f(a(&all, 0), a(&all, 1), a(&all, 2), a(&all, 3)),
        Handler::Arity5(f) => f(a(&all, 0), a(&all, 1), a(&all, 2), a(&all, 3), a(&all, 4)),
        Handler::Out(n, f) => {
            if all.len() < n {
                all.resize(n, v_null());
            }
            f(&mut all);
        }
        Handler::Indirect(id) => indirect(id, &all),
        Handler::IndirectThis(id, this) => {
            // The handler's first parameter is `This`, not the event's.
            shift_armed(1);
            let with_this: Vec<Value> = std::iter::once(this).chain(all.iter().cloned()).collect();
            indirect(id, &with_this);
        }
        Handler::Closure(f) => f(&mut all),
    }
    if !queued {
        ARMED.with(|a| a.set(armed));
    }
    all
}

/// A continuation, given the event's arguments as the handler left them.
pub type Then = Box<dyn FnOnce(&[Value])>;

thread_local! {
    static THEN: RefCell<HashMap<u32, Then>> = RefCell::new(HashMap::new());
    static NEXT: Cell<u32> = const { Cell::new(1) };
    /// The continuation of the event being fired (`arm`), until a host
    /// queues its handler (`take_armed`).
    static ARMED: Cell<Option<u32>> = const { Cell::new(None) };
}

/// Keeps `f` to run after the handler of the event about to be fired with
/// `args`: it gets `args` with what the handler wrote into them (an
/// argument the handler didn't declare keeps its value).
pub fn arm(args: &[Value], f: impl FnOnce(&[Value]) + 'static) {
    let args = args.to_vec();
    let merged: Then = Box::new(move |out: &[Value]| {
        let mut args = args;
        for (a, o) in args.iter_mut().zip(out) {
            *a = o.clone();
        }
        f(&args)
    });
    let id = NEXT.with(|n| {
        let id = n.get();
        n.set(id.wrapping_add(1).max(1));
        id
    });
    THEN.with(|t| t.borrow_mut().insert(id, merged));
    ARMED.with(|a| a.set(Some(id)));
}

/// The armed continuation's arguments start `n` parameters into the
/// handler's (after `This`).
fn shift_armed(n: usize) {
    let Some(id) = ARMED.with(|a| a.get()) else { return };
    THEN.with(|t| {
        let mut t = t.borrow_mut();
        if let Some(f) = t.remove(&id) {
            t.insert(id, Box::new(move |out: &[Value]| f(out.get(n..).unwrap_or(&[]))));
        }
    });
}

/// For a host queuing a handler: the continuation to go with it, if any.
pub fn take_armed() -> Vec<u32> {
    ARMED.with(|a| a.take()).into_iter().collect()
}

/// After firing: the continuation, if no host queued it (a native build ran
/// the handler already, or there was none) — the caller runs it now.
pub fn disarm() -> Option<Then> {
    let id = ARMED.with(|a| a.take())?;
    THEN.with(|t| t.borrow_mut().remove(&id))
}

/// Runs the continuations of a handler that has returned, given its
/// parameters' final values (none if it failed: the arguments stand).
pub fn run_all(then: Vec<u32>, params: &[Value]) {
    for id in then {
        if let Some(f) = THEN.with(|t| t.borrow_mut().remove(&id)) {
            f(params);
        }
    }
}

/// Fires an event through `fire` (which runs or queues the handler and
/// returns the arguments a compiled one left), then runs `then` with the
/// handler's answers — now if it ran, or when a queued one returns.
pub fn fire_then(args: &[Value], fire: impl FnOnce() -> Vec<Value>, then: impl FnOnce(&[Value]) + 'static) {
    arm(args, then);
    let out = fire();
    if let Some(then) = disarm() {
        then(&out);
    }
}

/// OnClose's `Action` (RapidQ's `caNone`, `caHide`, `caFree`,
/// `caMinimize`): what closing a form does. The runtime passes `caHide`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloseAction {
    /// `caNone` (or `False`): the form stays.
    Stay,
    /// `caHide`, `caFree`: the form goes.
    Close,
    /// `caMinimize`.
    Minimize,
}

/// The `Action` OnClose starts with.
pub const CA_HIDE: i64 = 1;

impl CloseAction {
    pub fn of(action: &Value) -> Self {
        match action.to_i64() {
            0 => CloseAction::Stay,
            3 => CloseAction::Minimize,
            _ => CloseAction::Close,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v_int;

    #[test]
    fn a_continuation_goes_with_the_queued_event_or_runs_at_once() {
        let log = Rc::new(RefCell::new(Vec::new()));
        // Queued (the interpreter): it goes with the event and gets what
        // the handler left in its parameters.
        let l = log.clone();
        arm(&[v_int(1), v_int(2)], move |a| l.borrow_mut().push(format!("queued {} {}", a[0].to_i64(), a[1].to_i64())));
        let ev = QueuedEvent { then: take_armed(), ..QueuedEvent::new(3, vec![]) };
        assert!(disarm().is_none() && log.borrow().is_empty());
        run_all(ev.then, &[v_int(0)]);
        // Not queued (a native build): the caller runs it.
        let l = log.clone();
        fire_then(&[v_int(5)], || vec![v_int(6), v_str("form")], move |a| l.borrow_mut().push(format!("now {}", a[0].to_i64())));
        assert_eq!(*log.borrow(), vec!["queued 0 2", "now 6"]);
    }

    #[test]
    fn compiled_handlers_write_their_parameters_back() {
        fn close(a: &mut [Value]) {
            a[0] = v_int(0);
        }
        let out = call(Handler::Out(2, close), "form", &[v_int(CA_HIDE)], |_, _| {});
        assert_eq!(CloseAction::of(&out[0]), CloseAction::Stay);
        assert_eq!(out[1].to_string_val(), "form");
    }

    #[test]
    fn a_handler_firing_its_own_events_keeps_the_outer_continuation() {
        thread_local!(static LOG: RefCell<Vec<i64>> = const { RefCell::new(Vec::new()) });
        fn inner(a: &mut [Value]) {
            a[0] = v_int(20);
        }
        fn outer(a: &mut [Value]) {
            fire_then(&[v_int(2)], || call(Handler::Out(1, inner), "b", &[v_int(2)], |_, _| {}), |a| LOG.with(|l| l.borrow_mut().push(a[0].to_i64())));
            a[0] = v_int(10);
        }
        fire_then(&[v_int(1)], || call(Handler::Out(1, outer), "a", &[v_int(1)], |_, _| {}), |a| LOG.with(|l| l.borrow_mut().push(a[0].to_i64())));
        assert_eq!(LOG.with(|l| l.borrow().clone()), vec![20, 10]);
    }

    #[test]
    fn an_instance_handler_answers_after_this() {
        let got = Rc::new(Cell::new(-1));
        let g = got.clone();
        arm(&[v_int(1)], move |a| g.set(a[0].to_i64()));
        call(Handler::IndirectThis(7, v_str("obj")), "grid", &[v_int(1)], |_, _| {});
        let then = take_armed();
        // The handler's parameters: This, CanSelect, Sender.
        run_all(then, &[v_str("obj"), v_int(0), v_str("grid")]);
        assert_eq!(got.get(), 0);
    }
}

/// QBUTTON's `Kind` (RapidQ's bkCustom 0 … bkAll 10, Delphi's TBitBtn): the
/// caption it gets and the ModalResult it closes a modal form with (bkClose
/// closes the form; bkHelp only gets its caption). None for bkCustom.
pub fn button_kind(kind: i64) -> Option<(&'static str, i64)> {
    Some(match kind {
        1 => ("OK", 1),
        2 => ("Cancel", 2),
        3 => ("&Help", 0),
        4 => ("&Yes", 6),
        5 => ("&No", 7),
        6 => ("&Close", 0),
        7 => ("Abort", 3),
        8 => ("&Retry", 4),
        9 => ("&Ignore", 5),
        10 => ("&All", 8),
        _ => return None,
    })
}

/// What `ShowModal` returns: the form's ModalResult, or mrCancel (2) when it
/// was closed some other way (its Close, the window's close button).
pub fn modal_result(stored: i64) -> i64 {
    if stored == 0 {
        2
    } else {
        stored
    }
}


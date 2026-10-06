//! The debuggee's [`Host`]: the native host, plus what a debug session
//! needs of it — the VM stops for requests and pauses (a yield it is asked
//! for), INPUT reads lines the debugger sends, and the runtime's events go
//! to a queue of our own (the native host's is private to it).

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, Mutex};

use rapidr_runtime_core::object as obj;
use rapidr_value::events::QueuedEvent;
use rapidr_value::Value;
use rapidr_vm::Host;
use rapidr_vm_host_native::NativeHost;

/// What the request reader raises for the program running on the main
/// thread.
#[derive(Default)]
pub struct Flags {
    /// `pause` came: stop at the next statement.
    pub pause: AtomicBool,
    /// Requests wait in the channel (new breakpoints while running).
    pub pending: AtomicBool,
}

/// Lines for INPUT, from `input` requests.
#[derive(Default)]
pub struct InputQueue {
    lines: Mutex<(VecDeque<String>, bool)>,
    ready: Condvar,
}

impl InputQueue {
    pub fn push(&self, line: String) {
        self.lines.lock().unwrap().0.push_back(line);
        self.ready.notify_all();
    }

    /// No more lines (the debugger is gone).
    pub fn close(&self) {
        self.lines.lock().unwrap().1 = true;
        self.ready.notify_all();
    }

    pub fn pop(&self) -> Option<String> {
        let mut guard = self.lines.lock().unwrap();
        loop {
            if let Some(line) = guard.0.pop_front() {
                return Some(line);
            }
            if guard.1 {
                return None;
            }
            guard = self.ready.wait(guard).unwrap();
        }
    }
}

thread_local! {
    /// Event handlers the runtime fired, waiting for the VM to run them.
    static EVENTS: RefCell<VecDeque<QueuedEvent>> = const { RefCell::new(VecDeque::new()) };
    /// Waits started by the program's code (ShowModal, DOEVENTS, the
    /// dialogs) still going on. A yield inside one would let the code that
    /// started it go on as if it had ended (today's VM), so there are none.
    static WAITS: Cell<usize> = const { Cell::new(0) };
}

/// Marks a handler run inside a wait (in its `then`): its return is seen
/// in [`Host::event_finished`]. (The runtime's continuation ids count up
/// from 1.)
const IN_WAIT: u32 = u32::MAX;

/// Whether a wait the program's code started is going on.
pub fn in_wait() -> bool {
    WAITS.with(Cell::get) > 0
}

pub struct DebugHost {
    pub inner: NativeHost,
    flags: std::sync::Arc<Flags>,
    input: std::sync::Arc<InputQueue>,
    /// Waits the VM left when a handler run inside one stopped (today's VM
    /// unwinds the wait to stop): each is taken up again once that
    /// handler returns, so the code that started it (ShowModal) waits on.
    pub abandoned: usize,
    /// A handler run inside a wait has returned: the VM asks
    /// [`Host::wait_started`] next, for the code below it.
    returned_in_wait: bool,
}

impl DebugHost {
    pub fn new(flags: std::sync::Arc<Flags>, input: std::sync::Arc<InputQueue>) -> Self {
        DebugHost { inner: NativeHost::default(), flags, input, abandoned: 0, returned_in_wait: false }
    }
}

/// The pump of a wait the program's code started, counted out when it ends.
fn pump_wait_serving(serve: &mut dyn FnMut()) -> Option<Value> {
    let result = obj::rp_pump_wait_serving(serve);
    if result.is_some() {
        WAITS.with(|w| w.set(w.get().saturating_sub(1)));
    }
    result
}

impl Host for DebugHost {
    // (a yield is how a running program hears from the debugger)
    const YIELDS: bool = true;

    fn yield_now(&mut self) -> bool {
        WAITS.with(Cell::get) == 0 && (self.flags.pause.load(Ordering::Relaxed) || self.flags.pending.load(Ordering::Relaxed))
    }

    fn call_builtin(&mut self, name: &str, args: &[Value]) -> Result<Value, String> {
        self.inner.call_builtin(name, args)
    }

    fn create_comp(&mut self, kind: &str, id: &str) -> Result<Value, String> {
        self.inner.create_comp(kind, id)
    }

    fn set_prop(&mut self, id: &str, name: &str, value: Value) -> Result<(), String> {
        self.inner.set_prop(id, name, value)
    }

    fn get_prop(&mut self, id: &str, name: &str) -> Result<Value, String> {
        self.inner.get_prop(id, name)
    }

    fn call_method(&mut self, id: &str, method: &str, args: &[Value]) -> Result<Value, String> {
        self.inner.call_method(id, method, args)
    }

    fn register_event(&mut self, id: &str, event: &str, handler_fn_index: u32) -> Result<(), String> {
        self.inner.register_event(id, event, handler_fn_index)
    }

    fn print(&mut self, s: &str) -> Result<(), String> {
        self.inner.print(s)
    }

    /// A line the debugger sent (the debug console's text while the
    /// program runs): standard input carries the session's requests.
    fn input(&mut self) -> Result<String, String> {
        Ok(self.input.pop().unwrap_or_default())
    }

    fn take_events(&mut self) -> Vec<QueuedEvent> {
        let mut events: Vec<QueuedEvent> = EVENTS.with(|q| q.borrow_mut().drain(..).collect());
        if in_wait() {
            for e in &mut events {
                if !e.then.contains(&IN_WAIT) {
                    e.then.push(IN_WAIT);
                }
            }
        }
        events
    }

    fn defer_events(&mut self, events: Vec<QueuedEvent>) {
        // Before anything queued since.
        EVENTS.with(|q| {
            let mut q = q.borrow_mut();
            for e in events.into_iter().rev() {
                q.push_front(e);
            }
        });
    }

    fn event_finished(&mut self, mut then: Vec<u32>, params: &[Value]) {
        if let Some(i) = then.iter().position(|&id| id == IN_WAIT) {
            then.remove(i);
            if self.abandoned > 0 {
                self.returned_in_wait = true;
            }
        }
        self.inner.event_finished(then, params)
    }

    fn wait_started(&mut self) -> bool {
        // (the code below a handler that stopped inside its wait: the wait
        // goes on — counted already. ShowModal's result is lost: the VM
        // asks again without a place for it)
        if std::mem::take(&mut self.returned_in_wait) && self.abandoned > 0 {
            self.abandoned -= 1;
            return true;
        }
        let started = self.inner.wait_started();
        if started {
            WAITS.with(|w| w.set(w.get() + 1));
        }
        started
    }

    fn pump(&mut self) -> Option<Value> {
        let result = self.inner.pump();
        if result.is_some() {
            WAITS.with(|w| w.set(w.get().saturating_sub(1)));
        }
        result
    }

    fn pump_serving(&self) -> Option<fn(&mut dyn FnMut()) -> Option<Value>> {
        Some(pump_wait_serving)
    }
}

/// Routes the runtime's events for bytecode handlers into our queue (the
/// native host's `install_event_queue`).
pub fn install_event_queue() -> Option<obj::IndirectDispatcher> {
    obj::rp_set_cooperative_waits(true);
    obj::rp_set_event_dispatcher(Box::new(|fn_index, args| {
        // (with the continuation of an event fired with rp_fire_event_then)
        let event = QueuedEvent { then: rapidr_value::events::take_armed(), ..QueuedEvent::new(fn_index, args.to_vec()) };
        EVENTS.with(|q| q.borrow_mut().push_back(event));
    }))
}

pub fn remove_event_queue(prev: Option<obj::IndirectDispatcher>) {
    match prev {
        Some(p) => {
            let _ = obj::rp_set_event_dispatcher(p);
        }
        None => {
            let _ = obj::rp_clear_event_dispatcher();
        }
    }
    obj::rp_set_cooperative_waits(false);
    EVENTS.with(|q| q.borrow_mut().clear());
}

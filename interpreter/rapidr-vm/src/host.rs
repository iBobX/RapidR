//! The [`Host`] trait — abstraction layer between the VM and the runtime
//! environment (desktop UI kernel runtime, web/DOM runtime, or a test stub).
//!
//! All side-effecting operations the VM can perform — printing, reading
//! input, calling builtins, creating GUI components, accessing properties,
//! invoking methods, registering event handlers — go through this trait.
//!
//! Hosts may return errors as `String` (mapped to [`crate::VmError::HostError`]).

use rapidr_value::{v_null, v_str, Value};

/// Side-effect surface required by the VM.
///
/// Methods return `Result<_, String>`; a `String` error message is wrapped
/// in `VmError::HostError` by the VM dispatch loop.
pub trait Host {
    /// Whether the VM asks [`Host::yield_now`] as it runs (the web host:
    /// the page has a single thread). `false` compiles the question out of
    /// the VM's loop.
    const YIELDS: bool = false;
    /// Asked every so often at a jump or a call (only if [`Host::YIELDS`]):
    /// `true` makes the VM stop with [`crate::VmError::Yielded`], ready to
    /// continue with [`crate::Vm::resume`] — the host gives the page a turn
    /// meanwhile and runs nothing else of the program.
    fn yield_now(&mut self) -> bool {
        false
    }
    fn call_builtin(&mut self, name: &str, args: &[Value]) -> Result<Value, String>;
    fn create_comp(&mut self, kind: &str, id: &str) -> Result<Value, String>;
    fn set_prop(&mut self, id: &str, name: &str, value: Value) -> Result<(), String>;
    fn get_prop(&mut self, id: &str, name: &str) -> Result<Value, String>;
    fn call_method(&mut self, id: &str, method: &str, args: &[Value]) -> Result<Value, String>;
    fn register_event(&mut self, id: &str, event: &str, handler_fn_index: u32) -> Result<(), String>;
    fn print(&mut self, s: &str) -> Result<(), String>;
    fn input(&mut self) -> Result<String, String>;
    /// Asked right after a builtin or INPUT returns: `true` makes the VM
    /// stop with [`crate::VmError::Suspended`] and wait for
    /// [`crate::Vm::resume_with`] (the web host shows an in-page dialog).
    fn suspend_requested(&mut self) -> bool {
        false
    }
    /// The event handlers the runtime queued since the last call (bytecode
    /// function index and arguments, oldest first). Runtimes never call
    /// into the VM themselves: a button click, a timer or an event a
    /// builtin fires is queued, and the VM runs it at a safe point — right
    /// after the host operation that fired it, or from a wait / the host's
    /// idle loop. Each handler runs to completion before the next one, as
    /// in a native build, and no event re-enters the VM mid-operation.
    fn take_events(&mut self) -> Vec<rapidr_value::events::QueuedEvent> {
        Vec::new()
    }
    /// Events taken with [`Host::take_events`] that couldn't run because a
    /// handler before them stopped (waits for a dialog): the host runs them
    /// later, from its idle loop.
    fn defer_events(&mut self, events: Vec<rapidr_value::events::QueuedEvent>) {
        let _ = events;
    }
    /// An event handler returned: its continuations (what the runtime
    /// does once the handler has run, `rapidr_value::events`).
    fn event_finished(&mut self, then: Vec<u32>, params: &[Value]) {
        // (what they fire belongs to the event that ran — an MDI child closed
        // activates another —: it runs at once, as the statement's own)
        rapidr_value::events::program_call(|| rapidr_value::events::run_all(then, params));
    }
    /// Asked after each host operation: `true` if it started a wait the VM
    /// must serve (a desktop `ShowModal`): the VM then calls [`Host::pump`]
    /// and runs queued events until the wait ends.
    fn wait_started(&mut self) -> bool {
        false
    }
    /// Asked right after [`Host::wait_started`] said yes: whether the wait
    /// is an event loop that goes on after a run-time error in a handler it
    /// ran (RapidQ's ShowModal — RC.EXE, 2026-10-16: the handler, and the
    /// handlers that fired it, stop at the error; no message; the form keeps
    /// answering). A DOEVENTS' or a Popup's turn passes the error on to the
    /// code that called it, as RapidQ's does (a main program's DOEVENTS
    /// loop ends there).
    fn wait_catches_errors(&mut self) -> bool {
        false
    }
    /// A handler run by a wait that catches errors failed
    /// ([`Host::wait_catches_errors`]): the error, worded as the program's
    /// run-time errors are, for the program's error output (RapidQ shows
    /// none; RapidR says it where a person running the program looks:
    /// standard error, the IDE's Output). The program goes on. A host that
    /// tells the program (RapidR's Application.OnError) has the error in
    /// parts and the handler it ended ([`crate::HandlerFailure`]).
    fn handler_failed(&mut self, failure: &crate::HandlerFailure) {
        eprintln!("{}", failure.text);
    }
    /// Processes pending UI events once for the innermost wait: `None`
    /// while it goes on, `Some(result)` when it's over (the result of the
    /// operation that started it).
    fn pump(&mut self) -> Option<Value> {
        Some(Value::Null)
    }
    /// [`Host::pump`] for a host whose wait can give the program turns from
    /// inside the window system's own loop (a native menu held open: the
    /// program's timers keep firing there, as RapidQ's do): a function — it
    /// borrows no host, so the VM can lend itself as `serve`, which runs
    /// the events queued so far whenever the host's wait calls it. `None`
    /// (the default): the VM calls [`Host::pump`].
    #[allow(clippy::type_complexity)]
    fn pump_serving(&self) -> Option<fn(&mut dyn FnMut()) -> Option<Value>> {
        None
    }
    /// The program is stopped for its debugger (a breakpoint, a step, a
    /// pause, a run-time error): its windows' own events are served once —
    /// painted from what they show, activated, raised, moved, resized —
    /// and nothing of the program runs: no handler, no timer; the user's
    /// clicks and keys to its windows are dropped. Returns once something
    /// happened, or another thread woke the host (a request for the
    /// debugger arrived). `false`: nothing to serve (no window shown, a
    /// test's headless host, a system loop holding the pump), and the
    /// debugger waits for its requests instead. A paused program's windows
    /// stay on the screen and reachable, as a debugged program's should.
    fn serve_paused(&mut self) -> bool {
        false
    }
    /// After [`Host::serve_paused`]: the user closed the stopped program's
    /// last window meanwhile — the debugger ends it, as its Stop does (none
    /// of its code runs, not its OnClose).
    fn closed_while_paused(&mut self) -> bool {
        false
    }
    /// The type (`RBUTTON` …) of the program's component `id`, for the
    /// debugger (`None`: no such component, or the host has none).
    fn component_type(&mut self, _id: &str) -> Option<String> {
        None
    }
    /// The components component `id` holds (those whose Parent it is: a
    /// form's controls, a panel's), in the order they were made — for the
    /// debugger; `id` empty: those no component holds (forms, timers,
    /// frames).
    fn component_children(&mut self, _id: &str) -> Vec<String> {
        Vec::new()
    }
}

/// A test/no-op [`Host`].
///
/// * `print` accumulates into [`StubHost::output`].
/// * `input` returns successive lines from [`StubHost::inputs`].
/// * `call_builtin("HOSTUPPER", [s])` uppercases its argument (used by tests).
/// * Other builtins return `Null`. Component / property / method ops are no-ops.
#[derive(Debug, Default)]
pub struct StubHost {
    pub output: String,
    pub inputs: Vec<String>,
    pub events: Vec<(String, String, u32)>,
}

impl Host for StubHost {
    fn call_builtin(&mut self, name: &str, args: &[Value]) -> Result<Value, String> {
        match name {
            "HOSTUPPER" => {
                let s = args.first().map(|v| v.to_string_val()).unwrap_or_default();
                Ok(v_str(&s.to_uppercase()))
            }
            "PRINT" => {
                for a in args { self.output.push_str(&a.to_string_val()); }
                Ok(v_null())
            }
            _ => Ok(v_null()),
        }
    }
    fn create_comp(&mut self, _kind: &str, id: &str) -> Result<Value, String> { Ok(v_str(id)) }
    fn set_prop(&mut self, _id: &str, _name: &str, _value: Value) -> Result<(), String> { Ok(()) }
    fn get_prop(&mut self, _id: &str, _name: &str) -> Result<Value, String> { Ok(v_null()) }
    fn call_method(&mut self, _id: &str, _m: &str, _args: &[Value]) -> Result<Value, String> { Ok(v_null()) }
    fn register_event(&mut self, id: &str, ev: &str, fi: u32) -> Result<(), String> {
        self.events.push((id.to_string(), ev.to_string(), fi));
        Ok(())
    }
    fn print(&mut self, s: &str) -> Result<(), String> { self.output.push_str(s); Ok(()) }
    fn input(&mut self) -> Result<String, String> {
        Ok(if self.inputs.is_empty() { String::new() } else { self.inputs.remove(0) })
    }
}

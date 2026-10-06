//! The program's end of the session, on today's VM: requests on standard
//! input (a reader thread), events framed on standard output among what
//! the program prints, the program on the main thread (the window
//! system's). A stop blocks the main thread and serves requests until one
//! lets the program go on; the program's windows are frozen meanwhile.

use std::collections::{HashMap, HashSet};
use std::io::{BufRead, Write};
use std::sync::atomic::Ordering;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::Arc;

use rapidr_bytecode::Module;
use rapidr_runtime_core::object as obj;
use rapidr_vm::{Host, StepMode, Vm, VmError};

use super::host::{DebugHost, Flags, InputQueue};
use super::inspect::{self, Children, GLOBALS_REF, LOCALS_REF, NEEDS_EVALUATOR, PAGE};
use crate::session_wire::{frame, Command, Event, EventBody, PlacedBreakpoint, Request, ScopeInfo, StackFrame, PROTOCOL_VERSION};

/// Sends an event to the debugger: standard output, framed, after what the
/// program printed so far (one lock, flushed: the order is kept).
pub fn send(event: &Event) {
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(frame(event).as_bytes());
    let _ = out.flush();
}

fn output(stream: &str, text: String) {
    send(&Event::new(EventBody::Output { stream: stream.into(), text }));
}

/// How the program was let go, to tell why it stopped next.
#[derive(Clone, Copy)]
enum Going {
    Continue,
    Step(StepMode),
    Pause,
    Entry,
}

/// A request that lets a stopped program go on.
enum Go {
    Continue,
    StepIn,
    StepOver,
    StepOut,
}

/// Where the program stopped.
enum Stop {
    /// At a statement: a breakpoint, a step, a pause, the entry.
    Line(&'static str),
    /// At a run-time error (break on error): it goes on unwinding after.
    Error { file: Option<String>, line: Option<u32>, description: String },
}

/// Reads requests from standard input: `stop` ends the process, `input`
/// feeds INPUT, `pause` raises the flag the VM's yields look at, the rest go
/// to the main thread. The debugger gone (the end of input): the program
/// ends.
fn spawn_reader(tx: Sender<Request>, flags: Arc<Flags>, input: Arc<InputQueue>) {
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            let Ok(line) = line else { break };
            if line.trim().is_empty() {
                continue;
            }
            let request = match Request::from_json(&line) {
                Ok(r) => r,
                Err(e) => {
                    send(&Event::error(0, e));
                    continue;
                }
            };
            let seq = request.seq;
            match &request.command {
                Command::Stop => {
                    if seq != 0 {
                        send(&Event::reply(seq, EventBody::Ok));
                    }
                    send(&Event::new(EventBody::Exited { code: 0 }));
                    std::process::exit(0);
                }
                Command::Input { text: Some(text), .. } => {
                    input.push(text.clone());
                    if seq != 0 {
                        send(&Event::reply(seq, EventBody::Ok));
                    }
                }
                Command::Pause => {
                    flags.pause.store(true, Ordering::Relaxed);
                    if seq != 0 {
                        send(&Event::reply(seq, EventBody::Ok));
                    }
                }
                _ => {
                    if tx.send(request).is_err() {
                        break;
                    }
                    flags.pending.store(true, Ordering::Relaxed);
                }
            }
        }
        // (the debugger is gone)
        input.close();
        std::process::exit(0);
    });
}

/// Runs `module` (compiled from `program`) as the program's end of a
/// session; returns its exit code.
pub fn run(module: Module, program: &str, args: Vec<String>) -> i32 {
    rapidr_runtime_core::value::resources::set_all(&module.resources);
    let canonical = std::fs::canonicalize(program).map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|_| program.to_string());
    rapidr_vm_host_native::set_program(&canonical, args);
    // (the runtime running it and the system's temporary folder, for a
    // program that runs others: as `rapidr run` sets them)
    if let Ok(exe) = std::env::current_exe() {
        std::env::set_var("RAPIDR_RUNTIME", exe);
    }
    std::env::set_var("RAPIDR_TEMP", std::env::temp_dir());

    let flags = Arc::new(Flags::default());
    let input = Arc::new(InputQueue::default());
    let (tx, rx) = mpsc::channel();
    spawn_reader(tx, flags.clone(), input.clone());

    let mut host = DebugHost::new(flags.clone(), input);
    let mut vm = Vm::new(&mut host);
    let prev = super::host::install_event_queue();
    send(&Event::new(EventBody::Ready { protocol: PROTOCOL_VERSION, runtime: format!("RapidR {}", env!("CARGO_PKG_VERSION")), program: program.to_string() }));

    let mut end = End::new(&module, rx, flags);
    let code = if end.until_start(&mut vm) {
        let result = vm.run(&module);
        match end.drive(&mut vm, result) {
            Ok(()) => {
                if vm.host.inner.has_components {
                    end.serve_app(&mut vm);
                }
                0
            }
            Err(e) => {
                end.error_stop(&mut vm, &e);
                output("stderr", format!("vm error: {e}\n"));
                1
            }
        }
    } else {
        0
    };

    // Stop timers first so ticks due before the loop ended don't fire
    // into a finished program.
    obj::rp_stop_all_timers();
    obj::rp_mark_shutting_down();
    super::host::remove_event_queue(prev);
    // Files the program never closed keep what was written.
    rapidr_runtime_core::value::basic_files::close_all();
    send(&Event::new(EventBody::Exited { code }));
    code
}

/// The session's state on the main thread.
struct End<'m> {
    module: &'m Module,
    requests: Receiver<Request>,
    flags: Arc<Flags>,
    break_on_error: bool,
    /// Breakpoints by file (its name, lower case): compiled lines.
    breakpoints: HashMap<String, HashSet<u32>>,
    /// Compiled lines where a statement starts.
    code_lines: HashSet<u32>,
    going: Going,
    children: Children,
}

impl<'m> End<'m> {
    fn new(module: &'m Module, requests: Receiver<Request>, flags: Arc<Flags>) -> Self {
        let code_lines = module.functions.iter().flat_map(|f| f.line_info.iter().map(|&(_, line)| line)).collect();
        End { module, requests, flags, break_on_error: false, breakpoints: HashMap::new(), code_lines, going: Going::Continue, children: Children::default() }
    }

    /// Serves requests until `start` (breakpoints set first, inspection
    /// refused). False if the debugger went away.
    fn until_start(&mut self, vm: &mut Vm<'_, DebugHost>) -> bool {
        loop {
            let Ok(request) = self.requests.recv() else { return false };
            let seq = request.seq;
            match request.command {
                Command::Start { debug, stop_on_entry, break_on_error, .. } => {
                    vm.debug_mode = debug || stop_on_entry || break_on_error;
                    self.break_on_error = self.break_on_error || break_on_error;
                    if stop_on_entry {
                        vm.step_mode = StepMode::Into;
                        self.going = Going::Entry;
                    }
                    if !debug {
                        // (run without debugging: nothing stops it)
                        vm.set_breakpoints(HashSet::new());
                        self.breakpoints.clear();
                        self.break_on_error = false;
                    }
                    reply(seq, EventBody::Ok);
                    return true;
                }
                command => {
                    if !self.common(vm, seq, command) {
                        reply_error(seq, "the program hasn't started");
                    }
                }
            }
        }
    }

    /// Requests answered the same way whether the program runs or not;
    /// false for the others.
    fn common(&mut self, vm: &mut Vm<'_, DebugHost>, seq: u64, command: Command) -> bool {
        match command {
            Command::SetBreakpoints { file, breakpoints } => {
                let placed = self.set_breakpoints(vm, &file, breakpoints.iter().map(|b| b.line));
                if !breakpoints.is_empty() {
                    vm.debug_mode = true;
                }
                reply(seq, EventBody::Breakpoints { file, breakpoints: placed });
            }
            Command::SetBreakOnError { enabled } => {
                self.break_on_error = enabled;
                if enabled {
                    vm.debug_mode = true;
                }
                reply(seq, EventBody::Ok);
            }
            Command::SetProperty { .. } | Command::Properties { .. } => reply_error(seq, NEEDS_EVALUATOR),
            _ => return false,
        }
        true
    }

    /// Replaces `file`'s breakpoints: a line without code moves to the next
    /// line of the same file that has some.
    fn set_breakpoints(&mut self, vm: &mut Vm<'_, DebugHost>, file: &str, lines: impl Iterator<Item = u32>) -> Vec<PlacedBreakpoint> {
        let map = &self.module.source_map;
        let index = map.files.iter().position(|f| f.eq_ignore_ascii_case(file));
        // (the file's lines with code: its line → compiled lines)
        let mut with_code: Vec<(u32, u32)> = Vec::new();
        if let Some(index) = index {
            for &(first, f, line, count) in &map.runs {
                if f as usize == index {
                    for k in 0..count {
                        if self.code_lines.contains(&(first + k)) {
                            with_code.push((line + k, first + k));
                        }
                    }
                }
            }
        }
        with_code.sort();
        let mut compiled = HashSet::new();
        let placed = lines
            .map(|line| {
                let actual = with_code.iter().find(|(l, _)| *l >= line).map(|(l, _)| *l);
                if let Some(actual) = actual {
                    compiled.extend(with_code.iter().filter(|(l, _)| *l == actual).map(|(_, c)| *c));
                }
                PlacedBreakpoint { line, verified: actual.is_some(), actual_line: actual }
            })
            .collect();
        self.breakpoints.insert(file.to_ascii_lowercase(), compiled);
        vm.set_breakpoints(self.breakpoints.values().flatten().copied().collect());
        placed
    }

    /// Runs the VM on from `result` until the program (or the event
    /// handler) is done or fails, serving its stops on the way.
    fn drive(&mut self, vm: &mut Vm<'_, DebugHost>, mut result: Result<(), VmError>) -> Result<(), VmError> {
        loop {
            result = match result {
                Err(VmError::Yielded) => {
                    self.serve_pending(vm);
                    self.take_pause(vm);
                    vm.resume(self.module)
                }
                Err(VmError::Paused) => {
                    // (stopped in a handler run inside a wait: the VM left
                    // the wait; the host takes it up again after)
                    if super::host::in_wait() {
                        vm.host.abandoned += 1;
                    }
                    match self.why_stopped(vm) {
                        Some(reason) => {
                            let go = self.serve_stop(vm, &Stop::Line(reason));
                            self.go(vm, go)
                        }
                        // (a breakpoint where a call returned into the
                        // middle of its line: not a stop — on as before)
                        None => {
                            if let Going::Step(mode) = self.going {
                                vm.step_mode = mode;
                            }
                            vm.resume(self.module)
                        }
                    }
                }
                other => return other,
            };
        }
    }

    /// A pause asked for: the VM stops at the next statement.
    fn take_pause(&mut self, vm: &mut Vm<'_, DebugHost>) {
        if self.flags.pause.swap(false, Ordering::Relaxed) {
            vm.step_mode = StepMode::Into;
            vm.last_line = 0;
            self.going = Going::Pause;
        }
    }

    /// Why the VM paused, as the debugger reports it; `None` when it isn't
    /// a stop.
    fn why_stopped(&mut self, vm: &Vm<'_, DebugHost>) -> Option<&'static str> {
        let Some(top) = vm.frames.last() else { return Some("pause") };
        let (line, start) = self.module.functions.get(top.fn_index as usize).and_then(|f| inspect::line_at(f, top.ip)).unwrap_or((0, true));
        let breakpoint = start && vm.breakpoints.contains(&line);
        let depth = vm.frames.len();
        match self.going {
            Going::Entry => {
                self.going = Going::Continue;
                Some("entry")
            }
            Going::Pause => {
                self.going = Going::Continue;
                Some("pause")
            }
            Going::Continue => breakpoint.then_some("breakpoint"),
            Going::Step(_) if breakpoint => Some("breakpoint"),
            Going::Step(mode) => {
                let done = match mode {
                    StepMode::Into => true,
                    StepMode::Over { target_depth } => depth <= target_depth,
                    StepMode::Out { target_depth } => depth < target_depth,
                    StepMode::None => false,
                };
                done.then_some("step")
            }
        }
    }

    /// Lets the stopped program go on as `go` says.
    fn go(&mut self, vm: &mut Vm<'_, DebugHost>, go: Go) -> Result<(), VmError> {
        self.children.clear();
        self.flags.pause.store(false, Ordering::Relaxed);
        send(&Event::new(EventBody::Continued));
        let depth = vm.frames.len();
        match go {
            Go::Continue => {
                self.going = Going::Continue;
                vm.resume(self.module)
            }
            Go::StepIn => {
                self.going = Going::Step(StepMode::Into);
                vm.step_into(self.module)
            }
            Go::StepOver => {
                self.going = Going::Step(StepMode::Over { target_depth: depth });
                vm.step_over(self.module)
            }
            Go::StepOut => {
                self.going = Going::Step(StepMode::Out { target_depth: depth });
                vm.step_out(self.module)
            }
        }
    }

    /// A run-time error: with break on error, the debugger sees it first
    /// (the main program's frames are still there; a handler's are gone).
    fn error_stop(&mut self, vm: &mut Vm<'_, DebugHost>, error: &VmError) {
        if !self.break_on_error || !vm.debug_mode {
            return;
        }
        let stop = match error {
            VmError::At { error, file, line } => Stop::Error { file: file.clone(), line: Some(*line), description: error.to_string() },
            other => Stop::Error { file: None, line: None, description: other.to_string() },
        };
        let _ = self.serve_stop(vm, &stop);
        self.children.clear();
        send(&Event::new(EventBody::Continued));
    }

    /// What came while the program ran (new breakpoints …).
    fn serve_pending(&mut self, vm: &mut Vm<'_, DebugHost>) {
        self.flags.pending.store(false, Ordering::Relaxed);
        loop {
            match self.requests.try_recv() {
                Ok(request) => {
                    let seq = request.seq;
                    if !self.common(vm, seq, request.command) {
                        reply_error(seq, "the program isn't stopped");
                    }
                }
                Err(TryRecvError::Empty) => return,
                // (the reader ends the process)
                Err(TryRecvError::Disconnected) => return,
            }
        }
    }

    /// Stopped: tells the debugger and serves its requests until one lets
    /// the program go on.
    fn serve_stop(&mut self, vm: &mut Vm<'_, DebugHost>, stop: &Stop) -> Go {
        let (reason, file, line, description) = match stop {
            Stop::Line(reason) => {
                let (file, line) = vm.frames.len().checked_sub(1).and_then(|top| inspect::frame_location(vm, self.module, top)).map_or((None, None), |(f, l)| (f, Some(l)));
                (*reason, file, line, None)
            }
            Stop::Error { file, line, description } => ("exception", file.clone(), *line, Some(description.clone())),
        };
        send(&Event::new(EventBody::Stopped { reason: reason.into(), file, line, description }));
        loop {
            let Ok(request) = self.requests.recv() else {
                // (the reader ends the process)
                return Go::Continue;
            };
            if let Some(go) = self.handle_stopped(vm, request, stop) {
                return go;
            }
        }
    }

    fn handle_stopped(&mut self, vm: &mut Vm<'_, DebugHost>, request: Request, stop: &Stop) -> Option<Go> {
        let seq = request.seq;
        let module = self.module;
        let go = match request.command {
            Command::Continue => Go::Continue,
            Command::StepIn => Go::StepIn,
            Command::StepOver => Go::StepOver,
            Command::StepOut => Go::StepOut,
            Command::StackTrace => {
                let mut frames: Vec<StackFrame> = (0..vm.frames.len())
                    .rev()
                    .filter_map(|i| {
                        let name = module.functions.get(vm.frames[i].fn_index as usize)?.name.clone();
                        let (file, line) = inspect::frame_location(vm, module, i).unwrap_or((None, 0));
                        Some(StackFrame { id: i as u32, name, file, line })
                    })
                    .collect();
                if let Stop::Error { file, line, .. } = stop {
                    match frames.first_mut() {
                        // (the faulting statement, not the last stop)
                        Some(top) => {
                            top.file = file.clone();
                            top.line = line.unwrap_or(top.line);
                        }
                        None => frames.push(StackFrame { id: 0, name: "(event handler)".into(), file: file.clone(), line: line.unwrap_or(0) }),
                    }
                }
                reply(seq, EventBody::StackTrace { frames });
                return None;
            }
            Command::Scopes { frame } => {
                let mut scopes = Vec::new();
                if (frame as usize) < vm.frames.len() {
                    scopes.push(ScopeInfo { name: "Locals".into(), reference: LOCALS_REF + frame });
                }
                scopes.push(ScopeInfo { name: "Globals".into(), reference: GLOBALS_REF });
                reply(seq, EventBody::Scopes { scopes });
                return None;
            }
            Command::Variables { reference, start, count } => {
                match self.children.variables(vm, module, reference, start.unwrap_or(0), count.unwrap_or(PAGE)) {
                    Ok(variables) => reply(seq, EventBody::Variables { variables }),
                    Err(e) => reply_error(seq, e),
                }
                return None;
            }
            Command::Evaluate { expr, frame, context } => {
                let frame = self.frame_for(vm, frame);
                let text = expr.trim();
                let assignment = (context.as_deref() == Some("repl") && !text.starts_with('?')).then(|| inspect::split_assignment(text)).flatten();
                let result = match assignment {
                    Some((target, value)) => inspect::assign(vm, module, frame, target, value),
                    None => inspect::evaluate(vm, module, frame, text.strip_prefix('?').unwrap_or(text)),
                };
                match result {
                    Ok(v) => {
                        let (result, kind, reference) = self.children.render(&v);
                        reply(seq, EventBody::Evaluate { result, kind, reference });
                    }
                    Err(e) => reply_error(seq, e),
                }
                return None;
            }
            Command::SetVariable { frame, name, value } => {
                let frame = self.frame_for(vm, frame);
                match inspect::assign(vm, module, frame, &name, &value) {
                    Ok(v) => {
                        let (result, kind, reference) = self.children.render(&v);
                        reply(seq, EventBody::Evaluate { result, kind, reference });
                    }
                    Err(e) => reply_error(seq, e),
                }
                return None;
            }
            Command::Start { .. } => {
                reply_error(seq, "the program has started already");
                return None;
            }
            command => {
                if !self.common(vm, seq, command) {
                    reply_error(seq, "not supported by this program end");
                }
                return None;
            }
        };
        reply(seq, EventBody::Ok);
        Some(go)
    }

    /// The frame a request means: the one it names, else the innermost.
    fn frame_for(&self, vm: &Vm<'_, DebugHost>, frame: Option<u32>) -> Option<usize> {
        match frame {
            Some(f) if (f as usize) < vm.frames.len() => Some(f as usize),
            _ => vm.frames.len().checked_sub(1),
        }
    }

    /// After MAIN, the program's windows until none is left (the native
    /// host's `serve_app`): UI events pumped, the handlers they queue run —
    /// a breakpoint in one stops there.
    fn serve_app(&mut self, vm: &mut Vm<'_, DebugHost>) {
        obj::rp_begin_app_wait();
        loop {
            self.run_queued(vm);
            // (the VM lent to the wait: a native menu held open, the
            // runtime's tracking ticks fire the due timers and their
            // handlers run here)
            let me = &mut *self;
            let vm2 = &mut *vm;
            if obj::rp_pump_wait_serving(&mut || me.run_queued(vm2)).is_some() {
                return;
            }
        }
    }

    /// The handlers queued so far, each to completion before the next.
    fn run_queued(&mut self, vm: &mut Vm<'_, DebugHost>) {
        loop {
            self.serve_pending(vm);
            let events = vm.host.take_events();
            if events.is_empty() {
                break;
            }
            for event in events {
                let fn_index = event.handler;
                // (each handler's first line is a new line, even the same
                // handler's again)
                vm.last_line = 0;
                self.take_pause(vm);
                let result = vm.invoke_event(self.module, event).map(drop);
                if let Err(e) = self.drive(vm, result) {
                    self.error_stop(vm, &e);
                    output("stderr", format!("[rapidr] event handler #{fn_index} failed: {e}\n"));
                }
            }
        }
    }
}

fn reply(seq: u64, body: EventBody) {
    if seq != 0 {
        send(&Event::reply(seq, body));
    }
}

fn reply_error(seq: u64, message: impl Into<String>) {
    if seq != 0 {
        send(&Event::error(seq, message));
    }
}

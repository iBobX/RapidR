//! The program's end of the session: requests served on a running VM.
//!
//! [`ProgramEnd`] answers what doesn't depend on the host — breakpoints,
//! the stack, scopes and variables, evaluation, setting variables and
//! properties — and tells the host what to do about the rest
//! ([`Control`]: start, go on, pause, stop, input). The hosts add the
//! transport and their way of running: the desktop blocks at a stop
//! ([`BlockingDebugger`], a `rapidr_vm::Debugger`), the web page returns
//! from the VM and continues it later.

use std::sync::mpsc::{Receiver, TryRecvError};

use rapidr_bytecode::Module;
use rapidr_value::Value;
use rapidr_vm::{Debugger, Host, Resume, StopInfo, StopReason, Vm, EVAL_FUEL};

use crate::protocol::{Command, Event, EventBody, PlacedBreakpoint, Request, ScopeInfo, StackFrame, Variable};

/// `variables { ref }` of the globals.
pub const GLOBALS_REF: u32 = 1;
/// `variables { ref }` of frame `i`'s locals: this plus `i`.
pub const LOCALS_REF: u32 = 1_000;
/// Children of a value shown in a stop (an array's elements, an object's
/// fields): this plus the value's index; valid until the program goes on.
pub const CHILDREN_REF: u32 = 1_000_000;
/// Elements of an array shown when `variables` doesn't say how many.
pub const PAGE: u32 = 100;

/// What the host does after a request ([`ProgramEnd::handle`]).
#[derive(Debug, Clone, PartialEq)]
pub enum Control {
    /// Nothing more: the reply says it all.
    None,
    /// Run the program (the VM is configured already).
    Start { stop_on_entry: bool },
    Continue,
    StepIn,
    StepOver,
    StepOut,
    Pause,
    Stop,
    /// A line for the program's INPUT.
    Input(String),
}

/// The program's end of the protocol, for one run.
#[derive(Default)]
pub struct ProgramEnd {
    /// Values whose children were offered in this stop.
    children: Vec<Value>,
    /// The next stop is the entry's (`stopOnEntry`).
    entry_pending: bool,
    /// The program has started (`start` came).
    pub started: bool,
}

impl ProgramEnd {
    pub fn new() -> Self {
        Self::default()
    }

    /// The program goes on: the references of the last stop are gone.
    pub fn resumed(&mut self) {
        self.children.clear();
    }

    /// The `stopped` event for where the VM stopped now.
    pub fn stopped_event<H: Host + ?Sized>(&mut self, vm: &Vm<'_, H>, module: &Module) -> Event {
        let reason = match vm.stop_reason {
            StopReason::Step if std::mem::take(&mut self.entry_pending) => "entry",
            StopReason::Breakpoint => "breakpoint",
            StopReason::Step => "step",
            StopReason::Pause => "pause",
            StopReason::Exception => "exception",
        };
        let (file, line) = match vm.frames.len().checked_sub(1).and_then(|top| vm.frame_location(module, top)) {
            Some((file, line)) => (file, Some(line)),
            None => (None, None),
        };
        let description = (vm.stop_reason == StopReason::Exception).then(|| vm.stop_error.clone()).flatten();
        Event::new(EventBody::Stopped { reason: reason.into(), file, line, description })
    }

    /// Serves one request on `vm` running `module` (`paused`: stopped at a
    /// breakpoint, a step, a pause or an error). Returns the reply, if the
    /// request wants one, and what the host must do.
    pub fn handle<H: Host + ?Sized>(&mut self, vm: &mut Vm<'_, H>, module: &Module, request: Request, paused: bool) -> (Option<Event>, Control) {
        let seq = request.seq;
        let ok = || Event::reply(seq, EventBody::Ok);
        let reply = |e: Event| (seq != 0).then_some(e);
        let not_stopped = || Event::error(seq, "the program isn't stopped");
        match request.command {
            Command::Start { debug, stop_on_entry, break_on_error, .. } => {
                if self.started {
                    return (reply(Event::error(seq, "the program has started already")), Control::None);
                }
                self.started = true;
                vm.debug_mode = debug || stop_on_entry || break_on_error;
                vm.break_on_error = break_on_error;
                if stop_on_entry {
                    vm.step_mode = rapidr_vm::StepMode::Into;
                    self.entry_pending = true;
                }
                (reply(ok()), Control::Start { stop_on_entry })
            }
            Command::Stop => (reply(ok()), Control::Stop),
            Command::Pause => {
                if !paused {
                    vm.request_pause();
                }
                (reply(ok()), Control::Pause)
            }
            Command::SetBreakpoints { file, breakpoints } => {
                let lines: Vec<u32> = breakpoints.iter().map(|b| b.line).collect();
                let placed = vm.set_file_breakpoints(module, &file, &lines);
                // (a program that asks for breakpoints is debugged)
                if !lines.is_empty() && !self.started {
                    vm.debug_mode = true;
                }
                let breakpoints = lines
                    .iter()
                    .zip(placed)
                    .map(|(&line, at)| PlacedBreakpoint { line, verified: at.is_some(), actual_line: at })
                    .collect();
                (reply(Event::reply(seq, EventBody::Breakpoints { file, breakpoints })), Control::None)
            }
            Command::SetBreakOnError { enabled } => {
                vm.break_on_error = enabled;
                if enabled && !self.started {
                    vm.debug_mode = true;
                }
                (reply(ok()), Control::None)
            }
            Command::Continue | Command::StepIn | Command::StepOver | Command::StepOut if !paused => (reply(not_stopped()), Control::None),
            Command::Continue => (reply(ok()), Control::Continue),
            Command::StepIn => (reply(ok()), Control::StepIn),
            Command::StepOver => (reply(ok()), Control::StepOver),
            Command::StepOut => (reply(ok()), Control::StepOut),
            Command::StackTrace if !paused => (reply(not_stopped()), Control::None),
            Command::StackTrace => {
                let frames = (0..vm.frames.len())
                    .rev()
                    .filter_map(|i| {
                        let f = &vm.frames[i];
                        let name = module.functions.get(f.fn_index as usize)?.name.clone();
                        let (file, line) = vm.frame_location(module, i).unwrap_or((None, 0));
                        Some(StackFrame { id: i as u32, name, file, line })
                    })
                    .collect();
                (reply(Event::reply(seq, EventBody::StackTrace { frames })), Control::None)
            }
            Command::Scopes { frame } => {
                if !paused || frame as usize >= vm.frames.len() {
                    return (reply(Event::error(seq, format!("no frame {frame}"))), Control::None);
                }
                let scopes = vec![
                    ScopeInfo { name: "Locals".into(), reference: LOCALS_REF + frame },
                    ScopeInfo { name: "Globals".into(), reference: GLOBALS_REF },
                ];
                (reply(Event::reply(seq, EventBody::Scopes { scopes })), Control::None)
            }
            Command::Variables { reference, start, count } => {
                let event = match self.variables(vm, module, reference, start.unwrap_or(0), count.unwrap_or(PAGE)) {
                    Ok(variables) => Event::reply(seq, EventBody::Variables { variables }),
                    Err(e) => Event::error(seq, e),
                };
                (reply(event), Control::None)
            }
            Command::Evaluate { expr, frame, context } => {
                let frame = self.frame_for(vm, frame, paused);
                let repl = context.as_deref() == Some("repl");
                let text = expr.trim();
                let result = if repl && !text.starts_with('?') {
                    self.run(vm, module, frame, text, true).map(|_| (String::new(), String::new(), 0))
                } else {
                    let text = text.strip_prefix('?').unwrap_or(text);
                    self.evaluate(vm, module, frame, text).map(|v| self.render(&v))
                };
                let event = match result {
                    Ok((result, kind, reference)) => Event::reply(seq, EventBody::Evaluate { result, kind, reference }),
                    Err(e) => Event::error(seq, e),
                };
                (reply(event), Control::None)
            }
            Command::SetVariable { frame, name, value } => {
                let frame = self.frame_for(vm, frame, paused);
                let event = match self
                    .run(vm, module, frame, &format!("{name} = ({value})"), true)
                    .and_then(|_| self.evaluate(vm, module, frame, &name))
                {
                    Ok(v) => {
                        let (result, kind, reference) = self.render(&v);
                        Event::reply(seq, EventBody::Evaluate { result, kind, reference })
                    }
                    Err(e) => Event::error(seq, e),
                };
                (reply(event), Control::None)
            }
            Command::SetProperty { object, prop, value } => {
                let event = match self.run(vm, module, None, &format!("{object}.{prop} = ({value})"), false) {
                    Ok(()) => ok(),
                    Err(e) => Event::error(seq, e),
                };
                (reply(event), Control::None)
            }
            Command::Properties { object } => {
                let event = match vm.host.component_properties(&object) {
                    Some((kind, mut props)) => {
                        props.sort_by(|a, b| a.0.cmp(&b.0));
                        let properties = props
                            .into_iter()
                            .map(|(name, v)| {
                                let (value, kind, reference) = self.render(&v);
                                Variable { name, value, kind, reference, count: 0 }
                            })
                            .collect();
                        Event::reply(seq, EventBody::Properties { kind, properties })
                    }
                    None => Event::error(seq, format!("{object}: no such component")),
                };
                (reply(event), Control::None)
            }
            Command::Input { text: Some(text), .. } => (reply(ok()), Control::Input(text)),
            Command::Input { .. } => (reply(Event::error(seq, "input to a form drawn in the IDE comes with remote forms (I5)")), Control::None),
        }
    }

    /// The frame a request means: the one it names, else the innermost
    /// while stopped, else none (the program's top level: globals).
    fn frame_for<H: Host + ?Sized>(&self, vm: &Vm<'_, H>, frame: Option<u32>, paused: bool) -> Option<usize> {
        match frame {
            Some(f) if (f as usize) < vm.frames.len() && paused => Some(f as usize),
            _ if paused => vm.frames.len().checked_sub(1),
            _ => None,
        }
    }

    /// Evaluates a BASIC expression in `frame`.
    pub fn evaluate<H: Host + ?Sized>(&mut self, vm: &mut Vm<'_, H>, module: &Module, frame: Option<usize>, expr: &str) -> Result<Value, String> {
        let source = format!("{} = ({expr})", rapidr_bcgen::SNIPPET_RESULT);
        let snippet = compile(vm, module, frame, &source)?;
        vm.evaluate(&snippet.module, snippet.function, frame, false, EVAL_FUEL).map_err(|e| e.to_string())
    }

    /// Runs BASIC statements in `frame` (what they assign to its locals is
    /// written back when `write_back`).
    pub fn run<H: Host + ?Sized>(&mut self, vm: &mut Vm<'_, H>, module: &Module, frame: Option<usize>, code: &str, write_back: bool) -> Result<(), String> {
        let snippet = compile(vm, module, frame, code)?;
        vm.evaluate(&snippet.module, snippet.function, frame, write_back, EVAL_FUEL).map(drop).map_err(|e| e.to_string())
    }

    fn variables<H: Host + ?Sized>(&mut self, vm: &Vm<'_, H>, module: &Module, reference: u32, start: u32, count: u32) -> Result<Vec<Variable>, String> {
        let mut named: Vec<(String, Value)> = Vec::new();
        if reference == GLOBALS_REF {
            for (name, v) in vm.global_values(module) {
                if !hidden(name) {
                    named.push((name.to_string(), v.clone()));
                }
            }
            named.sort_by_key(|(n, _)| n.to_ascii_lowercase());
        } else if (LOCALS_REF..CHILDREN_REF).contains(&reference) {
            let i = (reference - LOCALS_REF) as usize;
            let frame = vm.frames.get(i).ok_or_else(|| format!("no frame {i}"))?;
            let names = module.functions.get(frame.fn_index as usize).map(|f| &f.local_names[..]).unwrap_or(&[]);
            for (slot, v) in frame.locals.iter().enumerate() {
                match names.get(slot) {
                    Some(name) if !hidden(name) => named.push((name.clone(), v.clone())),
                    _ => {}
                }
            }
        } else if reference >= CHILDREN_REF {
            let value = self.children.get((reference - CHILDREN_REF) as usize).cloned().ok_or("that value is gone (the program went on)")?;
            match &value {
                Value::Array(a) => {
                    let a = a.borrow();
                    let end = start.saturating_add(count).min(a.data.len() as u32);
                    for k in start..end {
                        named.push((index_label(&a.bounds, k as usize), a.data[k as usize].clone()));
                    }
                }
                Value::Object(o) => {
                    for (name, f) in o.names.iter().zip(o.fields.borrow().iter()) {
                        named.push((name.clone(), f.clone()));
                    }
                }
                _ => {}
            }
        } else {
            return Err(format!("no variables {reference}"));
        }
        Ok(named
            .into_iter()
            .map(|(name, v)| {
                let (value, kind, reference) = self.render(&v);
                let count = match &v {
                    Value::Array(a) => a.borrow().data.len() as u32,
                    Value::Object(o) => o.names.len() as u32,
                    _ => 0,
                };
                Variable { name, value, kind, reference, count }
            })
            .collect())
    }

    /// A value as the debugger shows it, its kind, and a reference to its
    /// children (0: none).
    fn render(&mut self, v: &Value) -> (String, String, u32) {
        match v {
            Value::Null => (String::new(), "Empty".into(), 0),
            Value::Integer(_) => (rapidr_value::format::print_text(v), "Integer".into(), 0),
            Value::Double(_) => (rapidr_value::format::print_text(v), "Double".into(), 0),
            Value::Boolean(b) => ((if *b { "True" } else { "False" }).into(), "Boolean".into(), 0),
            Value::String(s) => (quoted(s), "String".into(), 0),
            Value::Array(a) => {
                let bounds = a.borrow().bounds.iter().map(|(lo, hi)| format!("{lo} TO {hi}")).collect::<Vec<_>>().join(", ");
                (format!("Array({bounds})"), "Array".into(), self.child(v))
            }
            Value::Object(o) => (o.type_name.clone(), o.type_name.clone(), self.child(v)),
        }
    }

    fn child(&mut self, v: &Value) -> u32 {
        self.children.push(v.clone());
        CHILDREN_REF + (self.children.len() - 1) as u32
    }
}

/// Compiler-made names (`__for_end1`, the result slot) aren't shown.
fn hidden(name: &str) -> bool {
    name.is_empty() || name.starts_with("__")
}

/// `"text"` with BASIC's view of a quote (`""`) and control characters
/// escaped, as a debugger shows a string.
fn quoted(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\"\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\x{:02X}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// `(2, 3)`: element `k`'s subscripts (stored by the last one first).
fn index_label(bounds: &[(i64, i64)], mut k: usize) -> String {
    let mut parts = vec![0i64; bounds.len()];
    for (d, &(lo, hi)) in bounds.iter().enumerate().rev() {
        let extent = (hi - lo + 1).max(1) as usize;
        parts[d] = lo + (k % extent) as i64;
        k /= extent;
    }
    format!("({})", parts.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", "))
}

/// Parses and compiles `source` against frame `frame`'s function.
fn compile<H: Host + ?Sized>(vm: &Vm<'_, H>, module: &Module, frame: Option<usize>, source: &str) -> Result<rapidr_bcgen::Snippet, String> {
    let mut text = source.to_string();
    text.push('\n');
    let tokens = rapidr_lexer::Lexer::new(&text, None).tokenize().map_err(|e| e.to_string())?;
    let program = rapidr_parser::parse_tokens(&tokens).map_err(|e| e.to_string().trim().to_string())?;
    let fn_index = frame.and_then(|i| vm.frames.get(i)).map(|f| f.fn_index);
    let globals = &vm.globals;
    rapidr_bcgen::compile_snippet(module, fn_index, &program.statements, &|i| globals.get(i).is_some_and(Option::is_some))
}

/// A [`Debugger`] for hosts that can wait: at a stop it sends the `stopped`
/// event and serves requests from `requests` until one lets the program go
/// on; when interrupted while running it serves what's pending.
pub struct BlockingDebugger {
    pub end: ProgramEnd,
    requests: Receiver<Request>,
    send: Box<dyn Fn(&Event)>,
    /// What the host must still do (an `input`, a `stop`) — the debugger
    /// can't, the host takes it with [`Self::take_controls`].
    controls: Vec<Control>,
}

impl BlockingDebugger {
    pub fn new(requests: Receiver<Request>, send: Box<dyn Fn(&Event)>) -> Self {
        BlockingDebugger { end: ProgramEnd::new(), requests, send, controls: Vec::new() }
    }

    /// Serves requests until `start` (breakpoints set first, inspection
    /// refused): the program then runs. `None` if the IDE went away.
    pub fn until_start<H: Host + ?Sized>(&mut self, vm: &mut Vm<'_, H>, module: &Module) -> Option<bool> {
        loop {
            let request = self.requests.recv().ok()?;
            let (reply, control) = self.end.handle(vm, module, request, false);
            if let Some(reply) = reply {
                (self.send)(&reply);
            }
            match control {
                Control::Start { stop_on_entry } => return Some(stop_on_entry),
                Control::Stop => return None,
                Control::None | Control::Pause => {}
                other => self.controls.push(other),
            }
        }
    }

    /// What the host must still do.
    pub fn take_controls(&mut self) -> Vec<Control> {
        std::mem::take(&mut self.controls)
    }
}

impl<H: Host + ?Sized> Debugger<H> for BlockingDebugger {
    fn stopped(&mut self, vm: &mut Vm<'_, H>, module: &Module, _stop: &StopInfo) -> Resume {
        let event = self.end.stopped_event(vm, module);
        (self.send)(&event);
        loop {
            let Ok(request) = self.requests.recv() else { return Resume::Terminate };
            let (reply, control) = self.end.handle(vm, module, request, true);
            if let Some(reply) = reply {
                (self.send)(&reply);
            }
            let resume = match control {
                Control::Continue => Resume::Continue,
                Control::StepIn => Resume::StepIn,
                Control::StepOver => Resume::StepOver,
                Control::StepOut => Resume::StepOut,
                Control::Stop => return Resume::Terminate,
                Control::None | Control::Pause | Control::Start { .. } => continue,
                other => {
                    self.controls.push(other);
                    continue;
                }
            };
            self.end.resumed();
            (self.send)(&Event::new(EventBody::Continued));
            return resume;
        }
    }

    fn interrupted(&mut self, vm: &mut Vm<'_, H>, module: &Module) -> bool {
        let mut pause = false;
        loop {
            match self.requests.try_recv() {
                Ok(request) => {
                    let (reply, control) = self.end.handle(vm, module, request, false);
                    if let Some(reply) = reply {
                        (self.send)(&reply);
                    }
                    match control {
                        Control::Pause => pause = true,
                        Control::None | Control::Start { .. } | Control::Continue | Control::StepIn | Control::StepOver | Control::StepOut => {}
                        other => self.controls.push(other),
                    }
                }
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => return pause,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::SourceBreakpoint;
    use rapidr_vm::StubHost;
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::mpsc;

    fn compile_program(src: &str) -> Module {
        let tokens = rapidr_lexer::Lexer::new(src, None).tokenize().unwrap();
        let program = rapidr_parser::parse_tokens(&tokens).unwrap();
        let mut m = rapidr_bcgen::compile_program_with_source(&program, Some(src)).unwrap().module;
        m.source_map = rapidr_bytecode::SourceMap::from_origins("prog.bas", (1..=src.lines().count() as u32).map(|l| (None, l)));
        m
    }

    fn req(seq: u64, command: Command) -> Request {
        Request::new(seq, command)
    }

    #[test]
    fn a_blocking_session_stops_inspects_steps_and_goes_on() {
        let src = "DIM total AS INTEGER, t(1 TO 3) AS INTEGER\nt(2) = 7\nSUB Add(n)\n  total = total + n\nEND SUB\ntotal = 1\nAdd 41\nPRINT total\n";
        let m = compile_program(src);
        let (tx, rx) = mpsc::channel();
        let sent: Rc<RefCell<Vec<Event>>> = Rc::default();
        let log = sent.clone();
        let mut dbg = BlockingDebugger::new(rx, Box::new(move |e| log.borrow_mut().push(e.clone())));
        let mut host = StubHost::default();
        let mut vm = Vm::new(&mut host);
        // before start: a breakpoint inside the SUB, then start
        tx.send(req(1, Command::SetBreakpoints { file: "prog.bas".into(), breakpoints: vec![SourceBreakpoint { line: 4, ..Default::default() }] })).unwrap();
        tx.send(req(2, Command::Start { program: None, args: vec![], debug: true, stop_on_entry: false, break_on_error: false })).unwrap();
        assert_eq!(dbg.until_start(&mut vm, &m), Some(false));
        // what the IDE does at the stop, queued before the program runs
        for (seq, c) in [
            (3, Command::StackTrace),
            (4, Command::Variables { reference: LOCALS_REF + 1, start: None, count: None }),
            (5, Command::Evaluate { expr: "total + n".into(), frame: None, context: None }),
            (6, Command::Variables { reference: GLOBALS_REF, start: None, count: None }),
            (7, Command::Variables { reference: CHILDREN_REF, start: None, count: None }),
            (8, Command::SetVariable { frame: None, name: "n".into(), value: "n + 1".into() }),
            (9, Command::Evaluate { expr: "total = 100".into(), frame: None, context: Some("repl".into()) }),
            (10, Command::Continue),
        ] {
            tx.send(req(seq, c)).unwrap();
        }
        vm.debug_mode = true;
        vm.debugger = Some(Box::new(dbg));
        vm.run(&m).unwrap();
        drop(vm);
        assert_eq!(host.output, "142\n");
        let sent = sent.borrow();
        let by_re = |re: u64| sent.iter().find(|e| e.re == Some(re)).map(|e| e.body.clone()).unwrap();
        assert!(matches!(by_re(1), EventBody::Breakpoints { ref breakpoints, .. } if breakpoints[0].verified && breakpoints[0].actual_line == Some(4)));
        assert!(sent.iter().any(|e| e.body == EventBody::Stopped { reason: "breakpoint".into(), file: Some("prog.bas".into()), line: Some(4), description: None }));
        let EventBody::StackTrace { frames } = by_re(3) else { panic!() };
        assert_eq!(frames.iter().map(|f| (f.name.as_str(), f.line)).collect::<Vec<_>>(), [("Add", 4), ("__main", 7)]);
        let EventBody::Variables { variables } = by_re(4) else { panic!() };
        assert_eq!(variables.iter().map(|v| (v.name.as_str(), v.value.as_str())).collect::<Vec<_>>(), [("n", "41")]);
        assert!(matches!(by_re(5), EventBody::Evaluate { ref result, .. } if result == "42"));
        let EventBody::Variables { variables } = by_re(6) else { panic!() };
        let t = variables.iter().find(|v| v.name.eq_ignore_ascii_case("t")).unwrap();
        assert_eq!((t.value.as_str(), t.reference, t.count), ("Array(1 TO 3)", CHILDREN_REF, 3));
        let EventBody::Variables { variables } = by_re(7) else { panic!() };
        assert_eq!(variables.iter().map(|v| (v.name.as_str(), v.value.as_str())).collect::<Vec<_>>(), [("(1)", "0"), ("(2)", "7"), ("(3)", "0")]);
        assert!(matches!(by_re(8), EventBody::Evaluate { ref result, .. } if result == "42"));
        assert_eq!(by_re(9), EventBody::Evaluate { result: String::new(), kind: String::new(), reference: 0 });
        assert!(sent.iter().any(|e| e.body == EventBody::Continued));
    }
}

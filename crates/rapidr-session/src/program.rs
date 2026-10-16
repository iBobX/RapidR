//! The program's end of the session: requests served on a running VM.
//!
//! [`ProgramEnd`] answers what doesn't depend on the host — breakpoints,
//! the stack, scopes and variables, evaluation, setting variables and
//! properties — and tells the host what to do about the rest
//! ([`Control`]: start, go on, pause, stop, input). The hosts add the
//! transport and their way of running: the desktop blocks at a stop
//! ([`BlockingDebugger`], a `rapidr_vm::Debugger`), the web page returns
//! from the VM and continues it later.
//!
//! What a stop shows opens like a tree, to any depth: an array's elements
//! and a TYPE's fields; a component's properties (the language registry's,
//! read as the program reads them) and the components it holds (a form's
//! controls); an RDataFrame as a table — its rows, each opening to its
//! cells by column; an RNum's numbers. Arrays, frames and RNums page
//! (`variables { start, count }`). Every value carries the expression that
//! names it (`evaluateName`), for watches, copying and setting it.

use std::sync::mpsc::{Receiver, TryRecvError};

use rapidr_bcgen::GlobalSlot;
use rapidr_bytecode::Module;
use rapidr_value::Value;
use rapidr_vm::{Debugger, Host, Resume, StopInfo, StopReason, Vm, EVAL_FUEL};

use crate::protocol::{Command, Event, EventBody, PlacedBreakpoint, Request, ScopeInfo, StackFrame, StepInTarget, Variable};

pub use crate::protocol::{CHILDREN_REF, COMPONENTS_REF, GLOBALS_REF, LOCALS_REF};
/// Elements of an array shown when `variables` doesn't say how many.
pub const PAGE: u32 = 100;

/// Characters of a string a value's text shows; the whole of it is
/// `evaluate` with context `clipboard` (copying it, the text viewer).
pub const SHOWN_CHARS: usize = 1_000;

/// A pause's description when the program was waiting for its events (a
/// ShowModal, a dialog): the `stopped` event's.
pub const WAITING_NOTE: &str = "Waiting for events";

/// What the host does after a request ([`ProgramEnd::handle`]).
#[derive(Debug, Clone, PartialEq)]
pub enum Control {
    /// Nothing more: the reply says it all.
    None,
    /// Run the program (the VM is configured already).
    Start { stop_on_entry: bool },
    Continue,
    StepIn,
    /// Step Into Specific: into this function's call only.
    StepInTarget(u32),
    StepOver,
    StepOut,
    Pause,
    Stop,
    /// A line for the program's INPUT.
    Input(String),
    /// The program moved where it's stopped (Set Next Statement): send
    /// [`ProgramEnd::stopped_event`] now, after the reply.
    Moved,
}

/// Something a stop showed whose children can be asked for
/// (`CHILDREN_REF` + its index; valid until the program goes on).
#[derive(Debug, Clone)]
enum Node {
    /// An array's elements, a TYPE's fields; the expression naming it.
    Value(Value, String),
    /// A component: its properties, then the components it holds.
    Component(String),
    /// The components a component holds (a form's controls).
    Parts(String),
    /// An RDataFrame's rows.
    Frame(String),
    /// Row `r` of an RDataFrame: its cells by column.
    Row(String, usize),
    /// An RNum's numbers.
    Num(String),
}

/// The program's end of the protocol, for one run.
#[derive(Default)]
pub struct ProgramEnd {
    /// What this stop offered to open.
    nodes: Vec<Node>,
    /// The next stop is the entry's (`stopOnEntry`).
    entry_pending: bool,
    /// The next `stopped` event is Set Next Statement's (reason `goto`).
    goto_pending: bool,
    /// The program has started (`start` came).
    pub started: bool,
    /// The breakpoints as the IDE set them, where they landed, and their
    /// hits: what decides whether a breakpoint stops ([`Self::at_breakpoint`]).
    breakpoints: Vec<Placed>,
    /// Why the program stopped beyond its reason (a condition that failed
    /// to evaluate): the next `stopped` event's description.
    note: Option<String>,
}

/// A breakpoint where it landed, with its condition, hit count, log
/// message and how often it was reached with its condition true.
#[derive(Debug, Clone)]
struct Placed {
    file: String,
    line: u32,
    condition: Option<String>,
    hit: Option<String>,
    log: Option<String>,
    hits: u32,
}

impl Placed {
    fn same_rules(&self, other: &Placed) -> bool {
        self.line == other.line && self.condition == other.condition && self.hit == other.hit && self.log == other.log && self.file.eq_ignore_ascii_case(&other.file)
    }
}

/// Whether hit `hits` (from 1) of a breakpoint satisfies its hit count
/// `rule`: `N` or `= N` (the Nth hit only), `>= N`, `> N`, `< N`, `<= N`,
/// `% N` (every Nth). A rule that doesn't read as one of these always
/// does.
pub fn hit_matches(rule: &str, hits: u32) -> bool {
    let r = rule.trim();
    let (op, rest) = ["==", ">=", "<=", "=", ">", "<", "%"].iter().find_map(|op| r.strip_prefix(op).map(|rest| (*op, rest))).unwrap_or(("=", r));
    let Ok(n) = rest.trim().parse::<u32>() else { return true };
    match op {
        ">=" => hits >= n,
        "<=" => hits <= n,
        ">" => hits > n,
        "<" => hits < n,
        "%" => n == 0 || hits % n == 0,
        _ => hits == n,
    }
}

/// A logpoint's message: each `{expression}` replaced by its value (`{{`
/// and `}}` are braces), `eval` giving the value's text.
pub fn interpolate(message: &str, mut eval: impl FnMut(&str) -> String) -> String {
    let mut out = String::new();
    let mut chars = message.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' if chars.peek() == Some(&'{') => {
                chars.next();
                out.push('{');
            }
            '}' if chars.peek() == Some(&'}') => {
                chars.next();
                out.push('}');
            }
            '{' => {
                let mut expr = String::new();
                let mut depth = 0;
                for c in chars.by_ref() {
                    match c {
                        '{' => depth += 1,
                        '}' if depth == 0 => break,
                        '}' => depth -= 1,
                        _ => {}
                    }
                    expr.push(c);
                }
                out.push_str(&eval(expr.trim()));
            }
            c => out.push(c),
        }
    }
    out
}

/// A child's name: the expression naming its parent (`base`) and its own
/// part — an element's subscripts (`(2, 3)`) follow the array, a field or
/// a property follows a dot. Empty when the parent can't be named.
fn child_name(base: &str, part: &str) -> String {
    if base.is_empty() {
        String::new()
    } else if part.starts_with('(') {
        format!("{base}{part}")
    } else {
        format!("{base}.{part}")
    }
}

impl ProgramEnd {
    pub fn new() -> Self {
        Self::default()
    }

    /// The program goes on: the references of the last stop are gone.
    pub fn resumed(&mut self) {
        self.nodes.clear();
    }

    /// The `stopped` event for where the VM stopped now.
    pub fn stopped_event<H: Host + ?Sized>(&mut self, vm: &Vm<'_, H>, module: &Module) -> Event {
        let reason = match vm.stop_reason {
            _ if std::mem::take(&mut self.goto_pending) => "goto",
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
        let description = match vm.stop_reason {
            StopReason::Exception => vm.stop_error.clone(),
            // (paused while it waits: a ShowModal, a dialog — no code runs)
            StopReason::Pause if vm.frames.last().is_some_and(|f| f.waiting) => Some(WAITING_NOTE.to_string()),
            _ => self.note.take(),
        };
        Event::new(EventBody::Stopped { reason: reason.into(), file, line, description })
    }

    /// At a breakpoint's stop: whether the program stops there. The
    /// breakpoint's condition is evaluated in the stopped frame (false: it
    /// goes on, the hit not counted), then its hit count is checked, then a
    /// logpoint prints its message (`output` gets the event) and goes on.
    /// A condition that fails to evaluate stops, saying why. Other stops
    /// (a step, a pause, an error) always stop.
    pub fn at_breakpoint<H: Host + ?Sized>(&mut self, vm: &mut Vm<'_, H>, module: &Module, output: &mut dyn FnMut(Event)) -> bool {
        if vm.stop_reason != StopReason::Breakpoint || self.entry_pending {
            return true;
        }
        let Some(top) = vm.frames.len().checked_sub(1) else { return true };
        let Some((Some(file), line)) = vm.frame_location(module, top) else { return true };
        let name = |f: &str| f.rsplit(['/', '\\']).next().unwrap_or(f).to_ascii_lowercase();
        let Some(i) = self.breakpoints.iter().position(|b| b.line == line && name(&b.file) == name(&file)) else { return true };
        if let Some(cond) = self.breakpoints[i].condition.clone().filter(|c| !c.trim().is_empty()) {
            match self.evaluate(vm, module, Some(top), &cond) {
                Ok(v) if !v.to_bool() => return false,
                Ok(_) => {}
                Err(e) => {
                    self.note = Some(format!("The breakpoint's condition `{cond}` failed: {e}"));
                    return true;
                }
            }
        }
        self.breakpoints[i].hits += 1;
        let b = self.breakpoints[i].clone();
        if let Some(rule) = b.hit.as_deref().filter(|r| !r.trim().is_empty()) {
            if !hit_matches(rule, b.hits) {
                return false;
            }
        }
        if let Some(message) = b.log.as_deref().filter(|m| !m.is_empty()) {
            let text = interpolate(message, |expr| match self.evaluate(vm, module, Some(top), expr) {
                Ok(Value::String(s)) => s,
                Ok(v) => rapidr_value::format::print_text(&v),
                Err(e) => format!("<{e}>"),
            });
            output(Event::new(EventBody::Output { stream: "stdout".into(), text: text + "\n" }));
            return false;
        }
        true
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
                // (their rules, where they landed; a breakpoint whose rules
                // didn't change keeps its hits)
                let old: Vec<Placed> = self.breakpoints.iter().filter(|b| b.file.eq_ignore_ascii_case(&file)).cloned().collect();
                self.breakpoints.retain(|b| !b.file.eq_ignore_ascii_case(&file));
                for (b, at) in breakpoints.iter().zip(&placed) {
                    if let Some(at) = *at {
                        let mut p = Placed { file: file.clone(), line: at, condition: b.condition.clone(), hit: b.hit.clone(), log: b.log.clone(), hits: 0 };
                        if let Some(o) = old.iter().find(|o| o.same_rules(&p)) {
                            p.hits = o.hits;
                        }
                        self.breakpoints.push(p);
                    }
                }
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
            Command::Continue | Command::StepIn { .. } | Command::StepOver | Command::StepOut | Command::StepInTargets | Command::SetNextStatement { .. } if !paused => {
                (reply(not_stopped()), Control::None)
            }
            Command::Continue => (reply(ok()), Control::Continue),
            Command::StepIn { target: None } => (reply(ok()), Control::StepIn),
            Command::StepIn { target: Some(function) } => (reply(ok()), Control::StepInTarget(function)),
            Command::StepOver => (reply(ok()), Control::StepOver),
            Command::StepOut => (reply(ok()), Control::StepOut),
            Command::StepInTargets => {
                let targets = vm
                    .step_in_targets(module)
                    .into_iter()
                    .filter_map(|id| Some(StepInTarget { id, label: module.functions.get(id as usize)?.name.clone() }))
                    .collect();
                (reply(Event::reply(seq, EventBody::StepInTargets { targets })), Control::None)
            }
            Command::SetNextStatement { file, line } => {
                // (the line of `file` as compiled: the one in the function
                // stopped in, when the file is included twice)
                let compiled = if module.source_map.runs.is_empty() { vec![line] } else { module.source_map.compiled_lines(&file, line) };
                let mut answer = Err(format!("{file}: no line {line} in the program"));
                // (a line without code: the next line of the same file that has some)
                let candidates: Vec<u32> = if compiled.is_empty() { (1..=200).flat_map(|d| module.source_map.compiled_lines(&file, line + d)).take(1).collect() } else { compiled };
                for c in candidates {
                    answer = vm.set_next_statement(module, c);
                    if answer.is_ok() {
                        break;
                    }
                }
                match answer {
                    Ok(_) => {
                        self.goto_pending = true;
                        self.nodes.clear();
                        (reply(ok()), Control::Moved)
                    }
                    Err(e) => (reply(Event::error(seq, e)), Control::None),
                }
            }
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
                    ScopeInfo { name: "Components".into(), reference: COMPONENTS_REF },
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
                // (`clipboard`: the value whole, as text — copying it, the
                // text viewer — not as a debugger shows it)
                let whole = context.as_deref() == Some("clipboard");
                // (`hover`: a data tip's — what the mouse rests on never
                // runs the program's own SUBs, FUNCTIONs or methods)
                let hover = context.as_deref() == Some("hover");
                let text = expr.trim();
                let result = if repl && !text.starts_with('?') {
                    self.run(vm, module, frame, text, true).map(|_| Variable::default())
                } else {
                    let text = text.strip_prefix('?').unwrap_or(text).trim();
                    self.evaluate_with(vm, module, frame, text, hover).map(|v| {
                        if whole {
                            let kind = self.render(vm, &v, "", false).kind;
                            Variable { value: v.to_string_val(), kind, ..Default::default() }
                        } else {
                            self.render(vm, &v, text, true)
                        }
                    })
                };
                let event = match result {
                    Ok(r) => Event::reply(seq, EventBody::Evaluate { result: r.value, kind: r.kind, reference: r.reference, count: r.count, columns: r.columns }),
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
                        let r = self.render(vm, &v, &name, true);
                        Event::reply(seq, EventBody::Evaluate { result: r.value, kind: r.kind, reference: r.reference, count: r.count, columns: r.columns })
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
                let event = match component_properties(vm, &object) {
                    Some((kind, props)) => {
                        let properties = props
                            .into_iter()
                            .map(|(name, v)| {
                                // (a property's text is a value, not a component it names)
                                let evaluate_name = child_name(&object, &name);
                                Variable { name, ..self.render(vm, &v, &evaluate_name, false) }
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
        self.evaluate_with(vm, module, frame, expr, false)
    }

    /// [`Self::evaluate`]; `no_calls`: refused if it would call one of the
    /// program's SUBs, FUNCTIONs or a component's method (a data tip: what
    /// the mouse rests on must never run code — `F(x).Name` isn't shown).
    pub fn evaluate_with<H: Host + ?Sized>(&mut self, vm: &mut Vm<'_, H>, module: &Module, frame: Option<usize>, expr: &str, no_calls: bool) -> Result<Value, String> {
        let source = format!("{} = ({expr})", rapidr_bcgen::SNIPPET_RESULT);
        // (the compiler's words, without the line it made up around it)
        let snippet = compile(vm, module, frame, &source).map_err(|e| {
            if e.contains(rapidr_bcgen::SNIPPET_RESULT) {
                format!("`{expr}` isn't an expression (syntax error)")
            } else {
                e
            }
        })?;
        if no_calls && calls_code(&snippet.module, snippet.function) {
            return Err(format!("`{expr}` calls a SUB, FUNCTION or method: not shown by a data tip"));
        }
        let v = vm.evaluate(&snippet.module, snippet.function, frame, false, EVAL_FUEL).map_err(|e| e.to_string())?;
        // (a component's own name — `DIM df AS RDataFrame`, a form — is the
        // component, though no variable holds it)
        let name = expr.trim();
        if matches!(v, Value::Null) && !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') && vm.host.component_type(name).is_some() {
            return Ok(Value::String(name.to_string()));
        }
        Ok(v)
    }

    /// Runs BASIC statements in `frame` (what they assign to its locals is
    /// written back when `write_back`).
    pub fn run<H: Host + ?Sized>(&mut self, vm: &mut Vm<'_, H>, module: &Module, frame: Option<usize>, code: &str, write_back: bool) -> Result<(), String> {
        let snippet = compile(vm, module, frame, code)?;
        vm.evaluate(&snippet.module, snippet.function, frame, write_back, EVAL_FUEL).map(drop).map_err(|e| e.to_string())
    }

    fn variables<H: Host + ?Sized>(&mut self, vm: &mut Vm<'_, H>, module: &Module, reference: u32, start: u32, count: u32) -> Result<Vec<Variable>, String> {
        let (start, end) = (start as usize, start.saturating_add(count) as usize);
        // (name, value, the expression naming it, a component's property —
        // its text a value, never the component it may name)
        let mut named: Vec<(String, Value, String, bool)> = Vec::new();
        if reference == GLOBALS_REF {
            // (only the program's globals: a routine's STATICs and its own
            // undeclared variables, which the compiler keeps in global
            // slots, are its frame's)
            for (name, v) in vm.global_values(module) {
                if !hidden(name) && rapidr_bcgen::global_slot(module, name) == GlobalSlot::Global {
                    named.push((name.to_string(), v.clone(), name.to_string(), false));
                }
            }
            named.sort_by_key(|(n, ..)| n.to_ascii_lowercase());
        } else if reference == COMPONENTS_REF {
            // (the components no other holds, in the order they were made:
            // each shown as itself)
            for id in vm.host.component_children("") {
                let v = Value::String(id.clone());
                named.push((id.clone(), v, id, false));
            }
        } else if (LOCALS_REF..CHILDREN_REF).contains(&reference) {
            let i = (reference - LOCALS_REF) as usize;
            let frame = vm.frames.get(i).ok_or_else(|| format!("no frame {i}"))?;
            let names = module.functions.get(frame.fn_index as usize).map(|f| &f.local_names[..]).unwrap_or(&[]);
            for (slot, v) in frame.locals.iter().enumerate() {
                match names.get(slot) {
                    Some(name) if !hidden(name) => named.push((name.clone(), v.clone(), name.clone(), false)),
                    _ => {}
                }
            }
            // Its STATICs and its own undeclared variables, as it names them.
            let mut own: Vec<(String, Value, String, bool)> = vm
                .global_values(module)
                .filter_map(|(name, v)| match rapidr_bcgen::global_slot(module, name) {
                    GlobalSlot::Routine { function, name, .. } if function == frame.fn_index => Some((name.to_string(), v.clone(), name.to_string(), false)),
                    _ => None,
                })
                .collect();
            own.sort_by_key(|(n, ..)| n.to_ascii_lowercase());
            named.extend(own);
        } else if reference >= CHILDREN_REF {
            let node = self.nodes.get((reference - CHILDREN_REF) as usize).cloned().ok_or("that value is gone (the program went on)")?;
            match node {
                Node::Value(Value::Array(a), base) => {
                    let a = a.borrow();
                    for k in start..end.min(a.data.len()) {
                        let label = index_label(&a.bounds, k);
                        named.push((label.clone(), a.data[k].clone(), child_name(&base, &label), false));
                    }
                }
                Node::Value(Value::Object(o), base) => {
                    for (name, f) in o.names.iter().zip(o.fields.borrow().iter()) {
                        named.push((name.clone(), f.clone(), child_name(&base, name), false));
                    }
                }
                Node::Value(..) => {}
                Node::Component(id) => {
                    let (_, props) = component_properties(vm, &id).ok_or_else(|| format!("{id}: no such component"))?;
                    let parts = vm.host.component_children(&id);
                    let mut out: Vec<Variable> = Vec::new();
                    // (the components it holds first, as one item: a form's
                    // controls before its forty properties)
                    if !parts.is_empty() {
                        let reference = self.offer(Node::Parts(id.clone()));
                        out.push(Variable { name: "Components".into(), value: format!("({})", parts.len()), kind: "Components".into(), reference, count: parts.len() as u32, ..Default::default() });
                    }
                    for (name, v) in props {
                        let evaluate_name = child_name(&id, &name);
                        out.push(Variable { name, ..self.render(vm, &v, &evaluate_name, false) });
                    }
                    return Ok(out);
                }
                Node::Parts(id) => {
                    for part in vm.host.component_children(&id) {
                        let v = Value::String(part.clone());
                        named.push((part.clone(), v, part, false));
                    }
                }
                Node::Frame(id) => {
                    return Ok(rapidr_value::datascience::frame::with_frame(&id, |f| {
                        (start..end.min(f.height()))
                            .map(|r| {
                                let cells: Vec<String> = f.row(r).into_iter().map(|c| c.map(|c| c.into_owned()).unwrap_or_default()).collect();
                                (r, cells)
                            })
                            .collect::<Vec<_>>()
                    })
                    .into_iter()
                    .map(|(r, cells)| {
                        let reference = self.offer(Node::Row(id.clone(), r));
                        Variable { name: format!("({r})"), value: cells.join("  |  "), kind: "Row".into(), reference, count: cells.len() as u32, cells, ..Default::default() }
                    })
                    .collect());
                }
                Node::Row(id, r) => {
                    let cells = rapidr_value::datascience::frame::with_frame(&id, |f| {
                        (0..f.width()).map(|c| (f.columns[c].clone(), f.cell(r, c).map(|c| c.into_owned()), f.dtype(c).name())).collect::<Vec<_>>()
                    });
                    return Ok(cells
                        .into_iter()
                        .map(|(col, cell, dtype)| Variable {
                            evaluate_name: format!("{id}.At({r}, {})", basic_string(&col)),
                            value: match &cell {
                                Some(c) if dtype == "str" => quoted(c),
                                Some(c) => c.clone(),
                                None => "(null)".into(),
                            },
                            kind: dtype.to_string(),
                            name: col,
                            ..Default::default()
                        })
                        .collect());
                }
                Node::Num(id) => {
                    let values = rapidr_value::datascience::num::with_data(&id, |a| a.get(start..end.min(a.len())).map(<[f64]>::to_vec).unwrap_or_default());
                    return Ok(values
                        .into_iter()
                        .enumerate()
                        .map(|(k, x)| Variable {
                            name: format!("({})", start + k),
                            value: rapidr_value::datascience::num_text(x),
                            kind: "Double".into(),
                            evaluate_name: format!("{id}.Get({})", start + k),
                            ..Default::default()
                        })
                        .collect());
                }
            }
        } else {
            return Err(format!("no variables {reference}"));
        }
        Ok(named.into_iter().map(|(name, v, evaluate_name, property)| Variable { name, ..self.render(vm, &v, &evaluate_name, !property) }).collect())
    }

    /// A value as the debugger shows it (strings quoted, the first
    /// [`SHOWN_CHARS`] of a long one), its kind, a reference to its
    /// children (0: none) and how many; `evaluate_name` names it. With
    /// `components`, a string naming one of the program's components (a
    /// handler's `Sender`, a variable holding a form's name) is shown as
    /// that component: its type, its properties and parts as children — an
    /// RDataFrame as a table, an RNum as its numbers.
    fn render<H: Host + ?Sized>(&mut self, vm: &mut Vm<'_, H>, v: &Value, evaluate_name: &str, components: bool) -> Variable {
        let plain = |value: String, kind: &str| Variable { value, kind: kind.into(), evaluate_name: evaluate_name.to_string(), ..Default::default() };
        match v {
            Value::Null => plain(String::new(), "Empty"),
            Value::Integer(_) => plain(rapidr_value::format::print_text(v), "Integer"),
            Value::Double(_) => plain(rapidr_value::format::print_text(v), "Double"),
            Value::Boolean(b) => plain((if *b { "True" } else { "False" }).into(), "Boolean"),
            Value::String(s) => {
                if components {
                    if let Some(shown) = self.component_value(vm, s, evaluate_name) {
                        return shown;
                    }
                }
                plain(shown_string(s), "String")
            }
            Value::Array(a) => {
                let (bounds, n) = {
                    let a = a.borrow();
                    (a.bounds.iter().map(|(lo, hi)| format!("{lo} TO {hi}")).collect::<Vec<_>>().join(", "), a.data.len() as u32)
                };
                let reference = self.offer(Node::Value(v.clone(), evaluate_name.to_string()));
                Variable { value: format!("Array({bounds})"), kind: "Array".into(), reference, count: n, evaluate_name: evaluate_name.to_string(), ..Default::default() }
            }
            Value::Object(o) => {
                let reference = self.offer(Node::Value(v.clone(), evaluate_name.to_string()));
                Variable { value: o.type_name.clone(), kind: o.type_name.clone(), reference, count: o.names.len() as u32, evaluate_name: evaluate_name.to_string(), ..Default::default() }
            }
        }
    }

    /// A string naming one of the program's components, shown as it.
    fn component_value<H: Host + ?Sized>(&mut self, vm: &mut Vm<'_, H>, s: &str, evaluate_name: &str) -> Option<Variable> {
        let id = s.trim();
        if id.is_empty() || id.len() > 64 || !id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '(' | ')' | '.')) {
            return None;
        }
        let kind = vm.host.component_type(id)?;
        let comp = rapidr_lang::component(&kind);
        let shown = comp.map_or(kind.clone(), |c| c.spelling());
        let base = Variable { kind: shown.clone(), evaluate_name: evaluate_name.to_string(), ..Default::default() };
        Some(match comp.map(|c| c.name) {
            Some("RDATAFRAME") => {
                let (rows, columns) = rapidr_value::datascience::frame::with_frame(id, |f| (f.height(), f.columns.clone()));
                let reference = self.offer(Node::Frame(id.to_string()));
                let value = format!("{} ({shown}: {rows} {} × {} {})", quoted(id), plural(rows, "row"), columns.len(), plural(columns.len(), "column"));
                Variable { value, reference, count: rows as u32, columns, ..base }
            }
            Some("RNUM") => {
                let (n, first) = rapidr_value::datascience::num::with_data(id, |a| (a.len(), a.iter().take(6).map(|x| rapidr_value::datascience::num_text(*x)).collect::<Vec<_>>()));
                let reference = self.offer(Node::Num(id.to_string()));
                let more = if n > first.len() { ", …" } else { "" };
                Variable { value: format!("{} ({shown}({n}): [{}{more}])", quoted(id), first.join(", ")), reference, count: n as u32, ..base }
            }
            _ => {
                let reference = self.offer(Node::Component(id.to_string()));
                Variable { value: format!("{} ({shown})", quoted(id)), reference, ..base }
            }
        })
    }

    /// `node`'s reference, for its children.
    fn offer(&mut self, node: Node) -> u32 {
        self.nodes.push(node);
        CHILDREN_REF + (self.nodes.len() - 1) as u32
    }
}

/// "1 row", "3 rows".
fn plural(n: usize, what: &str) -> String {
    if n == 1 {
        what.to_string()
    } else {
        format!("{what}s")
    }
}

/// A component's type (its RapidR name) and its properties as the language
/// registry lists them, each read through the host as the program reads it
/// (`None`: not a component).
#[allow(clippy::type_complexity)]
fn component_properties<H: Host + ?Sized>(vm: &mut Vm<'_, H>, id: &str) -> Option<(String, Vec<(String, Value)>)> {
    let kind = vm.host.component_type(id)?;
    let comp = rapidr_lang::component(&kind);
    let shown = comp.map_or(kind.clone(), |c| c.spelling());
    let mut props = Vec::new();
    for p in comp.map(|c| c.properties).unwrap_or(&[]) {
        if p.indexed > 0 || p.missing || p.access == rapidr_lang::Access::Write {
            continue;
        }
        if let Ok(v) = vm.host.get_prop(id, p.name) {
            props.push((p.name.to_string(), v));
        }
    }
    props.sort_by_key(|(n, _)| n.to_ascii_lowercase());
    Some((shown, props))
}

/// Whether function `f` of `module` calls a SUB, a FUNCTION or a method.
fn calls_code(module: &Module, f: u32) -> bool {
    use rapidr_bytecode::Op;
    let Some(f) = module.functions.get(f as usize) else { return false };
    let mut ip = 0;
    while ip < f.code.len() {
        let Some(op) = Op::from_u8(f.code[ip]) else { return true };
        if matches!(op, Op::CallSub | Op::CallFunc | Op::CallIndirect | Op::CallMethod | Op::CallMethodDyn | Op::Input | Op::Gosub) {
            return true;
        }
        ip += 1 + op.operand_len();
    }
    false
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

/// A string as a value shows it: quoted, a long one cut after
/// [`SHOWN_CHARS`] characters with its length said.
fn shown_string(s: &str) -> String {
    match s.char_indices().nth(SHOWN_CHARS) {
        None => quoted(s),
        Some((cut, _)) => format!("{}… ({} characters)", quoted(&s[..cut]), s.chars().count()),
    }
}

/// `text` as a BASIC string literal.
fn basic_string(text: &str) -> String {
    format!("\"{}\"", text.replace('"', "\"\""))
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
    // (one line, so a message's "1:5: error:" says nothing worth keeping)
    let plain = |e: String| e.lines().map(|l| l.split_once(" error: ").map_or(l, |(_, m)| m).trim().to_string()).collect::<Vec<_>>().join("; ");
    let tokens = rapidr_lexer::Lexer::new(&text, None).tokenize().map_err(|e| plain(e.to_string()))?;
    let program = rapidr_parser::parse_tokens(&tokens).map_err(|e| plain(e.to_string()))?;
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
        // (a breakpoint whose condition, hit count or log says go on)
        let send = &self.send;
        if !self.end.at_breakpoint(vm, module, &mut |e| send(&e)) {
            self.end.resumed();
            return Resume::Continue;
        }
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
                Control::StepInTarget(function) => Resume::StepInTarget(function),
                Control::StepOver => Resume::StepOver,
                Control::StepOut => Resume::StepOut,
                Control::Stop => return Resume::Terminate,
                // (Set Next Statement: where it's stopped now)
                Control::Moved => {
                    let event = self.end.stopped_event(vm, module);
                    (self.send)(&event);
                    continue;
                }
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
                        Control::None | Control::Start { .. } | Control::Continue | Control::StepIn | Control::StepInTarget(_) | Control::StepOver | Control::StepOut | Control::Moved => {}
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
        assert_eq!(by_re(9), EventBody::Evaluate { result: String::new(), kind: String::new(), reference: 0, count: 0, columns: vec![] });
        assert!(sent.iter().any(|e| e.body == EventBody::Continued));
    }

    /// A SUB's STATIC and its own undeclared variable (RapidQ's implicit
    /// scope) live in global slots (`SUB Tick::hits`, `Tick__p`): they are
    /// the SUB frame's Locals under their source names, never Globals.
    #[test]
    fn a_routines_own_variables_are_its_frames_not_globals() {
        let src = "DIM Total AS INTEGER\nSUB Tick(n AS INTEGER)\n  STATIC hits AS INTEGER\n  hits = hits + 1\n  p = p + n\n  Total = Total + p\nEND SUB\nTick 1\nTick 2\nq = 5\nPRINT Total; q\n";
        let m = compile_program(src);
        let (tx, rx) = mpsc::channel();
        let sent: Rc<RefCell<Vec<Event>>> = Rc::default();
        let log = sent.clone();
        let mut dbg = BlockingDebugger::new(rx, Box::new(move |e| log.borrow_mut().push(e.clone())));
        let mut host = StubHost::default();
        let mut vm = Vm::new(&mut host);
        tx.send(req(1, Command::SetBreakpoints { file: "prog.bas".into(), breakpoints: vec![SourceBreakpoint { line: 6, ..Default::default() }] })).unwrap();
        tx.send(req(2, Command::Start { program: None, args: vec![], debug: true, stop_on_entry: false, break_on_error: false })).unwrap();
        assert_eq!(dbg.until_start(&mut vm, &m), Some(false));
        for (seq, c) in [
            // the first call's stop: go on
            (3, Command::Continue),
            // the second call's
            (4, Command::Variables { reference: LOCALS_REF + 1, start: None, count: None }),
            (5, Command::Variables { reference: GLOBALS_REF, start: None, count: None }),
            (6, Command::Evaluate { expr: "hits * 100 + p".into(), frame: None, context: None }),
            (7, Command::SetVariable { frame: None, name: "hits".into(), value: "hits + 40".into() }),
            (8, Command::Variables { reference: LOCALS_REF, start: None, count: None }),
            (9, Command::Continue),
        ] {
            tx.send(req(seq, c)).unwrap();
        }
        vm.debug_mode = true;
        vm.debugger = Some(Box::new(dbg));
        vm.run(&m).unwrap();
        let module_strings = m.strings.clone();
        drop(vm);
        assert_eq!(host.output.trim(), "45");
        let sent = sent.borrow();
        let by_re = |re: u64| sent.iter().find(|e| e.re == Some(re)).map(|e| e.body.clone()).unwrap();
        let names = |re: u64| {
            let EventBody::Variables { variables } = by_re(re) else { panic!("{:?}", by_re(re)) };
            variables.iter().map(|v| (v.name.clone(), v.value.clone())).collect::<Vec<_>>()
        };
        let pairs = |v: &[(&str, &str)]| v.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect::<Vec<_>>();
        // (the compiler does keep them as globals: the test means something)
        assert!(module_strings.iter().any(|s| s == "Tick__p") && module_strings.iter().any(|s| s == "SUB Tick::hits"), "{module_strings:?}");
        assert_eq!(names(4), pairs(&[("n", "2"), ("hits", "2"), ("p", "3")]));
        assert_eq!(names(5), pairs(&[("q", "0"), ("Total", "1")]));
        assert!(matches!(by_re(6), EventBody::Evaluate { ref result, .. } if result == "203"), "{:?}", by_re(6));
        assert!(matches!(by_re(7), EventBody::Evaluate { ref result, .. } if result == "42"), "{:?}", by_re(7));
        // (the main program's frame has none of them)
        assert!(names(8).iter().all(|(n, _)| n != "hits" && n != "p" && !n.contains("__") && !n.contains("::")), "{:?}", names(8));
    }

    #[test]
    fn hit_counts_and_messages() {
        assert!(hit_matches("3", 3) && !hit_matches("3", 4) && !hit_matches("= 3", 2));
        assert!(hit_matches(">= 2", 2) && hit_matches(">2", 3) && !hit_matches("> 2", 2));
        assert!(hit_matches("% 3", 6) && !hit_matches("%3", 5) && hit_matches("<= 1", 1) && !hit_matches("< 1", 1));
        assert!(hit_matches("whatever", 1));
        assert_eq!(interpolate("i = {i}, {{x}} {a + 1}", |e| format!("[{e}]")), "i = [i], {x} [a + 1]");
    }

    /// A condition, a hit count and a logpoint, decided where the program
    /// stops (desktop): the loop's line stops only when `i > 2` and only
    /// from its second such hit (i = 4); the logpoint prints every pass and
    /// never stops; a condition that can't be evaluated stops and says so.
    #[test]
    fn conditions_hit_counts_and_logpoints() {
        let src = "total = 0\nFOR i = 1 TO 5\n  total = total + i\n  x = i * 10\nNEXT\nzz = 1\nPRINT total\n";
        let m = compile_program(src);
        let (tx, rx) = mpsc::channel();
        let sent: Rc<RefCell<Vec<Event>>> = Rc::default();
        let log = sent.clone();
        let mut dbg = BlockingDebugger::new(rx, Box::new(move |e| log.borrow_mut().push(e.clone())));
        let mut host = StubHost::default();
        let mut vm = Vm::new(&mut host);
        let bps = vec![
            SourceBreakpoint { line: 3, condition: Some("i > 2".into()), hit: Some(">= 2".into()), log: None },
            SourceBreakpoint { line: 4, condition: None, hit: None, log: Some("pass {i}: {total}".into()) },
            SourceBreakpoint { line: 6, condition: Some("nosuch(".into()), hit: None, log: None },
        ];
        tx.send(req(1, Command::SetBreakpoints { file: "prog.bas".into(), breakpoints: bps })).unwrap();
        tx.send(req(2, Command::Start { program: None, args: vec![], debug: true, stop_on_entry: false, break_on_error: false })).unwrap();
        assert_eq!(dbg.until_start(&mut vm, &m), Some(false));
        for (seq, c) in [
            (3, Command::Evaluate { expr: "i".into(), frame: None, context: None }),
            (4, Command::Continue),
            (5, Command::Evaluate { expr: "i".into(), frame: None, context: None }),
            (6, Command::Continue),
            (7, Command::Continue),
        ] {
            tx.send(req(seq, c)).unwrap();
        }
        vm.debug_mode = true;
        vm.debugger = Some(Box::new(dbg));
        vm.run(&m).unwrap();
        drop(vm);
        assert_eq!(host.output.trim(), "15");
        let sent = sent.borrow();
        let by_re = |re: u64| sent.iter().find(|e| e.re == Some(re)).map(|e| e.body.clone()).unwrap();
        // (the first stop: i = 4 — the hits were i = 3 (1st) and i = 4 (2nd))
        assert!(matches!(by_re(3), EventBody::Evaluate { ref result, .. } if result == "4"), "{:?}", by_re(3));
        assert!(matches!(by_re(5), EventBody::Evaluate { ref result, .. } if result == "5"), "{:?}", by_re(5));
        let stops: Vec<(u32, Option<String>)> = sent
            .iter()
            .filter_map(|e| match &e.body {
                EventBody::Stopped { line, description, .. } if e.re.is_none() => Some((line.unwrap_or(0), description.clone())),
                _ => None,
            })
            .collect();
        assert_eq!(stops.len(), 3, "{stops:?}");
        assert_eq!((stops[0].0, stops[1].0, stops[2].0), (3, 3, 6));
        assert!(stops[2].1.as_deref().is_some_and(|d| d.contains("nosuch(")), "{stops:?}");
        let logs: String = sent
            .iter()
            .filter_map(|e| match &e.body {
                EventBody::Output { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(logs, "pass 1: 1\npass 2: 3\npass 3: 6\npass 4: 10\npass 5: 15\n");
    }

    /// A host whose ShowModal waits for its events: three turns of the
    /// window system (`pump`), then the form is closed (its result 1). The
    /// IDE's pause arrives during the first turn (the reader's interrupt).
    #[derive(Default)]
    struct ModalHost {
        inner: StubHost,
        started: bool,
        left: u32,
        pumps: u32,
        interrupt: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    }

    impl Host for ModalHost {
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
            if method.eq_ignore_ascii_case("showmodal") {
                self.started = true;
                self.left = 3;
                return Ok(Value::Null);
            }
            self.inner.call_method(id, method, args)
        }
        fn register_event(&mut self, id: &str, event: &str, handler_fn_index: u32) -> Result<(), String> {
            self.inner.register_event(id, event, handler_fn_index)
        }
        fn print(&mut self, s: &str) -> Result<(), String> {
            self.inner.print(s)
        }
        fn input(&mut self) -> Result<String, String> {
            self.inner.input()
        }
        fn wait_started(&mut self) -> bool {
            std::mem::take(&mut self.started)
        }
        fn pump(&mut self) -> Option<Value> {
            self.pumps += 1;
            if self.pumps == 1 {
                if let Some(flag) = &self.interrupt {
                    flag.store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }
            if self.left == 0 {
                return Some(Value::Integer(1));
            }
            self.left -= 1;
            None
        }
    }

    /// A pause while the program waits for its events (its ShowModal; no
    /// code runs): it stops at once at the line that waits, "Waiting for
    /// events", its variables there; Step Over goes on with the wait and
    /// stops at the line after it once the form is closed.
    #[test]
    fn a_pause_while_the_program_waits_stops_at_the_waiting_line() {
        let src = "DIM clicks AS INTEGER\nCREATE Main AS QFORM\nEND CREATE\nclicks = 5\nMain.ShowModal\nPRINT clicks\n";
        let m = compile_program(src);
        let (tx, rx) = mpsc::channel();
        let sent: Rc<RefCell<Vec<Event>>> = Rc::default();
        let log = sent.clone();
        // (the IDE's side: at each stop, what it asks)
        let ide = tx.clone();
        let stops = Rc::new(std::cell::Cell::new(0));
        let mut dbg = BlockingDebugger::new(
            rx,
            Box::new(move |e: &Event| {
                log.borrow_mut().push(e.clone());
                if matches!(e.body, EventBody::Stopped { .. }) && e.re.is_none() {
                    stops.set(stops.get() + 1);
                    let next = if stops.get() == 1 { Command::StepOver } else { Command::Continue };
                    ide.send(req(10 * stops.get(), Command::StackTrace)).unwrap();
                    ide.send(req(10 * stops.get() + 1, Command::Evaluate { expr: "clicks * 2".into(), frame: None, context: None })).unwrap();
                    ide.send(req(10 * stops.get() + 2, next)).unwrap();
                }
            }),
        );
        let flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let mut host = ModalHost { interrupt: Some(flag.clone()), ..ModalHost::default() };
        let mut vm = Vm::new(&mut host);
        vm.interrupt = flag;
        tx.send(req(1, Command::Start { program: None, args: vec![], debug: true, stop_on_entry: false, break_on_error: false })).unwrap();
        assert_eq!(dbg.until_start(&mut vm, &m), Some(false));
        tx.send(req(2, Command::Pause)).unwrap();
        vm.debugger = Some(Box::new(dbg));
        vm.run(&m).unwrap();
        drop(vm);
        assert_eq!(host.inner.output.trim(), "5");
        assert_eq!(host.pumps, 4, "the wait went on after the pause");
        let sent = sent.borrow();
        let by_re = |re: u64| sent.iter().find(|e| e.re == Some(re)).map(|e| e.body.clone()).unwrap();
        let stops: Vec<(String, u32, Option<String>)> = sent
            .iter()
            .filter_map(|e| match &e.body {
                EventBody::Stopped { reason, line, description, .. } if e.re.is_none() => Some((reason.clone(), line.unwrap_or(0), description.clone())),
                _ => None,
            })
            .collect();
        assert_eq!(stops, vec![("pause".to_string(), 5, Some(WAITING_NOTE.to_string())), ("step".to_string(), 6, None)]);
        let EventBody::StackTrace { frames } = by_re(10) else { panic!("{:?}", by_re(10)) };
        assert_eq!(frames.iter().map(|f| f.line).collect::<Vec<_>>(), vec![5]);
        assert!(matches!(by_re(11), EventBody::Evaluate { ref result, .. } if result == "10"), "{:?}", by_re(11));
        let EventBody::StackTrace { frames } = by_re(20) else { panic!("{:?}", by_re(20)) };
        assert_eq!(frames.iter().map(|f| f.line).collect::<Vec<_>>(), vec![6]);
    }

    /// Runs `src` (as prog.bas) under a [`BlockingDebugger`], breaking on
    /// errors, with breakpoints at `bps`; at each of its stops (but Set
    /// Next Statement's, whose requests were queued with the move) the IDE
    /// sends the next list of `stops` (the k-th list's requests numbered
    /// k·100 + i, from 1), the last of which should let it go on. Every
    /// event sent comes back.
    fn drive<H: Host>(src: &str, host: &mut H, bps: &[u32], stops: Vec<Vec<Command>>) -> Vec<Event> {
        let m = compile_program(src);
        let (tx, rx) = mpsc::channel();
        let sent: Rc<RefCell<Vec<Event>>> = Rc::default();
        let log = sent.clone();
        let ide = tx.clone();
        let queue: Rc<RefCell<std::collections::VecDeque<Vec<Command>>>> = Rc::new(RefCell::new(stops.into()));
        let count = Rc::new(std::cell::Cell::new(0u64));
        let mut dbg = BlockingDebugger::new(
            rx,
            Box::new(move |e: &Event| {
                log.borrow_mut().push(e.clone());
                if let EventBody::Stopped { reason, .. } = &e.body {
                    if e.re.is_none() && reason != "goto" {
                        count.set(count.get() + 1);
                        let list = queue.borrow_mut().pop_front().unwrap_or_else(|| vec![Command::Continue]);
                        for (i, c) in list.into_iter().enumerate() {
                            ide.send(req(count.get() * 100 + i as u64, c)).unwrap();
                        }
                    }
                }
            }),
        );
        let mut vm = Vm::new(host);
        let breakpoints = bps.iter().map(|&line| SourceBreakpoint { line, ..Default::default() }).collect();
        tx.send(req(1, Command::SetBreakpoints { file: "prog.bas".into(), breakpoints })).unwrap();
        tx.send(req(2, Command::Start { program: None, args: vec![], debug: true, stop_on_entry: false, break_on_error: true })).unwrap();
        assert_eq!(dbg.until_start(&mut vm, &m), Some(false));
        vm.debugger = Some(Box::new(dbg));
        let _ = vm.run(&m);
        drop(vm);
        let out = sent.borrow().clone();
        out
    }

    /// A breakpoint in a one-line event handler stops on EVERY event —
    /// also after a Step Over walked out of the handler (Robert, 2026-10-16:
    /// typing into an edit stopped once, then never again).
    #[test]
    fn a_breakpoint_in_a_one_line_handler_stops_on_every_event() {
        let src = "DIM n AS INTEGER\nSUB Changed\n    n = n + 1\nEND SUB\nn = 0\n";
        let m = compile_program(src);
        let handler = m.functions.iter().position(|f| f.name.eq_ignore_ascii_case("changed")).expect("the handler") as u32;
        let (tx, rx) = mpsc::channel();
        let sent: Rc<RefCell<Vec<Event>>> = Rc::default();
        let log = sent.clone();
        let ide = tx.clone();
        let count = Rc::new(std::cell::Cell::new(0u64));
        // (the first stop: Step Over twice — out of the handler, back to the
        // event loop; every later stop: Continue)
        let mut dbg = BlockingDebugger::new(
            rx,
            Box::new(move |e: &Event| {
                log.borrow_mut().push(e.clone());
                if let EventBody::Stopped { .. } = &e.body {
                    if e.re.is_none() {
                        count.set(count.get() + 1);
                        let next = if count.get() <= 2 { Command::StepOver } else { Command::Continue };
                        ide.send(req(count.get() * 100, next)).unwrap();
                    }
                }
            }),
        );
        let mut host = StubHost::default();
        let mut vm = Vm::new(&mut host);
        let breakpoints = vec![SourceBreakpoint { line: 3, ..Default::default() }];
        tx.send(req(1, Command::SetBreakpoints { file: "prog.bas".into(), breakpoints })).unwrap();
        tx.send(req(2, Command::Start { program: None, args: vec![], debug: true, stop_on_entry: false, break_on_error: true })).unwrap();
        assert_eq!(dbg.until_start(&mut vm, &m), Some(false));
        vm.debugger = Some(Box::new(dbg));
        vm.run(&m).unwrap();
        // (three key presses: three events of the same one-line handler)
        for _ in 0..3 {
            vm.invoke_function(&m, handler, vec![]).unwrap();
        }
        drop(vm);
        let stops: Vec<(String, u32)> = stops_of(&sent.borrow()).into_iter().filter(|(reason, _)| reason == "breakpoint").collect();
        assert_eq!(stops, vec![stop("breakpoint", 3); 3], "{:?}", sent.borrow());
    }

    fn stops_of(events: &[Event]) -> Vec<(String, u32)> {
        events
            .iter()
            .filter_map(|e| match &e.body {
                EventBody::Stopped { reason, line, .. } if e.re.is_none() => Some((reason.clone(), line.unwrap_or(0))),
                _ => None,
            })
            .collect()
    }

    fn reply_to(events: &[Event], re: u64) -> EventBody {
        events.iter().find(|e| e.re == Some(re)).map(|e| e.body.clone()).unwrap_or_else(|| panic!("no reply {re}: {events:?}"))
    }

    fn stop(reason: &str, line: u32) -> (String, u32) {
        (reason.to_string(), line)
    }

    /// Step Over a line that calls a SUB stops at the next line — not in
    /// the middle of the same line, where the call returns.
    #[test]
    fn step_over_a_call_goes_to_the_next_line() {
        let src = "SUB Twice(n)\n  PRINT n * 2\nEND SUB\nx = 1\nTwice x\nx = 2\nPRINT x\n";
        let mut host = StubHost::default();
        let events = drive(src, &mut host, &[5], vec![vec![Command::StepOver], vec![Command::Continue]]);
        assert_eq!(stops_of(&events), [stop("breakpoint", 5), stop("step", 6)]);
        assert_eq!(host.output, "2\n2\n");
    }

    /// Step Into Specific: the line's calls in the order they happen; into
    /// the one picked (the outer call, past the inner one).
    #[test]
    fn step_into_specific_goes_into_the_call_picked() {
        let src = "FUNCTION A(n)\n  Result = n + 1\nEND FUNCTION\nFUNCTION B(n)\n  Result = n * 2\nEND FUNCTION\nx = A(B(3))\nPRINT x\n";
        let m = compile_program(src);
        let a = m.functions.iter().position(|f| f.name.eq_ignore_ascii_case("A")).unwrap() as u32;
        let mut host = StubHost::default();
        let events = drive(
            src,
            &mut host,
            &[7],
            vec![vec![Command::StepInTargets, Command::StepIn { target: Some(a) }], vec![Command::Evaluate { expr: "n".into(), frame: None, context: None }, Command::Continue]],
        );
        let EventBody::StepInTargets { targets } = reply_to(&events, 100) else { panic!() };
        assert_eq!(targets.iter().map(|t| t.label.to_ascii_uppercase()).collect::<Vec<_>>(), ["B", "A"]);
        assert_eq!(stops_of(&events), [stop("breakpoint", 7), stop("step", 2)]);
        assert!(matches!(reply_to(&events, 200), EventBody::Evaluate { ref result, .. } if result == "6"), "{:?}", reply_to(&events, 200));
        assert_eq!(host.output.trim(), "7");
    }

    /// Set Next Statement skips a line (the program goes on from the line
    /// asked for), within the routine stopped in only.
    #[test]
    fn set_next_statement_skips_lines() {
        let src = "SUB Other\n  PRINT \"other\"\nEND SUB\nx = 1\nx = x + 10\nx = x + 100\nPRINT x\n";
        let mut host = StubHost::default();
        let events = drive(
            src,
            &mut host,
            &[5],
            vec![vec![
                Command::SetNextStatement { file: "prog.bas".into(), line: 2 },
                Command::SetNextStatement { file: "prog.bas".into(), line: 6 },
                Command::Evaluate { expr: "x".into(), frame: None, context: None },
                Command::Continue,
            ]],
        );
        assert!(matches!(reply_to(&events, 100), EventBody::Error { ref message } if message.to_ascii_uppercase().contains("OTHER")), "{:?}", reply_to(&events, 100));
        assert_eq!(reply_to(&events, 101), EventBody::Ok);
        assert_eq!(stops_of(&events), [stop("breakpoint", 5), stop("goto", 6)]);
        assert!(matches!(reply_to(&events, 102), EventBody::Evaluate { ref result, .. } if result == "1"), "{:?}", reply_to(&events, 102));
        assert_eq!(host.output.trim(), "101");
    }

    /// Stopped at a run-time error, the value fixed and the line run again
    /// (Set Next Statement to it): the program goes on instead of failing.
    #[test]
    fn set_next_statement_retries_a_line_that_failed() {
        let src = "z = 0\ny = 10 \\ z\nPRINT \"after\"; y\n";
        let mut host = StubHost::default();
        let events = drive(
            src,
            &mut host,
            &[],
            vec![vec![
                Command::SetVariable { frame: None, name: "z".into(), value: "2".into() },
                Command::SetNextStatement { file: "prog.bas".into(), line: 2 },
                Command::Continue,
            ]],
        );
        assert_eq!(stops_of(&events), [stop("exception", 2), stop("goto", 2)]);
        assert_eq!(host.output.trim(), "after5");
    }

    /// A host with an RDataFrame, an RNum and a form holding a button, as
    /// a runtime holds them (the shared data-science model).
    #[derive(Default)]
    struct PartsHost {
        inner: StubHost,
    }

    impl Host for PartsHost {
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
            Ok(match (id.to_ascii_lowercase().as_str(), name.to_ascii_lowercase().as_str()) {
                ("form1", "caption") => Value::String("Main".into()),
                ("button1", "caption") => Value::String("OK".into()),
                _ => Value::Null,
            })
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
        fn input(&mut self) -> Result<String, String> {
            self.inner.input()
        }
        fn component_type(&mut self, id: &str) -> Option<String> {
            match id.to_ascii_lowercase().as_str() {
                "sales" => Some("RDATAFRAME".into()),
                "temps" => Some("RNUM".into()),
                "form1" => Some("RFORM".into()),
                "button1" => Some("RBUTTON".into()),
                _ => None,
            }
        }
        fn component_children(&mut self, id: &str) -> Vec<String> {
            if id.eq_ignore_ascii_case("form1") {
                vec!["Button1".into()]
            } else {
                Vec::new()
            }
        }
    }

    /// What a stop shows opens like a tree: a frame as a table (its rows,
    /// each to its cells by column), an RNum's numbers (paged), a form's
    /// components then its properties, an array's elements; every value names
    /// itself.
    #[test]
    fn frames_numbers_and_forms_open_like_a_tree() {
        use rapidr_value::datascience::{frame, num};
        frame::set_frame("sales", frame::parse_csv("region,amount\nNorth,120\nSouth,80.5\n"));
        num::set_data("temps", (0..250).map(|i| i as f64 / 2.0).collect());
        let src = "\n\n\n\nDIM p(1 TO 2) AS INTEGER\nDIM df AS STRING, t AS STRING, f AS STRING\np(1) = 3\ndf = \"Sales\"\nt = \"Temps\"\nf = \"Form1\"\nPRINT \"done\"\n";
        let mut host = PartsHost::default();
        // (the globals' nodes are offered in their order: df, f, p, t)
        let events = drive(
            src,
            &mut host,
            &[11],
            vec![vec![
                Command::Variables { reference: GLOBALS_REF, start: None, count: None },
                Command::Variables { reference: CHILDREN_REF, start: None, count: None },
                Command::Variables { reference: CHILDREN_REF + 4, start: None, count: None },
                Command::Variables { reference: CHILDREN_REF + 1, start: None, count: None },
                Command::Variables { reference: CHILDREN_REF + 3, start: Some(200), count: Some(10) },
                Command::Variables { reference: CHILDREN_REF + 2, start: None, count: None },
                Command::Continue,
            ]],
        );
        let vars = |re: u64| {
            let EventBody::Variables { variables } = reply_to(&events, re) else { panic!("{:?}", reply_to(&events, re)) };
            variables
        };
        let globals = vars(100);
        let names: Vec<String> = globals.iter().map(|v| v.name.to_ascii_lowercase()).collect();
        assert_eq!(names, ["df", "f", "p", "t"]);
        let df = &globals[0];
        assert_eq!((df.kind.as_str(), df.count, df.columns.clone()), ("RDataFrame", 2, vec!["region".to_string(), "amount".to_string()]));
        assert!(df.evaluate_name.eq_ignore_ascii_case("df") && df.value.contains("2 rows × 2 columns"), "{df:?}");
        let t = &globals[3];
        assert_eq!((t.kind.as_str(), t.count), ("RNum", 250));
        assert!(t.value.contains("[0, 0.5, 1, 1.5, 2, 2.5, …]"), "{}", t.value);
        assert_eq!(globals[1].value, "\"Form1\" (RForm)");
        // the frame's rows, with their cells
        let rows = vars(101);
        assert_eq!(
            rows.iter().map(|r| (r.name.as_str(), r.cells.clone())).collect::<Vec<_>>(),
            [("(0)", vec!["North".to_string(), "120".to_string()]), ("(1)", vec!["South".to_string(), "80.5".to_string()])]
        );
        // a row's cells by column, each naming itself
        let cells = vars(102);
        assert_eq!(
            cells.iter().map(|c| (c.name.as_str(), c.value.as_str(), c.evaluate_name.as_str())).collect::<Vec<_>>(),
            [("region", "\"North\"", "Sales.At(0, \"region\")"), ("amount", "120", "Sales.At(0, \"amount\")")]
        );
        // the form: its components first, then its properties
        let form = vars(103);
        assert_eq!((form[0].name.as_str(), form[0].count), ("Components", 1));
        assert!(form.iter().any(|v| v.name == "Caption" && v.value == "\"Main\"" && v.evaluate_name == "Form1.Caption"), "{form:?}");
        // the RNum paged
        let page = vars(104);
        assert_eq!(page.len(), 10);
        assert_eq!((page[0].name.as_str(), page[0].value.as_str(), page[0].evaluate_name.as_str()), ("(200)", "100", "Temps.Get(200)"));
        // the array's elements
        let p = vars(105);
        assert_eq!(
            p.iter().map(|v| (v.name.clone(), v.value.clone(), v.evaluate_name.to_ascii_lowercase())).collect::<Vec<_>>(),
            [("(1)".to_string(), "3".to_string(), "p(1)".to_string()), ("(2)".to_string(), "0".to_string(), "p(2)".to_string())]
        );
    }

    /// Every opcode of real programs decodes with its operands' length
    /// (what Step Into Specific reads the code with) and the walk ends
    /// exactly at each function's end.
    #[test]
    fn operand_lengths_walk_compiled_code() {
        let src = "TYPE T\n  A AS INTEGER\nEND TYPE\nDIM o AS T\nDIM a(3) AS STRING\nSUB S(BYREF x AS INTEGER)\n  x = x + 1\nEND SUB\nFUNCTION F(n) AS INTEGER\n  SELECT CASE n\n    CASE 1: Result = 1\n    CASE ELSE: Result = F(n - 1) * n\n  END SELECT\nEND FUNCTION\nCREATE Frm AS QFORM\n  Caption = \"x\"\nEND CREATE\nDIM k AS INTEGER\nFOR k = 1 TO 3\n  a(k) = STR$(k) + \"!\"\n  S k\n  o.A = F(k)\nNEXT\nGOSUB there\nPRINT a(1); o.A, Frm.Caption\nEND\nthere:\nPRINT \"g\"\nRETURN\n";
        let m = compile_program(src);
        for f in &m.functions {
            let mut ip = 0;
            while ip < f.code.len() {
                let op = rapidr_bytecode::Op::from_u8(f.code[ip]).unwrap_or_else(|| panic!("{}: bad opcode at {ip}", f.name));
                ip += 1 + op.operand_len();
            }
            assert_eq!(ip, f.code.len(), "{}", f.name);
        }
    }

    /// The web's way (no blocking debugger: the VM returns at a stop and is
    /// continued later): stopped at a run-time error, the value fixed and
    /// the line set as the next statement, `resume` goes on from there.
    #[test]
    fn set_next_statement_after_an_error_without_a_blocking_debugger() {
        let src = "z = 0\ny = 10 \\ z\nPRINT \"after\"; y\n";
        let m = compile_program(src);
        let mut host = StubHost::default();
        let mut vm = Vm::new(&mut host);
        vm.debug_mode = true;
        vm.break_on_error = true;
        assert!(matches!(vm.run(&m), Err(rapidr_vm::VmError::Paused)));
        assert_eq!(vm.stop_reason, StopReason::Exception);
        let mut end = ProgramEnd::new();
        let (r, _) = end.handle(&mut vm, &m, req(1, Command::SetVariable { frame: None, name: "z".into(), value: "2".into() }), true);
        assert!(matches!(r.map(|e| e.body), Some(EventBody::Evaluate { ref result, .. }) if result == "2"));
        let (r, control) = end.handle(&mut vm, &m, req(2, Command::SetNextStatement { file: "prog.bas".into(), line: 2 }), true);
        assert_eq!((r.map(|e| e.body), control), (Some(EventBody::Ok), Control::Moved));
        let stopped = end.stopped_event(&vm, &m);
        assert!(matches!(stopped.body, EventBody::Stopped { ref reason, line: Some(2), .. } if reason == "goto"), "{stopped:?}");
        vm.resume(&m).unwrap();
        drop(vm);
        assert_eq!(host.output.trim(), "after5");
    }

    /// Hovering never runs code: an expression that calls a FUNCTION is
    /// refused in the `hover` context (and answered otherwise).
    #[test]
    fn a_hover_never_calls_a_function() {
        let src = "FUNCTION Bump(n)\n  Result = n + 1\nEND FUNCTION\nx = 1\nPRINT x\n";
        let m = compile_program(src);
        let mut host = StubHost::default();
        let mut vm = Vm::new(&mut host);
        vm.debug_mode = true;
        vm.set_file_breakpoints(&m, "prog.bas", &[5]);
        assert!(matches!(vm.run(&m), Err(rapidr_vm::VmError::Paused)));
        let mut end = ProgramEnd::new();
        let (r, _) = end.handle(&mut vm, &m, req(1, Command::Evaluate { expr: "Bump(x)".into(), frame: None, context: Some("hover".into()) }), true);
        assert!(matches!(r.map(|e| e.body), Some(EventBody::Error { ref message }) if message.contains("not shown by a data tip")));
        let (r, _) = end.handle(&mut vm, &m, req(2, Command::Evaluate { expr: "Bump(x)".into(), frame: None, context: None }), true);
        assert!(matches!(r.map(|e| e.body), Some(EventBody::Evaluate { ref result, .. }) if result == "2"));
        let (r, _) = end.handle(&mut vm, &m, req(3, Command::Evaluate { expr: "x".into(), frame: None, context: Some("hover".into()) }), true);
        assert!(matches!(r.map(|e| e.body), Some(EventBody::Evaluate { ref result, .. }) if result == "1"));
    }
}

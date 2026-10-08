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

use rapidr_bcgen::GlobalSlot;
use rapidr_bytecode::Module;
use rapidr_value::Value;
use rapidr_vm::{Debugger, Host, Resume, StopInfo, StopReason, Vm, EVAL_FUEL};

use crate::protocol::{Command, Event, EventBody, PlacedBreakpoint, Request, ScopeInfo, StackFrame, Variable};

pub use crate::protocol::{CHILDREN_REF, COMPONENT_REF, GLOBALS_REF, LOCALS_REF};
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
    /// Components offered in this stop (their properties: COMPONENT_REF).
    components: Vec<String>,
    /// The next stop is the entry's (`stopOnEntry`).
    entry_pending: bool,
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

impl ProgramEnd {
    pub fn new() -> Self {
        Self::default()
    }

    /// The program goes on: the references of the last stop are gone.
    pub fn resumed(&mut self) {
        self.children.clear();
        self.components.clear();
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
        let description = match vm.stop_reason {
            StopReason::Exception => vm.stop_error.clone(),
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
                    self.evaluate(vm, module, frame, text).map(|v| self.render_in(vm, &v))
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
                        let (result, kind, reference) = self.render_in(vm, &v);
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
                let event = match component_properties(vm, &object) {
                    Some((kind, props)) => {
                        let properties = props
                            .into_iter()
                            .map(|(name, v)| {
                                let (value, kind, reference) = self.render_in(vm, &v);
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
        // (the compiler's words, without the line it made up around it)
        let snippet = compile(vm, module, frame, &source).map_err(|e| {
            if e.contains(rapidr_bcgen::SNIPPET_RESULT) {
                format!("`{expr}` isn't an expression (syntax error)")
            } else {
                e
            }
        })?;
        vm.evaluate(&snippet.module, snippet.function, frame, false, EVAL_FUEL).map_err(|e| e.to_string())
    }

    /// Runs BASIC statements in `frame` (what they assign to its locals is
    /// written back when `write_back`).
    pub fn run<H: Host + ?Sized>(&mut self, vm: &mut Vm<'_, H>, module: &Module, frame: Option<usize>, code: &str, write_back: bool) -> Result<(), String> {
        let snippet = compile(vm, module, frame, code)?;
        vm.evaluate(&snippet.module, snippet.function, frame, write_back, EVAL_FUEL).map(drop).map_err(|e| e.to_string())
    }

    fn variables<H: Host + ?Sized>(&mut self, vm: &mut Vm<'_, H>, module: &Module, reference: u32, start: u32, count: u32) -> Result<Vec<Variable>, String> {
        let mut named: Vec<(String, Value)> = Vec::new();
        if reference == GLOBALS_REF {
            // (only the program's globals: a routine's STATICs and its own
            // undeclared variables, which the compiler keeps in global
            // slots, are its frame's)
            for (name, v) in vm.global_values(module) {
                if !hidden(name) && rapidr_bcgen::global_slot(module, name) == GlobalSlot::Global {
                    named.push((name.to_string(), v.clone()));
                }
            }
            named.sort_by_key(|(n, _)| n.to_ascii_lowercase());
        } else if (COMPONENT_REF..CHILDREN_REF).contains(&reference) {
            let id = self.components.get((reference - COMPONENT_REF) as usize).cloned().ok_or("that component is gone (the program went on)")?;
            let (_, props) = component_properties(vm, &id).ok_or_else(|| format!("{id}: no such component"))?;
            named = props;
        } else if (LOCALS_REF..COMPONENT_REF).contains(&reference) {
            let i = (reference - LOCALS_REF) as usize;
            let frame = vm.frames.get(i).ok_or_else(|| format!("no frame {i}"))?;
            let names = module.functions.get(frame.fn_index as usize).map(|f| &f.local_names[..]).unwrap_or(&[]);
            for (slot, v) in frame.locals.iter().enumerate() {
                match names.get(slot) {
                    Some(name) if !hidden(name) => named.push((name.clone(), v.clone())),
                    _ => {}
                }
            }
            // Its STATICs and its own undeclared variables, as it names them.
            let mut own: Vec<(String, Value)> = vm
                .global_values(module)
                .filter_map(|(name, v)| match rapidr_bcgen::global_slot(module, name) {
                    GlobalSlot::Routine { function, name, .. } if function == frame.fn_index => Some((name.to_string(), v.clone())),
                    _ => None,
                })
                .collect();
            own.sort_by_key(|(n, _)| n.to_ascii_lowercase());
            named.extend(own);
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
                let (value, kind, reference) = self.render_in(vm, &v);
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

    /// [`Self::render`], and a string naming one of the program's
    /// components (a handler's `Sender`) shown as that component: its type,
    /// its properties as children.
    fn render_in<H: Host + ?Sized>(&mut self, vm: &mut Vm<'_, H>, v: &Value) -> (String, String, u32) {
        if let Value::String(s) = v {
            let id = s.as_str();
            if !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '(' | ')' | '.')) {
                if let Some(kind) = vm.host.component_type(id) {
                    let shown = rapidr_lang::component(&kind).map_or(kind.clone(), |c| c.written_name().to_string());
                    let index = match self.components.iter().position(|c| c.eq_ignore_ascii_case(id)) {
                        Some(i) => i,
                        None => {
                            self.components.push(id.to_string());
                            self.components.len() - 1
                        }
                    };
                    return (format!("{} ({shown})", quoted(id)), shown, COMPONENT_REF + index as u32);
                }
            }
        }
        self.render(v)
    }

    fn child(&mut self, v: &Value) -> u32 {
        self.children.push(v.clone());
        CHILDREN_REF + (self.children.len() - 1) as u32
    }
}

/// A component's type (its Q name when it has one) and its properties as
/// the language registry lists them, each read through the host as the
/// program reads it (`None`: not a component).
#[allow(clippy::type_complexity)]
fn component_properties<H: Host + ?Sized>(vm: &mut Vm<'_, H>, id: &str) -> Option<(String, Vec<(String, Value)>)> {
    let kind = vm.host.component_type(id)?;
    let comp = rapidr_lang::component(&kind);
    let shown = comp.map_or(kind.clone(), |c| c.written_name().to_string());
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
}

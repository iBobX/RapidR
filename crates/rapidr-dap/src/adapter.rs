//! The adapter: DAP requests from the editor become session requests to
//! the program (a child process), and the program's replies and events
//! become DAP responses and events. One thread ("main", id 1).
//!
//! Everything arrives on one channel — the editor's messages (a reader
//! thread on our standard input) and the program's output (readers on its
//! standard output, deframed, and its standard error) — so the main loop
//! never blocks on either side: a request the program must answer is
//! remembered by its session `seq` and answered when the reply comes.

use std::collections::{BTreeMap, HashMap};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Stdio};
use std::sync::mpsc::{self, Sender};

use serde_json::{json, Value};

use crate::dap;
use crate::session_wire::{self as wire, Command, Deframer, Event as WireEvent, EventBody, Incoming, Request as WireRequest};

/// The one thread a BASIC program has.
const THREAD_ID: i64 = 1;

/// What the main loop hears.
enum Msg {
    /// A message from the editor; `None`: it's gone.
    Client(Option<Value>),
    /// What the program sent.
    Program(Incoming),
    /// One of the program's output streams ended.
    StreamEnded,
}

/// What a session reply completes.
enum Pending {
    Breakpoints { path: String },
    StackTrace { start: usize, levels: usize },
    Scopes { frame: u32 },
    Variables { reference: u32 },
    Evaluate { expr: String, frame: Option<u32> },
    SetVariable { path: String, frame: Option<u32> },
}

/// A DAP request waiting for the program's reply.
struct Waiting {
    seq: i64,
    command: String,
    pending: Pending,
}

struct Adapter<W: Write> {
    out: W,
    seq: i64,
    tx: Sender<Msg>,
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    next_wire_seq: u64,
    waiting: HashMap<u64, Waiting>,
    /// Breakpoints by the path the editor gave (resent if it comes before
    /// the program).
    breakpoints: BTreeMap<String, Vec<wire::SourceBreakpoint>>,
    /// Every file's path by its name (lower case): the session names files
    /// by name, the editor by path.
    paths: HashMap<String, PathBuf>,
    program_dir: Option<PathBuf>,
    launch: dap::LaunchArguments,
    break_on_error: bool,
    stopped: bool,
    /// Variable references handed out in this stop: the frame they belong
    /// to and the expression that names them (none for a scope).
    refs: HashMap<u32, (Option<u32>, Option<String>)>,
    streams_open: usize,
    exited: bool,
    terminated: bool,
    done: bool,
}

/// `rapidr dap`: serves one debug session on stdin / stdout.
pub fn run_stdio() -> std::process::ExitCode {
    let (tx, rx) = mpsc::channel();
    let client_tx = tx.clone();
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        let mut input = stdin.lock();
        loop {
            match dap::read_message(&mut input) {
                Ok(Some(message)) => {
                    if client_tx.send(Msg::Client(Some(message))).is_err() {
                        return;
                    }
                }
                Ok(None) | Err(_) => {
                    let _ = client_tx.send(Msg::Client(None));
                    return;
                }
            }
        }
    });
    let stdout = std::io::stdout();
    let mut adapter = Adapter::new(stdout.lock(), tx);
    while let Ok(msg) = rx.recv() {
        match msg {
            Msg::Client(Some(message)) => adapter.client(message),
            Msg::Client(None) => break,
            Msg::Program(item) => adapter.program(item),
            Msg::StreamEnded => adapter.stream_ended(),
        }
        if adapter.done {
            break;
        }
    }
    adapter.kill();
    std::process::ExitCode::SUCCESS
}

impl<W: Write> Adapter<W> {
    fn new(out: W, tx: Sender<Msg>) -> Self {
        Adapter {
            out,
            seq: 0,
            tx,
            child: None,
            stdin: None,
            next_wire_seq: 1,
            waiting: HashMap::new(),
            breakpoints: BTreeMap::new(),
            paths: HashMap::new(),
            program_dir: None,
            launch: dap::LaunchArguments::default(),
            break_on_error: false,
            stopped: false,
            refs: HashMap::new(),
            streams_open: 0,
            exited: false,
            terminated: false,
            done: false,
        }
    }

    // ------------------------------------------------ to the editor --

    fn next_seq(&mut self) -> i64 {
        self.seq += 1;
        self.seq
    }

    fn respond(&mut self, seq: i64, command: &str, body: Option<Value>) {
        let response = dap::Response { seq: self.next_seq(), kind: "response", request_seq: seq, success: true, command: command.to_string(), message: None, body };
        let _ = dap::write_message(&mut self.out, &response);
    }

    fn fail(&mut self, seq: i64, command: &str, message: impl Into<String>) {
        let message = message.into();
        let response = dap::Response {
            seq: self.next_seq(),
            kind: "response",
            request_seq: seq,
            success: false,
            command: command.to_string(),
            body: Some(json!({ "error": { "id": 1, "format": message, "showUser": false } })),
            message: Some(message),
        };
        let _ = dap::write_message(&mut self.out, &response);
    }

    fn event(&mut self, event: &str, body: Option<Value>) {
        let event = dap::Event { seq: self.next_seq(), kind: "event", event: event.to_string(), body };
        let _ = dap::write_message(&mut self.out, &event);
    }

    fn output(&mut self, category: &str, text: &str) {
        if !text.is_empty() {
            self.event("output", Some(json!({ "category": category, "output": text })));
        }
    }

    // ------------------------------------------------ to the program --

    /// Sends a session request; its reply completes `pending` (none: no
    /// reply wanted).
    fn send(&mut self, command: Command, pending: Option<(i64, &str, Pending)>) {
        let seq = match pending {
            Some((dap_seq, name, pending)) => {
                let seq = self.next_wire_seq;
                self.next_wire_seq += 1;
                self.waiting.insert(seq, Waiting { seq: dap_seq, command: name.to_string(), pending });
                seq
            }
            None => 0,
        };
        let mut line = WireRequest::new(seq, command).to_json();
        line.push('\n');
        let sent = self.stdin.as_mut().is_some_and(|stdin| stdin.write_all(line.as_bytes()).and_then(|_| stdin.flush()).is_ok());
        if !sent && seq != 0 {
            if let Some(w) = self.waiting.remove(&seq) {
                self.fail(w.seq, &w.command, "the program isn't running");
            }
        }
    }

    // ------------------------------------------------ files --

    /// A file's name as the session names it (its name, never its path).
    fn file_name(path: &str) -> String {
        path.rsplit(['/', '\\']).next().unwrap_or(path).to_string()
    }

    fn remember_path(&mut self, path: &Path) {
        // (as the editor wrote it, never resolved: /var/… and /private/var/…
        // are the same file to the system but two editors to VS Code)
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            self.paths.entry(name.to_ascii_lowercase()).or_insert_with(|| path.to_path_buf());
        }
    }

    /// The path of a file the session names: one the editor gave, one the
    /// program includes, or one in the program's folder.
    fn path_of(&self, name: &str) -> Option<PathBuf> {
        if let Some(p) = self.paths.get(&name.to_ascii_lowercase()) {
            return Some(p.clone());
        }
        let p = self.program_dir.as_ref()?.join(name);
        p.is_file().then_some(p)
    }

    fn source(&self, name: Option<&str>) -> Value {
        match name {
            Some(name) => match self.path_of(name) {
                Some(path) => json!({ "name": name, "path": path.to_string_lossy() }),
                None => json!({ "name": name }),
            },
            None => Value::Null,
        }
    }

    // ------------------------------------------------ the editor's requests --

    fn client(&mut self, message: Value) {
        let Ok(request) = serde_json::from_value::<dap::Request>(message) else { return };
        if request.kind != "request" {
            return;
        }
        let (seq, command) = (request.seq, request.command.as_str());
        let args = request.arguments.clone();
        match command {
            "initialize" => {
                let body = json!({
                    "supportsConfigurationDoneRequest": true,
                    "supportsEvaluateForHovers": true,
                    "supportsTerminateRequest": true,
                    "supportsSetVariable": true,
                    "exceptionBreakpointFilters": [
                        { "filter": "error", "label": "Run-time errors", "default": false }
                    ],
                });
                self.respond(seq, command, Some(body));
            }
            "launch" => match serde_json::from_value::<dap::LaunchArguments>(args) {
                Ok(launch) => match self.start_program(launch) {
                    Ok(()) => {
                        self.respond(seq, command, None);
                        // (configuration comes now: breakpoints, then
                        // configurationDone)
                        self.event("initialized", None);
                    }
                    Err(e) => self.fail(seq, command, e),
                },
                Err(e) => self.fail(seq, command, format!("launch: {e}")),
            },
            "setBreakpoints" => {
                let args: dap::SetBreakpointsArguments = serde_json::from_value(args).unwrap_or_default();
                let Some(path) = args.source.path.clone().or(args.source.name.clone()) else {
                    return self.fail(seq, command, "setBreakpoints: no source");
                };
                self.remember_path(Path::new(&path));
                let breakpoints: Vec<wire::SourceBreakpoint> = match (args.breakpoints, args.lines) {
                    (Some(b), _) => b
                        .into_iter()
                        .map(|b| wire::SourceBreakpoint { line: b.line, condition: b.condition, hit: b.hit_condition, log: b.log_message })
                        .collect(),
                    (None, Some(lines)) => lines.into_iter().map(|line| wire::SourceBreakpoint { line, ..Default::default() }).collect(),
                    (None, None) => Vec::new(),
                };
                self.breakpoints.insert(path.clone(), breakpoints.clone());
                if self.stdin.is_some() {
                    let file = Self::file_name(&path);
                    self.send(Command::SetBreakpoints { file, breakpoints }, Some((seq, command, Pending::Breakpoints { path })));
                } else {
                    let list: Vec<Value> = breakpoints.iter().map(|b| json!({ "verified": false, "line": b.line })).collect();
                    self.respond(seq, command, Some(json!({ "breakpoints": list })));
                }
            }
            "setExceptionBreakpoints" => {
                let args: dap::SetExceptionBreakpointsArguments = serde_json::from_value(args).unwrap_or_default();
                self.break_on_error = args.filters.iter().any(|f| f == "error");
                if self.stdin.is_some() {
                    self.send(Command::SetBreakOnError { enabled: self.break_on_error }, None);
                }
                self.respond(seq, command, None);
            }
            "configurationDone" => {
                let start = Command::Start {
                    program: None,
                    args: Vec::new(),
                    debug: !self.launch.no_debug,
                    stop_on_entry: self.launch.stop_on_entry && !self.launch.no_debug,
                    break_on_error: self.break_on_error && !self.launch.no_debug,
                };
                self.send(start, None);
                self.respond(seq, command, None);
            }
            "threads" => self.respond(seq, command, Some(json!({ "threads": [{ "id": THREAD_ID, "name": "main" }] }))),
            "stackTrace" => {
                if !self.stopped {
                    return self.respond(seq, command, Some(json!({ "stackFrames": [], "totalFrames": 0 })));
                }
                let args: dap::StackTraceArguments = serde_json::from_value(args).unwrap_or_default();
                let (start, levels) = (args.start_frame.unwrap_or(0), args.levels.filter(|&l| l > 0).unwrap_or(usize::MAX));
                self.send(Command::StackTrace, Some((seq, command, Pending::StackTrace { start, levels })));
            }
            "scopes" => match serde_json::from_value::<dap::ScopesArguments>(args) {
                Ok(a) if a.frame_id >= 1 => {
                    let frame = (a.frame_id - 1) as u32;
                    self.send(Command::Scopes { frame }, Some((seq, command, Pending::Scopes { frame })));
                }
                _ => self.fail(seq, command, "scopes: no such frame"),
            },
            "variables" => match serde_json::from_value::<dap::VariablesArguments>(args) {
                Ok(a) if a.variables_reference > 0 => {
                    let reference = a.variables_reference as u32;
                    self.send(Command::Variables { reference, start: a.start, count: a.count.filter(|&c| c > 0) }, Some((seq, command, Pending::Variables { reference })));
                }
                _ => self.respond(seq, command, Some(json!({ "variables": [] }))),
            },
            "evaluate" => {
                let Ok(a) = serde_json::from_value::<dap::EvaluateArguments>(args) else {
                    return self.fail(seq, command, "evaluate: no expression");
                };
                let frame = a.frame_id.filter(|&f| f >= 1).map(|f| (f - 1) as u32);
                if self.stopped {
                    let expr = a.expression.clone();
                    // (the debug console: an expression is printed, as VS
                    // Code's users expect; a statement runs — VB's Immediate)
                    let text = if a.context.as_deref() == Some("repl") { repl_text(&a.expression) } else { a.expression };
                    self.send(Command::Evaluate { expr: text, frame, context: a.context }, Some((seq, command, Pending::Evaluate { expr, frame })));
                } else if a.context.as_deref() == Some("repl") && self.stdin.is_some() {
                    // (while the program runs, the debug console is its
                    // keyboard: a line for INPUT)
                    self.send(Command::Input { text: Some(a.expression), form: None, event: None }, None);
                    self.respond(seq, command, Some(json!({ "result": "", "variablesReference": 0 })));
                } else {
                    self.fail(seq, command, "the program isn't stopped");
                }
            }
            "setVariable" => {
                let Ok(a) = serde_json::from_value::<dap::SetVariableArguments>(args) else {
                    return self.fail(seq, command, "setVariable: bad arguments");
                };
                let Some((frame, parent)) = self.refs.get(&(a.variables_reference as u32)).cloned() else {
                    return self.fail(seq, command, "that variable is gone (the program went on)");
                };
                let path = child_path(parent.as_deref(), &a.name);
                self.send(Command::SetVariable { frame, name: path.clone(), value: a.value }, Some((seq, command, Pending::SetVariable { path, frame })));
            }
            "continue" | "next" | "stepIn" | "stepOut" => {
                let go = match command {
                    "continue" => Command::Continue,
                    "next" => Command::StepOver,
                    "stepIn" => Command::StepIn,
                    _ => Command::StepOut,
                };
                self.stopped = false;
                self.refs.clear();
                self.send(go, None);
                let body = (command == "continue").then(|| json!({ "allThreadsContinued": true }));
                self.respond(seq, command, body);
            }
            "pause" => {
                self.send(Command::Pause, None);
                self.respond(seq, command, None);
            }
            "terminate" => {
                self.send(Command::Stop, None);
                self.respond(seq, command, None);
                let code = self.kill();
                self.finish(code);
            }
            "disconnect" => {
                self.send(Command::Stop, None);
                self.kill();
                self.respond(seq, command, None);
                self.done = true;
            }
            _ => self.fail(seq, command, format!("{command}: not supported by rapidr dap")),
        }
    }

    /// `launch`: runs the program's end of the session.
    fn start_program(&mut self, launch: dap::LaunchArguments) -> Result<(), String> {
        if self.child.is_some() {
            return Err("a program is running already".into());
        }
        let program = launch.program.clone().filter(|p| !p.is_empty()).ok_or("launch: no `program` (the .bas / .rr file to debug)")?;
        let base = match &launch.cwd {
            Some(cwd) if !cwd.is_empty() => PathBuf::from(cwd),
            _ => std::env::current_dir().map_err(|e| e.to_string())?,
        };
        let program = base.join(&program);
        // (a launch names a RapidR program, never another file: SEC-18)
        crate::confine::check_program(&program)?;
        self.program_dir = program.parent().map(Path::to_path_buf);
        self.remember_path(&program);
        // (the files it includes, as the preprocessor finds them)
        if let Ok(pre) = rapidr_preprocessor::preprocess_file(&program, rapidr_preprocessor::PreprocessOptions::default()) {
            for file in pre.line_map.iter().filter_map(|(f, _)| f.as_deref()) {
                self.remember_path(file);
            }
        }
        // (the program's folder unless asked otherwise: a RapidQ program
        // reads its files from there)
        let cwd = match &launch.cwd {
            Some(cwd) if !cwd.is_empty() => PathBuf::from(cwd),
            _ => self.program_dir.clone().unwrap_or(base),
        };
        let mut command = crate::program_end_command(&program, &launch.args).map_err(|e| format!("rapidr: {e}"))?;
        command.current_dir(&cwd).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
        // (the program's environment as the launch asks, but never what
        // makes a process load other code: SEC-18, confine::env_allowed)
        let (env, refused) = crate::confine::debuggee_env(&launch.env);
        for (name, value) in &env {
            match value {
                Some(v) => command.env(name, v),
                None => command.env_remove(name),
            };
        }
        if !refused.is_empty() {
            self.output("console", &format!("rapidr dap: not passed to the program (they change how a process loads code, or aren't variable names): {}\n", refused.join(", ")));
        }
        let mut child = command.spawn().map_err(|e| format!("{}: {e}", program.display()))?;
        let stdout = child.stdout.take().expect("piped stdout");
        let stderr = child.stderr.take().expect("piped stderr");
        spawn_stdout_reader(stdout, self.tx.clone());
        spawn_stderr_reader(stderr, self.tx.clone());
        self.streams_open = 2;
        self.stdin = child.stdin.take();
        self.child = Some(child);
        self.launch = launch;
        // (breakpoints the editor set before the program came)
        let early: Vec<(String, Vec<wire::SourceBreakpoint>)> = std::mem::take(&mut self.breakpoints).into_iter().collect();
        for (path, breakpoints) in early {
            self.send(Command::SetBreakpoints { file: Self::file_name(&path), breakpoints: breakpoints.clone() }, None);
            self.breakpoints.insert(path, breakpoints);
        }
        Ok(())
    }

    // ------------------------------------------------ the program's messages --

    fn program(&mut self, item: Incoming) {
        match item {
            Incoming::Text(text) | Incoming::Garbled(text) => self.output("stdout", &text),
            Incoming::Event(WireEvent { re: Some(re), body }) => self.reply(re, body),
            Incoming::Event(WireEvent { re: None, body }) => match body {
                EventBody::Output { stream, text } => {
                    let category = if stream == "stderr" { "stderr" } else { "stdout" };
                    self.output(category, &text);
                }
                EventBody::Stopped { reason, description, .. } => {
                    self.stopped = true;
                    self.refs.clear();
                    let mut body = json!({ "reason": reason, "threadId": THREAD_ID, "allThreadsStopped": true });
                    if let Some(d) = description {
                        body["description"] = json!(d);
                        body["text"] = json!(d);
                    }
                    self.event("stopped", Some(body));
                }
                EventBody::Exited { code } => self.finish(Some(code)),
                // (the program says so itself when it goes on after a
                // request; the editor knows)
                EventBody::Continued => {}
                EventBody::Error { message } => self.output("console", &format!("rapidr dap: {message}\n")),
                _ => {}
            },
        }
    }

    /// A session reply: the DAP response it completes.
    fn reply(&mut self, re: u64, body: EventBody) {
        let Some(Waiting { seq, command, pending }) = self.waiting.remove(&re) else { return };
        if let EventBody::Error { message } = body {
            return self.fail(seq, &command, message);
        }
        match (pending, body) {
            (Pending::Breakpoints { path }, EventBody::Breakpoints { breakpoints, .. }) => {
                let source = json!({ "name": Self::file_name(&path), "path": path });
                let list: Vec<Value> = breakpoints
                    .iter()
                    .map(|b| {
                        let mut v = json!({ "verified": b.verified, "line": b.actual_line.unwrap_or(b.line), "source": source });
                        if !b.verified {
                            v["message"] = json!("no code on this line or after it in this file");
                        }
                        v
                    })
                    .collect();
                self.respond(seq, &command, Some(json!({ "breakpoints": list })));
            }
            (Pending::StackTrace { start, levels }, EventBody::StackTrace { frames }) => {
                let total = frames.len();
                let list: Vec<Value> = frames
                    .iter()
                    .skip(start)
                    .take(levels)
                    .map(|f| {
                        let name = if f.name == "__main" { f.file.clone().unwrap_or_else(|| "(main program)".into()) } else { f.name.clone() };
                        let mut v = json!({ "id": f.id as i64 + 1, "name": name, "line": f.line, "column": 1 });
                        let source = self.source(f.file.as_deref());
                        if !source.is_null() {
                            v["source"] = source;
                        }
                        v
                    })
                    .collect();
                self.respond(seq, &command, Some(json!({ "stackFrames": list, "totalFrames": total })));
            }
            (Pending::Scopes { frame }, EventBody::Scopes { scopes }) => {
                let list: Vec<Value> = scopes
                    .iter()
                    .map(|s| {
                        let locals = s.name.eq_ignore_ascii_case("locals");
                        self.refs.insert(s.reference, (if locals { Some(frame) } else { None }, None));
                        let mut v = json!({ "name": s.name, "variablesReference": s.reference, "expensive": false });
                        if locals {
                            v["presentationHint"] = json!("locals");
                        }
                        v
                    })
                    .collect();
                self.respond(seq, &command, Some(json!({ "scopes": list })));
            }
            (Pending::Variables { reference }, EventBody::Variables { variables }) => {
                let (frame, parent) = self.refs.get(&reference).cloned().unwrap_or((None, None));
                let list: Vec<Value> = variables
                    .iter()
                    .map(|var| {
                        let path = child_path(parent.as_deref(), &var.name);
                        let mut v = json!({ "name": var.name, "value": shown(&var.value, &var.kind), "type": var.kind, "variablesReference": var.reference, "evaluateName": path });
                        if var.reference != 0 {
                            self.refs.insert(var.reference, (frame, Some(path)));
                            if var.kind == "Array" {
                                v["indexedVariables"] = json!(var.count);
                            } else {
                                v["namedVariables"] = json!(var.count);
                            }
                        }
                        v
                    })
                    .collect();
                self.respond(seq, &command, Some(json!({ "variables": list })));
            }
            (Pending::Evaluate { expr, frame }, EventBody::Evaluate { result, kind, reference }) => {
                if reference != 0 {
                    self.refs.insert(reference, (frame, Some(expr.trim().trim_start_matches('?').trim().to_string())));
                }
                self.respond(seq, &command, Some(json!({ "result": shown(&result, &kind), "type": kind, "variablesReference": reference })));
            }
            (Pending::SetVariable { path, frame }, EventBody::Evaluate { result, kind, reference }) => {
                if reference != 0 {
                    self.refs.insert(reference, (frame, Some(path)));
                }
                self.respond(seq, &command, Some(json!({ "value": result, "type": kind, "variablesReference": reference })));
            }
            (_, EventBody::Ok) => self.respond(seq, &command, None),
            (_, other) => self.fail(seq, &command, format!("unexpected reply from the program: {}", WireEvent::new(other).to_json())),
        }
    }

    fn stream_ended(&mut self) {
        self.streams_open = self.streams_open.saturating_sub(1);
        if self.streams_open > 0 {
            return;
        }
        // (the program ended without saying so: a kill, END's exit, a crash)
        let code = self.child.as_mut().and_then(|c| c.wait().ok()).map(|s| s.code().unwrap_or(-1));
        self.finish(code);
    }

    /// The program has ended: `exited` (when its code is known) and
    /// `terminated`, once each; requests still waiting fail.
    fn finish(&mut self, code: Option<i32>) {
        let waiting: Vec<Waiting> = self.waiting.drain().map(|(_, w)| w).collect();
        for w in waiting {
            self.fail(w.seq, &w.command, "the program has ended");
        }
        self.stopped = false;
        if !self.exited && !self.terminated {
            if let Some(code) = code {
                self.exited = true;
                self.event("exited", Some(json!({ "exitCode": code })));
            }
        }
        if !self.terminated {
            self.terminated = true;
            self.event("terminated", None);
        }
    }

    /// Ends the program at once; its exit code.
    fn kill(&mut self) -> Option<i32> {
        self.stdin = None;
        let child = self.child.as_mut()?;
        if child.try_wait().ok().flatten().is_none() {
            let _ = child.kill();
        }
        child.wait().ok().map(|s| s.code().unwrap_or(-1))
    }
}

/// The expression naming a child of `parent`: an element `a(1)`, a field
/// `p.Name` (a scope's variable: just its name).
/// A value as the editor shows it: a variable never assigned is `Empty`
/// (the session sends no text for it).
fn shown(value: &str, kind: &str) -> String {
    if value.is_empty() && kind == "Empty" {
        "Empty".to_string()
    } else {
        value.to_string()
    }
}

/// What the debug console's text is for the session's `repl` evaluation:
/// a statement as typed (an assignment, `PRINT …`, `CALL …`, `? …`), any
/// other text as an expression to print (`? text`).
fn repl_text(text: &str) -> String {
    let t = text.trim();
    let upper = t.to_ascii_uppercase();
    let first = upper.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '$')).next().unwrap_or("");
    const STATEMENTS: &[&str] = &[
        "PRINT", "CALL", "DIM", "REDIM", "INC", "DEC", "SWAP", "IF", "FOR", "WHILE", "DO", "SELECT", "INPUT", "LET", "GOSUB", "GOTO", "END", "EXIT", "WITH",
        "CREATE", "BIND", "CLS", "LOCATE", "COLOR", "SHOWMESSAGE",
    ];
    if t.starts_with('?') || STATEMENTS.contains(&first) || is_assignment(t) {
        t.to_string()
    } else {
        format!("? {t}")
    }
}

/// `name … = value`: a name (members, indexes) then `=` at the top level.
fn is_assignment(t: &str) -> bool {
    let mut depth = 0;
    let mut in_string = false;
    let mut seen_name = false;
    for c in t.chars() {
        match c {
            '"' => in_string = !in_string,
            _ if in_string => {}
            '(' => depth += 1,
            ')' => depth -= 1,
            '=' if depth == 0 => return seen_name,
            c if depth == 0 && (c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '$' | '%' | '&' | '!' | '#')) => seen_name = true,
            c if depth == 0 && c.is_whitespace() => {}
            _ if depth == 0 => return false,
            _ => {}
        }
    }
    false
}

fn child_path(parent: Option<&str>, name: &str) -> String {
    match parent {
        None => name.to_string(),
        Some(p) if name.starts_with('(') => format!("{p}{name}"),
        Some(p) => format!("{p}.{name}"),
    }
}

/// The program's standard output: its text and the session's events.
fn spawn_stdout_reader(mut stdout: impl Read + Send + 'static, tx: Sender<Msg>) {
    std::thread::spawn(move || {
        let mut deframer = Deframer::new();
        let mut buf = [0u8; 8192];
        loop {
            match stdout.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    for item in deframer.push(&buf[..n]) {
                        if tx.send(Msg::Program(item)).is_err() {
                            return;
                        }
                    }
                }
            }
        }
        if let Some(rest) = deframer.finish() {
            let _ = tx.send(Msg::Program(rest));
        }
        let _ = tx.send(Msg::StreamEnded);
    });
}

/// The program's standard error: output (whole characters only).
fn spawn_stderr_reader(mut stderr: impl Read + Send + 'static, tx: Sender<Msg>) {
    std::thread::spawn(move || {
        let mut buf = [0u8; 8192];
        let mut pending: Vec<u8> = Vec::new();
        loop {
            match stderr.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    pending.extend_from_slice(&buf[..n]);
                    let cut = match std::str::from_utf8(&pending) {
                        Ok(_) => pending.len(),
                        Err(e) if e.error_len().is_none() => e.valid_up_to(),
                        Err(_) => pending.len(),
                    };
                    let text = String::from_utf8_lossy(&pending[..cut]).into_owned();
                    pending.drain(..cut);
                    let event = WireEvent::new(EventBody::Output { stream: "stderr".into(), text });
                    if tx.send(Msg::Program(Incoming::Event(event))).is_err() {
                        return;
                    }
                }
            }
        }
        if !pending.is_empty() {
            let text = String::from_utf8_lossy(&pending).into_owned();
            let _ = tx.send(Msg::Program(Incoming::Event(WireEvent::new(EventBody::Output { stream: "stderr".into(), text }))));
        }
        let _ = tx.send(Msg::StreamEnded);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_console_prints_expressions_and_runs_statements() {
        assert_eq!(repl_text("a + 1"), "? a + 1");
        assert_eq!(repl_text("total = 100"), "total = 100");
        assert_eq!(repl_text("v(i).x = 2"), "v(i).x = 2");
        assert_eq!(repl_text("a = 1 OR b"), "a = 1 OR b");
        assert_eq!(repl_text("(a = 1)"), "? (a = 1)");
        assert_eq!(repl_text("x > 1 = y"), "? x > 1 = y");
        assert_eq!(repl_text("? a"), "? a");
        assert_eq!(repl_text("PRINT a"), "PRINT a");
        assert_eq!(repl_text("Mean(5)"), "? Mean(5)");
    }

    #[test]
    fn children_are_named_as_basic_writes_them() {
        assert_eq!(child_path(None, "total"), "total");
        assert_eq!(child_path(Some("a"), "(1, 2)"), "a(1, 2)");
        assert_eq!(child_path(Some("list(3)"), "Name"), "list(3).Name");
    }

    #[test]
    fn the_adapter_answers_initialize_and_refuses_what_it_does_not_know() {
        let (tx, _rx) = mpsc::channel();
        let mut a = Adapter::new(Vec::new(), tx);
        a.client(json!({"seq": 1, "type": "request", "command": "initialize", "arguments": {"adapterID": "rapidr"}}));
        a.client(json!({"seq": 2, "type": "request", "command": "fly"}));
        a.client(json!({"seq": 3, "type": "request", "command": "stackTrace", "arguments": {"threadId": 1}}));
        let mut reader = std::io::BufReader::new(&a.out[..]);
        let init = dap::read_message(&mut reader).unwrap().unwrap();
        assert_eq!(init["success"], true);
        assert_eq!(init["body"]["supportsConfigurationDoneRequest"], true);
        assert_eq!(init["body"]["exceptionBreakpointFilters"][0]["filter"], "error");
        let fly = dap::read_message(&mut reader).unwrap().unwrap();
        assert_eq!((fly["request_seq"].as_i64(), fly["success"].as_bool()), (Some(2), Some(false)));
        let st = dap::read_message(&mut reader).unwrap().unwrap();
        assert_eq!(st["body"]["totalFrames"], 0);
    }
}

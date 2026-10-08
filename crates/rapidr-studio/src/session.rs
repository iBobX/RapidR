//! RPROGRAMSESSION: a run of the program under development
//! (docs/ide-components.md §3.9), `rapidr-session`'s client as a component.
//! The program runs where the runtime's [`Host::launch`] puts it — its own
//! process on the desktop (`rapidr run --session`: its forms are real
//! windows; Stop kills it), a sandboxed frame on the web (its forms kernel
//! windows there) — and speaks the session protocol.
//!
//! What the debugger shows is asked of the program without waiting (the web
//! can't wait for its frame), the same way on both hosts: at a stop the
//! session fetches the call stack, the selected frame's locals, the globals
//! and the watches, and fires OnStopped once they're in; the program then
//! reads them at once (`StackTrace`, `Variables`, `WatchValues`). Anything
//! more — an object's children, an expression — comes back as an event.
//!
//! | Member | |
//! |---|---|
//! | `Program`, `Args`, `Debug`, `BreakOnError` | what to run (a source file), its arguments (one string, as COMMAND$ reads them), under the debugger (default True), stop at a run-time error |
//! | `Theme` | the look a program that names none is drawn in (`$THEME`'s names; "" its default, RapidR's look): Studio's "Preview in classic" sets `classic` |
//! | `State` | `stopped`, `running`, `paused` |
//! | `CurrentFile`, `CurrentLine`, `ExitCode`, `Error` | where it is paused; its last exit code; why Start failed |
//! | `StopReason`, `StopMessage` | why it paused (`breakpoint`, `step`, `pause`, `entry`, `exception`); a run-time error's message |
//! | `Frame` | the call stack's selected frame (0: the innermost); setting it fetches its locals and the watches again (then OnVariables(0)) |
//! | `LocalsRef`, `GlobalsRef` | the selected frame's locals' and the globals' references, for `Variables` |
//! | `Watches` | the watch expressions, one a line; evaluated at every stop |
//! | `Start` → True / False, `Stop`, `Pause`, `Continue`, `StepIn`, `StepOver`, `StepOut` | |
//! | `SetBreakpoint(File, Line [, Condition [, HitCount [, LogMessage]]])`, `ClearBreakpoint(File, Line)`, `ClearBreakpoints([File])` | lines from 1 |
//! | `RunToCursor(File, Line)` | on to that line (a stopped program starts) |
//! | `StackTrace`, `Variables(Ref)`, `WatchValues` → text | what the last stop fetched: one item a line, its fields tab-separated |
//! | `Expand(Ref)` | fetches a value's children (an array's elements, an object's fields, a component's properties): OnVariables(Ref) |
//! | `Evaluate(Expr [, Context])` → Id | in the selected frame: OnEvaluate(Id, Result). Context `repl` (the Immediate window): `? x` prints, a statement runs |
//! | `SetVariable(Name, Value)` → Id | a variable of the selected frame set to an expression's value: OnEvaluate(Id, NewValue), then the variables again (OnVariables(0)) |
//! | `Input(Text)` | a line for the program's INPUT |
//! | `OnOutput(Text)`, `OnStopped(Reason, File, Line)`, `OnContinue`, `OnExit(Code)`, `OnFormShown(Id)`, `OnVariables(Ref)`, `OnEvaluate(Id, Result)` | |

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::time::Duration;

use rapidr_session::protocol::{SourceBreakpoint, StackFrame, Variable, GLOBALS_REF, LOCALS_REF};
use rapidr_session::{Command, EventBody, ProgramSession, SessionEvent, State};
use rapidr_value::Value;

use crate::{flag, int_arg, text_arg, Host};

/// What a request in flight is for.
#[derive(Debug, Clone, PartialEq)]
enum Want {
    /// The call stack (a stop's first request).
    Stack,
    /// A scope's or a value's children, for the stop's snapshot.
    Vars(u32),
    /// Watch `i`'s value, for the snapshot.
    Watch(usize),
    /// `Expand(Ref)`'s children: OnVariables(Ref).
    Expand(u32),
    /// `Evaluate` / `SetVariable`'s result: OnEvaluate(Id, …); `refresh`:
    /// the variables are fetched again after it.
    Eval { id: i64, refresh: bool },
}

#[derive(Default)]
struct Model {
    session: ProgramSession,
    args: String,
    error: String,
    /// The watch expressions.
    watches: Vec<String>,
    /// The last stop's call stack (innermost first) and the selected frame.
    frames: Vec<StackFrame>,
    frame: usize,
    /// Variables fetched since the stop, by reference.
    vars: BTreeMap<u32, Vec<Variable>>,
    /// Each watch's value, kind and reference (its children).
    watch_values: Vec<(String, String, u32)>,
    /// Requests in flight.
    pending: BTreeMap<u64, Want>,
    /// A stop whose snapshot is still arriving: OnStopped waits for it.
    held: Option<(String, String, i64)>,
    /// A frame or watch refresh in flight: OnVariables(0) when done.
    refreshing: bool,
    next_eval: i64,
    /// The look a program that names none is drawn in ($THEME's names).
    theme: String,
}

impl Model {
    fn locals_ref(&self) -> u32 {
        self.frames.get(self.frame).map_or(0, |f| LOCALS_REF + f.id)
    }

    fn frame_id(&self) -> Option<u32> {
        self.frames.get(self.frame).map(|f| f.id)
    }

    fn ask(&mut self, command: Command, want: Want) -> bool {
        match self.session.request(command) {
            Ok(seq) => {
                self.pending.insert(seq, want);
                true
            }
            Err(_) => false,
        }
    }

    /// Everything of the last stop forgotten (the program goes on, or
    /// ended).
    fn forget(&mut self) {
        self.frames.clear();
        self.frame = 0;
        self.vars.clear();
        self.watch_values.clear();
        self.pending.clear();
        self.held = None;
        self.refreshing = false;
    }

    /// The selected frame's locals and the watches, asked again.
    fn fetch_frame(&mut self) {
        let locals = self.locals_ref();
        if locals != 0 {
            self.vars.remove(&locals);
            self.ask(Command::Variables { reference: locals, start: None, count: None }, Want::Vars(locals));
        }
        self.fetch_watches();
    }

    fn fetch_watches(&mut self) {
        let frame = self.frame_id();
        self.watch_values = self.watches.iter().map(|_| (String::new(), String::new(), 0)).collect();
        for (i, w) in self.watches.clone().into_iter().enumerate() {
            self.ask(Command::Evaluate { expr: w, frame, context: None }, Want::Watch(i));
        }
    }

    /// StackTrace's text: a frame a line, its name, file and line.
    fn stack_text(&self) -> Value {
        table(self.frames.iter().map(|f| vec![f.name.clone(), f.file.clone().unwrap_or_default(), f.line.to_string()]))
    }

    /// WatchValues' text: a watch a line, its expression, value, type and
    /// children's reference.
    fn watch_text(&self) -> Value {
        table(self.watches.iter().enumerate().map(|(i, w)| {
            let (v, k, r) = self.watch_values.get(i).cloned().unwrap_or_default();
            vec![w.clone(), v, k, r.to_string()]
        }))
    }

    /// Whether the snapshot (stack, scopes, watches) is all in.
    fn snapshot_done(&self) -> bool {
        !self.pending.values().any(|w| matches!(w, Want::Stack | Want::Vars(_) | Want::Watch(_)))
    }
}

thread_local! {
    static SESSIONS: RefCell<BTreeMap<String, Model>> = RefCell::new(BTreeMap::new());
}

fn with<R>(name: &str, f: impl FnOnce(&mut Model) -> R) -> R {
    SESSIONS.with(|s| f(s.borrow_mut().entry(name.to_ascii_lowercase()).or_default()))
}

/// COMMAND$'s arguments from one string: spaces separate them, a "quoted"
/// argument keeps its spaces.
pub fn split_args(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let (mut quoted, mut any) = (false, false);
    for c in s.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                any = true;
            }
            ' ' | '\t' if !quoted => {
                if any {
                    out.push(std::mem::take(&mut cur));
                    any = false;
                }
            }
            _ => {
                cur.push(c);
                any = true;
            }
        }
    }
    if any {
        out.push(cur);
    }
    out
}

pub fn get(name: &str, prop: &str) -> Option<Value> {
    with(name, |m| {
        let s = &m.session;
        Some(match prop {
            "program" => Value::String(s.program.clone()),
            "args" => Value::String(m.args.clone()),
            "debug" => flag(s.debug),
            "breakonerror" => flag(s.break_on_error),
            "stoponentry" => flag(s.stop_on_entry),
            "state" => Value::String(s.state().as_str().to_string()),
            "currentfile" => Value::String(s.current_file().unwrap_or("").to_string()),
            "currentline" => Value::Integer(i64::from(s.current_line())),
            "exitcode" => Value::Integer(i64::from(s.exit_code().unwrap_or(0))),
            "stopreason" => Value::String(s.stop_reason().to_string()),
            "stopmessage" => Value::String(s.stop_description().to_string()),
            "error" => Value::String(m.error.clone()),
            "frame" => Value::Integer(m.frame as i64),
            "localsref" => Value::Integer(i64::from(m.locals_ref())),
            "globalsref" => Value::Integer(i64::from(GLOBALS_REF)),
            "watches" => Value::String(m.watches.iter().map(|w| format!("{w}\n")).collect()),
            "browserurl" => Value::String(browser::url(name)),
            "theme" => Value::String(m.theme.clone()),
            _ => return None,
        })
    })
}

pub fn set<H: Host>(_host: H, name: &str, prop: &str, v: &Value) -> bool {
    with(name, |m| {
        match prop {
            "program" => m.session.program = v.to_string_val(),
            "args" => {
                m.args = v.to_string_val();
                m.session.args = split_args(&m.args);
            }
            "debug" => m.session.debug = v.to_bool(),
            "breakonerror" => m.session.break_on_error = v.to_bool(),
            "stoponentry" => m.session.stop_on_entry = v.to_bool(),
            "frame" => {
                let f = (v.to_i64().max(0) as usize).min(m.frames.len().saturating_sub(1));
                if f != m.frame && m.session.state() == State::Paused && m.held.is_none() {
                    m.frame = f;
                    m.refreshing = true;
                    m.fetch_frame();
                }
            }
            "watches" => {
                m.watches = v.to_string_val().lines().map(str::trim).filter(|l| !l.is_empty()).map(str::to_string).collect();
                if m.session.state() == State::Paused && m.held.is_none() && !m.frames.is_empty() {
                    m.refreshing = true;
                    m.fetch_watches();
                } else {
                    m.watch_values.clear();
                }
            }
            "state" | "currentfile" | "currentline" | "exitcode" | "error" | "stopreason" | "stopmessage" | "localsref" | "globalsref" | "browserurl" => {}
            "theme" => m.theme = v.to_string_val(),
            _ => return false,
        }
        true
    })
}

pub fn call<H: Host>(host: H, name: &str, method: &str, args: &[Value]) -> Option<Value> {
    let r = |res: Result<(), String>| -> Value {
        let ok = res.is_ok();
        with(name, |m| m.error = res.err().unwrap_or_default());
        flag(ok)
    };
    Some(match method {
        "start" => {
            let (program, run_args, theme, running) = with(name, |m| (m.session.program.clone(), m.session.args.clone(), m.theme.clone(), m.session.state() != State::Stopped));
            if running {
                return Some(r(Err("the program is running already".into())));
            }
            let started = host.launch(&program, &run_args, &theme).and_then(|t| with(name, |m| {
                m.forget();
                m.session.start(t)
            }));
            let ok = started.is_ok();
            let v = r(started);
            if ok {
                poll(host);
            }
            v
        }
        "stop" => {
            with(name, |m| m.session.stop());
            poll(host);
            Value::Null
        }
        "pause" => r(with(name, |m| m.session.pause())),
        "continue" => r(with(name, |m| m.session.continue_())),
        "stepin" | "stepinto" => r(with(name, |m| m.session.step_in())),
        "stepover" => r(with(name, |m| m.session.step_over())),
        "stepout" => r(with(name, |m| m.session.step_out())),
        "setbreakpoint" => {
            let (file, line) = (text_arg(args, 0), int_arg(args, 1, 0).max(1) as u32);
            let opt = |i: usize| Some(text_arg(args, i)).filter(|t| !t.trim().is_empty());
            let bp = SourceBreakpoint { line, condition: opt(2), hit: opt(3), log: opt(4) };
            r(with(name, |m| m.session.set_breakpoint_rules(bp, &file)))
        }
        "clearbreakpoint" => {
            let (file, line) = (text_arg(args, 0), int_arg(args, 1, 0).max(1) as u32);
            r(with(name, |m| m.session.clear_breakpoint(&file, line)))
        }
        "clearbreakpoints" => {
            let file = text_arg(args, 0);
            r(with(name, |m| m.session.clear_breakpoints(if file.is_empty() { None } else { Some(file.as_str()) })))
        }
        // (a stopped program stops there once started: Start it after)
        "runtocursor" => {
            let (file, line) = (text_arg(args, 0), int_arg(args, 1, 0).max(1) as u32);
            r(with(name, |m| m.session.run_to(&file, line)))
        }
        "runinbrowser" => {
            let program = with(name, |m| m.session.program.clone());
            let res = browser::start(host, name, &program);
            if res.is_ok() {
                poll(host);
            }
            r(res)
        }
        "stopbrowser" => {
            browser::stop(name);
            Value::Null
        }
        "stacktrace" => with(name, |m| m.stack_text()),
        "variables" => {
            let reference = int_arg(args, 0, 0).max(0) as u32;
            with(name, |m| table(m.vars.get(&reference).into_iter().flatten().map(|v| vec![v.name.clone(), v.value.clone(), v.kind.clone(), v.reference.to_string(), v.count.to_string()])))
        }
        "watchvalues" => with(name, |m| m.watch_text()),
        "expand" => {
            let reference = int_arg(args, 0, 0).max(0) as u32;
            let cached = with(name, |m| {
                if m.vars.contains_key(&reference) {
                    return true;
                }
                if m.session.state() == State::Paused && reference != 0 {
                    m.ask(Command::Variables { reference, start: None, count: None }, Want::Expand(reference));
                }
                false
            });
            if cached {
                host.fire(name, "onvariables", &[Value::Integer(i64::from(reference))]);
            }
            Value::Null
        }
        "evaluate" | "setvariable" => {
            let setting = method == "setvariable";
            let (text, value, context) = (text_arg(args, 0), text_arg(args, 1), text_arg(args, 1));
            let (id, sent) = with(name, |m| {
                m.next_eval += 1;
                let id = m.next_eval;
                let frame = m.frame_id();
                let command = if setting {
                    Command::SetVariable { frame, name: text.clone(), value }
                } else {
                    Command::Evaluate { expr: text.clone(), frame, context: (context.eq_ignore_ascii_case("repl")).then(|| "repl".to_string()) }
                };
                let sent = m.session.state() == State::Paused && m.ask(command, Want::Eval { id, refresh: setting || context.eq_ignore_ascii_case("repl") });
                (id, sent)
            });
            if !sent {
                host.fire(name, "onevaluate", &[Value::Integer(id), Value::String("error: the program isn't paused".into())]);
            }
            Value::Integer(id)
        }
        "input" => r(with(name, |m| m.session.input(&text_arg(args, 0)))),
        _ => return None,
    })
}

/// Rows as text: one a line, fields tab-separated (tabs and line breaks
/// inside a field as spaces).
fn table(rows: impl Iterator<Item = Vec<String>>) -> Value {
    Value::String(rows.map(|r| r.iter().map(|f| f.replace(['\t', '\n', '\r'], " ")).collect::<Vec<_>>().join("\t") + "\n").collect())
}

/// Every session's news as its events; whether one is running.
pub fn poll<H: Host>(host: H) -> bool {
    let names: Vec<String> = SESSIONS.with(|s| s.borrow().keys().cloned().collect());
    let mut running = browser::poll(host);
    for name in names {
        let (events, live) = with(&name, |m| {
            let e = m.session.poll(Some(Duration::ZERO));
            (e, m.session.state() != State::Stopped)
        });
        running |= live;
        for e in events {
            match e {
                SessionEvent::Output { text, .. } => host.fire(&name, "onoutput", &[Value::String(text)]),
                SessionEvent::Stopped { reason, file, line, .. } => {
                    // (the snapshot first: OnStopped once it's in)
                    let fire = with(&name, |m| {
                        m.forget();
                        m.held = Some((reason, file.unwrap_or_default(), i64::from(line.unwrap_or(0))));
                        m.ask(Command::Variables { reference: GLOBALS_REF, start: None, count: None }, Want::Vars(GLOBALS_REF));
                        if m.ask(Command::StackTrace, Want::Stack) {
                            None
                        } else {
                            m.held.take()
                        }
                    });
                    if let Some((reason, file, line)) = fire {
                        host.fire(&name, "onstopped", &[Value::String(reason), Value::String(file), Value::Integer(line)]);
                    }
                }
                SessionEvent::Continued => {
                    with(&name, Model::forget);
                    host.fire(&name, "oncontinue", &[]);
                }
                SessionEvent::Exited { code } => {
                    with(&name, Model::forget);
                    host.fire(&name, "onexit", &[Value::Integer(i64::from(code))]);
                }
                SessionEvent::FormShown { id } => host.fire(&name, "onformshown", &[Value::String(id)]),
                SessionEvent::Reply { re, body } => reply(host, &name, re, body),
                SessionEvent::FormClosed { .. } => {}
            }
        }
    }
    running
}

/// A reply to one of the session's own requests.
fn reply<H: Host>(host: H, name: &str, re: u64, body: EventBody) {
    let mut fire: Vec<(&'static str, Vec<Value>)> = Vec::new();
    with(name, |m| {
        let Some(want) = m.pending.remove(&re) else { return };
        let error = match &body {
            EventBody::Error { message } => Some(message.clone()),
            _ => None,
        };
        match want {
            Want::Stack => {
                if let EventBody::StackTrace { frames } = body {
                    m.frames = frames;
                }
                m.frame = 0;
                m.fetch_frame();
            }
            Want::Vars(r) => {
                let list = match body {
                    EventBody::Variables { variables } => variables,
                    _ => Vec::new(),
                };
                m.vars.insert(r, list);
            }
            Want::Watch(i) => {
                let v = match body {
                    EventBody::Evaluate { result, kind, reference } => (result, kind, reference),
                    _ => (format!("error: {}", error.unwrap_or_default()), String::new(), 0),
                };
                if let Some(slot) = m.watch_values.get_mut(i) {
                    *slot = v;
                }
            }
            Want::Expand(r) => {
                let list = match body {
                    EventBody::Variables { variables } => variables,
                    _ => Vec::new(),
                };
                m.vars.insert(r, list);
                fire.push(("onvariables", vec![Value::Integer(i64::from(r))]));
            }
            Want::Eval { id, refresh } => {
                let text = match body {
                    EventBody::Evaluate { result, .. } => result,
                    EventBody::Ok => String::new(),
                    _ => format!("error: {}", error.unwrap_or_default()),
                };
                fire.push(("onevaluate", vec![Value::Integer(id), Value::String(text)]));
                // (a statement or an assignment may have changed what's shown)
                if refresh && m.held.is_none() && m.session.state() == State::Paused {
                    m.vars.clear();
                    m.ask(Command::Variables { reference: GLOBALS_REF, start: None, count: None }, Want::Vars(GLOBALS_REF));
                    m.refreshing = true;
                    m.fetch_frame();
                }
            }
        }
        if m.snapshot_done() {
            if let Some((reason, file, line)) = m.held.take() {
                fire.push(("onstopped", vec![Value::String(reason), Value::String(file), Value::Integer(line)]));
            } else if std::mem::take(&mut m.refreshing) {
                fire.push(("onvariables", vec![Value::Integer(0)]));
            }
        }
    });
    for (event, args) in fire {
        host.fire(name, event, &args);
    }
}

/// Whether a session is running (the desktop's loop then wakes often).
pub fn any_running() -> bool {
    SESSIONS.with(|s| s.borrow().values().any(|m| m.session.state() != State::Stopped)) || browser::starting()
}

/// Run in Browser (docs/studio-wow.md RUN-2): `rapidr serve <program>
/// --open` — the program's web build on 127.0.0.1 under a random path,
/// opened in the default browser — as a child process (the desktop's; the
/// web's Studio is in a browser already). Its lines are the session's
/// OnOutput; `BrowserURL` is where it serves. One per session: a new run
/// replaces the last; StopBrowser (or Studio's end: the server ends when its
/// input closes) ends it.
mod browser {
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::sync::mpsc::{self, Receiver};

    use rapidr_value::Value;

    use crate::Host;

    struct Server {
        child: std::process::Child,
        lines: Receiver<String>,
        url: String,
    }

    thread_local! {
        static SERVERS: RefCell<BTreeMap<String, Server>> = RefCell::new(BTreeMap::new());
    }

    pub fn start<H: Host>(host: H, name: &str, program: &str) -> Result<(), String> {
        use std::io::{BufRead, BufReader};
        use std::process::{Command, Stdio};
        if program.is_empty() {
            return Err("no program to run".into());
        }
        let rapidr = host.rapidr().ok_or("Studio runs in a browser already: Run (F5) runs the program here")?;
        stop(name);
        let mut cmd = Command::new(&rapidr);
        cmd.arg("serve").arg(program).arg("--open").stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000);
        }
        let mut child = cmd.spawn().map_err(|e| format!("{}: {e}", rapidr.display()))?;
        let (tx, lines) = mpsc::channel();
        for stream in [child.stdout.take().map(|s| Box::new(s) as Box<dyn std::io::Read + Send>), child.stderr.take().map(|s| Box::new(s) as Box<dyn std::io::Read + Send>)].into_iter().flatten() {
            let tx = tx.clone();
            std::thread::spawn(move || {
                for line in BufReader::new(stream).lines() {
                    let Ok(line) = line else { break };
                    if tx.send(line).is_err() {
                        break;
                    }
                }
            });
        }
        SERVERS.with(|s| s.borrow_mut().insert(name.to_ascii_lowercase(), Server { child, lines, url: String::new() }));
        Ok(())
    }

    pub fn stop(name: &str) {
        if let Some(mut s) = SERVERS.with(|s| s.borrow_mut().remove(&name.to_ascii_lowercase())) {
            let _ = s.child.kill();
            let _ = s.child.wait();
        }
    }

    pub fn url(name: &str) -> String {
        SERVERS.with(|s| s.borrow().get(&name.to_ascii_lowercase()).map(|s| s.url.clone()).unwrap_or_default())
    }

    /// A server still building its page (the desktop's loop wakes often
    /// until it says where it serves).
    pub fn starting() -> bool {
        SERVERS.with(|s| s.borrow().values().any(|s| s.url.is_empty()))
    }

    /// The servers' lines as OnOutput; a server that ended goes (its exit
    /// said why). Whether one is still starting.
    pub fn poll<H: Host>(host: H) -> bool {
        let mut out: Vec<(String, String)> = Vec::new();
        SERVERS.with(|s| {
            let mut s = s.borrow_mut();
            let mut ended = Vec::new();
            for (name, server) in s.iter_mut() {
                while let Ok(line) = server.lines.try_recv() {
                    if let Some(rest) = line.strip_prefix("Serving ") {
                        server.url = rest.split_whitespace().next().unwrap_or("").to_string();
                    }
                    out.push((name.clone(), line + "\n"));
                }
                if let Ok(Some(status)) = server.child.try_wait() {
                    if server.url.is_empty() {
                        out.push((name.clone(), format!("[Run in Browser ended: exit code {}]\n", status.code().unwrap_or(1))));
                    }
                    ended.push(name.clone());
                }
            }
            for n in ended {
                s.remove(&n);
            }
        });
        for (name, text) in out {
            host.fire(&name, "onoutput", &[Value::String(text)]);
        }
        starting()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arguments() {
        assert_eq!(split_args(r#"a  "b c" d"#), ["a", "b c", "d"]);
        assert_eq!(split_args(r#""""#), [""]);
        assert!(split_args("  ").is_empty());
    }
}

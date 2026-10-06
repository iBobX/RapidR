//! The IDE's end of the session: [`ProgramSession`], the model of the
//! non-visual RProgramSession component (docs/ide-components.md):
//! `Program`, `Args`, `Debug`, `SeparateWindows`, `State`, `CurrentFile`,
//! `CurrentLine`; `Start`, `Stop`, `Pause`, `Continue`, `StepIn`,
//! `StepOver`, `StepOut`, `SetBreakpoint(File, Line, Condition)`,
//! `Evaluate(Expr)`, `SetProperty(Object, Prop, Value)`; `OnOutput(Text)`,
//! `OnStopped(Reason, File, Line)`, `OnExit(Code)`, `OnFormShown(Id)`.
//!
//! It speaks to the program through a [`Transport`]: a child process on the
//! desktop ([`crate::process::ProcessTransport`]: `rapidr run --session`),
//! the preview frame's MessagePort on the web (the page feeds what arrives
//! with [`ProgramSession::receive`]).

use std::collections::{BTreeMap, VecDeque};
use std::time::Duration;

use crate::protocol::{Command, Event, EventBody, Incoming, Request, SourceBreakpoint, StackFrame, Variable};

/// How the IDE reaches the program.
pub trait Transport {
    /// Sends a request.
    fn send(&mut self, request: &Request) -> Result<(), String>;
    /// What has arrived, waiting up to `timeout` for something (`None`:
    /// don't wait). [`Incoming::Text`] is the program's own output.
    fn receive(&mut self, timeout: Option<Duration>) -> Vec<Incoming>;
    /// Ends the program at once.
    fn kill(&mut self);
    /// The program's exit code once it has ended without saying so (a
    /// process that ended; END in a desktop program exits at once).
    fn ended(&mut self) -> Option<i32>;
}

/// The program's state, as `State` reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum State {
    #[default]
    Stopped,
    Running,
    Paused,
}

impl State {
    pub fn as_str(self) -> &'static str {
        match self {
            State::Stopped => "stopped",
            State::Running => "running",
            State::Paused => "paused",
        }
    }
}

/// What happened, as the component's events report it.
#[derive(Debug, Clone, PartialEq)]
pub enum SessionEvent {
    /// `OnOutput(Text)`: `stream` is `stdout` or `stderr`.
    Output { stream: String, text: String },
    /// `OnStopped(Reason, File, Line)`.
    Stopped { reason: String, file: Option<String>, line: Option<u32>, description: Option<String> },
    Continued,
    /// `OnExit(Code)`.
    Exited { code: i32 },
    /// `OnFormShown(Id)`.
    FormShown { id: String },
    FormClosed { id: String },
    /// The reply to an asynchronous request ([`ProgramSession::request`]).
    Reply { re: u64, body: EventBody },
}

/// A run of a program under the IDE (RProgramSession's model).
pub struct ProgramSession {
    /// `Program`: the file (or project's main file) to run.
    pub program: String,
    /// `Args`.
    pub args: Vec<String>,
    /// `Debug`: run under the debugger (breakpoints, stepping).
    pub debug: bool,
    /// `SeparateWindows`: the program's forms as its own windows (the only
    /// way until remote forms, I5).
    pub separate_windows: bool,
    /// Stop at a run-time error's statement.
    pub break_on_error: bool,
    /// Stop at the program's first statement.
    pub stop_on_entry: bool,
    state: State,
    current_file: Option<String>,
    current_line: u32,
    exit_code: Option<i32>,
    breakpoints: BTreeMap<String, Vec<SourceBreakpoint>>,
    transport: Option<Box<dyn Transport>>,
    next_seq: u64,
    events: VecDeque<SessionEvent>,
}

impl Default for ProgramSession {
    fn default() -> Self {
        ProgramSession {
            program: String::new(),
            args: Vec::new(),
            debug: true,
            separate_windows: true,
            break_on_error: false,
            stop_on_entry: false,
            state: State::Stopped,
            current_file: None,
            current_line: 0,
            exit_code: None,
            breakpoints: BTreeMap::new(),
            transport: None,
            next_seq: 1,
            events: VecDeque::new(),
        }
    }
}

/// How long a request that answers at once may take.
pub const REPLY_TIMEOUT: Duration = Duration::from_secs(10);

impl ProgramSession {
    pub fn new(program: impl Into<String>) -> Self {
        ProgramSession { program: program.into(), ..Self::default() }
    }

    /// `State`.
    pub fn state(&self) -> State {
        self.state
    }

    /// `CurrentFile`: where the program is stopped.
    pub fn current_file(&self) -> Option<&str> {
        self.current_file.as_deref()
    }

    /// `CurrentLine` (0 when not stopped).
    pub fn current_line(&self) -> u32 {
        self.current_line
    }

    /// The last run's exit code.
    pub fn exit_code(&self) -> Option<i32> {
        self.exit_code
    }

    /// `Start` over `transport` (a program that's ready for `start`): the
    /// breakpoints go first, then the program runs.
    pub fn start(&mut self, transport: Box<dyn Transport>) -> Result<(), String> {
        if self.transport.is_some() && self.state != State::Stopped {
            return Err("the program is running already".into());
        }
        self.transport = Some(transport);
        self.exit_code = None;
        self.current_file = None;
        self.current_line = 0;
        self.events.clear();
        let files: Vec<(String, Vec<SourceBreakpoint>)> = self.breakpoints.iter().map(|(f, b)| (f.clone(), b.clone())).collect();
        for (file, breakpoints) in files {
            self.request(Command::SetBreakpoints { file, breakpoints })?;
        }
        if self.break_on_error {
            self.request(Command::SetBreakOnError { enabled: true })?;
        }
        self.request(Command::Start {
            program: None,
            args: Vec::new(),
            debug: self.debug,
            stop_on_entry: self.stop_on_entry,
            break_on_error: self.break_on_error,
        })?;
        self.state = State::Running;
        Ok(())
    }

    /// Starts `rapidr run --session` on [`Self::program`] (`runtime`: the
    /// `rapidr` executable).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn start_process(&mut self, runtime: &std::path::Path) -> Result<(), String> {
        let transport = crate::process::ProcessTransport::spawn(runtime, &self.program, &self.args, None).map_err(|e| e.to_string())?;
        self.start(Box::new(transport))
    }

    /// `Stop`: the program ends at once.
    pub fn stop(&mut self) {
        if let Some(t) = self.transport.as_mut() {
            let _ = t.send(&Request::new(0, Command::Stop));
            t.kill();
        }
        if self.state != State::Stopped {
            self.finish(self.exit_code.unwrap_or(0));
        }
    }

    /// `Pause`: at the next statement the program runs.
    pub fn pause(&mut self) -> Result<(), String> {
        self.request(Command::Pause).map(drop)
    }

    /// `Continue`.
    pub fn continue_(&mut self) -> Result<(), String> {
        self.go(Command::Continue)
    }

    /// `StepIn`.
    pub fn step_in(&mut self) -> Result<(), String> {
        self.go(Command::StepIn)
    }

    /// `StepOver`.
    pub fn step_over(&mut self) -> Result<(), String> {
        self.go(Command::StepOver)
    }

    /// `StepOut`.
    pub fn step_out(&mut self) -> Result<(), String> {
        self.go(Command::StepOut)
    }

    fn go(&mut self, command: Command) -> Result<(), String> {
        if self.state != State::Paused {
            return Err("the program isn't stopped".into());
        }
        self.request(command)?;
        self.state = State::Running;
        self.current_line = 0;
        Ok(())
    }

    /// `SetBreakpoint(File, Line, Condition)`: adds (or changes) one.
    pub fn set_breakpoint(&mut self, file: &str, line: u32, condition: Option<&str>) -> Result<(), String> {
        let list = self.breakpoints.entry(file.to_string()).or_default();
        list.retain(|b| b.line != line);
        list.push(SourceBreakpoint { line, condition: condition.map(str::to_string), ..Default::default() });
        list.sort_by_key(|b| b.line);
        self.send_breakpoints(file)
    }

    /// Removes the breakpoint at `line` of `file`.
    pub fn clear_breakpoint(&mut self, file: &str, line: u32) -> Result<(), String> {
        if let Some(list) = self.breakpoints.get_mut(file) {
            list.retain(|b| b.line != line);
        }
        self.send_breakpoints(file)
    }

    /// Every breakpoint, by file.
    pub fn breakpoints(&self) -> &BTreeMap<String, Vec<SourceBreakpoint>> {
        &self.breakpoints
    }

    fn send_breakpoints(&mut self, file: &str) -> Result<(), String> {
        if self.state == State::Stopped {
            return Ok(());
        }
        let breakpoints = self.breakpoints.get(file).cloned().unwrap_or_default();
        self.request(Command::SetBreakpoints { file: file.to_string(), breakpoints }).map(drop)
    }

    /// `Evaluate(Expr)` in the stopped frame (or the top level): the value
    /// as the debugger shows it.
    pub fn evaluate(&mut self, expr: &str) -> Result<String, String> {
        match self.request_wait(Command::Evaluate { expr: expr.to_string(), frame: None, context: None })? {
            EventBody::Evaluate { result, .. } => Ok(result),
            other => Err(format!("unexpected reply {other:?}")),
        }
    }

    /// `SetProperty(Object, Prop, Value)`: `value` is a BASIC expression.
    pub fn set_property(&mut self, object: &str, prop: &str, value: &str) -> Result<(), String> {
        self.request_wait(Command::SetProperty { object: object.into(), prop: prop.into(), value: value.into() }).map(drop)
    }

    /// Sets a variable of the stopped frame to a BASIC expression's value;
    /// returns the new value.
    pub fn set_variable(&mut self, name: &str, value: &str) -> Result<String, String> {
        match self.request_wait(Command::SetVariable { frame: None, name: name.into(), value: value.into() })? {
            EventBody::Evaluate { result, .. } => Ok(result),
            other => Err(format!("unexpected reply {other:?}")),
        }
    }

    /// The stopped program's frames, innermost first.
    pub fn stack_trace(&mut self) -> Result<Vec<StackFrame>, String> {
        match self.request_wait(Command::StackTrace)? {
            EventBody::StackTrace { frames } => Ok(frames),
            other => Err(format!("unexpected reply {other:?}")),
        }
    }

    /// The variables of a scope or a value (`crate::program::GLOBALS_REF`,
    /// `LOCALS_REF + frame`, a value's `ref`).
    pub fn variables(&mut self, reference: u32) -> Result<Vec<Variable>, String> {
        match self.request_wait(Command::Variables { reference, start: None, count: None })? {
            EventBody::Variables { variables } => Ok(variables),
            other => Err(format!("unexpected reply {other:?}")),
        }
    }

    /// A line for the program's INPUT.
    pub fn input(&mut self, text: &str) -> Result<(), String> {
        self.request(Command::Input { text: Some(text.to_string()), form: None, event: None }).map(drop)
    }

    /// Sends a request; its reply arrives as [`SessionEvent::Reply`].
    pub fn request(&mut self, command: Command) -> Result<u64, String> {
        let seq = self.next_seq;
        self.next_seq += 1;
        let transport = self.transport.as_mut().ok_or("the program isn't running")?;
        transport.send(&Request::new(seq, command))?;
        Ok(seq)
    }

    /// Sends a request and waits for its reply (what else arrives
    /// meanwhile is kept for [`Self::poll`]).
    pub fn request_wait(&mut self, command: Command) -> Result<EventBody, String> {
        let seq = self.request(command)?;
        let mut waited = Duration::ZERO;
        let step = Duration::from_millis(50);
        while waited < REPLY_TIMEOUT {
            let incoming = match self.transport.as_mut() {
                Some(t) => t.receive(Some(step)),
                None => return Err("the program has ended".into()),
            };
            waited += step;
            let mut found = None;
            for item in incoming {
                if let Incoming::Event(Event { re: Some(re), body }) = &item {
                    if *re == seq {
                        found = Some(body.clone());
                        continue;
                    }
                }
                self.take(item);
            }
            if let Some(body) = found {
                return match body {
                    EventBody::Error { message } => Err(message),
                    body => Ok(body),
                };
            }
            if self.state == State::Stopped {
                return Err("the program has ended".into());
            }
            self.check_ended();
        }
        Err("the program didn't answer".into())
    }

    /// What happened since the last call, waiting up to `timeout` for
    /// something.
    pub fn poll(&mut self, timeout: Option<Duration>) -> Vec<SessionEvent> {
        if self.events.is_empty() {
            if let Some(t) = self.transport.as_mut() {
                for item in t.receive(timeout) {
                    self.take(item);
                }
            }
            self.check_ended();
        }
        self.events.drain(..).collect()
    }

    /// Feeds what arrived from the program (a transport that delivers by
    /// itself: the web page's MessagePort).
    pub fn receive(&mut self, item: Incoming) {
        self.take(item);
    }

    fn check_ended(&mut self) {
        if self.state == State::Stopped {
            return;
        }
        if let Some(code) = self.transport.as_mut().and_then(|t| t.ended()) {
            // (anything still on the way first)
            if let Some(t) = self.transport.as_mut() {
                for item in t.receive(None) {
                    self.take(item);
                }
            }
            if self.state != State::Stopped {
                self.finish(code);
            }
        }
    }

    fn finish(&mut self, code: i32) {
        self.state = State::Stopped;
        self.current_file = None;
        self.current_line = 0;
        self.exit_code = Some(code);
        self.events.push_back(SessionEvent::Exited { code });
        self.transport = None;
    }

    fn take(&mut self, item: Incoming) {
        match item {
            Incoming::Text(text) => self.push_output("stdout", text),
            Incoming::Garbled(text) => self.push_output("stdout", text),
            Incoming::Event(Event { re: Some(re), body }) => self.events.push_back(SessionEvent::Reply { re, body }),
            Incoming::Event(Event { re: None, body }) => match body {
                EventBody::Output { stream, text } => self.push_output(&stream, text),
                EventBody::Stopped { reason, file, line, description } => {
                    self.state = State::Paused;
                    self.current_file = file.clone();
                    self.current_line = line.unwrap_or(0);
                    self.events.push_back(SessionEvent::Stopped { reason, file, line, description });
                }
                EventBody::Continued => {
                    self.state = State::Running;
                    self.current_line = 0;
                    self.events.push_back(SessionEvent::Continued);
                }
                EventBody::Exited { code } => {
                    if let Some(t) = self.transport.as_mut() {
                        t.kill();
                    }
                    self.finish(code);
                }
                EventBody::FormShown { id, .. } => self.events.push_back(SessionEvent::FormShown { id }),
                EventBody::FormClosed { id } => self.events.push_back(SessionEvent::FormClosed { id }),
                // (remote forms: I5)
                EventBody::FormFrame { .. } | EventBody::Ready { .. } => {}
                other => self.events.push_back(SessionEvent::Reply { re: 0, body: other }),
            },
        }
    }

    fn push_output(&mut self, stream: &str, text: String) {
        if let Some(SessionEvent::Output { stream: s, text: t }) = self.events.back_mut() {
            if s == stream {
                t.push_str(&text);
                return;
            }
        }
        self.events.push_back(SessionEvent::Output { stream: stream.to_string(), text });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// A transport that records requests and plays back events.
    #[derive(Clone, Default)]
    struct Fake {
        sent: Arc<Mutex<Vec<Request>>>,
        inbox: Arc<Mutex<VecDeque<Incoming>>>,
    }

    impl Transport for Fake {
        fn send(&mut self, request: &Request) -> Result<(), String> {
            self.sent.lock().unwrap().push(request.clone());
            // (an evaluate is answered at once)
            if let Command::Evaluate { expr, .. } = &request.command {
                let body = EventBody::Evaluate { result: format!("<{expr}>"), kind: "String".into(), reference: 0 };
                self.inbox.lock().unwrap().push_back(Incoming::Event(Event::reply(request.seq, body)));
            }
            Ok(())
        }
        fn receive(&mut self, _timeout: Option<Duration>) -> Vec<Incoming> {
            self.inbox.lock().unwrap().drain(..).collect()
        }
        fn kill(&mut self) {}
        fn ended(&mut self) -> Option<i32> {
            None
        }
    }

    #[test]
    fn the_session_tracks_the_program() {
        let fake = Fake::default();
        let mut s = ProgramSession::new("prog.bas");
        s.set_breakpoint("util.inc", 3, None).unwrap();
        s.start(Box::new(fake.clone())).unwrap();
        let kinds: Vec<String> = fake.sent.lock().unwrap().iter().map(|r| r.to_json()).collect();
        assert!(kinds[0].contains("\"setBreakpoints\"") && kinds[0].contains("util.inc"), "{kinds:?}");
        assert!(kinds[1].contains("\"start\""), "{kinds:?}");
        assert_eq!(s.state(), State::Running);
        fake.inbox.lock().unwrap().extend([
            Incoming::Text("hel".into()),
            Incoming::Event(Event::new(EventBody::Output { stream: "stdout".into(), text: "lo\n".into() })),
            Incoming::Event(Event::new(EventBody::Stopped { reason: "breakpoint".into(), file: Some("util.inc".into()), line: Some(3), description: None })),
        ]);
        let events = s.poll(None);
        assert_eq!(events[0], SessionEvent::Output { stream: "stdout".into(), text: "hello\n".into() });
        assert!(matches!(&events[1], SessionEvent::Stopped { line: Some(3), .. }));
        assert_eq!((s.state(), s.current_file(), s.current_line()), (State::Paused, Some("util.inc"), 3));
        assert_eq!(s.evaluate("x + 1").unwrap(), "<x + 1>");
        s.continue_().unwrap();
        assert_eq!(s.state(), State::Running);
        fake.inbox.lock().unwrap().push_back(Incoming::Event(Event::new(EventBody::Exited { code: 3 })));
        assert_eq!(s.poll(None), vec![SessionEvent::Exited { code: 3 }]);
        assert_eq!((s.state(), s.exit_code()), (State::Stopped, Some(3)));
    }
}

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
//! | `LocalsRef`, `GlobalsRef`, `ComponentsRef` | the selected frame's locals', the globals' and the program's components' references, for `Variables` |
//! | `Watches` | the watch expressions, one a line; evaluated at every stop |
//! | `Start` → True / False, `Stop`, `Pause`, `Continue`, `StepIn`, `StepOver`, `StepOut` | |
//! | `ShowProgram` → True / False | the program's windows to the front, its app active (Show Program Window; a paused program's windows show and move but take no input) |
//! | `SetBreakpoint(File, Line [, Condition [, HitCount [, LogMessage]]])`, `ClearBreakpoint(File, Line)`, `ClearBreakpoints([File])` | lines from 1 |
//! | `RunToCursor(File, Line)` | on to that line (a stopped program starts) |
//! | `StackTrace`, `Variables(Ref)`, `WatchValues` → text | what the last stop fetched: one item a line, its fields tab-separated — a variable's name, value, type, children's reference, how many, the expression naming it, 1 when its value changed since the last stop, a table's columns and a table row's cells (CHR$(31) between them); a watch's expression, value, type, reference, changed, how many children |
//! | `Expand(Ref)`, `ExpandMore(Ref, Start)` | fetches a value's children (an array's elements, an object's fields, a component's properties and parts, a frame's rows, an RNum's numbers), 100 at a time — the next 100 from Start: OnVariables(Ref) |
//! | `Evaluate(Expr [, Context])` → Id | in the selected frame: OnEvaluate(Id, Result, Kind, Ref, Count, Columns). Context `repl` (the Immediate window): `? x` prints, a statement runs; `clipboard`: the value whole, as text (copying it, the text viewer) |
//! | `SetVariable(Name, Value)` → Id | a variable of the selected frame set to an expression's value: OnEvaluate(Id, NewValue), then the variables again (OnVariables(0)) |
//! | `SetNextStatement(File, Line)` → Id | Set Next Statement: the paused program goes on from that line (within its SUB or FUNCTION): OnEvaluate(Id, "") then OnStopped("goto", …), or OnEvaluate(Id, "error: …") |
//! | `StepInTargets`, `StepInTarget(Id)` | Step Into Specific: the calls the paused line still makes — OnStepInTargets(Targets: a line each, its id and name tab-separated) —; into the one picked |
//! | `Input(Text)` | a line for the program's INPUT |
//! | `App(Request)` → Id | the eyes and hands on the running program (rapidr-session's `app`, `rapidr_ui_app::link`): a JSON request — `{"do":"screenshot"}`, `{"do":"layout"}`, `{"do":"click","caption":"7"}`, `type`, `key`, `drag`, `scroll`, `wait` —; OnApp(Id, Ok, Text, Picture) when it is answered (Picture: the PNG a screenshot gave, a file in the temporary folder the caller removes — a `data:` URL on the web; "" for none), or when the program didn't answer in time |
//! | `OnOutput(Text)`, `OnStopped(Reason, File, Line)`, `OnContinue`, `OnExit(Code)`, `OnFormShown(Id)`, `OnVariables(Ref)`, `OnEvaluate(Id, Result)`, `OnBreakpointPlaced(File, Line, NewLine)` | |

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::time::Duration;

use rapidr_session::protocol::{SourceBreakpoint, StackFrame, Variable, COMPONENTS_REF, GLOBALS_REF, LOCALS_REF};

/// Elements, rows or numbers fetched at a time (`ExpandMore`).
const PAGE: u32 = 100;
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
    /// `ExpandMore(Ref, Start)`'s page: added to what Ref has.
    More(u32),
    /// `StepInTargets`' answer: OnStepInTargets.
    Targets,
    /// `App`'s answer: OnApp(Id, …); `until`: when it stops waiting for it
    /// (`rapidr_ai::rai::now_ms`).
    App { id: i64, until: f64 },
    /// RAI's app tool ([`app_send`]): its answer kept for [`app_take`].
    RaiApp { ticket: String, until: f64 },
}

/// How long `App` waits for the program: a look 10 s, input 20 s more
/// than what it asked to wait.
fn app_timeout(request: &serde_json::Value) -> f64 {
    let what = request.get("do").and_then(serde_json::Value::as_str).unwrap_or("");
    let waits = request.get("ms").and_then(serde_json::Value::as_f64).unwrap_or(0.0).clamp(0.0, 60_000.0);
    match what {
        "screenshot" | "capture" | "picture" | "layout" | "windows" | "cursor" => 10_000.0,
        _ => 20_000.0 + waits,
    }
}

thread_local! {
    /// RAI's app tools' answers by ticket (`None`: still waiting).
    static RAI_APPS: RefCell<std::collections::HashMap<String, Option<(bool, String)>>> = RefCell::new(std::collections::HashMap::new());
}

/// (RAI's app tools: the running program's eyes and hands) A request to
/// the program Studio runs now: its ticket for [`app_take`], or why it
/// can't be sent (nothing runs).
pub fn app_send(request: serde_json::Value) -> Result<String, String> {
    let running: Option<String> = SESSIONS.with(|s| s.try_borrow().ok().and_then(|s| s.iter().find(|(_, m)| m.session.state() != State::Stopped).map(|(n, _)| n.clone())));
    let Some(name) = running else {
        return Err("The program isn't running: ask the user to run it (Run, F5), then try again.".into());
    };
    let until = rapidr_ai::rai::now_ms() + app_timeout(&request);
    let ticket = with(&name, |m| {
        m.next_app += 1;
        let ticket = format!("{name}:{}", m.next_app);
        m.ask(Command::App { request }, Want::RaiApp { ticket: ticket.clone(), until }).then_some(ticket)
    });
    let ticket = ticket.ok_or("The program couldn't be reached (it is ending?): try again.")?;
    RAI_APPS.with(|r| r.borrow_mut().insert(ticket.clone(), None));
    Ok(ticket)
}

/// (RAI's app tools) Request `ticket`'s answer once it came: (done, what
/// was seen or done, in a sentence).
pub fn app_take(ticket: &str) -> Option<(bool, String)> {
    RAI_APPS.with(|r| {
        let mut r = r.borrow_mut();
        let done = r.get(ticket)?.clone()?;
        r.remove(ticket);
        Some(done)
    })
}

fn rai_answered(ticket: String, ok: bool, text: String) {
    RAI_APPS.with(|r| {
        r.borrow_mut().insert(ticket, Some((ok, text)));
    });
}

/// The pictures a Studio left in the temporary folder (`rapidr-app-*.png`:
/// each is removed as soon as it is handed over, but a crash or a killed
/// Studio leaves it) older than `age`: removed. How many.
#[cfg(not(target_arch = "wasm32"))]
pub fn sweep_pictures(dir: &std::path::Path, age: std::time::Duration) -> usize {
    let Ok(entries) = std::fs::read_dir(dir) else { return 0 };
    let now = std::time::SystemTime::now();
    let mut n = 0;
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if !name.starts_with("rapidr-app-") || !name.ends_with(".png") {
            continue;
        }
        // (a file of its own, never a link to something else)
        let Ok(meta) = std::fs::symlink_metadata(e.path()) else { continue };
        let old = meta.modified().ok().and_then(|m| now.duration_since(m).ok()).is_some_and(|d| d >= age);
        if meta.is_file() && old && std::fs::remove_file(e.path()).is_ok() {
            n += 1;
        }
    }
    n
}

/// A screenshot's PNG (base64) where the caller can take it: a file in the
/// temporary folder, readable by this user only, on the desktop; a `data:`
/// URL on the web.
fn app_picture(id: i64, png_base64: &str) -> String {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = id;
        format!("data:image/png;base64,{png_base64}")
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let Some(bytes) = rapidr_value::objects::codec::base64_decode(png_base64) else { return String::new() };
        let path = std::env::temp_dir().join(format!("rapidr-app-{}-{id}.png", std::process::id()));
        if std::fs::write(&path, bytes).is_err() {
            return String::new();
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        }
        path.to_string_lossy().into_owned()
    }
}

/// A variable as the last stop fetched it, and whether its value changed
/// since the stop before.
type Shown = (Variable, bool);

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
    vars: BTreeMap<u32, Vec<Shown>>,
    /// Each watch's value, kind, reference (its children) and whether it
    /// changed.
    watch_values: Vec<(String, String, u32, bool, u32)>,
    /// What each reference's values are compared under (their scope: a
    /// frame's routine, the globals, a watch, an evaluation), so a value
    /// changed since the last stop shows changed.
    scope: BTreeMap<u32, String>,
    /// The values seen at the stops before this one, and at this one, by
    /// scope and the expression naming them.
    seen_before: std::collections::HashMap<String, String>,
    seen_now: std::collections::HashMap<String, String>,
    /// (S11) What this stop showed first, by the same keys: a value set
    /// while paused (Variables' F2, Immediate's `x = 5`) differs from it.
    first_now: std::collections::HashMap<String, String>,
    /// Requests in flight.
    pending: BTreeMap<u64, Want>,
    /// A stop whose snapshot is still arriving: OnStopped waits for it.
    held: Option<(String, String, i64)>,
    /// A frame or watch refresh in flight: OnVariables(0) when done.
    refreshing: bool,
    next_eval: i64,
    /// `App`'s requests' numbers.
    next_app: i64,
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
        // (the eyes and hands' requests are answered whether it stops or
        // goes on: kept)
        self.pending.retain(|_, w| matches!(w, Want::App { .. } | Want::RaiApp { .. }));
        self.held = None;
        self.refreshing = false;
        self.scope.clear();
        // (what this stop showed is what the next one compares with)
        let now = std::mem::take(&mut self.seen_now);
        self.seen_before.extend(now);
        self.first_now.clear();
    }

    /// Whether `value` (named `key` in its scope) differs from what an
    /// earlier stop showed — or from what this one first showed: a value
    /// set while paused —; remembered for the next.
    fn changed(&mut self, key: String, value: &str) -> bool {
        let before = self.seen_before.get(&key).is_some_and(|old| old != value);
        let set_here = self.first_now.entry(key.clone()).or_insert_with(|| value.to_string()) != value;
        self.seen_now.insert(key, value.to_string());
        before || set_here
    }

    /// The scope reference `r`'s values are compared under.
    fn scope_of(&self, r: u32) -> String {
        if r == GLOBALS_REF {
            "G:".into()
        } else if r == COMPONENTS_REF {
            "C:".into()
        } else if (LOCALS_REF..rapidr_session::protocol::CHILDREN_REF).contains(&r) {
            let routine = self.frames.iter().find(|f| LOCALS_REF + f.id == r).map_or("", |f| f.name.as_str());
            format!("L:{routine}:")
        } else {
            self.scope.get(&r).cloned().unwrap_or_else(|| "?:".into())
        }
    }

    /// A list of variables in: each compared with the last stop's, their
    /// children's scope noted.
    fn mark(&mut self, r: u32, list: Vec<Variable>) -> Vec<Shown> {
        let scope = self.scope_of(r);
        list.into_iter()
            .map(|v| {
                if v.reference != 0 {
                    self.scope.insert(v.reference, scope.clone());
                }
                let key = if v.evaluate_name.is_empty() { None } else { Some(format!("{scope}{}", v.evaluate_name.to_ascii_lowercase())) };
                let changed = key.is_some_and(|k| self.changed(k, &v.value));
                (v, changed)
            })
            .collect()
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
        self.watch_values = self.watches.iter().map(|_| (String::new(), String::new(), 0, false, 0)).collect();
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
            let (v, k, r, changed, count) = self.watch_values.get(i).cloned().unwrap_or_default();
            vec![w.clone(), v, k, r.to_string(), flag_text(changed), count.to_string()]
        }))
    }

    /// Whether the snapshot (stack, scopes, watches) is all in.
    fn snapshot_done(&self) -> bool {
        !self.pending.values().any(|w| matches!(w, Want::Stack | Want::Vars(_) | Want::Watch(_)))
    }
}

thread_local! {
    static SESSIONS: RefCell<BTreeMap<String, Model>> = RefCell::new(BTreeMap::new());
    /// A [`poll`] was asked for while the table was in use: the poll that
    /// has it goes round again once it's free.
    static SKIPPED: Cell<bool> = const { Cell::new(false) };
    /// The temporary folder's leftover pictures were removed.
    static SWEPT: Cell<bool> = const { Cell::new(false) };
    /// The programs started so far ([`run_number`]).
    static RUNS: Cell<u64> = const { Cell::new(0) };
}

/// Whether a call on a session is in progress (the table is borrowed for
/// it): a transport's `kill` or `send` can reach the page's own script
/// (the web's Stop), which may call straight back — a message from the
/// program's frame is then not for now ([`poll`] waits its turn).
pub fn in_use() -> bool {
    SESSIONS.with(|s| s.try_borrow_mut().is_err())
}

fn with<R>(name: &str, f: impl FnOnce(&mut Model) -> R) -> R {
    // (Studio's start — its session set up —: the pictures an earlier
    // Studio left, older than a minute, go)
    #[cfg(not(target_arch = "wasm32"))]
    SWEPT.with(|swept| {
        if !swept.replace(true) {
            sweep_pictures(&std::env::temp_dir(), std::time::Duration::from_secs(60));
        }
    });
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
    // (the PATH the program gets: your login shell's, S6 — "" where it
    // isn't changed, Windows and the web)
    match prop {
        "shellpath" => return Some(Value::String(crate::login_path::shell_path().map(|s| s.path).unwrap_or_default().into())),
        "shellpathsource" => return Some(Value::String(crate::login_path::shell_path().map(|s| s.describe()).unwrap_or_default().into())),
        _ => {}
    }
    with(name, |m| {
        let s = &m.session;
        Some(match prop {
            "program" => Value::String(s.program.clone().into()),
            "args" => Value::String(m.args.clone().into()),
            "debug" => flag(s.debug),
            "breakonerror" => flag(s.break_on_error),
            "stoponentry" => flag(s.stop_on_entry),
            "state" => Value::String(s.state().as_str().to_string().into()),
            "currentfile" => Value::String(s.current_file().unwrap_or("").to_string().into()),
            "currentline" => Value::Integer(i64::from(s.current_line())),
            "exitcode" => Value::Integer(i64::from(s.exit_code().unwrap_or(0))),
            "stopreason" => Value::String(s.stop_reason().to_string().into()),
            "stopmessage" => Value::String(rapidr_value::runtime_text::localize_stop(s.stop_description()).into()),
            "error" => Value::String(m.error.clone().into()),
            "frame" => Value::Integer(m.frame as i64),
            "localsref" => Value::Integer(i64::from(m.locals_ref())),
            "globalsref" => Value::Integer(i64::from(GLOBALS_REF)),
            "componentsref" => Value::Integer(i64::from(COMPONENTS_REF)),
            "watches" => Value::String(m.watches.iter().map(|w| format!("{w}\n")).collect()),
            "browserurl" => Value::String(browser::url(name).into()),
            "theme" => Value::String(m.theme.clone().into()),
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
            "state" | "currentfile" | "currentline" | "exitcode" | "error" | "stopreason" | "stopmessage" | "localsref" | "globalsref" | "componentsref" | "browserurl" => {}
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
        // (S6) ReadShellPath: your login shell's PATH read in the background
        // (Studio's start-up); RefreshShellPath: read again now, the PATH
        "readshellpath" => {
            crate::login_path::start();
            Value::Null
        }
        "refreshshellpath" => Value::String(crate::login_path::refresh().map(|s| s.path).unwrap_or_default().into()),
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
                RUNS.with(|n| n.set(n.get() + 1));
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
        // (the program's windows to the front, its app active: Studio's
        // Show Program Window, Continue)
        "showprogram" => flag(with(name, |m| m.session.show_program())),
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
            with(name, |m| {
                table(m.vars.get(&reference).into_iter().flatten().map(|(v, changed)| {
                    vec![v.name.clone(), v.value.clone(), v.kind.clone(), v.reference.to_string(), v.count.to_string(), v.evaluate_name.clone(), flag_text(*changed), v.columns.join("\u{1f}"), v.cells.join("\u{1f}"), v.hint.clone()]
                }))
            })
        }
        "expandmore" => {
            let (reference, start) = (int_arg(args, 0, 0).max(0) as u32, int_arg(args, 1, 0).max(0) as u32);
            with(name, |m| {
                if m.session.state() == State::Paused && reference != 0 {
                    m.ask(Command::Variables { reference, start: Some(start), count: Some(PAGE) }, Want::More(reference));
                }
            });
            Value::Null
        }
        "stepintargets" => {
            let sent = with(name, |m| m.session.state() == State::Paused && m.ask(Command::StepInTargets, Want::Targets));
            if !sent {
                host.fire(name, "onstepintargets", &[Value::String(String::new().into())]);
            }
            Value::Null
        }
        "stepintarget" => {
            let target = int_arg(args, 0, 0).max(0) as u32;
            r(with(name, |m| m.session.step_in_target(target)))
        }
        "setnextstatement" => {
            let (file, line) = (text_arg(args, 0), int_arg(args, 1, 0).max(1) as u32);
            let (id, sent) = with(name, |m| {
                m.next_eval += 1;
                let id = m.next_eval;
                let sent = m.session.state() == State::Paused && m.ask(Command::SetNextStatement { file, line }, Want::Eval { id, refresh: false });
                (id, sent)
            });
            if !sent {
                host.fire(name, "onevaluate", &[Value::Integer(id), Value::String("error: the program isn't paused".into())]);
            }
            Value::Integer(id)
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
                    let context = ["repl", "clipboard", "hover"].into_iter().find(|c| context.eq_ignore_ascii_case(c)).map(str::to_string);
                    Command::Evaluate { expr: text.clone(), frame, context }
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
        // (the eyes and hands on the running program: OnApp when answered)
        "app" => {
            let text = text_arg(args, 0);
            let (id, refused) = with(name, |m| {
                m.next_app += 1;
                let id = m.next_app;
                let request: serde_json::Value = match serde_json::from_str(&text) {
                    Ok(v) => v,
                    Err(e) => return (id, Some(format!("The request isn't JSON: {e}"))),
                };
                if m.session.state() == State::Stopped {
                    return (id, Some("The program isn't running: run it first (run_program), then try again.".to_string()));
                }
                let until = rapidr_ai::rai::now_ms() + app_timeout(&request);
                if m.ask(Command::App { request }, Want::App { id, until }) {
                    (id, None)
                } else {
                    (id, Some("The program couldn't be reached (it is ending?): try again.".to_string()))
                }
            });
            if let Some(why) = refused {
                host.fire(name, "onapp", &[Value::Integer(id), Value::Integer(0), Value::String(why.into()), Value::String(String::new().into())]);
            }
            Value::Integer(id)
        }
        _ => return None,
    })
}

/// A flag as a field: "1" or "0".
fn flag_text(b: bool) -> String {
    (if b { "1" } else { "0" }).to_string()
}

/// Rows as text: one a line, fields tab-separated (tabs and line breaks
/// inside a field as spaces).
fn table(rows: impl Iterator<Item = Vec<String>>) -> Value {
    Value::String(rows.map(|r| r.iter().map(|f| f.replace(['\t', '\n', '\r'], " ")).collect::<Vec<_>>().join("\t") + "\n").collect())
}

/// Every session's news as its events; whether one is running.
pub fn poll<H: Host>(host: H) -> bool {
    // (asked for from inside a call on a session — Stop answered through
    // the page, which calls back at once —: the news waits for the poll
    // that is running, which goes round again when it's done)
    if in_use() {
        SKIPPED.with(|k| k.set(true));
        return true;
    }
    let mut running = poll_once(host);
    while SKIPPED.with(|k| k.replace(false)) {
        running |= poll_once(host);
    }
    running
}

fn poll_once<H: Host>(host: H) -> bool {
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
                SessionEvent::Output { text, .. } => host.fire(&name, "onoutput", &[Value::String(text.into())]),
                SessionEvent::Stopped { reason, file, line, .. } => {
                    // (the snapshot first: OnStopped once it's in)
                    let fire = with(&name, |m| {
                        m.forget();
                        m.held = Some((reason, file.unwrap_or_default(), i64::from(line.unwrap_or(0))));
                        m.ask(Command::Variables { reference: GLOBALS_REF, start: None, count: None }, Want::Vars(GLOBALS_REF));
                        m.ask(Command::Variables { reference: COMPONENTS_REF, start: None, count: None }, Want::Vars(COMPONENTS_REF));
                        if m.ask(Command::StackTrace, Want::Stack) {
                            None
                        } else {
                            m.held.take()
                        }
                    });
                    if let Some((reason, file, line)) = fire {
                        host.fire(&name, "onstopped", &[Value::String(reason.into()), Value::String(file.into()), Value::Integer(line)]);
                    }
                }
                SessionEvent::Continued => {
                    with(&name, Model::forget);
                    host.fire(&name, "oncontinue", &[]);
                }
                SessionEvent::Exited { code } => {
                    with(&name, Model::forget);
                    // (the eyes and hands' requests still waiting: the program ended)
                    for id in with(&name, |m| take_apps(m, f64::INFINITY)) {
                        host.fire(&name, "onapp", &[Value::Integer(id), Value::Integer(0), Value::String(format!("The program ended (exit code {code}) before it answered.").into()), Value::String(String::new().into())]);
                    }
                    host.fire(&name, "onexit", &[Value::Integer(i64::from(code))]);
                }
                SessionEvent::FormShown { id } => host.fire(&name, "onformshown", &[Value::String(id.into())]),
                SessionEvent::Reply { re, body } => reply(host, &name, re, body),
                SessionEvent::FormClosed { .. } => {}
            }
        }
        // (the eyes and hands' requests not answered in time: a program busy
        // in its own code — a long loop without DOEVENTS — answers only when
        // it next waits for its events)
        let late = with(&name, |m| take_apps(m, rapidr_ai::rai::now_ms()));
        for id in late {
            host.fire(&name, "onapp", &[Value::Integer(id), Value::Integer(0), Value::String("The program didn't answer in time: it may be busy running its own code (a long loop) and will answer when it next waits for its events. Wait and try again, or pause it in the debugger to see where it is.".into()), Value::String(String::new().into())]);
        }
    }
    running
}

/// The `App` requests whose time is up by `now` (`INFINITY`: all), taken
/// out: their ids.
fn take_apps(m: &mut Model, now: f64) -> Vec<i64> {
    let late: Vec<(u64, i64)> = m.pending.iter().filter_map(|(seq, w)| match w {
        Want::App { id, until } if *until <= now => Some((*seq, *id)),
        _ => None,
    }).collect();
    for (seq, _) in &late {
        m.pending.remove(seq);
    }
    // (RAI's: answered the same way)
    let rai: Vec<(u64, String)> = m.pending.iter().filter_map(|(seq, w)| match w {
        Want::RaiApp { ticket, until } if *until <= now => Some((*seq, ticket.clone())),
        _ => None,
    }).collect();
    for (seq, ticket) in rai {
        m.pending.remove(&seq);
        let why = if now.is_infinite() { "The program ended before it answered." } else { "The program didn't answer in time: it may be busy running its own code. Wait and try again." };
        rai_answered(ticket, false, why.to_string());
    }
    late.into_iter().map(|(_, id)| id).collect()
}

/// A reply to one of the session's own requests.
fn reply<H: Host>(host: H, name: &str, re: u64, body: EventBody) {
    let mut fire: Vec<(&'static str, Vec<Value>)> = Vec::new();
    // (where the program placed a file's breakpoints: one on a line without
    // code stops at the next line with code; with none after it, never —
    // OnBreakpointPlaced(File, Line, NewLine), 0 for never)
    if let EventBody::Breakpoints { file, breakpoints } = &body {
        for b in breakpoints {
            let at = if b.verified { b.actual_line.unwrap_or(b.line) } else { 0 };
            if at != b.line {
                fire.push(("onbreakpointplaced", vec![Value::String(file.clone().into()), Value::Integer(i64::from(b.line)), Value::Integer(i64::from(at))]));
            }
        }
    }
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
                let shown = m.mark(r, list);
                m.vars.insert(r, shown);
            }
            Want::Watch(i) => {
                let expr = m.watches.get(i).cloned().unwrap_or_default();
                let v = match body {
                    EventBody::Evaluate { result, kind, reference, count, .. } => {
                        let key = format!("W:{}", expr.to_ascii_lowercase());
                        if reference != 0 {
                            m.scope.insert(reference, key.clone() + "/");
                        }
                        let changed = m.changed(key, &result);
                        (result, kind, reference, changed, count)
                    }
                    _ => (format!("error: {}", error.unwrap_or_default()), String::new(), 0, false, 0),
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
                let shown = m.mark(r, list);
                m.vars.insert(r, shown);
                fire.push(("onvariables", vec![Value::Integer(i64::from(r))]));
            }
            Want::More(r) => {
                let list = match body {
                    EventBody::Variables { variables } => variables,
                    _ => Vec::new(),
                };
                let shown = m.mark(r, list);
                m.vars.entry(r).or_default().extend(shown);
                fire.push(("onvariables", vec![Value::Integer(i64::from(r))]));
            }
            Want::RaiApp { ticket, .. } => {
                let (ok, text) = match &body {
                    EventBody::App { result } => (result.get("ok").and_then(serde_json::Value::as_bool).unwrap_or(false), result.get("text").and_then(serde_json::Value::as_str).unwrap_or("").to_string()),
                    _ => (false, error.clone().unwrap_or_else(|| "the program gave no answer".into())),
                };
                rai_answered(ticket, ok, text);
            }
            Want::App { id, .. } => {
                let (ok, text, picture) = match &body {
                    EventBody::App { result } => {
                        let ok = result.get("ok").and_then(serde_json::Value::as_bool).unwrap_or(false);
                        let text = result.get("text").and_then(serde_json::Value::as_str).unwrap_or("").to_string();
                        let picture = result.get("png").and_then(serde_json::Value::as_str).map(|p| app_picture(id, p)).unwrap_or_default();
                        (ok, text, picture)
                    }
                    _ => (false, error.clone().unwrap_or_else(|| "the program gave no answer".into()), String::new()),
                };
                fire.push(("onapp", vec![Value::Integer(id), Value::Integer(if ok { -1 } else { 0 }), Value::String(text.into()), Value::String(picture.into())]));
            }
            Want::Targets => {
                let text = match body {
                    EventBody::StepInTargets { targets } => targets.iter().map(|t| format!("{}\t{}\n", t.id, t.label)).collect(),
                    _ => String::new(),
                };
                fire.push(("onstepintargets", vec![Value::String(text.into())]));
            }
            Want::Eval { id, refresh } => {
                let (text, kind, reference, count, columns) = match body {
                    EventBody::Evaluate { result, kind, reference, count, columns } => (result, kind, reference, count, columns.join("\u{1f}")),
                    EventBody::Ok => (String::new(), String::new(), 0, 0, String::new()),
                    _ => (format!("error: {}", error.unwrap_or_default()), String::new(), 0, 0, String::new()),
                };
                // (an evaluation's children: compared under its own scope)
                if reference != 0 {
                    m.scope.insert(reference, "E:".into());
                }
                fire.push(("onevaluate", vec![Value::Integer(id), Value::String(text.into()), Value::String(kind.into()), Value::Integer(i64::from(reference)), Value::Integer(i64::from(count)), Value::String(columns.into())]));
                // (a statement or an assignment may have changed what's shown)
                if refresh && m.held.is_none() && m.session.state() == State::Paused {
                    m.vars.clear();
                    m.ask(Command::Variables { reference: GLOBALS_REF, start: None, count: None }, Want::Vars(GLOBALS_REF));
                    m.ask(Command::Variables { reference: COMPONENTS_REF, start: None, count: None }, Want::Vars(COMPONENTS_REF));
                    m.refreshing = true;
                    m.fetch_frame();
                }
            }
        }
        if m.snapshot_done() {
            if let Some((reason, file, line)) = m.held.take() {
                fire.push(("onstopped", vec![Value::String(reason.into()), Value::String(file.into()), Value::Integer(line)]));
            } else if std::mem::take(&mut m.refreshing) {
                fire.push(("onvariables", vec![Value::Integer(0)]));
            }
        }
    });
    for (event, args) in fire {
        host.fire(name, event, &args);
    }
}

/// The run of the user's program going on now: its number (each start a
/// new one), `None` when nothing runs. RAI's leave to use the program
/// lasts a run (the chat's "Allow for this run").
pub fn run_number() -> Option<u64> {
    let running = SESSIONS.with(|s| s.try_borrow().ok().is_some_and(|s| s.values().any(|m| m.session.state() != State::Stopped)));
    running.then(|| RUNS.with(Cell::get))
}

/// Whether a session is running (the desktop's loop then looks at it now
/// and then; its transport wakes it when the program says something).
pub fn any_running() -> bool {
    // (in use: a session's call is on the stack — one is running)
    SESSIONS.with(|s| s.try_borrow().map_or(true, |s| s.values().any(|m| m.session.state() != State::Stopped))) || browser::starting()
}

/// Whether Run in Browser's server is still starting (its lines are
/// looked at often until it says where it serves).
pub fn starting() -> bool {
    browser::starting()
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

    use rapidr_value::i18n::{tr_ui, tr_ui_fill};
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
            return Err(tr_ui("no program to run"));
        }
        let rapidr = host.rapidr().ok_or_else(|| tr_ui("Studio runs in a browser already: Run (F5) runs the program here"))?;
        stop(name);
        let mut cmd = Command::new(&rapidr);
        cmd.arg("serve").arg(program).arg("--open").stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
        // (your Terminal's PATH: S6)
        crate::login_path::apply(&mut cmd);
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
                        out.push((name.clone(), format!("{}\n", tr_ui_fill("[Run in Browser ended: exit code {code}]", &[("code", &status.code().unwrap_or(1).to_string())]))));
                    }
                    ended.push(name.clone());
                }
            }
            for n in ended {
                s.remove(&n);
            }
        });
        for (name, text) in out {
            host.fire(&name, "onoutput", &[Value::String(text.into())]);
        }
        starting()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A picture a Studio left behind (a crash, a kill) goes at the next
    /// start once it is a minute old; anything else in the folder stays.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn leftover_pictures_are_swept() {
        let dir = std::env::temp_dir().join(format!("rapidr-sweep-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let old = std::time::SystemTime::now() - std::time::Duration::from_secs(120);
        for (name, aged) in [("rapidr-app-1-1.png", true), ("rapidr-app-1-2.png", false), ("holiday.png", true), ("rapidr-app-1-3.txt", true)] {
            let f = std::fs::File::create(dir.join(name)).unwrap();
            if aged {
                f.set_modified(old).unwrap();
            }
        }
        assert_eq!(sweep_pictures(&dir, std::time::Duration::from_secs(60)), 1);
        let mut left: Vec<String> = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
        left.sort();
        assert_eq!(left, ["holiday.png", "rapidr-app-1-2.png", "rapidr-app-1-3.txt"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A value is changed when it differs from the stop before — or, set
    /// while paused, from what this stop first showed; set back, it isn't.
    #[test]
    fn a_value_set_while_paused_shows_changed() {
        let mut m = Model::default();
        // (the first stop: nothing to compare with)
        assert!(!m.changed("G:total".into(), "0"));
        // (set to 7 at this stop: changed; set back to 0: not)
        assert!(m.changed("G:total".into(), "7"));
        assert!(!m.changed("G:total".into(), "0"));
        assert!(m.changed("G:total".into(), "7"));
        // (the next stop compares with the 7 it went on with)
        m.forget();
        assert!(!m.changed("G:total".into(), "7"));
        assert!(m.changed("G:total".into(), "8"));
    }

    /// The web's Stop (and the program's own end) kills the frame from inside
    /// a call on the session, and the page's script answers by calling
    /// straight back into the wasm (`studio_session_ended`, which polls):
    /// the table is in use, and that poll used to panic ("RefCell already
    /// mutably borrowed": Robert, 2026-10-24, running a form project in the
    /// web Studio). It waits instead: the news is delivered, once.
    #[test]
    fn a_poll_from_inside_the_stop_waits_its_turn() {
        use crate::channel::{self, ChannelTransport};
        use crate::Transport;

        thread_local! {
            static FIRED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
            static ANSWERED: Cell<u32> = const { Cell::new(0) };
        }
        #[derive(Clone, Copy)]
        struct Page;
        impl Host for Page {
            fn fire(self, name: &str, event: &str, _args: &[Value]) {
                FIRED.with(|f| f.borrow_mut().push(format!("{name}.{event}")));
            }
            fn launch(self, _program: &str, _args: &[String], _theme: &str) -> Result<Box<dyn Transport>, String> {
                // (the frame's stop: the page's script calls back at once)
                Ok(Box::new(ChannelTransport::new(
                    |_json| Ok(()),
                    || {
                        ANSWERED.with(|a| a.set(a.get() + 1));
                        assert!(in_use(), "the session's table is in use while the frame stops");
                        poll(Page);
                    },
                )))
            }
            fn list_files(self, _folder: &str) -> Vec<String> {
                Vec::new()
            }
        }
        let start = || call(Page, "Run", "start", &[]).is_some_and(|v| v.to_bool());

        set(Page, "Run", "program", &Value::String("x.rr".into()));
        assert!(start(), "the program starts");
        // (the program ends by itself: its `exited` is the frame's last message)
        channel::incoming(r#"{"type":"exited","code":0}"#);
        poll(Page);
        assert_eq!(ANSWERED.with(Cell::get), 1, "the frame was stopped once, the page answered");
        assert_eq!(FIRED.with(|f| f.borrow().clone()), ["run.onexit"], "OnExit once");
        assert!(!any_running(), "nothing runs after it");
        // (and the user's Stop of a running program: the same call back)
        assert!(start());
        call(Page, "Run", "stop", &[]);
        assert_eq!(ANSWERED.with(Cell::get), 2);
        assert_eq!(FIRED.with(|f| f.borrow().len()), 2, "Stop's OnExit once more");
        assert!(!any_running());
    }

    #[test]
    fn arguments() {
        assert_eq!(split_args(r#"a  "b c" d"#), ["a", "b c", "d"]);
        assert_eq!(split_args(r#""""#), [""]);
        assert!(split_args("  ").is_empty());
    }
}

//! RPROGRAMSESSION: a run of the program under development
//! (docs/ide-components.md §3.9), `rapidr-session`'s client as a component.
//! The program runs where the runtime's [`Host::launch`] puts it — its own
//! process on the desktop (`rapidr run --session`: its forms are real
//! windows; Stop kills it), a sandboxed frame on the web (its forms kernel
//! windows there) — and speaks the session protocol.
//!
//! | Member | |
//! |---|---|
//! | `Program`, `Args`, `Debug`, `BreakOnError` | what to run (a source file), its arguments (one string, as COMMAND$ reads them), under the debugger (default True), stop at a run-time error |
//! | `State` | `stopped`, `running`, `paused` |
//! | `CurrentFile`, `CurrentLine`, `ExitCode`, `Error` | where it is paused; its last exit code; why Start failed |
//! | `StopReason`, `StopMessage` | why it paused (`breakpoint`, `step`, `pause`, `entry`, `exception`); a run-time error's message |
//! | `Start` → True / False, `Stop`, `Pause`, `Continue`, `StepIn`, `StepOver`, `StepOut` | |
//! | `SetBreakpoint(File, Line [, Condition [, HitCount [, LogMessage]]])`, `ClearBreakpoint(File, Line)`, `ClearBreakpoints([File])` | lines from 1 |
//! | `RunToCursor(File, Line)` | on to that line (a stopped program starts) |
//! | `StackTrace`, `Scopes(Frame)`, `Variables(Ref)`, `Properties(Object)` → text | one item a line, its fields tab-separated |
//! | `Evaluate(Expr [, Frame])`, `Execute(Line [, Frame])`, `SetVariable(Name, Value [, Frame])` → text | in a paused frame (the innermost by default) |
//! | `Input(Text)` | a line for the program's INPUT |
//! | `OnOutput(Text)`, `OnStopped(Reason, File, Line)`, `OnContinue`, `OnExit(Code)`, `OnFormShown(Id)` | |

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::time::Duration;

use rapidr_session::protocol::SourceBreakpoint;
use rapidr_session::{ProgramSession, SessionEvent, State};
use rapidr_value::Value;

use crate::{flag, int_arg, text_arg, Host};

#[derive(Default)]
struct Model {
    session: ProgramSession,
    args: String,
    error: String,
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
            "state" => Value::String(s.state().as_str().to_string()),
            "currentfile" => Value::String(s.current_file().unwrap_or("").to_string()),
            "currentline" => Value::Integer(i64::from(s.current_line())),
            "exitcode" => Value::Integer(i64::from(s.exit_code().unwrap_or(0))),
            "stopreason" => Value::String(s.stop_reason().to_string()),
            "stopmessage" => Value::String(s.stop_description().to_string()),
            "error" => Value::String(m.error.clone()),
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
            "state" | "currentfile" | "currentline" | "exitcode" | "error" | "stopreason" | "stopmessage" => {}
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
            let (program, run_args, running) = with(name, |m| (m.session.program.clone(), m.session.args.clone(), m.session.state() != State::Stopped));
            if running {
                return Some(r(Err("the program is running already".into())));
            }
            let started = host.launch(&program, &run_args).and_then(|t| with(name, |m| m.session.start(t)));
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
        "clearbreakpoints" => {
            let file = text_arg(args, 0);
            r(with(name, |m| m.session.clear_breakpoints(if file.is_empty() { None } else { Some(file.as_str()) })))
        }
        "runtocursor" => {
            let (file, line) = (text_arg(args, 0), int_arg(args, 1, 0).max(1) as u32);
            let stopped = with(name, |m| m.session.state() == State::Stopped);
            let res = with(name, |m| m.session.run_to(&file, line));
            if stopped && res.is_ok() {
                return call(host, name, "start", &[]);
            }
            r(res)
        }
        "stacktrace" => {
            let res = with(name, |m| m.session.stack_trace());
            table(res.map(|frames| frames.iter().map(|f| vec![f.id.to_string(), f.name.clone(), f.file.clone().unwrap_or_default(), f.line.to_string()]).collect()))
        }
        "scopes" => {
            let frame = int_arg(args, 0, 0).max(0) as u32;
            let res = with(name, |m| m.session.scopes(frame));
            table(res.map(|scopes| scopes.iter().map(|sc| vec![sc.name.clone(), sc.reference.to_string()]).collect()))
        }
        "variables" => {
            let reference = int_arg(args, 0, 0).max(0) as u32;
            let res = with(name, |m| m.session.variables(reference));
            table(res.map(|vars| vars.iter().map(|v| vec![v.name.clone(), v.value.clone(), v.kind.clone(), v.reference.to_string(), v.count.to_string()]).collect()))
        }
        "properties" => {
            let object = text_arg(args, 0);
            let res = with(name, |m| m.session.properties(&object));
            table(res.map(|(_, props)| props.iter().map(|v| vec![v.name.clone(), v.value.clone(), v.kind.clone()]).collect()))
        }
        "execute" => {
            let (text, frame) = (text_arg(args, 0), frame_arg(args, 1));
            match with(name, |m| m.session.evaluate_in(&text, frame, true)) {
                Ok((v, ..)) => Value::String(v),
                Err(e) => Value::String(format!("error: {e}")),
            }
        }
        "setvariable" => {
            let (var, value, frame) = (text_arg(args, 0), text_arg(args, 1), frame_arg(args, 2));
            match with(name, |m| m.session.set_variable_in(&var, &value, frame)) {
                Ok(v) => Value::String(v),
                Err(e) => Value::String(format!("error: {e}")),
            }
        }
        "clearbreakpoint" => {
            let (file, line) = (text_arg(args, 0), int_arg(args, 1, 0).max(1) as u32);
            r(with(name, |m| m.session.clear_breakpoint(&file, line)))
        }
        "evaluate" => {
            let (expr, frame) = (text_arg(args, 0), frame_arg(args, 1));
            match with(name, |m| m.session.evaluate_in(&expr, frame, false)) {
                Ok((v, ..)) => Value::String(v),
                Err(e) => Value::String(format!("error: {e}")),
            }
        }
        "input" => r(with(name, |m| m.session.input(&text_arg(args, 0)))),
        _ => return None,
    })
}

/// A frame's number from an optional argument (`None`: absent or below 0,
/// the innermost).
fn frame_arg(args: &[Value], i: usize) -> Option<u32> {
    args.get(i).map(Value::to_i64).filter(|&f| f >= 0).map(|f| f as u32)
}

/// Rows as text: one a line, fields tab-separated (tabs and line breaks
/// inside a field as spaces); an error as `error: …`.
fn table(rows: Result<Vec<Vec<String>>, String>) -> Value {
    match rows {
        Ok(rows) => Value::String(
            rows.iter()
                .map(|r| r.iter().map(|f| f.replace(['\t', '\n', '\r'], " ")).collect::<Vec<_>>().join("\t") + "\n")
                .collect(),
        ),
        Err(e) => Value::String(format!("error: {e}")),
    }
}

/// Every session's news as its events; whether one is running.
pub fn poll<H: Host>(host: H) -> bool {
    let names: Vec<String> = SESSIONS.with(|s| s.borrow().keys().cloned().collect());
    let mut running = false;
    for name in names {
        let events = with(&name, |m| {
            let e = m.session.poll(Some(Duration::ZERO));
            (e, m.session.state() != State::Stopped)
        });
        running |= events.1;
        for e in events.0 {
            match e {
                SessionEvent::Output { text, .. } => host.fire(&name, "onoutput", &[Value::String(text)]),
                SessionEvent::Stopped { reason, file, line, .. } => host.fire(
                    &name,
                    "onstopped",
                    &[Value::String(reason), Value::String(file.unwrap_or_default()), Value::Integer(i64::from(line.unwrap_or(0)))],
                ),
                SessionEvent::Continued => host.fire(&name, "oncontinue", &[]),
                SessionEvent::Exited { code } => host.fire(&name, "onexit", &[Value::Integer(i64::from(code))]),
                SessionEvent::FormShown { id } => host.fire(&name, "onformshown", &[Value::String(id)]),
                SessionEvent::FormClosed { .. } | SessionEvent::Reply { .. } => {}
            }
        }
    }
    running
}

/// Whether a session is running (the desktop's loop then wakes often).
pub fn any_running() -> bool {
    SESSIONS.with(|s| s.borrow().values().any(|m| m.session.state() != State::Stopped))
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

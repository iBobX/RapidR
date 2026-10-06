//! `rapidr dap` driven over stdio as an editor drives it: launch, breakpoints
//! in the program and in a file it includes, the stack across files,
//! variables, evaluation, stepping, the program's output, its end; a
//! program that doesn't compile; pause; break on a run-time error; INPUT
//! from the debug console.

use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use rapidr_dap::dap::{read_message, write_message};
use serde_json::{json, Value};

/// How long anything may take (a debug build compiling and running).
const WAIT: Duration = Duration::from_secs(60);

/// A folder of its own for each test, with the program's files.
fn folder(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("dap-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for (file, text) in files {
        std::fs::write(dir.join(file), text).unwrap();
    }
    dir.canonicalize().unwrap()
}

/// `rapidr dap` and what it has said.
struct Dap {
    child: Child,
    stdin: ChildStdin,
    incoming: Receiver<Value>,
    seq: i64,
    /// Events not looked at yet, in order.
    events: Vec<Value>,
    /// Everything the program printed (output events, stdout).
    stdout: String,
    /// Everything on stderr.
    stderr: String,
}

impl Dap {
    fn start(dir: &Path) -> Dap {
        Dap::start_with(dir, &[])
    }

    /// With more environment (the GUI tests' hooks: the program inherits it).
    fn start_with(dir: &Path, env: &[(&str, String)]) -> Dap {
        let mut child = Command::new(env!("CARGO_BIN_EXE_rapidr"))
            .arg("dap")
            .current_dir(dir)
            // (never the real printer, nor the real registry)
            .env("RAPIDR_PRINT_TO", dir.join("printer.out"))
            .env("RAPIDR_REGISTRY", dir.join("registry"))
            .envs(env.iter().map(|(k, v)| (*k, v.as_str())))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("rapidr dap");
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (tx, incoming) = mpsc::channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            while let Ok(Some(message)) = read_message(&mut reader) {
                if tx.send(message).is_err() {
                    return;
                }
            }
        });
        Dap { child, stdin, incoming, seq: 0, events: Vec::new(), stdout: String::new(), stderr: String::new() }
    }

    fn next(&mut self) -> Value {
        let message = self.incoming.recv_timeout(WAIT).expect("rapidr dap said nothing");
        if message["type"] == "event" && message["event"] == "output" {
            let text = message["body"]["output"].as_str().unwrap_or("").to_string();
            match message["body"]["category"].as_str() {
                Some("stderr") => self.stderr.push_str(&text),
                _ => self.stdout.push_str(&text),
            }
        }
        message
    }

    /// Sends a request and returns its response (events meanwhile are
    /// kept).
    fn request(&mut self, command: &str, arguments: Value) -> Value {
        self.seq += 1;
        let seq = self.seq;
        write_message(&mut self.stdin, &json!({ "seq": seq, "type": "request", "command": command, "arguments": arguments })).unwrap();
        loop {
            let message = self.next();
            if message["type"] == "response" && message["request_seq"] == seq {
                assert_eq!(message["command"], command);
                return message;
            }
            self.events.push(message);
        }
    }

    /// A request that must succeed: its body.
    fn ok(&mut self, command: &str, arguments: Value) -> Value {
        let response = self.request(command, arguments);
        assert_eq!(response["success"], true, "{command}: {response}");
        response["body"].clone()
    }

    /// Waits for event `name` (earlier ones are dropped).
    fn event(&mut self, name: &str) -> Value {
        loop {
            let message = if self.events.is_empty() { self.next() } else { self.events.remove(0) };
            if message["type"] == "event" && message["event"] == name {
                return message["body"].clone();
            }
        }
    }

    /// Waits for a stop: (reason, file name, line) of the top frame.
    fn stopped(&mut self) -> (String, String, u64) {
        let body = self.event("stopped");
        let reason = body["reason"].as_str().unwrap().to_string();
        let frames = self.ok("stackTrace", json!({ "threadId": 1 }));
        let top = &frames["stackFrames"][0];
        let file = top["source"]["name"].as_str().unwrap_or("").to_string();
        (reason, file, top["line"].as_u64().unwrap())
    }

    /// The variables of a scope or value, by name.
    fn variables(&mut self, reference: &Value) -> Vec<(String, String)> {
        let body = self.ok("variables", json!({ "variablesReference": reference }));
        body["variables"].as_array().unwrap().iter().map(|v| (v["name"].as_str().unwrap().to_string(), v["value"].as_str().unwrap().to_string())).collect()
    }

    fn launch(&mut self, program: &Path, more: Value) {
        let init = self.ok("initialize", json!({ "adapterID": "rapidr", "linesStartAt1": true, "columnsStartAt1": true }));
        assert_eq!(init["supportsConfigurationDoneRequest"], true);
        let mut args = json!({ "program": program, "cwd": program.parent().unwrap() });
        for (k, v) in more.as_object().unwrap() {
            args[k] = v.clone();
        }
        self.ok("launch", args);
        self.event("initialized");
    }

    /// Waits for the end: the exit code.
    fn exited(&mut self) -> i64 {
        let code = self.event("exited")["exitCode"].as_i64().unwrap();
        self.event("terminated");
        code
    }
}

impl Drop for Dap {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

const MAIN: &str = "$INCLUDE \"util.inc\"
DIM total AS INTEGER
DIM names(3) AS STRING
total = 5
names(1) = \"ann\"
PRINT \"start\"
x = AddUp(2, 3)
PRINT \"x=\"; x
total = total + x
PRINT \"done \"; total
";

const UTIL: &str = "' helpers
FUNCTION AddUp(a AS INTEGER, b AS INTEGER) AS INTEGER
    DIM s AS INTEGER
    s = a + b
    AddUp = s
END FUNCTION
";

#[test]
fn breakpoints_in_two_files_the_stack_variables_and_stepping() {
    let dir = folder("debug", &[("main.bas", MAIN), ("util.inc", UTIL)]);
    let mut dap = Dap::start(&dir);
    dap.launch(&dir.join("main.bas"), json!({}));

    // A breakpoint in the included file, one in the program.
    let placed = dap.ok("setBreakpoints", json!({ "source": { "path": dir.join("util.inc") }, "breakpoints": [{ "line": 4 }] }));
    assert_eq!(placed["breakpoints"][0]["verified"], true);
    assert_eq!(placed["breakpoints"][0]["line"], 4);
    let placed = dap.ok("setBreakpoints", json!({ "source": { "path": dir.join("main.bas") }, "breakpoints": [{ "line": 10 }] }));
    assert_eq!(placed["breakpoints"][0]["verified"], true);
    dap.ok("configurationDone", json!({}));

    // Stopped in AddUp, called from the program's line 7.
    assert_eq!(dap.stopped(), ("breakpoint".into(), "util.inc".into(), 4));
    let frames = dap.ok("stackTrace", json!({ "threadId": 1 }))["stackFrames"].clone();
    assert_eq!(frames.as_array().unwrap().len(), 2, "{frames}");
    assert_eq!(frames[0]["name"], "AddUp");
    assert_eq!(frames[0]["source"]["path"], json!(dir.join("util.inc")));
    assert_eq!((frames[1]["source"]["name"].as_str(), frames[1]["line"].as_u64()), (Some("main.bas"), Some(7)));
    assert_eq!(frames[1]["source"]["path"], json!(dir.join("main.bas")));
    let (top, main) = (frames[0]["id"].clone(), frames[1]["id"].clone());
    assert!(dap.stdout.contains("start"), "{}", dap.stdout);

    // A local with its value, a global.
    let scopes = dap.ok("scopes", json!({ "frameId": top }))["scopes"].clone();
    assert_eq!(scopes[0]["name"], "Locals");
    let locals = dap.variables(&scopes[0]["variablesReference"]);
    assert!(locals.contains(&("a".into(), "2".into())) && locals.contains(&("b".into(), "3".into())), "{locals:?}");
    let globals = dap.variables(&scopes[1]["variablesReference"]);
    assert!(globals.contains(&("total".into(), "5".into())), "{globals:?}");

    // Evaluate: a local, an array element of the program, an array's children.
    assert_eq!(dap.ok("evaluate", json!({ "expression": "a", "frameId": top, "context": "hover" }))["result"], "2");
    assert_eq!(dap.ok("evaluate", json!({ "expression": "names(1)", "frameId": main, "context": "watch" }))["result"], "\"ann\"");
    let names = dap.ok("evaluate", json!({ "expression": "names", "frameId": main, "context": "watch" }));
    let elements = dap.variables(&names["variablesReference"]);
    assert_eq!(elements[1], ("(1)".into(), "\"ann\"".into()));
    // Expressions, evaluated by the VM in the frame (rapidr run --session).
    assert_eq!(dap.ok("evaluate", json!({ "expression": "a + 1", "frameId": top, "context": "watch" }))["result"], "3");
    assert_eq!(dap.ok("evaluate", json!({ "expression": "UCASE$(names(1)) + \"!\"", "frameId": main, "context": "watch" }))["result"], "\"ANN!\"");
    assert_eq!(dap.ok("evaluate", json!({ "expression": "a * b + total", "frameId": top, "context": "repl" }))["result"], "11");
    // A statement in the debug console runs in the frame.
    dap.ok("evaluate", json!({ "expression": "total = total + 0", "frameId": top, "context": "repl" }));
    let failed = dap.request("evaluate", json!({ "expression": "a +", "frameId": top, "context": "watch" }));
    assert_eq!(failed["success"], false);

    // Set a variable: AddUp then returns 4 + 3.
    let set = dap.ok("setVariable", json!({ "variablesReference": scopes[0]["variablesReference"], "name": "a", "value": "4" }));
    assert_eq!(set["value"], "4");

    // Step over, out (back in the program's line 7), over, in.
    dap.ok("next", json!({ "threadId": 1 }));
    assert_eq!(dap.stopped(), ("step".into(), "util.inc".into(), 5));
    assert_eq!(dap.ok("evaluate", json!({ "expression": "s", "context": "hover" }))["result"], "7");
    dap.ok("stepOut", json!({ "threadId": 1 }));
    assert_eq!(dap.stopped(), ("step".into(), "main.bas".into(), 7));
    dap.ok("next", json!({ "threadId": 1 }));
    assert_eq!(dap.stopped(), ("step".into(), "main.bas".into(), 8));
    dap.ok("stepIn", json!({ "threadId": 1 }));
    assert_eq!(dap.stopped(), ("step".into(), "main.bas".into(), 9));

    // On to the breakpoint, then the end.
    dap.ok("continue", json!({ "threadId": 1 }));
    assert_eq!(dap.stopped(), ("breakpoint".into(), "main.bas".into(), 10));
    dap.ok("continue", json!({ "threadId": 1 }));
    assert_eq!(dap.exited(), 0);
    assert!(dap.stdout.contains("x=7") && dap.stdout.contains("done 12"), "{}", dap.stdout);
    dap.ok("disconnect", json!({}));
}

#[test]
fn a_program_that_does_not_compile_says_why() {
    let dir = folder("compile-error", &[("bad.bas", "PRINT \"a\"\nIF THEN\n")]);
    let mut dap = Dap::start(&dir);
    dap.launch(&dir.join("bad.bas"), json!({}));
    dap.ok("configurationDone", json!({}));
    assert_eq!(dap.exited(), 1);
    assert!(dap.stderr.contains("bad.bas:2:") && dap.stderr.contains("error"), "{}", dap.stderr);
}

#[test]
fn pause_stops_a_busy_program() {
    let dir = folder("pause", &[("busy.bas", "n = 0\nDO\n  n = n + 1\nLOOP UNTIL n < 0\n")]);
    let mut dap = Dap::start(&dir);
    dap.launch(&dir.join("busy.bas"), json!({}));
    dap.ok("configurationDone", json!({}));
    std::thread::sleep(Duration::from_millis(300));
    dap.ok("pause", json!({ "threadId": 1 }));
    let (reason, file, line) = dap.stopped();
    assert_eq!((reason.as_str(), file.as_str()), ("pause", "busy.bas"));
    assert!((2..=4).contains(&line), "{line}");
    let n: f64 = dap.ok("evaluate", json!({ "expression": "n", "context": "watch" }))["result"].as_str().unwrap().parse().unwrap();
    assert!(n > 0.0);
    dap.ok("terminate", json!({}));
    dap.event("terminated");
}

#[test]
fn break_on_a_run_time_error_and_input_from_the_console() {
    let dir = folder("error", &[("oops.bas", "INPUT \"name\"; who$\nPRINT \"hi \"; who$\np = 12345\nx = CALLFUNC(p)\nPRINT \"never\"\n")]);
    let mut dap = Dap::start(&dir);
    dap.launch(&dir.join("oops.bas"), json!({}));
    dap.ok("setExceptionBreakpoints", json!({ "filters": ["error"] }));
    dap.ok("configurationDone", json!({}));
    // (the debug console is the program's keyboard while it runs)
    std::thread::sleep(Duration::from_millis(300));
    dap.ok("evaluate", json!({ "expression": "bob", "context": "repl" }));
    let body = dap.event("stopped");
    assert_eq!(body["reason"], "exception");
    assert!(body["text"].as_str().unwrap().to_lowercase().contains("function pointer"), "{body}");
    let frames = dap.ok("stackTrace", json!({ "threadId": 1 }))["stackFrames"].clone();
    assert_eq!((frames[0]["source"]["name"].as_str(), frames[0]["line"].as_u64()), (Some("oops.bas"), Some(4)));
    assert_eq!(dap.ok("evaluate", json!({ "expression": "who$", "context": "hover" }))["result"], "\"bob\"");
    dap.ok("continue", json!({ "threadId": 1 }));
    assert_eq!(dap.exited(), 1);
    assert!(dap.stdout.contains("hi bob") && !dap.stdout.contains("never"), "{}", dap.stdout);
}

/// The GUI tests' headless host (`RAPIDR_CAPTURE`): no desktop needed.
fn headless(dir: &Path, delay: &str) -> Vec<(&'static str, String)> {
    vec![("RAPIDR_CAPTURE", dir.join("capture").to_string_lossy().into_owned()), ("RAPIDR_CAPTURE_DELAY", delay.to_string())]
}

#[test]
fn a_breakpoint_in_an_event_handler_after_main() {
    let program = "SUB Clicked\n  PRINT \"clicked\"\n  Form.Caption = \"done\"\nEND SUB\nCREATE Form AS QFORM\n  Caption = \"Click me\"\n  CREATE Btn AS QBUTTON\n    Caption = \"Go\"\n    OnClick = Clicked\n  END CREATE\nEND CREATE\nForm.Show\nPRINT \"main done\"\n";
    let dir = folder("gui-click", &[("click.bas", program)]);
    let mut env = headless(&dir, "1");
    env.push(("RAPIDR_TEST_EVENTS", "btn.onclick".into()));
    env.push(("RAPIDR_TEST_DUMP", "form.caption".into()));
    let mut dap = Dap::start_with(&dir, &env);
    dap.launch(&dir.join("click.bas"), json!({}));
    dap.ok("setBreakpoints", json!({ "source": { "path": dir.join("click.bas") }, "breakpoints": [{ "line": 2 }] }));
    dap.ok("configurationDone", json!({}));
    assert_eq!(dap.stopped(), ("breakpoint".into(), "click.bas".into(), 2));
    assert!(dap.stdout.contains("main done") && !dap.stdout.contains("clicked"), "{}", dap.stdout);
    assert_eq!(dap.ok("evaluate", json!({ "expression": "Btn.Caption", "context": "hover" }))["result"], "\"Go\"");
    dap.ok("continue", json!({ "threadId": 1 }));
    assert_eq!(dap.exited(), 0);
    assert!(dap.stdout.contains("clicked") && dap.stdout.contains("form.caption=done"), "{}", dap.stdout);
}

#[test]
fn a_breakpoint_in_a_timer_of_a_modal_form_keeps_the_form_modal() {
    let program = "DIM ticks AS INTEGER\nSUB Tick\n  ticks = ticks + 1\n  PRINT \"tick\"; ticks\n  IF ticks >= 3 THEN Form.Close\nEND SUB\nCREATE Form AS QFORM\n  Caption = \"Debug me\"\n  CREATE T AS QTIMER\n    Interval = 50\n    OnTimer = Tick\n  END CREATE\nEND CREATE\nForm.ShowModal\nPRINT \"main done\"\n";
    let dir = folder("gui-modal", &[("modal.bas", program)]);
    let mut dap = Dap::start_with(&dir, &headless(&dir, "120"));
    dap.launch(&dir.join("modal.bas"), json!({}));
    dap.ok("setBreakpoints", json!({ "source": { "path": dir.join("modal.bas") }, "breakpoints": [{ "line": 3 }] }));
    dap.ok("configurationDone", json!({}));
    assert_eq!(dap.stopped(), ("breakpoint".into(), "modal.bas".into(), 3));
    // (the handler runs on top of the main program, waiting in ShowModal)
    let frames = dap.ok("stackTrace", json!({ "threadId": 1 }))["stackFrames"].clone();
    assert_eq!((frames[0]["name"].as_str(), frames[1]["line"].as_u64()), (Some("Tick"), Some(14)));
    assert_eq!(dap.ok("evaluate", json!({ "expression": "Form.Caption", "context": "watch" }))["result"], "\"Debug me\"");
    dap.ok("continue", json!({ "threadId": 1 }));
    assert_eq!(dap.stopped(), ("breakpoint".into(), "modal.bas".into(), 3));
    assert_eq!(dap.ok("evaluate", json!({ "expression": "ticks", "context": "watch" }))["result"], "1");
    // Without breakpoints the timer ticks on until the form closes, and only
    // then does ShowModal return.
    dap.ok("setBreakpoints", json!({ "source": { "path": dir.join("modal.bas") }, "breakpoints": [] }));
    dap.ok("continue", json!({ "threadId": 1 }));
    assert_eq!(dap.exited(), 0);
    let out = dap.stdout.clone();
    let (last_tick, done) = (out.find("tick3").or(out.find("tick 3")), out.find("main done"));
    assert!(last_tick.is_some() && done.is_some() && last_tick < done, "{out}");
}

#[test]
fn break_on_error_inside_an_event_handler_keeps_its_frames() {
    let program = "SUB Clicked\n  DIM n AS INTEGER\n  n = 41\n  p = n + 1\n  x = CALLFUNC(p)\n  PRINT \"never\"\nEND SUB\nCREATE Form AS QFORM\n  CREATE Btn AS QBUTTON\n    OnClick = Clicked\n  END CREATE\nEND CREATE\nForm.Show\n";
    let dir = folder("gui-error", &[("handler.bas", program)]);
    let mut env = headless(&dir, "1");
    env.push(("RAPIDR_TEST_EVENTS", "btn.onclick".into()));
    let mut dap = Dap::start_with(&dir, &env);
    dap.launch(&dir.join("handler.bas"), json!({}));
    dap.ok("setExceptionBreakpoints", json!({ "filters": ["error"] }));
    dap.ok("configurationDone", json!({}));
    let body = dap.event("stopped");
    assert_eq!(body["reason"], "exception");
    // Stopped at the faulting statement, in the handler, its locals there.
    let frames = dap.ok("stackTrace", json!({ "threadId": 1 }))["stackFrames"].clone();
    assert_eq!((frames[0]["name"].as_str(), frames[0]["line"].as_u64()), (Some("Clicked"), Some(5)));
    let top = frames[0]["id"].clone();
    assert_eq!(dap.ok("evaluate", json!({ "expression": "n * 2", "frameId": top, "context": "watch" }))["result"], "82");
    assert_eq!(dap.ok("evaluate", json!({ "expression": "p", "frameId": top, "context": "hover" }))["result"], "42");
    dap.ok("terminate", json!({}));
    dap.event("terminated");
    assert!(!dap.stdout.contains("never"), "{}", dap.stdout);
}

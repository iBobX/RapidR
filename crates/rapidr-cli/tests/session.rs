//! The session protocol end to end on the desktop: the IDE's end
//! (rapidr_session::ProgramSession) driving `rapidr run --session` — a
//! breakpoint in an `$INCLUDE`d file, the stack, evaluation, setting a
//! variable, input, pause on demand, break on a run-time error.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use rapidr_session::{ProgramSession, SessionEvent, State};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rapidr-session-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // (nothing a test runs reaches a real printer or the user's registry)
    std::env::set_var("RAPIDR_PRINT_TO", dir.join("prints"));
    std::env::set_var("RAPIDR_REGISTRY", dir.join("registry.reg"));
    dir
}

fn runtime() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_rapidr"))
}

/// Polls until `want` matches an event (collecting output), or fails.
fn wait_for(s: &mut ProgramSession, output: &mut String, want: impl Fn(&SessionEvent) -> bool) -> SessionEvent {
    let deadline = Instant::now() + Duration::from_secs(60);
    while Instant::now() < deadline {
        for e in s.poll(Some(Duration::from_millis(100))) {
            if let SessionEvent::Output { text, .. } = &e {
                output.push_str(text);
            }
            if want(&e) {
                return e;
            }
        }
    }
    panic!("timed out; output so far: {output:?}");
}

#[test]
fn a_breakpoint_in_an_included_file_stops_and_the_frame_can_be_inspected_and_changed() {
    let dir = scratch("include");
    std::fs::write(dir.join("util.inc"), "FUNCTION Twice(n)\n  Twice = n * 2\nEND FUNCTION\n").unwrap();
    let main = dir.join("main.bas");
    std::fs::write(
        &main,
        "$INCLUDE \"util.inc\"\ntotal = 0\nFOR i = 1 TO 3\n  total = Twice(total + i)\nNEXT\nPRINT \"total=\"; total\nINPUT name$\nPRINT \"hi \"; name$\n",
    )
    .unwrap();
    let mut s = ProgramSession::new(main.to_string_lossy());
    s.set_breakpoint("util.inc", 2, None).unwrap();
    s.start_process(&runtime()).unwrap();
    let mut output = String::new();

    let stop = wait_for(&mut s, &mut output, |e| matches!(e, SessionEvent::Stopped { .. }));
    assert_eq!(stop, SessionEvent::Stopped { reason: "breakpoint".into(), file: Some("util.inc".into()), line: Some(2), description: None });
    assert_eq!((s.state(), s.current_file(), s.current_line()), (State::Paused, Some("util.inc"), 2));

    let frames = s.stack_trace().unwrap();
    let where_: Vec<(&str, Option<&str>, u32)> = frames.iter().map(|f| (f.name.as_str(), f.file.as_deref(), f.line)).collect();
    assert_eq!(where_, [("Twice", Some("util.inc"), 2), ("__main", Some("main.bas"), 4)]);

    assert_eq!(s.evaluate("n").unwrap(), "1");
    assert_eq!(s.evaluate("n * 100 + i").unwrap(), "101");
    assert_eq!(s.set_variable("n", "10").unwrap(), "10");
    assert!(s.evaluate("nosuchfunc(").is_err());

    s.clear_breakpoint("util.inc", 2).unwrap();
    s.continue_().unwrap();
    // 10*2 = 20, (20+2)*2 = 44, (44+3)*2 = 94
    while !output.contains("total=94") {
        wait_for(&mut s, &mut output, |e| matches!(e, SessionEvent::Output { .. }));
    }
    s.input("Ann").unwrap();
    let exit = wait_for(&mut s, &mut output, |e| matches!(e, SessionEvent::Exited { .. }));
    assert_eq!(exit, SessionEvent::Exited { code: 0 });
    assert!(output.contains("hi Ann"), "{output:?}");
    assert_eq!(s.state(), State::Stopped);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn pause_on_demand_then_stop() {
    let dir = scratch("pause");
    let main = dir.join("spin.bas");
    std::fs::write(&main, "PRINT \"go\"\ni = 0\nDO\n  i = i + 1\nLOOP UNTIL i < 0\n").unwrap();
    let mut s = ProgramSession::new(main.to_string_lossy());
    s.start_process(&runtime()).unwrap();
    let mut output = String::new();
    // (running: past its first statement)
    while !output.contains("go") {
        wait_for(&mut s, &mut output, |e| matches!(e, SessionEvent::Output { .. }));
    }
    std::thread::sleep(Duration::from_millis(100));
    s.pause().unwrap();
    let stop = wait_for(&mut s, &mut output, |e| matches!(e, SessionEvent::Stopped { .. }));
    assert!(matches!(&stop, SessionEvent::Stopped { reason, file: Some(f), .. } if reason == "pause" && f == "spin.bas"), "{stop:?}");
    assert!(s.evaluate("i").unwrap().parse::<i64>().unwrap() > 0);
    s.stop();
    assert_eq!(s.state(), State::Stopped);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn break_on_a_runtime_error_then_it_unwinds() {
    let dir = scratch("error");
    let main = dir.join("oops.bas");
    std::fs::write(&main, "z = 0\nSUB Bad\n  y = 5 \\ z\nEND SUB\nPRINT \"before\"\nBad\nPRINT \"after\"\n").unwrap();
    let mut s = ProgramSession::new(main.to_string_lossy());
    s.break_on_error = true;
    s.start_process(&runtime()).unwrap();
    let mut output = String::new();
    let stop = wait_for(&mut s, &mut output, |e| matches!(e, SessionEvent::Stopped { .. }));
    let SessionEvent::Stopped { reason, file, line, description } = stop else { unreachable!() };
    assert_eq!((reason.as_str(), file.as_deref(), line), ("exception", Some("oops.bas"), Some(3)));
    assert!(description.unwrap_or_default().to_lowercase().contains("division"));
    // fix the cause and look again: the error still unwinds (it happened)
    assert_eq!(s.evaluate("z").unwrap(), "0");
    s.continue_().unwrap();
    let exit = wait_for(&mut s, &mut output, |e| matches!(e, SessionEvent::Exited { .. }));
    assert_eq!(exit, SessionEvent::Exited { code: 1 });
    assert!(output.contains("before") && !output.contains("after"), "{output:?}");
    assert!(output.contains("oops.bas line 3"), "{output:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

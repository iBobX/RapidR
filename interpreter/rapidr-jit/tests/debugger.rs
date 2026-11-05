//! The debugger and compiled code (docs/jit-design.md §5): a program run
//! under the debugger with nothing watched runs compiled; a pause asked
//! while it does stops at a line of the loop (or the routine) it was in,
//! with the variables there as values; going on finishes with the answer
//! the interpreter gives. With a breakpoint set, nothing compiled runs.

use std::sync::atomic::Ordering;
use std::time::Duration;

use rapidr_jit::Mode;
use rapidr_value::Value;
use rapidr_vm::{StubHost, Vm, VmError};

fn compile(src: &str) -> rapidr_bytecode::Module {
    let toks = rapidr_lexer::Lexer::new(src, Some("test".into())).tokenize().expect("lex");
    let prog = rapidr_parser::parse_tokens(&toks).expect("parse");
    rapidr_bcgen::compile_program_with_source(&prog, Some(src)).expect("compile").module
}

const LOOP: &str = "DIM i AS INTEGER\nDIM s AS INTEGER\ns = 0\nFOR i = 1 TO 400000000\n  s = s + 1\nNEXT\nPRINT s\n";

/// What a paused run gave: the first answer, the line, the variables, the
/// output after going on, the JIT's report.
type Paused = (Result<(), String>, Option<u32>, Vec<(String, Value)>, String, String);

/// Runs `src` under the debugger (debug mode, nothing watched, the JIT
/// as `mode`), raises the interrupt after `after`, and returns the VM's
/// answer, the line it stopped at, the output after going on, and the
/// JIT's report.
fn pause_and_go_on(src: &str, mode: Mode, after: Duration) -> Paused {
    let m = compile(src);
    let mut h = StubHost::default();
    let mut vm = Vm::new(&mut h);
    let _ = rapidr_jit::install_mode(&mut vm, mode);
    vm.debug_mode = true;
    let flag = vm.interrupt.clone();
    let raiser = std::thread::spawn(move || {
        std::thread::sleep(after);
        flag.store(true, Ordering::Relaxed);
    });
    let first = vm.run(&m).map_err(|e| e.to_string());
    raiser.join().unwrap();
    let line = vm.current_line(&m);
    let top = vm.frames.last().cloned();
    let names = top.as_ref().map(|f| m.functions[f.fn_index as usize].local_names.clone()).unwrap_or_default();
    let mut vars: Vec<(String, Value)> = vm.global_values(&m).map(|(n, v)| (n.to_string(), v.clone())).collect();
    if let Some(f) = &top {
        vars.extend(names.into_iter().zip(f.locals.iter().cloned()));
    }
    let after_pause = if matches!(first, Err(ref e) if e == "paused") { vm.resume(&m).map_err(|e| e.to_string()) } else { first.clone() };
    let report = rapidr_jit::report(&vm).unwrap_or_default();
    drop(vm);
    assert!(after_pause.is_ok(), "{after_pause:?}");
    (first, line, vars, h.output, report)
}

fn var(vars: &[(String, Value)], name: &str) -> Value {
    vars.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, v)| v.clone()).unwrap_or(Value::Null)
}

#[test]
fn a_pause_stops_compiled_code_in_its_loop() {
    let (first, line, vars, out, report) = pause_and_go_on(LOOP, Mode::On, Duration::from_millis(150));
    assert_eq!(first, Err("paused".into()), "{report}");
    // (the FOR line or the line in it: where the interpreter stops)
    assert!(matches!(line, Some(4 | 5)), "stopped at {line:?} ({report})");
    let (i, s) = (var(&vars, "i"), var(&vars, "s"));
    assert!(matches!(i, Value::Integer(n) if n > 10_000 && n < 400_000_000), "i = {i:?} ({report})");
    assert!(matches!(s, Value::Integer(_)), "s = {s:?}");
    assert_eq!(out.trim(), "400000000");
    assert!(report.contains("the debugger"), "compiled code left for the pause: {report}");
}

#[test]
fn a_pause_inside_a_compiled_call() {
    let src = "FUNCTION Twice(x AS INTEGER) AS INTEGER\n  Result = x * 2\nEND FUNCTION\nSUB Work\n  DIM i AS INTEGER, s AS INTEGER\n  FOR i = 1 TO 200000000\n    s = s + Twice(1) - 1\n  NEXT\n  PRINT s\nEND SUB\nWork\n";
    let (first, line, _vars, out, report) = pause_and_go_on(src, Mode::On, Duration::from_millis(150));
    assert_eq!(first, Err("paused".into()), "{report}");
    // (in the loop, or in the FUNCTION: at its first line — the FUNCTION
    // line, where its parameter is converted, as the interpreter stops
    // there too — or its body)
    assert!(matches!(line, Some(1 | 2 | 6 | 7)), "stopped at {line:?} ({report})");
    assert_eq!(out.trim(), "200000000", "{report}");
}

#[test]
fn with_a_breakpoint_nothing_compiled_runs() {
    let src = "DIM i AS INTEGER\nDIM s AS INTEGER\nFOR i = 1 TO 50000\n  s = s + 1\nNEXT\nPRINT s\nPRINT \"end\"\n";
    let m = compile(src);
    let mut h = StubHost::default();
    let mut vm = Vm::new(&mut h);
    let _ = rapidr_jit::install_mode(&mut vm, Mode::Always);
    vm.debug_mode = true;
    vm.add_breakpoint(7);
    assert!(matches!(vm.run(&m), Err(VmError::Paused)));
    assert_eq!(vm.current_line(&m), Some(7));
    let report = rapidr_jit::report(&vm).unwrap_or_default();
    assert!(report.contains("ran 0 time"), "{report}");
    vm.resume(&m).unwrap();
    drop(vm);
    assert_eq!(h.output, "50000\nend\n");
}

#[test]
fn stepping_after_a_pause_goes_on_in_the_interpreter() {
    let m = compile(LOOP);
    let mut h = StubHost::default();
    let mut vm = Vm::new(&mut h);
    let _ = rapidr_jit::install_mode(&mut vm, Mode::On);
    vm.debug_mode = true;
    let flag = vm.interrupt.clone();
    let t = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(150));
        flag.store(true, Ordering::Relaxed);
    });
    assert!(matches!(vm.run(&m), Err(VmError::Paused)));
    t.join().unwrap();
    let i0 = vm.global_values(&m).find(|(n, _)| n.eq_ignore_ascii_case("i")).map(|(_, v)| v.to_i64()).unwrap();
    // (two steps: the line in the loop, then the FOR line again — the
    // counter one more)
    let mut lines = Vec::new();
    for _ in 0..3 {
        assert!(matches!(vm.step_over(&m), Err(VmError::Paused)));
        lines.push(vm.current_line(&m).unwrap());
    }
    assert!(lines.iter().all(|l| *l == 4 || *l == 5), "{lines:?}");
    let i1 = vm.global_values(&m).find(|(n, _)| n.eq_ignore_ascii_case("i")).map(|(_, v)| v.to_i64()).unwrap();
    assert!(i1 == i0 + 1 || i1 == i0 + 2, "{i0} {i1}");
    // (and on to the end: compiled again once nothing is watched)
    vm.resume(&m).unwrap();
    drop(vm);
    assert_eq!(h.output.trim(), "400000000");
}

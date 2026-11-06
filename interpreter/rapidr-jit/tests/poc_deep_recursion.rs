//! PoC (SECURITY-AUDIT-CORE, 2026-11-05), now SEC-50's regression test:
//! compiled recursion must never overrun the native stack.
//!
//! The audit found the engine built the code generator with
//! `enable_probestack` off and no stack limit, and capped compiled-call
//! nesting at `NESTING = 256` frames *by count only*, ignoring each frame's
//! size: a routine with a large frame (many live scalar locals, so a large
//! deopt/exit stack slot) recursing compiled ran off the end of the thread's
//! stack (SIGBUS on a 1 MiB thread, SIGSEGV on 8 MiB), where the interpreter
//! — frames on the heap, `MAX_CALL_DEPTH` — answers.
//!
//! Fixed (docs/jit-design.md §12.7): stack probes in every frame larger than
//! a page, and before each compiled call a check of the stack pointer
//! against the thread's real stack end (plus a margin and the largest
//! compiled frame): below it the interpreter makes the call. The PoC's two
//! cases now end with the interpreter's answer, never a signal. (The same
//! on the real main thread, and a runaway recursion's error: `native_stack.rs`.)
//!
//! The test re-execs itself as a child that runs the program with the JIT
//! forced on (`RAPIDR_JIT=always`) or off on a thread with a bounded stack;
//! the parent checks the child's exit and compares the outputs.

use std::process::Command;

/// A FUNCTION with `k` INTEGER locals, all live across a self-recursive
/// call (so the compiled frame carries them), counting down from `n`; it
/// prints its sum (a real answer to compare).
fn deep_program(k: usize, n: i64) -> String {
    let mut s = String::new();
    s.push_str("FUNCTION deep(n AS INTEGER) AS INTEGER\n");
    for i in 0..k {
        s.push_str(&format!("DIM v{i} AS INTEGER\n"));
    }
    s.push_str("DIM r AS INTEGER\n");
    for i in 0..k {
        s.push_str(&format!("v{i} = n + {i}\n"));
    }
    s.push_str("IF n <= 0 THEN\nResult = 0\nELSE\n");
    s.push_str("r = deep(n - 1)\n");
    // read every local after the recursive call: keeps them live across it
    // (shallow expression trees — sum in chunks — so the parser is happy)
    let mut i = 0;
    while i < k {
        s.push_str("r = r");
        for j in i..(i + 32).min(k) {
            s.push_str(&format!(" + v{j}"));
        }
        s.push('\n');
        i += 32;
    }
    s.push_str("Result = r\nEND IF\nEND FUNCTION\n");
    s.push_str(&format!("PRINT deep({n})\n"));
    s
}

fn child() {
    use rapidr_jit::Mode;
    let k: usize = std::env::var("RAPIDR_POC_LOCALS").ok().and_then(|v| v.parse().ok()).unwrap_or(1200);
    let n: i64 = std::env::var("RAPIDR_POC_DEPTH").ok().and_then(|v| v.parse().ok()).unwrap_or(900);
    let stack_kib: usize = std::env::var("RAPIDR_POC_STACK").ok().and_then(|v| v.parse().ok()).unwrap_or(1024);
    let jit = std::env::var("RAPIDR_POC_JIT").map(|v| v == "always").unwrap_or(true);
    let src = deep_program(k, n);
    let handle = std::thread::Builder::new()
        .stack_size(stack_kib * 1024)
        .spawn(move || {
            use rapidr_vm::{StubHost, Vm};
            let toks = rapidr_lexer::Lexer::new(&src, Some("poc".into())).tokenize().expect("lex");
            let prog = rapidr_parser::parse_tokens(&toks).expect("parse");
            let m = rapidr_bcgen::compile_program_with_source(&prog, Some(&src)).expect("compile").module;
            let mut h = StubHost::default();
            let mut vm = Vm::new(&mut h);
            let _ = rapidr_jit::install_mode(&mut vm, if jit { Mode::Always } else { Mode::Off });
            let r = vm.run(&m);
            let report = rapidr_jit::report(&vm).unwrap_or_default().replace('\n', " | ");
            println!("CHILD-DONE result={r:?} output={:?}", h.output);
            println!("CHILD-REPORT {report}");
        })
        .expect("spawn worker");
    handle.join().expect("worker thread");
}

fn run_child(jit: &str, locals: usize, depth: i64, stack_kib: usize) -> std::process::Output {
    let exe = std::env::current_exe().expect("exe");
    Command::new(exe)
        .args(["--nocapture", "--exact", "compiled_recursion_never_overruns_the_native_stack"])
        .env("RAPIDR_POC", "1")
        .env("RAPIDR_POC_JIT", jit)
        .env("RAPIDR_POC_LOCALS", locals.to_string())
        .env("RAPIDR_POC_DEPTH", depth.to_string())
        .env("RAPIDR_POC_STACK", stack_kib.to_string())
        .output()
        .expect("run child")
}

/// A child's line starting with `tag`.
fn line(o: &std::process::Output, tag: &str) -> String {
    String::from_utf8_lossy(&o.stdout).lines().find(|l| l.starts_with(tag)).unwrap_or("").to_string()
}

/// What ended the child: `None` when it exited by itself, successfully.
fn crash(o: &std::process::Output) -> Option<String> {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(sig) = o.status.signal() {
            return Some(format!("signal {sig}"));
        }
    }
    (!o.status.success()).then(|| format!("{:?}", o.status))
}

/// The PoC's program run both ways in children: the JIT's run must end as
/// the interpreter's did, and must have run compiled code.
fn same_both_ways(locals: usize, depth: i64, stack_kib: usize) {
    let off = run_child("off", locals, depth, stack_kib);
    let on = run_child("always", locals, depth, stack_kib);
    eprintln!("{locals} locals, depth {depth}, a {stack_kib} KiB stack");
    eprintln!("  interpreter: {:?} {}", off.status, line(&off, "CHILD-DONE"));
    eprintln!("  JIT:         {:?} {}", on.status, line(&on, "CHILD-DONE"));
    eprintln!("               {}", line(&on, "CHILD-REPORT"));
    if let Some(c) = crash(&off) {
        panic!("the interpreter should not crash ({c})\n{}", String::from_utf8_lossy(&off.stderr));
    }
    if let Some(c) = crash(&on) {
        panic!("the JIT run crashed ({c}) — SEC-50: compiled code overran the native stack\n{}", String::from_utf8_lossy(&on.stderr));
    }
    assert!(line(&off, "CHILD-DONE").contains("result=Ok(())"), "{}", line(&off, "CHILD-DONE"));
    assert_eq!(line(&on, "CHILD-DONE"), line(&off, "CHILD-DONE"), "the JIT's answer differs from the interpreter's");
    assert!(line(&on, "CHILD-REPORT").contains("routine(s) compiled"), "{}", line(&on, "CHILD-REPORT"));
    // (and it was the stack check, by bytes, that stopped the compiled calls)
    assert!(line(&on, "CHILD-REPORT").contains("at the machine stack's room"), "{}", line(&on, "CHILD-REPORT"));
}

#[test]
fn compiled_recursion_never_overruns_the_native_stack() {
    if std::env::var("RAPIDR_POC").is_ok() {
        child();
        return;
    }
    // A routine with a large frame, recursing past the 256-frame compiled
    // cap, on a 1 MiB-stack worker (a modest but ordinary thread stack) —
    // and with a larger frame on an 8 MiB one (the main thread's size on
    // macOS and Linux). Both crashed before SEC-50's fix. (6,000 locals
    // take the unoptimised code generator minutes to compile: optimised
    // builds only — tools/regress.sh runs the JIT's tests so.)
    same_both_ways(1200, 900, 1024);
    if !cfg!(debug_assertions) {
        same_both_ways(6000, 700, 8 * 1024);
    }
}

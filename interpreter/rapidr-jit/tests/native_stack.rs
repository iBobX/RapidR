//! SEC-50 (docs/security/audit-2026-11-05-core.md, docs/jit-design.md
//! §12.7): compiled code never overruns the machine stack — on the
//! process's real **main thread** too, the one `rapidr run` runs a program
//! on (8 MiB on macOS and Linux, 1 MB on Windows unless the executable asks
//! for more).
//!
//! `harness = false` (Cargo.toml): `main` runs on the main thread itself,
//! and runs each case as a child process (itself again), the interpreter's
//! way and the JIT's (`always`), so a crash is a child's signal the parent
//! reports — never this test dying with the reason lost:
//!
//! - routines with frames of tens of KB (the old cap of 256 compiled frames
//!   by count would have overrun the stack with them) recursing deep: the
//!   interpreter's answer, the stack check by bytes having stopped
//!   compiled calls ("at the machine stack's room");
//! - a runaway recursion on a small worker: the interpreter's "stack
//!   overflow" error, the same text, the same line;
//! - each child says how much machine stack it measured (the JIT's
//!   report), so a run on another system shows its main thread's size.

use std::process::{Command, ExitCode};

/// A FUNCTION with `k` INTEGER locals, all live across its self-recursive
/// call, counting down from `n` (`None`: without end); it prints its sum.
fn program(k: usize, n: Option<i64>) -> String {
    let mut s = String::from("FUNCTION deep(n AS INTEGER) AS INTEGER\n");
    for i in 0..k {
        s.push_str(&format!("DIM v{i} AS INTEGER\n"));
    }
    s.push_str("DIM r AS INTEGER\n");
    for i in 0..k {
        s.push_str(&format!("v{i} = n + {i}\n"));
    }
    // (without end: the test never says so — n only falls)
    s.push_str(if n.is_some() { "IF n <= 0 THEN\n" } else { "IF n > 1 THEN\n" });
    s.push_str("Result = 0\nELSE\nr = deep(n - 1)\n");
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
    s.push_str(&format!("PRINT deep({})\n", n.unwrap_or(1)));
    s
}

/// The child: runs the case named by the environment, on the main thread
/// (`RAPIDR_STACK_KIB` unset) or on a worker of that many KiB.
fn child() {
    let var = |k: &str| std::env::var(k).ok();
    let k: usize = var("RAPIDR_STACK_LOCALS").and_then(|v| v.parse().ok()).unwrap_or(1200);
    let n: Option<i64> = var("RAPIDR_STACK_DEPTH").and_then(|v| v.parse().ok());
    let jit = var("RAPIDR_STACK_JIT").as_deref() == Some("always");
    let src = program(k, n);
    let work = move || {
        use rapidr_vm::{StubHost, Vm};
        let toks = rapidr_lexer::Lexer::new(&src, Some("stack".into())).tokenize().expect("lex");
        let prog = rapidr_parser::parse_tokens(&toks).expect("parse");
        let m = rapidr_bcgen::compile_program_with_source(&prog, Some(&src)).expect("compile").module;
        let mut h = StubHost::default();
        let mut vm = Vm::new(&mut h);
        let _ = rapidr_jit::install_mode(&mut vm, if jit { rapidr_jit::Mode::Always } else { rapidr_jit::Mode::Off });
        let r = vm.run(&m);
        let report = rapidr_jit::report(&vm).unwrap_or_default().replace('\n', " | ");
        println!("CHILD-DONE result={r:?} output={:?}", h.output);
        println!("CHILD-REPORT {report}");
    };
    match var("RAPIDR_STACK_KIB").and_then(|v| v.parse::<usize>().ok()) {
        Some(kib) => std::thread::Builder::new().stack_size(kib << 10).spawn(work).expect("spawn").join().expect("worker"),
        None => work(),
    }
}

struct Run {
    status: std::process::ExitStatus,
    done: String,
    report: String,
    stderr: String,
}

fn run_child(jit: &str, k: usize, n: Option<i64>, kib: Option<usize>) -> Run {
    let mut c = Command::new(std::env::current_exe().expect("exe"));
    c.env("RAPIDR_STACK_CHILD", "1").env("RAPIDR_STACK_JIT", jit).env("RAPIDR_STACK_LOCALS", k.to_string());
    if let Some(n) = n {
        c.env("RAPIDR_STACK_DEPTH", n.to_string());
    }
    if let Some(kib) = kib {
        c.env("RAPIDR_STACK_KIB", kib.to_string());
    }
    let o = c.output().expect("run child");
    let out = String::from_utf8_lossy(&o.stdout).to_string();
    let line = |tag: &str| out.lines().find(|l| l.starts_with(tag)).unwrap_or("").to_string();
    Run { status: o.status, done: line("CHILD-DONE"), report: line("CHILD-REPORT"), stderr: String::from_utf8_lossy(&o.stderr).to_string() }
}

/// What ended a child: `None` when it exited by itself, successfully.
fn crash(r: &Run) -> Option<String> {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(sig) = r.status.signal() {
            return Some(format!("signal {sig}"));
        }
    }
    (!r.status.success()).then(|| format!("{:?}", r.status))
}

/// `"machine stack F KB free, frames up to X KB"` from a report.
fn stack_numbers(report: &str) -> Option<(usize, usize)> {
    let at = report.find("machine stack ")?;
    let rest = &report[at + "machine stack ".len()..];
    let free: usize = rest.split(' ').next()?.parse().ok()?;
    let at = rest.find("frames up to ")?;
    let frame: usize = rest[at + "frames up to ".len()..].split(' ').next()?.parse().ok()?;
    Some((free, frame))
}

/// Case `name`: both ways in children; `Err` says what went wrong.
fn case(name: &str, k: usize, n: Option<i64>, kib: Option<usize>, fails: bool) -> Result<(), String> {
    let off = run_child("off", k, n, kib);
    let on = run_child("always", k, n, kib);
    let thread = kib.map_or("the main thread".to_string(), |k| format!("a {k} KiB worker"));
    eprintln!("{name}: {k} locals, {}, on {thread}", n.map_or("without end".to_string(), |n| format!("depth {n}")));
    eprintln!("  interpreter: {}", off.done);
    eprintln!("  JIT:         {}", on.done);
    eprintln!("               {}", on.report);
    if let Some(c) = crash(&off) {
        return Err(format!("{name}: the interpreter crashed ({c})\n{}", off.stderr));
    }
    if let Some(c) = crash(&on) {
        return Err(format!("{name}: the JIT run crashed ({c}) — compiled code overran the machine stack\n{}", on.stderr));
    }
    if on.done != off.done {
        return Err(format!("{name}: the JIT's run ended otherwise than the interpreter's:\n  {}\n  {}", on.done, off.done));
    }
    if fails != off.done.contains("stack overflow") || fails == off.done.contains("result=Ok(())") {
        return Err(format!("{name}: expected {}: {}", if fails { "the stack overflow error" } else { "an answer" }, off.done));
    }
    if !on.report.contains("routine(s) compiled") {
        return Err(format!("{name}: nothing ran compiled: {}", on.report));
    }
    // (the check by bytes did the stopping — fewer frames than the nesting
    // cap's 256 —, else the case doesn't test it)
    let (free, frame) = stack_numbers(&on.report).ok_or_else(|| format!("{name}: no stack in the report: {}", on.report))?;
    if !on.report.contains("at the machine stack's room") {
        return Err(format!("{name}: no compiled call was stopped by the machine stack's room ({free} KB free, frames up to {frame} KB): {}", on.report));
    }
    Ok(())
}

fn main() -> ExitCode {
    if std::env::var("RAPIDR_STACK_CHILD").is_ok() {
        child();
        return ExitCode::SUCCESS;
    }
    // (`cargo test`'s arguments: this binary is one test, `native_stack`;
    // it has no ignored part)
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--list") {
        println!("native_stack: test");
        return ExitCode::SUCCESS;
    }
    let filters: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    if args.iter().any(|a| a == "--ignored") || (!filters.is_empty() && !filters.iter().any(|f| "native_stack".contains(f.as_str()))) {
        println!("test result: ok. 0 passed; 0 failed; 1 filtered out");
        return ExitCode::SUCCESS;
    }
    // (frames too big for 256 of them in the free stack: ~24 bytes a local
    // per exit; the runaway case's 100,000 interpreted frames of 64 locals
    // are ~200 MB of heap while it runs — its worker small enough for its
    // frames, large enough for an unoptimised VM's own frames and the margin)
    let worker = if cfg!(debug_assertions) { 768 } else { 512 };
    let mut results = vec![case("large frames on the main thread", 2000, Some(600), None, false), case("runaway recursion", 64, None, Some(worker), true)];
    // (6,000 locals take the unoptimised code generator minutes to compile:
    // optimised builds only — tools/regress.sh runs the JIT's tests so)
    if !cfg!(debug_assertions) {
        results.push(case("larger frames on the main thread", 6000, Some(700), None, false));
    }
    let mut failed = 0;
    for r in &results {
        if let Err(e) = r {
            eprintln!("FAILED {e}");
            failed += 1;
        }
    }
    println!("test result: {}. {} passed; {failed} failed", if failed == 0 { "ok" } else { "FAILED" }, results.len() - failed);
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

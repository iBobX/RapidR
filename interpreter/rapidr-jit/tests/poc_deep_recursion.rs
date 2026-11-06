//! PoC (SECURITY-AUDIT-CORE, 2026-11-05): compiled recursion overflows the
//! native stack with NO stack-overflow check.
//!
//! The engine (engine.rs `new_state`) builds the code generator with
//! `enable_probestack` left at its default (false) and sets no stack limit,
//! and caps compiled-call nesting at `NESTING = 256` frames *by count only*
//! (engine.rs), ignoring each frame's size. A routine with a large frame
//! (many live scalar locals, so a large deopt/exit stack slot) that recurses
//! compiled therefore runs up to 256 native frames with no probe. The
//! interpreter, by contrast, keeps its frames on the heap and stops at
//! `MAX_CALL_DEPTH` with a graceful "stack overflow" run-time error.
//!
//! This test re-execs itself as a child that runs the program with the JIT
//! forced on (`RAPIDR_JIT=always`) on a thread with a bounded stack, and the
//! parent observes whether the child died by signal (SIGSEGV/SIGBUS) — a
//! crash the interpreter never takes. `#[ignore]` (it deliberately crashes a
//! child process).

use std::process::Command;

/// A FUNCTION with `k` INTEGER locals, all live across a self-recursive
/// call (so the compiled frame carries them), counting down from `n`.
fn deep_program(k: usize, n: i64) -> String {
    let mut s = String::new();
    s.push_str("FUNCTION deep(n AS INTEGER) AS INTEGER\n");
    for i in 0..k {
        s.push_str(&format!("DIM v{i} AS INTEGER\n"));
    }
    for i in 0..k {
        s.push_str(&format!("v{i} = n + {i}\n"));
    }
    s.push_str("IF n <= 0 THEN\nFUNCTION = 0\nRETURN\nEND IF\n");
    s.push_str("DIM r AS INTEGER\n");
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
    s.push_str("FUNCTION = r\nEND FUNCTION\n");
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
            println!("CHILD-DONE jit={jit} result={r:?} output={:?}", h.output);
        })
        .expect("spawn worker");
    handle.join().expect("worker thread");
}

fn run_child(jit: &str, locals: usize, depth: i64, stack_kib: usize) -> std::process::Output {
    let exe = std::env::current_exe().expect("exe");
    Command::new(exe)
        .args(["--ignored", "--nocapture", "--exact", "compiled_recursion_overflows_native_stack"])
        .env("RAPIDR_POC", "1")
        .env("RAPIDR_POC_JIT", jit)
        .env("RAPIDR_POC_LOCALS", locals.to_string())
        .env("RAPIDR_POC_DEPTH", depth.to_string())
        .env("RAPIDR_POC_STACK", stack_kib.to_string())
        .output()
        .expect("run child")
}

#[test]
#[ignore = "PoC: deliberately crashes a child process (native stack overflow in compiled code)"]
fn compiled_recursion_overflows_native_stack() {
    if std::env::var("RAPIDR_POC").is_ok() {
        child();
        return;
    }
    // A routine with a large frame, recursing past the 256-frame compiled
    // cap, on a 1 MiB-stack worker (a modest but ordinary thread stack).
    let (locals, depth, stack) = (1200usize, 900i64, 1024usize);

    let off = run_child("off", locals, depth, stack);
    let on = run_child("always", locals, depth, stack);

    eprintln!("interpreter (RAPIDR_JIT=off): status={:?}", off.status);
    eprintln!("  stdout: {}", String::from_utf8_lossy(&off.stdout).trim());
    eprintln!("JIT (RAPIDR_JIT=always):      status={:?}", on.status);
    eprintln!("  stdout: {}", String::from_utf8_lossy(&on.stdout).trim());

    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        let off_sig = off.status.signal();
        let on_sig = on.status.signal();
        eprintln!("interpreter signal: {off_sig:?}; JIT signal: {on_sig:?}");
        assert!(off_sig.is_none(), "the interpreter should not crash (it got signal {off_sig:?})");
        assert!(
            on_sig.is_some(),
            "expected the JIT run to crash with a native-stack overflow (no signal seen); \
             status={:?}. If this platform's default frame layout or guard changed, tune \
             RAPIDR_POC_LOCALS / RAPIDR_POC_DEPTH / RAPIDR_POC_STACK.",
            on.status
        );
    }
}

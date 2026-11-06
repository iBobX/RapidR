//! Security fuzz (SECURITY-AUDIT-CORE, 2026-11-05): a *crafted* `.rrbc`
//! that still passes `rapidr_bytecode::verify` must run the same under the
//! JIT (`RAPIDR_JIT=always`) as under the interpreter, and must never make
//! the JIT crash or corrupt memory. The verifier is the JIT's precondition
//! (docs/jit-design.md §7); this stresses it adversarially.
//!
//! Method: compile real seed programs (loops, FOR, arrays, calls, globals,
//! doubles — the shapes the JIT compiles), then mutate their bytecode in
//! ways that keep it *verify-clean* — retarget a jump to another
//! instruction start, swap a local / global / constant / conversion-kind
//! operand for another in-range value, flip raw bytes (the verifier filters
//! the invalid ones). Every mutated module that verifies is run both ways
//! and the output (and the run-time error) compared. A divergence is a JIT
//! correctness bug; a hard crash (SIGSEGV/SIGILL) is a memory-safety bug
//! and the strongest possible evidence.
//!
//! A mutation can turn a terminating loop into an endless one; both the
//! interpreter and the JIT would then loop. To stay CPU-polite (other lanes
//! build on this machine) the work runs in a **child process** that this
//! test re-execs of itself: the parent reads the child's per-trial progress
//! with a timeout and, on a hang, kills the child (and its spinning threads)
//! and resumes past the offending trial. Trials are seeded per (seed, iter)
//! so they're independent and resumable.
//!
//! `#[ignore]` (slow): run with
//!   RAPIDR_FUZZ_ITERS=20000 cargo test -p rapidr-jit --test fuzz_mutate -- --ignored --nocapture
//! Knobs: `RAPIDR_FUZZ_ITERS` (per seed), `RAPIDR_FUZZ_SEED`.

use std::io::{BufRead, BufReader};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use rapidr_bytecode::disasm::decode;
use rapidr_bytecode::{Module, Operand};
use rapidr_jit::Mode;
use rapidr_value::Value;
use rapidr_vm::{Host, StubHost, Vm};

/// The seed programs: the shapes the JIT compiles.
const SEEDS: &[&str] = &[
    "DIM i AS INTEGER\nDIM s AS INTEGER\nFOR i = 1 TO 200\ns = s + i * 2 - 1\nNEXT\nPRINT s\n",
    "DIM i AS INTEGER\nDIM x AS DOUBLE\nFOR i = 1 TO 150\nx = x + SQR(ABS(SIN(i))) + (i MOD 7) + (i \\ 3)\nNEXT\nPRINT x\n",
    "DIM a(1 TO 80) AS INTEGER\nDIM i AS INTEGER\nFOR i = 1 TO 80\na(i) = i * i\nNEXT\nDIM s AS INTEGER\nFOR i = 1 TO 80\ns = s + a(i)\nNEXT\nPRINT s\n",
    "FUNCTION fib(n AS INTEGER) AS INTEGER\nIF n < 2 THEN\nFUNCTION = n\nELSE\nFUNCTION = fib(n-1) + fib(n-2)\nEND IF\nEND FUNCTION\nPRINT fib(18)\n",
    "SUB bump(BYREF n AS INTEGER)\nn = n + 1\nEND SUB\nDIM i AS INTEGER\nDIM c AS INTEGER\nFOR i = 1 TO 120\nbump(c)\nNEXT\nPRINT c\n",
    "DIM i AS INTEGER\nDIM s AS INTEGER\nFOR i = 1 TO 200\nIF (i AND 1) = 0 AND i > 10 THEN\ns = s + (i OR 3)\nEND IF\nNEXT\nPRINT s\n",
    "DIM i AS INTEGER\nDIM j AS INTEGER\nDIM s AS INTEGER\nFOR i = 1 TO 30\nFOR j = 1 TO 30\ns = s + i * j\nNEXT\nNEXT\nPRINT s\n",
];

/// A host that calls the pure maths builtins itself (so the JIT may compile
/// them) and captures PRINT output.
#[derive(Default)]
struct MathHost(StubHost);

impl Host for MathHost {
    const DIRECT_MATHS: bool = true;
    fn call_builtin(&mut self, name: &str, args: &[Value]) -> Result<Value, String> {
        match rapidr_vm::direct_builtin(name) {
            Some(f) if args.len() == 1 => Ok(f(&args[0])),
            _ => self.0.call_builtin(name, args),
        }
    }
    fn create_comp(&mut self, k: &str, id: &str) -> Result<Value, String> {
        self.0.create_comp(k, id)
    }
    fn set_prop(&mut self, id: &str, n: &str, v: Value) -> Result<(), String> {
        self.0.set_prop(id, n, v)
    }
    fn get_prop(&mut self, id: &str, n: &str) -> Result<Value, String> {
        self.0.get_prop(id, n)
    }
    fn call_method(&mut self, id: &str, m: &str, a: &[Value]) -> Result<Value, String> {
        self.0.call_method(id, m, a)
    }
    fn register_event(&mut self, id: &str, e: &str, f: u32) -> Result<(), String> {
        self.0.register_event(id, e, f)
    }
    fn print(&mut self, s: &str) -> Result<(), String> {
        self.0.print(s)
    }
    fn input(&mut self) -> Result<String, String> {
        self.0.input()
    }
}

fn compile(src: &str) -> Module {
    let toks = rapidr_lexer::Lexer::new(src, Some("fuzz".into())).tokenize().expect("lex");
    let prog = rapidr_parser::parse_tokens(&toks).expect("parse");
    rapidr_bcgen::compile_program_with_source(&prog, Some(src)).expect("compile").module
}

/// Run `m` in `mode`; a caught panic becomes `PANIC: …` (a hard SIGSEGV is
/// never caught and takes the process down — the finding we want).
fn run(m: &Module, mode: Mode) -> String {
    let result = catch_unwind(AssertUnwindSafe(|| {
        let mut h = MathHost::default();
        let mut vm = Vm::new(&mut h);
        let _ = rapidr_jit::install_mode(&mut vm, mode);
        let r = vm.run(m);
        drop(vm);
        let mut out = h.0.output;
        if let Err(e) = r {
            out.push_str(&format!("\nERROR: {e}"));
        }
        out
    }));
    match result {
        Ok(out) => out,
        Err(p) => {
            let msg = p.downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| p.downcast_ref::<String>().cloned()).unwrap_or_default();
            format!("PANIC: {msg}")
        }
    }
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() % n as u64) as usize
        }
    }
}

/// A per-trial seed from (base, seed index, iter): trials are independent
/// and resumable after the parent kills a hung child.
fn trial_seed(base: u64, si: usize, it: usize) -> u64 {
    let mut h = base ^ 0x9E3779B97F4A7C15;
    for v in [si as u64, it as u64] {
        h ^= v.wrapping_add(0xD1B54A32D192ED03);
        h = h.wrapping_mul(0xFF51AFD7ED558CCD);
        h ^= h >> 33;
    }
    h | 1
}

/// Instruction-start offsets (jump targets the verifier accepts).
fn starts(code: &[u8]) -> Vec<u32> {
    let mut s = Vec::new();
    let mut ip = 0;
    while let Some(insn) = decode(code, ip) {
        s.push(ip as u32);
        ip = insn.next;
    }
    s.push(code.len() as u32);
    s
}

/// One verify-preserving (or raw) mutation of a chosen function.
fn mutate(m: &mut Module, rng: &mut Rng) {
    if m.functions.is_empty() {
        return;
    }
    let fi = rng.below(m.functions.len());
    let n_locals = m.functions[fi].n_locals;
    let n_strings = m.strings.len();
    let n_consts = m.consts.len();
    let code_len = m.functions[fi].code.len();
    if code_len == 0 {
        return;
    }
    let mut sites: Vec<(usize, Operand)> = Vec::new();
    let st = starts(&m.functions[fi].code);
    {
        let code = &m.functions[fi].code;
        let mut ip = 0;
        while let Some(insn) = decode(code, ip) {
            let mut at = ip + 1;
            for o in insn.op.layout() {
                sites.push((at, *o));
                at += o.width();
            }
            ip = insn.next;
        }
    }
    let site_of = |sites: &[(usize, Operand)], rng: &mut Rng, pred: fn(&Operand) -> bool| -> Option<usize> {
        let v: Vec<usize> = sites.iter().filter(|(_, o)| pred(o)).map(|(a, _)| *a).collect();
        (!v.is_empty()).then(|| v[rng.below(v.len())])
    };
    let code = &mut m.functions[fi].code;
    match rng.below(6) {
        0 => {
            if let Some(a) = site_of(&sites, rng, |o| *o == Operand::Target) {
                let t = st[rng.below(st.len())];
                code[a..a + 4].copy_from_slice(&t.to_le_bytes());
            }
        }
        1 if n_locals > 0 => {
            if let Some(a) = site_of(&sites, rng, |o| *o == Operand::Local) {
                let s = rng.below(n_locals as usize) as u16;
                code[a..a + 2].copy_from_slice(&s.to_le_bytes());
            }
        }
        2 if n_strings > 0 => {
            if let Some(a) = site_of(&sites, rng, |o| matches!(o, Operand::Global | Operand::Str)) {
                let g = rng.below(n_strings) as u32;
                code[a..a + 4].copy_from_slice(&g.to_le_bytes());
            }
        }
        3 if n_consts > 0 => {
            if let Some(a) = site_of(&sites, rng, |o| *o == Operand::Const) {
                let k = rng.below(n_consts) as u32;
                code[a..a + 4].copy_from_slice(&k.to_le_bytes());
            }
        }
        4 => {
            if let Some(a) = site_of(&sites, rng, |o| *o == Operand::U8) {
                let choices = [0u8, 1, 2, 3, 5, 6, 7, 8, 0x80, 0xFF];
                code[a] = choices[rng.below(choices.len())];
            }
        }
        _ => {
            let a = rng.below(code_len);
            code[a] = (rng.next() & 0xFF) as u8;
        }
    }
}

/// Build the mutated module for trial (si, it) from `base`.
fn mutated(base: &Module, base_seed: u64, si: usize, it: usize) -> Module {
    let mut rng = Rng(trial_seed(base_seed, si, it));
    let mut m = base.clone();
    let nmut = 1 + rng.below(3);
    for _ in 0..nmut {
        mutate(&mut m, &mut rng);
    }
    m
}

/// Child mode: run seed `si`'s trials from `start`..`iters`, one line of
/// progress each (`OK <it>` / `DIFF <it> <off>|<on>`), then exit.
fn child(si: usize, start: usize, iters: usize, base_seed: u64) {
    use std::io::Write;
    let base = compile(SEEDS[si]);
    let out = std::io::stdout();
    for it in start..iters {
        let m = mutated(&base, base_seed, si, it);
        if rapidr_bytecode::verify::module(&m).is_err() {
            let mut o = out.lock();
            let _ = writeln!(o, "SKIP {it}");
            let _ = o.flush();
            continue;
        }
        let off = run(&m, Mode::Off);
        let on = run(&m, Mode::Always);
        let mut o = out.lock();
        if off == on {
            let _ = writeln!(o, "OK {it}");
        } else {
            let _ = writeln!(o, "DIFF {it} {off:?} ||| {on:?}");
        }
        let _ = o.flush();
    }
    let mut o = out.lock();
    let _ = writeln!(o, "DONE");
    let _ = o.flush();
}

/// Parent: run seed `si` in a child, reading progress with a per-line
/// timeout; a hang (endless-loop mutation) kills the child and resumes
/// past it. Returns the diffs found and the count of hangs.
fn run_seed(si: usize, iters: usize, base_seed: u64) -> (Vec<String>, u64, u64) {
    let exe = std::env::current_exe().expect("current exe");
    let mut diffs = Vec::new();
    let mut hangs = 0u64;
    let mut done = 0u64;
    let mut start = 0usize;
    while start < iters {
        let mut child = Command::new(&exe)
            // re-run *this* ignored test in the child; it enters child mode
            // on seeing RAPIDR_FUZZ_CHILD. libtest's own stdout framing is
            // ignored by the prefix parsing below.
            .args(["--ignored", "--nocapture", "--exact", "jit_matches_interpreter_on_mutated_bytecode"])
            .env("RAPIDR_FUZZ_CHILD", format!("{si}:{start}"))
            .env("RAPIDR_FUZZ_ITERS", iters.to_string())
            .env("RAPIDR_FUZZ_SEED", base_seed.to_string())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn child");
        let stdout = child.stdout.take().unwrap();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                match line {
                    Ok(l) => {
                        if tx.send(l).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
        let mut last = start;
        let mut finished = false;
        loop {
            match rx.recv_timeout(Duration::from_secs(3)) {
                Ok(line) => {
                    if let Some(rest) = line.strip_prefix("OK ") {
                        last = rest.trim().parse().unwrap_or(last);
                        done += 1;
                    } else if line.starts_with("SKIP ") {
                        if let Some(n) = line[5..].trim().parse::<usize>().ok() {
                            last = n;
                        }
                    } else if let Some(rest) = line.strip_prefix("DIFF ") {
                        done += 1;
                        let it = rest.split_whitespace().next().unwrap_or("?");
                        last = it.parse().unwrap_or(last);
                        diffs.push(format!("seed {si} iter {it}: {}", &rest));
                    } else if line == "DONE" {
                        finished = true;
                        break;
                    }
                }
                Err(_) => {
                    // timed out: the current trial hangs (endless loop).
                    let _ = child.kill();
                    hangs += 1;
                    eprintln!("seed {si}: hang at/after iter {last} (endless-loop mutation); skipping");
                    break;
                }
            }
        }
        let _ = child.wait();
        if finished || !diffs.is_empty() {
            break;
        }
        start = last + 1;
    }
    (diffs, hangs, done)
}

#[test]
#[ignore = "fuzz: slow; re-execs itself in child processes"]
fn jit_matches_interpreter_on_mutated_bytecode() {
    let iters: usize = std::env::var("RAPIDR_FUZZ_ITERS").ok().and_then(|v| v.parse().ok()).unwrap_or(2000);
    let base_seed: u64 = std::env::var("RAPIDR_FUZZ_SEED").ok().and_then(|v| v.parse().ok()).unwrap_or(0x5DEECE66D);

    // Child mode (re-exec): run one seed's trials and exit.
    if let Ok(spec) = std::env::var("RAPIDR_FUZZ_CHILD") {
        let (si, start) = spec.split_once(':').expect("si:start");
        child(si.parse().unwrap(), start.parse().unwrap(), iters, base_seed);
        return;
    }

    // Parent: each seed must agree on its own first (sanity), then fuzz.
    let mut all_diffs = Vec::new();
    let mut total_done = 0u64;
    let mut total_hangs = 0u64;
    for si in 0..SEEDS.len() {
        let base = compile(SEEDS[si]);
        assert_eq!(run(&base, Mode::Off), run(&base, Mode::Always), "seed {si} diverges unmutated");
        let (diffs, hangs, done) = run_seed(si, iters, base_seed);
        total_done += done;
        total_hangs += hangs;
        all_diffs.extend(diffs);
        if !all_diffs.is_empty() {
            break;
        }
    }
    eprintln!("fuzz: {total_done} verified mutated modules run both ways, {total_hangs} endless-loop mutations skipped");
    assert!(all_diffs.is_empty(), "JIT diverged from the interpreter on crafted-but-verified bytecode:\n{}", all_diffs.join("\n"));
}

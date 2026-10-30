//! The string hot paths of a program that parses binary data stay linear on
//! the VM: reading a long string a character at a time (`ASC(MID$(b, i,
//! 1))`, `b[i]`, `LEN(b)` in the loop's test) and building one a piece at a
//! time (`s = s + CHR$(x)`, `s = s & x`). Each was quadratic before
//! STRING-SPEED (2026-10-29): every MID$ copied the whole string twice and
//! made a 4-bytes-a-character copy of it — 11.8 s for a 64 KB string — and
//! every `s = s + x` copied all of `s`.
//!
//! Four times the data must take about four times as long: the test fails
//! when it takes 8 times as long (quadratic is 16). Times are the CPU time
//! of the run (steady on a machine busy with other builds), the fastest of
//! a few runs.

use rapidr_vm::Vm;
use rapidr_vm_host_native::NativeHost;
use std::cell::RefCell;
use std::rc::Rc;

/// CPU seconds this thread has used (wall time where there's no such clock).
fn cpu_now() -> f64 {
    #[cfg(unix)]
    {
        let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
        // SAFETY: `ts` is a valid timespec for clock_gettime to fill.
        if unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut ts) } == 0 {
            return ts.tv_sec as f64 + ts.tv_nsec as f64 * 1e-9;
        }
    }
    use std::sync::OnceLock;
    static START: OnceLock<std::time::Instant> = OnceLock::new();
    START.get_or_init(std::time::Instant::now).elapsed().as_secs_f64()
}

/// Runs `src` on the VM: what it printed, and the CPU seconds it took.
fn run(src: &str) -> (String, f64) {
    let tokens = rapidr_lexer::Lexer::new(src, Some("string_speed.rr".into())).tokenize().expect("lex");
    let program = rapidr_parser::parse_tokens(&tokens).expect("parse");
    let module = rapidr_bcgen::compile_program(&program).expect("compile").module;
    let out = Rc::new(RefCell::new(String::new()));
    let sink = out.clone();
    let mut host = NativeHost::default();
    host.output = Some(Box::new(move |s: &str| sink.borrow_mut().push_str(s)));
    let t0 = cpu_now();
    Vm::new(&mut host).run(&module).expect("run");
    let took = cpu_now() - t0;
    let text = out.borrow().clone();
    (text, took)
}

/// The fastest of `runs` runs of `program(n)` — its output must be the
/// same each time.
fn fastest(program: &dyn Fn(usize) -> String, n: usize, runs: usize) -> (String, f64) {
    let mut best: Option<(String, f64)> = None;
    for _ in 0..runs {
        let (out, t) = run(&program(n));
        if let Some((o, _)) = &best {
            assert_eq!(*o, out);
        }
        if best.as_ref().is_none_or(|(_, b)| t < *b) {
            best = Some((out, t));
        }
    }
    best.unwrap()
}

/// `program` on n and 4n: the time grows by less than 8 times (linear: 4).
fn check_linear(what: &str, program: &dyn Fn(usize) -> String, n: usize) {
    let (small_out, small) = fastest(program, n, 5);
    let (big_out, big) = fastest(program, 4 * n, 5);
    let ratio = big / small.max(1e-6);
    eprintln!("{what}: {n} → {small:.4} s, {} → {big:.4} s, ratio {ratio:.1} (output {small_out:?} / {big_out:?})", 4 * n);
    assert!(ratio < 8.0, "{what}: 4 times the data took {ratio:.1} times as long ({small:.4} s → {big:.4} s): quadratic again?");
}

/// A long string with characters of every width: bytes CHR$(0..255) (one or
/// two bytes in UTF-8), é, €, an emoji — built a piece at a time.
fn build(n: usize) -> String {
    format!(
        "DIM b AS STRING, i AS INTEGER, s AS INTEGER\n\
         FOR i = 1 TO {}\n\
           b = b + CHR$(i MOD 256)\n\
           b = b & \"é\"\n\
           b = b + \"€a\"\n\
           b = b + \"😀\"\n\
         NEXT\n",
        n / 5
    )
}

#[test]
fn reading_a_long_string_a_character_at_a_time_is_linear() {
    let scan = |n: usize| {
        format!(
            "{}s = 0\n\
             FOR i = 1 TO LEN(b)\n\
               s = s + ASC(MID$(b, i, 1))\n\
             NEXT\n\
             i = 1\n\
             WHILE i <= LEN(b)\n\
               s = s + ASC(b[i])\n\
               i = i + 1\n\
             WEND\n\
             PRINT LEN(b); \" \"; s\n",
            build(n)
        )
    };
    check_linear("MID$ / b[i] / LEN scan", &scan, 16_000);
    // (the answer: every character's code, twice)
    let (out, _) = run(&scan(50));
    let one: i64 = (1..=10).map(|i| (i % 256) + 233 + 8364 + 97 + 128512).sum();
    assert_eq!(out.trim(), format!("50 {}", 2 * one));
}

#[test]
fn building_a_long_string_a_piece_at_a_time_is_linear() {
    let grow = |n: usize| format!("{}PRINT LEN(b)\n", build(n));
    check_linear("s = s + x", &grow, 16_000);
    let (out, _) = run(&grow(50));
    assert_eq!(out.trim(), "50");
}

#[test]
fn a_string_another_variable_shares_keeps_its_text() {
    let (out, _) = run(
        "DIM a AS STRING, k AS STRING\n\
         a = STRING$(100, \"x\")\n\
         k = a\n\
         a = a + \"y\"\n\
         a = a & \"z\"\n\
         a = a + a\n\
         PRINT LEN(a); \" \"; LEN(k); \" \"; RIGHT$(a, 2); \" \"; RIGHT$(k, 1)\n",
    );
    assert_eq!(out.trim(), "204 100 yz x");
}

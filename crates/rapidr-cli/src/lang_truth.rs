//! `rapidr lang truth`: the language reference tells the truth (docs/preview4-plan.md K7).
//!
//! ```text
//! rapidr lang truth [--verbose] [--list] [--emit DIR] [NAME…]
//! ```
//!
//! The registry's checks (crates/rapidr-lang/src/truth.rs) made into
//! programs: every `syntax` line of a statement, a directive and a builtin
//! must compile; every `example` must compile; an example with an `expect`
//! facet is run (the interpreter, in a scratch folder, with no stdin, a time
//! limit, and the settings, printer and key store pointed at scratch ones)
//! and must print exactly that. It also fails for an argument-order trap
//! without an `expect` in the covered families. Exit 1 on any failure.
//!
//! `--verbose` lists what is skipped and why; `--list` prints the cases and
//! checks nothing; `--emit DIR` writes each case as a `.bas` (and an
//! `.expected` when it has one) for the other runtimes' runners; names
//! (`RDATAFRAME`, `statement FOR`, `MID$`) limit it to the entries
//! containing them.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::{Duration, Instant};

use rapidr_lang::truth::{self, Case, Run, What};

/// A program may run this long.
const LIMIT: Duration = Duration::from_secs(20);

pub fn command(args: &[String]) -> ExitCode {
    let flag = |f: &str| args.iter().any(|a| a == f);
    let mut names: Vec<String> = Vec::new();
    let mut emit: Option<&str> = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--verbose" | "--list" => {}
            "--emit" => emit = it.next().map(String::as_str),
            o if o.starts_with("--") => {
                eprintln!("rapidr lang truth: unknown option {o} (--verbose, --list, --emit DIR)");
                return ExitCode::from(2);
            }
            n => names.push(n.to_ascii_lowercase()),
        }
    }
    let plan = truth::plan();
    let wanted = |entry: &str| names.is_empty() || names.iter().any(|n| entry.to_ascii_lowercase().contains(n));
    let cases: Vec<&Case> = plan.cases.iter().filter(|c| wanted(&c.entry)).collect();
    if flag("--list") {
        for c in &cases {
            println!("{} [{}] {}", c.entry, if c.what == What::Syntax { "syntax" } else { "example" }, c.text.replace('\n', " / "));
        }
        return ExitCode::SUCCESS;
    }
    if let Some(dir) = emit {
        return emit_cases(&cases, Path::new(dir));
    }
    let scratch = std::env::temp_dir().join(format!("rapidr-truth-{}", std::process::id()));
    if let Err(e) = fs::create_dir_all(&scratch) {
        eprintln!("rapidr lang truth: {}: {e}", scratch.display());
        return ExitCode::from(1);
    }
    let me = std::env::current_exe().ok();
    let mut failures: Vec<String> = Vec::new();
    let (mut syntax, mut examples, mut ran) = (0, 0, 0);
    for (n, c) in cases.iter().enumerate() {
        match c.what {
            What::Syntax => syntax += 1,
            What::Example => examples += 1,
        }
        match check_case(c, &scratch, n, me.as_deref()) {
            Ok(did_run) => ran += usize::from(did_run),
            Err(why) => failures.push(format!("{} ({}): `{}`\n    {why}", c.entry, if c.what == What::Syntax { "syntax" } else { "example" }, c.text.replace('\n', " / "))),
        }
    }
    let _ = fs::remove_dir_all(&scratch);
    for f in &failures {
        println!("FAIL  {f}");
    }
    for (entry, what) in plan.problems.iter().filter(|(e, _)| wanted(e)) {
        println!("FAIL  {entry}: {what}");
        failures.push(entry.clone());
    }
    if flag("--verbose") {
        for (entry, why) in plan.skipped.iter().filter(|(e, _)| wanted(e)) {
            println!("skip  {entry}: {why}");
        }
    }
    println!(
        "truth: {} programs from syntax lines and {} from examples compile ({} of them run and print what `expect` says); {} skipped; argument-order traps: {} in the covered families ({}), {} elsewhere{}",
        syntax,
        examples,
        ran,
        plan.skipped.len(),
        plan.traps_covered,
        truth::COVERED,
        plan.traps_elsewhere,
        if failures.is_empty() { "" } else { " — FAILED" }
    );
    if failures.is_empty() { ExitCode::SUCCESS } else { ExitCode::from(1) }
}

/// Compiles a case (and runs it, when it has an `expect`); true when it ran.
fn check_case(c: &Case, scratch: &Path, n: usize, me: Option<&Path>) -> Result<bool, String> {
    let mut first_error = String::new();
    let mut compiled: Option<PathBuf> = None;
    for (k, form) in c.forms.iter().enumerate() {
        let src = scratch.join(format!("c{n}_{k}.{}", if c.rapidr { "rr" } else { "bas" }));
        fs::write(&src, form).map_err(|e| format!("{}: {e}", src.display()))?;
        match compile(&src) {
            Ok(bytes) => {
                let out = src.with_extension("rrbc");
                fs::write(&out, bytes).map_err(|e| e.to_string())?;
                compiled = Some(out);
                break;
            }
            Err(e) if first_error.is_empty() => first_error = e,
            Err(_) => {}
        }
    }
    let Some(program) = compiled else { return Err(first_error) };
    let Run::Execute(expect) = &c.run else { return Ok(false) };
    let me = me.ok_or("can't find the rapidr executable to run it")?;
    let got = run_program(me, &program, scratch)?;
    let (want, got_n) = (normalize(expect), normalize(&got));
    if want != got_n {
        return Err(format!("it prints\n{}\n    but the registry says it prints\n{}", indent(&got_n), indent(&want)));
    }
    Ok(true)
}

fn indent(s: &str) -> String {
    s.lines().map(|l| format!("        {l}")).collect::<Vec<_>>().join("\n")
}

fn normalize(s: &str) -> String {
    s.replace("\r\n", "\n").lines().map(str::trim_end).collect::<Vec<_>>().join("\n").trim_end().to_string()
}

/// The program as bytecode, or what the compiler said (its first line).
fn compile(src: &Path) -> Result<Vec<u8>, String> {
    let path = src.to_string_lossy().into_owned();
    let result = std::panic::catch_unwind(|| crate::compile_to_bytecode(&path));
    match result {
        Ok(Ok(compiled)) => Ok(compiled.module.to_bytes()),
        Ok(Err(e)) => Err(e.lines().next().unwrap_or("").replace(&format!("{path}:"), "line ")),
        Err(_) => Err("the compiler panicked".into()),
    }
}

/// Runs bytecode in the scratch folder and returns what it printed.
fn run_program(me: &Path, program: &Path, scratch: &Path) -> Result<String, String> {
    let mut child = Command::new(me)
        .arg("run-bc")
        .arg(program)
        .current_dir(scratch)
        .env("RAPIDR_REGISTRY", scratch.join("registry.reg"))
        .env("RAPIDR_PRINT_TO", scratch.join("prints"))
        .env("RAPIDR_AI_KEYSTORE", "memory")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("can't run it: {e}"))?;
    // (both pipes are read on threads: a program that prints a lot can't fill one and wait)
    let mut out = child.stdout.take().unwrap();
    let mut err = child.stderr.take().unwrap();
    let t_out = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = out.read_to_string(&mut s);
        s
    });
    let t_err = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = err.read_to_string(&mut s);
        s
    });
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) if start.elapsed() > LIMIT => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("it didn't finish in {} seconds", LIMIT.as_secs()));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(5)),
            Err(e) => return Err(e.to_string()),
        }
    };
    let (stdout, stderr) = (t_out.join().unwrap_or_default(), t_err.join().unwrap_or_default());
    if !status.success() {
        return Err(format!("it stopped with an error: {}", stderr.trim().lines().next().unwrap_or("")));
    }
    // (a runtime that doesn't answer a member says so on stderr)
    if let Some(w) = stderr.lines().find(|l| l.contains("not implemented") || l.contains("no such method")) {
        return Err(format!("the runtime says: {}", w.trim()));
    }
    Ok(stdout)
}

/// `--emit DIR`: each case as a program (the first form that compiles is the
/// runners' to find; the first is written) and its expected output.
fn emit_cases(cases: &[&Case], dir: &Path) -> ExitCode {
    if let Err(e) = fs::create_dir_all(dir) {
        eprintln!("{}: {e}", dir.display());
        return ExitCode::from(1);
    }
    let mut written = 0;
    for (n, c) in cases.iter().enumerate() {
        let Run::Execute(expect) = &c.run else { continue };
        let base: String = c.entry.chars().map(|ch| if ch.is_ascii_alphanumeric() { ch.to_ascii_lowercase() } else { '_' }).collect();
        let path = dir.join(format!("{base}_{n}"));
        if fs::write(path.with_extension(if c.rapidr { "rr" } else { "bas" }), &c.forms[0]).and_then(|_| fs::write(path.with_extension("expected"), format!("{expect}\n"))).is_err() {
            eprintln!("{}: can't write", path.display());
            return ExitCode::from(1);
        }
        written += 1;
    }
    println!("{written} programs with their expected output in {}", dir.display());
    ExitCode::SUCCESS
}

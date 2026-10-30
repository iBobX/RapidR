//! RapidR's crash reports (rapidr-crash): `rapidr run` of a program that
//! RapidR crashes on purpose (the test hook `RAPIDR_TEST_CRASH`, a real
//! panic, a real stack overflow) writes a report into the crash folder the
//! test gives (`RAPIDR_CRASH_DIR` — never the user's): RapidR's version and
//! commit, the RapidR line, SUB and event, the backtrace's function names,
//! the home folder scrubbed to `~`; nothing anywhere else.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const PROGRAM: &str = "' a program RapidR crashes on purpose\n\
DECLARE SUB Work(N AS INTEGER)\n\
\n\
SUB Work(N AS INTEGER)\n\
  DIM s AS STRING\n\
  s = STR$(N)\n\
  PRINT \"work \" + s\n\
END SUB\n\
\n\
PRINT \"start\"\n\
Work 1\n\
PRINT \"done\"\n";

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("rapidr-crash-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// `rapidr run work.rr` with the crash asked for, its home folder `home`
/// (so the scrubbing can be seen): the output and the reports written.
fn run(dir: &Path, crash: &str, home: &Path) -> (Output, Vec<(String, String)>) {
    let file = home.join("Projects").join("work.rr");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, PROGRAM).unwrap();
    let crashes = dir.join("crashes");
    let out = Command::new(env!("CARGO_BIN_EXE_rapidr"))
        .arg("run")
        .arg(&file)
        .current_dir(dir)
        .env("RAPIDR_TEST_CRASH", crash)
        .env("RAPIDR_CRASH_DIR", &crashes)
        .env("HOME", home)
        .env("RAPIDR_PRINT_TO", dir.join("prints"))
        .env("RAPIDR_REGISTRY", dir.join("registry.reg"))
        .env("RAPIDR_AI_KEYSTORE", "memory")
        .stdin(Stdio::null())
        .output()
        .expect("runs rapidr");
    let mut reports: Vec<(String, String)> = std::fs::read_dir(&crashes)
        .map(|rd| rd.flatten().filter(|e| e.file_name().to_string_lossy().ends_with(".txt") && e.file_name() != "seen.txt").map(|e| (e.file_name().to_string_lossy().into_owned(), std::fs::read_to_string(e.path()).unwrap())).collect())
        .unwrap_or_default();
    reports.sort();
    (out, reports)
}

fn field<'a>(text: &'a str, key: &str) -> &'a str {
    text.lines().find_map(|l| l.strip_prefix(&format!("{key}: "))).unwrap_or("")
}

#[test]
fn a_panic_writes_a_scrubbed_report_with_the_rapidr_line() {
    let dir = scratch("panic");
    let home = dir.join("home").join("ana");
    let (out, reports) = run(&dir, "panic@work.rr:6", &home);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(101), "{stderr}");
    assert!(String::from_utf8_lossy(&out.stdout).contains("start"));
    assert!(stderr.contains("A crash report was written (it stays on this computer)"), "{stderr}");
    assert_eq!(reports.len(), 1, "{reports:?}");
    let (name, text) = &reports[0];
    assert!(name.contains("-program-") && name.ends_with("-crash.txt"), "{name}");
    assert_eq!(field(text, "Kind"), "crash");
    assert_eq!(field(text, "App"), "Program");
    assert!(field(text, "RapidR").starts_with(&format!("{} (commit ", env!("CARGO_PKG_VERSION"))), "{text}");
    assert_eq!(field(text, "Where"), "work.rr line 6, in SUB Work");
    assert_eq!(field(text, "File"), "~/Projects/work.rr");
    assert!(field(text, "Message").contains("RAPIDR_TEST_CRASH"));
    // (the backtrace names RapidR's functions: the interpreter that ran it)
    assert!(text.contains("rapidr_vm::Vm") && text.contains("rapidr_crash::test_crash"), "{text}");
    assert!(!text.contains(&home.to_string_lossy().into_owned()), "the home folder is scrubbed:\n{text}");
    assert!(text.contains("Privacy\n-------\nThis report was written by RapidR on this computer and stays here"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(unix)]
#[test]
fn a_stack_overflow_writes_the_report_a_signal_handler_can() {
    let dir = scratch("overflow");
    let home = dir.join("home").join("ana");
    let (out, reports) = run(&dir, "overflow@work.rr:6", &home);
    let stderr = String::from_utf8_lossy(&out.stderr);
    // (std's handler still says it and aborts)
    assert!(stderr.contains("has overflowed its stack"), "{stderr}");
    assert_eq!(reports.len(), 1, "one report, not a second for the abort: {reports:?}");
    let (name, text) = &reports[0];
    assert!(name.ends_with("-stack-overflow.txt"), "{name}");
    assert_eq!(field(text, "Kind"), "stack-overflow");
    assert_eq!(field(text, "Where"), "work.rr line 6, in SUB Work");
    assert_eq!(field(text, "File"), "~/Projects/work.rr");
    assert!(text.contains("(not taken: after a fatal signal it isn't safe to"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_program_that_runs_well_writes_nothing() {
    let dir = scratch("none");
    let home = dir.join("home").join("ana");
    // (a line the program never reaches with a call into the runtime)
    let (out, reports) = run(&dir, "panic@work.rr:99", &home);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert!(reports.is_empty(), "{reports:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

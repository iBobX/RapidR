//! `rapidr import-rapidq` and `rapidr upgrade-names`: RapidR's names in a
//! copy of a RapidQ program, or in one of RapidR's own files
//! (crates/rapidr-import).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use rapidr_import::{diff, report, Options, Verification};

fn usage() -> ExitCode {
    eprintln!("usage: rapidr import-rapidq <file.bas|.rqw|.rqb|.rq|.inc | folder | project.rrproj> [OUT_DIR | -o OUT_DIR] [--include-dir DIR]… [--no-verify] [--rwin32]");
    eprintln!("       --rwin32: a program that includes WIN32API.INC gets RWin32.inc, RapidR's standard Windows API declarations, in the copy (the report lists what differs)");
    eprintln!("       rapidr upgrade-names <file> [--dry-run] [--include-dir DIR]…");
    ExitCode::from(2)
}

struct Args {
    path: Option<String>,
    /// A second path (import-rapidq's copy folder).
    second: Option<String>,
    out: Option<String>,
    includes: Vec<PathBuf>,
    dry_run: bool,
    verify: bool,
    rwin32: bool,
    bad: bool,
}

fn args(rest: &[String]) -> Args {
    let mut a = Args { path: None, second: None, out: None, includes: Vec::new(), dry_run: false, verify: true, rwin32: false, bad: false };
    let mut it = rest.iter();
    while let Some(x) = it.next() {
        match x.as_str() {
            "-o" | "--output" => a.out = it.next().cloned(),
            "--include" | "--include-dir" | "-I" => match it.next() {
                Some(d) => a.includes.push(PathBuf::from(d)),
                None => a.bad = true,
            },
            "--dry-run" | "-n" => a.dry_run = true,
            "--no-verify" => a.verify = false,
            "--rwin32" => a.rwin32 = true,
            s if s.starts_with('-') => a.bad = true,
            s if a.path.is_none() => a.path = Some(s.to_string()),
            s if a.second.is_none() => a.second = Some(s.to_string()),
            _ => a.bad = true,
        }
    }
    a
}

/// `rapidr import-rapidq <input> [out | -o out]`: the copy goes to `out`
/// (else `<name>-rapidr` beside the input); the input is only read.
pub fn import_rapidq(rest: &[String]) -> ExitCode {
    let a = args(rest);
    let (Some(path), false, false, false) = (a.path.as_deref(), a.bad, a.dry_run, a.out.is_some() && a.second.is_some()) else { return usage() };
    let input = Path::new(path);
    let out = a.out.or(a.second).map(PathBuf::from).unwrap_or_else(|| {
        let stem = input.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "program".into());
        input.parent().unwrap_or(Path::new(".")).join(format!("{stem}-rapidr"))
    });
    let options = Options { include_dirs: a.includes, normalize_case: true };
    let mut r = match rapidr_import::import(input, &out, &options, a.verify) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("import-rapidq: {e}");
            return ExitCode::from(1);
        }
    };
    // (WIN32API.INC: RWin32.inc in the copy when asked, else offered)
    if a.rwin32 {
        if let Err(e) = rapidr_import::rwin32::switch_copy(&rapidr_import::Disk, &mut r) {
            eprintln!("import-rapidq: {e}");
            return ExitCode::from(1);
        }
    }
    let report_path = r.out_dir.join(report::FILE_NAME);
    if let Err(e) = fs::write(&report_path, report::markdown(&r)) {
        eprintln!("{}: {e}", report_path.display());
        return ExitCode::from(1);
    }
    println!("{}", report::summary(&r));
    println!("copy: {}", r.out_dir.display());
    println!("report: {}", report_path.display());
    if let Some(o) = r.rwin32.as_ref().filter(|o| o.switched.is_none()) {
        println!("this program includes WIN32API.INC and keeps it; RWin32.inc, RapidR's standard Windows API declarations, can take its place: run again with --rwin32 ({} difference(s) for what the program uses, in the report)", o.differences.len());
    }
    if let Some(Some(Err(e))) = r.rwin32.as_ref().map(|o| &o.switched) {
        eprintln!("import-rapidq: with RWin32.inc the copy doesn't compile yet: {e} (the report lists what differs)");
        return ExitCode::from(1);
    }
    // (an include found nowhere is never passed over: each one, and what to do)
    let mut said: Vec<String> = Vec::new();
    for m in r.missing() {
        let at = format!("{}:{}", m.from.display(), m.line);
        if said.contains(&at) {
            continue;
        }
        eprintln!("{at}: include file not found: {} — {}", m.written, rapidr_import::missing_fix(&m.written));
        said.push(at);
    }
    let bad = r.count(|v| matches!(v, Verification::Differs | Verification::CopyFails(_)));
    if bad > 0 {
        eprintln!("import-rapidq: {bad} program(s) don't compile as their original: see the report");
        return ExitCode::from(1);
    }
    if !said.is_empty() {
        eprintln!("import-rapidq: {} include file(s) not found: the copy is made without them (see the report)", said.len());
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

/// `rapidr upgrade-names <file> [--dry-run]`: one of RapidR's own files,
/// in place.
pub fn upgrade_names(rest: &[String]) -> ExitCode {
    let a = args(rest);
    let (Some(path), false, None) = (a.path.as_deref(), a.bad, a.second.as_deref()) else { return usage() };
    let options = Options { include_dirs: a.includes, normalize_case: true };
    let u = match rapidr_import::upgrade_file(Path::new(path), &options) {
        Ok(u) => u,
        Err(e) => {
            eprintln!("upgrade-names: {e}");
            return ExitCode::from(1);
        }
    };
    for n in &u.notes {
        eprintln!("{path}:{}: not converted: {}", n.line, n.message);
    }
    if u.changes.is_empty() {
        println!("{path}: already RapidR's names");
        return ExitCode::SUCCESS;
    }
    if a.dry_run {
        print!("{}", diff::unified(path, &u.original, &u.converted, 2));
        println!("{} name(s) would change{}", u.changes.len(), verified_text(u.verified));
        return ExitCode::SUCCESS;
    }
    if u.verified == Some(false) {
        eprintln!("upgrade-names: {path}: the upgraded file wouldn't compile as before; left unchanged (please report it)");
        return ExitCode::from(1);
    }
    if let Err(e) = fs::write(path, &u.bytes) {
        eprintln!("{path}: {e}");
        return ExitCode::from(1);
    }
    println!("{path}: {} name(s) changed{}", u.changes.len(), verified_text(u.verified));
    ExitCode::SUCCESS
}

fn verified_text(v: Option<bool>) -> &'static str {
    match v {
        Some(true) => " (compiles to the same bytecode)",
        Some(false) => " (WARNING: the bytecode would differ)",
        None => " (not checked: the program doesn't compile today)",
    }
}

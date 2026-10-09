//! `rapidr import-rapidq` and `rapidr upgrade-names`: RapidR's names in a
//! copy of a RapidQ program, or in one of RapidR's own files
//! (crates/rapidr-import).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use rapidr_import::{diff, report, Options, Verification};

fn usage() -> ExitCode {
    eprintln!("usage: rapidr import-rapidq <file.bas|.rqw|.rqb|.rq|.inc | folder | project.rrproj> [OUT_DIR | -o OUT_DIR] [--include DIR]… [--no-verify]");
    eprintln!("       rapidr upgrade-names <file> [--dry-run] [--include DIR]…");
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
    bad: bool,
}

fn args(rest: &[String]) -> Args {
    let mut a = Args { path: None, second: None, out: None, includes: Vec::new(), dry_run: false, verify: true, bad: false };
    let mut it = rest.iter();
    while let Some(x) = it.next() {
        match x.as_str() {
            "-o" | "--output" => a.out = it.next().cloned(),
            "--include" | "-I" => match it.next() {
                Some(d) => a.includes.push(PathBuf::from(d)),
                None => a.bad = true,
            },
            "--dry-run" | "-n" => a.dry_run = true,
            "--no-verify" => a.verify = false,
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
    let r = match rapidr_import::import(input, &out, &options, a.verify) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("import-rapidq: {e}");
            return ExitCode::from(1);
        }
    };
    let report_path = r.out_dir.join(report::FILE_NAME);
    if let Err(e) = fs::write(&report_path, report::markdown(&r)) {
        eprintln!("{}: {e}", report_path.display());
        return ExitCode::from(1);
    }
    println!("{}", report::summary(&r));
    println!("copy: {}", r.out_dir.display());
    println!("report: {}", report_path.display());
    let bad = r.count(|v| matches!(v, Verification::Differs | Verification::CopyFails(_)));
    if bad > 0 {
        eprintln!("import-rapidq: {bad} program(s) don't compile as their original: see the report");
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

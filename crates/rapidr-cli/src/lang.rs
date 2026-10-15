//! `rapidr lang …`: what the language registry (crates/rapidr-lang) generates.
//!
//! ```text
//! rapidr lang export --json [-o FILE]     the registry as JSON (the IDE's completion data)
//! rapidr lang export --prompt [-o FILE]   the AI system prompt's language section
//! rapidr lang export --manual [DIR]       the manual's reference pages (docs/manual/reference)
//! rapidr lang export --all [--check]      every file the repository keeps generated, from its root
//! rapidr lang conformance DIR [--target desktop|web] [--gaps FILE]
//!                                         one program per component and the output it must print
//! rapidr lang ai-check [--summary|--json] [--strict] [NAME…]
//!                                         which entries lack an AI facet (docs/ai-architecture.md §2.6)
//! ```
//!
//! `--check`: write nothing, exit 1 naming the files that are out of date.
//! `ai-check` reports and exits 0 (phase 0: a list, not a gate); `--strict`
//! exits 1 when anything is missing.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use rapidr_lang::conformance::{self, Gaps, Target};
use rapidr_lang::export;

/// The files the repository keeps generated from the registry (paths from its root).
fn generated_files() -> Vec<(PathBuf, String)> {
    export::generated_files().into_iter().map(|(p, t)| (PathBuf::from(p), t)).collect()
}

pub fn command(args: &[String]) -> ExitCode {
    match args.first().map(String::as_str) {
        Some("export") => export_cmd(&args[1..]),
        Some("conformance") => conformance_cmd(&args[1..]),
        Some("ai-check") => ai_check_cmd(&args[1..]),
        _ => {
            eprintln!("usage: rapidr lang export --json|--prompt [-o FILE] | --manual [DIR] | --all [--check]");
            eprintln!("       rapidr lang conformance DIR [--target desktop|web] [--gaps FILE]");
            eprintln!("       rapidr lang ai-check [--summary|--json] [--strict] [NAME…]");
            ExitCode::from(2)
        }
    }
}

/// `rapidr lang ai-check`: every registry entry lacking an AI facet
/// (semantics, types, range, effects, example), then the counts.
fn ai_check_cmd(args: &[String]) -> ExitCode {
    let flag = |f: &str| args.iter().any(|a| a == f);
    if let Some(bad) = args.iter().find(|a| a.starts_with('-') && !["--summary", "--json", "--strict"].contains(&a.as_str())) {
        eprintln!("rapidr lang ai-check: unknown option {bad} (--summary, --json, --strict)");
        return ExitCode::from(2);
    }
    let names: Vec<&str> = args.iter().filter(|a| !a.starts_with('-')).map(String::as_str).collect();
    let report = if names.is_empty() {
        rapidr_lang::ai::check()
    } else {
        match rapidr_lang::ai::check_names(&names) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("rapidr lang ai-check: {e}");
                return ExitCode::from(2);
            }
        }
    };
    let text = if flag("--json") {
        report.json()
    } else if flag("--summary") {
        report.summary()
    } else {
        report.text()
    };
    // (a closed pipe — `| head` — ends the output quietly)
    let _ = std::io::Write::write_all(&mut std::io::stdout().lock(), text.as_bytes());
    if flag("--strict") && !report.is_complete() {
        eprintln!("rapidr lang ai-check --strict: {} gaps", report.gaps.len());
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

fn write_or_print(out: Option<&str>, text: &str) -> ExitCode {
    match out {
        Some(path) => match fs::write(path, text) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("{path}: {e}");
                ExitCode::from(1)
            }
        },
        None => {
            print!("{text}");
            ExitCode::SUCCESS
        }
    }
}

fn export_cmd(args: &[String]) -> ExitCode {
    let flag = |f: &str| args.iter().any(|a| a == f);
    let out = args.iter().position(|a| a == "-o" || a == "--output").and_then(|i| args.get(i + 1)).map(String::as_str);
    if flag("--json") {
        return write_or_print(out, &export::json());
    }
    if flag("--prompt") {
        return write_or_print(out, &export::prompt());
    }
    let check = flag("--check");
    let files: Vec<(PathBuf, String)> = if flag("--manual") {
        let dir = args.iter().skip_while(|a| *a != "--manual").nth(1).filter(|a| !a.starts_with('-')).map_or("docs/manual/reference", String::as_str);
        export::manual().into_iter().map(|(n, t)| (Path::new(dir).join(n), t)).collect()
    } else if flag("--all") {
        generated_files()
    } else {
        eprintln!("rapidr lang export: --json, --prompt, --manual or --all");
        return ExitCode::from(2);
    };
    let mut stale = Vec::new();
    for (path, text) in files {
        if fs::read_to_string(&path).ok().as_deref() == Some(text.as_str()) {
            continue;
        }
        if check {
            stale.push(path.display().to_string());
            continue;
        }
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        if let Err(e) = fs::write(&path, text) {
            eprintln!("{}: {e}", path.display());
            return ExitCode::from(1);
        }
        println!("wrote {}", path.display());
    }
    if !stale.is_empty() {
        eprintln!("out of date (rapidr lang export --all): {}", stale.join(", "));
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

fn conformance_cmd(args: &[String]) -> ExitCode {
    let Some(dir) = args.first().filter(|a| !a.starts_with('-')) else {
        eprintln!("rapidr lang conformance DIR [--target desktop|web] [--gaps FILE]");
        return ExitCode::from(2);
    };
    let opt = |f: &str| args.iter().position(|a| a == f).and_then(|i| args.get(i + 1)).map(String::as_str);
    let target = match opt("--target").unwrap_or("desktop") {
        "desktop" => Target::Desktop,
        "web" => Target::Web,
        t => {
            eprintln!("--target {t}: desktop or web");
            return ExitCode::from(2);
        }
    };
    let gaps = match opt("--gaps") {
        Some(path) => match fs::read_to_string(path).map_err(|e| e.to_string()).and_then(|t| Gaps::parse(&t)) {
            Ok(g) => g,
            Err(e) => {
                eprintln!("{path}: {e}");
                return ExitCode::from(1);
            }
        },
        None => Gaps::default(),
    };
    if let Err(e) = fs::create_dir_all(dir) {
        eprintln!("{dir}: {e}");
        return ExitCode::from(1);
    }
    let programs = conformance::programs(target, &gaps);
    for p in &programs {
        let base = Path::new(dir).join(p.component.to_ascii_lowercase());
        if let Err(e) = fs::write(base.with_extension("bas"), &p.source).and_then(|_| fs::write(base.with_extension("expected"), &p.expected)) {
            eprintln!("{}: {e}", base.display());
            return ExitCode::from(1);
        }
    }
    println!("{} programs for the {} in {dir}", programs.len(), target.name());
    ExitCode::SUCCESS
}

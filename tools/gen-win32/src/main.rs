//! `gen-win32`: writes RWin32.inc (see the library's documentation).
//!
//!   cargo run -p rapidr-gen-win32 --release                 the file, from cargo's cache
//!   cargo run -p rapidr-gen-win32 --release -- --check      whether the committed file is what the input gives
//!   … -- --source <windows-sys-0.61.2 folder> --out <file>

use std::path::PathBuf;
use std::process::ExitCode;

/// Where the generated file lives: RapidR's built-in include libraries.
const DEFAULT_OUT: &str = "crates/rapidr-preprocessor/src/libraries/RWin32.inc";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut source: Option<PathBuf> = None;
    let mut out = PathBuf::from(DEFAULT_OUT);
    let mut check = false;
    let mut verbose = false;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--source" => source = it.next().map(PathBuf::from),
            "--out" => out = it.next().map(PathBuf::from).unwrap_or(out),
            "--check" => check = true,
            "--verbose" | "-v" => verbose = true,
            "-h" | "--help" => {
                println!("gen-win32 [--source <windows-sys-{} folder>] [--out <file>] [--check]", rapidr_gen_win32::WINDOWS_SYS_VERSION);
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("gen-win32: unknown argument {other}");
                return ExitCode::from(2);
            }
        }
    }
    let Some(root) = source.or_else(rapidr_gen_win32::default_source_dir) else {
        eprintln!("gen-win32: windows-sys {} isn't in cargo's cache; run `cargo fetch` in the workspace (it is in Cargo.lock), or give --source", rapidr_gen_win32::WINDOWS_SYS_VERSION);
        return ExitCode::from(1);
    };
    let src = match rapidr_gen_win32::read_source(&root) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("gen-win32: {e}");
            return ExitCode::from(1);
        }
    };
    let generated = rapidr_gen_win32::generate(&src, rapidr_preprocessor::RAPIDQ_INC_CONSTANTS);
    let c = &generated.counts;
    println!("input: {} ({} files, SHA-256 {})", root.display(), src.files.len(), src.sha256);
    println!(
        "RWin32.inc: {} bytes, {} lines; {} constants ({} text), {} TYPEs ({} with two layouts), {} functions; classic: {} functions, {} TYPEs",
        generated.text.len(),
        generated.text.lines().count(),
        c.consts,
        c.string_consts,
        c.types,
        c.types_arch_specific,
        c.functions,
        c.classic_functions,
        c.classic_types
    );
    println!("left to RAPIDQ.INC: {}; name clashes skipped: {}; TYPEs that can't be written: {}", c.left_to_rapidq_inc, c.name_clashes, c.skipped_types);
    for (why, n) in &c.skipped_consts {
        println!("  constants left out ({why}): {n}");
    }
    for (why, n) in &c.skipped_functions {
        println!("  functions left out ({why}): {n}");
    }
    if verbose {
        println!("  the functions left out: {}", c.skipped_function_names.join(" "));
    }
    if check {
        let now = std::fs::read_to_string(&out).unwrap_or_default();
        if now != generated.text {
            eprintln!("gen-win32: {} is not what the input gives: run gen-win32 without --check", out.display());
            return ExitCode::from(1);
        }
        println!("{} is up to date", out.display());
        return ExitCode::SUCCESS;
    }
    if let Err(e) = std::fs::write(&out, &generated.text) {
        eprintln!("gen-win32: {}: {e}", out.display());
        return ExitCode::from(1);
    }
    println!("wrote {}", out.display());
    ExitCode::SUCCESS
}

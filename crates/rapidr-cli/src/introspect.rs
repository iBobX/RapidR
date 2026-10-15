//! `rapidr introspect FILE… [--request JSON]` (cargo feature
//! `ai-introspect`; AI-ARCH phase 0, docs/ai-architecture.md §8): the
//! introspection interface's design-time side over a program's files, for
//! developers and the tests of what comes next (`rapidr mcp`).
//!
//! Requests are JSON lines on standard input (or one `--request`); each gets
//! one JSON reply line on standard output. The files are read from the
//! folder of the first one (or `--root DIR`), and nothing outside it.
//!
//! ```text
//! $ echo '{"op":"list"}' | rapidr introspect examples/data/csv_explorer.rr
//! {"ok":true,"result":{"rev":"…","total":12,"objects":[{"id":"Form", …
//! ```

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use rapidr_introspect::design::DesignModel;
use rapidr_introspect::handle_json;

pub fn command(args: &[String]) -> ExitCode {
    let mut files: Vec<PathBuf> = Vec::new();
    let mut request: Option<String> = None;
    let mut root: Option<PathBuf> = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--request" => request = it.next().cloned(),
            "--root" => root = it.next().map(PathBuf::from),
            s if s.starts_with('-') => {
                eprintln!("rapidr introspect: unknown option {s}");
                return usage();
            }
            f => files.push(PathBuf::from(f)),
        }
    }
    if files.is_empty() {
        return usage();
    }
    let root = match root.or_else(|| files[0].parent().map(Path::to_path_buf)) {
        Some(r) if r.as_os_str().is_empty() => PathBuf::from("."),
        Some(r) => r,
        None => PathBuf::from("."),
    };
    let root = match std::fs::canonicalize(&root) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{}: {e}", root.display());
            return ExitCode::from(1);
        }
    };
    let files: Vec<PathBuf> = files.iter().map(|f| std::fs::canonicalize(f).unwrap_or_else(|_| f.clone())).collect();
    let model = match DesignModel::open(&root, &files) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("rapidr introspect: {e}");
            return ExitCode::from(1);
        }
    };
    let mut out = std::io::stdout().lock();
    if let Some(r) = request {
        let _ = writeln!(out, "{}", handle_json(Some(&model), &r));
        return ExitCode::SUCCESS;
    }
    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let _ = writeln!(out, "{}", handle_json(Some(&model), &line));
        let _ = out.flush();
    }
    ExitCode::SUCCESS
}

fn usage() -> ExitCode {
    eprintln!("usage: rapidr introspect FILE… [--root DIR] [--request JSON]");
    eprintln!("       requests (one JSON object a line on standard input): {{\"op\":\"describe\",\"type\":\"RButton\"}}, {{\"op\":\"list\"}}, {{\"op\":\"get\",\"id\":\"Button1\"}}");
    ExitCode::from(2)
}

//! The import's report, as Markdown (written beside the copy as
//! `rapidr-import-report.md`).

use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::Path;

use crate::import::{ImportReport, Verification};

/// The report's file name in the copy.
pub const FILE_NAME: &str = "rapidr-import-report.md";

fn rel<'a>(p: &'a Path, base: &Path) -> std::borrow::Cow<'a, str> {
    match p.strip_prefix(base) {
        Ok(r) => std::borrow::Cow::Owned(r.display().to_string()),
        Err(_) => p.to_string_lossy(),
    }
}

fn source_root(r: &ImportReport) -> &Path {
    if r.input.is_dir() {
        &r.input
    } else {
        r.input.parent().unwrap_or(Path::new(""))
    }
}

/// One line per number: what was done.
pub fn summary(r: &ImportReport) -> String {
    let sources = r.files.iter().filter(|f| f.is_source).count();
    let changed = r.files.iter().filter(|f| !f.changes.is_empty()).count();
    let mut s = format!(
        "{} program(s), {sources} source file(s) ({changed} changed, {} name(s)), {} other file(s) copied",
        r.programs.len(),
        r.changes(),
        r.files.len() - sources
    );
    if r.programs.iter().any(|p| p.verification.is_some()) {
        let _ = write!(
            s,
            "; compiled as the original: {} identical, {} different, {} copy failed, {} original doesn't compile in RapidR today",
            r.count(|v| *v == Verification::Identical),
            r.count(|v| *v == Verification::Differs),
            r.count(|v| matches!(v, Verification::CopyFails(_))),
            r.count(|v| matches!(v, Verification::OriginalFails(_)))
        );
    }
    s
}

/// The whole report.
pub fn markdown(r: &ImportReport) -> String {
    let root = source_root(r);
    let mut out = String::new();
    let _ = writeln!(out, "# RapidQ import: {}\n", r.input.file_name().map(|n| n.to_string_lossy()).unwrap_or_default());
    let _ = writeln!(out, "A copy of `{}` in `{}`, with RapidR's component names (`QBUTTON` → `RButton`). The original is unchanged. RapidR runs both alike: it reads RapidQ's names and RapidR's as the same components.\n", r.input.display(), r.out_dir.display());
    let _ = writeln!(out, "**Summary:** {}.\n", summary(r));

    // Kinds of source files.
    let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
    for f in r.files.iter().filter(|f| f.is_source) {
        *kinds.entry(f.source.extension().map(|e| format!(".{}", e.to_string_lossy().to_ascii_lowercase())).unwrap_or_else(|| "(none)".into())).or_default() += 1;
    }
    if !kinds.is_empty() {
        let list: Vec<String> = kinds.iter().map(|(k, n)| format!("{n} `{k}`")).collect();
        let _ = writeln!(out, "Source files: {} (RapidQ's `.rqw` window programs, `.rqb` / `.rq` libraries and `.inc` includes keep their extensions).\n", list.join(", "));
    }

    if !r.programs.is_empty() {
        let _ = writeln!(out, "## Programs\n\n| Program | Compiled as the original |\n|---|---|");
        for p in &r.programs {
            let v = match &p.verification {
                None => "not checked".to_string(),
                Some(Verification::Identical) => "yes: identical bytecode".to_string(),
                Some(Verification::Differs) => "**no: the bytecode differs** (please report it)".to_string(),
                Some(Verification::CopyFails(e)) => format!("**the copy doesn't compile**: {e}"),
                Some(Verification::OriginalFails(e)) => format!("not checked: the original doesn't compile in RapidR today ({e})"),
            };
            let _ = writeln!(out, "| `{}` | {} |", rel(&p.entry, root), v.replace('|', "\\|"));
        }
        out.push('\n');
    }

    if r.rapidq_inc_includers > 0 || !r.rapidq_inc_left_out.is_empty() {
        let _ = writeln!(out, "## RAPIDQ.INC\n");
        let _ = writeln!(out, "`$INCLUDE \"RAPIDQ.INC\"` is kept: RapidR supplies RAPIDQ.INC's constants (`clRed`, `MB_OK` …) through that line without needing the file, as RapidQ gives them through it. Without the line those names would be undeclared ({} program(s) include it).", r.rapidq_inc_includers);
        if !r.rapidq_inc_left_out.is_empty() {
            let _ = writeln!(out, "\nRapidQ's own RAPIDQ.INC file isn't copied (RapidR's constants replace it; the program uses nothing else of it): {}.", r.rapidq_inc_left_out.iter().map(|p| format!("`{}`", p.display())).collect::<Vec<_>>().join(", "));
        }
        out.push('\n');
    }

    let _ = writeln!(out, "## Changes\n");
    let mut any = false;
    for f in r.files.iter().filter(|f| !f.changes.is_empty()) {
        any = true;
        let _ = writeln!(out, "### `{}`\n", rel(&f.source, root));
        for c in &f.changes {
            if c.to.is_empty() {
                let _ = writeln!(out, "- line {}: removed `{}`", c.line, c.from);
            } else {
                let _ = writeln!(out, "- line {}, column {}: `{}` → `{}`", c.line, c.column, c.from, c.to);
            }
        }
        out.push('\n');
    }
    if !any {
        out.push_str("None: the program already uses RapidR's names.\n\n");
    }

    let _ = writeln!(out, "## Not converted\n");
    if r.notes.is_empty() {
        out.push_str("Nothing.\n");
    }
    for n in &r.notes {
        let place = match (&n.path, n.line) {
            (Some(p), 0) => format!("`{}`", rel(p, root)),
            (Some(p), l) => format!("`{}`, line {l}", rel(p, root)),
            (None, _) => String::new(),
        };
        let _ = writeln!(out, "- {place}: {}", n.message);
    }
    out
}

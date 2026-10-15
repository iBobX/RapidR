//! The import's report, as Markdown (written beside the copy as
//! `rapidr-import-report.md`).

use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::Path;

use crate::import::{missing_fix, ImportReport, IncludeReport, Verification};
use crate::includes::Place;

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
    let (copied, builtin, missing) = include_counts(r);
    let _ = write!(s, "; include files: {copied} copied, {builtin} RapidR's built-in, {missing} not found");
    s
}

/// The includes copied, those RapidR supplies, those found nowhere (each
/// file once).
pub fn include_counts(r: &ImportReport) -> (usize, usize, usize) {
    let mut copied: Vec<String> = r.includes.iter().filter_map(|i| i.copy.as_ref()).map(|c| c.to_string_lossy().to_lowercase()).collect();
    copied.sort();
    copied.dedup();
    let mut builtin: Vec<String> = r.includes.iter().filter(|i| i.copy.is_none() && i.place != Place::Missing).map(|i| short(&i.written)).collect();
    builtin.sort();
    builtin.dedup();
    let mut missing: Vec<String> = r.missing().map(|i| i.written.to_lowercase()).collect();
    missing.sort();
    missing.dedup();
    (copied.len(), builtin.len(), missing.len())
}

/// A path's file name, upper case (RAPIDQ.INC).
fn short(written: &str) -> String {
    let n = written.trim().replace('\\', "/");
    n.rsplit('/').next().unwrap_or(&n).to_ascii_uppercase()
}

/// Where an include was found, as the report says it.
fn place_text(i: &IncludeReport, root: &Path) -> String {
    let file = |f: &Option<std::path::PathBuf>| f.as_ref().map(|p| format!(": `{}`", rel(p, root))).unwrap_or_default();
    match &i.place {
        Place::ProgramFolder => format!("in the program's folder{}", file(&i.file)),
        Place::IncludeFolder(d) => format!("in the include folder `{}`", d.display()),
        Place::BesideIncluder => format!("beside the file that includes it{}", file(&i.file)),
        Place::ByName(d) => format!("by its name, in `{}`", d.display()),
        Place::Absolute => "at that path".to_string(),
        Place::Elsewhere(d) => format!("in `{}`", d.display()),
        Place::BuiltIn => "RapidR's built-in".to_string(),
        Place::Missing => "**not found**".to_string(),
    }
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
        let _ = writeln!(out, "Source files: {} (RapidQ's `.rqw`, `.rqb` and `.rq` programs and `.inc` includes keep their extensions).\n", list.join(", "));
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

    if !r.includes.is_empty() {
        let (copied, builtin, missing) = include_counts(r);
        let _ = writeln!(out, "## Include files\n");
        let _ = writeln!(out, "Each file an `$INCLUDE` reads, found as RapidR's compiler finds it (as RapidQ's: in the program's folder, then the include folders; then beside the file that includes it, and a path written on another computer by its file name): {copied} copied with the program, {builtin} RapidR's built-in, {missing} not found.\n");
        let _ = writeln!(out, "| `$INCLUDE` | In | Found | In the copy |\n|---|---|---|---|");
        for i in &r.includes {
            let copy = match (&i.copy, &i.rewritten) {
                (Some(c), Some(w)) => format!("`{}` (the line now names `{}`)", rel(c, &r.out_dir), w),
                (Some(c), None) => format!("`{}`", rel(c, &r.out_dir)),
                (None, _) if i.place == Place::Missing => "—".to_string(),
                (None, _) if i.file.is_some() => "not copied: RapidR supplies it built in".to_string(),
                (None, _) => "—".to_string(),
            };
            let _ = writeln!(out, "| `{}` | `{}`, line {} | {} | {} |", i.written, rel(&i.from, root), i.line, place_text(i, root).replace('|', "\\|"), copy.replace('|', "\\|"));
        }
        out.push('\n');
        if missing > 0 {
            let _ = writeln!(out, "### Not found\n");
            let mut said: Vec<(String, usize)> = Vec::new();
            for i in r.missing() {
                let k = (rel(&i.from, root).into_owned(), i.line);
                if said.contains(&k) {
                    continue;
                }
                said.push(k);
                let _ = writeln!(out, "- `{}` (`{}`, line {}): {}.", i.written, rel(&i.from, root), i.line, missing_fix(&i.written));
            }
            out.push('\n');
        }
        if r.include_dirs.is_empty() {
            let _ = writeln!(out, "Include folders searched: none.\n");
        } else {
            let _ = writeln!(out, "Include folders searched: {}.\n", r.include_dirs.iter().map(|d| format!("`{}`", d.display())).collect::<Vec<_>>().join(", "));
        }
    }

    if r.rapidq_inc_includers > 0 || !r.builtin_left_out.is_empty() {
        let _ = writeln!(out, "## RAPIDQ.INC and RapidR's built-in includes\n");
        if r.rapidq_inc_includers > 0 {
            let _ = writeln!(out, "`$INCLUDE \"RAPIDQ.INC\"` is kept: RapidR supplies RAPIDQ.INC's constants (`clRed`, `MB_OK` …) through that line without needing the file, as RapidQ gives them through it. Without the line those names would be undeclared ({} program(s) include it).", r.rapidq_inc_includers);
        }
        if !r.builtin_left_out.is_empty() {
            let _ = writeln!(out, "\nYour own copies of includes RapidR supplies built in aren't copied (RapidR's replace them): {}.", r.builtin_left_out.iter().map(|p| format!("`{}`", p.display())).collect::<Vec<_>>().join(", "));
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

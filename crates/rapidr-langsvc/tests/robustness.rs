//! Every program RapidR has — its examples, the conformance cases and,
//! when it's on this machine, RapidQ's example corpus (`RAPIDQ_CORPUS`,
//! default ~/Downloads/Rapidq/examples) — analyses without a panic, every
//! request at many places of each file, and with every file cut short
//! (what an editor sees while typing).

use std::fs;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::time::Instant;

use rapidr_langsvc::{Analysis, Options};

fn programs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            programs(&p, out);
        } else if p.extension().and_then(|e| e.to_str()).is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "bas" | "rr")) {
            out.push(p);
        }
    }
}

fn exercise(path: &Path, text: &str) {
    let mut a = Analysis::new(Options { rapidq_compatible: true, ..Default::default() });
    a.update(path.to_path_buf(), text.to_string());
    let _ = a.diagnostics(path);
    let _ = a.outline(path);
    let _ = a.semantic_tokens(path);
    let _ = a.format(path, "    ");
    let n = text.len().max(1);
    let step = (n / 24).max(1);
    let mut at = 0;
    while at <= text.len() {
        if text.is_char_boundary(at) {
            let _ = a.completions(path, at);
            let _ = a.hover(path, at);
            let _ = a.signature(path, at);
            let _ = a.definition(path, at);
            let _ = a.references(path, at, true);
            let _ = a.prepare_rename(path, at);
            let _ = a.code_actions(path, at, at);
        }
        at += step;
    }
}

#[test]
fn every_program_analyses_without_a_panic() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    programs(&root.join("examples"), &mut files);
    programs(&root.join("tests/conformance/cases"), &mut files);
    let corpus = std::env::var_os("RAPIDQ_CORPUS")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Downloads/Rapidq/examples")));
    if let Some(c) = corpus.filter(|c| c.is_dir()) {
        programs(&c, &mut files);
    }
    let limit: usize = std::env::var("ROBUSTNESS_LIMIT").ok().and_then(|v| v.parse().ok()).unwrap_or(usize::MAX);
    files.truncate(limit);
    let started = Instant::now();
    let mut panics = Vec::new();
    for f in &files {
        let Ok(text) = rapidr_preprocessor::read_source(f) else { continue };
        if catch_unwind(AssertUnwindSafe(|| exercise(f, &text))).is_err() {
            panics.push(format!("{}", f.display()));
            continue;
        }
        // Cut short in the middle of a line (typing).
        let mut cut = text.len() * 2 / 3;
        while !text.is_char_boundary(cut) {
            cut -= 1;
        }
        if catch_unwind(AssertUnwindSafe(|| exercise(f, &text[..cut]))).is_err() {
            panics.push(format!("{} (cut at {cut})", f.display()));
        }
    }
    eprintln!("{} programs in {:?}", files.len(), started.elapsed());
    assert!(panics.is_empty(), "panics in:\n{}", panics.join("\n"));
}

//! The analysis off the UI thread (docs/speed-audit-2026-11.md, "Studio:
//! the edit pause"): what the worker makes is what a synchronous analysis
//! makes of the same texts — the problems, the project's problems, the
//! colours, the outline, the helpers — for every example and a sample of
//! the conformance cases; work for texts that changed since is never taken
//! for theirs; one parse serves every analysis of the same texts, and an
//! `$INCLUDE` changed on the disk is parsed again.
//!
//! (One test, run in its own binary: it switches the shared parses off for
//! its references, a process-wide switch.)

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use rapidr_langsvc::{Analysis, FileDiagnostic, Options, OutlineItem, SemanticToken, Want};

#[derive(Debug, PartialEq)]
struct Made {
    diagnostics: Vec<FileDiagnostic>,
    root_diagnostics: Vec<FileDiagnostic>,
    project: Vec<FileDiagnostic>,
    tokens: Vec<SemanticToken>,
    outline: Vec<OutlineItem>,
    helpers: Vec<rapidr_langsvc::HelperSite>,
}

const ALL: fn(&Path) -> Vec<Want> = |main| vec![Want::Snapshot, Want::RootDiagnostics, Want::Diagnostics, Want::Tokens, Want::Outline, Want::Helpers, Want::Project(main.to_path_buf())];

/// What an analysis says of `file` (as its main file).
fn made(a: &mut Analysis, file: &Path) -> Made {
    let root = a.snapshot(file).map_or_else(|| file.to_path_buf(), |s| s.parsed.root.clone());
    Made {
        diagnostics: a.diagnostics(file),
        root_diagnostics: a.diagnostics(&root),
        project: a.project_diagnostics(file),
        tokens: a.semantic_tokens(file),
        outline: a.outline(file),
        helpers: a.helpers(file),
    }
}

/// Polls (as an editor's tick does) until `wants` of `file` are in.
fn wait_ready(a: &mut Analysis, file: &Path, wants: &[Want]) {
    let until = Instant::now() + Duration::from_secs(120);
    while !a.ready(file, wants) {
        assert!(Instant::now() < until, "the background never answered for {}", file.display());
        assert!(a.pending(), "nothing under way, nothing in, for {}", file.display());
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn corpus() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut out = Vec::new();
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(rd) = fs::read_dir(dir) else { return };
        let mut entries: Vec<PathBuf> = rd.filter_map(Result::ok).map(|e| e.path()).collect();
        entries.sort();
        for p in entries {
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|e| e == "rr" || e == "bas") {
                out.push(p);
            }
        }
    }
    walk(&root.join("examples"), &mut out);
    let mut cases = Vec::new();
    walk(&root.join("tests/conformance/cases"), &mut cases);
    out.extend(cases.into_iter().step_by(9));
    out
}

#[test]
fn the_background_makes_what_the_thread_would() {
    // ---- every example, a ninth of the conformance cases ----
    let files = corpus();
    assert!(files.len() > 80, "the corpus: {} files", files.len());
    let mut compared = 0;
    for file in &files {
        let Ok(text) = fs::read_to_string(file) else { continue };
        // (the reference: made afresh, on this thread)
        rapidr_langsvc::background::share_parses(false);
        let mut sync = Analysis::new(Options::default());
        sync.update(file.clone(), text.clone());
        let want = made(&mut sync, file);
        // (the same in the background, parses shared as in RapidR Studio)
        rapidr_langsvc::background::share_parses(true);
        let mut bg = Analysis::new(Options::default());
        bg.set_background(true);
        bg.update(file.clone(), text.clone());
        let wants = ALL(file);
        bg.request(file, &wants);
        wait_ready(&mut bg, file, &wants);
        assert!(!bg.pending());
        assert_eq!(made(&mut bg, file), want, "{}", file.display());
        compared += 1;
    }
    assert!(compared > 80, "{compared} compared");

    // ---- a project on the disk, its modules open in editors ----
    let dir = std::env::temp_dir().join(format!("rapidr-langsvc-background-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let main = dir.join("main.rr");
    let (globals, ui) = (dir.join("globals.rr"), dir.join("ui.rr"));
    fs::write(&main, "$APPTYPE CONSOLE\n$INCLUDE \"globals.rr\"\n$INCLUDE \"ui.rr\"\nCounter = 1\nShowCount\n").unwrap();
    fs::write(&globals, "DIM Counter AS INTEGER\n").unwrap();
    fs::write(&ui, "SUB ShowCount\n    PRINT Counter\nEND SUB\n").unwrap();
    let options = Options { project: Some(main.clone()), ..Options::default() };
    let typed = "SUB ShowCount\n    PRINT Counter\n    Nope 1\nEND SUB\n";
    rapidr_langsvc::background::share_parses(false);
    let mut sync = Analysis::new(options.clone());
    sync.update(ui.clone(), typed);
    let want = made(&mut sync, &ui);
    assert!(want.project.iter().any(|d| d.severity == rapidr_langsvc::Severity::Error), "{:?}", want.project);
    rapidr_langsvc::background::share_parses(true);
    let mut bg = Analysis::new(options.clone());
    bg.set_background(true);
    bg.update(ui.clone(), typed);
    let wants = ALL(&main);
    bg.request(&ui, &wants);
    wait_ready(&mut bg, &ui, &wants);
    let got = made(&mut bg, &ui);
    assert_eq!(got.diagnostics, want.diagnostics);
    assert_eq!(got.tokens, want.tokens);
    assert_eq!(got.outline, want.outline);
    assert_eq!(got.helpers, want.helpers);
    assert_eq!(bg.project_diagnostics(&main), sync.project_diagnostics(&main));

    // ---- one parse for every analysis of the same texts ----
    let mut other = Analysis::new(options.clone());
    other.update(ui.clone(), typed);
    let (a, b) = (bg.snapshot(&ui).unwrap(), other.snapshot(&ui).unwrap());
    assert!(Arc::ptr_eq(&a, &b), "the panes' analysis takes the editor's parse");
    // (and the compiler's checks once: the same list, made on that parse)
    assert_eq!(other.project_diagnostics(&main), sync.project_diagnostics(&main));

    // ---- an $INCLUDE changed on the disk: parsed again ----
    fs::write(&globals, "DIM Counter AS INTEGER\nSUB Nope(x AS INTEGER)\nEND SUB\n").unwrap();
    let mut after = Analysis::new(options.clone());
    after.set_background(true);
    after.update(ui.clone(), typed);
    after.request(&ui, &wants);
    wait_ready(&mut after, &ui, &wants);
    let fresh = after.snapshot(&ui).unwrap();
    assert!(!Arc::ptr_eq(&a, &fresh), "the changed include made a new parse");
    assert!(!after.project_diagnostics(&main).iter().any(|d| d.severity == rapidr_langsvc::Severity::Error), "the include's new SUB is seen: {:?}", after.project_diagnostics(&main));

    // ---- work for older texts is never taken for newer ones ----
    let mut typing = Analysis::new(options.clone());
    typing.set_background(true);
    typing.update(ui.clone(), "SUB ShowCount\n    PRINT Counter + Gone1\nEND SUB\n");
    typing.request(&ui, &wants);
    // (typed again before the first answer: the newer texts' work asked)
    let last = "SUB ShowCount\n    PRINT Counter\nEND SUB\n";
    typing.update(ui.clone(), last);
    assert!(!typing.ready(&ui, &wants), "the first job's work isn't for these texts");
    assert!(!typing.pending(), "nothing asked for these texts yet");
    typing.request(&ui, &wants);
    wait_ready(&mut typing, &ui, &wants);
    rapidr_langsvc::background::share_parses(false);
    let mut reference = Analysis::new(options.clone());
    reference.update(ui.clone(), last);
    assert_eq!(made(&mut typing, &ui), made(&mut reference, &ui));
    rapidr_langsvc::background::share_parses(true);

    // ---- an exact question meanwhile waits for the work, never makes it twice ----
    // (Nope gone from globals.rr again: the error is back)
    fs::write(&globals, "DIM Counter AS INTEGER\n").unwrap();
    let mut exact = Analysis::new(options);
    exact.set_background(true);
    exact.update(ui.clone(), typed);
    exact.request(&ui, &wants);
    let d = exact.diagnostics(&ui);
    assert!(!exact.pending(), "the work under way was waited for and taken in");
    assert_eq!(d, want.diagnostics);
    let _ = fs::remove_dir_all(&dir);
}

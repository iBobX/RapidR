//! A project's files analysed together (Preview 4's S3 and S5): a module is
//! checked as part of the program that includes it — open or not, never
//! alone —, references and definitions cross its files in the program's
//! include order, and the Problems pane's list tells the truth: errors
//! first, grouped by file, and what works on one runtime only noted once
//! per component, only when the project's Targets reach the other runtime.

use std::fs;
use std::path::{Path, PathBuf};

use rapidr_langsvc::{Analysis, LineIndex, Options, Severity, Targets};

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("rapidr-langsvc-project-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

const MAIN: &str = "$APPTYPE CONSOLE\n$INCLUDE \"globals.rr\"\n$INCLUDE \"server.rr\"\n$INCLUDE \"ui.rr\"\nCounter = 1\nShowCount\n";
const GLOBALS: &str = "DIM Counter AS INTEGER\nDIM Srv AS RServerSocket\n";
const SERVER: &str = "SUB Bump\n    Counter = Counter + 1\n    Srv.Port = 8080\n    IF Srv.Start THEN Srv.Stop\nEND SUB\n";
const UI: &str = "SUB ShowCount\n    Bump\n    PRINT Counter\nEND SUB\n";

/// A project of four files on the disk: main.rr includes the three modules.
fn project(name: &str) -> (PathBuf, PathBuf, PathBuf, PathBuf) {
    let dir = scratch(name);
    let main = dir.join("main.rr");
    fs::write(&main, MAIN).unwrap();
    fs::write(dir.join("globals.rr"), GLOBALS).unwrap();
    fs::write(dir.join("server.rr"), SERVER).unwrap();
    fs::write(dir.join("ui.rr"), UI).unwrap();
    (main, dir.join("globals.rr"), dir.join("server.rr"), dir.join("ui.rr"))
}

fn errors(d: &[rapidr_langsvc::FileDiagnostic]) -> Vec<String> {
    d.iter().filter(|d| d.severity == Severity::Error).map(|d| format!("{}: {}", d.file.display(), d.message)).collect()
}

fn line_of(file: &Path, offset: usize) -> usize {
    let text = fs::read_to_string(file).unwrap();
    LineIndex::new(&text).line_col(offset).0 + 1
}

#[test]
fn a_module_is_checked_with_its_project_never_alone() {
    let (main, _globals, _server, ui) = project("alone");
    // Alone (no project, its main file not open): Bump and Counter are
    // nobody's — the false errors Robert saw in module files.
    let mut a = Analysis::new(Options::default());
    a.update(ui.clone(), UI);
    assert!(!errors(&a.diagnostics(&ui)).is_empty(), "checked alone, the module has errors");
    // As part of its project (main.rr not open, read from the disk): none.
    let mut a = Analysis::new(Options { project: Some(main.clone()), ..Options::default() });
    a.update(ui.clone(), UI);
    assert_eq!(errors(&a.diagnostics(&ui)), Vec::<String>::new());
    assert_eq!(errors(&a.project_diagnostics(&main)), Vec::<String>::new());
    // A real error in the module is still reported, in the module.
    let broken = "SUB ShowCount\n    Bump\n    Nope 1\nEND SUB\n";
    a.update(ui.clone(), broken);
    let e = a.diagnostics(&ui);
    assert!(e.iter().any(|d| d.severity == Severity::Error && d.file == ui && d.message.contains("Nope")), "{e:?}");
    let _ = fs::remove_dir_all(main.parent().unwrap());
}

#[test]
fn references_and_definitions_cross_the_project_files() {
    let (main, globals, server, ui) = project("refs");
    let mut a = Analysis::new(Options { project: Some(main.clone()), ..Options::default() });
    // Only the module is open, as when Robert asked "where is this used?"
    a.update(ui.clone(), UI);
    let at = UI.find("Counter").unwrap() + 2;
    let refs = a.references(&ui, at, true);
    let mut files: Vec<String> = refs.iter().map(|r| r.file.file_name().unwrap().to_string_lossy().to_string()).collect();
    files.sort();
    files.dedup();
    assert_eq!(files, ["globals.rr", "main.rr", "server.rr", "ui.rr"], "{refs:?}");
    // DIM in globals.rr, two in server.rr, one in ui.rr, one in main.rr
    assert_eq!(refs.len(), 5, "{refs:?}");
    // Go to Definition: the DIM in globals.rr, from ui.rr and from main.rr.
    let def = a.definition(&ui, at);
    assert_eq!(def.len(), 1);
    assert_eq!(def[0].file, globals);
    assert_eq!(line_of(&globals, def[0].start), 1);
    // A SUB in another module: Bump, defined in server.rr.
    let def = a.definition(&ui, UI.find("Bump").unwrap() + 1);
    assert_eq!(def.len(), 1, "{def:?}");
    assert_eq!(def[0].file, server);
    let bump_refs = a.references(&ui, UI.find("Bump").unwrap() + 1, true);
    assert_eq!(bump_refs.len(), 2, "the SUB and its call: {bump_refs:?}");
    // A component: Srv, declared in globals.rr, used in server.rr.
    let srv = a.references(&server, SERVER.find("Srv").unwrap() + 1, true);
    assert_eq!(srv.iter().filter(|r| r.file == server).count(), 3, "{srv:?}");
    assert_eq!(srv.iter().filter(|r| r.file == globals).count(), 1, "{srv:?}");
    let _ = fs::remove_dir_all(main.parent().unwrap());
}

#[test]
fn portability_notes_follow_the_targets_once_per_component() {
    let (main, _globals, _server, _ui) = project("targets");
    let notes = |targets: Targets| {
        let mut a = Analysis::new(Options { project: Some(main.clone()), targets, ..Options::default() });
        a.project_diagnostics(&main).into_iter().filter(|d| d.message.contains("works on the desktop only")).map(|d| d.message).collect::<Vec<_>>()
    };
    // A desktop project: nothing to say.
    assert_eq!(notes(Targets::Desktop), Vec::<String>::new());
    // On the web: one note for RServerSocket, however many uses (the type,
    // Start and Stop: 3; Port works everywhere).
    let web = notes(Targets::Web);
    assert_eq!(web.len(), 1, "{web:?}");
    assert!(web[0].starts_with("RServerSocket works on the desktop only") && web[0].ends_with("(3 uses)"), "{web:?}");
    assert_eq!(notes(Targets::Both), web);
    // The per-file diagnostics (the editor's squiggles) follow the targets too.
    let mut a = Analysis::new(Options { project: Some(main.clone()), targets: Targets::Desktop, ..Options::default() });
    assert!(!a.diagnostics(&main).iter().any(|d| d.message.contains("desktop only")));
    let _ = fs::remove_dir_all(main.parent().unwrap());
}

#[test]
fn problems_are_errors_first_grouped_by_file() {
    let (main, _globals, server, ui) = project("order");
    let mut a = Analysis::new(Options { project: Some(main.clone()), targets: Targets::Web, ..Options::default() });
    // An error in ui.rr (a module included last): its file comes first.
    a.update(ui.clone(), "SUB ShowCount\n    Bump\n    Nope 1\nEND SUB\n");
    let d = a.project_diagnostics(&main);
    assert!(!d.is_empty());
    assert_eq!(d[0].severity, Severity::Error, "{d:?}");
    assert_eq!(d[0].file, ui, "{d:?}");
    // grouped: once a file's problems end, the file doesn't come back
    let mut seen: Vec<&Path> = Vec::new();
    for x in &d {
        if seen.last() != Some(&x.file.as_path()) {
            assert!(!seen.contains(&x.file.as_path()), "{} twice: {d:?}", x.file.display());
            seen.push(&x.file);
        }
    }
    // in a file: errors, then warnings, then notes
    for f in &seen {
        let ranks: Vec<u8> = d.iter().filter(|x| x.file == *f).map(|x| x.severity as u8).collect();
        assert!(ranks.windows(2).all(|w| w[0] <= w[1]), "{ranks:?}");
    }
    let _ = server;
    let _ = fs::remove_dir_all(main.parent().unwrap());
}

//! Security regression SEC-37 (docs/security/audit-2026-11-05-web.md): the
//! files RapidR Studio sends a browser's program (`project::program_files`,
//! the web's resolver: the run frame and the web builds get them) are the
//! program's folder's only. An `$INCLUDE "..\x.inc"` stays RapidQ's rule on
//! the desktop — the RapidQ examples have one (QAnimate's
//! `..\include\Memory.inc`) — so the desktop's project listing still names
//! it; the web's resolver doesn't read it. Compiled into rapidr-studio's
//! tests (`cargo test -p rapidr-studio security_`).

use crate::project::program_files;

#[test]
fn sec37_a_browsers_program_gets_only_its_folders_files() {
    let base = std::env::temp_dir().join(format!("rapidr-sec37-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let (project, outside) = (base.join("project"), base.join("outside"));
    std::fs::create_dir_all(project.join("inc")).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(
        project.join("main.bas"),
        "$INCLUDE \"inc\\ok.inc\"\n$INCLUDE \"..\\outside\\secret.inc\"\n$INCLUDE \"../outside/other.inc\"\nPRINT 1\n",
    )
    .unwrap();
    std::fs::write(project.join("inc/ok.inc"), "' a library of the project's\n").unwrap();
    std::fs::write(outside.join("secret.inc"), "' not the program's\n").unwrap();
    std::fs::write(outside.join("other.inc"), "' not the program's either\n").unwrap();
    let main = project.join("main.bas").to_string_lossy().replace('\\', "/");

    let files = program_files(&main, &|_| Vec::new()).expect("the program's files");
    let names: Vec<&str> = files.sources.iter().map(|(p, _)| p.as_str()).collect();
    assert!(names.iter().any(|n| n.eq_ignore_ascii_case("inc/ok.inc")), "the project's include is sent: {names:?}");
    assert!(!names.iter().any(|n| n.contains("..") || n.to_lowercase().contains("secret") || n.to_lowercase().contains("other")), "nothing outside the folder: {names:?}");
    assert!(!files.sources.iter().any(|(_, t)| t.contains("not the program")), "no outside file's text");

    // The desktop's listing keeps RapidQ's rule: the include beside the
    // project's folder is one of its files.
    let desktop = rapidr_project::implicit_from_resolver("main.bas", &|rel: &str| {
        std::fs::read_to_string(project.join(rel)).ok().map(|t| (rel.to_string(), t))
    });
    assert!(desktop.files.iter().any(|f| f.path == "../outside/secret.inc"), "{:?}", desktop.files.iter().map(|f| &f.path).collect::<Vec<_>>());
    let _ = std::fs::remove_dir_all(&base);
}

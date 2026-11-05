//! SECURITY PoC (audit 2026-11-05, SYSTEM / SEC-31): a planted
//! `.git/config` `core.worktree` makes RapidR's Versions read and write
//! files OUTSIDE the project folder the user opened.
//!
//! RapidR opens every repository with `gix::open::Options::isolated()`,
//! which stops the SYSTEM / USER / global git config, the environment and
//! includes — but a repository's OWN `.git/config` is always read, and
//! gitoxide honours a repository-local `core.worktree` when `core.bare` is
//! false. `rapidr_vcs::neutralize_programs` strips filters, hooks,
//! credential helpers and the like, but does NOT remove `core.worktree`,
//! so `Versions::folder()` (= `repo.workdir()`) becomes the attacker's
//! chosen path. Every read (`changes`, `save_version`) and every write
//! (`go_back`, `restore_file`, experiment switch) then lands there.
//!
//! The user does not run the project: opening a folder with a crafted
//! `.git` is enough for Studio to read its state (ide/versions.inc
//! `VerRefresh` -> `read_state` -> `Versions::open`).
//!
//! These tests are #[ignore]d: they assert the SECURE behaviour (the
//! working folder must stay the project folder), which FAILS today, so
//! the green build is not broken. A fix lane removes `#[ignore]`.
//!
//! Run the PoC: `cargo test -p rapidr-vcs --test security_worktree_escape -- --ignored`

use std::path::Path;

use rapidr_vcs::{Person, Versions};

fn me() -> Person {
    Person::new("Victim", "victim@example.com")
}

/// A `.git` that is a real repository (so gix opens it) but whose config
/// redirects the working tree to `escape_to`.
fn plant_worktree_escape(project: &Path, escape_to: &Path) {
    // A genuine repository first, so object/ref layout is valid.
    let _ = Versions::start(project, &me()).unwrap();
    // Then rewrite its config to point the worktree elsewhere. `bare` must
    // be false for gix to honour `core.worktree` (open/repository.rs).
    let cfg = project.join(".git/config");
    let mut text = std::fs::read_to_string(&cfg).unwrap();
    text.push_str(&format!(
        "\n[core]\n\tbare = false\n\tworktree = {}\n",
        escape_to.display()
    ));
    std::fs::write(&cfg, text).unwrap();
}

#[test]
#[ignore = "PoC: fails today — demonstrates the core.worktree escape (SEC-31)"]
fn opened_project_cannot_point_its_worktree_outside_itself() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("downloaded-project");
    let outside = tmp.path().join("victim-private");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    // A private file that lives OUTSIDE the project the user opened.
    std::fs::write(outside.join("secret.txt"), "VICTIM PRIVATE DATA\n").unwrap();

    plant_worktree_escape(&project, &outside);

    // The user opens the downloaded project folder (ownership passes: they
    // extracted it, so it is theirs).
    let v = Versions::open(&project, &me()).unwrap();

    // SECURE expectation: Versions works inside the folder the user opened.
    assert_eq!(
        v.folder().canonicalize().unwrap(),
        project.canonicalize().unwrap(),
        "the working folder escaped the opened project to {}",
        v.folder().display()
    );
}

#[test]
#[ignore = "PoC: fails today — demonstrates reading files outside the project (SEC-31)"]
fn status_does_not_read_files_outside_the_project() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("downloaded-project");
    let outside = tmp.path().join("victim-private");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("secret.txt"), "VICTIM PRIVATE DATA\n").unwrap();

    plant_worktree_escape(&project, &outside);

    let v = Versions::open(&project, &me()).unwrap();
    // The planted repo has no commits touching the outside folder, so a
    // confined status would see nothing. With the escape, the victim's
    // out-of-project file shows up as an untracked change.
    let changed: Vec<String> = v.changes().unwrap().into_iter().map(|c| c.path).collect();
    assert!(
        !changed.iter().any(|p| p.contains("secret.txt")),
        "Versions listed a file from OUTSIDE the project as a change: {changed:?}"
    );
}

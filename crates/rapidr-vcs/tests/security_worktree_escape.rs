//! SEC-31 (docs/security/audit-2026-11-05-system.md): a project folder
//! that came with its own `.git` must never make Versions read or write
//! files OUTSIDE the folder the user opened.
//!
//! The audit's PoC (a planted `.git/config` `core.worktree`) is the first
//! two tests, now asserting the fix: the project isn't opened for changes
//! nor read-only ([`ErrorKind::OutsideFolder`], [`HISTORY_OUTSIDE`]), so
//! nothing outside is listed, read or written. The others are the other
//! ways a repository's own files point gitoxide elsewhere: a relative
//! `core.worktree`, a `.git` that is a file (`gitdir:`) or a link, a
//! `commondir` file, `gitoxide.core.indexFile`, `objects/info/alternates`,
//! links inside `.git` (reflogs and the history of actions are appended
//! to), a shared folder's alternates — and what must keep working: a
//! `core.worktree` naming the project itself, an `include.path` (never
//! followed), a planted `core.excludesFile` (stripped).

#![cfg(unix)]

use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use rapidr_vcs::{create_shared_folder, write_ignore_file, ErrorKind, Person, Versions, HISTORY_OUTSIDE, IGNORE_LINK, SHARED_OUTSIDE};

fn me() -> Person {
    Person::new("Victim", "victim@example.com")
}

/// A downloaded project (with one version) and a private folder beside it.
fn setup() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("downloaded-project");
    let outside = tmp.path().join("victim-private");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    // A private file that lives OUTSIDE the project the user opened.
    std::fs::write(outside.join("secret.txt"), "VICTIM PRIVATE DATA\n").unwrap();
    let mut v = Versions::start(&project, &me()).unwrap();
    std::fs::write(project.join("main.rr"), "PRINT 1\n").unwrap();
    v.save_version("First", &me()).unwrap();
    (tmp, project, outside)
}

fn append_config(project: &Path, text: &str) {
    let cfg = project.join(".git/config");
    let mut all = std::fs::read_to_string(&cfg).unwrap();
    all.push_str(text);
    std::fs::write(&cfg, all).unwrap();
}

/// A `.git` that is a real repository (so gix opens it) but whose config
/// redirects the working tree to `escape_to`. `bare` must be false for
/// gix to honour `core.worktree` (open/repository.rs).
fn plant_worktree_escape(project: &Path, escape_to: &str) {
    append_config(project, &format!("\n[core]\n\tbare = false\n\tworktree = {escape_to}\n"));
}

/// Every file under `dir` with its contents, sorted: what must not change.
fn snapshot(dir: &Path) -> Vec<(String, Vec<u8>)> {
    fn walk(dir: &Path, base: &Path, out: &mut Vec<(String, Vec<u8>)>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            let rel = p.strip_prefix(base).unwrap().to_string_lossy().into_owned();
            let kind = e.file_type().unwrap();
            if kind.is_dir() {
                walk(&p, base, out);
            } else if kind.is_symlink() {
                out.push((rel, std::fs::read_link(&p).unwrap().to_string_lossy().into_owned().into_bytes()));
            } else {
                out.push((rel, std::fs::read(&p).unwrap()));
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

/// Opening `project` is refused for changes AND read-only, in plain words,
/// with `what` in the detail.
fn refused(project: &Path, what: &str) {
    for e in [Versions::open(project, &me()).err(), Versions::open_read_only(project, &me()).err(), Versions::start(project, &me()).err()] {
        let e = e.expect("a project whose history points outside its folder must not be opened");
        assert_eq!(e.kind, ErrorKind::OutsideFolder, "{e} / {}", e.detail());
        assert_eq!(e.message, HISTORY_OUTSIDE);
        assert!(e.detail().contains(what), "the detail says what was found: {}", e.detail());
    }
}

#[test]
fn opened_project_cannot_point_its_worktree_outside_itself() {
    let (_tmp, project, outside) = setup();
    let before = snapshot(&outside);
    plant_worktree_escape(&project, &outside.display().to_string());

    // The user opens the downloaded project folder (ownership passes: they
    // extracted it, so it is theirs). SECURE: Versions refuses it rather
    // than working on the folder core.worktree names.
    refused(&project, "core.worktree");
    assert_eq!(snapshot(&outside), before, "nothing outside was written");
}

#[test]
fn status_does_not_read_files_outside_the_project() {
    let (_tmp, project, outside) = setup();
    plant_worktree_escape(&project, &outside.display().to_string());
    // No Versions means no status: the outside file can't be listed (the
    // PoC's `changes()` returned ["secret.txt"]).
    match Versions::open(&project, &me()) {
        Ok(v) => {
            let changed: Vec<String> = v.changes().unwrap().into_iter().map(|c| c.path).collect();
            panic!("opened; changes: {changed:?}");
        }
        Err(e) => assert_eq!(e.kind, ErrorKind::OutsideFolder),
    }
}

#[test]
fn a_relative_worktree_climbing_out_is_refused() {
    let (_tmp, project, _outside) = setup();
    // (relative to the .git folder: ../../victim-private)
    plant_worktree_escape(&project, "../../victim-private");
    refused(&project, "core.worktree");
}

#[test]
fn a_worktree_naming_the_project_itself_still_opens() {
    let (_tmp, project, _outside) = setup();
    plant_worktree_escape(&project, &project.display().to_string());
    let v = Versions::open(&project, &me()).unwrap();
    assert_eq!(v.folder().canonicalize().unwrap(), project.canonicalize().unwrap());
    assert_eq!(v.timeline(5).unwrap().len(), 1);
}

#[test]
fn a_git_file_or_link_naming_another_history_is_refused() {
    let (tmp, project, _outside) = setup();
    // another project of the victim's, with its own versions
    let other = tmp.path().join("victims-other-project");
    std::fs::create_dir_all(&other).unwrap();
    let mut o = Versions::start(&other, &me()).unwrap();
    std::fs::write(other.join("notes.txt"), "mine\n").unwrap();
    o.save_version("Mine", &me()).unwrap();
    drop(o);
    let other_before = snapshot(&other.join(".git"));

    // `.git` as a file: gitdir: <the other history>
    std::fs::remove_dir_all(project.join(".git")).unwrap();
    std::fs::write(project.join(".git"), format!("gitdir: {}\n", other.join(".git").display())).unwrap();
    refused(&project, "not a folder of the project's own");

    // `.git` as a link to it
    std::fs::remove_file(project.join(".git")).unwrap();
    symlink(other.join(".git"), project.join(".git")).unwrap();
    refused(&project, "not a folder of the project's own");

    // a `.git` link that leads nowhere: Start doesn't create a history
    // through it
    std::fs::remove_file(project.join(".git")).unwrap();
    let nowhere = tmp.path().join("would-be-created");
    symlink(&nowhere, project.join(".git")).unwrap();
    let e = Versions::start(&project, &me()).err().unwrap();
    assert_eq!(e.kind, ErrorKind::OutsideFolder);
    assert!(!nowhere.exists(), "nothing was created through the link");

    assert_eq!(snapshot(&other.join(".git")), other_before, "the other history wasn't touched");
}

#[test]
fn a_commondir_file_is_refused() {
    let (tmp, project, _outside) = setup();
    let other = tmp.path().join("other");
    std::fs::create_dir_all(&other).unwrap();
    Versions::start(&other, &me()).unwrap();
    std::fs::write(project.join(".git/commondir"), format!("{}\n", other.join(".git").display())).unwrap();
    refused(&project, "commondir");
}

#[test]
fn an_index_file_outside_is_refused() {
    let (_tmp, project, outside) = setup();
    let target = outside.join("overwritten-by-a-save");
    append_config(&project, &format!("\n[gitoxide \"core\"]\n\tindexFile = {}\n", target.display()));
    refused(&project, "indexFile");
    assert!(!target.exists());
}

#[test]
fn alternates_reading_versions_from_elsewhere_are_refused() {
    let (tmp, project, _outside) = setup();
    let other = tmp.path().join("other");
    std::fs::create_dir_all(&other).unwrap();
    Versions::start(&other, &me()).unwrap();
    std::fs::create_dir_all(project.join(".git/objects/info")).unwrap();
    std::fs::write(project.join(".git/objects/info/alternates"), format!("# a comment\n{}\n", other.join(".git/objects").display())).unwrap();
    refused(&project, "alternates");
}

#[test]
fn links_inside_the_history_that_lead_out_are_refused() {
    // the history of actions (appended to on every action)
    let (_tmp, project, outside) = setup();
    let before = snapshot(&outside);
    std::fs::remove_dir_all(project.join(".git/rapidr")).unwrap();
    symlink(&outside, project.join(".git/rapidr")).unwrap();
    refused(&project, "rapidr");
    assert_eq!(snapshot(&outside), before);

    // a reflog (appended to when the branch moves): to a shell's start-up
    // file, say
    let (_tmp, project, outside) = setup();
    let profile = outside.join("profile");
    std::fs::write(&profile, "# the victim's\n").unwrap();
    let log = project.join(".git/logs/refs/heads/main");
    std::fs::remove_file(&log).unwrap();
    symlink(&profile, &log).unwrap();
    refused(&project, "main");
    assert_eq!(std::fs::read_to_string(&profile).unwrap(), "# the victim's\n");

    // a folder of loose objects
    let (_tmp, project, outside) = setup();
    let objects = project.join(".git/objects");
    let some = std::fs::read_dir(&objects).unwrap().flatten().map(|e| e.path()).find(|p| p.file_name().unwrap().len() == 2).unwrap();
    let moved = outside.join("objects-moved");
    std::fs::rename(&some, &moved).unwrap();
    symlink(&moved, &some).unwrap();
    refused(&project, "objects");

    // a link that stays inside the history is fine
    let (_tmp, project, _outside) = setup();
    symlink(project.join(".git/HEAD"), project.join(".git/HEAD-link")).unwrap();
    Versions::open(&project, &me()).unwrap();
}

#[test]
fn an_include_is_never_followed() {
    let (_tmp, project, outside) = setup();
    // an included file that would move the working folder
    let inc = outside.join("planted.config");
    std::fs::write(&inc, format!("[core]\n\tbare = false\n\tworktree = {}\n", outside.display())).unwrap();
    append_config(&project, &format!("\n[include]\n\tpath = {}\n[includeIf \"gitdir:/\"]\n\tpath = {}\n", inc.display(), inc.display()));
    let v = Versions::open(&project, &me()).unwrap();
    assert_eq!(v.folder().canonicalize().unwrap(), project.canonicalize().unwrap());
    assert!(v.changes().unwrap().is_empty());
}

#[test]
fn a_planted_excludes_file_is_not_read() {
    let (_tmp, project, outside) = setup();
    // a file outside that would hide every new .rr file
    let excludes = outside.join("ignore-everything");
    std::fs::write(&excludes, "*.rr\n").unwrap();
    append_config(&project, &format!("\n[core]\n\texcludesFile = {}\n\tattributesFile = {}\n", excludes.display(), excludes.display()));
    std::fs::write(project.join("new.rr"), "PRINT 2\n").unwrap();
    let v = Versions::open(&project, &me()).unwrap();
    let changed: Vec<String> = v.changes().unwrap().into_iter().map(|c| c.path).collect();
    assert_eq!(changed, ["new.rr"], "core.excludesFile from the project's own config isn't used");
}

#[test]
fn a_shared_folder_with_alternates_is_refused() {
    let (tmp, project, _outside) = setup();
    let shared = tmp.path().join("usb/class.git");
    create_shared_folder(&shared).unwrap();
    let mut v = Versions::open(&project, &me()).unwrap();
    std::fs::write(shared.join("objects/info/alternates"), format!("{}\n", project.join(".git/objects").display())).unwrap();
    for e in [v.share_to_folder(&shared, "main").err(), v.get_latest_from_folder(&shared, "main", &me()).err()] {
        let e = e.unwrap();
        assert_eq!((e.kind, e.message.as_str()), (ErrorKind::OutsideFolder, SHARED_OUTSIDE), "{}", e.detail());
    }
}

#[test]
fn a_gitignore_that_is_a_link_is_not_written_through() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("p");
    std::fs::create_dir_all(&project).unwrap();
    let profile = tmp.path().join("profile");
    std::fs::write(&profile, "# the victim's\n").unwrap();
    symlink(&profile, project.join(".gitignore")).unwrap();
    let e = write_ignore_file(&project).err().unwrap();
    assert_eq!((e.kind, e.message.as_str()), (ErrorKind::OutsideFolder, IGNORE_LINK));
    assert_eq!(std::fs::read_to_string(&profile).unwrap(), "# the victim's\n");
    // (one that leads nowhere: nothing created there either)
    std::fs::remove_file(project.join(".gitignore")).unwrap();
    symlink(tmp.path().join("created"), project.join(".gitignore")).unwrap();
    assert!(write_ignore_file(&project).is_err());
    assert!(!tmp.path().join("created").exists());
}

#[test]
fn what_changed_shows_a_link_not_the_file_it_points_at() {
    let (_tmp, project, outside) = setup();
    symlink(outside.join("secret.txt"), project.join("notes.txt")).unwrap();
    let v = Versions::open(&project, &me()).unwrap();
    let diffs = v.what_changed().unwrap();
    let shown: String = diffs.iter().flat_map(|d| d.hunks.iter()).flat_map(|h| h.lines.iter()).map(|l| format!("{l:?}")).collect();
    assert!(!shown.contains("VICTIM PRIVATE DATA"), "{shown}");
    assert!(shown.contains("secret.txt"), "the link's target is what a version keeps: {shown}");
    assert!(v.check_secrets(None).unwrap().is_empty());
}

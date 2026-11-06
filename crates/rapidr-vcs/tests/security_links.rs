//! SEC-32 (docs/security/audit-2026-11-05-system.md): a version from
//! someone else — Get latest from a shared folder, or a project that came
//! with its history — must not make a checkout write through a link, or
//! create a link that leads outside the project's folder.
//!
//! The audit's PoC: a commit with a link `a -> <outside>`, then `a/x`,
//! brought in by Get latest and by Go back. RapidR refuses the link before
//! anything is written ([`ErrorKind::OutsideFolder`]); gitoxide's own
//! checkout never writes through a link already in the folder (the last
//! tests keep that as proof). Links that stay inside keep working.

#![cfg(unix)]

use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use rapidr_vcs::{create_shared_folder, ErrorKind, GetLatest, Person, Versions};

fn ana() -> Person {
    Person::new("Ana", "ana@example.com")
}

fn ben() -> Person {
    Person::new("Ben", "ben@example.com")
}

fn dir(tmp: &tempfile::TempDir, name: &str) -> PathBuf {
    let d = tmp.path().join(name);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Every name under `d` (links not followed), sorted.
fn names(d: &Path) -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(d).map(|r| r.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect()).unwrap_or_default();
    out.sort();
    out
}

fn is_link(p: &Path) -> bool {
    std::fs::symlink_metadata(p).is_ok_and(|m| m.file_type().is_symlink())
}

fn refused_as_outside<T: std::fmt::Debug>(r: Result<T, rapidr_vcs::Error>, link: &str) {
    let e = r.map(|ok| panic!("a link that leads outside the project must be refused ({link}): {ok:?}")).unwrap_err();
    assert_eq!(e.kind, ErrorKind::OutsideFolder, "{e} / {}", e.detail());
    assert!(e.message.contains(link) && e.message.contains("Nothing was changed"), "{e}");
}

#[test]
fn get_latest_never_creates_a_link_leading_out() {
    let tmp = tempfile::tempdir().unwrap();
    let outside = dir(&tmp, "outside");
    let shared = tmp.path().join("usb/class.git");
    create_shared_folder(&shared).unwrap();

    // the class's first version, harmless; Ana gets it
    let evil = dir(&tmp, "evil");
    let mut e = Versions::start(&evil, &ben()).unwrap();
    std::fs::write(evil.join("main.rr"), "PRINT 1\n").unwrap();
    e.save_version("First", &ben()).unwrap();
    e.share_to_folder(&shared, "main").unwrap();
    let with_versions = dir(&tmp, "with-versions");
    let mut a = Versions::start(&with_versions, &ana()).unwrap();
    a.get_latest_from_folder(&shared, "main", &ana()).unwrap();
    // then a version with `a` -> the victim's folder
    symlink(&outside, evil.join("a")).unwrap();
    std::fs::write(evil.join("main.rr"), "PRINT 2\n").unwrap();
    e.save_version("Looks harmless", &ben()).unwrap();
    e.share_to_folder(&shared, "main").unwrap();

    // into a project that has the first (a fast-forward) and into an empty
    // one (no version yet)
    let empty = dir(&tmp, "empty");
    for (project, main) in [(with_versions, Some("PRINT 1\n")), (empty, None)] {
        let mut v = Versions::start(&project, &ana()).unwrap();
        let timeline = v.timeline(10).unwrap().len();
        refused_as_outside(v.get_latest_from_folder(&shared, "main", &ana()), "a");
        assert!(!is_link(&project.join("a")), "no link was created");
        assert_eq!(std::fs::read_to_string(project.join("main.rr")).ok().as_deref(), main, "nothing was written");
        assert_eq!(v.timeline(10).unwrap().len(), timeline, "no version moved");
        assert!(v.changes().unwrap().is_empty());
    }
    assert!(names(&outside).is_empty(), "nothing was written outside");
}

#[test]
fn a_link_made_to_lead_out_by_a_later_one_is_refused() {
    // `x -> d/..` with no `d` yet (dangling: lexically inside), then
    // `d -> .` (inside on its own): together x would be the folder above.
    let tmp = tempfile::tempdir().unwrap();
    let project = dir(&tmp, "project");
    let mut v = Versions::start(&project, &ana()).unwrap();
    std::fs::write(project.join("main.rr"), "PRINT 1\n").unwrap();
    let plain = v.save_version("Plain", &ana()).unwrap();
    symlink("d/..", project.join("x")).unwrap();
    let with_x = v.save_version("x", &ana()).unwrap();
    symlink(".", project.join("d")).unwrap();
    let with_both = v.save_version("x and d", &ana()).unwrap();
    v.go_back(&plain, &ana()).unwrap();
    assert_eq!(names(&project), [".git", "main.rr"]);
    refused_as_outside(v.go_back(&with_x, &ana()), "x");
    refused_as_outside(v.go_back(&with_both, &ana()), "x");
    assert_eq!(names(&project), [".git", "main.rr"]);
}

#[test]
fn go_back_never_creates_a_link_leading_out() {
    let tmp = tempfile::tempdir().unwrap();
    let outside = dir(&tmp, "outside");
    let project = dir(&tmp, "project");
    let mut v = Versions::start(&project, &ana()).unwrap();
    std::fs::write(project.join("main.rr"), "PRINT 1\n").unwrap();
    // absolute, relative climbing out, and through another link of the
    // same version (`d -> .` then `x -> d/..`: lexically inside, really
    // the folder above)
    symlink(&outside, project.join("abs")).unwrap();
    let first = v.save_version("With an absolute link", &ana()).unwrap();
    std::fs::remove_file(project.join("abs")).unwrap();
    symlink("../outside", project.join("rel")).unwrap();
    let second = v.save_version("With a relative one", &ana()).unwrap();
    std::fs::remove_file(project.join("rel")).unwrap();
    symlink(".", project.join("d")).unwrap();
    symlink("d/..", project.join("x")).unwrap();
    let third = v.save_version("Through another link", &ana()).unwrap();
    std::fs::remove_file(project.join("d")).unwrap();
    std::fs::remove_file(project.join("x")).unwrap();
    v.save_version("No links", &ana()).unwrap();

    for (version, link) in [(&first, "abs"), (&second, "rel"), (&third, "x")] {
        let timeline = v.timeline(10).unwrap().len();
        refused_as_outside(v.go_back(version, &ana()), link);
        assert_eq!(v.timeline(10).unwrap().len(), timeline, "no version was saved");
        assert_eq!(names(&project), [".git", "main.rr"]);
        refused_as_outside(v.go_back_file(link, version, &ana()), link);
        refused_as_outside(v.restore_file(link, version), link);
        assert_eq!(names(&project), [".git", "main.rr"]);
    }
    assert!(names(&outside).is_empty());
}

#[test]
fn links_inside_the_project_keep_working() {
    let tmp = tempfile::tempdir().unwrap();
    let project = dir(&tmp, "project");
    let mut v = Versions::start(&project, &ana()).unwrap();
    std::fs::create_dir_all(project.join("docs/old")).unwrap();
    std::fs::write(project.join("docs/notes.txt"), "notes\n").unwrap();
    symlink("docs/notes.txt", project.join("latest")).unwrap();
    symlink("../notes.txt", project.join("docs/old/up")).unwrap();
    let with = v.save_version("Links inside", &ana()).unwrap();
    std::fs::remove_file(project.join("latest")).unwrap();
    std::fs::remove_file(project.join("docs/old/up")).unwrap();
    v.save_version("Without", &ana()).unwrap();
    v.go_back(&with, &ana()).unwrap();
    assert_eq!(std::fs::read_link(project.join("latest")).unwrap(), Path::new("docs/notes.txt"));
    assert_eq!(std::fs::read_to_string(project.join("docs/old/up")).unwrap(), "notes\n");
    // and Undo puts the folder back
    v.undo().unwrap();
    assert!(!is_link(&project.join("latest")));
}

#[test]
fn a_link_already_in_the_folder_is_never_written_through() {
    // gitoxide's checkout: a folder of the path that is a link is replaced
    // by a real folder (or refused), never followed.
    let tmp = tempfile::tempdir().unwrap();
    let outside = dir(&tmp, "outside");
    let project = dir(&tmp, "project");
    let mut v = Versions::start(&project, &ana()).unwrap();
    std::fs::create_dir_all(project.join("a")).unwrap();
    std::fs::write(project.join("a/x"), "the project's\n").unwrap();
    let with_file = v.save_version("a/x", &ana()).unwrap();
    std::fs::remove_dir_all(project.join("a")).unwrap();
    v.save_version("No a", &ana()).unwrap();
    // planted: `a` -> the victim's folder (untracked, as a project could
    // come with it)
    symlink(&outside, project.join("a")).unwrap();
    // (Go back needs nothing unsaved: the link is new, so it's saved first
    // — as a link, never as what it points at)
    v.save_version("The link", &ana()).unwrap();
    let r = v.go_back(&with_file, &ana());
    assert!(names(&outside).is_empty(), "nothing written through the link: {r:?}");
    if r.is_ok() {
        assert!(!is_link(&project.join("a")) && std::fs::read_to_string(project.join("a/x")).unwrap() == "the project's\n");
    }
}

#[test]
fn a_file_is_never_removed_through_a_link() {
    let tmp = tempfile::tempdir().unwrap();
    let outside = dir(&tmp, "outside");
    std::fs::write(outside.join("x"), "the victim's\n").unwrap();
    let project = dir(&tmp, "project");
    let mut v = Versions::start(&project, &ana()).unwrap();
    std::fs::write(project.join("main.rr"), "PRINT 1\n").unwrap();
    let without = v.save_version("No d", &ana()).unwrap();
    std::fs::create_dir_all(project.join("d")).unwrap();
    std::fs::write(project.join("d/x"), "mine\n").unwrap();
    v.save_version("d/x", &ana()).unwrap();
    // the folder `d` replaced by a link to the victim's folder, which has
    // its own `x`
    std::fs::remove_dir_all(project.join("d")).unwrap();
    symlink(&outside, project.join("d")).unwrap();
    // restoring d/x as the first version had it (none) must not remove
    // the victim's x
    let _ = v.restore_file("d/x", &without);
    assert_eq!(std::fs::read_to_string(outside.join("x")).unwrap(), "the victim's\n");
}

#[test]
fn get_latest_of_a_later_file_under_the_link_writes_nothing_outside() {
    // the audit's two-step: `a` a link in one version, `a/x` a file in the
    // next — the first is refused; with the link put there by hand, the
    // second still writes nothing through it.
    let tmp = tempfile::tempdir().unwrap();
    let outside = dir(&tmp, "outside");
    let shared = tmp.path().join("usb/class.git");
    create_shared_folder(&shared).unwrap();
    let evil = dir(&tmp, "evil");
    let mut e = Versions::start(&evil, &ben()).unwrap();
    std::fs::create_dir_all(evil.join("a")).unwrap();
    std::fs::write(evil.join("a/x"), "planted\n").unwrap();
    e.save_version("a/x", &ben()).unwrap();
    e.share_to_folder(&shared, "main").unwrap();

    let project = dir(&tmp, "project");
    let mut v = Versions::start(&project, &ana()).unwrap();
    symlink(&outside, project.join("a")).unwrap();
    v.save_version("A link of mine", &ana()).unwrap();
    let r = v.get_latest_from_folder(&shared, "main", &ana());
    assert!(names(&outside).is_empty(), "nothing written through the link: {r:?}");
    if let Ok(GetLatest::Merged { .. } | GetLatest::FastForward { .. }) = r {
        assert!(!is_link(&project.join("a")));
    }
}

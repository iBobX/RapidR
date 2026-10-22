//! Lane G1's acceptance, on temporary projects: save, the timeline, Undo,
//! safety points, going back and restoring, RAI checkpoints, the secret
//! scan, experiments that rewrite only what differs, getting the latest
//! from a shared folder. Every repository written is checked by the
//! system's `git` when it is installed (`git fsck --strict`, `git status`,
//! `git log`): the product never runs `git`, the tests use it as an
//! independent judge. (The read-only open of another account's project is
//! a unit test: src/tests.rs.)

use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use rapidr_vcs::{create_shared_folder, ErrorKind, FileState, GetLatest, Person, SaveOptions, Share, Versions};

fn ana() -> Person {
    Person::new("Ana", "ana@example.com")
}

fn rai() -> Person {
    Person::new("RAI", "rai@rapidr.invalid")
}

fn write(dir: &Path, rela: &str, text: &str) {
    let path = dir.join(rela);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, text).unwrap();
}

fn read(dir: &Path, rela: &str) -> String {
    std::fs::read_to_string(dir.join(rela)).unwrap()
}

/// The system `git`, isolated from this machine's configuration; `None`
/// when it isn't installed (the checks are then skipped).
fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let home = dir.parent().unwrap_or(dir);
    let out = Command::new("git").args(args).current_dir(dir).env("GIT_CONFIG_NOSYSTEM", "1").env("GIT_CONFIG_GLOBAL", "/dev/null").env("HOME", home).env("LC_ALL", "C").output().ok()?;
    assert!(out.status.success(), "git {args:?} failed in {}:\n{}{}", dir.display(), String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The repository is valid and git sees what Versions says.
fn fsck(dir: &Path) {
    git(dir, &["fsck", "--full", "--strict", "--no-dangling"]);
}

fn project(tmp: &tempfile::TempDir, name: &str) -> PathBuf {
    let dir = tmp.path().join(name);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A file's identity on disk: rewritten files get a new one.
fn stamp(dir: &Path, rela: &str) -> (u64, i64, i64) {
    let m = std::fs::metadata(dir.join(rela)).unwrap();
    (m.ino(), m.mtime(), m.mtime_nsec())
}

#[test]
fn save_timeline_and_undo() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = project(&tmp, "app");
    let mut v = Versions::start(&dir, &ana()).unwrap();
    write(&dir, "main.rr", "PRINT 1\n");
    let one = v.save_version("One", &ana()).unwrap();
    write(&dir, "main.rr", "PRINT 2\n");
    write(&dir, "notes.txt", "remember\n");
    // only one of the two changed files
    let two = v.save_version_with("Two", &ana(), &SaveOptions { files: Some(vec!["main.rr".into()]), allow_secrets: false }).unwrap();
    assert_eq!(v.changes().unwrap().iter().map(|c| (c.path.as_str(), c.state)).collect::<Vec<_>>(), [("notes.txt", FileState::New)]);
    let t: Vec<String> = v.timeline(10).unwrap().iter().map(|x| x.message.clone()).collect();
    assert_eq!(t, ["Two", "One"]);
    if let Some(log) = git(&dir, &["log", "--format=%s"]) {
        assert_eq!(log, "Two\nOne\n");
        assert_eq!(git(&dir, &["status", "--porcelain"]).unwrap(), "?? notes.txt\n");
        fsck(&dir);
    }

    // Undo the last save: the version goes, its changes stay as unsaved
    assert_eq!(v.can_undo().unwrap().as_deref(), Some("Saved the version \u{201C}Two\u{201D}"));
    assert_eq!(v.undo().unwrap(), "Saved the version \u{201C}Two\u{201D}");
    assert_eq!(v.timeline(10).unwrap()[0].id, one);
    assert_eq!(read(&dir, "main.rr"), "PRINT 2\n", "nothing is lost");
    let mut changed: Vec<String> = v.changes().unwrap().into_iter().map(|c| c.path).collect();
    changed.sort();
    assert_eq!(changed, ["main.rr", "notes.txt"]);
    // the history says it, and the undo is in it too
    let h = v.history().unwrap();
    assert_eq!(h.iter().map(|a| (a.kind.as_str(), a.undone)).collect::<Vec<_>>(), [("save", false), ("save", true), ("undo", false)]);
    // the next Undo takes back the one before
    assert_eq!(v.undo().unwrap(), "Saved the version \u{201C}One\u{201D}");
    assert!(v.timeline(10).unwrap().is_empty());
    assert_eq!(v.undo().err().unwrap().kind, ErrorKind::NothingToUndo);
    let again = v.save_version("Both", &ana()).unwrap();
    assert_ne!(again, two);
    if git(&dir, &["status", "--porcelain"]).is_some() {
        assert_eq!(git(&dir, &["status", "--porcelain"]).unwrap(), "");
        assert_eq!(git(&dir, &["log", "--format=%s"]).unwrap(), "Both\n");
        fsck(&dir);
    }
}

#[test]
fn undo_refuses_when_the_project_moved_on() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = project(&tmp, "app");
    let mut v = Versions::start(&dir, &ana()).unwrap();
    write(&dir, "a.txt", "1\n");
    v.save_version("One", &ana()).unwrap();
    v.start_experiment("idea").unwrap();
    write(&dir, "a.txt", "idea\n");
    v.save_version("Idea", &ana()).unwrap();
    v.switch_to("main").unwrap();
    assert_eq!(read(&dir, "a.txt"), "1\n");
    // edited after the switch: undoing the switch would overwrite it
    write(&dir, "a.txt", "newer work\n");
    let e = v.undo().err().unwrap();
    assert_eq!(e.kind, ErrorKind::UnsavedChanges);
    assert!(e.message.contains("a.txt"), "{e}");
    assert_eq!(read(&dir, "a.txt"), "newer work\n", "nothing was changed");
    assert_eq!(v.current_experiment().unwrap(), "main");
    // put back as it was: the switch can be undone
    write(&dir, "a.txt", "1\n");
    assert_eq!(v.undo().unwrap(), "Switched to the experiment \u{201C}main\u{201D}");
    assert_eq!(v.current_experiment().unwrap(), "idea");
    assert_eq!(read(&dir, "a.txt"), "idea\n");
    if git(&dir, &["status", "--porcelain"]).is_some() {
        assert_eq!(git(&dir, &["status", "--porcelain"]).unwrap(), "");
        assert_eq!(git(&dir, &["branch", "--show-current"]).unwrap(), "idea\n");
        fsck(&dir);
    }
}

#[test]
fn go_back_restore_and_safety_points() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = project(&tmp, "app");
    let mut v = Versions::start(&dir, &ana()).unwrap();
    write(&dir, "main.rr", "PRINT \"one\"\n");
    write(&dir, "form.rr", "CREATE Form AS RForm\nEND CREATE\n");
    let one = v.save_version("One", &ana()).unwrap();
    write(&dir, "main.rr", "PRINT \"two\"\n");
    std::fs::remove_file(dir.join("form.rr")).unwrap();
    write(&dir, "extra.rr", "' new\n");
    let two = v.save_version("Two", &ana()).unwrap();

    // restore one file as it was, as an unsaved change
    v.restore_file("main.rr", &one).unwrap();
    assert_eq!(read(&dir, "main.rr"), "PRINT \"one\"\n");
    assert_eq!(v.changes().unwrap().len(), 1);
    assert_eq!(v.timeline(1).unwrap()[0].id, two, "no version moved");
    // its safety point has the file as it was before
    let sp = &v.safety_points().unwrap()[0];
    assert!(sp.label.starts_with("Before: Restored main.rr"), "{}", sp.label);
    assert_eq!(v.file_at("main.rr", &sp.id).unwrap().unwrap(), b"PRINT \"two\"\n");
    v.undo().unwrap();
    assert_eq!(read(&dir, "main.rr"), "PRINT \"two\"\n");
    assert!(v.changes().unwrap().is_empty());

    // go back with unsaved work: refused, nothing touched
    write(&dir, "extra.rr", "' edited\n");
    assert_eq!(v.go_back(&one, &ana()).err().unwrap().kind, ErrorKind::UnsavedChanges);
    assert_eq!(read(&dir, "extra.rr"), "' edited\n");
    write(&dir, "extra.rr", "' new\n");

    // go back to One: a new version with One's files
    let back = v.go_back(&one, &ana()).unwrap();
    let tl = v.timeline(5).unwrap();
    assert_eq!(tl.iter().map(|x| x.id.as_str()).collect::<Vec<_>>(), [back.as_str(), two.as_str(), one.as_str()], "Two stays on the timeline");
    assert!(tl[0].message.starts_with("Go back to \u{201C}One\u{201D}"));
    assert_eq!(read(&dir, "main.rr"), "PRINT \"one\"\n");
    assert!(dir.join("form.rr").exists() && !dir.join("extra.rr").exists());
    assert!(v.changes().unwrap().is_empty());
    if git(&dir, &["status", "--porcelain"]).is_some() {
        assert_eq!(git(&dir, &["status", "--porcelain"]).unwrap(), "");
        assert_eq!(git(&dir, &["diff", &one, "HEAD", "--stat"]).unwrap(), "", "the same files as One");
        fsck(&dir);
    }
    // and Undo: back on Two with its files
    v.undo().unwrap();
    assert_eq!(v.timeline(1).unwrap()[0].id, two);
    assert_eq!(read(&dir, "main.rr"), "PRINT \"two\"\n");
    assert!(!dir.join("form.rr").exists() && dir.join("extra.rr").exists());
    assert!(v.changes().unwrap().is_empty());
    // every risky action left a safety point
    let labels: Vec<String> = v.safety_points().unwrap().into_iter().map(|s| s.label).collect();
    assert_eq!(labels.len(), 4, "{labels:?}");
    if git(&dir, &["status", "--porcelain"]).is_some() {
        assert_eq!(git(&dir, &["status", "--porcelain"]).unwrap(), "");
        assert!(git(&dir, &["for-each-ref", "refs/rapidr/safety/"]).unwrap().lines().count() == 4);
        fsck(&dir);
    }
}

#[test]
fn checkpoints_for_rai() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = project(&tmp, "app");
    let mut v = Versions::start(&dir, &ana()).unwrap();
    write(&dir, "main.rr", "PRINT 1\n");
    let one = v.save_version("One", &ana()).unwrap();
    // unsaved work, and a new file: the checkpoint keeps both
    write(&dir, "main.rr", "PRINT 1\nPRINT \"mine\"\n");
    write(&dir, "draft.rr", "' draft\n");
    let cp = v.checkpoint("make the button bigger", &rai()).unwrap();
    assert_eq!((cp.n, cp.author.as_str(), cp.label.as_str()), (1, "RAI", "Checkpoint: make the button bigger"));
    assert_eq!(v.timeline(5).unwrap().len(), 1, "a checkpoint is no version");
    assert_eq!(v.changes().unwrap().len(), 2, "and changes nothing");
    // RAI's turn changes things
    write(&dir, "main.rr", "PRINT \"rai\"\n");
    std::fs::remove_file(dir.join("draft.rr")).unwrap();
    write(&dir, "button.rr", "' RAI's\n");
    // what changed since the checkpoint
    let diff = v.compare_versions(&cp.id, &cp.id).unwrap();
    assert!(diff.is_empty());
    // go back to before RAI's change
    v.restore_snapshot(&cp.id).unwrap();
    assert_eq!(read(&dir, "main.rr"), "PRINT 1\nPRINT \"mine\"\n");
    assert_eq!(read(&dir, "draft.rr"), "' draft\n");
    assert!(!dir.join("button.rr").exists());
    assert_eq!(v.timeline(1).unwrap()[0].id, one);
    assert_eq!(v.checkpoints().unwrap().len(), 1);
    // …and Undo brings RAI's change back
    v.undo().unwrap();
    assert_eq!(read(&dir, "main.rr"), "PRINT \"rai\"\n");
    assert!(dir.join("button.rr").exists() && !dir.join("draft.rr").exists());
    if git(&dir, &["status"]).is_some() {
        assert_eq!(git(&dir, &["for-each-ref", "--format=%(refname)", "refs/rapidr/checkpoints/"]).unwrap(), "refs/rapidr/checkpoints/1\n");
        assert_eq!(git(&dir, &["show", &format!("{}:draft.rr", cp.id)]).unwrap(), "' draft\n");
        fsck(&dir);
    }
}

#[test]
fn the_secret_scan_warns_with_the_line() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = project(&tmp, "app");
    let mut v = Versions::start(&dir, &ana()).unwrap();
    write(&dir, "main.rr", "PRINT 1\n");
    v.save_version("One", &ana()).unwrap();
    write(&dir, "main.rr", "PRINT 1\nDIM Key AS STRING\nKey = \"sk-ant-api03-ABCDEFGHIJKLMNOPQRSTUVWX\"\n");
    write(&dir, "deploy/id_rsa", "-----BEGIN RSA PRIVATE KEY-----\nMIIE\n-----END RSA PRIVATE KEY-----\n");
    let found = v.check_secrets(None).unwrap();
    let lines: Vec<(&str, usize)> = found.iter().map(|w| (w.path.as_str(), w.line)).collect();
    assert_eq!(lines, [("deploy/id_rsa", 1), ("main.rr", 3)]);
    assert!(!found[1].masked.contains("ABCDEFGHIJ"), "the warning doesn't repeat the secret");
    let e = v.save_version("With a key", &ana()).err().unwrap();
    assert_eq!(e.kind, ErrorKind::SecretsFound);
    assert!(e.message.starts_with("deploy/id_rsa, line 1, looks like a private key (1 more lines too)"), "{e}");
    assert_eq!(e.secrets.len(), 2);
    assert_eq!(v.timeline(5).unwrap().len(), 1, "nothing was saved");
    // only the chosen file is scanned
    assert_eq!(v.check_secrets(Some(&["main.rr".into()])).unwrap().len(), 1);
    // saving anyway, after the warning
    v.save_version_with("With a key", &ana(), &SaveOptions { files: None, allow_secrets: true }).unwrap();
    // a later change doesn't warn about lines already saved
    write(&dir, "main.rr", &(read(&dir, "main.rr") + "PRINT 2\n"));
    assert!(v.check_secrets(None).unwrap().is_empty());
    if git(&dir, &["status"]).is_some() {
        fsck(&dir);
    }
}

#[test]
fn experiments_rewrite_only_what_differs() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = project(&tmp, "app");
    let mut v = Versions::start(&dir, &ana()).unwrap();
    for i in 0..20 {
        write(&dir, &format!("lib/f{i}.rr"), &format!("' file {i}\n"));
    }
    write(&dir, "main.rr", "PRINT 1\n");
    v.save_version("Start", &ana()).unwrap();
    v.start_experiment("idea").unwrap();
    assert_eq!(v.experiments().unwrap(), ["idea", "main"]);
    write(&dir, "main.rr", "PRINT \"idea\"\n");
    write(&dir, "lib/new.rr", "' only in idea\n");
    std::fs::remove_file(dir.join("lib/f3.rr")).unwrap();
    v.save_version("Idea", &ana()).unwrap();
    let before: Vec<_> = (0..20).filter(|i| *i != 3).map(|i| stamp(&dir, &format!("lib/f{i}.rr"))).collect();
    v.switch_to("main").unwrap();
    assert_eq!(read(&dir, "main.rr"), "PRINT 1\n");
    assert!(!dir.join("lib/new.rr").exists() && dir.join("lib/f3.rr").exists());
    let after: Vec<_> = (0..20).filter(|i| *i != 3).map(|i| stamp(&dir, &format!("lib/f{i}.rr"))).collect();
    assert_eq!(before, after, "the 19 files both experiments share were not rewritten");
    assert!(v.changes().unwrap().is_empty());
    // a bad name, an unknown experiment: plain words
    assert_eq!(v.start_experiment("my idea").err().unwrap().kind, ErrorKind::InvalidName);
    assert_eq!(v.switch_to("nope").err().unwrap().kind, ErrorKind::NotFound);
    // undo the switch, then undo starting the experiment
    v.undo().unwrap();
    assert_eq!(v.current_experiment().unwrap(), "idea");
    if git(&dir, &["status", "--porcelain"]).is_some() {
        assert_eq!(git(&dir, &["status", "--porcelain"]).unwrap(), "");
        fsck(&dir);
    }
}

#[test]
fn get_latest_from_a_shared_folder() {
    let tmp = tempfile::tempdir().unwrap();
    let shared = tmp.path().join("usb/class.git");
    create_shared_folder(&shared).unwrap();
    // Ana starts and shares
    let a_dir = project(&tmp, "ana");
    let mut a = Versions::start(&a_dir, &ana()).unwrap();
    write(&a_dir, "main.rr", "PRINT 1\nPRINT 2\nPRINT 3\n");
    a.save_version("One", &ana()).unwrap();
    assert!(matches!(a.share_to_folder(&shared, "main").unwrap(), Share::Shared { .. }));
    // Ben gets it into an empty project
    let b_dir = project(&tmp, "ben");
    let ben = Person::new("Ben", "ben@example.com");
    let mut b = Versions::start(&b_dir, &ben).unwrap();
    assert!(matches!(b.get_latest_from_folder(&shared, "main", &ben).unwrap(), GetLatest::FastForward { .. }));
    assert_eq!(read(&b_dir, "main.rr"), "PRINT 1\nPRINT 2\nPRINT 3\n");
    // both change different lines
    write(&a_dir, "main.rr", "PRINT \"one\"\nPRINT 2\nPRINT 3\n");
    a.save_version("Ana's", &ana()).unwrap();
    a.share_to_folder(&shared, "main").unwrap();
    write(&b_dir, "main.rr", "PRINT 1\nPRINT 2\nPRINT \"three\"\n");
    b.save_version("Ben's", &ben).unwrap();
    assert_eq!(b.share_to_folder(&shared, "main").unwrap(), Share::NeedsGetLatest);
    match b.get_latest_from_folder(&shared, "main", &ben).unwrap() {
        GetLatest::Merged { objects, .. } => assert_eq!(objects, 3),
        other => panic!("{other:?}"),
    }
    assert_eq!(read(&b_dir, "main.rr"), "PRINT \"one\"\nPRINT 2\nPRINT \"three\"\n");
    assert!(matches!(b.share_to_folder(&shared, "main").unwrap(), Share::Shared { .. }));
    assert!(matches!(a.get_latest_from_folder(&shared, "main", &ana()).unwrap(), GetLatest::FastForward { .. }));
    assert_eq!(read(&a_dir, "main.rr"), read(&b_dir, "main.rr"));
    assert_eq!(a.get_latest_from_folder(&shared, "main", &ana()).unwrap(), GetLatest::UpToDate);
    // undo the get latest: back where Ana was
    a.undo().unwrap();
    assert_eq!(read(&a_dir, "main.rr"), "PRINT \"one\"\nPRINT 2\nPRINT 3\n");
    // a folder that isn't shared: plain words
    assert_eq!(a.get_latest_from_folder(&tmp.path().join("nowhere"), "main", &ana()).err().unwrap().kind, ErrorKind::NotFound);
    if git(&shared, &["fsck"]).is_some() {
        fsck(&shared);
        fsck(&a_dir);
        fsck(&b_dir);
        assert_eq!(git(&b_dir, &["log", "--format=%s", "-1"]).unwrap(), "Get the latest main into main\n");
    }
}

/// The plan's bar (versions-plan §8): the timeline and the changes of a
/// project with a thousand versions in under 100 ms (release builds).
#[test]
fn a_thousand_versions() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = project(&tmp, "big");
    let mut v = Versions::start(&dir, &ana()).unwrap();
    for i in 0..30 {
        write(&dir, &format!("src/f{i}.rr"), &format!("' file {i}\n"));
    }
    let n = if cfg!(debug_assertions) { 120 } else { 1000 };
    for i in 0..n {
        write(&dir, "main.rr", &format!("PRINT {i}\n"));
        v.save_version(&format!("Version {i}"), &ana()).unwrap();
    }
    write(&dir, "main.rr", "PRINT \"unsaved\"\n");
    let v = Versions::open(&dir, &ana()).unwrap();
    let t = std::time::Instant::now();
    let tl = v.timeline(usize::MAX).unwrap();
    let timeline_ms = t.elapsed().as_secs_f64() * 1000.0;
    let t = std::time::Instant::now();
    let changes = v.changes().unwrap();
    let status_ms = t.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(tl.len(), n);
    assert_eq!(changes.len(), 1);
    eprintln!("{n} versions: timeline {timeline_ms:.1} ms, status {status_ms:.1} ms");
    if !cfg!(debug_assertions) {
        assert!(timeline_ms < 100.0 && status_ms < 100.0, "timeline {timeline_ms:.1} ms, status {status_ms:.1} ms");
    }
    if git(&dir, &["status"]).is_some() {
        fsck(&dir);
    }
}

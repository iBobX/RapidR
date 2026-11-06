//! Unit tests that need the crate's insides: a folder that belongs to
//! another user account (the ownership check is swapped, since a test
//! can't give a folder to another account without being root).

use super::*;

fn ana() -> Person {
    Person::new("Ana", "ana@example.com")
}

fn not_mine(_: &Path) -> bool {
    false
}

#[test]
fn another_users_project_opens_read_only() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("usb-project");
    std::fs::create_dir_all(&dir).unwrap();
    let mut v = Versions::start(&dir, &ana()).unwrap();
    std::fs::write(dir.join("main.rr"), "PRINT 1\n").unwrap();
    let first = v.save_version("First", &ana()).unwrap();
    drop(v);
    let history = std::fs::read_to_string(dir.join(".git/rapidr/history.jsonl")).unwrap();

    // opened for work: refused, with the reason in words
    let err = Versions::open_checked(&dir, &ana(), false, not_mine).err().unwrap();
    assert_eq!(err.kind, ErrorKind::NotYours);
    assert!(err.message.contains("another user account") && err.message.contains("read-only"), "{err}");

    // read-only: everything to look at works, nothing that writes does
    let mut v = Versions::open_checked(&dir, &ana(), true, not_mine).unwrap();
    assert!(v.is_read_only());
    assert_eq!(v.timeline(5).unwrap()[0].id, first);
    std::fs::write(dir.join("main.rr"), "PRINT 2\n").unwrap();
    assert_eq!(v.changes().unwrap().len(), 1);
    assert_eq!(v.what_changed().unwrap()[0].hunks.len(), 1);
    assert_eq!(v.file_at("main.rr", "HEAD").unwrap().unwrap(), b"PRINT 1\n");
    for e in [
        v.save_version("Two", &ana()).err(),
        v.checkpoint("x", &ana()).err(),
        v.restore_file("main.rr", "HEAD").err(),
        v.start_experiment("idea").err(),
        v.undo().err(),
    ] {
        assert_eq!(e.unwrap().kind, ErrorKind::ReadOnly);
    }
    assert_eq!(std::fs::read_to_string(dir.join("main.rr")).unwrap(), "PRINT 2\n", "nothing was rewritten");
    assert!(v.safety_points().unwrap().is_empty() && v.checkpoints().unwrap().is_empty());
    assert_eq!(std::fs::read_to_string(dir.join(".git/rapidr/history.jsonl")).unwrap(), history);
}

/// SEC-31's belt and braces: a project's own config can't turn off the
/// checkout's protection of `.git` look-alikes (`GIT~1`, Windows' short
/// name of `.git`): `core.protectNTFS = false` is stripped when it opens.
#[test]
fn a_planted_protect_ntfs_off_is_stripped() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("p");
    std::fs::create_dir_all(&dir).unwrap();
    let mut v = Versions::start(&dir, &ana()).unwrap();
    std::fs::write(dir.join("main.rr"), "PRINT 1\n").unwrap();
    v.save_version("First", &ana()).unwrap();
    // a version with GIT~1/config, its trees written as objects (as
    // another program's would come, through Get latest: gitoxide's tree
    // editor refuses the name)
    use gix::objs::tree::{Entry, EntryKind};
    let head = v.head_id().unwrap().unwrap();
    let main = checkout::entry_at(&v.repo, v.head_tree_id().unwrap().unwrap(), "main.rr").unwrap().unwrap().0;
    let blob = v.repo.write_blob(b"[core]\n\thooksPath = /tmp\n").unwrap().detach();
    let inner = gix::objs::Tree { entries: vec![Entry { mode: EntryKind::Blob.into(), filename: "config".into(), oid: blob }] };
    let inner = v.repo.write_object(&inner).unwrap().detach();
    let top = gix::objs::Tree {
        entries: vec![Entry { mode: EntryKind::Tree.into(), filename: "GIT~1".into(), oid: inner }, Entry { mode: EntryKind::Blob.into(), filename: "main.rr".into(), oid: main }],
    };
    let tree = v.repo.write_object(&top).unwrap().detach();
    let evil = v.commit_on_head("Looks harmless", tree, Some(head), &ana()).unwrap();
    v.save_version_files_back(head);
    drop(v);
    let cfg = dir.join(".git/config");
    let text = std::fs::read_to_string(&cfg).unwrap();
    std::fs::write(&cfg, format!("{text}\n[core]\n\tprotectNTFS = false\n\tprotectHFS = false\n[gitoxide \"core\"]\n\tprotectWindows = false\n")).unwrap();
    let mut v = Versions::open(&dir, &ana()).unwrap();
    assert!(v.go_back(&evil.to_string(), &ana()).is_err(), "GIT~1 is refused as gitoxide's default says");
    assert!(!dir.join("GIT~1").exists());
}

impl Versions {
    /// (tests: the branch back on `id`, nothing else changed)
    fn save_version_files_back(&self, id: ObjectId) {
        let branch = self.head_ref().unwrap();
        edit_ref(&self.repo, &branch, Target::Object(id), PreviousValue::Any, "test".into(), &ana()).unwrap();
    }
}

#[test]
fn errors_are_plain_words_with_the_detail_apart() {
    let tmp = tempfile::tempdir().unwrap();
    let err = Versions::open(tmp.path(), &ana()).err().unwrap();
    assert_eq!(err.kind, ErrorKind::NotAProject);
    assert_eq!(err.to_string(), "This folder has no versions yet: start versioning it first.");
    let mut v = Versions::start(tmp.path(), &ana()).unwrap();
    assert_eq!(v.undo().err().unwrap().kind, ErrorKind::NothingToUndo);
    assert_eq!(v.save_version("Nothing", &ana()).err().unwrap().kind, ErrorKind::NothingToSave);
    let e = v.go_back("no-such-version", &ana()).err().unwrap();
    assert_eq!((e.kind, e.message.as_str()), (ErrorKind::NotFound, "There is no version \u{201C}no-such-version\u{201D}."));
    assert!(!e.detail().is_empty(), "the technical cause is kept");
}

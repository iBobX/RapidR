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

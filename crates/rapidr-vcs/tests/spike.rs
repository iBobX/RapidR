//! Lane G0's spike, end to end on temporary repositories
//! (docs/versions-g0.md). Every repository written here is checked with
//! the system's `git` when it is installed (`git fsck`, `git status`,
//! `git log`, `git diff`, `git blame`, `git clone`): the product never
//! runs `git`, the tests use it as an independent judge. Without `git`
//! those checks are skipped and the rest still runs.

use std::path::{Path, PathBuf};
use std::process::Command;

use rapidr_vcs::{create_shared_folder, BringInto, FileState, Line, Person, Share, Versions};

fn ana() -> Person {
    Person::new("Ana", "ana@example.com")
}

fn ben() -> Person {
    Person::new("Ben", "ben@example.com")
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

fn lines(n: usize, prefix: &str) -> String {
    (1..=n).map(|i| format!("{prefix} {i}\n")).collect()
}

/// The system `git`, isolated from this machine's configuration; `None`
/// when it isn't installed (the checks are then skipped).
fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let home = dir.parent().unwrap_or(dir);
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("HOME", home)
        .env("LC_ALL", "C")
        .output()
        .ok()?;
    assert!(
        out.status.success(),
        "git {args:?} failed in {}:\n{}{}",
        dir.display(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn have_git() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn project(tmp: &tempfile::TempDir, name: &str) -> PathBuf {
    let dir = tmp.path().join(name);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn save_change_diff_blame_and_timeline() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = project(&tmp, "hello");
    let mut v = Versions::start(&dir, &ana()).unwrap();
    assert_eq!(v.current_experiment().unwrap(), "main");

    write(&dir, "main.rr", &lines(8, "PRINT"));
    write(&dir, "README.md", "Hello\n");
    let changes = v.changes().unwrap();
    assert_eq!(changes.len(), 2);
    assert!(changes.iter().all(|c| c.state == FileState::New));
    let first = v.save_version("First version", &ana()).unwrap();
    assert!(v.changes().unwrap().is_empty());

    if have_git() {
        git(&dir, &["fsck", "--full", "--strict"]);
        assert_eq!(
            git(&dir, &["status", "--porcelain"]).unwrap(),
            "",
            "git sees what we saved"
        );
        assert_eq!(
            git(&dir, &["log", "--format=%an %ae|%s"]).unwrap(),
            "Ana ana@example.com|First version\n"
        );
    }

    // change line 3, add a last line, a new file, delete one
    let mut text = lines(8, "PRINT").replace("PRINT 3\n", "PRINT \"three\"\n");
    text.push_str("PRINT 9\n");
    write(&dir, "main.rr", &text);
    write(&dir, "forms/form.rr", "CREATE Form AS RForm\nEND CREATE\n");
    std::fs::remove_file(dir.join("README.md")).unwrap();
    let changes = v.changes().unwrap();
    let summary: Vec<(String, FileState)> =
        changes.iter().map(|c| (c.path.clone(), c.state)).collect();
    assert_eq!(
        summary,
        vec![
            ("README.md".into(), FileState::Deleted),
            ("forms/form.rr".into(), FileState::New),
            ("main.rr".into(), FileState::Changed),
        ]
    );

    let diffs = v.what_changed().unwrap();
    let main = diffs.iter().find(|d| d.path == "main.rr").unwrap();
    assert!(!main.binary);
    let headers: Vec<String> = main.hunks.iter().map(|h| h.header()).collect();
    assert_eq!(headers, vec!["@@ -1,8 +1,9 @@"]);
    assert!(main.hunks[0]
        .lines
        .contains(&Line::Removed("PRINT 3".into())));
    assert!(main.hunks[0]
        .lines
        .contains(&Line::Added("PRINT \"three\"".into())));
    assert_eq!(
        main.hunks[0].lines.last(),
        Some(&Line::Added("PRINT 9".into()))
    );
    if have_git() {
        // the same hunk headers as git's own diff
        let out = git(&dir, &["diff", "-U3", "--", "main.rr"]).unwrap();
        let git_headers: Vec<&str> = out
            .lines()
            .filter(|l| l.starts_with("@@"))
            .map(|l| l.trim_end_matches(" @@").trim_end())
            .collect();
        let ours: Vec<String> = headers
            .iter()
            .map(|h| h.trim_end_matches(" @@").to_string())
            .collect();
        assert_eq!(git_headers, ours);
    }

    let second = v.save_version("Say three", &ben()).unwrap();
    assert_eq!(v.changes().unwrap(), vec![]);

    let timeline = v.timeline(10).unwrap();
    let t: Vec<(&str, &str)> = timeline
        .iter()
        .map(|x| (x.message.as_str(), x.author.as_str()))
        .collect();
    assert_eq!(t, vec![("Say three", "Ben"), ("First version", "Ana")]);
    assert_eq!(timeline[0].id, second);
    assert_eq!(timeline[1].id, first);

    // who wrote each line
    let blame = v.who_wrote("main.rr").unwrap();
    assert_eq!(blame.len(), 9);
    let who: Vec<&str> = blame.iter().map(|b| b.author.as_str()).collect();
    assert_eq!(
        who,
        vec!["Ana", "Ana", "Ben", "Ana", "Ana", "Ana", "Ana", "Ana", "Ben"]
    );
    assert_eq!(blame[2].text, "PRINT \"three\"");
    assert_eq!(blame[2].version, second);
    if have_git() {
        let out = git(&dir, &["blame", "--line-porcelain", "main.rr"]).unwrap();
        let git_who: Vec<&str> = out
            .lines()
            .filter_map(|l| l.strip_prefix("author "))
            .collect();
        assert_eq!(git_who, who, "git blame agrees");
    }

    // tree to tree
    let diffs = v.compare_versions(&first, &second).unwrap();
    let kinds: Vec<(&str, FileState, usize)> = diffs
        .iter()
        .map(|d| (d.path.as_str(), d.state, d.hunks.len()))
        .collect();
    assert_eq!(
        kinds,
        vec![
            ("README.md", FileState::Deleted, 1),
            ("forms/form.rr", FileState::New, 1),
            ("main.rr", FileState::Changed, 1)
        ]
    );

    if have_git() {
        git(&dir, &["fsck", "--full", "--strict"]);
        assert_eq!(git(&dir, &["status", "--porcelain"]).unwrap(), "");
    }
}

#[test]
fn experiments_fast_forward_merge_and_conflict() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = project(&tmp, "game");
    let mut v = Versions::start(&dir, &ana()).unwrap();
    write(&dir, "a.txt", &lines(10, "line"));
    write(&dir, "b.txt", "b\n");
    v.save_version("Start", &ana()).unwrap();

    // an experiment, then back to main: the file is as it was
    v.start_experiment("idea").unwrap();
    assert_eq!(v.current_experiment().unwrap(), "idea");
    write(
        &dir,
        "a.txt",
        &lines(10, "line").replace("line 2\n", "idea 2\n"),
    );
    write(&dir, "new-in-idea.txt", "only in idea\n");
    let idea = v.save_version("Idea", &ana()).unwrap();
    v.switch_to("main").unwrap();
    assert_eq!(v.current_experiment().unwrap(), "main");
    assert_eq!(read(&dir, "a.txt"), lines(10, "line"));
    assert!(
        !dir.join("new-in-idea.txt").exists(),
        "a file only in the experiment goes away"
    );
    assert!(v.changes().unwrap().is_empty());

    // nothing new on main: a fast-forward
    assert_eq!(
        v.bring_into_current("idea", &ana()).unwrap(),
        BringInto::FastForward(idea.clone())
    );
    assert!(read(&dir, "a.txt").contains("idea 2"));
    assert!(dir.join("new-in-idea.txt").exists());
    assert_eq!(v.timeline(1).unwrap()[0].id, idea);
    assert_eq!(
        v.bring_into_current("idea", &ana()).unwrap(),
        BringInto::AlreadyUpToDate
    );

    // both sides changed different places: a real three-way merge
    v.start_experiment("other").unwrap();
    write(&dir, "b.txt", "b changed in other\n");
    write(
        &dir,
        "a.txt",
        &read(&dir, "a.txt").replace("line 9\n", "other 9\n"),
    );
    v.save_version("Other", &ben()).unwrap();
    v.switch_to("main").unwrap();
    write(
        &dir,
        "a.txt",
        &read(&dir, "a.txt").replace("line 4\n", "main 4\n"),
    );
    v.save_version("Main", &ana()).unwrap();
    let merged = match v.bring_into_current("other", &ana()).unwrap() {
        BringInto::Merged(id) => id,
        other => panic!("expected a merge, got {other:?}"),
    };
    let top = &v.timeline(1).unwrap()[0];
    assert_eq!(top.id, merged);
    assert_eq!(top.parents, 2);
    let a = read(&dir, "a.txt");
    assert!(
        a.contains("main 4") && a.contains("other 9") && a.contains("idea 2"),
        "{a}"
    );
    assert_eq!(read(&dir, "b.txt"), "b changed in other\n");
    assert!(v.changes().unwrap().is_empty());

    // unsaved work is never thrown away by a switch
    write(&dir, "b.txt", "unsaved\n");
    assert!(v.switch_to("other").is_err());
    assert_eq!(read(&dir, "b.txt"), "unsaved\n");
    v.save_version("Unsaved saved", &ana()).unwrap();

    // the same line on both sides: conflicts reported, nothing touched
    v.start_experiment("x").unwrap();
    write(
        &dir,
        "a.txt",
        &read(&dir, "a.txt").replace("line 5\n", "X 5\n"),
    );
    v.save_version("X", &ben()).unwrap();
    v.switch_to("main").unwrap();
    write(
        &dir,
        "a.txt",
        &read(&dir, "a.txt").replace("line 5\n", "M 5\n"),
    );
    let before = v.save_version("M", &ana()).unwrap();
    let a_before = read(&dir, "a.txt");
    assert_eq!(
        v.bring_into_current("x", &ana()).unwrap(),
        BringInto::Conflicts(vec!["a.txt".into()])
    );
    assert_eq!(v.timeline(1).unwrap()[0].id, before, "no version was made");
    assert_eq!(read(&dir, "a.txt"), a_before, "the file was not touched");
    assert!(v.changes().unwrap().is_empty());

    if have_git() {
        git(&dir, &["fsck", "--full", "--strict"]);
        assert_eq!(git(&dir, &["status", "--porcelain"]).unwrap(), "");
        assert_eq!(
            git(&dir, &["log", "--merges", "--format=%s"]).unwrap(),
            "Bring other into main\n"
        );
        assert_eq!(git(&dir, &["branch", "--show-current"]).unwrap(), "main\n");
    }
}

#[test]
fn share_with_a_local_bare_repository() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = project(&tmp, "app");
    let shared = tmp.path().join("shared.git");
    create_shared_folder(&shared).unwrap();

    let mut v = Versions::start(&dir, &ana()).unwrap();
    write(&dir, "main.rr", "PRINT 1\n");
    write(&dir, "lib/util.rr", "SUB Util\nEND SUB\n");
    v.save_version("One", &ana()).unwrap();
    write(&dir, "main.rr", "PRINT 2\n");
    let two = v.save_version("Two", &ana()).unwrap();

    match v.share_to_folder(&shared, "main").unwrap() {
        Share::Shared {
            from: None,
            to,
            objects,
        } => {
            assert_eq!(to, two);
            // 2 commits, 3 trees (root ×2, lib), 3 blobs
            assert_eq!(objects, 8);
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(v.share_to_folder(&shared, "main").unwrap(), Share::UpToDate);

    // one more version: only what is new travels
    write(&dir, "main.rr", "PRINT 3\n");
    let three = v.save_version("Three", &ben()).unwrap();
    assert_eq!(
        v.share_to_folder(&shared, "main").unwrap(),
        Share::Shared {
            from: Some(two.clone()),
            to: three.clone(),
            objects: 3
        }
    );

    // another project with its own history may not overwrite it
    let other_dir = project(&tmp, "other");
    let mut other = Versions::start(&other_dir, &ben()).unwrap();
    write(&other_dir, "x.rr", "PRINT \"x\"\n");
    other.save_version("Unrelated", &ben()).unwrap();
    assert_eq!(
        other.share_to_folder(&shared, "main").unwrap(),
        Share::NeedsGetLatest
    );

    if have_git() {
        git(&shared, &["fsck", "--full", "--strict"]);
        assert_eq!(
            git(&shared, &["log", "--format=%s", "main"]).unwrap(),
            "Three\nTwo\nOne\n"
        );
        // the system git clones it and finds the same files
        git(
            tmp.path(),
            &["clone", "-q", shared.to_str().unwrap(), "cloned"],
        );
        let cloned = tmp.path().join("cloned");
        assert_eq!(read(&cloned, "main.rr"), "PRINT 3\n");
        assert_eq!(read(&cloned, "lib/util.rr"), "SUB Util\nEND SUB\n");
        assert_eq!(git(&cloned, &["rev-parse", "HEAD"]).unwrap().trim(), three);
    }
}

/// A repository whose own configuration asks for programs everywhere, and
/// whose hooks would leave marks: Versions must run none of them.
#[cfg(unix)]
#[test]
fn never_runs_configured_programs_or_hooks() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = tempfile::tempdir().unwrap();
    let dir = project(&tmp, "evil");
    let marks = tmp.path().join("marks");
    std::fs::create_dir_all(&marks).unwrap();
    let mut v = Versions::start(&dir, &ana()).unwrap();
    write(&dir, "a.txt", &lines(10, "line"));
    v.save_version("Start", &ana()).unwrap();
    drop(v);

    let touch = |what: &str| format!("touch '{}'", marks.join(what).display());
    let config_path = dir.join(".git/config");
    let mut config = std::fs::read_to_string(&config_path).unwrap();
    config.push_str(&format!(
        "[filter \"evil\"]\n\tclean = {}\n\tsmudge = {}\n\tprocess = {}\n\trequired = true\n\
         [diff \"evil\"]\n\ttextconv = {}\n\tcommand = {}\n\
         [merge \"evil\"]\n\tdriver = {}\n\
         [core]\n\tfsmonitor = {}\n\tsshCommand = {}\n\thooksPath = .git/hooks\n\
         [credential]\n\thelper = !{}\n\
         [remote \"origin\"]\n\turl = /nowhere\n\tuploadpack = {}\n",
        touch("filter-clean"),
        touch("filter-smudge"),
        touch("filter-process"),
        touch("diff-textconv"),
        touch("diff-command"),
        touch("merge-driver"),
        touch("fsmonitor"),
        touch("ssh"),
        touch("credential"),
        touch("uploadpack"),
    ));
    std::fs::write(&config_path, &config).unwrap();
    write(
        &dir,
        ".gitattributes",
        "* filter=evil diff=evil merge=evil\n",
    );
    for hook in [
        "pre-commit",
        "commit-msg",
        "post-commit",
        "post-checkout",
        "post-merge",
        "pre-push",
        "reference-transaction",
    ] {
        let path = dir.join(".git/hooks").join(hook);
        std::fs::write(
            &path,
            format!("#!/bin/sh\n{}\n", touch(&format!("hook-{hook}"))),
        )
        .unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    let mut v = Versions::open(&dir, &ana()).unwrap();
    write(
        &dir,
        "a.txt",
        &lines(10, "line").replace("line 2\n", "changed 2\n"),
    );
    assert!(!v.changes().unwrap().is_empty());
    assert!(!v.what_changed().unwrap().is_empty());
    v.save_version("With attributes", &ana()).unwrap();
    v.start_experiment("side").unwrap();
    write(
        &dir,
        "a.txt",
        &read(&dir, "a.txt").replace("line 9\n", "side 9\n"),
    );
    v.save_version("Side", &ben()).unwrap();
    v.switch_to("main").unwrap();
    write(
        &dir,
        "a.txt",
        &read(&dir, "a.txt").replace("line 4\n", "main 4\n"),
    );
    v.save_version("Main", &ana()).unwrap();
    assert!(matches!(
        v.bring_into_current("side", &ana()).unwrap(),
        BringInto::Merged(_)
    ));
    v.who_wrote("a.txt").unwrap();
    v.compare_versions("HEAD~1", "HEAD").unwrap();
    let shared = tmp.path().join("shared.git");
    create_shared_folder(&shared).unwrap();
    assert!(matches!(
        v.share_to_folder(&shared, "main").unwrap(),
        Share::Shared { .. }
    ));
    // G1's actions too: checkpoints, restoring, going back, Undo, the
    // secret scan, getting the latest (the filters would run while
    // snapshots read the files)
    write(&dir, "a.txt", &read(&dir, "a.txt").replace("line 7\n", "rai 7\n"));
    let cp = v.checkpoint("rai", &ana()).unwrap();
    write(&dir, "a.txt", &read(&dir, "a.txt").replace("rai 7\n", "after 7\n"));
    v.check_secrets(None).unwrap();
    v.restore_snapshot(&cp.id).unwrap();
    v.undo().unwrap();
    v.restore_file("a.txt", "HEAD").unwrap();
    v.go_back("HEAD~1", &ana()).unwrap();
    v.undo().unwrap();
    v.get_latest_from_folder(&shared, "main", &ana()).unwrap();

    let left: Vec<String> = std::fs::read_dir(&marks)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(left.is_empty(), "programs ran: {left:?}");
    // the user's configuration file is never rewritten
    assert_eq!(std::fs::read_to_string(&config_path).unwrap(), config);
}

/// The control for the test above: gitoxide on its own *does* run a
/// repository's filter driver, so Versions' neutralizing is what keeps
/// clones safe. (If gitoxide ever stops, this test says so.)
#[cfg(unix)]
#[test]
fn plain_gitoxide_runs_a_configured_filter_driver() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = project(&tmp, "control");
    let mark = tmp.path().join("ran");
    Versions::start(&dir, &ana()).unwrap();
    let config_path = dir.join(".git/config");
    let mut config = std::fs::read_to_string(&config_path).unwrap();
    config.push_str(&format!(
        "[filter \"evil\"]\n\tclean = touch '{}'\n",
        mark.display()
    ));
    std::fs::write(&config_path, config).unwrap();
    write(&dir, ".gitattributes", "* filter=evil\n");
    write(&dir, "a.txt", "a\n");

    let repo = gix::open(&dir).unwrap();
    let index = repo.index_or_empty().unwrap();
    let (mut pipeline, _) = repo.filter_pipeline(None).unwrap();
    let file = std::fs::File::open(dir.join("a.txt")).unwrap();
    let mut out = Vec::new();
    use std::io::Read;
    let _ = pipeline
        .convert_to_git(file, Path::new("a.txt"), &index)
        .map(|mut r| r.read_to_end(&mut out));
    assert!(mark.exists(), "plain gitoxide ran the filter driver");
}

#[test]
fn hunks_match_unified_diff_numbering() {
    let old = b"a\nb\nc\nd\ne\nf\ng\nh\ni\nj\nk\nl\nm\nn\n";
    let new = b"a\nB\nc\nd\ne\nf\ng\nh\ni\nj\nk\nl\nM\nn\nadded\n";
    let h = rapidr_vcs::hunks(old, new, 3);
    let headers: Vec<String> = h.iter().map(|h| h.header()).collect();
    assert_eq!(headers, vec!["@@ -1,5 +1,5 @@", "@@ -10,5 +10,6 @@"]);
    // a pure insertion into an empty file
    let h = rapidr_vcs::hunks(b"", b"x\ny\n", 3);
    assert_eq!(h[0].header(), "@@ -0,0 +1,2 @@");
}

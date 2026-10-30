//! **RREPOSITORY**: a project's versions, in plain words (Versions G3,
//! docs/versions-plan.md) — what RapidR Studio's Versions pane, its Save a
//! Version dialog, the editor's change marks and the Project pane's badges
//! use, and a component any RapidR program run by `rapidr run` can use too.
//!
//! * **Versions** (the desktop): `rapidr-vcs` on gitoxide — Start, Save a
//!   version (the secret scan first), the Timeline (RAI's checkpoints in it,
//!   marked), What changed?, Go back (the project or one file: a new
//!   version, never a destructive reset, a safety point first), Undo of
//!   any of them. Every repository is an ordinary Git repository.
//! * **Local history** (everywhere, the web too): a copy of each file
//!   every time it is saved, before the project has versions and after
//!   ([`crate::local_history`]).
//! * **The web**: gitoxide doesn't run in a browser (docs/versions-g0.md
//!   §5), so `Available` is False there and the versions' methods say so in
//!   `Error`; the local history works.
//!
//! Lists come back as text, a line each, fields separated by tabs, as
//! RapidR's other Studio components give them.

use std::cell::RefCell;
use std::collections::HashMap;

use rapidr_value::Value;

use crate::local_history;
use crate::{flag, int_arg, join, slashes, text_arg};

/// What a project's versions are, as last read.
#[derive(Default, Clone)]
struct Repo {
    folder: String,
    author: String,
    email: String,
    error: String,
    kind: String,
    secrets: Vec<String>,
    versioned: bool,
    read_only: bool,
    experiment: String,
    /// (state letter, path): M changed, N new, D removed.
    changes: Vec<(char, String)>,
}

thread_local! {
    static REPOS: RefCell<HashMap<String, Repo>> = RefCell::new(HashMap::new());
}

fn with<R>(name: &str, f: impl FnOnce(&mut Repo) -> R) -> R {
    REPOS.with(|r| f(r.borrow_mut().entry(name.to_ascii_lowercase()).or_default()))
}

/// What the web says for anything that needs versions.
pub const NOT_ON_THE_WEB: &str = "Versions need RapidR Studio on your computer: in a browser, Studio keeps a local history of your saves only.";

fn text(s: impl Into<String>) -> Value {
    Value::String(s.into().into())
}

/// A property (`None`: not one of its own).
pub fn get(name: &str, prop: &str) -> Option<Value> {
    let r = with(name, |r| r.clone());
    Some(match prop {
        "folder" => text(r.folder),
        "available" => flag(cfg!(not(target_arch = "wasm32"))),
        "versioned" => flag(r.versioned),
        "readonly" => flag(r.read_only),
        "experiment" => text(r.experiment),
        "changecount" => Value::Integer(r.changes.len() as i64),
        "changes" => text(r.changes.iter().map(|(s, p)| format!("{s}\t{p}")).collect::<Vec<_>>().join("\n")),
        "author" => text(r.author),
        "email" => text(r.email),
        "error" => text(r.error),
        "errorkind" => text(r.kind),
        "secrets" => text(r.secrets.join("\n")),
        "canundo" => text(crate::notes::ui_history(&engine::can_undo(&r).unwrap_or_default())),
        "historyfolder" => text(if r.folder.is_empty() { String::new() } else { local_history::history_folder(&r.folder) }),
        // (when the timeline's and the history's times are: "2 minutes ago")
        "now" => Value::Integer(local_history::now()),
        _ => return None,
    })
}

/// Sets a property; whether it was one of its own.
pub fn set(name: &str, prop: &str, v: &Value) -> bool {
    match prop {
        "folder" => {
            let f = slashes(&v.to_string_val()).trim_end_matches('/').to_string();
            with(name, |r| {
                r.folder = f;
                refresh(r);
            });
        }
        "author" => with(name, |r| r.author = v.to_string_val().trim().to_string()),
        "email" => with(name, |r| r.email = v.to_string_val().trim().to_string()),
        _ => return false,
    }
    true
}

/// Reads the project's state again: versioned or not, the experiment, the
/// changes not saved as a version.
fn refresh(r: &mut Repo) {
    r.versioned = false;
    r.read_only = false;
    r.experiment.clear();
    r.changes.clear();
    if r.folder.is_empty() {
        return;
    }
    if let Err(e) = engine::read_state(r) {
        r.error = e;
    }
}

fn ok(r: &mut Repo) {
    r.error.clear();
    r.kind.clear();
}

fn failed(r: &mut Repo, kind: &str, message: String) {
    r.kind = kind.to_string();
    r.error = message;
}

/// A method (`None`: not one of its own).
pub fn call(name: &str, method: &str, args: &[Value]) -> Option<Value> {
    if !is_method(method) {
        return None;
    }
    let s = |i: usize| text_arg(args, i);
    Some(with(name, |r| match method {
        "refresh" => {
            refresh(r);
            flag(r.versioned)
        }
        "start" => {
            let done = engine::start(r);
            refresh(r);
            if done {
                ok(r);
            }
            flag(done && r.versioned)
        }
        "saveversion" => {
            let allow = args.get(2).is_some_and(|v| v.to_i64() != 0);
            let id = engine::save(r, &s(0), &s(1), allow);
            refresh(r);
            text(id.unwrap_or_default())
        }
        "checksecrets" => Value::Integer(engine::check_secrets(r, &s(0)) as i64),
        "timeline" => text(engine::timeline(r, int_arg(args, 0, 200).max(1) as usize)),
        "versionfiles" => text(engine::version_files(r, &s(0))),
        "fileat" => text(engine::file_at(r, &s(0), &s(1)).unwrap_or_default()),
        "hasfile" => flag(engine::file_at(r, &s(0), &s(1)).is_some()),
        "goback" => {
            let id = engine::go_back(r, &s(0));
            refresh(r);
            text(id.unwrap_or_default())
        }
        "gobackfile" => {
            let id = engine::go_back_file(r, &s(0), &s(1));
            refresh(r);
            text(id.unwrap_or_default())
        }
        "undo" => {
            let words = engine::undo(r);
            refresh(r);
            text(words.unwrap_or_default())
        }
        "history" => text(engine::history(r)),
        "checkpoint" => {
            let id = engine::checkpoint(r, &s(0));
            text(id.unwrap_or_default())
        }
        // ---- the local history: everywhere, with or without versions ----
        "snapshot" => {
            let why = if s(1).trim().is_empty() { local_history::WHY_SAVED.to_string() } else { s(1).trim().to_string() };
            match local_history::snapshot_file(&r.folder, &abs(r, &s(0)), &why) {
                Ok(snap) => {
                    ok(r);
                    text(snap.map(|x| x.n.to_string()).unwrap_or_default())
                }
                Err(e) => {
                    failed(r, "failed", e);
                    text("")
                }
            }
        }
        // KeepCopy(Path, Text, Why): Text kept as a copy of the file — what
        // an editor had (the file changed outside Studio: "before-reload")
        "keepcopy" => {
            let why = if s(2).trim().is_empty() { local_history::WHY_BEFORE_RELOAD.to_string() } else { s(2).trim().to_string() };
            match local_history::snapshot(&r.folder, &abs(r, &s(0)), s(1).as_bytes(), &why) {
                Ok(snap) => {
                    ok(r);
                    text(snap.map(|x| x.n.to_string()).unwrap_or_default())
                }
                Err(e) => {
                    failed(r, "failed", e);
                    text("")
                }
            }
        }
        "localhistory" => text(
            local_history::list(&r.folder, &abs(r, &s(0)))
                .iter()
                .map(|x| format!("{}\t{}\t{}\t{}", x.n, x.seconds, x.why, x.size))
                .collect::<Vec<_>>()
                .join("\n"),
        ),
        "localfiles" => text(local_history::files(&r.folder).join("\n")),
        "snapshottext" => match local_history::read(&r.folder, s(0).trim().parse().unwrap_or(0)) {
            Ok(b) => {
                ok(r);
                text(String::from_utf8_lossy(&b).into_owned())
            }
            Err(e) => {
                failed(r, "failed", e);
                text("")
            }
        },
        _ => Value::Null,
    }))
}

fn is_method(m: &str) -> bool {
    matches!(
        m,
        "refresh" | "start" | "saveversion" | "checksecrets" | "timeline" | "versionfiles" | "fileat" | "hasfile" | "goback" | "gobackfile" | "undo" | "history" | "checkpoint" | "snapshot" | "keepcopy" | "localhistory" | "localfiles" | "snapshottext"
    )
}

/// `path` under the project's folder unless it is absolute already — or,
/// on the web, already a path of the page's store under the folder
/// (`tests/app/main.rr` with the folder `tests/app`).
fn abs(r: &Repo, path: &str) -> String {
    let p = slashes(path);
    let f = r.folder.trim_end_matches('/');
    if p.starts_with('/') || p.get(1..3) == Some(":/") || (!f.is_empty() && p.starts_with(&format!("{f}/"))) {
        p
    } else {
        join(&r.folder, &p)
    }
}

/// `path` relative to the project's folder (as versions name files).
fn rel(r: &Repo, path: &str) -> String {
    local_history::key_of(&r.folder, path)
}

/// The account's name, when Studio was given none (Settings).
fn default_author() -> String {
    #[cfg(not(target_arch = "wasm32"))]
    {
        for k in ["RAPIDR_VERSIONS_NAME", "USER", "USERNAME"] {
            if let Ok(v) = std::env::var(k) {
                if !v.trim().is_empty() {
                    return v.trim().to_string();
                }
            }
        }
    }
    "RapidR user".to_string()
}

// ---- the desktop: rapidr-vcs ------------------------------------------------------

#[cfg(not(target_arch = "wasm32"))]
mod engine {
    use std::path::{Path, PathBuf};

    use rapidr_vcs::{ErrorKind, FileState, Person, SaveOptions, Versions};

    use super::{abs, default_author, failed, ok, rel, Repo};

    fn person(r: &Repo) -> Person {
        let name = if r.author.is_empty() { default_author() } else { r.author.clone() };
        Person::new(&name, &r.email)
    }

    fn kind_word(k: ErrorKind) -> &'static str {
        match k {
            ErrorKind::NothingToSave => "nothing-to-save",
            ErrorKind::UnsavedChanges => "unsaved-changes",
            ErrorKind::NotFound => "not-found",
            ErrorKind::InvalidName => "invalid",
            ErrorKind::ReadOnly => "read-only",
            ErrorKind::NotYours => "not-yours",
            ErrorKind::SecretsFound => "secrets",
            ErrorKind::NothingToUndo => "nothing-to-undo",
            ErrorKind::ChangedMeanwhile => "changed-meanwhile",
            ErrorKind::Conflicts => "conflicts",
            ErrorKind::NotAProject => "not-versioned",
            ErrorKind::Failed => "failed",
        }
    }

    fn said(r: &mut Repo, e: rapidr_vcs::Error) {
        if e.kind == ErrorKind::SecretsFound {
            r.secrets = e.secrets.iter().map(|s| format!("{}\t{}\t{}\t{}", s.path, s.line, s.what, s.masked)).collect();
        }
        failed(r, kind_word(e.kind), e.message);
    }

    fn dir(r: &Repo) -> PathBuf {
        PathBuf::from(&r.folder)
    }

    /// Opened for changes, else (another account's) read-only.
    fn open(r: &Repo) -> Result<Versions, rapidr_vcs::Error> {
        match Versions::open(&dir(r), &person(r)) {
            Err(e) if e.kind == ErrorKind::NotYours => Versions::open_read_only(&dir(r), &person(r)),
            other => other,
        }
    }

    pub fn read_state(r: &mut Repo) -> Result<(), String> {
        if !dir(r).join(".git").exists() {
            return Ok(());
        }
        let v = open(r).map_err(|e| e.message)?;
        r.versioned = true;
        r.read_only = v.is_read_only();
        r.experiment = v.current_experiment().unwrap_or_default();
        r.changes = v
            .changes()
            .map_err(|e| e.message)?
            .into_iter()
            .map(|c| {
                let s = match c.state {
                    FileState::New => 'N',
                    FileState::Changed => 'M',
                    FileState::Deleted => 'D',
                };
                (s, c.path)
            })
            .collect();
        Ok(())
    }

    /// Why versioning shouldn't start in `d` (words), if it shouldn't: a
    /// drive itself, the home folder or its Desktop, Documents or
    /// Downloads (versions of everything in them), a folder inside another
    /// one that has versions.
    fn refusal(d: &Path) -> Option<String> {
        let canon = d.canonicalize().ok()?;
        if canon.parent().is_none() {
            return Some(rapidr_value::i18n::tr_ui("Versions keep a project's own folder, and this is a whole drive. Put the project in a folder of its own first (File > New Project)."));
        }
        let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from).and_then(|h| h.canonicalize().ok());
        if let Some(home) = home {
            let shared = [home.clone(), home.join("Desktop"), home.join("Documents"), home.join("Downloads")];
            if shared.iter().any(|s| *s == canon) {
                return Some(rapidr_value::i18n::tr_ui_fill("Versions keep a project's own folder, and {folder} holds much more than this project. Put the project in a folder of its own first (File > New Project, or Save All into a new folder).", &[("folder", &canon.display().to_string())]));
            }
        }
        for up in canon.ancestors().skip(1) {
            if up.join(".git").exists() {
                return Some(rapidr_value::i18n::tr_ui_fill("This project is inside {folder}, which already has versions of its own. Studio keeps the versions of a project's own folder only, so it doesn't start new ones inside it. Copy the project into a folder of its own to version it here.", &[("folder", &up.display().to_string())]));
            }
        }
        None
    }

    pub fn start(r: &mut Repo) -> bool {
        if r.folder.is_empty() {
            failed(r, "not-versioned", rapidr_value::i18n::tr_ui("Open a project first: versions are kept for a project's folder."));
            return false;
        }
        let d = dir(r);
        if !d.join(".git").exists() {
            if let Some(why) = refusal(&d) {
                failed(r, "refused", why);
                return false;
            }
        }
        match Versions::start(&d, &person(r)) {
            Ok(_) => {
                if let Err(e) = rapidr_vcs::write_ignore_file(&d) {
                    said(r, e);
                    return false;
                }
                ok(r);
                true
            }
            Err(e) => {
                said(r, e);
                false
            }
        }
    }

    pub fn save(r: &mut Repo, message: &str, files: &str, allow: bool) -> Option<String> {
        r.secrets.clear();
        let only: Vec<String> = files.lines().map(|l| rel(r, l.trim())).filter(|l| !l.is_empty()).collect();
        let options = SaveOptions { files: (!only.is_empty()).then_some(only), allow_secrets: allow };
        let who = person(r);
        let res = open(r).and_then(|mut v| v.save_version_with(message, &who, &options));
        match res {
            Ok(id) => {
                ok(r);
                Some(id)
            }
            Err(e) => {
                said(r, e);
                None
            }
        }
    }

    pub fn check_secrets(r: &mut Repo, files: &str) -> usize {
        let only: Vec<String> = files.lines().map(|l| rel(r, l.trim())).filter(|l| !l.is_empty()).collect();
        match open(r).and_then(|v| v.check_secrets((!only.is_empty()).then_some(only.as_slice()))) {
            Ok(found) => {
                ok(r);
                r.secrets = found.iter().map(|s| format!("{}\t{}\t{}\t{}", s.path, s.line, s.what, s.masked)).collect();
                found.len()
            }
            Err(e) => {
                said(r, e);
                0
            }
        }
    }

    /// The versions, newest first, RAI's checkpoints among them by time:
    /// id, short id, when (seconds since 1970), who, the message's first
    /// line, "version" or "checkpoint".
    pub fn timeline(r: &mut Repo, limit: usize) -> String {
        let v = match open(r) {
            Ok(v) => v,
            Err(e) => {
                if e.kind != ErrorKind::NotAProject {
                    said(r, e);
                }
                return String::new();
            }
        };
        let mut rows: Vec<(i64, String)> = Vec::new();
        match v.timeline(limit) {
            Ok(list) => {
                for x in list {
                    let first = x.message.lines().next().unwrap_or("").trim().to_string();
                    rows.push((x.seconds, format!("{}\t{}\t{}\t{}\t{}\tversion", x.id, x.short_id(), x.seconds, x.author, first)));
                }
            }
            Err(e) => {
                said(r, e);
                return String::new();
            }
        }
        if let Ok(cps) = v.checkpoints() {
            for c in cps {
                let label = c.label.strip_prefix("Checkpoint: ").unwrap_or(&c.label).lines().next().unwrap_or("").to_string();
                let short = &c.id[..c.id.len().min(7)];
                rows.push((c.seconds, format!("{}\t{}\t{}\t{}\t{}\tcheckpoint", c.id, short, c.seconds, c.author, label)));
            }
        }
        // (newest first; the same second keeps the versions' own order)
        rows.sort_by_key(|(t, _)| std::cmp::Reverse(*t));
        ok(r);
        rows.into_iter().take(limit).map(|(_, l)| l).collect::<Vec<_>>().join("\n")
    }

    /// What a version changed (`""`: what changed since the last version),
    /// a line each: the state letter (M, N, D), the path.
    pub fn version_files(r: &mut Repo, version: &str) -> String {
        let res = open(r).and_then(|v| if version.trim().is_empty() { v.what_changed() } else { v.changes_in(version.trim()) });
        match res {
            Ok(diffs) => {
                ok(r);
                diffs
                    .into_iter()
                    .map(|d| {
                        let s = match d.state {
                            FileState::New => 'N',
                            FileState::Changed => 'M',
                            FileState::Deleted => 'D',
                        };
                        format!("{s}\t{}", d.path)
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            Err(e) => {
                said(r, e);
                String::new()
            }
        }
    }

    /// `path` as `version` has it (`""`: as the disk has it now); `None`
    /// when it hasn't the file.
    pub fn file_at(r: &mut Repo, path: &str, version: &str) -> Option<String> {
        if version.trim().is_empty() {
            return std::fs::read(abs(r, path)).ok().map(|b| String::from_utf8_lossy(&b).into_owned());
        }
        let p = rel(r, path);
        match open(r).and_then(|v| v.file_at(&p, version.trim())) {
            Ok(b) => {
                ok(r);
                b.map(|b| String::from_utf8_lossy(&b).into_owned())
            }
            Err(e) => {
                // (no version yet: nothing to compare with, not an error)
                if e.kind != ErrorKind::NotFound {
                    said(r, e);
                }
                None
            }
        }
    }

    pub fn go_back(r: &mut Repo, version: &str) -> Option<String> {
        let who = person(r);
        match open(r).and_then(|mut v| v.go_back(version.trim(), &who)) {
            Ok(id) => {
                ok(r);
                Some(id)
            }
            Err(e) => {
                said(r, e);
                None
            }
        }
    }

    pub fn go_back_file(r: &mut Repo, path: &str, version: &str) -> Option<String> {
        let who = person(r);
        let p = rel(r, path);
        match open(r).and_then(|mut v| v.go_back_file(&p, version.trim(), &who)) {
            Ok(id) => {
                ok(r);
                Some(id)
            }
            Err(e) => {
                said(r, e);
                None
            }
        }
    }

    pub fn undo(r: &mut Repo) -> Option<String> {
        match open(r).and_then(|mut v| v.undo()) {
            Ok(words) => {
                ok(r);
                Some(words)
            }
            Err(e) => {
                said(r, e);
                None
            }
        }
    }

    pub fn can_undo(r: &Repo) -> Option<String> {
        if !r.versioned {
            return None;
        }
        open(r).ok()?.can_undo().ok().flatten()
    }

    /// The history of actions, newest first: its number, when, its kind,
    /// 1 when it was undone, its words.
    pub fn history(r: &mut Repo) -> String {
        match open(r).and_then(|v| v.history()) {
            Ok(list) => {
                ok(r);
                list.iter().rev().map(|a| format!("{}\t{}\t{}\t{}\t{}", a.n, a.seconds, a.kind, i32::from(a.undone), a.words)).collect::<Vec<_>>().join("\n")
            }
            Err(e) => {
                said(r, e);
                String::new()
            }
        }
    }

    pub fn checkpoint(r: &mut Repo, label: &str) -> Option<String> {
        let who = person(r);
        match open(r).and_then(|mut v| v.checkpoint(label, &who)) {
            Ok(s) => {
                ok(r);
                Some(s.id)
            }
            Err(e) => {
                said(r, e);
                None
            }
        }
    }
}

// ---- the web: no versions (gitoxide doesn't build for a browser) ---------------------

#[cfg(target_arch = "wasm32")]
mod engine {
    use super::{failed, Repo, NOT_ON_THE_WEB};

    fn no(r: &mut Repo) {
        failed(r, "not-available", rapidr_value::i18n::tr_ui("Versions need RapidR Studio on your computer: in a browser, Studio keeps a local history of your saves only."));
    }

    pub fn read_state(_: &mut Repo) -> Result<(), String> {
        Ok(())
    }
    pub fn start(r: &mut Repo) -> bool {
        no(r);
        false
    }
    pub fn save(r: &mut Repo, _: &str, _: &str, _: bool) -> Option<String> {
        no(r);
        None
    }
    pub fn check_secrets(r: &mut Repo, _: &str) -> usize {
        no(r);
        0
    }
    pub fn timeline(_: &mut Repo, _: usize) -> String {
        String::new()
    }
    pub fn version_files(r: &mut Repo, _: &str) -> String {
        no(r);
        String::new()
    }
    pub fn file_at(_: &mut Repo, _: &str, _: &str) -> Option<String> {
        None
    }
    pub fn go_back(r: &mut Repo, _: &str) -> Option<String> {
        no(r);
        None
    }
    pub fn go_back_file(r: &mut Repo, _: &str, _: &str) -> Option<String> {
        no(r);
        None
    }
    pub fn undo(r: &mut Repo) -> Option<String> {
        no(r);
        None
    }
    pub fn can_undo(_: &Repo) -> Option<String> {
        None
    }
    pub fn history(_: &mut Repo) -> String {
        String::new()
    }
    pub fn checkpoint(r: &mut Repo, _: &str) -> Option<String> {
        no(r);
        None
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use std::process::Command;

    fn s(v: Option<Value>) -> String {
        v.unwrap().to_string_val()
    }

    fn n(v: Option<Value>) -> i64 {
        v.unwrap().to_i64()
    }

    /// The system's git, isolated, as an independent judge (skipped when
    /// it isn't installed): the repository is valid.
    fn fsck(dir: &std::path::Path) {
        let Ok(out) = Command::new("git").args(["fsck", "--full", "--strict", "--no-dangling"]).current_dir(dir).env("GIT_CONFIG_NOSYSTEM", "1").env("GIT_CONFIG_GLOBAL", "/dev/null").output() else { return };
        assert!(out.status.success(), "git fsck: {}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    }

    /// The acceptance walk of lane G3 through the component: start, save
    /// "first", change, the changes, save "second", the timeline, what
    /// "first" changed, go back to it, Undo; the local history of a file.
    #[test]
    fn the_walk() {
        let t = tempfile::tempdir().unwrap();
        let f = slashes(&t.path().join("Hello").to_string_lossy());
        std::fs::create_dir_all(&f).unwrap();
        std::fs::write(format!("{f}/main.rr"), "PRINT \"one\"\n").unwrap();
        let r = "repo_walk";
        set(r, "author", &text("Ana"));
        set(r, "folder", &text(&f));
        assert_eq!(n(get(r, "versioned")), 0);
        assert_eq!(n(call(r, "start", &[])), 1, "{}", s(get(r, "error")));
        assert_eq!(s(get(r, "experiment")), "main");
        assert!(std::fs::read_to_string(format!("{f}/.gitignore")).unwrap().contains(".rapidr/"));
        // the local history Studio keeps: never a change
        assert_eq!(s(call(r, "snapshot", &[text("main.rr")])), "1");
        assert_eq!(s(get(r, "changes")), "N\t.gitignore\nN\tmain.rr");
        let first = s(call(r, "saveversion", &[text("first")]));
        assert_eq!(first.len(), 40, "{}", s(get(r, "error")));
        assert_eq!(n(get(r, "changecount")), 0);
        std::fs::write(format!("{f}/main.rr"), "PRINT \"one\"\nPRINT \"two\"\n").unwrap();
        assert_eq!(s(get(r, "changes")), "", "the state is as last read");
        call(r, "refresh", &[]);
        assert_eq!(s(get(r, "changes")), "M\tmain.rr");
        assert_eq!(s(call(r, "fileat", &[text("main.rr"), text("HEAD")])), "PRINT \"one\"\n");
        assert_eq!(s(call(r, "versionfiles", &[text("")])), "M\tmain.rr");
        let second = s(call(r, "saveversion", &[text("second"), text("main.rr")]));
        assert_eq!(second.len(), 40);
        let tl = s(call(r, "timeline", &[]));
        let rows: Vec<&str> = tl.lines().collect();
        assert_eq!(rows.len(), 2);
        assert!(rows[0].starts_with(&second) && rows[0].contains("\tAna\tsecond\tversion"), "{}", rows[0]);
        // what "first" changed: every file new
        assert_eq!(s(call(r, "versionfiles", &[text(&first)])), "N\t.gitignore\nN\tmain.rr");
        assert_eq!(s(call(r, "versionfiles", &[text(&second)])), "M\tmain.rr");
        // go back to "first": a third version, main.rr as it was
        let back = s(call(r, "goback", &[text(&first)]));
        assert_eq!(back.len(), 40, "{}", s(get(r, "error")));
        assert_eq!(std::fs::read_to_string(format!("{f}/main.rr")).unwrap(), "PRINT \"one\"\n");
        let tl = s(call(r, "timeline", &[]));
        assert_eq!(tl.lines().count(), 3);
        assert!(tl.lines().next().unwrap().contains("\tWent back to \u{201C}first\u{201D}"), "{tl}");
        assert!(s(get(r, "canundo")).starts_with("Went back to"));
        fsck(t.path().join("Hello").as_path());
        // Undo: "second" again
        assert!(s(call(r, "undo", &[])).starts_with("Went back to"));
        assert_eq!(std::fs::read_to_string(format!("{f}/main.rr")).unwrap(), "PRINT \"one\"\nPRINT \"two\"\n");
        assert_eq!(s(call(r, "timeline", &[])).lines().count(), 2);
        assert!(s(call(r, "history", &[])).lines().next().unwrap().contains("\tundo\t0\tUndid: Went back to"));
        fsck(t.path().join("Hello").as_path());
        // nothing to save: said in words
        assert_eq!(s(call(r, "saveversion", &[text("again")])), "");
        assert_eq!(s(get(r, "errorkind")), "nothing-to-save");
        // a secret stops the save, said with the line; Save anyway saves
        // (not a real key: two pieces, so no file holds a key's shape)
        std::fs::write(format!("{f}/keys.rr"), concat!("key$ = \"sk", "-ant-api03-abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGH\"\n")).unwrap();
        call(r, "refresh", &[]);
        assert_eq!(s(call(r, "saveversion", &[text("keys")])), "");
        assert_eq!(s(get(r, "errorkind")), "secrets");
        assert!(s(get(r, "secrets")).starts_with("keys.rr\t1\t"), "{}", s(get(r, "secrets")));
        assert_eq!(s(call(r, "saveversion", &[text("keys"), text(""), Value::Integer(1)])).len(), 40);
        fsck(t.path().join("Hello").as_path());
        // the local history: a copy at each save, newest first
        std::fs::write(format!("{f}/main.rr"), "PRINT 3\n").unwrap();
        assert_eq!(s(call(r, "snapshot", &[text(&format!("{f}/main.rr"))])), "2");
        let lh = s(call(r, "localhistory", &[text("main.rr")]));
        assert_eq!(lh.lines().map(|l| l.split('\t').next().unwrap()).collect::<Vec<_>>(), ["2", "1"]);
        assert_eq!(s(call(r, "snapshottext", &[text("1")])), "PRINT \"one\"\n");
        assert_eq!(s(call(r, "localfiles", &[])), "main.rr");
        assert_eq!(s(call(r, "versionfiles", &[text("")])), "M\tmain.rr", "the history never shows as a change");
        fsck(t.path().join("Hello").as_path());
    }

    #[test]
    fn refuses_a_folder_inside_another_project_with_versions() {
        let t = tempfile::tempdir().unwrap();
        let outer = t.path().join("outer");
        std::fs::create_dir_all(outer.join(".git")).unwrap();
        let inner = slashes(&outer.join("inner").to_string_lossy());
        std::fs::create_dir_all(&inner).unwrap();
        let r = "repo_nested";
        set(r, "folder", &text(&inner));
        assert_eq!(n(call(r, "start", &[])), 0);
        assert_eq!(s(get(r, "errorkind")), "refused");
        assert!(s(get(r, "error")).contains("already has versions"), "{}", s(get(r, "error")));
        assert!(!std::path::Path::new(&format!("{inner}/.git")).exists());
        // a project never versioned: still a local history
        std::fs::write(format!("{inner}/main.rr"), "PRINT 1\n").unwrap();
        for k in 1..=3 {
            std::fs::write(format!("{inner}/main.rr"), format!("PRINT {k}\n")).unwrap();
            call(r, "snapshot", &[text("main.rr")]);
        }
        assert_eq!(s(call(r, "localhistory", &[text("main.rr")])).lines().count(), 3);
        assert_eq!(n(get(r, "available")), 1);
    }
}

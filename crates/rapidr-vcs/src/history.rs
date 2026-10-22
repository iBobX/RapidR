//! Nothing is ever lost (docs/versions-plan.md §1): safety points, RAI's
//! checkpoints, the history of actions and Undo.
//!
//! - A **snapshot** is a commit of the whole working folder as it is —
//!   saved and unsaved changes, new files (not ignored ones) — whose
//!   parent is the current version. It never moves the current
//!   experiment or touches the index; a hidden reference keeps it:
//!   `refs/rapidr/safety/<n>` (taken before a risky action) or
//!   `refs/rapidr/checkpoints/<n>` (RAI's, ai-ux-plan §3). Other Git tools
//!   see ordinary commits; sharing never sends them.
//! - The **history** (`.git/rapidr/history.jsonl`, one JSON object per
//!   line, append-only) records each action: what it was in words, the
//!   references it moved (from what, to what), the safety point taken
//!   before it and the working folder's tree right after it.
//! - **Undo** takes back the last action not undone yet: the references
//!   go back (only if they are still where the action left them), the
//!   files the action rewrote come back from its safety point (only if
//!   nobody changed them since), the index follows the current version.
//!   Undo is itself an action: it has a safety point and a history entry.

use std::path::PathBuf;

use gix::bstr::ByteSlice;
use gix::refs::transaction::PreviousValue;
use gix::refs::Target;
use gix::ObjectId;

use crate::{checkout, delete_ref, edit_ref, fail, Ctx, Error, ErrorKind, Person, Result, Versions};

/// A reference's value: none, a commit, or another reference (HEAD).
#[derive(Clone, Debug, PartialEq, Eq)]
enum RefValue {
    None,
    Id(ObjectId),
    Symbolic(String),
}

impl RefValue {
    fn to_json(&self) -> serde_json::Value {
        match self {
            RefValue::None => serde_json::Value::Null,
            RefValue::Id(id) => serde_json::Value::String(id.to_string()),
            RefValue::Symbolic(s) => serde_json::json!({ "ref": s }),
        }
    }

    fn from_json(v: &serde_json::Value) -> Option<RefValue> {
        match v {
            serde_json::Value::Null => Some(RefValue::None),
            serde_json::Value::String(s) => ObjectId::from_hex(s.as_bytes()).ok().map(RefValue::Id),
            serde_json::Value::Object(o) => Some(RefValue::Symbolic(o.get("ref")?.as_str()?.to_string())),
            _ => None,
        }
    }
}

/// A reference an action moved.
#[derive(Clone, Debug, PartialEq, Eq)]
struct RefMove {
    name: String,
    old: RefValue,
    new: RefValue,
}

/// One entry of the history (see the module's notes).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Entry {
    n: u64,
    kind: String,
    words: String,
    seconds: i64,
    refs: Vec<RefMove>,
    /// The safety point taken before (`None`: the action rewrote no file).
    before: Option<ObjectId>,
    /// The working folder's tree right after.
    after: Option<ObjectId>,
    /// For an undo: the entry it took back.
    undo_of: Option<u64>,
}

impl Entry {
    pub(crate) fn new(kind: &str, words: String) -> Entry {
        Entry { n: 0, kind: kind.into(), words, seconds: now(), refs: Vec::new(), before: None, after: None, undo_of: None }
    }

    /// Branch `name` moved from `old` to `new` (`None`: didn't exist).
    pub(crate) fn moved(mut self, name: &str, old: Option<ObjectId>, new: Option<ObjectId>) -> Entry {
        let v = |x: Option<ObjectId>| x.map_or(RefValue::None, RefValue::Id);
        self.refs.push(RefMove { name: name.into(), old: v(old), new: v(new) });
        self
    }

    /// HEAD moved from branch `old` to branch `new`.
    pub(crate) fn head(mut self, old: &str, new: &str) -> Entry {
        self.refs.push(RefMove { name: "HEAD".into(), old: RefValue::Symbolic(old.into()), new: RefValue::Symbolic(new.into()) });
        self
    }

    pub(crate) fn with_before(mut self, safety: ObjectId) -> Entry {
        self.before = Some(safety);
        self
    }

    pub(crate) fn with_after(mut self, tree: ObjectId) -> Entry {
        self.after = Some(tree);
        self
    }

    fn to_json(&self) -> String {
        let id = |x: Option<ObjectId>| x.map_or(serde_json::Value::Null, |i| serde_json::Value::String(i.to_string()));
        serde_json::json!({
            "n": self.n,
            "kind": self.kind,
            "words": self.words,
            "seconds": self.seconds,
            "refs": self.refs.iter().map(|r| serde_json::json!({ "name": r.name, "old": r.old.to_json(), "new": r.new.to_json() })).collect::<Vec<_>>(),
            "before": id(self.before),
            "after": id(self.after),
            "undo_of": self.undo_of,
        })
        .to_string()
    }

    fn from_json(line: &str) -> Option<Entry> {
        let v: serde_json::Value = serde_json::from_str(line).ok()?;
        let id = |k: &str| v.get(k).and_then(|s| s.as_str()).and_then(|s| ObjectId::from_hex(s.as_bytes()).ok());
        let refs = v
            .get("refs")?
            .as_array()?
            .iter()
            .map(|r| Some(RefMove { name: r.get("name")?.as_str()?.to_string(), old: RefValue::from_json(r.get("old")?)?, new: RefValue::from_json(r.get("new")?)? }))
            .collect::<Option<Vec<_>>>()?;
        Some(Entry {
            n: v.get("n")?.as_u64()?,
            kind: v.get("kind")?.as_str()?.to_string(),
            words: v.get("words")?.as_str()?.to_string(),
            seconds: v.get("seconds")?.as_i64()?,
            refs,
            before: id("before"),
            after: id("after"),
            undo_of: v.get("undo_of").and_then(|u| u.as_u64()),
        })
    }
}

fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

/// One action in the history, as the person sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Action {
    /// Its number (from 1, in order).
    pub n: u64,
    /// What it was: `save`, `go-back`, `restore-file`, `switch`,
    /// `start-experiment`, `bring-into`, `get-latest`, `restore-snapshot`,
    /// `undo`.
    pub kind: String,
    /// In words: "Saved the version “Bigger button”".
    pub words: String,
    pub seconds: i64,
    /// It was undone since.
    pub undone: bool,
    /// The safety point taken before it (a snapshot's id), if any.
    pub safety_point: Option<String>,
}

/// A safety point or a checkpoint: a snapshot of the working folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    /// Its commit's id: give it to [`Versions::restore_snapshot`] or
    /// [`Versions::compare_versions`].
    pub id: String,
    /// Its label ("Before: Switched to the experiment “idea”", RAI's
    /// "Before: make the button bigger").
    pub label: String,
    pub author: String,
    pub seconds: i64,
    /// Its number in its list (from 1).
    pub n: u64,
}

const SAFETY: &str = "refs/rapidr/safety/";
const CHECKPOINTS: &str = "refs/rapidr/checkpoints/";

impl Versions {
    fn history_path(&self) -> PathBuf {
        self.git_dir().join("rapidr").join("history.jsonl")
    }

    fn entries(&self) -> Result<Vec<Entry>> {
        let text = match std::fs::read_to_string(self.history_path()) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e).ctx("Could not read the history of actions."),
        };
        // (a line cut short by a crash is skipped, not fatal)
        Ok(text.lines().filter_map(Entry::from_json).collect())
    }

    /// Adds `entry` to the history (numbered after the last one).
    pub(crate) fn record(&self, mut entry: Entry) -> Result<()> {
        use std::io::Write;
        entry.n = self.entries()?.last().map_or(1, |e| e.n + 1);
        let path = self.history_path();
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).ctx("Could not write the history of actions.")?;
        }
        let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&path).ctx("Could not write the history of actions.")?;
        writeln!(f, "{}", entry.to_json()).ctx("Could not write the history of actions.")?;
        Ok(())
    }

    /// The **history** of actions, oldest first.
    pub fn history(&self) -> Result<Vec<Action>> {
        let entries = self.entries()?;
        let undone: Vec<u64> = entries.iter().filter_map(|e| e.undo_of).collect();
        Ok(entries
            .iter()
            .map(|e| Action { n: e.n, kind: e.kind.clone(), words: e.words.clone(), seconds: e.seconds, undone: undone.contains(&e.n), safety_point: e.before.map(|b| b.to_string()) })
            .collect())
    }

    /// What [`Versions::undo`] would take back, in words (`None`: nothing).
    pub fn can_undo(&self) -> Result<Option<String>> {
        Ok(self.undoable()?.map(|e| e.words))
    }

    fn undoable(&self) -> Result<Option<Entry>> {
        let entries = self.entries()?;
        let undone: Vec<u64> = entries.iter().filter_map(|e| e.undo_of).collect();
        Ok(entries.into_iter().rev().find(|e| e.kind != "undo" && !undone.contains(&e.n)))
    }

    /// The value reference `name` has now.
    fn ref_value(&self, name: &str) -> Result<RefValue> {
        if name == "HEAD" {
            return Ok(match self.repo.head_name().ctx("Could not read the current version.")? {
                Some(n) => RefValue::Symbolic(n.as_bstr().to_string()),
                None => match self.head_id()? {
                    Some(id) => RefValue::Id(id),
                    None => RefValue::None,
                },
            });
        }
        Ok(match self.repo.try_find_reference(name).ctx("Could not read the experiments.")? {
            Some(mut r) => RefValue::Id(r.peel_to_id().ctx("Could not read the experiments.")?.detach()),
            None => RefValue::None,
        })
    }

    /// **Undo** the last action not undone yet (see the module's notes);
    /// returns what was undone, in words. Refused, changing nothing, when
    /// the project moved on since ([`ErrorKind::ChangedMeanwhile`]) or the
    /// files it would rewrite were changed since ([`ErrorKind::UnsavedChanges`]).
    pub fn undo(&mut self) -> Result<String> {
        self.writable()?;
        let Some(entry) = self.undoable()? else {
            return fail(ErrorKind::NothingToUndo, "There is nothing to undo.");
        };
        // the references must still be where the action left them
        for r in &entry.refs {
            if self.ref_value(&r.name)? != r.new {
                return fail(
                    ErrorKind::ChangedMeanwhile,
                    format!("\u{201C}{}\u{201D} can't be undone any more: the project changed after it (a newer version, another experiment or another program). Nothing was changed.", entry.words),
                );
            }
        }
        // the files it rewrote must be as it left them
        let mut paths = Vec::new();
        if let (Some(before), Some(after)) = (entry.before, entry.after) {
            let before_tree = self.tree_id_of_commit(before)?;
            let now = self.working_tree()?;
            for (path, _, _) in checkout::tree_changes(&self.repo, Some(before_tree), after)? {
                let p = path.to_str_lossy().to_string();
                if checkout::entry_at(&self.repo, now, &p)? != checkout::entry_at(&self.repo, after, &p)? {
                    return fail(
                        ErrorKind::UnsavedChanges,
                        format!("Undoing \u{201C}{}\u{201D} would replace your newer changes to {p}. Save a version first (or undo your own changes), then undo.", entry.words),
                    );
                }
                paths.push(p);
            }
        }
        let words = format!("Undid: {}", entry.words);
        let safety = self.safety_point(&words)?;
        // the references back, HEAD last
        let person = self.person.clone();
        let mut refs = entry.refs.clone();
        refs.sort_by_key(|r| r.name == "HEAD");
        for r in &refs {
            match (&r.old, &r.new) {
                (RefValue::Symbolic(old), _) => {
                    let target = Target::Symbolic(old.as_str().try_into().ctx("Not a valid experiment name.")?);
                    edit_ref(&self.repo, &r.name, target, PreviousValue::Any, format!("rapidr: undo {}", entry.kind), &person)?;
                }
                (RefValue::Id(old), RefValue::Id(new)) => {
                    edit_ref(&self.repo, &r.name, Target::Object(*old), PreviousValue::MustExistAndMatch(Target::Object(*new)), format!("rapidr: undo {}", entry.kind), &person)?;
                }
                (RefValue::Id(old), RefValue::None) => {
                    edit_ref(&self.repo, &r.name, Target::Object(*old), PreviousValue::MustNotExist, format!("rapidr: undo {}", entry.kind), &person)?;
                }
                (RefValue::None, RefValue::Id(new)) => {
                    // (a new experiment goes; the first version undone: the
                    // branch goes and HEAD stays on it, with no version yet)
                    delete_ref(&self.repo, &r.name, Target::Object(*new), &person)?;
                }
                _ => {}
            }
        }
        // the files back from the safety point, the index on the version
        if let Some(before) = entry.before {
            let before_tree = self.tree_id_of_commit(before)?;
            checkout::restore_paths(self, before_tree, &paths)?;
        }
        checkout::reset_index(self)?;
        let after = self.working_tree()?;
        let inverse: Vec<RefMove> = entry.refs.iter().map(|r| RefMove { name: r.name.clone(), old: r.new.clone(), new: r.old.clone() }).collect();
        let mut undo = Entry::new("undo", words).with_before(safety).with_after(after);
        undo.refs = inverse;
        undo.undo_of = Some(entry.n);
        self.record(undo)?;
        Ok(entry.words)
    }

    /// A snapshot of the working folder under `prefix` + the next number;
    /// its id.
    fn snapshot(&self, prefix: &str, label: &str, who: &Person) -> Result<(ObjectId, u64)> {
        let tree = self.working_tree()?;
        let parent = self.head_id()?;
        let sig = who.signature();
        let commit = gix::objs::Commit {
            tree,
            parents: parent.into_iter().collect(),
            author: sig.clone(),
            committer: sig,
            encoding: None,
            message: format!("{label}\n").into(),
            extra_headers: Vec::new(),
        };
        let id = self.repo.write_object(&commit).ctx("Could not save a safety point.")?.detach();
        let n = self.snapshots(prefix)?.iter().map(|s| s.n).max().unwrap_or(0) + 1;
        edit_ref(&self.repo, &format!("{prefix}{n}"), Target::Object(id), PreviousValue::MustNotExist, format!("rapidr: {label}"), who)?;
        Ok((id, n))
    }

    /// A **safety point** before `what` (the action, in words).
    pub(crate) fn safety_point(&self, what: &str) -> Result<ObjectId> {
        let person = self.person.clone();
        Ok(self.snapshot(SAFETY, &format!("Before: {what}"), &person)?.0)
    }

    /// The snapshots under `prefix`, newest first.
    fn snapshots(&self, prefix: &str) -> Result<Vec<Snapshot>> {
        let platform = self.repo.references().ctx("Could not list the safety points.")?;
        let mut out = Vec::new();
        for r in platform.prefixed(prefix).ctx("Could not list the safety points.")? {
            let mut r = r.map_err(|e| Error { kind: ErrorKind::Failed, message: "Could not list the safety points.".into(), detail: e.to_string(), secrets: Vec::new() })?;
            let name = r.name().as_bstr().to_string();
            let Ok(n) = name[prefix.len()..].parse::<u64>() else { continue };
            let id = r.peel_to_id().ctx("Could not read a safety point.")?.detach();
            let v = self.version(id)?;
            out.push(Snapshot { id: v.id, label: v.message, author: v.author, seconds: v.seconds, n });
        }
        out.sort_by_key(|s| std::cmp::Reverse(s.n));
        Ok(out)
    }

    /// The safety points, newest first.
    pub fn safety_points(&self) -> Result<Vec<Snapshot>> {
        self.snapshots(SAFETY)
    }

    /// An RAI **checkpoint**: the working folder as it is now, labelled
    /// (`label`: what RAI is about to do), by `who` (RAI). It changes
    /// nothing: no version, no file, no index.
    pub fn checkpoint(&mut self, label: &str, who: &Person) -> Result<Snapshot> {
        self.writable()?;
        let (id, n) = self.snapshot(CHECKPOINTS, &format!("Checkpoint: {label}"), who)?;
        let v = self.version(id)?;
        Ok(Snapshot { id: v.id, label: v.message, author: v.author, seconds: v.seconds, n })
    }

    /// The checkpoints, newest first.
    pub fn checkpoints(&self) -> Result<Vec<Snapshot>> {
        self.snapshots(CHECKPOINTS)
    }

    /// Puts the working folder back as snapshot `id` (a checkpoint or a
    /// safety point) had it: the files that differ are rewritten, files
    /// it didn't have are removed (ignored files are left alone). The
    /// current version doesn't move: what differs from it shows as unsaved
    /// changes. A safety point first; undoable.
    pub fn restore_snapshot(&mut self, id: &str) -> Result<()> {
        self.writable()?;
        let commit = self.resolve(id)?;
        let tree = self.tree_id_of_commit(commit)?;
        let label = self.version(commit)?.message;
        let now = self.working_tree()?;
        let paths: Vec<String> = checkout::tree_changes(&self.repo, Some(now), tree)?.into_iter().map(|(p, _, _)| p.to_str_lossy().to_string()).collect();
        let words = format!("Went back to {}", label.strip_prefix("Checkpoint: ").map_or(label.clone(), |l| format!("the checkpoint \u{201C}{l}\u{201D}")));
        let safety = self.safety_point(&words)?;
        checkout::restore_paths(self, tree, &paths)?;
        let after = self.working_tree()?;
        self.record(Entry::new("restore-snapshot", words).with_before(safety).with_after(after))
    }
}

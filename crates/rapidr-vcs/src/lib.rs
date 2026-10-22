//! Versions (Git) for RapidR Studio, on gitoxide: the plain-words engine
//! (docs/versions-plan.md, lane G1; the spike's findings in
//! docs/versions-g0.md).
//!
//! What a person does, and what Git calls it:
//! - **Start versioning** a folder (`git init`, branch `main`):
//!   [`Versions::start`]; open one: [`Versions::open`];
//! - **Save a version** of every changed file, or of some (staging and a
//!   commit): [`Versions::save_version`], after the **secret scan**
//!   ([`Versions::check_secrets`]): a line that looks like a key or a
//!   password stops the save, with the file and the line, until the person
//!   says "save anyway" ([`SaveOptions::allow_secrets`]);
//! - the **Timeline** (the log): [`Versions::timeline`]; **What changed?**
//!   ([`Versions::what_changed`], [`Versions::compare_versions`]) with
//!   RapidR's own diff (`rapidr-diff`, Myers' algorithm);
//! - **Go back** to a version — the project ([`Versions::go_back`]: a new
//!   version with the old files, never a destructive reset) or one file
//!   ([`Versions::restore_file`]: the file as it was, unsaved);
//! - **Experiments** (branches): [`Versions::start_experiment`],
//!   [`Versions::switch_to`] (only the files that differ are rewritten);
//! - **Get latest** from, and **share** with, a shared folder (a bare
//!   repository on a USB stick or a network drive), in-process:
//!   [`Versions::get_latest_from_folder`], [`Versions::share_to_folder`].
//!
//! **Nothing is ever lost** (versions-plan §1):
//! - before anything that rewrites files or moves a version, a hidden
//!   **safety point** saves the whole working folder, unsaved changes and
//!   new files included (`refs/rapidr/safety/<n>`: a commit that never
//!   touches the current experiment or the index);
//! - every action goes into the **history** (`.git/rapidr/history.jsonl`)
//!   and the last one can be **undone** ([`Versions::undo`]), even the next
//!   day: the versions it moved go back, the files it rewrote come back
//!   from its safety point, and Undo refuses — changing nothing — when that
//!   would overwrite newer work;
//! - RAI's **checkpoints** (`refs/rapidr/checkpoints/<n>`, ai-ux-plan §3)
//!   are the same kind of snapshot, on demand: [`Versions::checkpoint`],
//!   [`Versions::checkpoints`], [`Versions::restore_snapshot`].
//!
//! **Safety** (versions-plan §5 and §10, kept from the spike):
//! - repositories are opened *isolated*: no system or user git
//!   configuration, no environment, no includes; who saves is given by
//!   Studio ([`Person`]), never read from `~/.gitconfig`;
//! - nothing configured can run a program: filter, diff and merge
//!   drivers, credential helpers, `core.fsmonitor`, `core.sshCommand`,
//!   `remote.*.uploadpack` … are removed from the repository's in-memory
//!   configuration (the file on disk is never rewritten); hooks never run;
//! - the `git` command is never run: shared folders are read and written
//!   in-process;
//! - a repository **owned by another user account** (a USB stick, another
//!   account's folder) isn't opened for changes: [`Versions::open`]
//!   explains why ([`ErrorKind::NotYours`]) and
//!   [`Versions::open_read_only`] opens it to look at, changing nothing.
//!
//! Errors are in plain words ([`Error`]); the technical cause is kept
//! apart ([`Error::detail`]) for Pro mode and bug reports.

mod checkout;
mod folders;
mod history;
mod secrets;

use std::path::Path;

use gix::bstr::{BStr, BString, ByteSlice};
use gix::refs::transaction::{Change, LogChange, PreviousValue, RefEdit, RefLog};
use gix::refs::Target;
use gix::ObjectId;

pub use folders::{create_shared_folder, GetLatest, Share};
pub use history::{Action, Snapshot};
pub use secrets::{scan_text, SecretWarning};

/// What kind of problem stopped an action (the message says it in words).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// Nothing changed since the last version.
    NothingToSave,
    /// Unsaved changes are in the way: save a version first.
    UnsavedChanges,
    /// No such version, experiment, file or folder.
    NotFound,
    /// A name that can't be an experiment's.
    InvalidName,
    /// The project was opened read-only.
    ReadOnly,
    /// The folder belongs to another user account: open it read-only.
    NotYours,
    /// Lines that look like keys or passwords ([`Error::secrets`]).
    SecretsFound,
    /// Nothing in the history to undo.
    NothingToUndo,
    /// The project changed meanwhile (another program, a newer version):
    /// nothing was changed.
    ChangedMeanwhile,
    /// Both sides changed the same lines (the files are listed).
    Conflicts,
    /// The folder isn't a project with versions (or is a shared folder).
    NotAProject,
    /// The disk or the repository refused (see [`Error::detail`]).
    Failed,
}

/// What went wrong, in words a beginner understands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub kind: ErrorKind,
    /// The plain-words message (what [`std::fmt::Display`] shows).
    pub message: String,
    /// The technical cause, for Pro mode and bug reports ("" when none).
    pub detail: String,
    /// For [`ErrorKind::SecretsFound`]: each line that looks like a secret.
    pub secrets: Vec<SecretWarning>,
}

impl Error {
    fn new(kind: ErrorKind, message: impl Into<String>) -> Error {
        Error { kind, message: message.into(), detail: String::new(), secrets: Vec::new() }
    }

    /// The technical cause ("" when there is none).
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

/// Turns a library error into a plain-words one: `what` says what could
/// not be done; the library's text is kept as the detail.
trait Ctx<T> {
    fn ctx(self, what: &str) -> Result<T>;
}

impl<T, E: std::fmt::Display> Ctx<T> for std::result::Result<T, E> {
    fn ctx(self, what: &str) -> Result<T> {
        self.map_err(|e| Error { kind: ErrorKind::Failed, message: what.to_string(), detail: e.to_string(), secrets: Vec::new() })
    }
}

fn fail<T>(kind: ErrorKind, msg: impl Into<String>) -> Result<T> {
    Err(Error::new(kind, msg))
}

/// Who saves a version (Studio's Settings › Versions), or RAI for its
/// checkpoints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Person {
    pub name: String,
    pub email: String,
}

impl Person {
    pub fn new(name: &str, email: &str) -> Person {
        Person { name: name.into(), email: email.into() }
    }

    fn signature(&self) -> gix::actor::Signature {
        gix::actor::Signature { name: self.name.as_str().into(), email: self.email.as_str().into(), time: gix::date::Time::now_local_or_utc() }
    }
}

/// How a file differs from the last saved version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileState {
    New,
    Changed,
    Deleted,
}

/// A file with unsaved changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangedFile {
    pub path: String,
    pub state: FileState,
}

/// One saved version on the timeline.
#[derive(Debug, Clone)]
pub struct Version {
    pub id: String,
    pub message: String,
    pub author: String,
    pub seconds: i64,
    pub parents: usize,
}

impl Version {
    /// The id's first seven characters, as the timeline shows it.
    pub fn short_id(&self) -> &str {
        &self.id[..self.id.len().min(7)]
    }
}

/// One line of a hunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Line {
    Same(String),
    Removed(String),
    Added(String),
}

/// A group of changed lines with up to three lines of context, numbered
/// from 1 like a unified diff (`@@ -old_start,old_len +new_start,new_len @@`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hunk {
    pub old_start: u32,
    pub old_len: u32,
    pub new_start: u32,
    pub new_len: u32,
    pub lines: Vec<Line>,
}

impl Hunk {
    /// The unified-diff header, as `git diff` prints it.
    pub fn header(&self) -> String {
        format!("@@ -{},{} +{},{} @@", self.old_start, self.old_len, self.new_start, self.new_len)
    }
}

/// What changed in one file.
#[derive(Debug, Clone)]
pub struct FileDiff {
    pub path: String,
    pub state: FileState,
    pub binary: bool,
    pub hunks: Vec<Hunk>,
}

/// Who wrote one line of a file, at the current version.
#[derive(Debug, Clone)]
pub struct BlameLine {
    pub line: u32,
    pub version: String,
    pub author: String,
    pub text: String,
}

/// What bringing an experiment into the current one did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BringInto {
    /// Everything in the experiment is already here.
    AlreadyUpToDate,
    /// The current experiment simply moved forward to it (no new version).
    FastForward(String),
    /// A new version joining both (two parents).
    Merged(String),
    /// Both changed the same lines: nothing was changed (paths listed).
    Conflicts(Vec<String>),
}

/// How to save a version.
#[derive(Debug, Clone, Default)]
pub struct SaveOptions {
    /// Only these files (paths relative to the project, `/` separated);
    /// `None`: every changed file.
    pub files: Option<Vec<String>>,
    /// Save even when lines look like secrets (the person said "save
    /// anyway" after seeing the warnings).
    pub allow_secrets: bool,
}

/// The options every repository is opened with: only its own
/// configuration, never the system's, the user's, the environment's or
/// included files.
fn open_options() -> gix::open::Options {
    gix::open::Options::isolated()
}

/// Whether `path` belongs to the account running Studio.
fn owned_by_me(path: &Path) -> bool {
    gix::sec::identity::is_path_owned_by_current_user(path).unwrap_or(false)
}

/// A project folder under versions.
pub struct Versions {
    repo: gix::Repository,
    /// Who is working (reflogs, safety points).
    person: Person,
    /// Opened read-only (another account's folder): nothing is written.
    read_only: bool,
}

impl Versions {
    /// **Start versioning this project**: `git init` with `main` (an
    /// existing repository is opened instead). `person` is who works on
    /// it (Studio's Settings > Versions).
    pub fn start(dir: &Path, person: &Person) -> Result<Versions> {
        if dir.join(".git").exists() {
            return Versions::open(dir, person);
        }
        if !owned_by_me(dir) {
            return fail(ErrorKind::NotYours, "This folder belongs to another user account on this computer, so RapidR won't start versioning it. Copy the project into one of your own folders first.");
        }
        let repo = gix::ThreadSafeRepository::init_opts(dir, gix::create::Kind::WithWorktree, gix::create::Options::default(), open_options())
            .ctx("Could not start versioning this folder (it may be read-only).")?
            .to_thread_local();
        Versions::from_repo(repo, person, false)
    }

    /// Open the project at `dir` (its own folder only: a repository in a
    /// parent folder is not used). A folder that belongs to another user
    /// account is refused ([`ErrorKind::NotYours`]): open it with
    /// [`Versions::open_read_only`] to look at it.
    pub fn open(dir: &Path, person: &Person) -> Result<Versions> {
        Versions::open_checked(dir, person, false, owned_by_me)
    }

    /// Open the project at `dir` to look at it — the timeline, what
    /// changed, the files of any version — changing nothing in it: no
    /// version, safety point, history entry or index is written.
    pub fn open_read_only(dir: &Path, person: &Person) -> Result<Versions> {
        Versions::open_checked(dir, person, true, owned_by_me)
    }

    fn open_checked(dir: &Path, person: &Person, read_only: bool, mine: fn(&Path) -> bool) -> Result<Versions> {
        if !dir.join(".git").exists() {
            return fail(ErrorKind::NotAProject, "This folder has no versions yet: start versioning it first.");
        }
        let git_dir = dir.join(".git");
        if !read_only && !(mine(dir) && mine(&git_dir)) {
            return fail(
                ErrorKind::NotYours,
                "This project belongs to another user account on this computer (or to another computer, on a USB stick or a network drive). \
                 RapidR can't be sure its versions are safe to work on, so it doesn't open it for changes. \
                 You can open it read-only to look at its versions, or copy the project into one of your own folders.",
            );
        }
        let repo = gix::open_opts(dir, open_options()).ctx("Could not open the versions of this folder (they may be damaged).")?;
        Versions::from_repo(repo, person, read_only)
    }

    fn from_repo(mut repo: gix::Repository, person: &Person, read_only: bool) -> Result<Versions> {
        if repo.workdir().is_none() {
            return fail(ErrorKind::NotAProject, "This is a shared folder of versions (it has no project files): open the project instead, or get the latest from it.");
        }
        neutralize_programs(&mut repo)?;
        Ok(Versions { repo, person: person.clone(), read_only })
    }

    /// Whether it was opened read-only.
    pub fn is_read_only(&self) -> bool {
        self.read_only
    }

    fn writable(&self) -> Result<()> {
        if self.read_only {
            return fail(ErrorKind::ReadOnly, "This project is open read-only (it belongs to another user account), so nothing in it can be changed. Copy it into one of your own folders to work on it.");
        }
        Ok(())
    }

    /// The working folder.
    pub fn folder(&self) -> &Path {
        self.repo.workdir().expect("checked at open")
    }

    /// The git directory (for tests and diagnostics).
    pub fn git_dir(&self) -> &Path {
        self.repo.git_dir()
    }

    /// The current experiment (branch) name, e.g. `main`.
    pub fn current_experiment(&self) -> Result<String> {
        match self.repo.head_name().ctx("Could not read which experiment is current.")? {
            Some(name) => Ok(name.shorten().to_string()),
            None => fail(ErrorKind::NotFound, "The project isn't on an experiment right now (Git calls this a detached HEAD)."),
        }
    }

    /// The experiments (branches), sorted by name.
    pub fn experiments(&self) -> Result<Vec<String>> {
        let refs = self.repo.references().ctx("Could not list the experiments.")?;
        let mut out = Vec::new();
        for r in refs.local_branches().ctx("Could not list the experiments.")? {
            let r = r.map_err(|e| Error { kind: ErrorKind::Failed, message: "Could not list the experiments.".into(), detail: e.to_string(), secrets: Vec::new() })?;
            out.push(r.name().shorten().to_string());
        }
        out.sort();
        Ok(out)
    }

    fn head_id(&self) -> Result<Option<ObjectId>> {
        let mut head = self.repo.head().ctx("Could not read the current version.")?;
        Ok(head.try_peel_to_id().ctx("Could not read the current version.")?.map(|id| id.detach()))
    }

    /// The full name of the branch HEAD points to (`refs/heads/main`).
    fn head_ref(&self) -> Result<String> {
        match self.repo.head_name().ctx("Could not read which experiment is current.")? {
            Some(name) => Ok(name.as_bstr().to_string()),
            None => fail(ErrorKind::NotFound, "The project isn't on an experiment right now (Git calls this a detached HEAD)."),
        }
    }

    fn tree_id_of_commit(&self, commit: ObjectId) -> Result<ObjectId> {
        Ok(self.repo.find_commit(commit).ctx("Could not read a version.")?.tree_id().ctx("Could not read a version.")?.detach())
    }

    fn head_tree_id(&self) -> Result<Option<ObjectId>> {
        match self.head_id()? {
            Some(id) => Ok(Some(self.tree_id_of_commit(id)?)),
            None => Ok(None),
        }
    }

    /// Files that differ from the last saved version: changed, deleted,
    /// and new (untracked, not ignored) files.
    pub fn changes(&self) -> Result<Vec<ChangedFile>> {
        use gix::status::index_worktree::Item;
        use gix::status::plumbing::index_as_worktree::{Change as WtChange, EntryStatus};

        let iter = self
            .repo
            .status(gix::progress::Discard)
            .ctx("Could not check the project for changes.")?
            .untracked_files(gix::status::UntrackedFiles::Files)
            .index_worktree_rewrites(None)
            .into_index_worktree_iter(Vec::<BString>::new())
            .ctx("Could not check the project for changes.")?;
        let mut out = Vec::new();
        for item in iter {
            let item = item.ctx("Could not check the project for changes.")?;
            match item {
                Item::Modification { rela_path, status, .. } => {
                    let state = match status {
                        EntryStatus::Change(WtChange::Removed) => FileState::Deleted,
                        EntryStatus::Change(_) | EntryStatus::Conflict { .. } | EntryStatus::IntentToAdd => FileState::Changed,
                        EntryStatus::NeedsUpdate(_) => continue,
                    };
                    out.push(ChangedFile { path: rela_path.to_string(), state });
                }
                Item::DirectoryContents { entry, .. } => {
                    if entry.status == gix::dir::entry::Status::Untracked {
                        out.push(ChangedFile { path: entry.rela_path.to_string(), state: FileState::New });
                    }
                }
                Item::Rewrite { .. } => {}
            }
        }
        out.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(out)
    }

    /// The index with `changes` (those in `only`, when given) put in, and
    /// the tree it makes: the blobs are written, the index isn't.
    fn staged(&self, changes: &[ChangedFile], only: Option<&[String]>) -> Result<(gix::index::File, ObjectId)> {
        let mut index: gix::index::File = (**self.repo.index_or_empty().ctx("Could not read the project's file list (the index).")?).clone();
        let (mut pipeline, _) = self.repo.filter_pipeline(None).ctx("Could not prepare the files to be saved.")?;
        let workdir = self.folder().to_owned();
        let mut removed: Vec<BString> = Vec::new();
        let mut new_entries = Vec::new();
        for change in changes {
            if only.is_some_and(|o| !o.iter().any(|p| p == &change.path)) {
                continue;
            }
            let rela: &BStr = change.path.as_bytes().as_bstr();
            if change.state == FileState::Deleted {
                removed.push(rela.to_owned());
                continue;
            }
            // the file, converted to its stored form (no drivers: see
            // neutralize_programs), written as a blob
            let Some((id, kind, _)) = pipeline.worktree_file_to_object(rela, &index).ctx(&format!("Could not read {}.", change.path))? else {
                continue;
            };
            let meta = gix::index::fs::Metadata::from_path_no_follow(&workdir.join(&change.path)).ctx(&format!("Could not read {}.", change.path))?;
            let stat = gix::index::entry::Stat::from_fs(&meta).ctx(&format!("Could not read {}.", change.path))?;
            let mode = match kind {
                gix::objs::tree::EntryKind::BlobExecutable => gix::index::entry::Mode::FILE_EXECUTABLE,
                gix::objs::tree::EntryKind::Link => gix::index::entry::Mode::SYMLINK,
                _ => gix::index::entry::Mode::FILE,
            };
            if let Some(entry) = index.entry_mut_by_path_and_stage(rela, gix::index::entry::Stage::Unconflicted) {
                entry.id = id;
                entry.stat = stat;
                entry.mode = mode;
            } else {
                // pushed after the loop: lookups need sorted entries
                new_entries.push((stat, id, mode, rela.to_owned()));
            }
        }
        if !new_entries.is_empty() {
            for (stat, id, mode, rela) in new_entries {
                index.dangerously_push_entry(stat, id, gix::index::entry::Flags::empty(), mode, rela.as_bstr());
            }
            index.sort_entries();
        }
        if !removed.is_empty() {
            index.remove_entries(|_, path, _| removed.iter().any(|r| r.as_bstr() == path));
        }
        let tree = self.tree_of_index(&index)?;
        Ok((index, tree))
    }

    /// The tree of an index's entries (written).
    fn tree_of_index(&self, index: &gix::index::File) -> Result<ObjectId> {
        let empty = self.repo.empty_tree().id;
        let mut editor = self.repo.edit_tree(empty).ctx("Could not build the version's list of files.")?;
        for entry in index.entries() {
            let kind = match entry.mode {
                gix::index::entry::Mode::FILE_EXECUTABLE => gix::objs::tree::EntryKind::BlobExecutable,
                gix::index::entry::Mode::SYMLINK => gix::objs::tree::EntryKind::Link,
                gix::index::entry::Mode::COMMIT => gix::objs::tree::EntryKind::Commit,
                _ => gix::objs::tree::EntryKind::Blob,
            };
            editor.upsert(entry.path(index), kind, entry.id).ctx("Could not build the version's list of files.")?;
        }
        Ok(editor.write().ctx("Could not write the version's list of files.")?.detach())
    }

    /// The working folder as it is now — saved and unsaved changes, new
    /// files — as a tree (its files written as objects; nothing else
    /// changes).
    pub(crate) fn working_tree(&self) -> Result<ObjectId> {
        let changes = self.changes()?;
        Ok(self.staged(&changes, None)?.1)
    }

    /// The lines that look like secrets in what a save would include:
    /// the added lines of the changed and new text files (all of them, or
    /// those in `only`).
    pub fn check_secrets(&self, only: Option<&[String]>) -> Result<Vec<SecretWarning>> {
        let tree = self.head_tree_id()?;
        let mut out = Vec::new();
        for change in self.changes()? {
            if change.state == FileState::Deleted || only.is_some_and(|o| !o.iter().any(|p| p == &change.path)) {
                continue;
            }
            let Ok(new) = std::fs::read(self.folder().join(&change.path)) else { continue };
            if new.len() > secrets::MAX_SCANNED || new.iter().take(8000).any(|b| *b == 0) {
                continue;
            }
            let old = match (tree, change.state) {
                (Some(t), FileState::Changed) => self.blob_at(t, &change.path)?.unwrap_or_default(),
                _ => Vec::new(),
            };
            out.extend(secrets::scan_added(&change.path, &old, &new));
        }
        Ok(out)
    }

    /// **Save a version**: every changed, new and deleted file, or those
    /// `options.files` names (Git's add + commit). Lines that look like
    /// secrets stop it ([`ErrorKind::SecretsFound`], each line in
    /// [`Error::secrets`]) unless `options.allow_secrets`. Returns the new
    /// version's id.
    pub fn save_version(&mut self, message: &str, who: &Person) -> Result<String> {
        self.save_version_with(message, who, &SaveOptions::default())
    }

    /// [`Versions::save_version`] with options.
    pub fn save_version_with(&mut self, message: &str, who: &Person, options: &SaveOptions) -> Result<String> {
        self.writable()?;
        let changes = self.changes()?;
        let head = self.head_id()?;
        let only = options.files.as_deref();
        let chosen = changes.iter().filter(|c| only.is_none_or(|o| o.iter().any(|p| p == &c.path))).count();
        if chosen == 0 {
            return fail(ErrorKind::NothingToSave, "Nothing changed since the last version, so there is nothing to save.");
        }
        if message.trim().is_empty() {
            return fail(ErrorKind::InvalidName, "Say in a few words what this version changes (the message can't be empty).");
        }
        if !options.allow_secrets {
            let found = self.check_secrets(only)?;
            if !found.is_empty() {
                let first = &found[0];
                let more = if found.len() > 1 { format!(" ({} more lines too)", found.len() - 1) } else { String::new() };
                return Err(Error {
                    kind: ErrorKind::SecretsFound,
                    message: format!(
                        "{}, line {}, looks like {}{more}. Versions are often shared, and a secret saved in one stays in the timeline. Remove it before saving, or save anyway if it isn't a real one.",
                        first.path, first.line, first.what
                    ),
                    detail: String::new(),
                    secrets: found,
                });
            }
        }
        let (mut index, tree) = self.staged(&changes, only)?;
        index.write(Default::default()).ctx("Could not write the project's file list (the index).")?;
        let id = self.commit_on_head(message, tree, head, who)?;
        let branch = self.head_ref()?;
        self.record(history::Entry::new("save", format!("Saved the version \u{201C}{}\u{201D}", first_line(message))).moved(&branch, head, Some(id)).with_after(tree))?;
        Ok(id.to_string())
    }

    /// A new commit on HEAD's branch (moved with a compare-and-swap).
    fn commit_on_head(&self, message: &str, tree: ObjectId, parent: Option<ObjectId>, who: &Person) -> Result<ObjectId> {
        let sig = who.signature();
        let mut buf = gix::date::parse::TimeBuf::default();
        let mut buf2 = gix::date::parse::TimeBuf::default();
        Ok(self.repo.commit_as(sig.to_ref(&mut buf), sig.to_ref(&mut buf2), "HEAD", message, tree, parent).ctx("Could not save the version.")?.detach())
    }

    /// The **timeline** of the current experiment, newest first.
    pub fn timeline(&self, limit: usize) -> Result<Vec<Version>> {
        let Some(head) = self.head_id()? else {
            return Ok(Vec::new());
        };
        self.versions_from(head, limit)
    }

    fn versions_from(&self, tip: ObjectId, limit: usize) -> Result<Vec<Version>> {
        let walk = self.repo.rev_walk([tip]).all().ctx("Could not read the timeline.")?;
        let mut out = Vec::new();
        for info in walk.take(limit) {
            let info = info.ctx("Could not read the timeline.")?;
            out.push(self.version(info.id)?);
        }
        Ok(out)
    }

    fn version(&self, id: ObjectId) -> Result<Version> {
        let commit = self.repo.find_commit(id).ctx("Could not read a version.")?;
        let author = commit.author().ctx("Could not read who saved a version.")?;
        Ok(Version {
            id: id.to_string(),
            message: commit.message_raw_sloppy().to_str_lossy().trim_end().to_string(),
            author: author.name.to_string(),
            seconds: commit.time().ctx("Could not read when a version was saved.")?.seconds,
            parents: commit.parent_ids().count(),
        })
    }

    /// The bytes of `path` in `tree` (`None`: not there).
    fn blob_at(&self, tree: ObjectId, path: &str) -> Result<Option<Vec<u8>>> {
        let tree = self.repo.find_tree(tree).ctx("Could not read a version.")?;
        match tree.lookup_entry_by_path(path).ctx("Could not read a version.")? {
            Some(entry) if !entry.mode().is_tree() => Ok(Some(self.repo.find_object(entry.object_id()).ctx(&format!("Could not read {path}."))?.detach().data)),
            _ => Ok(None),
        }
    }

    /// `path` as it is in `version` (any version id, experiment name,
    /// `HEAD`, a checkpoint's or a safety point's id); `None` when the
    /// version has no such file. What a diff view shows as the left side.
    pub fn file_at(&self, path: &str, version: &str) -> Result<Option<Vec<u8>>> {
        let tree = self.tree_of(version)?;
        self.blob_at(tree, path)
    }

    /// **What changed?** since the last version: the working files against
    /// the last version, with hunks.
    pub fn what_changed(&self) -> Result<Vec<FileDiff>> {
        let tree = self.head_tree_id()?;
        let index = self.repo.index_or_empty().ctx("Could not read the project's file list (the index).")?;
        let (mut pipeline, _) = self.repo.filter_pipeline(None).ctx("Could not prepare the files to be compared.")?;
        let mut out = Vec::new();
        for change in self.changes()? {
            let old = match (tree, change.state) {
                (Some(t), FileState::Changed | FileState::Deleted) => self.blob_at(t, &change.path)?.unwrap_or_default(),
                _ => Vec::new(),
            };
            let new = if change.state == FileState::Deleted {
                Vec::new()
            } else {
                // the stored form of the working file (line endings as stored)
                let path = self.folder().join(&change.path);
                let file = std::fs::File::open(&path).ctx(&format!("Could not read {}.", change.path))?;
                let mut data = Vec::new();
                use std::io::Read;
                pipeline
                    .convert_to_git(file, Path::new(&change.path), &index)
                    .ctx(&format!("Could not read {}.", change.path))?
                    .read_to_end(&mut data)
                    .ctx(&format!("Could not read {}.", change.path))?;
                data
            };
            out.push(file_diff(change.path, change.state, &old, &new));
        }
        Ok(out)
    }

    /// What changed between two versions (any of [`Versions::file_at`]'s
    /// names), with hunks, files sorted by path.
    pub fn compare_versions(&self, old: &str, new: &str) -> Result<Vec<FileDiff>> {
        let (a, b) = (self.tree_of(old)?, self.tree_of(new)?);
        self.compare_trees(a, b)
    }

    fn compare_trees(&self, a: ObjectId, b: ObjectId) -> Result<Vec<FileDiff>> {
        let mut out = Vec::new();
        for (path, old, new) in checkout::tree_changes(&self.repo, Some(a), b)? {
            let data = |id: Option<(ObjectId, gix::objs::tree::EntryMode)>| -> Result<Vec<u8>> {
                match id {
                    Some((id, _)) => Ok(self.repo.find_object(id).ctx("Could not read a file.")?.detach().data),
                    None => Ok(Vec::new()),
                }
            };
            let state = match (old, new) {
                (None, _) => FileState::New,
                (_, None) => FileState::Deleted,
                _ => FileState::Changed,
            };
            out.push(file_diff(path.to_string(), state, &data(old)?, &data(new)?));
        }
        Ok(out)
    }

    fn resolve(&self, spec: &str) -> Result<ObjectId> {
        match self.repo.rev_parse_single(spec) {
            Ok(id) => Ok(id.detach()),
            Err(e) => Err(Error { kind: ErrorKind::NotFound, message: format!("There is no version \u{201C}{spec}\u{201D}."), detail: e.to_string(), secrets: Vec::new() }),
        }
    }

    fn tree_of(&self, spec: &str) -> Result<ObjectId> {
        let id = self.resolve(spec)?;
        Ok(self.repo.find_object(id).ctx("Could not read a version.")?.peel_to_tree().ctx(&format!("\u{201C}{spec}\u{201D} isn't a version."))?.id)
    }

    /// **Who wrote this line?** for every line of `path` at the current
    /// version.
    pub fn who_wrote(&self, path: &str) -> Result<Vec<BlameLine>> {
        let Some(head) = self.head_id()? else {
            return Ok(Vec::new());
        };
        let outcome = self.repo.blame_file(path.as_bytes().as_bstr(), head, Default::default()).ctx(&format!("Could not find who wrote {path}."))?;
        let mut out = Vec::new();
        for (entry, lines) in outcome.entries_with_lines() {
            let commit = self.repo.find_commit(entry.commit_id).ctx("Could not read a version.")?;
            let author = commit.author().ctx("Could not read who saved a version.")?.name.to_string();
            for (i, text) in lines.into_iter().enumerate() {
                out.push(BlameLine {
                    line: entry.start_in_blamed_file + i as u32 + 1,
                    version: entry.commit_id.to_string(),
                    author: author.clone(),
                    text: text.to_str_lossy().trim_end_matches(['\n', '\r']).to_string(),
                });
            }
        }
        Ok(out)
    }

    // ------------------------------------------------- go back, restore --

    /// **Go back to this version**: a new version whose files are as they
    /// were in `version` (never a destructive reset: the versions after it
    /// stay on the timeline). Needs no unsaved changes; a safety point
    /// first. Returns the new version's id.
    pub fn go_back(&mut self, version: &str, who: &Person) -> Result<String> {
        self.writable()?;
        self.ensure_saved()?;
        let target = self.resolve(version)?;
        let tree = self.tree_of(version)?;
        let head = self.head_id()?;
        if self.head_tree_id()? == Some(tree) {
            return fail(ErrorKind::NothingToSave, "The project is already as it was in that version.");
        }
        let v = self.version(target)?;
        let words = format!("Went back to \u{201C}{}\u{201D} ({})", first_line(&v.message), v.short_id());
        let safety = self.safety_point(&words)?;
        let id = self.commit_on_head(&format!("Go back to \u{201C}{}\u{201D} ({})", first_line(&v.message), v.short_id()), tree, head, who)?;
        checkout::switch_files(self, tree)?;
        let branch = self.head_ref()?;
        self.record(history::Entry::new("go-back", words).moved(&branch, head, Some(id)).with_before(safety).with_after(tree))?;
        Ok(id.to_string())
    }

    /// **Restore a file** as it was in `version` (any of
    /// [`Versions::file_at`]'s names): the working file is rewritten (or
    /// removed, when the version didn't have it), as an unsaved change
    /// to save or undo. A safety point first.
    pub fn restore_file(&mut self, path: &str, version: &str) -> Result<()> {
        self.writable()?;
        let tree = self.tree_of(version)?;
        let words = format!("Restored {path} from version {}", &self.resolve(version)?.to_string()[..7]);
        let safety = self.safety_point(&words)?;
        checkout::restore_paths(self, tree, &[path.to_string()])?;
        let after = self.working_tree()?;
        self.record(history::Entry::new("restore-file", words).with_before(safety).with_after(after))
    }

    // ---------------------------------------------------- experiments --

    /// **Try something new**: a new experiment (branch) from the current
    /// version, switched to (no file changes).
    pub fn start_experiment(&mut self, name: &str) -> Result<()> {
        self.writable()?;
        let Some(head) = self.head_id()? else {
            return fail(ErrorKind::NothingToSave, "Save a first version before starting an experiment.");
        };
        let full = format!("refs/heads/{name}");
        if gix::validate::reference::branch_name(full.as_bytes().as_bstr()).is_err() || name.trim().is_empty() || name.contains(' ') {
            return fail(ErrorKind::InvalidName, format!("\u{201C}{name}\u{201D} can't be an experiment's name: use letters, digits, - and _ (no spaces)."));
        }
        if self.repo.try_find_reference(full.as_str()).ctx("Could not read the experiments.")?.is_some() {
            return fail(ErrorKind::InvalidName, format!("There is already an experiment called \u{201C}{name}\u{201D}."));
        }
        let old_head = self.head_ref()?;
        edit_ref(&self.repo, &full, Target::Object(head), PreviousValue::MustNotExist, format!("branch: Created from HEAD (experiment {name})"), &self.person).map_err(|e| Error { message: "Could not start the experiment.".into(), ..e })?;
        self.set_head(&full)?;
        let after = self.working_tree()?;
        self.record(history::Entry::new("start-experiment", format!("Started the experiment \u{201C}{name}\u{201D}")).moved(&full, None, Some(head)).head(&old_head, &full).with_after(after))
    }

    /// Switch to another experiment: only the files that differ between
    /// the two are rewritten. Refused while there are unsaved changes
    /// (nothing is ever thrown away silently); a safety point first.
    pub fn switch_to(&mut self, name: &str) -> Result<()> {
        self.writable()?;
        self.ensure_saved()?;
        let full = format!("refs/heads/{name}");
        let current = self.head_ref()?;
        if current == full {
            return Ok(());
        }
        let tip = match self.repo.try_find_reference(full.as_str()).ctx("Could not read the experiments.")? {
            Some(mut r) => r.peel_to_id().ctx("Could not read the experiment.")?.detach(),
            None => return fail(ErrorKind::NotFound, format!("There is no experiment called \u{201C}{name}\u{201D}.")),
        };
        let words = format!("Switched to the experiment \u{201C}{name}\u{201D}");
        let safety = self.safety_point(&words)?;
        let tree = self.tree_id_of_commit(tip)?;
        checkout::switch_files(self, tree)?;
        self.set_head(&full)?;
        self.record(history::Entry::new("switch", words).head(&current, &full).with_before(safety).with_after(tree))
    }

    fn ensure_saved(&self) -> Result<()> {
        let changes = self.changes()?;
        if !changes.is_empty() {
            let names: Vec<&str> = changes.iter().take(3).map(|c| c.path.as_str()).collect();
            let more = if changes.len() > 3 { format!(" and {} more", changes.len() - 3) } else { String::new() };
            return fail(ErrorKind::UnsavedChanges, format!("You have changes that aren't in a version yet ({}{more}). Save a version first, then try again.", names.join(", ")));
        }
        Ok(())
    }

    fn set_head(&mut self, full: &str) -> Result<()> {
        let target = Target::Symbolic(full.try_into().ctx("Not a valid experiment name.")?);
        edit_ref(&self.repo, "HEAD", target, PreviousValue::Any, format!("checkout: moving to {full}"), &self.person).map_err(|e| Error { message: "Could not switch experiments.".into(), ..e })
    }

    /// **Bring this experiment into the current one**: a fast-forward when
    /// possible, else a three-way merge saved as a new version with two
    /// parents. With conflicts, nothing is changed and the paths are
    /// returned (the Merge tab, lane G4, resolves them). A safety point
    /// first.
    pub fn bring_into_current(&mut self, experiment: &str, who: &Person) -> Result<BringInto> {
        self.writable()?;
        let theirs = self.resolve(&format!("refs/heads/{experiment}")).map_err(|_| Error::new(ErrorKind::NotFound, format!("There is no experiment called \u{201C}{experiment}\u{201D}.")))?;
        self.bring_commit(theirs, experiment, &format!("Brought \u{201C}{experiment}\u{201D} in"), &format!("Bring {experiment} into"), who)
    }

    /// Brings commit `theirs` into the current experiment (see
    /// [`Versions::bring_into_current`]).
    fn bring_commit(&mut self, theirs: ObjectId, label: &str, words: &str, message: &str, who: &Person) -> Result<BringInto> {
        self.ensure_saved()?;
        let current = self.current_experiment()?;
        let branch = self.head_ref()?;
        let Some(ours) = self.head_id()? else {
            return fail(ErrorKind::NothingToSave, "Save a first version first.");
        };
        if ours == theirs {
            return Ok(BringInto::AlreadyUpToDate);
        }
        let Some(base) = self.repo.merge_base(ours, theirs).ok().flatten().map(|b| b.detach()) else {
            return fail(ErrorKind::Conflicts, "These versions have nothing in common (they were started separately), so they can't be joined.");
        };
        if base == theirs {
            return Ok(BringInto::AlreadyUpToDate);
        }
        if base == ours {
            // fast-forward: a safety point, the files, then the branch (CAS)
            let tree = self.tree_id_of_commit(theirs)?;
            let safety = self.safety_point(words)?;
            checkout::switch_files(self, tree)?;
            edit_ref(&self.repo, &branch, Target::Object(theirs), PreviousValue::MustExistAndMatch(Target::Object(ours)), format!("merge {label}: Fast-forward"), who)?;
            self.record(history::Entry::new("bring-into", words.to_string()).moved(&branch, Some(ours), Some(theirs)).with_before(safety).with_after(tree))?;
            return Ok(BringInto::FastForward(theirs.to_string()));
        }
        let labels = gix::merge::blob::builtin_driver::text::Labels { ancestor: Some("base".into()), current: Some(current.as_bytes().as_bstr()), other: Some(label.as_bytes().as_bstr()) };
        let mut outcome = self.repo.merge_commits(ours, theirs, labels, gix::merge::commit::Options::default()).ctx("Could not join the two experiments.")?;
        let how = gix::merge::tree::TreatAsUnresolved::git();
        if outcome.tree_merge.has_unresolved_conflicts(how) {
            let mut paths: Vec<String> = outcome.tree_merge.conflicts.iter().filter(|c| c.is_unresolved(how)).map(|c| c.ours.location().to_string()).collect();
            paths.sort();
            paths.dedup();
            return Ok(BringInto::Conflicts(paths));
        }
        let tree = outcome.tree_merge.tree.write().ctx("Could not write the joined version.")?.detach();
        let safety = self.safety_point(words)?;
        checkout::switch_files(self, tree)?;
        let sig = who.signature();
        let mut buf = gix::date::parse::TimeBuf::default();
        let mut buf2 = gix::date::parse::TimeBuf::default();
        let id = self.repo.commit_as(sig.to_ref(&mut buf), sig.to_ref(&mut buf2), "HEAD", format!("{message} {current}"), tree, [ours, theirs]).ctx("Could not save the joined version.")?.detach();
        self.record(history::Entry::new("bring-into", words.to_string()).moved(&branch, Some(ours), Some(id)).with_before(safety).with_after(tree))?;
        Ok(BringInto::Merged(id.to_string()))
    }
}

/// A message's first line (what the timeline shows).
fn first_line(message: &str) -> &str {
    message.lines().next().unwrap_or("").trim()
}

/// Update one reference with a compare-and-swap (`expected`), its reflog
/// written as `who` (repositories are opened without the user's git
/// identity). A failed swap is [`ErrorKind::ChangedMeanwhile`].
fn edit_ref(repo: &gix::Repository, name: &str, new: Target, expected: PreviousValue, message: String, who: &Person) -> Result<()> {
    let sig = who.signature();
    let mut buf = gix::date::parse::TimeBuf::default();
    let edit = RefEdit {
        change: Change::Update { log: LogChange { mode: RefLog::AndReference, force_create_reflog: false, message: message.into() }, expected, new },
        name: name.try_into().ctx("Not a valid reference name.")?,
        deref: false,
    };
    repo.edit_references_as([edit], Some(sig.to_ref(&mut buf))).map_err(|e| Error {
        kind: ErrorKind::ChangedMeanwhile,
        message: "The project changed meanwhile (another program, or a newer version), so nothing was changed. Try again.".into(),
        detail: e.to_string(),
        secrets: Vec::new(),
    })?;
    Ok(())
}

/// Delete one reference if it still points at `expected`.
fn delete_ref(repo: &gix::Repository, name: &str, expected: Target, who: &Person) -> Result<()> {
    let sig = who.signature();
    let mut buf = gix::date::parse::TimeBuf::default();
    let edit = RefEdit { change: Change::Delete { expected: PreviousValue::MustExistAndMatch(expected), log: RefLog::AndReference }, name: name.try_into().ctx("Not a valid reference name.")?, deref: false };
    repo.edit_references_as([edit], Some(sig.to_ref(&mut buf))).map_err(|e| Error {
        kind: ErrorKind::ChangedMeanwhile,
        message: "The project changed meanwhile (another program, or a newer version), so nothing was changed. Try again.".into(),
        detail: e.to_string(),
        secrets: Vec::new(),
    })?;
    Ok(())
}

/// Remove everything from the repository's in-memory configuration that
/// would make gitoxide start a program (the file on disk is untouched).
fn neutralize_programs(repo: &mut gix::Repository) -> Result<()> {
    let mut config = repo.config_snapshot_mut();
    let mut remove_ids = Vec::new();
    let mut strip: Vec<(gix::config::file::SectionId, &'static [&'static str])> = Vec::new();
    for (section, id) in config.sections_and_ids() {
        let name = section.header().name().to_str_lossy().to_ascii_lowercase();
        let sub = section.header().subsection_name().is_some();
        match name.as_str() {
            // filter.<driver>.clean / smudge / process
            "filter" => remove_ids.push(id),
            // credential helpers
            "credential" => remove_ids.push(id),
            // diff.<driver>.textconv / command; merge.<driver>.driver
            "diff" | "merge" if sub => remove_ids.push(id),
            "gpg" => remove_ids.push(id),
            "core" => strip.push((id, &["fsmonitor", "sshcommand", "askpass", "gitproxy", "hookspath", "editor", "pager"])),
            "remote" => strip.push((id, &["uploadpack", "receivepack", "vcs"])),
            "ssh" => strip.push((id, &["variant"])),
            _ => {}
        }
    }
    for id in remove_ids {
        config.remove_section_by_id(id);
    }
    for (id, keys) in strip {
        if let Some(mut section) = config.section_mut_by_id(id) {
            let present: Vec<String> = section.value_names().map(|n| n.to_string()).filter(|n| keys.contains(&n.to_ascii_lowercase().as_str())).collect();
            for key in present {
                while section.remove(&key).is_some() {}
            }
        }
    }
    config.commit().ctx("Could not apply the safe settings to this project.")?;
    Ok(())
}

fn is_binary(data: &[u8]) -> bool {
    data.iter().take(8000).any(|b| *b == 0)
}

fn file_diff(path: String, state: FileState, old: &[u8], new: &[u8]) -> FileDiff {
    if is_binary(old) || is_binary(new) {
        return FileDiff { path, state, binary: true, hunks: Vec::new() };
    }
    FileDiff { path, state, binary: false, hunks: hunks(old, new, 3) }
}

/// Line hunks between `old` and `new` with `context` lines around each
/// change: RapidR's own diff (`rapidr-diff`: Myers' algorithm, runs of
/// added or removed lines slid down to sit with their block).
pub fn hunks(old: &[u8], new: &[u8], context: u32) -> Vec<Hunk> {
    use rapidr_diff::text::{self, HunkLine};
    let (a, b) = (text::split_lines(old), text::split_lines(new));
    let changes = text::changes(&a, &b);
    let as_text = |l: &[u8]| l.to_str_lossy().trim_end_matches(['\n', '\r']).to_string();
    text::hunks(&changes, a.len(), b.len(), context as usize)
        .into_iter()
        .map(|h| Hunk {
            old_start: h.old_start as u32,
            old_len: h.old_len as u32,
            new_start: h.new_start as u32,
            new_len: h.new_len as u32,
            lines: h
                .lines
                .iter()
                .map(|l| match *l {
                    HunkLine::Same { old, .. } => Line::Same(as_text(a[old])),
                    HunkLine::Removed(i) => Line::Removed(as_text(a[i])),
                    HunkLine::Added(j) => Line::Added(as_text(b[j])),
                })
                .collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests;

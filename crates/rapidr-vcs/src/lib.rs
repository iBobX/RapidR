//! Versions (Git) for RapidR Studio, on gitoxide — lane G0's spike
//! (docs/versions-plan.md, findings in docs/versions-g0.md).
//!
//! A tiny plain-words API over `gix` that proves the engine end to end:
//! start versioning a folder, save a version, what changed (hunks), the
//! timeline, who wrote each line, experiments (branches), bring an
//! experiment into the current one (fast-forward or a real three-way
//! merge, conflicts reported without touching anything), and share with a
//! bare repository in a local folder.
//!
//! Safety, decided in versions-plan §5 and enforced here:
//! - repositories are opened *isolated*: no system or global git
//!   configuration, no environment, no includes; who saves a version is
//!   given by Studio, not read from `~/.gitconfig`;
//! - nothing configured can run a program: filter drivers, diff
//!   textconv/command drivers, merge drivers, credential helpers,
//!   `core.fsmonitor`, `core.sshCommand`, `remote.*.uploadpack`… are
//!   removed from the repository's *in-memory* configuration when it is
//!   opened (the file on disk is never rewritten);
//! - gitoxide never runs hooks (it has no hook support at all; a test
//!   pins that);
//! - the `git` command is never run: gitoxide's own local transport
//!   (`file://` and plain paths) spawns `git-upload-pack`, so sharing with
//!   a local folder is done in-process here, object by object, with a
//!   compare-and-swap update of the remote branch.
//!
//! Not in the spike (G1): safety points, the action history with undo,
//! checkpoints, hunk staging, stash, cherry-pick/revert, tags, and the
//! network (gitoxide has fetch/clone over HTTPS and SSH but no push).

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use gix::bstr::{BStr, BString, ByteSlice};
use gix::refs::transaction::{Change, LogChange, PreviousValue, RefEdit, RefLog};
use gix::refs::Target;
use gix::ObjectId;

/// What went wrong, in words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

trait Ctx<T> {
    fn ctx(self, what: &str) -> Result<T>;
}

impl<T, E: std::fmt::Display> Ctx<T> for std::result::Result<T, E> {
    fn ctx(self, what: &str) -> Result<T> {
        self.map_err(|e| Error(format!("{what}: {e}")))
    }
}

fn err<T>(msg: impl Into<String>) -> Result<T> {
    Err(Error(msg.into()))
}

/// Who saves a version (Studio's Settings › Versions).
#[derive(Debug, Clone)]
pub struct Person {
    pub name: String,
    pub email: String,
}

impl Person {
    pub fn new(name: &str, email: &str) -> Person {
        Person {
            name: name.into(),
            email: email.into(),
        }
    }

    fn signature(&self) -> gix::actor::Signature {
        gix::actor::Signature {
            name: self.name.as_str().into(),
            email: self.email.as_str().into(),
            time: gix::date::Time::now_local_or_utc(),
        }
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
        format!(
            "@@ -{},{} +{},{} @@",
            self.old_start, self.old_len, self.new_start, self.new_len
        )
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

/// What sharing with a folder did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Share {
    /// The folder already had this version.
    UpToDate,
    /// The folder's experiment moved from `from` (None: new there) to `to`.
    Shared {
        from: Option<String>,
        to: String,
        objects: usize,
    },
    /// The folder has versions this project doesn't have: get them first.
    NeedsGetLatest,
}

/// The options every repository is opened with: only its own
/// configuration, never the system's, the user's, the environment's or
/// included files.
fn open_options() -> gix::open::Options {
    gix::open::Options::isolated()
}

/// Create an empty shared folder (a bare repository) to share versions
/// with, e.g. on a USB stick or a shared drive.
pub fn create_shared_folder(path: &Path) -> Result<()> {
    gix::ThreadSafeRepository::init_opts(
        path,
        gix::create::Kind::Bare,
        gix::create::Options::default(),
        open_options(),
    )
    .ctx("Could not create the shared folder")?;
    Ok(())
}

/// A project folder under versions.
pub struct Versions {
    repo: gix::Repository,
    /// Who is working (reflogs of switches and experiments).
    person: Person,
}

impl Versions {
    /// **Start versioning this project**: `git init` with `main` (an
    /// existing repository is opened instead). `person` is who works on
    /// it (Studio's Settings > Versions).
    pub fn start(dir: &Path, person: &Person) -> Result<Versions> {
        if dir.join(".git").exists() {
            return Versions::open(dir, person);
        }
        let repo = gix::ThreadSafeRepository::init_opts(
            dir,
            gix::create::Kind::WithWorktree,
            gix::create::Options::default(),
            open_options(),
        )
        .ctx("Could not start versioning this folder")?
        .to_thread_local();
        Versions::from_repo(repo, person)
    }

    /// Open the project at `dir` (its own folder only: a repository in a
    /// parent folder is not used).
    pub fn open(dir: &Path, person: &Person) -> Result<Versions> {
        let repo = gix::open_opts(dir, open_options())
            .ctx("Could not open the versions of this folder")?;
        Versions::from_repo(repo, person)
    }

    fn from_repo(mut repo: gix::Repository, person: &Person) -> Result<Versions> {
        if repo.workdir().is_none() {
            return err("This is a bare repository (no project files)");
        }
        neutralize_programs(&mut repo)?;
        Ok(Versions {
            repo,
            person: person.clone(),
        })
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
        match self.repo.head_name().ctx("Could not read HEAD")? {
            Some(name) => Ok(name.shorten().to_string()),
            None => err("Not on an experiment (detached)"),
        }
    }

    fn head_id(&self) -> Result<Option<ObjectId>> {
        let mut head = self.repo.head().ctx("Could not read HEAD")?;
        Ok(head
            .try_peel_to_id()
            .ctx("Could not read HEAD's version")?
            .map(|id| id.detach()))
    }

    /// Files that differ from the last saved version: changed, deleted,
    /// and new (untracked, not ignored) files.
    pub fn changes(&self) -> Result<Vec<ChangedFile>> {
        use gix::status::index_worktree::Item;
        use gix::status::plumbing::index_as_worktree::{Change as WtChange, EntryStatus};

        let iter = self
            .repo
            .status(gix::progress::Discard)
            .ctx("Could not check for changes")?
            .untracked_files(gix::status::UntrackedFiles::Files)
            .index_worktree_rewrites(None)
            .into_index_worktree_iter(Vec::<BString>::new())
            .ctx("Could not check for changes")?;
        let mut out = Vec::new();
        for item in iter {
            let item = item.ctx("Could not check for changes")?;
            match item {
                Item::Modification {
                    rela_path, status, ..
                } => {
                    let state = match status {
                        EntryStatus::Change(WtChange::Removed) => FileState::Deleted,
                        EntryStatus::Change(_)
                        | EntryStatus::Conflict { .. }
                        | EntryStatus::IntentToAdd => FileState::Changed,
                        EntryStatus::NeedsUpdate(_) => continue,
                    };
                    out.push(ChangedFile {
                        path: rela_path.to_string(),
                        state,
                    });
                }
                Item::DirectoryContents { entry, .. } => {
                    if entry.status == gix::dir::entry::Status::Untracked {
                        out.push(ChangedFile {
                            path: entry.rela_path.to_string(),
                            state: FileState::New,
                        });
                    }
                }
                Item::Rewrite { .. } => {}
            }
        }
        out.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(out)
    }

    /// **Save a version**: every changed, new and deleted file (Git's
    /// add -A + commit). Returns the new version's id.
    pub fn save_version(&mut self, message: &str, who: &Person) -> Result<String> {
        let changes = self.changes()?;
        let head = self.head_id()?;
        if changes.is_empty() && head.is_some() {
            return err("Nothing changed since the last version");
        }
        let mut index: gix::index::File =
            (**self.repo.index_or_empty().ctx("Could not read the index")?).clone();
        let (mut pipeline, _) = self
            .repo
            .filter_pipeline(None)
            .ctx("Could not prepare file conversion")?;
        let workdir = self.folder().to_owned();
        let mut removed: Vec<BString> = Vec::new();
        let mut new_entries = Vec::new();
        for change in &changes {
            let rela: &BStr = change.path.as_bytes().as_bstr();
            if change.state == FileState::Deleted {
                removed.push(rela.to_owned());
                continue;
            }
            // the file, converted to its stored form (no drivers: see
            // neutralize_programs), written as a blob
            let Some((id, kind, _)) = pipeline
                .worktree_file_to_object(rela, &index)
                .ctx("Could not read a changed file")?
            else {
                continue;
            };
            let meta = gix::index::fs::Metadata::from_path_no_follow(&workdir.join(&change.path))
                .ctx("Could not read a file's details")?;
            let stat =
                gix::index::entry::Stat::from_fs(&meta).ctx("Could not read a file's time")?;
            let mode = match kind {
                gix::objs::tree::EntryKind::BlobExecutable => {
                    gix::index::entry::Mode::FILE_EXECUTABLE
                }
                gix::objs::tree::EntryKind::Link => gix::index::entry::Mode::SYMLINK,
                _ => gix::index::entry::Mode::FILE,
            };
            if let Some(entry) =
                index.entry_mut_by_path_and_stage(rela, gix::index::entry::Stage::Unconflicted)
            {
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
                index.dangerously_push_entry(
                    stat,
                    id,
                    gix::index::entry::Flags::empty(),
                    mode,
                    rela.as_bstr(),
                );
            }
            index.sort_entries();
        }
        if !removed.is_empty() {
            index.remove_entries(|_, path, _| removed.iter().any(|r| r.as_bstr() == path));
        }

        // the tree, from the index's entries
        let empty = self.repo.empty_tree().id;
        let mut editor = self
            .repo
            .edit_tree(empty)
            .ctx("Could not build the version's tree")?;
        for entry in index.entries() {
            let kind = match entry.mode {
                gix::index::entry::Mode::FILE_EXECUTABLE => {
                    gix::objs::tree::EntryKind::BlobExecutable
                }
                gix::index::entry::Mode::SYMLINK => gix::objs::tree::EntryKind::Link,
                gix::index::entry::Mode::COMMIT => gix::objs::tree::EntryKind::Commit,
                _ => gix::objs::tree::EntryKind::Blob,
            };
            editor
                .upsert(entry.path(&index), kind, entry.id)
                .ctx("Could not build the version's tree")?;
        }
        let tree = editor
            .write()
            .ctx("Could not write the version's tree")?
            .detach();
        index
            .write(Default::default())
            .ctx("Could not write the index")?;

        let sig = who.signature();
        let mut buf = gix::date::parse::TimeBuf::default();
        let mut buf2 = gix::date::parse::TimeBuf::default();
        let id = self
            .repo
            .commit_as(
                sig.to_ref(&mut buf),
                sig.to_ref(&mut buf2),
                "HEAD",
                message,
                tree,
                head,
            )
            .ctx("Could not save the version")?;
        Ok(id.to_string())
    }

    /// The **timeline**, newest first.
    pub fn timeline(&self, limit: usize) -> Result<Vec<Version>> {
        let Some(head) = self.head_id()? else {
            return Ok(Vec::new());
        };
        let walk = self
            .repo
            .rev_walk([head])
            .all()
            .ctx("Could not read the timeline")?;
        let mut out = Vec::new();
        for info in walk.take(limit) {
            let info = info.ctx("Could not read the timeline")?;
            let commit = info.object().ctx("Could not read a version")?;
            let author = commit.author().ctx("Could not read a version's author")?;
            out.push(Version {
                id: info.id.to_string(),
                message: commit
                    .message_raw_sloppy()
                    .to_str_lossy()
                    .trim_end()
                    .to_string(),
                author: author.name.to_string(),
                seconds: commit
                    .time()
                    .ctx("Could not read a version's time")?
                    .seconds,
                parents: commit.parent_ids().count(),
            });
        }
        Ok(out)
    }

    /// **What changed?** since the last version: the working files against
    /// the last version, with hunks.
    pub fn what_changed(&self) -> Result<Vec<FileDiff>> {
        let tree = match self.head_id()? {
            Some(_) => Some(
                self.repo
                    .head_tree()
                    .ctx("Could not read the last version")?,
            ),
            None => None,
        };
        let index = self.repo.index_or_empty().ctx("Could not read the index")?;
        let (mut pipeline, _) = self
            .repo
            .filter_pipeline(None)
            .ctx("Could not prepare file conversion")?;
        let mut out = Vec::new();
        for change in self.changes()? {
            let old = match (&tree, change.state) {
                (Some(tree), FileState::Changed | FileState::Deleted) => {
                    match tree
                        .lookup_entry_by_path(&change.path)
                        .ctx("Could not read the last version")?
                    {
                        Some(entry) => {
                            self.repo
                                .find_object(entry.object_id())
                                .ctx("Could not read a file")?
                                .detach()
                                .data
                        }
                        None => Vec::new(),
                    }
                }
                _ => Vec::new(),
            };
            let new = if change.state == FileState::Deleted {
                Vec::new()
            } else {
                // the stored form of the working file (line endings as stored)
                let path = self.folder().join(&change.path);
                let file = std::fs::File::open(&path).ctx("Could not read a changed file")?;
                let mut data = Vec::new();
                use std::io::Read;
                let rela = Path::new(&change.path);
                pipeline
                    .convert_to_git(file, rela, &index)
                    .ctx("Could not read a changed file")?
                    .read_to_end(&mut data)
                    .ctx("Could not read a changed file")?;
                data
            };
            out.push(file_diff(change.path, change.state, &old, &new));
        }
        Ok(out)
    }

    /// What changed between two versions (tree to tree), with hunks.
    pub fn compare_versions(&self, old: &str, new: &str) -> Result<Vec<FileDiff>> {
        let old_tree = self.tree_of(old)?;
        let new_tree = self.tree_of(new)?;
        let changes = self
            .repo
            .diff_tree_to_tree(Some(&old_tree), Some(&new_tree), None)
            .ctx("Could not compare the versions")?;
        let mut out = Vec::new();
        for change in changes {
            use gix::diff::tree_with_rewrites::Change as C;
            // folders appear as their own additions/deletions: files only
            if change.entry_mode().is_tree() {
                continue;
            }
            let (path, state, old_id, new_id) = match change {
                C::Addition { location, id, .. } => (location, FileState::New, None, Some(id)),
                C::Deletion { location, id, .. } => (location, FileState::Deleted, Some(id), None),
                C::Modification {
                    location,
                    previous_id,
                    id,
                    ..
                } => (location, FileState::Changed, Some(previous_id), Some(id)),
                C::Rewrite {
                    location,
                    source_id,
                    id,
                    ..
                } => (location, FileState::Changed, Some(source_id), Some(id)),
            };
            let data = |id: Option<ObjectId>| -> Result<Vec<u8>> {
                match id {
                    Some(id) => Ok(self
                        .repo
                        .find_object(id)
                        .ctx("Could not read a file")?
                        .detach()
                        .data),
                    None => Ok(Vec::new()),
                }
            };
            out.push(file_diff(
                path.to_string(),
                state,
                &data(old_id)?,
                &data(new_id)?,
            ));
        }
        out.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(out)
    }

    fn resolve(&self, spec: &str) -> Result<ObjectId> {
        Ok(self
            .repo
            .rev_parse_single(spec)
            .ctx(&format!("No version '{spec}'"))?
            .detach())
    }

    fn tree_of(&self, spec: &str) -> Result<gix::Tree<'_>> {
        let id = self.resolve(spec)?;
        self.repo
            .find_object(id)
            .ctx("Could not read a version")?
            .peel_to_tree()
            .ctx("Not a version")
    }

    /// **Who wrote this line?** for every line of `path` at the current
    /// version.
    pub fn who_wrote(&self, path: &str) -> Result<Vec<BlameLine>> {
        let Some(head) = self.head_id()? else {
            return Ok(Vec::new());
        };
        let outcome = self
            .repo
            .blame_file(path.as_bytes().as_bstr(), head, Default::default())
            .ctx("Could not find who wrote this file")?;
        let mut out = Vec::new();
        for (entry, lines) in outcome.entries_with_lines() {
            let commit = self
                .repo
                .find_commit(entry.commit_id)
                .ctx("Could not read a version")?;
            let author = commit
                .author()
                .ctx("Could not read a version's author")?
                .name
                .to_string();
            for (i, text) in lines.into_iter().enumerate() {
                out.push(BlameLine {
                    line: entry.start_in_blamed_file + i as u32 + 1,
                    version: entry.commit_id.to_string(),
                    author: author.clone(),
                    text: text
                        .to_str_lossy()
                        .trim_end_matches(['\n', '\r'])
                        .to_string(),
                });
            }
        }
        Ok(out)
    }

    /// **Try something new**: a new experiment (branch) from the current
    /// version, switched to.
    pub fn start_experiment(&mut self, name: &str) -> Result<()> {
        let Some(head) = self.head_id()? else {
            return err("Save a first version before an experiment");
        };
        let full = format!("refs/heads/{name}");
        gix::validate::reference::branch_name(full.as_bytes().as_bstr())
            .ctx("Not a valid experiment name")?;
        edit_ref(
            &self.repo,
            &full,
            Target::Object(head),
            PreviousValue::MustNotExist,
            format!("branch: Created from HEAD (experiment {name})"),
            &self.person,
        )
        .ctx("Could not start the experiment")?;
        self.set_head(&full)
    }

    /// Switch to another experiment. Refused while there are unsaved
    /// changes (nothing is ever thrown away silently).
    pub fn switch_to(&mut self, name: &str) -> Result<()> {
        self.ensure_saved()?;
        let full = format!("refs/heads/{name}");
        let tip = self
            .repo
            .find_reference(full.as_str())
            .ctx(&format!("No experiment '{name}'"))?
            .peel_to_id()
            .ctx("Could not read the experiment")?
            .detach();
        let tree = self
            .repo
            .find_commit(tip)
            .ctx("Could not read a version")?
            .tree_id()
            .ctx("Not a version")?
            .detach();
        self.checkout_tree(tree)?;
        self.set_head(&full)
    }

    fn ensure_saved(&self) -> Result<()> {
        if !self.changes()?.is_empty() {
            return err("There are unsaved changes: save a version or put them aside first");
        }
        Ok(())
    }

    fn set_head(&mut self, full: &str) -> Result<()> {
        let target = Target::Symbolic(full.try_into().ctx("Not a valid experiment name")?);
        edit_ref(
            &self.repo,
            "HEAD",
            target,
            PreviousValue::Any,
            format!("checkout: moving to {full}"),
            &self.person,
        )
        .ctx("Could not switch")
    }

    /// Make the working folder and the index match `tree`: changed files
    /// are rewritten, files only in the old version are removed. Unsaved
    /// changes must have been checked by the caller.
    fn checkout_tree(&mut self, tree: ObjectId) -> Result<()> {
        let workdir = self.folder().to_owned();
        let old_index = self.repo.index_or_empty().ctx("Could not read the index")?;
        let mut new_index = self
            .repo
            .index_from_tree(&tree)
            .ctx("Could not read the version")?;
        let mut opts = self
            .repo
            .checkout_options(gix::worktree::stack::state::attributes::Source::IdMapping)
            .ctx("Could not prepare the checkout")?;
        opts.overwrite_existing = true;
        opts.destination_is_initially_empty = false;
        let objects = self
            .repo
            .objects
            .clone()
            .into_arc()
            .ctx("Could not share the object store")?;
        let outcome = gix::worktree::state::checkout(
            &mut new_index,
            &workdir,
            objects,
            &gix::progress::Discard,
            &gix::progress::Discard,
            &AtomicBool::new(false),
            opts,
        )
        .ctx("Could not update the files")?;
        if !outcome.errors.is_empty() || !outcome.collisions.is_empty() {
            return err(format!(
                "Could not update {} file(s)",
                outcome.errors.len() + outcome.collisions.len()
            ));
        }
        // files the new version doesn't have
        for entry in old_index.entries() {
            let path = entry.path(&old_index);
            if new_index.entry_by_path(path).is_none() {
                let Ok(rela) = gix::path::from_bstr(path) else {
                    continue;
                };
                let file = workdir.join(rela);
                let _ = std::fs::remove_file(&file);
                remove_empty_parents(&workdir, &file);
            }
        }
        new_index
            .write(Default::default())
            .ctx("Could not write the index")?;
        Ok(())
    }

    /// **Bring this experiment into the current one**: a fast-forward when
    /// possible, else a three-way merge saved as a new version with two
    /// parents. With conflicts, nothing is changed and the paths are
    /// returned (the Merge tab, lane G4, resolves them).
    pub fn bring_into_current(&mut self, experiment: &str, who: &Person) -> Result<BringInto> {
        self.ensure_saved()?;
        let current = self.current_experiment()?;
        let Some(ours) = self.head_id()? else {
            return err("Save a first version first");
        };
        let theirs = self.resolve(&format!("refs/heads/{experiment}"))?;
        if ours == theirs {
            return Ok(BringInto::AlreadyUpToDate);
        }
        let Some(base) = self
            .repo
            .merge_base(ours, theirs)
            .ctx("Could not compare the experiments")?
        else {
            return err("These experiments share no version");
        };
        let base = base.detach();
        if base == theirs {
            return Ok(BringInto::AlreadyUpToDate);
        }
        if base == ours {
            // fast-forward: files first, then the experiment's ref (CAS)
            let tree = self
                .repo
                .find_commit(theirs)
                .ctx("Could not read a version")?
                .tree_id()
                .ctx("Not a version")?
                .detach();
            self.checkout_tree(tree)?;
            edit_ref(
                &self.repo,
                &format!("refs/heads/{current}"),
                Target::Object(theirs),
                PreviousValue::MustExistAndMatch(Target::Object(ours)),
                format!("merge {experiment}: Fast-forward"),
                who,
            )
            .ctx("Could not move the experiment forward")?;
            return Ok(BringInto::FastForward(theirs.to_string()));
        }
        let labels = gix::merge::blob::builtin_driver::text::Labels {
            ancestor: Some("base".into()),
            current: Some(current.as_bytes().as_bstr()),
            other: Some(experiment.as_bytes().as_bstr()),
        };
        let mut outcome = self
            .repo
            .merge_commits(ours, theirs, labels, gix::merge::commit::Options::default())
            .ctx("Could not merge")?;
        let how = gix::merge::tree::TreatAsUnresolved::git();
        if outcome.tree_merge.has_unresolved_conflicts(how) {
            let mut paths: Vec<String> = outcome
                .tree_merge
                .conflicts
                .iter()
                .filter(|c| c.is_unresolved(how))
                .map(|c| c.ours.location().to_string())
                .collect();
            paths.sort();
            paths.dedup();
            return Ok(BringInto::Conflicts(paths));
        }
        let tree = outcome
            .tree_merge
            .tree
            .write()
            .ctx("Could not write the merged tree")?
            .detach();
        self.checkout_tree(tree)?;
        let sig = who.signature();
        let mut buf = gix::date::parse::TimeBuf::default();
        let mut buf2 = gix::date::parse::TimeBuf::default();
        let id = self
            .repo
            .commit_as(
                sig.to_ref(&mut buf),
                sig.to_ref(&mut buf2),
                "HEAD",
                format!("Bring {experiment} into {current}"),
                tree,
                [ours, theirs],
            )
            .ctx("Could not save the merged version")?;
        Ok(BringInto::Merged(id.to_string()))
    }

    /// **Share** `experiment` with a bare repository in a local folder (a
    /// USB stick, a shared drive): the missing objects are copied in
    /// process and the folder's branch moves forward with a
    /// compare-and-swap. Never a force: if the folder has versions this
    /// project lacks, nothing changes ([`Share::NeedsGetLatest`]).
    pub fn share_to_folder(&self, bare: &Path, experiment: &str) -> Result<Share> {
        let remote =
            gix::open_opts(bare, open_options()).ctx("Could not open the shared folder")?;
        if !remote.is_bare() {
            return err("The shared folder must be a bare repository");
        }
        let full = format!("refs/heads/{experiment}");
        let tip = self.resolve(&full)?;
        let old = match remote
            .try_find_reference(full.as_str())
            .ctx("Could not read the shared folder")?
        {
            Some(mut r) => Some(
                r.peel_to_id()
                    .ctx("Could not read the shared folder")?
                    .detach(),
            ),
            None => None,
        };
        if old == Some(tip) {
            return Ok(Share::UpToDate);
        }
        if let Some(old) = old {
            let known = self.repo.has_object(old);
            let ancestor =
                known && matches!(self.repo.merge_base(old, tip), Ok(Some(b)) if b.detach() == old);
            if !ancestor {
                return Ok(Share::NeedsGetLatest);
            }
        }
        // every object reachable from tip and not from old
        let walk = self
            .repo
            .rev_walk([tip])
            .with_hidden(old)
            .all()
            .ctx("Could not list the versions to share")?;
        let mut copied = 0usize;
        for info in walk {
            let info = info.ctx("Could not list the versions to share")?;
            let commit = self
                .repo
                .find_commit(info.id)
                .ctx("Could not read a version")?;
            let tree = commit.tree_id().ctx("Not a version")?.detach();
            copied += copy_tree(&self.repo, &remote, tree)?;
            copied += copy_object(&self.repo, &remote, info.id)?;
        }
        let constraint = match old {
            Some(old) => PreviousValue::MustExistAndMatch(Target::Object(old)),
            None => PreviousValue::MustNotExist,
        };
        edit_ref(
            &remote,
            &full,
            Target::Object(tip),
            constraint,
            "push".into(),
            &self.person,
        )
        .ctx("The shared folder changed meanwhile: get the latest first")?;
        Ok(Share::Shared {
            from: old.map(|o| o.to_string()),
            to: tip.to_string(),
            objects: copied,
        })
    }
}

/// Update one reference with a compare-and-swap (`expected`), its reflog
/// written as `who` (repositories are opened without the user's git
/// identity).
fn edit_ref(
    repo: &gix::Repository,
    name: &str,
    new: Target,
    expected: PreviousValue,
    message: String,
    who: &Person,
) -> Result<()> {
    let sig = who.signature();
    let mut buf = gix::date::parse::TimeBuf::default();
    let edit = RefEdit {
        change: Change::Update {
            log: LogChange {
                mode: RefLog::AndReference,
                force_create_reflog: false,
                message: message.into(),
            },
            expected,
            new,
        },
        name: name.try_into().ctx("Not a valid reference name")?,
        deref: false,
    };
    repo.edit_references_as([edit], Some(sig.to_ref(&mut buf)))
        .ctx("Could not update the reference")?;
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
            "core" => strip.push((
                id,
                &[
                    "fsmonitor",
                    "sshcommand",
                    "askpass",
                    "gitproxy",
                    "hookspath",
                    "editor",
                    "pager",
                ],
            )),
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
            let present: Vec<String> = section
                .value_names()
                .map(|n| n.to_string())
                .filter(|n| keys.contains(&n.to_ascii_lowercase().as_str()))
                .collect();
            for key in present {
                while section.remove(&key).is_some() {}
            }
        }
    }
    config
        .commit()
        .ctx("Could not apply the safe configuration")?;
    Ok(())
}

fn copy_object(from: &gix::Repository, to: &gix::Repository, id: ObjectId) -> Result<usize> {
    if to.has_object(id) {
        return Ok(0);
    }
    let object = from.find_object(id).ctx("Could not read an object")?;
    use gix::objs::Write;
    let written = to
        .objects
        .write_buf(object.kind, &object.data)
        .ctx("Could not write to the shared folder")?;
    if written != id {
        return err("An object changed while copying");
    }
    Ok(1)
}

fn copy_tree(from: &gix::Repository, to: &gix::Repository, tree: ObjectId) -> Result<usize> {
    if to.has_object(tree) {
        return Ok(0);
    }
    let mut copied = 0;
    let entries: Vec<(ObjectId, bool)> = {
        let object = from.find_tree(tree).ctx("Could not read a tree")?;
        let decoded = object.decode().ctx("Could not read a tree")?;
        decoded
            .entries
            .iter()
            .filter(|e| !e.mode.is_commit())
            .map(|e| (e.oid.to_owned(), e.mode.is_tree()))
            .collect()
    };
    for (id, is_tree) in entries {
        copied += if is_tree {
            copy_tree(from, to, id)?
        } else {
            copy_object(from, to, id)?
        };
    }
    copied += copy_object(from, to, tree)?;
    Ok(copied)
}

fn remove_empty_parents(root: &Path, file: &Path) {
    let mut dir: Option<PathBuf> = file.parent().map(Path::to_path_buf);
    while let Some(d) = dir {
        if d == root || !d.starts_with(root) {
            break;
        }
        if std::fs::remove_dir(&d).is_err() {
            break;
        }
        dir = d.parent().map(Path::to_path_buf);
    }
}

fn is_binary(data: &[u8]) -> bool {
    data.iter().take(8000).any(|b| *b == 0)
}

fn file_diff(path: String, state: FileState, old: &[u8], new: &[u8]) -> FileDiff {
    if is_binary(old) || is_binary(new) {
        return FileDiff {
            path,
            state,
            binary: true,
            hunks: Vec::new(),
        };
    }
    FileDiff {
        path,
        state,
        binary: false,
        hunks: hunks(old, new, 3),
    }
}

/// Line hunks between `old` and `new` (histogram diff, imara-diff as
/// gitoxide ships it) with `context` lines around each change.
pub fn hunks(old: &[u8], new: &[u8], context: u32) -> Vec<Hunk> {
    use gix::diff::blob::{sources, Algorithm, Diff, InternedInput};
    let input = InternedInput::new(sources::byte_lines(old), sources::byte_lines(new));
    let mut diff = Diff::compute(Algorithm::Histogram, &input);
    diff.postprocess_lines(&input);
    let before: Vec<&[u8]> = old.split_inclusive(|b| *b == b'\n').collect();
    let after: Vec<&[u8]> = new.split_inclusive(|b| *b == b'\n').collect();
    let text = |l: &[u8]| l.to_str_lossy().trim_end_matches(['\n', '\r']).to_string();

    // group raw hunks whose context would touch
    let raw: Vec<gix::diff::blob::Hunk> = diff.hunks().collect();
    let mut groups: Vec<Vec<gix::diff::blob::Hunk>> = Vec::new();
    for h in raw {
        match groups.last_mut() {
            Some(g) if h.before.start <= g.last().unwrap().before.end + 2 * context => g.push(h),
            _ => groups.push(vec![h]),
        }
    }
    let mut out = Vec::new();
    for g in groups {
        let first = g.first().unwrap();
        let last = g.last().unwrap();
        let old_from = first.before.start.saturating_sub(context);
        let new_from = first
            .after
            .start
            .saturating_sub(first.before.start - old_from);
        let old_to = (last.before.end + context).min(before.len() as u32);
        let new_to = (last.after.end + (old_to - last.before.end)).min(after.len() as u32);
        let mut lines = Vec::new();
        let mut o = old_from;
        for h in &g {
            while o < h.before.start {
                lines.push(Line::Same(text(before[o as usize])));
                o += 1;
            }
            for i in h.before.clone() {
                lines.push(Line::Removed(text(before[i as usize])));
            }
            for i in h.after.clone() {
                lines.push(Line::Added(text(after[i as usize])));
            }
            o = h.before.end;
        }
        while o < old_to {
            lines.push(Line::Same(text(before[o as usize])));
            o += 1;
        }
        let old_len = old_to - old_from;
        let new_len = new_to - new_from;
        // unified diff numbering: an empty side starts at the line before
        let old_start = if old_len == 0 { old_from } else { old_from + 1 };
        let new_start = if new_len == 0 { new_from } else { new_from + 1 };
        out.push(Hunk {
            old_start,
            old_len,
            new_start,
            new_len,
            lines,
        });
    }
    out
}

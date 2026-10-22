//! Sharing versions through a folder — a USB stick, a network drive: a
//! bare repository there, read and written in-process.
//!
//! gitoxide's own local transport runs the `git` command
//! (docs/versions-g0.md §1), so Versions never uses it: the objects a side
//! lacks are copied one by one, and the receiving experiment moves with a
//! compare-and-swap. Never a force: sharing refuses when the folder has
//! versions the project lacks ([`Share::NeedsGetLatest`]), and getting
//! the latest joins the two histories as bringing an experiment in does.

use std::collections::HashSet;
use std::path::Path;

use gix::refs::transaction::PreviousValue;
use gix::refs::Target;
use gix::ObjectId;

use crate::history::Entry;
use crate::{checkout, edit_ref, fail, open_options, owned_by_me, BringInto, Ctx, ErrorKind, Person, Result, Versions};

/// What sharing with a folder did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Share {
    /// The folder already had this version.
    UpToDate,
    /// The folder's experiment moved from `from` (None: new there) to `to`.
    Shared { from: Option<String>, to: String, objects: usize },
    /// The folder has versions this project doesn't have: get them first.
    NeedsGetLatest,
}

/// What getting the latest from a folder did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GetLatest {
    /// Nothing new there.
    UpToDate,
    /// The project moved forward to the folder's version (no new
    /// version); `objects` were copied.
    FastForward { to: String, objects: usize },
    /// Both had new versions: a new version joins them.
    Merged { id: String, objects: usize },
    /// Both changed the same lines: the folder's versions were copied but
    /// nothing in the project changed (paths listed; the Merge tab, lane
    /// G4, resolves them).
    Conflicts(Vec<String>),
}

/// Create an empty shared folder (a bare repository) to share versions
/// with, e.g. on a USB stick or a shared drive.
pub fn create_shared_folder(path: &Path) -> Result<()> {
    std::fs::create_dir_all(path).ctx("Could not create the shared folder (is the drive read-only?).")?;
    gix::ThreadSafeRepository::init_opts(path, gix::create::Kind::Bare, gix::create::Options::default(), open_options()).ctx("Could not create the shared folder (is the drive read-only?).")?;
    Ok(())
}

/// Opens a shared folder (a bare repository).
fn open_shared(bare: &Path, writing: bool) -> Result<gix::Repository> {
    let remote = match gix::open_opts(bare, open_options()) {
        Ok(r) => r,
        Err(e) => {
            return Err(crate::Error { kind: ErrorKind::NotFound, message: "That folder isn't a shared folder of versions.".into(), detail: e.to_string(), secrets: Vec::new() });
        }
    };
    if !remote.is_bare() {
        return fail(ErrorKind::NotAProject, "That folder is a project, not a shared folder of versions: share through a shared folder (Create shared folder).");
    }
    if writing && !owned_by_me(bare) {
        return fail(ErrorKind::NotYours, "That shared folder belongs to another user account on this computer, so RapidR won't write into it. You can still get the latest from it.");
    }
    Ok(remote)
}

fn copy_object(from: &gix::Repository, to: &gix::Repository, id: ObjectId) -> Result<usize> {
    if to.has_object(id) {
        return Ok(0);
    }
    let object = from.find_object(id).ctx("Could not read a version to copy.")?;
    use gix::objs::Write;
    let written = to.objects.write_buf(object.kind, &object.data).ctx("Could not copy a version (is the drive full or read-only?).")?;
    if written != id {
        return fail(ErrorKind::Failed, "A version changed while it was copied: try again.");
    }
    Ok(1)
}

fn copy_tree(from: &gix::Repository, to: &gix::Repository, tree: ObjectId) -> Result<usize> {
    if to.has_object(tree) {
        return Ok(0);
    }
    let mut copied = 0;
    let entries: Vec<(ObjectId, bool)> = {
        let object = from.find_tree(tree).ctx("Could not read a version to copy.")?;
        let decoded = object.decode().ctx("Could not read a version to copy.")?;
        decoded.entries.iter().filter(|e| !e.mode.is_commit()).map(|e| (e.oid.to_owned(), e.mode.is_tree())).collect()
    };
    for (id, is_tree) in entries {
        copied += if is_tree { copy_tree(from, to, id)? } else { copy_object(from, to, id)? };
    }
    copied += copy_object(from, to, tree)?;
    Ok(copied)
}

/// Copies every commit reachable from `tip` in `from` that `to` lacks,
/// with its files (a commit `to` has is assumed complete, with its
/// history).
fn copy_history(from: &gix::Repository, to: &gix::Repository, tip: ObjectId) -> Result<usize> {
    let mut copied = 0;
    let mut todo = vec![tip];
    let mut seen = HashSet::new();
    let mut commits = Vec::new();
    while let Some(id) = todo.pop() {
        if !seen.insert(id) || to.has_object(id) {
            continue;
        }
        let commit = from.find_commit(id).ctx("Could not read a version to copy.")?;
        todo.extend(commit.parent_ids().map(|p| p.detach()));
        commits.push((id, commit.tree_id().ctx("Could not read a version to copy.")?.detach()));
    }
    // (oldest first: a commit lands after what it needs)
    for (id, tree) in commits.into_iter().rev() {
        copied += copy_tree(from, to, tree)?;
        copied += copy_object(from, to, id)?;
    }
    Ok(copied)
}

impl Versions {
    /// **Share** `experiment` with a bare repository in a local folder (a
    /// USB stick, a shared drive): the missing objects are copied in
    /// process and the folder's branch moves forward with a
    /// compare-and-swap. Never a force: if the folder has versions this
    /// project lacks, nothing changes ([`Share::NeedsGetLatest`]).
    pub fn share_to_folder(&self, bare: &Path, experiment: &str) -> Result<Share> {
        let remote = open_shared(bare, true)?;
        let full = format!("refs/heads/{experiment}");
        let tip = self.resolve(&full).map_err(|_| crate::Error::new(ErrorKind::NotFound, format!("There is no experiment called \u{201C}{experiment}\u{201D}.")))?;
        let old = match remote.try_find_reference(full.as_str()).ctx("Could not read the shared folder.")? {
            Some(mut r) => Some(r.peel_to_id().ctx("Could not read the shared folder.")?.detach()),
            None => None,
        };
        if old == Some(tip) {
            return Ok(Share::UpToDate);
        }
        if let Some(old) = old {
            let known = self.repo.has_object(old);
            let ancestor = known && matches!(self.repo.merge_base(old, tip), Ok(Some(b)) if b.detach() == old);
            if !ancestor {
                return Ok(Share::NeedsGetLatest);
            }
        }
        let copied = copy_history(&self.repo, &remote, tip)?;
        let constraint = match old {
            Some(old) => PreviousValue::MustExistAndMatch(Target::Object(old)),
            None => PreviousValue::MustNotExist,
        };
        edit_ref(&remote, &full, Target::Object(tip), constraint, "push".into(), &self.person).map_err(|e| crate::Error { message: "The shared folder changed meanwhile: get the latest from it first.".into(), ..e })?;
        Ok(Share::Shared { from: old.map(|o| o.to_string()), to: tip.to_string(), objects: copied })
    }

    /// **Get latest** from a shared folder: the versions of its
    /// `experiment` are copied in, then the current experiment moves
    /// forward to them, or — when both sides have new versions — a new
    /// version joins them. Needs no unsaved changes; a safety point first
    /// when files change. The folder is only read.
    pub fn get_latest_from_folder(&mut self, bare: &Path, experiment: &str, who: &Person) -> Result<GetLatest> {
        self.writable()?;
        self.ensure_saved()?;
        let remote = open_shared(bare, false)?;
        let full = format!("refs/heads/{experiment}");
        let Some(mut r) = remote.try_find_reference(full.as_str()).ctx("Could not read the shared folder.")? else {
            return fail(ErrorKind::NotFound, format!("The shared folder has no experiment called \u{201C}{experiment}\u{201D}."));
        };
        let theirs = r.peel_to_id().ctx("Could not read the shared folder.")?.detach();
        let objects = copy_history(&remote, &self.repo, theirs)?;
        let branch = self.head_ref()?;
        match self.head_id()? {
            None => {
                // (no version here yet: the folder's become this project's)
                let tree = self.tree_id_of_commit(theirs)?;
                let words = format!("Got the latest from the shared folder ({experiment})");
                let safety = self.safety_point(&words)?;
                checkout::switch_files(self, tree)?;
                edit_ref(&self.repo, &branch, Target::Object(theirs), PreviousValue::MustNotExist, "rapidr: get latest".into(), who)?;
                self.record(Entry::new("get-latest", words).moved(&branch, None, Some(theirs)).with_before(safety).with_after(tree))?;
                Ok(GetLatest::FastForward { to: theirs.to_string(), objects })
            }
            Some(_) => {
                let words = format!("Got the latest from the shared folder ({experiment})");
                Ok(match self.bring_commit(theirs, &format!("shared folder/{experiment}"), &words, &format!("Get the latest {experiment} into"), who)? {
                    BringInto::AlreadyUpToDate => GetLatest::UpToDate,
                    BringInto::FastForward(to) => GetLatest::FastForward { to, objects },
                    BringInto::Merged(id) => GetLatest::Merged { id, objects },
                    BringInto::Conflicts(paths) => GetLatest::Conflicts(paths),
                })
            }
        }
    }
}

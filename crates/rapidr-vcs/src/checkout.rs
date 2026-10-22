//! Writing versions into the working folder: only what differs.
//!
//! gitoxide has the blocks but no switch or restore command
//! (docs/versions-g0.md §1). Here:
//! - [`switch_files`]: the working folder and the index move to a tree
//!   (switching experiments, going back, bringing an experiment in). Only
//!   the entries whose content or mode differ from the current index are
//!   written; the others keep their file and their index stat; files the
//!   tree doesn't have are removed. Unsaved changes are checked by the
//!   caller.
//! - [`restore_paths`]: some paths of the working folder as a tree has
//!   them (restoring a file, a checkpoint, Undo); the index is left as it
//!   is, so what differs from the current version shows as unsaved.
//! - [`reset_index`]: the index back on the current version's tree.
//!
//! The files are written by gitoxide's own checkout (line-ending
//! conversions, executable bits, symbolic links), given a temporary index
//! of just those paths.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use gix::bstr::{BString, ByteSlice};
use gix::objs::tree::EntryMode;
use gix::ObjectId;

use crate::{fail, Ctx, ErrorKind, Result, Versions};

/// A file's content id and mode in a tree.
pub(crate) type Entry = (ObjectId, EntryMode);

/// A path whose file differs: its entry on each side (`None`: absent).
pub(crate) type PathChange = (BString, Option<Entry>, Option<Entry>);

/// Every file of `tree` (not its folders) by path.
fn flat(repo: &gix::Repository, tree: ObjectId) -> Result<BTreeMap<BString, Entry>> {
    let mut rec = gix::traverse::tree::Recorder::default();
    repo.find_tree(tree).ctx("Could not read a version.")?.traverse().breadthfirst(&mut rec).ctx("Could not read a version.")?;
    Ok(rec.records.into_iter().filter(|e| !e.mode.is_tree()).map(|e| (e.filepath, (e.oid, e.mode))).collect())
}

/// The paths whose files differ between trees `a` (`None`: empty) and
/// `b`, sorted, with each side's entry.
pub(crate) fn tree_changes(repo: &gix::Repository, a: Option<ObjectId>, b: ObjectId) -> Result<Vec<PathChange>> {
    let old = match a {
        Some(a) if a == b => return Ok(Vec::new()),
        Some(a) => flat(repo, a)?,
        None => BTreeMap::new(),
    };
    let new = flat(repo, b)?;
    let mut out = Vec::new();
    let paths: std::collections::BTreeSet<&BString> = old.keys().chain(new.keys()).collect();
    for p in paths {
        let (o, n) = (old.get(p).copied(), new.get(p).copied());
        if o != n {
            out.push((p.clone(), o, n));
        }
    }
    Ok(out)
}

/// `path`'s entry in `tree` (`None`: no such file).
pub(crate) fn entry_at(repo: &gix::Repository, tree: ObjectId, path: &str) -> Result<Option<Entry>> {
    let tree = repo.find_tree(tree).ctx("Could not read a version.")?;
    Ok(tree.lookup_entry_by_path(path).ctx("Could not read a version.")?.filter(|e| !e.mode().is_tree()).map(|e| (e.object_id(), e.mode())))
}

/// Writes `index`'s entries into the working folder (their stats filled
/// in).
fn write_entries(v: &Versions, index: &mut gix::index::File) -> Result<()> {
    if index.entries().is_empty() {
        return Ok(());
    }
    let mut opts = v.repo.checkout_options(gix::worktree::stack::state::attributes::Source::IdMapping).ctx("Could not prepare to write the files.")?;
    opts.overwrite_existing = true;
    opts.destination_is_initially_empty = false;
    let objects = v.repo.objects.clone().into_arc().ctx("Could not prepare to write the files.")?;
    let outcome = gix::worktree::state::checkout(index, v.folder(), objects, &gix::progress::Discard, &gix::progress::Discard, &AtomicBool::new(false), opts).ctx("Could not write the files.")?;
    if !outcome.errors.is_empty() || !outcome.collisions.is_empty() {
        let first = outcome.errors.first().map(|e| format!("{}: {}", e.path, e.error)).unwrap_or_default();
        return Err(crate::Error {
            kind: ErrorKind::Failed,
            message: format!("Could not write {} file(s) (is one open in another program, or the folder read-only?).", outcome.errors.len() + outcome.collisions.len()),
            detail: first,
            secrets: Vec::new(),
        });
    }
    Ok(())
}

fn remove_file(root: &Path, rela: &[u8]) {
    let Ok(rela) = gix::path::from_byte_slice(rela) else { return };
    let file = root.join(rela);
    if std::fs::remove_file(&file).is_ok() {
        remove_empty_parents(root, &file);
    }
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

/// The working folder and the index moved to `tree` (see the module's
/// notes).
pub(crate) fn switch_files(v: &Versions, tree: ObjectId) -> Result<()> {
    let old = v.repo.index_or_empty().ctx("Could not read the project's file list (the index).")?;
    let mut new = v.repo.index_from_tree(&tree).ctx("Could not read the version.")?;
    let mut write = HashSet::new();
    let mut keep = Vec::new();
    for (i, e) in new.entries().iter().enumerate() {
        match old.entry_by_path(e.path(&new)) {
            Some(o) if o.id == e.id && o.mode == e.mode => keep.push((i, o.stat)),
            _ => {
                write.insert(i);
            }
        }
    }
    for (i, stat) in keep {
        new.entries_mut()[i].stat = stat;
    }
    let mut part = new.clone();
    part.remove_entries(|i, _, _| !write.contains(&i));
    write_entries(v, &mut part)?;
    for e in part.entries() {
        let stat = e.stat;
        if let Some(n) = new.entry_mut_by_path_and_stage(e.path(&part), gix::index::entry::Stage::Unconflicted) {
            n.stat = stat;
        }
    }
    // the files the tree doesn't have
    for e in old.entries() {
        let path = e.path(&old);
        if new.entry_by_path(path).is_none() {
            remove_file(v.folder(), path.as_bytes());
        }
    }
    new.write(Default::default()).ctx("Could not write the project's file list (the index).")?;
    Ok(())
}

/// `paths` of the working folder as `tree` has them (removed where it has
/// none); the index unchanged.
pub(crate) fn restore_paths(v: &Versions, tree: ObjectId, paths: &[String]) -> Result<()> {
    if paths.is_empty() {
        return Ok(());
    }
    let wanted: HashSet<&[u8]> = paths.iter().map(|p| p.as_bytes()).collect();
    let mut part = v.repo.index_from_tree(&tree).ctx("Could not read the version.")?;
    part.remove_entries(|_, path, _| !wanted.contains(path.as_bytes()));
    let present: HashSet<Vec<u8>> = part.entries().iter().map(|e| e.path(&part).as_bytes().to_vec()).collect();
    for p in paths {
        if !present.contains(p.as_bytes()) {
            let file = v.folder().join(p);
            if file.is_dir() {
                return fail(ErrorKind::Failed, format!("{p} is a folder now, so the file can't be put back."));
            }
            remove_file(v.folder(), p.as_bytes());
        }
    }
    write_entries(v, &mut part)
}

/// The index back on the current version's tree (an empty one before the
/// first version), keeping the stats of entries that didn't change.
pub(crate) fn reset_index(v: &Versions) -> Result<()> {
    let old = v.repo.index_or_empty().ctx("Could not read the project's file list (the index).")?;
    let mut new = match v.head_tree_id()? {
        Some(tree) => v.repo.index_from_tree(&tree).ctx("Could not read the version.")?,
        None => gix::index::File::from_state(gix::index::State::new(v.repo.object_hash()), v.repo.index_path()),
    };
    let mut keep = Vec::new();
    for (i, e) in new.entries().iter().enumerate() {
        if let Some(o) = old.entry_by_path(e.path(&new)) {
            if o.id == e.id && o.mode == e.mode {
                keep.push((i, o.stat));
            }
        }
    }
    for (i, stat) in keep {
        new.entries_mut()[i].stat = stat;
    }
    new.write(Default::default()).ctx("Could not write the project's file list (the index).")?;
    Ok(())
}

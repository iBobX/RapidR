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
//!
//! **Nothing leaves the project's folder** (SEC-32,
//! docs/security/audit-2026-11-05-system.md): a version may come from
//! someone else (Get latest, a project that came with its history), so
//! - a link it has is written only when it points inside the project —
//!   followed through the other links the folder will have, `d -> .` then
//!   `x -> d/..` too — else nothing at all is written
//!   ([`ErrorKind::OutsideFolder`], [`check_links`]);
//! - files are never written through a link: gitoxide's checkout opens
//!   every file without following links and replaces a folder of the path
//!   that is a link by a real folder (its `create_leading_directory`,
//!   proved by `tests/security_links.rs`);
//! - a file is never removed through a folder of its path that is a link
//!   now ([`remove_file`]).

use std::collections::{BTreeMap, HashSet, VecDeque};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::AtomicBool;

use gix::bstr::{BStr, BString, ByteSlice};
use gix::objs::tree::EntryMode;
use gix::ObjectId;

use crate::{fail, Ctx, Error, ErrorKind, Result, Versions};

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

/// What a path of the working folder will be once a checkout is done.
enum Will {
    /// A link to this target.
    Link(Vec<u8>),
    /// A real folder.
    Folder,
    /// A file, or nothing yet (it could become a link later: `..` after it
    /// isn't trusted).
    Other,
}

/// Whether a link's `target`, as Git stores it, names a place on its own:
/// `/x`, `\x`, `C:…`, `\\server\…` — or has a `:` anywhere (a drive or an
/// NTFS stream on Windows).
fn is_absolute_target(target: &[u8]) -> bool {
    target.first().is_some_and(|b| *b == b'/' || *b == b'\\') || target.contains(&b':')
}

fn target_parts(target: &[u8]) -> Vec<String> {
    target.split(|b| *b == b'/' || *b == b'\\').map(|p| String::from_utf8_lossy(p).into_owned()).collect()
}

/// Whether the link at `link` (a project path, `/`-separated) pointing to
/// `target` leads outside the project's folder once resolved — through the
/// links `will` says the folder will have — or could later: a `..` after
/// a name that isn't a real folder (nothing there yet, which a later link
/// could fill), too many links.
fn leads_out(link: &str, target: &[u8], will: &dyn Fn(&str) -> Will) -> bool {
    if is_absolute_target(target) {
        return true;
    }
    // (each step taken, and whether it is a real folder: the link's own
    // folders are, its checkout makes them)
    let mut cur: Vec<(String, bool)> = link.split('/').map(|s| (s.to_string(), true)).collect();
    cur.pop();
    let mut todo: VecDeque<String> = target_parts(target).into();
    // (as many links as a system follows)
    let mut budget = 40;
    while let Some(part) = todo.pop_front() {
        match part.as_str() {
            "" | "." => {}
            ".." => match cur.pop() {
                Some((_, true)) => {}
                _ => return true,
            },
            name => {
                let path = cur.iter().map(|(s, _)| s.as_str()).chain([name]).collect::<Vec<_>>().join("/");
                match will(&path) {
                    Will::Link(t) => {
                        budget -= 1;
                        if budget == 0 || is_absolute_target(&t) {
                            return true;
                        }
                        for p in target_parts(&t).into_iter().rev() {
                            todo.push_front(p);
                        }
                    }
                    Will::Folder => cur.push((name.to_string(), true)),
                    Will::Other => cur.push((name.to_string(), false)),
                }
            }
        }
    }
    false
}

/// SEC-32: every link `part` would write must point inside the project's
/// folder — and so must the links `tracked` (the project's files once it
/// is written) keeps, which a new link may now lead elsewhere. Resolved as
/// the folder will be: `part`'s entries and the folders they make, else
/// the disk. Checked before anything is written; the first link that leads
/// out refuses the whole checkout. Nothing to do (the usual case) when
/// `part` writes no link.
fn check_links(v: &Versions, part: &gix::index::File, tracked: &gix::index::File) -> Result<()> {
    let is_link = |e: &gix::index::Entry| e.mode == gix::index::entry::Mode::SYMLINK;
    if !part.entries().iter().any(is_link) {
        return Ok(());
    }
    // (the folders part's files will be in: real folders, made by the
    // checkout, a link there replaced)
    let mut folders: HashSet<String> = HashSet::new();
    for e in part.entries() {
        let p = e.path(part).to_str_lossy().into_owned();
        let mut at = p.as_str();
        while let Some((up, _)) = at.rsplit_once('/') {
            if !folders.insert(up.to_string()) {
                break;
            }
            at = up;
        }
    }
    let root = v.folder().to_path_buf();
    let blob = |id: ObjectId| v.repo.find_object(id).ok().map(|o| o.data.clone()).unwrap_or_default();
    let will = |p: &str| -> Will {
        if let Some(e) = part.entry_by_path(BStr::new(p.as_bytes())) {
            return if is_link(e) { Will::Link(blob(e.id)) } else { Will::Other };
        }
        if folders.contains(p) {
            return Will::Folder;
        }
        let on_disk = root.join(p);
        match std::fs::symlink_metadata(&on_disk) {
            Ok(m) if m.file_type().is_symlink() => crate::link_target(&on_disk).map_or(Will::Other, Will::Link),
            Ok(m) if m.is_dir() => Will::Folder,
            _ => Will::Other,
        }
    };
    // (part's links, then the kept ones as the disk has them)
    let mut links: Vec<(String, Vec<u8>)> = part.entries().iter().filter(|e| is_link(e)).map(|e| (e.path(part).to_str_lossy().into_owned(), blob(e.id))).collect();
    for e in tracked.entries().iter().filter(|e| is_link(e)) {
        let path = e.path(tracked);
        if part.entry_by_path(path).is_none() {
            if let Some(t) = crate::link_target(&root.join(path.to_str_lossy().as_ref())) {
                links.push((path.to_str_lossy().into_owned(), t));
            }
        }
    }
    for (path, target) in links {
        if leads_out(&path, &target, &will) {
            return Err(Error {
                kind: ErrorKind::OutsideFolder,
                message: format!(
                    "This version has a link, {path}, that points outside the project's folder ({}), so Versions won't write it. Nothing was changed.",
                    target.to_str_lossy()
                ),
                detail: format!("{path} -> {}", target.to_str_lossy()),
                secrets: Vec::new(),
            });
        }
    }
    Ok(())
}

/// Writes `index`'s entries into the working folder (their stats filled
/// in); `tracked` is the project's files once they are ([`check_links`]).
fn write_entries(v: &Versions, index: &mut gix::index::File, tracked: &gix::index::File) -> Result<()> {
    if index.entries().is_empty() {
        return Ok(());
    }
    check_links(v, index, tracked)?;
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

/// Whether a folder on the way to `rela` (below `root`) isn't a real
/// folder of the project: a link (it leads elsewhere), a file, missing, or
/// a step that isn't a name (`..`).
fn through_link(root: &Path, rela: &Path) -> bool {
    let steps: Vec<Component> = rela.components().collect();
    let mut at = root.to_path_buf();
    for step in &steps[..steps.len().saturating_sub(1)] {
        let Component::Normal(name) = step else { return true };
        at.push(name);
        if !std::fs::symlink_metadata(&at).is_ok_and(|m| m.is_dir()) {
            return true;
        }
    }
    !matches!(steps.last(), Some(Component::Normal(_)))
}

/// Removes the file at `rela` — the file itself when it is a link, never
/// what it points at — and the folders it leaves empty. Nothing when a
/// folder of its path is a link now (SEC-32: the file there isn't the
/// project's).
fn remove_file(root: &Path, rela: &[u8]) {
    let Ok(rela) = gix::path::from_byte_slice(rela) else { return };
    if through_link(root, rela) {
        return;
    }
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

/// What moving to `tree` writes: the index it makes (the stats of the
/// entries that don't change kept) and the part of it that differs.
fn plan_switch(v: &Versions, old: &gix::index::State, tree: ObjectId) -> Result<(gix::index::File, gix::index::File)> {
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
    Ok((new, part))
}

/// Whether [`switch_files`] to `tree` would write a link that leads out of
/// the project ([`ErrorKind::OutsideFolder`]), changing nothing: for the
/// actions that save a version before they write the files.
pub(crate) fn check_switch(v: &Versions, tree: ObjectId) -> Result<()> {
    let old = v.repo.index_or_empty().ctx("Could not read the project's file list (the index).")?;
    let (new, part) = plan_switch(v, &old, tree)?;
    check_links(v, &part, &new)
}

/// What [`restore_paths`] writes: `paths` as `tree` has them; and the
/// project's tracked files (its index, left as it is) for [`check_links`].
fn plan_restore(v: &Versions, tree: ObjectId, paths: &[String]) -> Result<(gix::index::File, gix::index::File)> {
    let wanted: HashSet<&[u8]> = paths.iter().map(|p| p.as_bytes()).collect();
    let mut part = v.repo.index_from_tree(&tree).ctx("Could not read the version.")?;
    part.remove_entries(|_, path, _| !wanted.contains(path.as_bytes()));
    let tracked: gix::index::File = (**v.repo.index_or_empty().ctx("Could not read the project's file list (the index).")?).clone();
    Ok((part, tracked))
}

/// [`check_switch`] for [`restore_paths`].
pub(crate) fn check_restore(v: &Versions, tree: ObjectId, paths: &[String]) -> Result<()> {
    let (part, tracked) = plan_restore(v, tree, paths)?;
    check_links(v, &part, &tracked)
}

/// The working folder and the index moved to `tree` (see the module's
/// notes).
pub(crate) fn switch_files(v: &Versions, tree: ObjectId) -> Result<()> {
    let old = v.repo.index_or_empty().ctx("Could not read the project's file list (the index).")?;
    let (mut new, mut part) = plan_switch(v, &old, tree)?;
    write_entries(v, &mut part, &new)?;
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
    let (mut part, tracked) = plan_restore(v, tree, paths)?;
    let present: HashSet<Vec<u8>> = part.entries().iter().map(|e| e.path(&part).as_bytes().to_vec()).collect();
    // (the links first: nothing removed when one would lead out)
    check_links(v, &part, &tracked)?;
    for p in paths {
        if !present.contains(p.as_bytes()) {
            let file = v.folder().join(p);
            // (a link to a folder is removed as the link it is)
            if std::fs::symlink_metadata(&file).is_ok_and(|m| m.is_dir()) {
                return fail(ErrorKind::Failed, format!("{p} is a folder now, so the file can't be put back."));
            }
            remove_file(v.folder(), p.as_bytes());
        }
    }
    write_entries(v, &mut part, &tracked)
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

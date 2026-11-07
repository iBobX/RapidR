//! The cache: what was downloaded and checked, kept so the next build of
//! the same target needs no network.
//!
//! ```text
//! <cache>/<version>/runners-manifest.json
//! <cache>/<version>/runners-manifest.json.minisig
//! <cache>/<version>/<target>.zst
//! ```
//!
//! Files are written only after they were checked, whole (a temporary name,
//! then a rename: a build that stops halfway leaves nothing a later build
//! would read), and read back through the same checks every time — the cache
//! is a place to keep files, never a reason to trust them. A file that no
//! longer passes is removed.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::urls::{MANIFEST_NAME, SIGNATURE_NAME};

/// A version's folder in the cache.
pub fn version_dir(cache: &Path, version: &str) -> PathBuf {
    cache.join(version)
}

/// Writes `bytes` to `path` whole, or not at all.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let part = path.with_file_name(format!(".{name}.part-{}", std::process::id()));
    let result = fs::write(&part, bytes).and_then(|()| fs::rename(&part, path));
    if result.is_err() {
        let _ = fs::remove_file(&part);
    }
    result
}

/// The cached list and its signature, if both are there.
pub fn read_list(dir: &Path) -> Option<(Vec<u8>, String)> {
    let manifest = fs::read(dir.join(MANIFEST_NAME)).ok()?;
    let signature = fs::read_to_string(dir.join(SIGNATURE_NAME)).ok()?;
    Some((manifest, signature))
}

/// Keeps a checked list and its signature.
pub fn write_list(dir: &Path, manifest: &[u8], signature: &str) -> io::Result<()> {
    write_atomic(&dir.join(MANIFEST_NAME), manifest)?;
    write_atomic(&dir.join(SIGNATURE_NAME), signature.as_bytes())
}

/// Removes the cached list (it no longer verifies).
pub fn remove_list(dir: &Path) {
    let _ = fs::remove_file(dir.join(MANIFEST_NAME));
    let _ = fs::remove_file(dir.join(SIGNATURE_NAME));
}

/// The target files of a version's folder: `(file name, size)`.
pub fn list_files(dir: &Path) -> Vec<(String, u64)> {
    let mut out: Vec<(String, u64)> = fs::read_dir(dir)
        .map(|d| {
            d.flatten()
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().into_owned();
                    let meta = e.metadata().ok()?;
                    (meta.is_file() && name.ends_with(".zst")).then_some((name, meta.len()))
                })
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

/// Removes the folders of other versions that nothing has used for `unused`
/// (a RapidR that was updated leaves the old version's runners, tens of
/// megabytes each). Folders with another name than a version, and the
/// version in use, stay. Gives how many it removed.
pub fn prune_other_versions(cache: &Path, keep: &str, unused: Duration) -> usize {
    let Ok(read) = fs::read_dir(cache) else { return 0 };
    let now = SystemTime::now();
    let mut removed = 0;
    for entry in read.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let looks_like_version = name.chars().next().is_some_and(|c| c.is_ascii_digit()) && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+'));
        if name == keep || !looks_like_version || !entry.path().is_dir() {
            continue;
        }
        if newest_use(&entry.path()).and_then(|t| now.duration_since(t).ok()).is_some_and(|age| age >= unused) {
            removed += usize::from(fs::remove_dir_all(entry.path()).is_ok());
        }
    }
    removed
}

/// The newest time any file in `dir` was made or read.
fn newest_use(dir: &Path) -> Option<SystemTime> {
    fs::read_dir(dir).ok()?.flatten().filter_map(|e| e.metadata().ok()).filter_map(|m| m.modified().ok().into_iter().chain(m.accessed().ok()).max()).max()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rapidr-runners-store-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn writes_whole_files_and_leaves_no_part_behind() {
        let dir = temp("write");
        let file = dir.join("2.123.1").join("web.zst");
        write_atomic(&file, b"abc").unwrap();
        write_atomic(&file, b"abcdef").unwrap();
        assert_eq!(fs::read(&file).unwrap(), b"abcdef");
        let names: Vec<_> = fs::read_dir(file.parent().unwrap()).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
        assert_eq!(names, ["web.zst"]);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_list_is_kept_and_removed_together() {
        let dir = temp("list");
        assert!(read_list(&dir).is_none());
        write_list(&dir, b"{}", "sig").unwrap();
        assert_eq!(read_list(&dir).unwrap(), (b"{}".to_vec(), "sig".to_string()));
        remove_list(&dir);
        assert!(read_list(&dir).is_none());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn old_versions_go_only_when_unused_and_only_version_folders() {
        let dir = temp("prune");
        for name in ["2.120.0", "2.121.0", "2.123.1", "notes"] {
            fs::create_dir_all(dir.join(name)).unwrap();
            fs::write(dir.join(name).join("web.zst"), b"x").unwrap();
        }
        // everything was just used: nothing is old enough
        assert_eq!(prune_other_versions(&dir, "2.123.1", Duration::from_secs(3600)), 0);
        // "unused for zero seconds": every other version folder goes, nothing else
        assert_eq!(prune_other_versions(&dir, "2.123.1", Duration::ZERO), 2);
        assert!(dir.join("2.123.1").is_dir() && dir.join("notes").is_dir());
        assert!(!dir.join("2.120.0").exists());
        fs::remove_dir_all(&dir).unwrap();
    }
}

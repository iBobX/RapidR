//! Where `rapidr build` compiles: a build cache outside the folder the app
//! goes to, so that folder holds only what is shipped (docs/manual/
//! building-apps.md, "Where a build happens").
//!
//! - `<cache>/build/<key>/target`: cargo's target folder, shared by every
//!   program this RapidR builds — its runtime compiled once and reused.
//!   A program's own files in it (its executable, its object files) are
//!   removed once its app is made.
//! - `<cache>/build/<key>/src/<program>-<hash>`: the Rust RapidR generates
//!   for one program while it builds, removed after (`--keep-rust` copies
//!   it beside the app first).
//!
//! `<cache>` is `RAPIDR_BUILD_CACHE`, else the system's cache folder:
//! `~/Library/Caches/RapidR` (macOS), `%LOCALAPPDATA%\RapidR\Cache`
//! (Windows), `$XDG_CACHE_HOME/rapidr` or `~/.cache/rapidr` (Linux).
//! `<key>` is this RapidR (its version and home): another version starts
//! its own, and one no build used for 30 days is removed.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::home::Home;

/// A key's folder unused this long is removed by the next build.
const UNUSED_FOR: Duration = Duration::from_secs(30 * 24 * 60 * 60);

/// The file whose time says when a key's folder was last used.
const STAMP: &str = "last-used";

/// The build cache's root: `RAPIDR_BUILD_CACHE`, else the system's cache
/// folder's `RapidR` (`rapidr` on Linux).
pub fn root() -> Option<PathBuf> {
    let absolute = |v: std::ffi::OsString| Some(PathBuf::from(v)).filter(|p| p.is_absolute());
    if let Some(dir) = env::var_os("RAPIDR_BUILD_CACHE").and_then(absolute) {
        return Some(dir);
    }
    if cfg!(windows) {
        env::var_os("LOCALAPPDATA").and_then(absolute).map(|d| d.join("RapidR").join("Cache"))
    } else if cfg!(target_os = "macos") {
        env::home_dir().map(|h| h.join("Library/Caches/RapidR"))
    } else {
        env::var_os("XDG_CACHE_HOME").and_then(absolute).or_else(|| env::home_dir().map(|h| h.join(".cache"))).map(|d| d.join("rapidr"))
    }
}

/// This RapidR's place in the cache.
pub struct Cache {
    /// `<cache>/build/<key>`
    pub dir: PathBuf,
    /// Cargo's target folder: the cache's, or CARGO_TARGET_DIR when it's set.
    pub target: PathBuf,
}

/// This RapidR's folder in the build cache (made; its time stamped; other
/// keys' folders unused for 30 days removed).
pub fn open() -> Result<Cache, String> {
    let root = root().ok_or("no folder for the build cache: set RAPIDR_BUILD_CACHE")?.join("build");
    let home = Home::find().map(|h| h.root).unwrap_or_default();
    let dir = root.join(key(env!("CARGO_PKG_VERSION"), &home));
    fs::create_dir_all(&dir).map_err(|e| format!("the build cache ({}): {e}", dir.display()))?;
    let _ = fs::write(dir.join(STAMP), env!("CARGO_PKG_VERSION"));
    prune(&root, &dir);
    let target = match env::var_os("CARGO_TARGET_DIR").filter(|t| !t.is_empty()) {
        // (relative to where rapidr was run, as cargo takes it)
        Some(t) => env::current_dir().map(|c| c.join(&t)).unwrap_or_else(|_| PathBuf::from(t)),
        None => dir.join("target"),
    };
    Ok(Cache { dir, target })
}

/// `2.117.0-<10 hex digits of the home's path>`.
fn key(version: &str, home: &Path) -> String {
    format!("{version}-{}", &hex_digest(home.to_string_lossy().as_bytes())[..10])
}

fn hex_digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

/// Other keys' folders no build used for [`UNUSED_FOR`]: removed. Only
/// folders RapidR made (with their stamp) are touched.
fn prune(root: &Path, keep: &Path) {
    let now = SystemTime::now();
    for entry in fs::read_dir(root).into_iter().flatten().flatten() {
        let dir = entry.path();
        if dir == keep || !dir.is_dir() {
            continue;
        }
        let Ok(used) = fs::metadata(dir.join(STAMP)).and_then(|m| m.modified()) else { continue };
        if now.duration_since(used).is_ok_and(|age| age > UNUSED_FOR) {
            let _ = fs::remove_dir_all(&dir);
        }
    }
}

impl Cache {
    /// Where the Rust for the program `source` (package `name`) is
    /// generated: `src/<name>-<8 hex digits of its path>`.
    pub fn project_dir(&self, source: &Path, name: &str) -> PathBuf {
        let path = crate::home::canonical(source).unwrap_or_else(|_| source.to_path_buf());
        self.dir.join("src").join(format!("{name}-{}", &hex_digest(path.to_string_lossy().as_bytes())[..8]))
    }
}

/// Writes `bytes` to `path` unless it holds them already (cargo then sees
/// no change).
pub fn write_if_changed(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if fs::read(path).is_ok_and(|old| old == bytes) {
        return Ok(());
    }
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(path, bytes)
}

/// What cargo built for the program's own package, from its JSON messages
/// (`--message-format=json-render-diagnostics`): every file it wrote and
/// the executables (one per target: a universal macOS build has two).
#[derive(Default, Debug)]
pub struct Artifacts {
    pub files: Vec<PathBuf>,
    pub executables: Vec<PathBuf>,
}

impl Artifacts {
    /// Takes one line of cargo's standard output: a `compiler-artifact`
    /// message for the package whose manifest is `manifest`.
    pub fn take(&mut self, line: &str, manifest: &Path) {
        let Ok(msg) = serde_json::from_str::<serde_json::Value>(line) else { return };
        if msg["reason"] != "compiler-artifact" {
            return;
        }
        let ours = msg["manifest_path"].as_str().is_some_and(|m| same_file(Path::new(m), manifest));
        if !ours {
            return;
        }
        for f in msg["filenames"].as_array().into_iter().flatten().filter_map(|f| f.as_str()) {
            self.files.push(PathBuf::from(f));
        }
        if let Some(exe) = msg["executable"].as_str() {
            self.executables.push(PathBuf::from(exe));
        }
    }

    /// The program's own files removed from the target folder: what cargo
    /// reported, the original each was uplifted from (`deps/<name>-<hash>`,
    /// the same file: a hard link) with its `.d` and `.pdb`, and its
    /// fingerprint. The runtime and the other dependencies stay built — a
    /// dependency named as the program (`log.bas`) too: only the files
    /// with the program's own hash go.
    pub fn remove(&self) {
        let mut gone: Vec<PathBuf> = Vec::new();
        for f in self.files.iter().chain(&self.executables) {
            gone.push(f.clone());
            gone.push(f.with_extension("d"));
            let (Some(profile_dir), Ok(meta)) = (f.parent(), fs::metadata(f)) else { continue };
            let stem = f.file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_string();
            let deps = profile_dir.join("deps");
            // (the original: the same length and time as the uplifted copy)
            let hash = fs::read_dir(&deps).into_iter().flatten().flatten().find_map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                let m = e.metadata().ok()?;
                let same = is_hashed(&name, &stem) && m.is_file() && m.len() == meta.len() && m.modified().ok() == meta.modified().ok();
                same.then(|| name[stem.len() + 1..stem.len() + 17].to_string())
            });
            let Some(hash) = hash else { continue };
            for e in fs::read_dir(&deps).into_iter().flatten().flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                if name == format!("{stem}-{hash}") || name.starts_with(&format!("{stem}-{hash}.")) {
                    gone.push(e.path());
                }
            }
            for pkg in [stem.clone(), stem.replace('_', "-")] {
                gone.push(profile_dir.join(".fingerprint").join(format!("{pkg}-{hash}")));
            }
        }
        gone.sort();
        gone.dedup();
        for p in gone {
            if p.is_dir() {
                let _ = fs::remove_dir_all(&p);
            } else {
                let _ = fs::remove_file(&p);
            }
        }
    }
}

/// `name` is `<stem>-<16 hex digits>` (with an extension or not): one of
/// cargo's files for exactly that package, not one whose name starts the
/// same (`log` and `log-derive`).
fn is_hashed(name: &str, stem: &str) -> bool {
    let Some(rest) = name.strip_prefix(stem).and_then(|r| r.strip_prefix('-')) else { return false };
    let hash = rest.split('.').next().unwrap_or(rest);
    hash.len() == 16 && hash.chars().all(|c| c.is_ascii_hexdigit())
}

fn same_file(a: &Path, b: &Path) -> bool {
    a == b || matches!((fs::canonicalize(a), fs::canonicalize(b)), (Ok(x), Ok(y)) if x == y)
}

/// The generated Rust project copied to `to` (made again: what was there
/// before goes), without cargo's `target` folder.
pub fn copy_project(from: &Path, to: &Path) -> std::io::Result<()> {
    if to.exists() {
        fs::remove_dir_all(to)?;
    }
    copy_dir(from, to)
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for e in fs::read_dir(from)?.flatten() {
        let name = e.file_name();
        let p = e.path();
        if p.is_dir() {
            if name != "target" {
                copy_dir(&p, &to.join(&name))?;
            }
        } else {
            fs::copy(&p, to.join(&name))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_and_hashed_names() {
        let k = key("2.117.0", Path::new("/opt/rapidr/lib/rapidr"));
        assert!(k.starts_with("2.117.0-") && k.len() == "2.117.0-".len() + 10, "{k}");
        assert_ne!(k, key("2.117.0", Path::new("/other")));
        assert!(is_hashed("notepad-0123456789abcdef", "notepad"));
        assert!(is_hashed("notepad-0123456789abcdef.d", "notepad"));
        assert!(is_hashed("libnotepad-0123456789abcdef.rlib", "libnotepad"));
        assert!(!is_hashed("notepad_helper-0123456789abcdef", "notepad"));
        assert!(!is_hashed("log-derive-0123456789abcdef", "log"), "another package that starts the same");
        assert!(!is_hashed("notepad-0123", "notepad"));
    }

    #[test]
    fn a_programs_artifacts_go_and_the_rest_stays() {
        let dir = env::temp_dir().join(format!("rapidr-cache-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let release = dir.join("target/release");
        fs::create_dir_all(release.join("deps")).unwrap();
        fs::create_dir_all(release.join(".fingerprint/notepad-0123456789abcdef")).unwrap();
        fs::create_dir_all(release.join(".fingerprint/log-fedcba9876543210")).unwrap();
        for f in ["deps/notepad-0123456789abcdef.d", "deps/liblog-fedcba9876543210.rlib", "deps/notepad-fedcba9876543210", "notepad.d"] {
            fs::write(release.join(f), "x").unwrap();
        }
        // (the executable: deps' original and the uplifted hard link; a
        // dependency also called notepad, with another hash, stays)
        fs::write(release.join("deps/notepad-0123456789abcdef"), "the program").unwrap();
        fs::hard_link(release.join("deps/notepad-0123456789abcdef"), release.join("notepad")).unwrap();
        let manifest = dir.join("src/Cargo.toml");
        fs::create_dir_all(manifest.parent().unwrap()).unwrap();
        fs::write(&manifest, "").unwrap();
        let mut a = Artifacts::default();
        let msg = serde_json::json!({
            "reason": "compiler-artifact",
            "manifest_path": manifest.to_string_lossy(),
            "filenames": [release.join("notepad").to_string_lossy()],
            "executable": release.join("notepad").to_string_lossy(),
        });
        a.take(&msg.to_string(), &manifest);
        // (another package's artifact, a status line: not ours)
        a.take(&serde_json::json!({"reason": "compiler-artifact", "manifest_path": "/elsewhere/Cargo.toml", "filenames": ["/x/liblog.rlib"]}).to_string(), &manifest);
        a.take("   Compiling notepad v0.1.0", &manifest);
        assert_eq!(a.executables, [release.join("notepad")]);
        assert_eq!(a.files.len(), 1);
        a.remove();
        assert!(!release.join("notepad").exists() && !release.join("notepad.d").exists());
        assert!(!release.join("deps/notepad-0123456789abcdef").exists() && !release.join(".fingerprint/notepad-0123456789abcdef").exists());
        assert!(!release.join("deps/notepad-0123456789abcdef.d").exists());
        assert!(release.join("deps/liblog-fedcba9876543210.rlib").exists() && release.join(".fingerprint/log-fedcba9876543210").exists(), "the dependencies stay");
        assert!(release.join("deps/notepad-fedcba9876543210").exists(), "another hash: not the program's");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn old_keys_are_pruned() {
        let dir = env::temp_dir().join(format!("rapidr-cache-prune-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        for k in ["old", "new", "foreign"] {
            fs::create_dir_all(dir.join(k)).unwrap();
        }
        fs::write(dir.join("old").join(STAMP), "").unwrap();
        fs::write(dir.join("new").join(STAMP), "").unwrap();
        let long_ago = SystemTime::now() - UNUSED_FOR - Duration::from_secs(60);
        fs::File::options().write(true).open(dir.join("old").join(STAMP)).unwrap().set_modified(long_ago).unwrap();
        prune(&dir, &dir.join("new"));
        assert!(!dir.join("old").exists(), "unused for 30 days");
        assert!(dir.join("new").exists());
        assert!(dir.join("foreign").exists(), "a folder without RapidR's stamp is never touched");
        fs::remove_dir_all(&dir).unwrap();
    }
}

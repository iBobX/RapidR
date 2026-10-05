//! RapidR's home: where its own files are — the runtime crates and the
//! Cargo.lock native builds compile against, the runner stubs `--interp`
//! executables start from, the web interpreter `bundle-bc` packs.
//!
//! One rule, in this order:
//!
//! 1. `RAPIDR_HOME`: that directory (an install's home or a checkout).
//! 2. An install: `<the rapidr executable's folder>/../lib/rapidr`, which has
//!    `release.toml` (`tools/release/assemble.py` makes it).
//! 3. A source checkout: the RapidR workspace above the current directory,
//!    the `rapidr` executable, or where this CLI was compiled.
//!
//! An install's home is laid out like a checkout's root for what builds read
//! (`Cargo.toml`, `Cargo.lock`, `crates/`, `.cargo/config.toml`,
//! `tools/wasm-ar.sh`) and has what a checkout builds itself instead:
//! `runners/<os>-<arch>/rapidrintr-runner`, `web/rapidrintr*` and `vendor/`
//! (the crates.io sources: native builds work offline), `fonts/` (the web's
//! fallback-font chunks, which web builds copy: never downloaded), and on Windows
//! `toolchain/` (LLVM-MinGW: native builds link with it, no Visual Studio).
//! A runtime-only install has only `release.toml`.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub struct Home {
    pub root: PathBuf,
    /// An install's `release.toml`; `None` in a checkout.
    pub release: Option<Release>,
}

/// An install's `release.toml`.
pub struct Release {
    /// RapidR's version it was built from.
    pub version: String,
    /// The Rust it was tested with: native builds need it or newer.
    pub rust: String,
    /// `sdk` (the runtime sources, runners, web interpreter) or `runtime`
    /// (runs programs, builds none).
    pub kind: String,
}

impl Home {
    pub fn find() -> Option<Home> {
        let root = env::var_os("RAPIDR_HOME")
            .map(PathBuf::from)
            .or_else(installed_root)
            .or_else(checkout_root)?;
        let release = Release::read(&root.join("release.toml"));
        Some(Home { root, release })
    }

    /// Native and web builds can compile here: a checkout, or an install
    /// with the runtime's sources.
    pub fn can_build(&self) -> bool {
        self.release.as_ref().is_none_or(|r| r.kind == "sdk")
    }

    /// Where `--interp` executables for `target` (`<os>-<arch>`) start
    /// from in an install: the runner `name` (`rapidrintr-runner`, or
    /// Windows' windowed `rapidrintr-runnerw`).
    pub fn runner(&self, target: &str, name: &str) -> PathBuf {
        self.root.join("runners").join(target).join(format!("{name}{}", exe_suffix(target)))
    }

    /// The runner targets an install ships.
    pub fn runner_targets(&self) -> Vec<String> {
        let mut out: Vec<String> = fs::read_dir(self.root.join("runners"))
            .map(|d| d.flatten().filter(|e| e.path().is_dir()).map(|e| e.file_name().to_string_lossy().into_owned()).collect())
            .unwrap_or_default();
        out.sort();
        out
    }
}

impl Home {
    /// Windows: the LLVM-MinGW an install ships (`toolchain/`) that native
    /// builds link with, unless RAPIDR_TOOLCHAIN=msvc.
    pub fn windows_toolchain(&self) -> Option<PathBuf> {
        if !cfg!(windows) || env::var("RAPIDR_TOOLCHAIN").is_ok_and(|t| t == "msvc") {
            return None;
        }
        let tc = self.root.join("toolchain");
        tc.join("bin").is_dir().then_some(tc)
    }
}

/// Rust's gnullvm triple for this Windows machine.
pub fn windows_gnullvm_triple() -> String {
    format!("{}-pc-windows-gnullvm", env::consts::ARCH)
}

impl Release {
    fn read(path: &Path) -> Option<Release> {
        let text = fs::read_to_string(path).ok()?;
        let get = |key: &str| {
            text.lines().find_map(|l| {
                let (k, v) = l.split_once('=')?;
                (k.trim() == key).then(|| v.trim().trim_matches('"').to_string())
            })
        };
        Some(Release { version: get("version")?, rust: get("rust").unwrap_or_default(), kind: get("kind").unwrap_or_else(|| "sdk".into()) })
    }
}

/// This machine as runner targets name it: `macos-aarch64`, `windows-x86_64`, …
pub fn host_target() -> String {
    format!("{}-{}", env::consts::OS, env::consts::ARCH)
}

/// `.exe` for Windows targets.
pub fn exe_suffix(target: &str) -> &'static str {
    if target.starts_with("windows-") {
        ".exe"
    } else {
        ""
    }
}

/// `<exe folder>/../lib/rapidr` when it is an install's home.
fn installed_root() -> Option<PathBuf> {
    let exe = plain(env::current_exe().ok()?.canonicalize().ok()?);
    let home = exe.parent()?.parent()?.join("lib").join("rapidr");
    home.join("release.toml").is_file().then_some(home)
}

/// `fs::canonicalize`, as people and tools read paths (Application.ExeName,
/// cargo's configuration): see [`plain`].
pub fn canonical(path: impl AsRef<Path>) -> std::io::Result<PathBuf> {
    fs::canonicalize(path).map(plain)
}

/// A canonical path as tools read it: on Windows without the `\\?\` prefix
/// `canonicalize` gives (cargo's configuration, clang and the linker don't
/// take it).
fn plain(path: PathBuf) -> PathBuf {
    match path.to_str().and_then(|p| p.strip_prefix(r"\\?\")) {
        Some(rest) if rest.as_bytes().get(1) == Some(&b':') => PathBuf::from(rest),
        _ => path,
    }
}

/// The RapidR workspace (the Cargo.toml with `[workspace]` that has
/// crates/rapidr-runtime-core) nearest above the current directory, the
/// `rapidr` executable, or where this CLI was compiled.
fn checkout_root() -> Option<PathBuf> {
    let is_root = |dir: &Path| {
        dir.join("crates/rapidr-runtime-core").is_dir()
            && fs::read_to_string(dir.join("Cargo.toml")).is_ok_and(|c| c.contains("[workspace]"))
    };
    let above = |start: PathBuf| start.ancestors().find(|d| is_root(d)).map(Path::to_path_buf);
    env::current_dir()
        .ok()
        .and_then(above)
        .or_else(|| env::current_exe().ok().and_then(|e| e.canonicalize().ok()).map(plain).and_then(above))
        .or_else(|| above(PathBuf::from(env!("CARGO_MANIFEST_DIR"))))
}

/// A tool rustup installs (`cargo`, `rustc`, `rustup`): on PATH, else in
/// rustup's folder (`CARGO_HOME/bin`, `~/.cargo/bin`) — where it is right
/// after `rapidr setup`, and for programs started from the desktop, which
/// don't get a shell's PATH.
pub fn rust_tool(name: &str) -> PathBuf {
    let file = format!("{name}{}", env::consts::EXE_SUFFIX);
    let on_path = env::var_os("PATH").and_then(|p| env::split_paths(&p).map(|d| d.join(&file)).find(|f| f.is_file()));
    on_path
        .or_else(|| cargo_home().map(|h| h.join("bin").join(&file)).filter(|f| f.is_file()))
        .unwrap_or_else(|| PathBuf::from(name))
}

/// `CARGO_HOME`, else `~/.cargo`.
pub fn cargo_home() -> Option<PathBuf> {
    env::var_os("CARGO_HOME").map(PathBuf::from).or_else(|| dirs::home_dir().map(|h| h.join(".cargo")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_toml_is_read() {
        let dir = env::temp_dir().join(format!("rapidr-home-test-{}", std::process::id()));
        fs::create_dir_all(dir.join("runners/linux-x86_64")).unwrap();
        fs::create_dir_all(dir.join("runners/windows-aarch64")).unwrap();
        fs::write(dir.join("release.toml"), "# made by assemble.py\nversion = \"2.116.0\"\nrust = \"1.98.1\"\nkind = \"sdk\"\n").unwrap();
        let r = Release::read(&dir.join("release.toml")).unwrap();
        assert_eq!((r.version.as_str(), r.rust.as_str(), r.kind.as_str()), ("2.116.0", "1.98.1", "sdk"));
        let home = Home { root: dir.clone(), release: Some(r) };
        assert!(home.can_build());
        assert_eq!(home.runner_targets(), ["linux-x86_64", "windows-aarch64"]);
        assert!(home.runner("windows-aarch64", "rapidrintr-runnerw").ends_with("runners/windows-aarch64/rapidrintr-runnerw.exe"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn verbatim_windows_paths_are_made_plain() {
        assert_eq!(plain(PathBuf::from(r"\\?\C:\Users\me\RapidR")), PathBuf::from(r"C:\Users\me\RapidR"));
        assert_eq!(plain(PathBuf::from(r"\\?\UNC\server\share")), PathBuf::from(r"\\?\UNC\server\share"));
        assert_eq!(plain(PathBuf::from("/usr/lib/rapidr")), PathBuf::from("/usr/lib/rapidr"));
    }

    #[test]
    fn a_checkout_is_found_from_here() {
        let home = Home { root: checkout_root().unwrap(), release: None };
        assert!(home.root.join("crates/rapidr-runtime-core").is_dir());
        assert!(home.can_build());
    }
}

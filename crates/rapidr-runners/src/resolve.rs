//! `runner_bytes`: the runner for a target, from the first place that has it.
//!
//! 1. **This install.** `<install>/runners/<target>/…` (and `web/`), shipped
//!    inside the signed installer: used as it is.
//! 2. **The cache.** A runner downloaded before. Read back through the same
//!    checks as a download (the list's signature, this version, the file's
//!    size and SHA-256), so a damaged or swapped file is removed, not used.
//! 3. **The host.** The list and its signature, then the runner, each
//!    checked before anything is kept; the checked files go to the cache.
//!
//! Offline with the runner in 1 or 2 works; offline without says which
//! runner is missing and what to do. Nothing the network brings is used
//! unless it passes verify.rs.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::bundle::Bundle;
use crate::net::{Fetch, FetchError, UreqFetch};
use crate::store;
use crate::urls;
use crate::verify::{self, Refusal, Trust};
use crate::{names, NOTICES_FILE};

/// What `runner_bytes` needs to know. The caller reads its own settings and
/// environment (the CLI: `rapidr runners`, the Studio: Settings) and puts the
/// answers here; nothing in this crate reads either.
#[derive(Clone, Debug)]
pub struct Config {
    /// The key a list must be signed with, and RapidR's version.
    pub trust: Trust,
    /// This RapidR install's home (`<…>/lib/rapidr`: it has `runners/`,
    /// `notices/`, `web/`), if it is an install.
    pub install: Option<PathBuf>,
    /// Where downloaded runners are kept (`…/RapidR/runners`; each version in
    /// its own folder below). `None`: nothing is kept.
    pub cache: Option<PathBuf>,
    /// Where runners are downloaded from ([`urls`]). `None`: nowhere.
    pub base_url: Option<String>,
    /// Never use the network (`RAPIDR_RUNNERS_OFFLINE`): only the install and the cache.
    pub offline: bool,
}

/// Where a runner came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// This install's own.
    Install,
    /// A runner downloaded earlier (and checked again now).
    Cache,
    /// Downloaded now, and kept.
    Download,
}

/// A runner found.
#[derive(Clone, Debug)]
pub struct Resolved {
    pub bundle: Bundle,
    pub source: Source,
}

/// What is happening, for a progress line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    /// Getting the list of runners (and its signature).
    List,
    /// Getting the runner file.
    Download,
}

/// A step of a download, with how far it is.
#[derive(Clone, Debug)]
pub struct Progress {
    pub target: String,
    pub step: Step,
    pub done: u64,
    pub total: u64,
}

/// Why there is no runner. `Display` is the sentence to show.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// `name` is not a target RapidR builds for.
    UnknownTarget(String),
    /// The runner is not in the install or the cache and the network did not
    /// give it. Nothing wrong was downloaded.
    NotAvailable { target: String, why: Why },
    /// Something came from the host and was not used: it did not pass
    /// verify.rs.
    Refused(Refusal),
    /// The runner is not a program (`web` has none).
    NoProgram(String),
    /// The runner's files lack the program asked for.
    Incomplete { target: String, file: String },
}

/// Why the network did not give the runner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Why {
    /// Told not to use it (`RAPIDR_RUNNERS_OFFLINE`).
    OfflineSetting,
    /// No address to download from is set.
    NoAddress,
    /// The address is not one runners are taken from.
    BadAddress(String),
    /// The host could not be reached.
    Unreachable { host: String, detail: String },
    /// The host has no list of runners for this version.
    NotPublished { url: String },
    /// The host answered with an error.
    Server { url: String, status: u16 },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::UnknownTarget(name) => write!(
                f,
                "\"{name}\" is not a system RapidR builds apps for. It builds for: {}.",
                names::TARGETS.iter().map(|t| names::describe(t)).collect::<Vec<_>>().join(", ")
            ),
            Error::Refused(r) => write!(f, "{r}"),
            Error::NoProgram(_) => f.write_str("The web (browser) runner has no program to start an app from: its files are the web interpreter."),
            Error::Incomplete { target, file } => write!(f, "The {} runner is incomplete: it lacks {file}.", names::describe(target)),
            Error::NotAvailable { target, why } => {
                let what = names::describe(target);
                let reason = match why {
                    Why::OfflineSetting => "RapidR is set to work offline (RAPIDR_RUNNERS_OFFLINE).".to_string(),
                    Why::NoAddress => "RapidR has no address to download runners from. Set RAPIDR_RUNNERS_URL to one (https://…).".to_string(),
                    Why::BadAddress(why) => format!("The address runners are downloaded from can't be used: {why}."),
                    Why::Unreachable { host, detail } => format!("RapidR can't reach {host} ({detail})."),
                    Why::NotPublished { url } => format!("The host has no runners for this version of RapidR ({url} was not found)."),
                    Why::Server { url, status } => format!("The host answered with an error ({status} for {url})."),
                };
                let retry = match why {
                    Why::Unreachable { .. } => " Connect to the internet once and build again: the runner is kept after it is downloaded, and later builds for it work offline.",
                    Why::NotPublished { .. } => " They may not be published yet: try again later, or use a RapidR that has them.",
                    Why::Server { .. } => " Try again in a little while.",
                    _ => "",
                };
                write!(f, "The {what} runner is not on this computer: it is not part of this install, and it has not been downloaded yet. {reason}{retry}")
            }
        }
    }
}

impl std::error::Error for Error {}

impl Error {
    /// The runner is simply not here (no network, no address): as against a
    /// runner that was refused, or a target that does not exist.
    pub fn is_unavailable(&self) -> bool {
        matches!(self, Error::NotAvailable { .. })
    }
}

/// The runner `target` needs, from the install, the cache or the host.
/// `target` may be any name [`names::canonical`] knows. `progress` is called
/// while a download runs.
pub fn resolve(cfg: &Config, target: &str, progress: &mut dyn FnMut(&Progress)) -> Result<Resolved, Error> {
    resolve_with(cfg, target, &UreqFetch::new(), progress)
}

/// [`resolve`] with another way to GET (the tests').
pub fn resolve_with(cfg: &Config, target: &str, fetch: &dyn Fetch, progress: &mut dyn FnMut(&Progress)) -> Result<Resolved, Error> {
    let target = names::canonical(target).ok_or_else(|| Error::UnknownTarget(target.to_string()))?;
    if let Some(bundle) = cfg.install.as_deref().and_then(|root| from_install(root, target)) {
        return Ok(Resolved { bundle, source: Source::Install });
    }
    if let Some(bundle) = from_cache(cfg, target) {
        return Ok(Resolved { bundle, source: Source::Cache });
    }
    let bundle = download(cfg, target, fetch, progress)?;
    Ok(Resolved { bundle, source: Source::Download })
}

/// The runner program for `target`, ready to have an app's program appended
/// (names.rs: one per target; Windows' subsystem, console or windowed, is the
/// packer's to set, so `windowed` changes nothing). The one call a build makes.
pub fn runner_bytes(cfg: &Config, target: &str, windowed: bool, progress: &mut dyn FnMut(&Progress)) -> Result<Vec<u8>, Error> {
    let resolved = resolve(cfg, target, progress)?;
    take_runner(resolved.bundle, windowed)
}

/// [`runner_bytes`] with another way to GET (the tests').
pub fn runner_bytes_with(cfg: &Config, target: &str, windowed: bool, fetch: &dyn Fetch, progress: &mut dyn FnMut(&Progress)) -> Result<Vec<u8>, Error> {
    let resolved = resolve_with(cfg, target, fetch, progress)?;
    take_runner(resolved.bundle, windowed)
}

fn take_runner(bundle: Bundle, windowed: bool) -> Result<Vec<u8>, Error> {
    if bundle.target() == "web" {
        return Err(Error::NoProgram("web".to_string()));
    }
    match bundle.runner(windowed) {
        Some(bytes) => Ok(bytes.to_vec()),
        // (defensive: every source checks for it before it gives a bundle)
        None => Err(Error::Incomplete { target: bundle.target().to_string(), file: names::runner_file(bundle.target(), windowed).to_string() }),
    }
}

// ---------------------------------------------------------------- 1. the install

/// `target`'s files from an install's home, if it ships them: the runner
/// program(s) (the web: the interpreter and its fonts), and the notices.
pub fn from_install(root: &Path, target: &str) -> Option<Bundle> {
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    if target == "web" {
        for name in ["rapidrintr.js", "rapidrintr_bg.wasm"] {
            files.push((name.to_string(), fs::read(root.join("web").join(name)).ok()?));
        }
        collect(&root.join("web").join("fonts"), "fonts", &mut files, 0);
    } else {
        let dir = root.join("runners").join(target);
        for name in names::required_files(target).into_iter().filter(|n| *n != NOTICES_FILE) {
            files.push((name.to_string(), fs::read(dir.join(name)).ok()?));
        }
    }
    // (the notices are the install's own file, named by the target; an older install may lack them)
    if let Ok(notices) = fs::read(root.join("notices").join(format!("{target}.txt"))) {
        files.push((NOTICES_FILE.to_string(), notices));
    }
    Some(Bundle::new(target, files))
}

fn collect(dir: &Path, prefix: &str, out: &mut Vec<(String, Vec<u8>)>, depth: usize) {
    if depth > 3 {
        return;
    }
    let Ok(read) = fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = read.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let name = format!("{prefix}/{}", e.file_name().to_string_lossy());
        let Ok(kind) = e.file_type() else { continue };
        if kind.is_file() {
            if let Ok(bytes) = fs::read(e.path()) {
                out.push((name, bytes));
            }
        } else if kind.is_dir() {
            collect(&e.path(), &name, out, depth + 1);
        }
    }
}

// ------------------------------------------------------------------ 2. the cache

fn cache_dir(cfg: &Config) -> Option<PathBuf> {
    let v = &cfg.trust.version;
    // (the version is a folder name: nothing that could point elsewhere)
    let plain = !v.is_empty() && v.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+'));
    cfg.cache.as_deref().filter(|_| plain).map(|c| store::version_dir(c, v))
}

fn from_cache(cfg: &Config, target: &str) -> Option<Bundle> {
    let dir = cache_dir(cfg)?;
    let (manifest_json, signature) = store::read_list(&dir)?;
    let manifest = match verify::check_list(&cfg.trust, &manifest_json, &signature) {
        Ok(m) => m,
        Err(_) => {
            // (changed since it was kept, or kept by another RapidR: not used, not kept)
            store::remove_list(&dir);
            return None;
        }
    };
    let entry = manifest.entry(target)?;
    let path = dir.join(&entry.file);
    let bytes = fs::read(&path).ok()?;
    let checked = verify::check_file(&manifest, target, &bytes).and_then(|entry| verify::unpack_checked(&manifest, entry, &bytes));
    match checked {
        Ok(bundle) => Some(bundle),
        Err(_) => {
            let _ = fs::remove_file(&path);
            None
        }
    }
}

// ----------------------------------------------------------------- 3. the host

fn download(cfg: &Config, target: &str, fetch: &dyn Fetch, progress: &mut dyn FnMut(&Progress)) -> Result<Bundle, Error> {
    let unavailable = |why: Why| Error::NotAvailable { target: target.to_string(), why };
    if cfg.offline {
        return Err(unavailable(Why::OfflineSetting));
    }
    let base = cfg.base_url.as_deref().filter(|b| !b.trim().is_empty()).ok_or_else(|| unavailable(Why::NoAddress))?;
    let base = urls::check_base_url(base).map_err(|why| unavailable(Why::BadAddress(why)))?;
    let version = cfg.trust.version.as_str();
    let say = |step: Step| move |done: u64, total: u64| Progress { target: target.to_string(), step: step.clone(), done, total };
    let on_net = |url: &str, e: FetchError| match e {
        FetchError::NotFound => unavailable(Why::NotPublished { url: url.to_string() }),
        FetchError::Status(status) => unavailable(Why::Server { url: url.to_string(), status }),
        FetchError::Unreachable(detail) => unavailable(Why::Unreachable { host: urls::host_of(&base), detail }),
        FetchError::TooBig => Error::Refused(Refusal::ListUnreadable { version: version.to_string(), why: format!("{url} is bigger than a list of runners can be") }),
    };

    // the list and its signature (small), checked before anything it names is fetched
    let list_progress = say(Step::List);
    let manifest_url = urls::manifest_url(&base, version);
    let manifest_json = fetch.get(&manifest_url, 1 << 20, &mut |d, t| progress(&list_progress(d, t))).map_err(|e| on_net(&manifest_url, e))?;
    let signature_url = urls::signature_url(&base, version);
    let signature = fetch.get(&signature_url, 16 << 10, &mut |_, _| {}).map_err(|e| on_net(&signature_url, e))?;
    let signature = String::from_utf8_lossy(&signature).into_owned();
    let manifest = verify::check_list(&cfg.trust, &manifest_json, &signature).map_err(Error::Refused)?;
    let entry = manifest.entry(target).ok_or_else(|| Error::Refused(Refusal::NotListed { version: version.to_string(), target: target.to_string() }))?;

    // the runner itself: no more bytes than the list says it has
    let file_url = urls::file_url(&base, version, &entry.file);
    let download_progress = say(Step::Download);
    let bytes = match fetch.get(&file_url, entry.size, &mut |d, t| progress(&download_progress(d, t))) {
        Ok(b) => b,
        Err(FetchError::TooBig) => return Err(Error::Refused(Refusal::DoesNotMatch { version: version.to_string(), target: target.to_string() })),
        Err(e) => return Err(on_net(&file_url, e)),
    };
    let entry = verify::check_file(&manifest, target, &bytes).map_err(Error::Refused)?;
    let bundle = verify::unpack_checked(&manifest, entry, &bytes).map_err(Error::Refused)?;

    // kept only now that it passed (a cache that can't be written to is not an error)
    if let (Some(dir), Some(root)) = (cache_dir(cfg), cfg.cache.as_deref()) {
        let kept = store::write_list(&dir, &manifest_json, &signature).and_then(|()| store::write_atomic(&dir.join(&entry.file), &bytes));
        if kept.is_ok() {
            store::prune_other_versions(root, version, Duration::from_secs(30 * 24 * 60 * 60));
        }
    }
    Ok(bundle)
}

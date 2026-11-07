//! Runners for other systems: `rapidr runners`, and the one call a build makes
//! for a target's runner (docs/manual/build-runners.md, "Where runners come
//! from"; the library is `rapidr-runners`).
//!
//!   rapidr runners                    which runners are in this install, which
//!                                     were downloaded, where they come from
//!   rapidr runners fetch <target>… | --all
//!                                     get them now (so a later build works offline)
//!   rapidr runners clear              remove every downloaded runner
//!
//! The settings (this module reads them; the library reads none):
//!
//! - `RAPIDR_RUNNERS_URL`, else the first line of `runners-url.txt` in
//!   RapidR's settings folder, else the address built into RapidR
//!   (`rapidr_runners::urls::DEFAULT_BASE_URL`, https://studio.rapidr.dev/runners):
//!   where runners are downloaded from. Only `https://` (and `http://` to this
//!   computer, for a test server). Tests always set it: nothing a test runs
//!   reaches the real host.
//! - `RAPIDR_RUNNERS_OFFLINE=1`: never use the network, only the install and the cache.
//! - `RAPIDR_RUNNERS_CACHE`: where downloaded runners are kept (default: the
//!   build cache's `runners` folder, cache.rs).
//! - `RAPIDR_TEST_RUNNERS_KEY` (a test hook, only in development builds or
//!   with a test-mode token): another minisign public key to trust, for tests
//!   that serve their own signed list.

use std::env;
use std::fs;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::ExitCode;

use rapidr_runners::minisign::PublicKey;
use rapidr_runners::names;
use rapidr_runners::resolve::{self, Config, Error, Progress, Source, Step};
use rapidr_runners::store;
use rapidr_runners::urls;
use rapidr_runners::verify::Trust;

use crate::cache;
use crate::home::{self, Home};

/// The file in RapidR's settings folder that holds the download address.
const URL_FILE: &str = "runners-url.txt";

/// What this run knows: the install, the cache, the address, the key.
pub fn config() -> Config {
    let install = Home::find().map(|h| h.root);
    let cache = env::var_os("RAPIDR_RUNNERS_CACHE")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| cache::root().map(|r| r.join("runners")));
    let mut trust = Trust::release(rapidr_runners::VERSION);
    // (a development build, or a test-mode token: a made-up key, never a release's)
    if let Some(key) = rapidr_testmode::var("RAPIDR_TEST_RUNNERS_KEY").and_then(|k| PublicKey::parse(&k).ok()) {
        trust.key = key;
    }
    Config { trust, install, cache, base_url: base_url(), offline: truthy(env::var("RAPIDR_RUNNERS_OFFLINE").ok().as_deref()) }
}

fn truthy(value: Option<&str>) -> bool {
    value.is_some_and(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
}

/// The address runners are downloaded from, if any is set.
fn base_url() -> Option<String> {
    if let Some(v) = env::var("RAPIDR_RUNNERS_URL").ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty()) {
        return Some(v);
    }
    settings_dir()
        .and_then(|d| fs::read_to_string(d.join(URL_FILE)).ok())
        .and_then(|text| first_setting(&text))
        .or_else(|| Some(urls::DEFAULT_BASE_URL.to_string()).filter(|u| !u.is_empty()))
}

/// RapidR's settings folder, as launch.rs finds it (`RAPIDR_CONFIG_DIR` first).
fn settings_dir() -> Option<PathBuf> {
    env::var_os("RAPIDR_CONFIG_DIR").map(PathBuf::from).or_else(|| home::config_dir().map(|d| d.join(if cfg!(target_os = "linux") { "rapidr" } else { "RapidR" })))
}

/// The first line of a settings file that is not empty or a `#` comment.
fn first_setting(text: &str) -> Option<String> {
    text.lines().map(str::trim).find(|l| !l.is_empty() && !l.starts_with('#')).map(str::to_string)
}

/// The runner program `target` needs for an app (the windowed one on Windows
/// when `windowed`), from this install, the cache or a download that passed
/// its checks. The one call a build makes; a download shows its progress on
/// standard error. The error is a sentence to show as it is.
// (the build's call site — BUILD-CORE's `runner_bytes` — is not merged yet)
#[allow(dead_code)]
pub fn runner_bytes(target: &str, windowed: bool) -> Result<Vec<u8>, String> {
    let mut printer = Printer::new();
    resolve::runner_bytes(&config(), target, windowed, &mut |p| printer.show(p)).map_err(|e| e.to_string())
}

/// All of `target`'s files (the runner(s), the notices; the web interpreter
/// and fonts for `web`), and where they came from.
#[allow(dead_code)]
pub fn bundle(target: &str) -> Result<(rapidr_runners::Bundle, Source), String> {
    let mut printer = Printer::new();
    resolve::resolve(&config(), target, &mut |p| printer.show(p)).map(|r| (r.bundle, r.source)).map_err(|e| e.to_string())
}

// -------------------------------------------------------------------- progress

/// Progress on standard error: one line that counts up on a terminal, a
/// start and an end line in a log.
struct Printer {
    tty: bool,
    said_list: bool,
    started: Option<String>,
}

impl Printer {
    fn new() -> Printer {
        Printer { tty: std::io::stderr().is_terminal(), said_list: false, started: None }
    }

    fn show(&mut self, p: &Progress) {
        let what = names::describe(&p.target);
        match p.step {
            Step::List => {
                if !self.said_list {
                    self.said_list = true;
                    eprintln!("Getting RapidR {}'s list of runners…", rapidr_runners::VERSION);
                }
            }
            Step::Download => {
                let key = format!("download {}", p.target);
                if self.started.as_deref() != Some(key.as_str()) {
                    self.started = Some(key);
                    eprint!("Downloading the {what} runner ({})…", megabytes(p.total));
                    if !self.tty {
                        eprintln!();
                    }
                }
                if self.tty {
                    let percent = if p.total == 0 { 100 } else { (p.done.min(p.total) * 100 / p.total) as u32 };
                    eprint!("\rDownloading the {what} runner ({}): {percent}%   ", megabytes(p.total));
                    if p.done >= p.total {
                        eprintln!();
                    }
                }
            }
        }
    }
}

fn megabytes(bytes: u64) -> String {
    format!("{:.1} MB", bytes as f64 / 1_048_576.0)
}

// --------------------------------------------------------------------- command

pub fn command(args: &[String]) -> ExitCode {
    match args.first().map(String::as_str) {
        None | Some("status") => status(),
        Some("fetch") => fetch(&args[1..]),
        Some("clear") => clear(),
        Some("check") => check(&args[1..]),
        _ => {
            eprintln!("rapidr runners                      Which runners are in this install, which were downloaded, where they come from");
            eprintln!("rapidr runners fetch <target>…|--all  Get runners now, so building for them later works offline");
            eprintln!("rapidr runners clear                Remove every downloaded runner");
            eprintln!("rapidr runners check <release folder>  A release's runners (<folder>/runners-manifest.json + .minisig, <folder>/runners/*.zst) as a build would check them, offline");
            eprintln!("targets: {}", names::TARGETS.join(", "));
            ExitCode::from(2)
        }
    }
}

fn status() -> ExitCode {
    let cfg = config();
    println!("Runners for RapidR {} (the programs apps for each system start from)", rapidr_runners::VERSION);
    let kept = cfg.cache.as_ref().map(|c| store::version_dir(c, rapidr_runners::VERSION));
    let files = kept.as_deref().map(store::list_files).unwrap_or_default();
    for target in names::TARGETS {
        let here = cfg.install.as_deref().and_then(|root| resolve::from_install(root, target)).is_some();
        let downloaded = files.iter().find(|(name, _)| *name == format!("{target}.zst")).map(|(_, size)| *size);
        let state = match (here, downloaded) {
            (true, _) => "in this install".to_string(),
            (false, Some(size)) => format!("downloaded ({}), checked again each time it is used", megabytes(size)),
            (false, None) => "not on this computer yet".to_string(),
        };
        println!("  {:<32} {:<16} {state}", names::describe(target), target);
    }
    match cfg.base_url.as_deref() {
        _ if cfg.offline => println!("Downloads: off (RAPIDR_RUNNERS_OFFLINE)"),
        Some(url) => println!("Downloads from: {url}"),
        None => println!("Downloads from: nowhere (the address is empty: set RAPIDR_RUNNERS_URL, or {URL_FILE} in the settings folder)"),
    }
    match &kept {
        Some(dir) => println!("Kept in: {}", dir.display()),
        None => println!("Kept in: nowhere (no cache folder: set RAPIDR_RUNNERS_CACHE)"),
    }
    ExitCode::SUCCESS
}

fn fetch(args: &[String]) -> ExitCode {
    let targets: Vec<&str> = if args.iter().any(|a| a == "--all") {
        names::TARGETS.to_vec()
    } else {
        let mut list = Vec::new();
        for a in args {
            match names::canonical(a) {
                Some(t) if !list.contains(&t) => list.push(t),
                Some(_) => {}
                None => {
                    eprintln!("rapidr runners fetch: \"{a}\" is not a system RapidR builds apps for ({})", names::TARGETS.join(", "));
                    return ExitCode::from(2);
                }
            }
        }
        list
    };
    if targets.is_empty() {
        eprintln!("rapidr runners fetch <target>…|--all   (targets: {})", names::TARGETS.join(", "));
        return ExitCode::from(2);
    }
    let cfg = config();
    let mut failed = false;
    let mut printer = Printer::new();
    for target in targets {
        match resolve::resolve(&cfg, target, &mut |p| printer.show(p)) {
            Ok(r) => {
                let from = match r.source {
                    Source::Install => "in this install already",
                    Source::Cache => "downloaded before, checked again",
                    Source::Download => "downloaded, checked and kept",
                };
                println!("{}: {from}", names::describe(target));
            }
            Err(e) => {
                failed = true;
                // (most messages name the runner already: only the others get its name in front)
                let text = e.to_string();
                if text.contains(&names::describe(target)) {
                    eprintln!("{text}");
                } else {
                    eprintln!("{}: {text}", names::describe(target));
                }
                // (nothing else will work: no address, or offline)
                if matches!(e, Error::NotAvailable { why: resolve::Why::NoAddress | resolve::Why::OfflineSetting, .. }) {
                    break;
                }
            }
        }
    }
    if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

/// `rapidr runners check <folder>`: the release gate. A release folder
/// (`dist/<ver>/out`: `runners-manifest.json`, its `.minisig`, `runners/*.zst`)
/// checked by the code that checks a download — the signature against the
/// key built into this rapidr, this rapidr's version, every file's size and
/// SHA-256, every bundle unpacked — so what tools/release/runners.py wrote
/// is what the reader reads. No network.
fn check(args: &[String]) -> ExitCode {
    let Some(dir) = args.first().map(PathBuf::from) else {
        eprintln!("rapidr runners check <release folder>");
        return ExitCode::from(2);
    };
    let trust = config().trust;
    let (manifest, signature) = match (fs::read(dir.join(urls::MANIFEST_NAME)), fs::read_to_string(dir.join(urls::SIGNATURE_NAME))) {
        (Ok(m), Ok(s)) => (m, s),
        (Err(e), _) => {
            eprintln!("{}: {e}", dir.join(urls::MANIFEST_NAME).display());
            return ExitCode::from(1);
        }
        (_, Err(e)) => {
            eprintln!("{}: {e} (finish.sh makes it with RAPIDR_MINISIGN_KEY)", dir.join(urls::SIGNATURE_NAME).display());
            return ExitCode::from(1);
        }
    };
    let list = match rapidr_runners::verify::check_list(&trust, &manifest, &signature) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
    };
    println!("The list is signed by key {} and is for RapidR {}", trust.key.id_hex(), list.version);
    let mut bad = 0;
    for target in names::TARGETS {
        if list.entry(target).is_none() {
            println!("  {:<16} NOT IN THE LIST", target);
            bad += 1;
        }
    }
    for entry in &list.runners {
        let path = dir.join("runners").join(&entry.file);
        let result = fs::read(&path).map_err(|e| format!("{}: {e}", path.display())).and_then(|bytes| {
            let checked = rapidr_runners::verify::check_file(&list, &entry.target, &bytes).map_err(|e| e.to_string())?;
            rapidr_runners::verify::unpack_checked(&list, checked, &bytes).map_err(|e| e.to_string())
        });
        match result {
            Ok(bundle) => println!("  {:<16} ok  {:>9}  {} files, {} unpacked", entry.target, megabytes(entry.size), bundle.files().len(), megabytes(entry.unpacked)),
            Err(e) => {
                bad += 1;
                println!("  {:<16} FAILED: {e}", entry.target);
            }
        }
    }
    if bad == 0 {
        println!("All {} runners pass.", list.runners.len());
        ExitCode::SUCCESS
    } else {
        eprintln!("{bad} problem(s).");
        ExitCode::from(1)
    }
}

fn clear() -> ExitCode {
    let Some(cache) = config().cache else {
        eprintln!("rapidr runners clear: no cache folder (RAPIDR_RUNNERS_CACHE)");
        return ExitCode::from(1);
    };
    // (only folders named like versions: whatever else is there stays)
    let removed = store::prune_other_versions(&cache, "", std::time::Duration::ZERO);
    println!("Removed the downloaded runners of {removed} version{} from {}", if removed == 1 { "" } else { "s" }, cache.display());
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_setting_skips_comments_and_blanks() {
        assert_eq!(first_setting("# where runners come from\n\n  https://example.org/runners \nother\n").as_deref(), Some("https://example.org/runners"));
        assert_eq!(first_setting("# nothing\n"), None);
        assert_eq!(first_setting(""), None);
    }

    #[test]
    fn offline_is_a_yes() {
        for yes in ["1", "true", "YES", " on "] {
            assert!(truthy(Some(yes)), "{yes}");
        }
        for no in ["", "0", "false", "off", "maybe"] {
            assert!(!truthy(Some(no)), "{no}");
        }
        assert!(!truthy(None));
    }

    #[test]
    fn megabytes_are_one_decimal() {
        assert_eq!(megabytes(22_020_096), "21.0 MB");
        assert_eq!(megabytes(0), "0.0 MB");
    }
}

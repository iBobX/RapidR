//! The acceptance cases for "where runners come from", against a test server
//! on this computer (127.0.0.1 only; nothing leaves it):
//!
//! - a good runner is downloaded, kept in the cache and used;
//! - a changed byte is refused with the message, and nothing is kept;
//! - a list for another version is refused;
//! - offline with the runner in the cache works;
//! - offline without it says which runner is missing;
//! - the install's own runner is used first, with no network;
//! - a list signed by another key is refused before it is read;
//! - a kept file that changed is not used.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use rapidr_runners::net::UreqFetch;
use rapidr_runners::resolve::{resolve_with, runner_bytes_with, Config, Error, Progress, Source, Step, Why};
use rapidr_runners::verify::fixture::{files_for, release, Release, Runner};
use rapidr_runners::verify::Refusal;

const VERSION: &str = "2.123.1";

/// A web server for a few files, on a port the system chose.
struct Server {
    addr: SocketAddr,
    stop: Arc<AtomicBool>,
    requests: Arc<Mutex<Vec<String>>>,
    thread: Option<JoinHandle<()>>,
}

impl Server {
    fn start(files: HashMap<String, Vec<u8>>) -> Server {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let addr = listener.local_addr().unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let (stop2, requests2) = (stop.clone(), requests.clone());
        let thread = std::thread::spawn(move || {
            while !stop2.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => answer(stream, &files, &requests2),
                    Err(_) => std::thread::sleep(Duration::from_millis(5)),
                }
            }
        });
        Server { addr, stop, requests, thread: Some(thread) }
    }

    fn url(&self) -> String {
        format!("http://{}/runners", self.addr)
    }

    fn requests(&self) -> Vec<String> {
        self.requests.lock().unwrap().clone()
    }

    fn stop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            t.join().unwrap();
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop();
    }
}

fn answer(mut stream: TcpStream, files: &HashMap<String, Vec<u8>>, requests: &Mutex<Vec<String>>) {
    stream.set_nonblocking(false).unwrap();
    stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        match stream.read(&mut byte) {
            Ok(1) => head.push(byte[0]),
            _ => return,
        }
    }
    let line = String::from_utf8_lossy(&head).lines().next().unwrap_or("").to_string();
    let path = line.split_whitespace().nth(1).unwrap_or("/").to_string();
    requests.lock().unwrap().push(path.clone());
    let (status, body): (&str, &[u8]) = match files.get(&path) {
        Some(b) => ("200 OK", b),
        None => ("404 Not Found", b"not found"),
    };
    let _ = write!(stream, "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
    let _ = stream.write_all(body);
}

/// The files a host serves for `release`, under `/runners/<version>/`.
fn hosted(r: &Release, version: &str) -> HashMap<String, Vec<u8>> {
    let mut map = HashMap::new();
    map.insert(format!("/runners/{version}/runners-manifest.json"), r.manifest.clone());
    map.insert(format!("/runners/{version}/runners-manifest.json.minisig"), r.signature.clone().into_bytes());
    for (name, bytes) in &r.files {
        map.insert(format!("/runners/{version}/{name}"), bytes.clone());
    }
    map
}

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rapidr-runners-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn config(r: &Release, cache: &Path, base: Option<String>) -> Config {
    Config { trust: r.trust.clone(), install: None, cache: Some(cache.to_path_buf()), base_url: base, offline: false }
}

fn windows_and_web() -> Release {
    release(
        VERSION,
        &[Runner { target: "windows-x86_64", files: files_for("windows-x86_64") }, Runner { target: "linux-aarch64", files: files_for("linux-aarch64") }, Runner { target: "web", files: files_for("web") }],
        3,
    )
}

fn quiet(_: &Progress) {}

#[test]
fn a_good_runner_is_downloaded_kept_and_used() {
    let r = windows_and_web();
    let server = Server::start(hosted(&r, VERSION));
    let cache = temp("good");
    let cfg = config(&r, &cache, Some(server.url()));

    let mut seen: Vec<Progress> = Vec::new();
    let got = resolve_with(&cfg, "windows-x64", &UreqFetch::new(), &mut |p| seen.push(p.clone())).unwrap();
    assert_eq!(got.source, Source::Download);
    assert_eq!(got.bundle.target(), "windows-x86_64", "the name people use is understood");
    assert_eq!(got.bundle.runner(true).unwrap(), b"windows-x86_64:rapidrintr-runner.exe:contents");
    assert_eq!(got.bundle.runner(false).unwrap(), got.bundle.runner(true).unwrap(), "one Windows program");
    // progress: the list, then the runner, ending complete
    assert!(seen.iter().any(|p| p.step == Step::List));
    let last = seen.iter().rev().find(|p| p.step == Step::Download).expect("a download step");
    assert_eq!(last.done, last.total);
    // kept: the list, its signature and the file, in the version's folder
    let kept = cache.join(VERSION);
    for f in ["runners-manifest.json", "runners-manifest.json.minisig", "windows-x86_64.zst"] {
        assert!(kept.join(f).is_file(), "{f} kept");
    }
    // nothing was fetched but the list, its signature and that one runner
    assert_eq!(server.requests().len(), 3, "{:?}", server.requests());
    std::fs::remove_dir_all(&cache).unwrap();
}

#[test]
fn offline_with_the_runner_in_the_cache_works() {
    let r = windows_and_web();
    let mut server = Server::start(hosted(&r, VERSION));
    let cache = temp("offline-cached");
    let cfg = config(&r, &cache, Some(server.url()));
    resolve_with(&cfg, "linux-aarch64", &UreqFetch::new(), &mut quiet).unwrap();
    let before = server.requests().len();
    // the host goes away: the runner is still there, and no request is made
    server.stop();
    let again = resolve_with(&cfg, "linux-aarch64", &UreqFetch::new(), &mut quiet).unwrap();
    assert_eq!(again.source, Source::Cache);
    assert_eq!(again.bundle.runner(false).unwrap(), b"linux-aarch64:rapidrintr-runner:contents");
    assert_eq!(server.requests().len(), before);
    // and with the setting that forbids the network
    let mut offline = cfg.clone();
    offline.offline = true;
    assert_eq!(resolve_with(&offline, "linux-aarch64", &UreqFetch::new(), &mut quiet).unwrap().source, Source::Cache);
    std::fs::remove_dir_all(&cache).unwrap();
}

#[test]
fn offline_without_the_runner_says_which_one_is_missing() {
    let r = windows_and_web();
    // (a port nothing listens on: bind, note the port, close)
    let dead = {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        l.local_addr().unwrap()
    };
    let cache = temp("offline-empty");
    let cfg = config(&r, &cache, Some(format!("http://{dead}/runners")));
    let err = resolve_with(&cfg, "windows-arm64", &UreqFetch::new(), &mut quiet).unwrap_err();
    assert!(matches!(err, Error::NotAvailable { why: Why::Unreachable { .. }, .. }), "{err:?}");
    let text = err.to_string();
    assert!(text.starts_with("The Windows ARM64 runner is not on this computer"), "{text}");
    assert!(text.contains("can't reach") && text.contains("build again"), "{text}");

    let mut off = cfg.clone();
    off.offline = true;
    let err = resolve_with(&off, "windows-arm64", &UreqFetch::new(), &mut quiet).unwrap_err();
    assert!(matches!(err, Error::NotAvailable { why: Why::OfflineSetting, .. }), "{err:?}");
    assert!(err.to_string().contains("Windows ARM64") && err.to_string().contains("offline"), "{err}");

    let mut none = cfg.clone();
    none.base_url = None;
    let err = resolve_with(&none, "windows-arm64", &UreqFetch::new(), &mut quiet).unwrap_err();
    assert!(matches!(err, Error::NotAvailable { why: Why::NoAddress, .. }), "{err:?}");
    assert!(err.to_string().contains("RAPIDR_RUNNERS_URL"), "{err}");
    assert!(err.is_unavailable());
    std::fs::remove_dir_all(&cache).unwrap();
}

#[test]
fn a_changed_byte_is_refused_with_the_message_and_nothing_is_kept() {
    let r = windows_and_web();
    let mut files = hosted(&r, VERSION);
    let file = files.get_mut(&format!("/runners/{VERSION}/linux-aarch64.zst")).unwrap();
    let middle = file.len() / 2;
    file[middle] ^= 0x10;
    let server = Server::start(files);
    let cache = temp("tampered");
    let cfg = config(&r, &cache, Some(server.url()));
    let err = resolve_with(&cfg, "linux-aarch64", &UreqFetch::new(), &mut quiet).unwrap_err();
    assert!(matches!(err, Error::Refused(Refusal::DoesNotMatch { .. })), "{err:?}");
    assert!(err.to_string().starts_with("This runner doesn't match RapidR 2.123.1's list: it was not used."), "{err}");
    assert!(!cache.join(VERSION).exists() || std::fs::read_dir(cache.join(VERSION)).unwrap().next().is_none(), "nothing kept");
    // the other runners of the same list still work
    assert!(resolve_with(&cfg, "web", &UreqFetch::new(), &mut quiet).is_ok());
    std::fs::remove_dir_all(&cache).unwrap();
}

#[test]
fn a_host_that_sends_more_than_the_list_says_is_refused() {
    let r = windows_and_web();
    let mut files = hosted(&r, VERSION);
    files.get_mut(&format!("/runners/{VERSION}/web.zst")).unwrap().extend_from_slice(b"extra bytes after the file");
    let server = Server::start(files);
    let cache = temp("longer");
    let err = resolve_with(&config(&r, &cache, Some(server.url())), "web", &UreqFetch::new(), &mut quiet).unwrap_err();
    assert!(matches!(err, Error::Refused(Refusal::DoesNotMatch { .. })), "{err:?}");
    std::fs::remove_dir_all(&cache).unwrap();
}

#[test]
fn runners_of_another_version_are_refused() {
    // a host that answers 2.123.1's address with a correctly signed list for 2.122.1
    let old = release("2.122.1", &[Runner { target: "web", files: files_for("web") }], 3);
    let mut files = HashMap::new();
    files.insert(format!("/runners/{VERSION}/runners-manifest.json"), old.manifest.clone());
    files.insert(format!("/runners/{VERSION}/runners-manifest.json.minisig"), old.signature.clone().into_bytes());
    files.insert(format!("/runners/{VERSION}/web.zst"), old.files[0].1.clone());
    let server = Server::start(files);
    let cache = temp("wrong-version");
    let mut cfg = config(&old, &cache, Some(server.url()));
    cfg.trust.version = VERSION.to_string();
    let err = resolve_with(&cfg, "web", &UreqFetch::new(), &mut quiet).unwrap_err();
    assert_eq!(err, Error::Refused(Refusal::WrongVersion { found: "2.122.1".into(), wanted: VERSION.into() }));
    assert!(err.to_string().contains("for RapidR 2.122.1, not 2.123.1: they were not used"), "{err}");
    // and the runner file was not even asked for
    assert!(!server.requests().iter().any(|p| p.ends_with("web.zst")), "{:?}", server.requests());
    std::fs::remove_dir_all(&cache).unwrap();
}

#[test]
fn a_list_signed_by_another_key_is_refused_and_no_runner_is_asked_for() {
    let theirs = release(VERSION, &[Runner { target: "web", files: files_for("web") }], 9);
    let ours = windows_and_web(); // (the key this RapidR trusts)
    let server = Server::start(hosted(&theirs, VERSION));
    let cache = temp("other-key");
    let err = resolve_with(&config(&ours, &cache, Some(server.url())), "web", &UreqFetch::new(), &mut quiet).unwrap_err();
    assert!(matches!(err, Error::Refused(Refusal::ListNotSigned { .. })), "{err:?}");
    assert!(err.to_string().contains("not signed by RapidR"), "{err}");
    assert!(!server.requests().iter().any(|p| p.ends_with(".zst")), "{:?}", server.requests());
    std::fs::remove_dir_all(&cache).unwrap();
}

#[test]
fn a_kept_file_that_changed_is_removed_and_downloaded_again() {
    let r = windows_and_web();
    let mut server = Server::start(hosted(&r, VERSION));
    let cache = temp("changed-cache");
    let cfg = config(&r, &cache, Some(server.url()));
    resolve_with(&cfg, "web", &UreqFetch::new(), &mut quiet).unwrap();
    let kept = cache.join(VERSION).join("web.zst");
    let mut bytes = std::fs::read(&kept).unwrap();
    bytes[10] ^= 0xFF;
    std::fs::write(&kept, bytes).unwrap();
    // online: not used, fetched again
    let again = resolve_with(&cfg, "web", &UreqFetch::new(), &mut quiet).unwrap();
    assert_eq!(again.source, Source::Download);
    // offline: the changed file is not used and is gone; the message names the runner
    let mut bytes = std::fs::read(&kept).unwrap();
    bytes[10] ^= 0xFF;
    std::fs::write(&kept, bytes).unwrap();
    server.stop();
    let err = resolve_with(&cfg, "web", &UreqFetch::new(), &mut quiet).unwrap_err();
    assert!(err.is_unavailable(), "{err:?}");
    assert!(!kept.exists(), "the changed file was removed");
    std::fs::remove_dir_all(&cache).unwrap();
}

#[test]
fn a_cached_list_that_was_changed_is_not_trusted() {
    let r = windows_and_web();
    let server = Server::start(hosted(&r, VERSION));
    let cache = temp("changed-list");
    let cfg = config(&r, &cache, Some(server.url()));
    resolve_with(&cfg, "web", &UreqFetch::new(), &mut quiet).unwrap();
    let list = cache.join(VERSION).join("runners-manifest.json");
    let text = std::fs::read_to_string(&list).unwrap().replace("2.123.1", "2.123.2");
    std::fs::write(&list, text).unwrap();
    let again = resolve_with(&cfg, "web", &UreqFetch::new(), &mut quiet).unwrap();
    assert_eq!(again.source, Source::Download, "the changed list was not used");
    std::fs::remove_dir_all(&cache).unwrap();
}

#[test]
fn the_installs_own_runner_comes_first_and_needs_no_network() {
    let r = windows_and_web();
    let server = Server::start(hosted(&r, VERSION));
    let install = temp("install");
    std::fs::create_dir_all(install.join("runners/linux-aarch64")).unwrap();
    std::fs::create_dir_all(install.join("notices")).unwrap();
    std::fs::write(install.join("runners/linux-aarch64/rapidrintr-runner"), b"the install's own").unwrap();
    std::fs::write(install.join("notices/linux-aarch64.txt"), b"notices").unwrap();
    let cache = temp("install-cache");
    let mut cfg = config(&r, &cache, Some(server.url()));
    cfg.install = Some(install.clone());
    let got = resolve_with(&cfg, "linux-arm64", &UreqFetch::new(), &mut quiet).unwrap();
    assert_eq!(got.source, Source::Install);
    assert_eq!(got.bundle.runner(false).unwrap(), b"the install's own");
    assert_eq!(got.bundle.notices().unwrap(), b"notices");
    assert!(server.requests().is_empty());
    // a target the install does not have still comes from the host
    let other = resolve_with(&cfg, "windows-x64", &UreqFetch::new(), &mut quiet).unwrap();
    assert_eq!(other.source, Source::Download);
    // an install whose folder lacks the program does not count as having it
    std::fs::create_dir_all(install.join("runners/windows-x86_64")).unwrap();
    std::fs::write(install.join("runners/windows-x86_64/rapidrintr-runnerw.exe"), b"x").unwrap();
    assert!(rapidr_runners::resolve::from_install(&install, "windows-x86_64").is_none());
    // ...and an install's Windows runner is the console program, whichever kind is asked for
    std::fs::write(install.join("runners/windows-x86_64/rapidrintr-runner.exe"), b"console").unwrap();
    let win = rapidr_runners::resolve::from_install(&install, "windows-x86_64").unwrap();
    assert_eq!(win.runner(true).unwrap(), b"console");
    std::fs::remove_dir_all(&install).unwrap();
    std::fs::remove_dir_all(&cache).unwrap();
}

#[test]
fn the_runner_asked_for_is_the_program_and_web_has_none() {
    let r = windows_and_web();
    let server = Server::start(hosted(&r, VERSION));
    let cache = temp("bytes");
    let cfg = config(&r, &cache, Some(server.url()));
    let fetch = UreqFetch::new();
    let gui = runner_bytes_with(&cfg, "windows-x86_64", true, &fetch, &mut quiet).unwrap();
    let console = runner_bytes_with(&cfg, "windows-x86_64", false, &fetch, &mut quiet).unwrap();
    assert_eq!(gui, console, "one Windows program for both kinds of app");
    assert_eq!(gui, b"windows-x86_64:rapidrintr-runner.exe:contents");
    assert!(matches!(runner_bytes_with(&cfg, "web", false, &fetch, &mut quiet), Err(Error::NoProgram(_))));
    assert!(matches!(runner_bytes_with(&cfg, "amiga", false, &fetch, &mut quiet), Err(Error::UnknownTarget(_))));
    let text = runner_bytes_with(&cfg, "amiga", false, &fetch, &mut quiet).unwrap_err().to_string();
    assert!(text.contains("\"amiga\" is not a system RapidR builds apps for") && text.contains("Windows 64-bit (x64)"), "{text}");
    std::fs::remove_dir_all(&cache).unwrap();
}

#[test]
fn a_host_without_the_version_says_so() {
    let r = windows_and_web();
    let server = Server::start(HashMap::new());
    let cache = temp("unpublished");
    let err = resolve_with(&config(&r, &cache, Some(server.url())), "web", &UreqFetch::new(), &mut quiet).unwrap_err();
    assert!(matches!(err, Error::NotAvailable { why: Why::NotPublished { .. }, .. }), "{err:?}");
    assert!(err.to_string().contains("no runners for this version"), "{err}");
    std::fs::remove_dir_all(&cache).unwrap();
}

#[test]
fn an_address_that_is_not_https_or_this_computer_is_not_used() {
    let r = windows_and_web();
    let cache = temp("bad-address");
    let cfg = config(&r, &cache, Some("http://rapidr.example/runners".into()));
    let err = resolve_with(&cfg, "web", &UreqFetch::new(), &mut quiet).unwrap_err();
    assert!(matches!(err, Error::NotAvailable { why: Why::BadAddress(_), .. }), "{err:?}");
    std::fs::remove_dir_all(&cache).unwrap();
}

//! RPROJECT's Build: the program made into an app for the system Studio
//! runs on — `rapidr build` (crates/rapidr-cli: Name.app on macOS, the .exe
//! with its icon on Windows, Name.AppDir on Linux) run as a child process,
//! what it prints coming back as `OnBuildOutput` lines and its end as
//! `OnBuildDone(Code, Path)`. The desktop's only: on the web the host has
//! no `rapidr` to run (Build says so).

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};

use rapidr_value::Value;

use crate::Host;

/// A build running: its lines as they come, then its exit code.
pub struct Running {
    lines: Receiver<String>,
    child: std::process::Child,
    /// What it made (the line starting "App:", "AppDir:" or "Executable:").
    made: String,
}

/// What the page's script says of a web build (RapidR Studio on the web: the
/// page zips the web app itself): a line of its log, or its end.
enum WebNews {
    Line(String),
    Done(i32, String),
}

thread_local! {
    static BUILDS: RefCell<BTreeMap<String, Running>> = RefCell::new(BTreeMap::new());
    /// The page's builds, by component, with the news not told yet.
    static WEB_BUILDS: RefCell<BTreeMap<String, Vec<WebNews>>> = RefCell::new(BTreeMap::new());
}

/// Whether a build is running (the desktop's loop then wakes often to hear it).
pub fn any() -> bool {
    BUILDS.with(|b| !b.borrow().is_empty()) || WEB_BUILDS.with(|b| !b.borrow().is_empty())
}

/// Whether component `name` is building.
pub fn building(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    BUILDS.with(|b| b.borrow().contains_key(&name)) || WEB_BUILDS.with(|b| b.borrow().contains_key(&name))
}

/// The page starts a web build for component `name` (its news come in through
/// [`web_output`] and [`web_done`]).
pub fn web_start(name: &str) -> Result<(), String> {
    if building(name) {
        return Err("a build is running already".into());
    }
    WEB_BUILDS.with(|b| b.borrow_mut().insert(name.to_ascii_lowercase(), Vec::new()));
    Ok(())
}

/// A line of the web build's log (the page's script says it).
pub fn web_output(name: &str, line: &str) {
    WEB_BUILDS.with(|b| {
        if let Some(news) = b.borrow_mut().get_mut(&name.to_ascii_lowercase()) {
            news.push(WebNews::Line(line.to_string()));
        }
    });
}

/// The web build ended: `code` 0 is made, `made` the file it made.
pub fn web_done(name: &str, code: i32, made: &str) {
    WEB_BUILDS.with(|b| {
        if let Some(news) = b.borrow_mut().get_mut(&name.to_ascii_lowercase()) {
            news.push(WebNews::Done(code, made.to_string()));
        }
    });
}

/// Starts `rapidr <args>` (in `cwd`) for component `name`.
pub fn start(name: &str, rapidr: PathBuf, args: &[String], cwd: &str) -> Result<(), String> {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};
    if building(name) {
        return Err("a build is running already".into());
    }
    let mut cmd = Command::new(&rapidr);
    cmd.args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    if !cwd.is_empty() {
        cmd.current_dir(cwd);
    }
    // (a Windows console isn't opened for it)
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let mut child = cmd.spawn().map_err(|e| format!("{}: {e}", rapidr.display()))?;
    let (tx, lines) = mpsc::channel();
    for stream in [child.stdout.take().map(|s| Box::new(s) as Box<dyn std::io::Read + Send>), child.stderr.take().map(|s| Box::new(s) as Box<dyn std::io::Read + Send>)].into_iter().flatten() {
        let tx = tx.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stream).lines() {
                let Ok(line) = line else { break };
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
    }
    BUILDS.with(|b| b.borrow_mut().insert(name.to_ascii_lowercase(), Running { lines, child, made: String::new() }));
    Ok(())
}

/// Whether native builds can run: `rapidr setup --rust` says yes (exit code
/// 0). RAPIDR_NO_RUST=1 in Studio's environment says no, to test the paths
/// without Rust.
pub fn rust_ready(rapidr: &std::path::Path) -> bool {
    use std::process::{Command, Stdio};
    let mut cmd = Command::new(rapidr);
    cmd.args(["setup", "--rust"]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    cmd.status().is_ok_and(|s| s.success())
}

/// Stops component `name`'s build.
pub fn stop(name: &str) {
    WEB_BUILDS.with(|b| b.borrow_mut().remove(&name.to_ascii_lowercase()));
    if let Some(mut r) = BUILDS.with(|b| b.borrow_mut().remove(&name.to_ascii_lowercase())) {
        let _ = r.child.kill();
        let _ = r.child.wait();
    }
}

/// What `rapidr build` says it made, from one of its lines.
pub fn made_by(line: &str) -> Option<&str> {
    ["App: ", "AppDir: ", "Executable: ", "Web: "].iter().find_map(|p| line.strip_prefix(p)).map(str::trim)
}

/// Every build's news as its events (`OnBuildOutput`, `OnBuildDone`);
/// the path each finished build made, by component. Whether one is running.
pub fn poll<H: Host>(host: H, mut finished: impl FnMut(&str, &str)) -> bool {
    // (the page's builds: what its script said, told as the events)
    let web: Vec<(String, Vec<WebNews>)> = WEB_BUILDS.with(|b| b.borrow_mut().iter_mut().map(|(n, v)| (n.clone(), std::mem::take(v))).collect());
    for (name, news) in web {
        for n in news {
            match n {
                WebNews::Line(l) => host.fire(&name, "onbuildoutput", &[Value::String(l)]),
                WebNews::Done(code, made) => {
                    WEB_BUILDS.with(|b| b.borrow_mut().remove(&name));
                    let made = if code == 0 { made } else { String::new() };
                    finished(&name, &made);
                    host.fire(&name, "onbuilddone", &[Value::Integer(i64::from(code)), Value::String(made)]);
                }
            }
        }
    }
    let names: Vec<String> = BUILDS.with(|b| b.borrow().keys().cloned().collect());
    for name in names {
        let (lines, done) = BUILDS.with(|b| {
            let mut b = b.borrow_mut();
            let r = b.get_mut(&name).expect("a running build");
            let mut lines = Vec::new();
            // (both streams read to their end: every line is in)
            let mut drained = false;
            loop {
                match r.lines.try_recv() {
                    Ok(l) => lines.push(l),
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        drained = true;
                        break;
                    }
                }
            }
            for l in &lines {
                if let Some(p) = made_by(l) {
                    r.made = p.to_string();
                }
            }
            let done = if drained {
                match r.child.try_wait() {
                    Ok(Some(status)) => Some(status.code().unwrap_or(-1)),
                    Ok(None) => None,
                    Err(_) => Some(-1),
                }
            } else {
                None
            };
            (lines, done)
        });
        for l in lines {
            host.fire(&name, "onbuildoutput", &[Value::String(l)]);
        }
        if let Some(code) = done {
            let made = BUILDS.with(|b| b.borrow_mut().remove(&name)).map(|r| r.made);
            let made = if code == 0 { made.unwrap_or_default() } else { String::new() };
            finished(&name, &made);
            host.fire(&name, "onbuilddone", &[Value::Integer(i64::from(code)), Value::String(made)]);
        }
    }
    any()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The page's web build: its lines and its end come back as events, the
    /// path it made kept for Reveal.
    #[test]
    fn a_web_build_tells_its_news() {
        use std::cell::RefCell;
        thread_local! { static SEEN: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) }; }
        #[derive(Clone, Copy)]
        struct Page;
        impl Host for Page {
            fn fire(self, name: &str, event: &str, args: &[Value]) {
                let a: Vec<String> = args.iter().map(Value::to_string_val).collect();
                SEEN.with(|s| s.borrow_mut().push(format!("{name} {event} {}", a.join("|"))));
            }
            fn launch(self, _p: &str, _a: &[String], _t: &str) -> Result<Box<dyn crate::Transport>, String> {
                Err("no".into())
            }
            fn list_files(self, _folder: &str) -> Vec<String> {
                Vec::new()
            }
        }
        assert!(web_start("Pn").is_ok());
        assert!(building("pn") && any());
        assert!(web_start("pn").is_err(), "one build at a time");
        web_output("PN", "zipping");
        web_done("pn", 0, "x-web.zip");
        let mut made = String::new();
        assert!(!poll(Page, |_, p| made = p.to_string()));
        assert_eq!(made, "x-web.zip");
        assert!(!building("pn"));
        SEEN.with(|s| assert_eq!(*s.borrow(), vec!["pn onbuildoutput zipping", "pn onbuilddone 0|x-web.zip"]));
        // (a failed build keeps nothing)
        web_start("pn").unwrap();
        web_done("pn", 1, "ignored");
        poll(Page, |_, p| made = p.to_string());
        assert_eq!(made, "");
    }

    #[test]
    fn what_was_made() {
        assert_eq!(made_by("Web: /x/app-web.zip"), Some("/x/app-web.zip"));
        assert_eq!(made_by("App: /x/Notepad.app"), Some("/x/Notepad.app"));
        assert_eq!(made_by("AppDir: a b.AppDir"), Some("a b.AppDir"));
        assert_eq!(made_by("Executable: C:\\x\\n.exe"), Some("C:\\x\\n.exe"));
        assert_eq!(made_by("Building with cargo"), None);
    }

    /// Rust ready is the exit code of `rapidr setup --rust`.
    #[cfg(unix)]
    #[test]
    fn rust_ready_is_an_exit_code() {
        assert!(rust_ready(std::path::Path::new("/usr/bin/true")));
        assert!(!rust_ready(std::path::Path::new("/usr/bin/false")));
        assert!(!rust_ready(std::path::Path::new("/nonexistent/rapidr")));
    }
}

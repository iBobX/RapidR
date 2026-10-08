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

thread_local! {
    static BUILDS: RefCell<BTreeMap<String, Running>> = RefCell::new(BTreeMap::new());
}

/// Whether a build is running (the desktop's loop then wakes often to hear it).
pub fn any() -> bool {
    BUILDS.with(|b| !b.borrow().is_empty())
}

/// Whether component `name` is building.
pub fn building(name: &str) -> bool {
    BUILDS.with(|b| b.borrow().contains_key(&name.to_ascii_lowercase()))
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

/// Stops component `name`'s build.
pub fn stop(name: &str) {
    if let Some(mut r) = BUILDS.with(|b| b.borrow_mut().remove(&name.to_ascii_lowercase())) {
        let _ = r.child.kill();
        let _ = r.child.wait();
    }
}

/// What `rapidr build` says it made, from one of its lines.
pub fn made_by(line: &str) -> Option<&str> {
    ["App: ", "AppDir: ", "Executable: "].iter().find_map(|p| line.strip_prefix(p)).map(str::trim)
}

/// Every build's news as its events (`OnBuildOutput`, `OnBuildDone`);
/// the path each finished build made, by component. Whether one is running.
pub fn poll<H: Host>(host: H, mut finished: impl FnMut(&str, &str)) -> bool {
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
    BUILDS.with(|b| !b.borrow().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_was_made() {
        assert_eq!(made_by("App: /x/Notepad.app"), Some("/x/Notepad.app"));
        assert_eq!(made_by("AppDir: a b.AppDir"), Some("a b.AppDir"));
        assert_eq!(made_by("Executable: C:\\x\\n.exe"), Some("C:\\x\\n.exe"));
        assert_eq!(made_by("Building with cargo"), None);
    }
}

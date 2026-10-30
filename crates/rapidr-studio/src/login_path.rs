//! Your Terminal's PATH for the programs Studio runs (Preview 4 S6).
//!
//! On macOS and Linux an app started from the Dock, Finder or a desktop
//! launcher gets the system's short PATH (`/usr/bin:/bin:/usr/sbin:/sbin`),
//! not the one your login shell builds from its startup files — so a
//! program's `SHELL "php …"`, a `command -v curl` or a Homebrew tool worked
//! in Terminal and failed from Studio. Studio reads the login shell's PATH
//! once at start-up: `$SHELL -l -i -c` prints `$PATH` between two markers
//! (whatever the startup files print around it is left out), in a session
//! of its own with no terminal and nothing to read (it can never stop to
//! ask), given three seconds. When that fails, the PATH Studio has plus the
//! usual folders that are there (Homebrew's, `/usr/local/bin`,
//! `~/.cargo/bin` …). The programs Studio runs, builds and serves get it;
//! their `SHELL` / `RUN` calls inherit it. Studio only reads: the shell's
//! files are never written.
//!
//! Windows' apps get the full PATH already, and the web runs nothing: there
//! the PATH isn't changed (`program_path` is `None`).

use std::time::Duration;

use rapidr_value::i18n::tr_ui_fill;


/// How long the login shell is given.
pub const TIMEOUT: Duration = Duration::from_secs(3);

/// The markers around `$PATH` in the shell's output.
pub const BEGIN: &str = "__RAPIDR_PATH_BEGIN__";
pub const END: &str = "__RAPIDR_PATH_END__";

/// Where the PATH came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    /// Your login shell (its path) printed it.
    LoginShell(String),
    /// The shell couldn't say (why): Studio's PATH and the usual folders.
    Fallback(String),
}

/// The PATH the programs Studio runs get, and where it came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShellPath {
    pub path: String,
    pub source: Source,
}

impl ShellPath {
    /// Where it came from, in words (Settings shows it).
    pub fn describe(&self) -> String {
        match &self.source {
            Source::LoginShell(shell) => tr_ui_fill("From your login shell ({shell}), as Terminal has it.", &[("shell", shell)]),
            Source::Fallback(why) => tr_ui_fill("Your login shell couldn't say ({why}): Studio's own PATH and the usual folders that are on this computer.", &[("why", why)]),
        }
    }
}

/// The command the shell runs: `$PATH` between the markers, each printed
/// on its own (no variable name runs into a marker; `printf` is every
/// shell's, fish's too).
pub fn script() -> String {
    format!("printf '%s' '{BEGIN}'; printf '%s' \"$PATH\"; printf '%s' '{END}'")
}

/// The PATH between the markers of a shell's output (what the startup
/// files printed before or after left out); `None` when the markers aren't
/// both there or what's between them isn't a PATH (empty, a line break, no
/// folder).
pub fn parse(output: &str) -> Option<String> {
    let start = output.find(BEGIN)? + BEGIN.len();
    let len = output[start..].find(END)?;
    let path = output[start..start + len].trim();
    if path.is_empty() || path.contains(['\n', '\r', '\0']) || !path.split(':').any(|p| p.starts_with('/')) {
        return None;
    }
    Some(path.to_string())
}

/// `first`'s folders, then `then`'s that aren't among them, empty ones left
/// out — nothing Studio's PATH had is lost.
pub fn merge(first: &str, then: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    for p in first.split(':').chain(then.split(':')) {
        let p = p.trim();
        if !p.is_empty() && !out.contains(&p) {
            out.push(p);
        }
    }
    out.join(":")
}

/// The usual folders tools are installed in, `~` as `home`.
pub fn usual_folders(home: &str) -> Vec<String> {
    let mut v: Vec<String> = ["/opt/homebrew/bin", "/opt/homebrew/sbin", "/usr/local/bin", "/usr/local/sbin", "/opt/local/bin", "/snap/bin"].iter().map(|s| s.to_string()).collect();
    if !home.is_empty() {
        let home = home.trim_end_matches('/');
        for d in [".cargo/bin", ".local/bin", "bin"] {
            v.push(format!("{home}/{d}"));
        }
    }
    v
}

/// Studio's PATH and the usual folders that are there (`is_dir` says).
pub fn fallback(current: &str, home: &str, is_dir: &dyn Fn(&str) -> bool) -> String {
    let extra: Vec<String> = usual_folders(home).into_iter().filter(|d| is_dir(d)).collect();
    merge(current, &extra.join(":"))
}

#[cfg(all(unix, not(target_arch = "wasm32")))]
mod reader {
    use std::io::Read;
    use std::os::unix::process::CommandExt;
    use std::process::{Command, Stdio};
    use std::sync::mpsc;
    use std::sync::{Condvar, Mutex};
    use std::time::{Duration, Instant};

    use super::{fallback, merge, parse, script, ShellPath, Source, TIMEOUT};

    /// The user's login shell: `$SHELL`, else the account's (`/etc/passwd`),
    /// else the system's default.
    pub fn user_shell() -> String {
        if let Some(s) = std::env::var("RAPIDR_LOGIN_SHELL").ok().filter(|s| !s.is_empty()) {
            return s;
        }
        if let Some(s) = std::env::var("SHELL").ok().filter(|s| s.starts_with('/')) {
            return s;
        }
        let user = std::env::var("USER").or_else(|_| std::env::var("LOGNAME")).unwrap_or_default();
        if !user.is_empty() {
            if let Ok(passwd) = std::fs::read_to_string("/etc/passwd") {
                if let Some(shell) = passwd.lines().find(|l| l.split(':').next() == Some(user.as_str())).and_then(|l| l.rsplit(':').next()).filter(|s| s.starts_with('/')) {
                    return shell.to_string();
                }
            }
        }
        if cfg!(target_os = "macos") { "/bin/zsh" } else { "/bin/sh" }.to_string()
    }

    /// Runs `shell -l -i -c <script>` and reads `$PATH` from what it prints:
    /// in a session of its own (no terminal: an interactive shell can't take
    /// one, or stop for input), nothing on its input, its errors dropped;
    /// given `timeout`, then it and whatever it started are ended.
    pub fn read_login_path(shell: &str, timeout: Duration) -> Result<String, String> {
        let mut cmd = Command::new(shell);
        cmd.args(["-l", "-i", "-c", &script()]).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
        // (a session of its own: no controlling terminal, its own process group)
        // SAFETY: setsid is async-signal-safe; nothing else runs between fork and exec.
        unsafe {
            cmd.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }
        let mut child = cmd.spawn().map_err(|e| format!("{shell}: {e}"))?;
        let pid = child.id() as i32;
        let mut out = child.stdout.take().ok_or("no output")?;
        let (tx, rx) = mpsc::channel::<Vec<u8>>();
        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match out.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                }
            }
        });
        let deadline = Instant::now() + timeout;
        let mut got: Vec<u8> = Vec::new();
        let found = loop {
            if let Some(p) = parse(&String::from_utf8_lossy(&got)) {
                break Some(p);
            }
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                break None;
            }
            match rx.recv_timeout(left) {
                Ok(chunk) => got.extend(chunk),
                // (it ended without saying, or the time is up)
                Err(mpsc::RecvTimeoutError::Disconnected) => break parse(&String::from_utf8_lossy(&got)),
                Err(mpsc::RecvTimeoutError::Timeout) => break None,
            }
        };
        // (the shell ends on its own once it has said it: a moment for it,
        // then it and anything its startup files left running are ended)
        let grace = Instant::now() + Duration::from_millis(if found.is_some() { 500 } else { 0 });
        loop {
            if matches!(child.try_wait(), Ok(Some(_))) {
                break;
            }
            if Instant::now() >= grace {
                // SAFETY: a signal to the process group this function made.
                unsafe {
                    libc::kill(-pid, libc::SIGKILL);
                }
                let _ = child.wait();
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let secs = timeout.as_secs();
        found.ok_or_else(|| {
            if Instant::now() >= deadline {
                if secs == 1 { rapidr_value::i18n::tr_ui_fill("{shell} didn't answer in 1 second", &[("shell", shell)]) } else { rapidr_value::i18n::tr_ui_fill("{shell} didn't answer in {secs} seconds", &[("shell", shell), ("secs", &secs.to_string())]) }
            } else {
                rapidr_value::i18n::tr_ui_fill("{shell} didn't print its PATH", &[("shell", shell)])
            }
        })
    }

    /// The PATH for the programs: the login shell's (then Studio's own
    /// folders it lacks), else the fallback.
    pub fn compute() -> ShellPath {
        let current = std::env::var("PATH").unwrap_or_default();
        let shell = user_shell();
        match read_login_path(&shell, TIMEOUT) {
            Ok(login) => ShellPath { path: merge(&login, &current), source: Source::LoginShell(shell) },
            Err(why) => {
                let home = std::env::var("HOME").unwrap_or_default();
                ShellPath { path: fallback(&current, &home, &|d| std::path::Path::new(d).is_dir()), source: Source::Fallback(why) }
            }
        }
    }

    /// The read: not started, running (in the background), or its result.
    #[derive(Default)]
    struct State {
        started: bool,
        result: Option<ShellPath>,
    }

    fn state() -> &'static (Mutex<State>, Condvar) {
        static S: std::sync::OnceLock<(Mutex<State>, Condvar)> = std::sync::OnceLock::new();
        S.get_or_init(|| (Mutex::new(State::default()), Condvar::new()))
    }

    /// Starts reading it in the background (once; Studio's start-up).
    pub fn start() {
        let (m, cv) = state();
        let mut s = m.lock().unwrap_or_else(|e| e.into_inner());
        if s.started {
            return;
        }
        s.started = true;
        drop(s);
        std::thread::spawn(move || {
            let r = compute();
            let mut s = m.lock().unwrap_or_else(|e| e.into_inner());
            s.result = Some(r);
            cv.notify_all();
        });
    }

    /// The result, waited for (started if it wasn't): at most the shell's
    /// time and a little more.
    pub fn get() -> Option<ShellPath> {
        start();
        let (m, cv) = state();
        let s = m.lock().unwrap_or_else(|e| e.into_inner());
        let (s, _) = cv.wait_timeout_while(s, TIMEOUT + Duration::from_secs(2), |s| s.result.is_none()).unwrap_or_else(|e| e.into_inner());
        s.result.clone()
    }

    /// Read again now (Settings' Refresh): the new result.
    pub fn refresh() -> ShellPath {
        let r = compute();
        let (m, cv) = state();
        let mut s = m.lock().unwrap_or_else(|e| e.into_inner());
        s.started = true;
        s.result = Some(r.clone());
        cv.notify_all();
        r
    }
}

#[cfg(all(unix, not(target_arch = "wasm32")))]
pub use reader::read_login_path;

/// Whether the PATH is left as it is: `RAPIDR_LOGIN_PATH=off`, or a GUI
/// test's run (`RAPIDR_CAPTURE`) that names no stand-in shell
/// (`RAPIDR_LOGIN_SHELL`) — the tests never read the user's own shell.
fn off() -> bool {
    std::env::var("RAPIDR_LOGIN_PATH").is_ok_and(|v| v == "off") || (std::env::var_os("RAPIDR_LOGIN_SHELL").is_none() && std::env::var_os("RAPIDR_CAPTURE").is_some())
}

/// Starts reading the login shell's PATH in the background (Studio's
/// start-up; nothing where the PATH isn't changed).
pub fn start() {
    if off() {
        return;
    }
    #[cfg(all(unix, not(target_arch = "wasm32")))]
    reader::start();
}

/// The PATH the programs Studio runs get, and where it came from; `None`
/// where it isn't changed (Windows, the web, `RAPIDR_LOGIN_PATH=off`).
pub fn shell_path() -> Option<ShellPath> {
    if off() {
        return None;
    }
    #[cfg(all(unix, not(target_arch = "wasm32")))]
    {
        reader::get()
    }
    #[cfg(not(all(unix, not(target_arch = "wasm32"))))]
    {
        None
    }
}

/// Read again now (Settings' Refresh).
pub fn refresh() -> Option<ShellPath> {
    if off() {
        return None;
    }
    #[cfg(all(unix, not(target_arch = "wasm32")))]
    {
        Some(reader::refresh())
    }
    #[cfg(not(all(unix, not(target_arch = "wasm32"))))]
    {
        None
    }
}

/// The PATH to give a program Studio runs (`None`: as it is).
pub fn program_path() -> Option<String> {
    shell_path().map(|s| s.path)
}

/// A command Studio starts (a build, a served program) gets it.
pub fn apply(cmd: &mut std::process::Command) {
    if let Some(p) = program_path() {
        cmd.env("PATH", p);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_path_between_the_markers() {
        // (startup files print around it: a banner, a prompt's escapes)
        let out = format!("Welcome!\n\x1b]0;title\x07{BEGIN}/opt/homebrew/bin:/usr/bin:/bin{END}\nbye\n");
        assert_eq!(parse(&out).as_deref(), Some("/opt/homebrew/bin:/usr/bin:/bin"));
        assert_eq!(parse(&format!("{BEGIN}/usr/bin")), None, "no end marker");
        assert_eq!(parse(&format!("/usr/bin{END}")), None, "no begin marker");
        assert_eq!(parse(&format!("{BEGIN}{END}")), None, "empty");
        assert_eq!(parse(&format!("{BEGIN}/usr/bin\n/bin{END}")), None, "a line break");
        assert_eq!(parse(&format!("{BEGIN}not a path{END}")), None, "no folder");
        // (the script names the markers, each printed alone)
        assert!(script().contains(BEGIN) && script().contains(END) && script().contains("\"$PATH\""));
    }

    #[test]
    fn merged_and_fallback() {
        assert_eq!(merge("/opt/homebrew/bin:/usr/bin::/bin", "/usr/bin:/bin:/usr/sbin:/sbin"), "/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin");
        let there = |d: &str| d == "/opt/homebrew/bin" || d == "/Users/x/.cargo/bin";
        assert_eq!(fallback("/usr/bin:/bin", "/Users/x", &there), "/usr/bin:/bin:/opt/homebrew/bin:/Users/x/.cargo/bin");
    }

    #[cfg(all(unix, not(target_arch = "wasm32")))]
    fn fake_shell(name: &str, body: &str) -> String {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("rapidr-login-path-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join(name);
        std::fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        p.to_string_lossy().into_owned()
    }

    /// A shell that prints around the PATH, one that never answers (ended
    /// after the time given, its child too), one that prints nothing.
    #[cfg(all(unix, not(target_arch = "wasm32")))]
    #[test]
    fn a_login_shell_read_with_a_timeout() {
        use std::time::Instant;
        // (the arguments are -l -i -c SCRIPT: the fake runs SCRIPT with a PATH of its own)
        let ok = fake_shell("ok.sh", "echo 'motd line'\nPATH=/fake/bin:/usr/bin:/bin; export PATH\neval \"$4\"\necho trailing");
        assert_eq!(read_login_path(&ok, TIMEOUT).unwrap(), "/fake/bin:/usr/bin:/bin");
        let marker = std::env::temp_dir().join(format!("rapidr-login-path-{}/child-alive", std::process::id()));
        let _ = std::fs::remove_file(&marker);
        // (it hangs, and its child would write the marker after 3 s: both ended at 1 s)
        let hang = fake_shell("hang.sh", &format!("(sleep 3; touch '{}') &\nsleep 30", marker.display()));
        let t = Instant::now();
        let e = read_login_path(&hang, Duration::from_secs(1)).unwrap_err();
        assert!(e.contains("didn't answer in 1 second"), "{e}");
        assert!(t.elapsed() < Duration::from_millis(2500), "{:?}", t.elapsed());
        std::thread::sleep(Duration::from_millis(2600));
        assert!(!marker.exists(), "the shell's child was ended with it");
        let mute = fake_shell("mute.sh", "exit 0");
        assert!(read_login_path(&mute, TIMEOUT).unwrap_err().contains("didn't print its PATH"));
        assert!(read_login_path("/no/such/shell", TIMEOUT).is_err());
    }
}

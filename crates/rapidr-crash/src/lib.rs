//! RapidR's own crash and hang reports (Preview 5, "never lose anything").
//!
//! When RapidR Studio, or a program run by `rapidr run` (Studio's Run
//! included), stops on an error in RapidR's own code — a Rust panic, a
//! stack overflow, a fatal signal — or Studio's window stops answering, a
//! report is written **on this computer only** (nothing is ever sent: no
//! telemetry), in RapidR's crash folder ([`folder`]):
//!
//! - what happened, in plain words, and the error's message;
//! - RapidR's version and git commit, the system;
//! - where the RapidR program was — file, line, SUB, the event being
//!   handled ([`place`]) — and what Studio was doing (its last command);
//! - the Rust backtrace with function names (release builds keep their
//!   symbols' names);
//! - the last AI and MCP calls (their names only) and RapidR's last 50 log
//!   lines;
//! - scrubbed ([`scrub`]): the home folder as `~`, no keys, tokens or
//!   passwords; no file contents and no program data in the first place.
//!
//! A panic is reported by the panic hook ([`install`]); a stack overflow
//! or another fatal signal by a signal handler that writes a smaller
//! report with nothing but system calls (Unix: `fatal`); a hang by
//! Studio's watchdog ([`watchdog`]). Reports older than 30 days, and past
//! the newest 20, are pruned. RapidR Studio offers the newest one at its
//! next start ([`pending_studio`]): view, copy, save, or report it on
//! GitHub — the person's choice, a prefilled page in their browser.

pub mod place;
pub mod report;
pub mod scrub;
pub mod slots;
pub mod watchdog;

#[cfg(unix)]
mod fatal;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::OnceLock;

use report::Report;
use slots::{Ring, Slot};

/// RapidR's version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// The git commit RapidR was built from ("unknown" outside a checkout).
pub const COMMIT: &str = env!("RAPIDR_BUILD_COMMIT");

/// What runs in this process.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum App {
    /// RapidR Studio itself.
    Studio,
    /// A program: `rapidr run`, Studio's Run (`rapidr run --session`).
    Program,
    /// Another RapidR command.
    Tool,
}

impl App {
    pub fn name(self) -> &'static str {
        match self {
            App::Studio => "RapidR Studio",
            App::Program => "Program",
            App::Tool => "RapidR",
        }
    }

    fn from_u8(v: u8) -> App {
        match v {
            1 => App::Studio,
            2 => App::Program,
            _ => App::Tool,
        }
    }

    fn to_u8(self) -> u8 {
        match self {
            App::Studio => 1,
            App::Program => 2,
            App::Tool => 3,
        }
    }
}

static APP: AtomicU8 = AtomicU8::new(0);
static INSTALLED: AtomicBool = AtomicBool::new(false);
/// A report of this process's end was written (the abort that follows a
/// panic, or std's after a stack overflow, writes no second one).
pub(crate) static WROTE: AtomicBool = AtomicBool::new(false);

/// What Studio is doing (its last command).
pub(crate) static DOING: Slot<160> = Slot::new();
/// The program's file (scrubbed).
pub(crate) static FILE: Slot<512> = Slot::new();
/// The last AI and MCP calls, by name.
pub(crate) static CALLS: Ring<12, 96> = Ring::new();
/// RapidR's last log lines.
pub(crate) static LOG: Ring<50, 240> = Ring::new();
/// `2.121.3 (commit ce23cb65)`, then the system, each set at install.
pub(crate) static VERSION_TEXT: Slot<96> = Slot::new();
pub(crate) static SYSTEM_TEXT: Slot<128> = Slot::new();

/// Payloads that are no crash: a host's own unwinding (a run-time error
/// travelling to the code that reports it), as the host says.
static IGNORE: OnceLock<fn(&(dyn std::any::Any + Send)) -> bool> = OnceLock::new();

thread_local! {
    static IN_HOOK: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// The app this process is.
pub fn app() -> App {
    App::from_u8(APP.load(Ordering::Relaxed))
}

/// Says what this process is (RapidR Studio is a program `rapidr run`
/// runs: it says so once it knows).
pub fn set_app(app: App) {
    APP.store(app.to_u8(), Ordering::Relaxed);
}

/// Turns crash reports on for this process: the panic hook (after the one
/// there, which still prints), the fatal signals' handler (Unix), the
/// folder made, old reports pruned. Once; later calls only say the app.
pub fn install(app: App) {
    set_app(app);
    if INSTALLED.swap(true, Ordering::AcqRel) {
        return;
    }
    VERSION_TEXT.set(&version());
    SYSTEM_TEXT.set(system());
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        previous(info);
        on_panic(info);
    }));
    let dir = folder();
    if let Some(d) = &dir {
        let _ = std::fs::create_dir_all(d);
        report::prune_in(d, report::now_secs());
    }
    #[cfg(unix)]
    fatal::install(dir.as_deref());
}

/// Whether [`install`] ran.
pub fn installed() -> bool {
    INSTALLED.load(Ordering::Acquire)
}

/// What the host's own unwinding looks like (not a crash: never reported).
pub fn set_ignore(f: fn(&(dyn std::any::Any + Send)) -> bool) {
    let _ = IGNORE.set(f);
}

/// The program this process runs (its path: the home folder as `~`).
pub fn set_program_file(path: &str) {
    FILE.set(&scrub::scrub(path));
}

/// What Studio is doing now (a command's id: `run.start`).
pub fn doing(what: &str) {
    DOING.set(&scrub::scrub(what));
}

/// An AI or MCP call, by name only (`RAI: read_file`, `MCP: run_program`).
pub fn call(name: &str) {
    CALLS.push(&scrub::scrub(name));
}

/// A line of RapidR's own log (Studio's messages, run-time errors): kept,
/// scrubbed, the last 50.
pub fn note(line: &str) {
    for l in line.lines().filter(|l| !l.trim().is_empty()) {
        LOG.push(&scrub::scrub(l.trim_end()));
    }
}

/// `2.121.3 (commit ce23cb65)`.
pub fn version() -> String {
    format!("{VERSION} (commit {COMMIT})")
}

/// The system: `macOS 27.2 (aarch64)`.
pub fn system() -> &'static str {
    static SYSTEM: OnceLock<String> = OnceLock::new();
    SYSTEM.get_or_init(|| {
        let arch = std::env::consts::ARCH;
        let name = if cfg!(target_os = "macos") {
            let v = std::fs::read_to_string("/System/Library/CoreServices/SystemVersion.plist").ok().and_then(|t| {
                let at = t.find("<key>ProductVersion</key>")?;
                let s = t[at..].find("<string>")? + at + 8;
                let e = t[s..].find("</string>")? + s;
                Some(t[s..e].to_string())
            });
            format!("macOS {}", v.unwrap_or_default()).trim_end().to_string()
        } else if cfg!(target_os = "linux") {
            let pretty = std::fs::read_to_string("/etc/os-release").ok().and_then(|t| t.lines().find_map(|l| l.strip_prefix("PRETTY_NAME=").map(|v| v.trim_matches('"').to_string())));
            match pretty {
                Some(p) => format!("Linux, {p}"),
                None => "Linux".into(),
            }
        } else if cfg!(windows) {
            "Windows".into()
        } else if cfg!(target_arch = "wasm32") {
            "a web browser".into()
        } else {
            std::env::consts::OS.into()
        };
        format!("{name} ({arch})")
    })
}

/// RapidR's crash folder: `RAPIDR_CRASH_DIR` when set (the tests'); else
/// `crashes` in `RAPIDR_STUDIO_DATA`, or beside the settings store
/// (`RAPIDR_REGISTRY`'s folder) — so a test never writes the user's —;
/// else `~/Library/Application Support/RapidR/crashes` (macOS),
/// `%APPDATA%\RapidR\crashes` (Windows), `~/.local/share/rapidr/crashes`
/// (Linux, `XDG_DATA_HOME`). `None` on the web (the page's store is
/// Studio's to write).
pub fn folder() -> Option<PathBuf> {
    if cfg!(target_arch = "wasm32") {
        return None;
    }
    let var = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    if let Some(d) = var("RAPIDR_CRASH_DIR") {
        return Some(d);
    }
    if let Some(d) = var("RAPIDR_STUDIO_DATA") {
        return Some(d.join("crashes"));
    }
    if let Some(r) = var("RAPIDR_REGISTRY") {
        return Some(r.parent().map(|p| p.join("crashes")).unwrap_or_else(|| PathBuf::from("crashes")));
    }
    let home = var("HOME");
    let base = if cfg!(windows) {
        var("APPDATA").map(|a| a.join("RapidR"))
    } else if cfg!(target_os = "macos") {
        home.map(|h| h.join("Library").join("Application Support").join("RapidR"))
    } else {
        var("XDG_DATA_HOME").or_else(|| home.map(|h| h.join(".local").join("share"))).map(|d| d.join("rapidr"))
    };
    base.map(|b| b.join("crashes"))
}

/// Writes `r` into the crash folder (made if need be; old reports pruned):
/// its path.
pub fn write(r: &Report) -> Option<PathBuf> {
    let dir = folder()?;
    std::fs::create_dir_all(&dir).ok()?;
    let now = report::now_secs();
    let mut name = report::file_name(now, &r.app, r.process, &r.kind);
    // (two in the same second: the second gets a number)
    let mut n = 2;
    while dir.join(&name).exists() {
        name = report::file_name(now, &r.app, r.process, &format!("{}{n}", r.kind));
        n += 1;
    }
    let path = dir.join(&name);
    write_private(&path, &r.render()).ok()?;
    report::prune_in(&dir, now);
    Some(path)
}

/// A file only its owner can read (Unix: 0600).
fn write_private(path: &Path, text: &str) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = std::fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(path)?;
        f.write_all(text.as_bytes())
    }
    #[cfg(not(unix))]
    {
        std::fs::write(path, text)
    }
}

/// A report of now, its context filled in: the version, the system, what
/// it was doing, the program's file and place, the calls and log lines.
pub fn report_now(kind: &str, headline: &str, message: &str) -> Report {
    let app = app();
    let place = place::describe(place::current());
    Report {
        kind: kind.into(),
        app: app.name().into(),
        headline: headline.into(),
        message: scrub::scrub(slots::cut(message, 600)),
        place,
        doing: DOING.get(),
        file: FILE.get(),
        when: report::when(report::now_secs()),
        version: version(),
        system: system().into(),
        process: std::process::id(),
        calls: CALLS.lines(),
        log: LOG.lines(),
        ..Report::default()
    }
}

/// The panic hook's report.
fn on_panic(info: &std::panic::PanicHookInfo<'_>) {
    if IN_HOOK.with(|h| h.replace(true)) {
        return;
    }
    struct Out;
    impl Drop for Out {
        fn drop(&mut self) {
            IN_HOOK.with(|h| h.set(false));
        }
    }
    let _out = Out;
    if IGNORE.get().is_some_and(|f| f(info.payload())) {
        return;
    }
    let payload = info.payload();
    let message = payload.downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| payload.downcast_ref::<String>().cloned()).unwrap_or_else(|| "a panic without a message".into());
    let thread = std::thread::current();
    let tname = thread.name().unwrap_or("unnamed").to_string();
    let main = tname == "main";
    let app = app();
    let kind = if main { "crash" } else { "thread-panic" };
    let who = match app {
        App::Studio => "RapidR Studio",
        App::Program => "Your program",
        App::Tool => "RapidR",
    };
    let headline = if main {
        format!("{who} stopped unexpectedly: an error in RapidR's own code (a Rust panic).")
    } else {
        format!("A background task of {} stopped on an error in RapidR's own code (a Rust panic); {} went on.", if app == App::Program { "your program" } else { who }, if app == App::Program { "the program" } else { "it" })
    };
    let mut r = report_now(kind, &headline, &message);
    r.thread = tname;
    r.code = info.location().map(|l| scrub::scrub(&format!("{}:{}", l.file(), l.line()))).unwrap_or_default();
    r.backtrace = backtrace_lines(&std::backtrace::Backtrace::force_capture().to_string());
    if let Some(path) = write(&r) {
        if main {
            WROTE.store(true, Ordering::Release);
        }
        eprintln!("[rapidr] A crash report was written (it stays on this computer): {}", scrub::home(&path.to_string_lossy()));
    }
}

/// A backtrace's lines, from the code that panicked (the panic machinery
/// and this hook left out), the names kept, paths scrubbed; 80 at most.
pub fn backtrace_lines(text: &str) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    // (the frames on top — taking the backtrace, this hook, the panic
    // machinery — go; the code starts at the first frame after them)
    let machinery = ["std::panicking", "core::panicking", "rust_begin_unwind", "std::sys::backtrace", "std::backtrace", "rapidr_crash::on_panic", "rapidr_crash::install", "<alloc::boxed::Box<", "core::panic::", "std::sys::"];
    let mut start = 0;
    for (i, l) in lines.iter().enumerate() {
        let t = l.trim_start();
        if !t.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            continue;
        }
        if machinery.iter().any(|m| t.contains(m)) {
            start = i + 1;
            // (its "at file:line" line too)
            while start < lines.len() && lines[start].trim_start().starts_with("at ") {
                start += 1;
            }
        } else {
            break;
        }
    }
    lines[start..].iter().take(80).map(|l| scrub::scrub(l)).collect()
}

/// `RAPIDR_TEST_CRASH`'s crash (`place::test`), or Studio's test step's:
/// `panic`, `overflow` (a real stack overflow), `abort`, `hang` (the
/// thread sleeps 8 s).
pub fn test_crash(kind: &str) {
    match kind {
        "panic" => panic!("RapidR's crash test (RAPIDR_TEST_CRASH): a panic, as a bug in RapidR would make one"),
        "overflow" => {
            let mut sink = 0u64;
            let n = deeper(1, &mut sink);
            std::hint::black_box(n);
        }
        "abort" => std::process::abort(),
        "hang" => std::thread::sleep(std::time::Duration::from_secs(8)),
        _ => {}
    }
}

/// Calls itself until the stack runs out (the test of a stack overflow).
#[inline(never)]
fn deeper(n: u64, sink: &mut u64) -> u64 {
    let buf = std::hint::black_box([n; 64]);
    *sink = sink.wrapping_add(buf[(n % 64) as usize]);
    if n == u64::MAX {
        return *sink;
    }
    deeper(n + 1, sink).wrapping_add(buf[1])
}

/// Whether the GUI test hooks are on (a test never sees a dialog or a
/// browser).
pub fn testing() -> bool {
    ["RAPIDR_TEST_DUMP", "RAPIDR_CAPTURE", "RAPIDR_TEST_EVENTS"].iter().any(|k| std::env::var_os(k).is_some())
}

// ---- what Studio asks -----------------------------------------------------------------------

/// The newest report of a Studio that ended unexpectedly, not seen yet
/// (`report::pending_studio_in`): its name.
pub fn pending_studio() -> Option<String> {
    report::pending_studio_in(&folder()?).map(|(n, _)| n)
}

/// A report's path by its name ("" when it isn't there).
pub fn path_of(name: &str) -> Option<PathBuf> {
    if name.is_empty() || name.contains(['/', '\\']) || name.contains("..") {
        return None;
    }
    let p = folder()?.join(name);
    p.is_file().then_some(p)
}

/// A report's text.
pub fn text_of(name: &str) -> Option<String> {
    std::fs::read_to_string(path_of(name)?).ok()
}

/// Notes a report as seen: Studio doesn't offer it again.
pub fn mark_seen(name: &str) {
    if let Some(dir) = folder() {
        report::mark_seen_in(&dir, &[name.to_string()]);
    }
}

/// The newest report a run of `file` wrote at or after `since` (seconds
/// since 1970): its name.
pub fn program_report(since: u64, file: &str) -> Option<String> {
    report::program_report_in(&folder()?, since, file).map(|(n, _)| n)
}

/// The reports kept, newest first (their names).
pub fn list() -> Vec<String> {
    folder().map(|d| report::list_in(&d).into_iter().map(|(n, _)| n).collect()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backtraces_start_at_the_code_and_keep_names() {
        let bt = "   0: std::backtrace_rs::backtrace::trace\n             at /rustc/x/library/std/src/backtrace.rs:1\n   1: rapidr_crash::on_panic\n   2: std::panicking::rust_panic_with_hook\n   3: rapidr_value::boom\n             at ./crates/rapidr-value/src/lib.rs:12:5\n   4: main\n";
        assert_eq!(backtrace_lines(bt), ["   3: rapidr_value::boom", "             at ./crates/rapidr-value/src/lib.rs:12:5", "   4: main"]);
    }

    #[test]
    fn the_version_and_system_say_which() {
        assert!(version().starts_with(VERSION));
        assert!(version().contains("(commit "));
        assert!(system().contains(std::env::consts::ARCH));
    }

    #[test]
    fn notes_are_scrubbed_and_kept() {
        let home = std::env::var("HOME").unwrap_or_default();
        note(&format!("Saved {home}/Clock/main.rr\nkey {}{}", "sk-ant-", "api03-Ab12Cd34Ef56Gh78Ij90Kl12"));
        let log = LOG.lines();
        assert!(log.iter().any(|l| l == "Saved ~/Clock/main.rr"), "{log:?}");
        assert!(log.iter().any(|l| l == "key [removed]"), "{log:?}");
    }
}

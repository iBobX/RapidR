//! A report: what it says, how it reads, where it is kept (one text file
//! each in the crash folder, newest by name), which ones the person has
//! seen, the pruning (older than 30 days, more than 20), and the GitHub
//! issue a person may open with it (prefilled; never sent by RapidR).

use std::path::{Path, PathBuf};

/// Reports older than this go.
pub const KEEP_DAYS: u64 = 30;
/// At most this many are kept (the newest).
pub const KEEP_COUNT: usize = 20;
/// The new-issue page's address.
pub const ISSUES_NEW: &str = "https://github.com/iBobX/RapidR/issues/new";
/// The longest prefilled address a report goes into whole (GitHub refuses
/// much longer ones); past it the person pastes the copied report.
pub const URL_MAX: usize = 7500;

/// What a report says.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// `crash` (a panic that ended it), `thread-panic` (a background
    /// thread's: it went on), `stack-overflow`, `fatal` (another signal),
    /// `hang`.
    pub kind: String,
    /// `RapidR Studio`, `Program` (one run by `rapidr run` or Studio),
    /// `RapidR` (a command).
    pub app: String,
    /// The first sentence: what happened, in plain words.
    pub headline: String,
    pub message: String,
    /// Where in the RapidR program (file, line, SUB, event).
    pub place: String,
    /// Where in RapidR's own code (the Rust source and line).
    pub code: String,
    /// What it was doing (Studio's last command).
    pub doing: String,
    /// The program's file (the home folder as ~).
    pub file: String,
    pub thread: String,
    pub when: String,
    pub version: String,
    pub system: String,
    pub process: u32,
    /// A hang's end: answered again, or quit.
    pub outcome: String,
    pub backtrace: Vec<String>,
    pub threads: Vec<String>,
    pub calls: Vec<String>,
    pub log: Vec<String>,
}

/// The privacy note every report ends with.
pub const PRIVACY: &str = "This report was written by RapidR on this computer and stays here: RapidR sends nothing anywhere. It has RapidR's version, your system's name, where RapidR and your program were, and RapidR's own last messages. Your home folder is shown as ~; it has no file contents, no keys, tokens or passwords, and no values from your program. You decide whether to share it.";

impl Report {
    /// The report as text (English: it may be read by anyone who helps).
    pub fn render(&self) -> String {
        let mut o = String::new();
        o.push_str("RapidR crash report\n===================\n\n");
        o.push_str(&self.headline);
        o.push_str("\n\n");
        let mut field = |k: &str, v: &str| {
            if !v.is_empty() {
                o.push_str(&format!("{k}: {v}\n"));
            }
        };
        field("Kind", &self.kind);
        field("App", &self.app);
        field("Message", &self.message);
        field("Where", &self.place);
        field("RapidR code", &self.code);
        field("Doing", &self.doing);
        field("File", &self.file);
        field("Thread", &self.thread);
        field("Outcome", &self.outcome);
        field("When", &self.when);
        field("RapidR", &self.version);
        field("System", &self.system);
        if self.process != 0 {
            o.push_str(&format!("Process: {}\n", self.process));
        }
        let section = |o: &mut String, title: &str, lines: &[String], empty: &str| {
            o.push_str(&format!("\n{title}\n{}\n", "-".repeat(title.chars().count())));
            if lines.is_empty() {
                o.push_str(empty);
                o.push('\n');
            }
            for l in lines {
                o.push_str(l);
                o.push('\n');
            }
        };
        if self.kind == "hang" {
            section(&mut o, "Threads", &self.threads, "(not known on this system)");
        } else {
            section(&mut o, "Rust backtrace", &self.backtrace, "(none could be taken)");
        }
        section(&mut o, "Last AI and MCP calls (names only)", &self.calls, "(none)");
        section(&mut o, "Last log lines", &self.log, "(none)");
        o.push_str(&format!("\nPrivacy\n-------\n{PRIVACY}\n"));
        o
    }
}

/// A report's header fields read back from its text: (key, value).
pub fn fields(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in text.lines().skip(3) {
        if line.starts_with('-') || line.is_empty() && !out.is_empty() {
            if line.is_empty() {
                break;
            }
            continue;
        }
        if let Some((k, v)) = line.split_once(": ") {
            if !k.is_empty() && k.len() < 16 && k.chars().all(|c| c.is_ascii_alphabetic() || c == ' ') {
                out.push((k.to_string(), v.to_string()));
            }
        }
    }
    out
}

/// One field of a report's text ("" when it has none).
pub fn field(text: &str, key: &str) -> String {
    fields(text).into_iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v).unwrap_or_default()
}

/// The headline (the report's first sentence).
pub fn headline(text: &str) -> String {
    text.lines().nth(3).unwrap_or("").to_string()
}

// ---- time without a library ----------------------------------------------------------------

/// (year, month, day) of a day count since 1970-01-01 (H. Hinnant's
/// civil_from_days).
pub fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Seconds since 1970 as (y, m, d, h, min, s), UTC.
pub fn utc(secs: u64) -> (i64, u32, u32, u32, u32, u32) {
    let (y, m, d) = civil((secs / 86_400) as i64);
    let r = secs % 86_400;
    (y, m, d, (r / 3600) as u32, (r % 3600 / 60) as u32, (r % 60) as u32)
}

pub fn now_secs() -> u64 {
    // (a browser has no operating system clock: std's `SystemTime::now`
    // panics there, so the page's own clock)
    #[cfg(target_arch = "wasm32")]
    {
        (js_sys::Date::now() / 1000.0) as u64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
    }
}

/// `2026-10-30 14:22:05 UTC`.
pub fn when(secs: u64) -> String {
    let (y, m, d, h, mi, s) = utc(secs);
    format!("{y:04}-{m:02}-{d:02} {h:02}:{mi:02}:{s:02} UTC")
}

/// A report's file name: `20261030-142205-studio-12345-crash.txt`.
pub fn file_name(secs: u64, app: &str, pid: u32, kind: &str) -> String {
    let (y, m, d, h, mi, s) = utc(secs);
    format!("{y:04}{m:02}{d:02}-{h:02}{mi:02}{s:02}-{}-{pid}-{kind}.txt", app_slug(app))
}

pub fn app_slug(app: &str) -> &'static str {
    match app {
        "RapidR Studio" => "studio",
        "Program" => "program",
        _ => "rapidr",
    }
}

/// What a report's name says: (seconds since 1970, app slug, process, kind).
pub fn parse_name(name: &str) -> Option<(u64, String, u32, String)> {
    let stem = name.strip_suffix(".txt")?;
    let mut parts = stem.splitn(5, '-');
    let date = parts.next()?;
    let time = parts.next()?;
    let app = parts.next()?.to_string();
    let pid = parts.next()?.parse().ok()?;
    let kind = parts.next()?.to_string();
    if date.len() != 8 || time.len() != 6 {
        return None;
    }
    let n = |s: &str| s.parse::<i64>().ok();
    let (y, mo, d) = (n(&date[..4])?, n(&date[4..6])?, n(&date[6..])?);
    let (h, mi, s) = (n(&time[..2])?, n(&time[2..4])?, n(&time[4..])?);
    // (days from civil: the inverse of `civil`)
    let y2 = if mo <= 2 { y - 1 } else { y };
    let era = if y2 >= 0 { y2 } else { y2 - 399 } / 400;
    let yoe = y2 - era * 400;
    let doy = (153 * (if mo > 2 { mo - 3 } else { mo + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    let secs = days * 86_400 + h * 3600 + mi * 60 + s;
    Some((secs.max(0) as u64, app, pid, kind))
}

// ---- the folder --------------------------------------------------------------------------------

/// The reports in `dir`, newest first: (name, path).
pub fn list_in(dir: &Path) -> Vec<(String, PathBuf)> {
    let Ok(rd) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut out: Vec<(String, PathBuf)> = rd.flatten().filter_map(|e| {
        let name = e.file_name().to_string_lossy().into_owned();
        parse_name(&name).map(|_| (name, e.path()))
    }).collect();
    out.sort_by(|a, b| b.0.cmp(&a.0));
    out
}

const SEEN: &str = "seen.txt";

/// The names of the reports the person has seen (or a Studio that went on
/// after them: they ended nothing).
pub fn seen_in(dir: &Path) -> Vec<String> {
    std::fs::read_to_string(dir.join(SEEN)).map(|t| t.lines().map(str::to_string).filter(|l| !l.is_empty()).collect()).unwrap_or_default()
}

/// Notes `names` as seen.
pub fn mark_seen_in(dir: &Path, names: &[String]) {
    let mut seen = seen_in(dir);
    let mut changed = false;
    for n in names {
        if !seen.contains(n) {
            seen.push(n.clone());
            changed = true;
        }
    }
    if changed {
        let _ = std::fs::write(dir.join(SEEN), seen.join("\n") + "\n");
    }
}

/// Takes away reports older than [`KEEP_DAYS`] and past the newest
/// [`KEEP_COUNT`] (and their names from the seen list): the names taken.
pub fn prune_in(dir: &Path, now: u64) -> Vec<String> {
    let mut gone = Vec::new();
    for (i, (name, path)) in list_in(dir).into_iter().enumerate() {
        let old = parse_name(&name).is_some_and(|(t, ..)| now.saturating_sub(t) > KEEP_DAYS * 86_400);
        if (old || i >= KEEP_COUNT) && std::fs::remove_file(&path).is_ok() {
            gone.push(name);
        }
    }
    if !gone.is_empty() {
        let seen: Vec<String> = seen_in(dir).into_iter().filter(|n| !gone.contains(n)).collect();
        let _ = std::fs::write(dir.join(SEEN), if seen.is_empty() { String::new() } else { seen.join("\n") + "\n" });
    }
    gone
}

/// Whether process `pid` still runs (Unix: it exists; elsewhere: never
/// known, so false).
pub fn alive(pid: u32) -> bool {
    if pid == std::process::id() {
        return true;
    }
    #[cfg(unix)]
    {
        // SAFETY: signal 0 only asks whether the process exists.
        pid > 0 && unsafe { libc::kill(pid as libc::pid_t, 0) == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM) }
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        false
    }
}

/// The newest report of a RapidR Studio that ended unexpectedly — a crash
/// (or a stack overflow, a fatal signal), or a hang it never came back from
/// —, not seen yet, of a Studio that no longer runs: its name and path.
pub fn pending_studio_in(dir: &Path) -> Option<(String, PathBuf)> {
    let seen = seen_in(dir);
    for (name, path) in list_in(dir) {
        let Some((_, app, pid, kind)) = parse_name(&name) else { continue };
        if app != "studio" || seen.contains(&name) || kind == "thread-panic" || alive(pid) {
            continue;
        }
        if kind == "hang" {
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            if field(&text, "Outcome").starts_with("Answered again") {
                continue;
            }
        }
        return Some((name, path));
    }
    None
}

/// The newest report a program run of `file` wrote at or after `since`
/// (seconds since 1970): its name and path.
pub fn program_report_in(dir: &Path, since: u64, file: &str) -> Option<(String, PathBuf)> {
    let want = crate::scrub::home(file).replace('\\', "/");
    for (name, path) in list_in(dir) {
        let Some((t, app, ..)) = parse_name(&name) else { continue };
        if t + 1 < since {
            break;
        }
        if app != "program" {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        let f = field(&text, "File").replace('\\', "/");
        if want.is_empty() || f.is_empty() || f.eq_ignore_ascii_case(&want) || f.rsplit('/').next() == want.rsplit('/').next() {
            return Some((name, path));
        }
    }
    None
}

// ---- reporting it on GitHub (the person's choice) ------------------------------------------

/// A report cut to what fits a page's address: its head, the first 20
/// frames of its backtrace by name (no file lines), its threads; no calls
/// or log lines.
pub fn short_report(text: &str) -> String {
    let mut out = Vec::new();
    let mut section = "";
    let mut frames = 0;
    let lines: Vec<&str> = text.lines().collect();
    for (i, l) in lines.iter().enumerate() {
        // (a section starts: its title over a line of dashes)
        if lines.get(i + 1).is_some_and(|n| !n.is_empty() && n.chars().all(|c| c == '-')) {
            section = l;
        }
        match section {
            "Last AI and MCP calls (names only)" | "Last log lines" | "Privacy" => continue,
            "Rust backtrace" if l.trim_start().starts_with("at ") => continue,
            "Rust backtrace" if l.trim_start().chars().next().is_some_and(|c| c.is_ascii_digit()) => {
                frames += 1;
                if frames > 20 {
                    continue;
                }
            }
            _ => {}
        }
        out.push(*l);
    }
    out.join("\n").trim_end().to_string()
}

fn encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// The new-issue page prefilled from a report's text: (address, whether
/// the whole report is in it — else the body asks the person to paste the
/// copied report). Opening it sends nothing: the person reads it on GitHub
/// and decides.
pub fn issue_url(text: &str) -> (String, bool) {
    let kind = field(text, "Kind");
    let app = field(text, "App");
    let version = field(text, "RapidR");
    let version = version.split(' ').next().unwrap_or("");
    let message = field(text, "Message");
    let what = match kind.as_str() {
        "hang" => "stopped responding".to_string(),
        "stack-overflow" => "stack overflow".to_string(),
        _ => crate::slots::cut(&message, 70).to_string(),
    };
    let who = if app.is_empty() { "RapidR".to_string() } else { app };
    let title = format!("{who}: {what}{}", if version.is_empty() { String::new() } else { format!(" ({version})") });
    let intro = "**What happened** (what you were doing, and anything else that helps):\n\n\n";
    let whole = format!("{intro}**The report** (written by RapidR on this computer):\n\n```text\n{}\n```\n", text.trim_end());
    let url = format!("{ISSUES_NEW}?title={}&body={}", encode(&title), encode(&whole));
    if url.len() <= URL_MAX {
        return (url, true);
    }
    // (a shorter report: the head and the backtrace's first frames by name;
    // the whole one is copied, to paste)
    let short = format!("{intro}**The report**, shortened (RapidR copied the whole one: paste it below too):\n\n```text\n{}\n```\n", short_report(text));
    let url = format!("{ISSUES_NEW}?title={}&body={}", encode(&title), encode(&short));
    if url.len() <= URL_MAX {
        return (url, false);
    }
    let short = format!("{intro}**The report**: RapidR copied it. Paste it between the two lines below (Ctrl+V or Cmd+V).\n\n```text\n\n```\n");
    (format!("{ISSUES_NEW}?title={}&body={}", encode(&title), encode(&short)), false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Report {
        Report {
            kind: "crash".into(),
            app: "RapidR Studio".into(),
            headline: "RapidR Studio stopped unexpectedly: an error in RapidR's own code.".into(),
            message: "index out of bounds: the len is 3 but the index is 7".into(),
            place: "main.rr line 120, in SUB Tick (OnTimer of Clock)".into(),
            code: "crates/rapidr-value/src/lib.rs:12".into(),
            when: when(1_793_456_525),
            version: "2.121.3 (commit ce23cb65)".into(),
            system: "macOS 27.2 (aarch64)".into(),
            process: 4242,
            backtrace: vec!["   0: rapidr_value::boom".into()],
            calls: vec!["RAI: read_file".into()],
            log: vec!["Saved main.rr".into()],
            ..Report::default()
        }
    }

    #[test]
    fn a_report_reads_and_reads_back() {
        let text = sample().render();
        assert!(text.starts_with("RapidR crash report\n"));
        assert!(text.contains("\nWhere: main.rr line 120, in SUB Tick (OnTimer of Clock)\n"));
        assert!(text.contains("\nRust backtrace\n--------------\n   0: rapidr_value::boom\n"));
        assert!(text.contains("\nLast AI and MCP calls (names only)\n"));
        assert!(text.trim_end().ends_with(PRIVACY));
        assert_eq!(field(&text, "Kind"), "crash");
        assert_eq!(field(&text, "Message"), "index out of bounds: the len is 3 but the index is 7");
        assert_eq!(field(&text, "Process"), "4242");
        assert_eq!(field(&text, "Outcome"), "");
        assert_eq!(headline(&text), "RapidR Studio stopped unexpectedly: an error in RapidR's own code.");
        assert_eq!(when(0), "1970-01-01 00:00:00 UTC");
        assert_eq!(when(1_793_456_525), "2026-10-31 14:22:05 UTC");
        let name = file_name(1_793_456_525, "RapidR Studio", 4242, "crash");
        assert_eq!(name, "20261031-142205-studio-4242-crash.txt");
        assert_eq!(parse_name(&name), Some((1_793_456_525, "studio".into(), 4242, "crash".into())));
        assert_eq!(parse_name("20261031-142205-program-7-stack-overflow.txt").unwrap().3, "stack-overflow");
        assert_eq!(parse_name("notes.txt"), None);
    }

    #[test]
    fn the_issue_address_carries_the_report_or_asks_to_paste_it() {
        let text = sample().render();
        let (url, whole) = issue_url(&text);
        assert!(whole);
        assert!(url.starts_with("https://github.com/iBobX/RapidR/issues/new?title=RapidR%20Studio%3A%20index%20out%20of%20bounds"), "{url}");
        assert!(url.contains("%282.121.3%29&body="));
        assert!(url.contains("%60%60%60text%0ARapidR%20crash%20report"));
        let mut long = sample();
        long.log = (0..400).map(|i| format!("a long line of RapidR's own log, number {i}")).collect();
        let (url, whole) = issue_url(&long.render());
        // (too long whole: shortened — the head and the frames, no log)
        assert!(!whole && url.len() <= URL_MAX && url.contains("shortened") && url.contains("%60%60%60text%0ARapidR%20crash%20report"), "{}", url.len());
        assert!(url.contains("rapidr_value%3A%3Aboom") && !url.contains("number%20399") && !url.contains("Privacy"), "{url}");
        let s = short_report(&long.render());
        assert!(s.contains("\nKind: crash\n") && s.contains("\nRust backtrace\n--------------\n   0: rapidr_value::boom") && !s.contains("Last log lines"), "{s}");
    }

    #[test]
    fn pruning_keeps_the_newest_twenty_of_the_last_thirty_days() {
        let dir = std::env::temp_dir().join(format!("rapidr-crash-prune-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let now = 1_793_456_525;
        for i in 0..25u64 {
            std::fs::write(dir.join(file_name(now - i * 3600, "Program", 100 + i as u32, "crash")), "x").unwrap();
        }
        // (two old ones: 31 and 40 days)
        std::fs::write(dir.join(file_name(now - 31 * 86_400, "RapidR Studio", 1, "crash")), "x").unwrap();
        std::fs::write(dir.join(file_name(now - 40 * 86_400, "RapidR Studio", 2, "hang")), "x").unwrap();
        std::fs::write(dir.join("unrelated.txt"), "x").unwrap();
        mark_seen_in(&dir, &[file_name(now - 40 * 86_400, "RapidR Studio", 2, "hang"), file_name(now, "Program", 100, "crash")]);
        let gone = prune_in(&dir, now);
        assert_eq!(gone.len(), 7, "{gone:?}");
        let left = list_in(&dir);
        assert_eq!(left.len(), KEEP_COUNT);
        assert_eq!(left[0].0, file_name(now, "Program", 100, "crash"));
        assert!(dir.join("unrelated.txt").exists());
        assert_eq!(seen_in(&dir), [file_name(now, "Program", 100, "crash")]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_studio_that_ended_is_pending_until_seen() {
        let dir = std::env::temp_dir().join(format!("rapidr-crash-pending-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let now = now_secs();
        // (process 0 never runs; this process does: its own reports wait)
        let mine = file_name(now, "RapidR Studio", std::process::id(), "crash");
        std::fs::write(dir.join(&mine), sample().render()).unwrap();
        assert_eq!(pending_studio_in(&dir), None);
        let mut hang = sample();
        hang.kind = "hang".into();
        hang.outcome = "Answered again after 7 s".into();
        std::fs::write(dir.join(file_name(now - 5, "RapidR Studio", 0, "hang")), hang.render()).unwrap();
        std::fs::write(dir.join(file_name(now - 6, "RapidR Studio", 0, "thread-panic")), sample().render()).unwrap();
        assert_eq!(pending_studio_in(&dir), None);
        let old = file_name(now - 10, "RapidR Studio", 0, "crash");
        std::fs::write(dir.join(&old), sample().render()).unwrap();
        assert_eq!(pending_studio_in(&dir).map(|p| p.0), Some(old.clone()));
        mark_seen_in(&dir, std::slice::from_ref(&old));
        assert_eq!(pending_studio_in(&dir), None);
        // (a program's report, found by its file)
        let mut p = sample();
        p.app = "Program".into();
        p.file = format!("{}/Clock/main.rr", std::env::var("HOME").unwrap_or_default());
        p.file = crate::scrub::home(&p.file);
        let pn = file_name(now, "Program", 9, "crash");
        std::fs::write(dir.join(&pn), p.render()).unwrap();
        assert_eq!(program_report_in(&dir, now - 2, &format!("{}/Clock/main.rr", std::env::var("HOME").unwrap_or_default())).map(|x| x.0), Some(pn));
        assert_eq!(program_report_in(&dir, now + 60, "main.rr"), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

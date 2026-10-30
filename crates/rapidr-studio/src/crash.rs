//! RapidR Studio's crash reports (RPROJECT's Crash… methods) over
//! `rapidr-crash`: the report of a Studio that ended unexpectedly, offered
//! at the next start (view, copy, save, report it on GitHub — the person's
//! choice); the report a program run wrote when it stopped; what Studio
//! notes for a report (its messages, its last command, the AI and MCP
//! calls by name); the hang watchdog's start, with its words in Studio's
//! language. Nothing here sends anything anywhere.
//!
//! The desktop only: on the web the methods answer "" (no report is
//! written there yet).

use rapidr_crash::report;

/// The newest report of a Studio that ended unexpectedly, not seen yet
/// (its name; "" when there is none).
pub fn pending() -> String {
    rapidr_crash::pending_studio().unwrap_or_default()
}

/// A report's text ("" when it isn't there).
pub fn text(name: &str) -> String {
    rapidr_crash::text_of(name).unwrap_or_default()
}

/// A report's file ("" when it isn't there).
pub fn path(name: &str) -> String {
    rapidr_crash::path_of(name).map(|p| crate::slashes(&p.to_string_lossy())).unwrap_or_default()
}

/// One of a report's fields: Kind, App, Message, Where, RapidR code,
/// Doing, File, Thread, Outcome, When, RapidR, System, Process; or
/// "Headline".
pub fn field(name: &str, key: &str) -> String {
    let t = text(name);
    if key.eq_ignore_ascii_case("headline") {
        return report::headline(&t);
    }
    if let Some(part) = key.strip_prefix("Where").filter(|p| !p.is_empty()) {
        let w = place_parts(&report::field(&t, "Where"));
        return match part.to_ascii_lowercase().as_str() {
            "file" => w.0,
            "line" => w.1,
            "routine" => w.2,
            "event" => w.3,
            "component" => w.4,
            _ => String::new(),
        };
    }
    report::field(&t, key)
}

/// A report's Where (`main.rr line 120, in SUB Tick (OnTimer of Clock, 3
/// handlers deep)`) in its parts, for Studio to say in its language: the
/// file, the line, the routine (`SUB Tick`, "" for the main program), the
/// event and its component.
pub fn place_parts(w: &str) -> (String, String, String, String, String) {
    let (head, event) = match w.find(" (") {
        Some(i) => (&w[..i], w[i + 2..].trim_end_matches(')')),
        None => (w, ""),
    };
    let event = event.split(", ").next().unwrap_or("");
    let (ev, comp) = event.split_once(" of ").unwrap_or((event, ""));
    let (at, routine) = head.split_once(", in ").unwrap_or((head, ""));
    let routine = if routine == "the main program" { "" } else { routine };
    let (file, line) = match at.rsplit_once(" line ") {
        Some((f, l)) => (f, l),
        None => ("", at.strip_prefix("line ").unwrap_or("")),
    };
    (file.to_string(), line.to_string(), routine.to_string(), ev.to_string(), comp.to_string())
}

/// Writes a copy of a report to `dest`.
pub fn save(name: &str, dest: &str) -> Result<(), String> {
    let t = rapidr_crash::text_of(name).ok_or_else(|| rapidr_value::i18n::tr_ui_fill("{name}: there is no such report", &[("name", name)]))?;
    if dest.trim().is_empty() {
        return Err(rapidr_value::i18n::tr_ui("no file named"));
    }
    std::fs::write(dest, t).map_err(|e| format!("{dest}: {e}"))
}

/// The prefilled new-issue address, and whether the whole report is in it.
pub fn issue(name: &str) -> (String, bool) {
    let t = text(name);
    if t.is_empty() {
        return (String::new(), false);
    }
    report::issue_url(&t)
}

/// Now, for a run's start (seconds since 1970, as text).
pub fn mark() -> String {
    report::now_secs().to_string()
}

/// The report a run of `file` started at `since` wrote ("" none).
pub fn for_program(since: &str, file: &str) -> String {
    let since = since.trim().parse::<u64>().unwrap_or(0);
    rapidr_crash::program_report(since, file).unwrap_or_default()
}

/// The hang watchdog, with its question's words.
#[allow(clippy::too_many_arguments)]
pub fn watch(seconds: i64, title: &str, body: &str, wait: &str, quit: &str, theme: &str, lang: &str) -> bool {
    if cfg!(target_arch = "wasm32") || !rapidr_crash::installed() {
        return false;
    }
    let texts = rapidr_crash::watchdog::Texts {
        title: title.into(),
        body: body.into(),
        wait: wait.into(),
        quit: quit.into(),
        caption: "RapidR Studio".into(),
        theme: theme.into(),
        lang: lang.into(),
    };
    rapidr_crash::watchdog::start(std::time::Duration::from_secs(seconds.clamp(2, 120) as u64), texts);
    true
}

/// The test steps' crash (`panic`, `hang`, `overflow`, `abort`): only
/// under the GUI tests' hooks; elsewhere nothing happens.
pub fn test(kind: &str) -> bool {
    if !rapidr_crash::testing() {
        return false;
    }
    rapidr_crash::test_crash(&kind.to_ascii_lowercase());
    true
}

/// The crash folder ("" on the web).
pub fn folder() -> String {
    rapidr_crash::folder().map(|p| crate::slashes(&p.to_string_lossy())).unwrap_or_default()
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    /// A Studio's report found, read, saved, its issue address made, then
    /// seen; a program's found by its file.
    #[test]
    fn a_report_is_offered_read_saved_and_seen() {
        let dir = std::env::temp_dir().join(format!("rapidr-studio-crash-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("RAPIDR_CRASH_DIR", &dir);
        let r = rapidr_crash::report::Report {
            kind: "crash".into(),
            app: "RapidR Studio".into(),
            headline: "RapidR Studio stopped unexpectedly: an error in RapidR's own code (a Rust panic).".into(),
            message: "boom".into(),
            place: "debug.inc line 812, in SUB ProgramExited".into(),
            version: "2.121.3 (commit x)".into(),
            ..Default::default()
        };
        // (process 0 never runs: a Studio that ended)
        let name = report::file_name(report::now_secs() - 60, "RapidR Studio", 0, "crash");
        std::fs::write(dir.join(&name), r.render()).unwrap();
        assert_eq!(pending(), name);
        assert_eq!(field(&name, "Where"), "debug.inc line 812, in SUB ProgramExited");
        assert_eq!(field(&name, "headline"), r.headline);
        let copy = dir.join("copy.txt");
        save(&name, &copy.to_string_lossy()).unwrap();
        assert_eq!(std::fs::read_to_string(&copy).unwrap(), r.render());
        assert!(save("../x.txt", "/tmp/y").is_err());
        let (url, whole) = issue(&name);
        assert!(whole && url.starts_with(report::ISSUES_NEW));
        rapidr_crash::mark_seen(&name);
        assert_eq!(pending(), "");
        let mut p = r.clone();
        p.app = "Program".into();
        p.file = "~/Clock/main.rr".into();
        let pn = report::file_name(report::now_secs(), "Program", 7, "crash");
        std::fs::write(dir.join(&pn), p.render()).unwrap();
        let since = (report::now_secs() - 5).to_string();
        assert_eq!(for_program(&since, "/elsewhere/Clock/main.rr"), pn);
        assert_eq!(for_program(&since, "other.rr"), "");
        assert!(path(&pn).ends_with(&pn));
        assert_eq!(field(&name, "WhereFile"), "debug.inc");
        assert_eq!(field(&name, "WhereRoutine"), "SUB ProgramExited");
        let p = |s: &str| place_parts(s);
        assert_eq!(p("main.rr line 120, in SUB Tick (OnTimer of Clock, 3 handlers deep)"), ("main.rr".into(), "120".into(), "SUB Tick".into(), "OnTimer".into(), "Clock".into()));
        assert_eq!(p("main.rr line 4, in the main program"), ("main.rr".into(), "4".into(), String::new(), String::new(), String::new()));
        assert_eq!(p("line 9, in FUNCTION Area"), (String::new(), "9".into(), "FUNCTION Area".into(), String::new(), String::new()));
        std::env::remove_var("RAPIDR_CRASH_DIR");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

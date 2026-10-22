//! The program's command line: RapidQ's `COMMAND$(n)` and `CommandCount`,
//! and RapidR's bare `COMMAND$`.
//!
//! RapidQ (RC.EXE, checked in the Windows VM): `CommandCount` is the number
//! of arguments; `COMMAND$(0)` is the program's own path, `COMMAND$(1)` …
//! `COMMAND$(CommandCount)` its arguments (Windows' rules: `"b c"` is one),
//! and any other index (past the last, negative) is "". A bare `COMMAND$`
//! is RapidR's (RapidQ's compiler wants the parentheses): the arguments
//! joined with spaces.
//!
//! The arguments are the program's own, however it was started: a built
//! executable's, or — the RapidR Runtime running a program file (`rapidr
//! run prog.rrbc a b`, `rapidr run-bc`, `rapidr open`, a double-clicked
//! file, the IDE) — those after the file, never the runner's ([`set`]).
//! On the web they are the page's query string's (the web runtime's).

use crate::{v_int, v_str, Value};
use std::sync::OnceLock;

static PROGRAM: OnceLock<(String, Vec<String>)> = OnceLock::new();

/// The program is the file `path` with `args`: once, before it runs (the
/// RapidR Runtime running a program file).
pub fn set(path: &str, args: Vec<String>) {
    let _ = PROGRAM.set((path.to_string(), args));
}

/// The program's file: the one [`set`], else this executable.
pub fn path() -> String {
    match PROGRAM.get() {
        Some((path, _)) => path.clone(),
        None => std::env::current_exe().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default(),
    }
}

/// The program's arguments (without the program): the ones [`set`], else
/// this executable's.
pub fn args() -> Vec<String> {
    match PROGRAM.get() {
        Some((_, args)) => args.clone(),
        None => process_args(std::env::args_os().skip(1).map(|a| a.to_string_lossy().into_owned())),
    }
}

/// Where the program's own files are: a built macOS app's
/// `Contents/Resources` (`rapidr build` puts the data files the program
/// names there, beside its bytecode), else the program's folder — a
/// `.exe`'s, a Linux AppDir's `usr/bin`, a program file's. None on the web.
pub fn files_folder() -> Option<std::path::PathBuf> {
    let program = std::path::PathBuf::from(path());
    if PROGRAM.get().is_none() {
        if let Some(resources) = app_resources(&program) {
            return Some(resources);
        }
    }
    program.parent().filter(|p| !p.as_os_str().is_empty()).map(std::path::Path::to_path_buf)
}

/// The program's own copy of relative file `file`: the same name in the
/// folder its files are in ([`files_folder`]: a built app's
/// Contents/Resources, an executable's or a program file's folder). None
/// for an absolute name or a URL, and on the web (its files are the page's).
pub fn programs_own(file: &str) -> Option<String> {
    let f = file.trim().replace('\\', "/");
    let relative = !(f.is_empty() || f.starts_with('/') || f.as_bytes().get(1) == Some(&b':') || f.contains("://"));
    let dir = files_folder()?.to_string_lossy().replace('\\', "/");
    (relative && !dir.is_empty()).then(|| format!("{}/{f}", dir.trim_end_matches('/')))
}

/// `read(file)` as RapidQ reads it — the current folder's file — and, when
/// there is no such file there, `read` of the program's own copy
/// ([`programs_own`]): a built app started from a terminal or a script in
/// another folder still finds the files it came with. Additive: a file in
/// the current folder always wins. The error is the first read's. One rule
/// for the data links (`RCSV`, `RJson`: `datascience::link::read_beside`),
/// the files objects load (`objects::read_or_own`: pictures, sounds, MIDI,
/// meshes, lists) and the runtimes' sounds (PLAYWAV, PLAYSOUND).
pub fn read_or_programs_own<T>(file: &str, read: impl Fn(&str) -> Result<T, String>) -> Result<T, String> {
    match read(file) {
        Ok(t) => Ok(t),
        Err(e) => match programs_own(file) {
            Some(own) => read(&own).map_err(|_| e),
            None => Err(e),
        },
    }
}

/// `Name.app/Contents/Resources` when `exe` is `Name.app/Contents/MacOS/<exe>`.
fn app_resources(exe: &std::path::Path) -> Option<std::path::PathBuf> {
    let macos = exe.parent()?;
    let contents = macos.parent()?;
    let app = contents.parent()?;
    if macos.file_name()? != "MacOS" || contents.file_name()? != "Contents" || !app.extension().is_some_and(|e| e.eq_ignore_ascii_case("app")) {
        return None;
    }
    let resources = contents.join("Resources");
    resources.is_dir().then_some(resources)
}

/// Whether `exe` is a Linux AppDir's program: `Name.AppDir/usr/bin/<exe>`
/// beside the AppDir's `AppRun`.
#[cfg(any(target_os = "macos", target_os = "linux"))]
fn in_appdir(exe: &std::path::Path) -> bool {
    let Some(bin) = exe.parent() else { return false };
    let Some(usr) = bin.parent() else { return false };
    bin.file_name().is_some_and(|n| n == "bin") && usr.file_name().is_some_and(|n| n == "usr") && usr.parent().is_some_and(|d| d.join("AppRun").is_file())
}

/// Once, as a program starts: a built app the desktop started (Finder, the
/// Dock, `open`, a Linux launcher — no terminal) works in the folder its
/// files are in ([`files_folder`]), as a RapidQ program double-clicked on
/// Windows works in its `.exe`'s, so `"staff.csv"` is the one it came
/// with. macOS starts an app in `/`; a Linux launcher in the home folder.
/// Anything started from a terminal keeps the terminal's folder, and so do
/// console programs (never an app: a plain executable), Windows programs
/// (Explorer starts them in their folder already) and program files the
/// RapidR Runtime runs. Its answer: the folder it moved to.
pub fn settle_current_folder() -> Option<std::path::PathBuf> {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        use std::io::IsTerminal;
        if PROGRAM.get().is_some() || std::io::stdin().is_terminal() || std::io::stdout().is_terminal() || std::io::stderr().is_terminal() {
            return None;
        }
        let exe = std::env::current_exe().ok()?;
        let cwd = std::env::current_dir().ok();
        let to = if cfg!(target_os = "macos") {
            // (launchd's: an app Finder, the Dock or `open` starts)
            if cwd.as_deref() != Some(std::path::Path::new("/")) {
                return None;
            }
            app_resources(&exe)?
        } else {
            if !in_appdir(&exe) {
                return None;
            }
            let bin = exe.parent()?.to_path_buf();
            if cwd.as_ref().is_some_and(|c| c == &bin) {
                return None;
            }
            bin
        };
        std::env::set_current_dir(&to).ok()?;
        Some(to)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        None
    }
}

/// A process's arguments as the program's: without the `-psn_…` an old
/// macOS hands an app Finder starts (its process serial number, not an
/// argument anyone typed).
pub fn process_args(args: impl Iterator<Item = String>) -> Vec<String> {
    args.filter(|a| !(cfg!(target_os = "macos") && a.starts_with("-psn_"))).collect()
}

/// `COMMAND$(n)`: 0 the program, 1… an argument, else "".
pub fn command_arg(program: &str, args: &[String], n: &Value) -> Value {
    let n = n.to_f64().round();
    if n == 0.0 {
        return v_str(program);
    }
    if n >= 1.0 && n <= args.len() as f64 {
        return v_str(&args[n as usize - 1]);
    }
    v_str("")
}

/// `CommandCount`: how many arguments.
pub fn command_count(args: &[String]) -> Value {
    v_int(args.len() as i64)
}

/// A bare `COMMAND$`: the arguments joined with spaces.
pub fn command_line(args: &[String]) -> Value {
    v_str(&args.join(" "))
}

/// A page's query string (`?a&b%20c`, without or with its `?`) as a
/// program's arguments: one per `&`-separated part, `+` a space and `%XX`
/// decoded (an empty part is none).
pub fn query_args(query: &str) -> Vec<String> {
    query
        .strip_prefix('?')
        .unwrap_or(query)
        .split('&')
        .filter(|p| !p.is_empty())
        .map(|p| percent_decode(&p.replace('+', " ")))
        .collect()
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let hex = |c: u8| (c as char).to_digit(16);
        if b[i] == b'%' && i + 2 < b.len() {
            if let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2])) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: Value) -> String {
        v.to_string()
    }

    #[test]
    fn rapidq_indexes() {
        let args = vec!["a".to_string(), "b c".to_string()];
        assert_eq!(s(command_arg("/x/p", &args, &v_int(0))), "/x/p");
        assert_eq!(s(command_arg("/x/p", &args, &v_int(1))), "a");
        assert_eq!(s(command_arg("/x/p", &args, &v_int(2))), "b c");
        assert_eq!(s(command_arg("/x/p", &args, &v_int(3))), "");
        assert_eq!(s(command_arg("/x/p", &args, &v_int(-1))), "");
        assert_eq!(s(command_count(&args)), "2");
        assert_eq!(s(command_line(&args)), "a b c");
        assert_eq!(s(command_line(&[])), "");
    }

    #[test]
    fn an_apps_own_files() {
        let dir = std::env::temp_dir().join(format!("rapidr-app-files-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let app = dir.join("Staff.app/Contents");
        std::fs::create_dir_all(app.join("MacOS")).unwrap();
        std::fs::create_dir_all(app.join("Resources")).unwrap();
        assert_eq!(app_resources(&app.join("MacOS/staff")), Some(app.join("Resources")));
        assert_eq!(app_resources(&dir.join("staff")), None, "a plain executable: its own folder");
        assert_eq!(app_resources(&dir.join("Staff/Contents/MacOS/staff")), None, "not an .app");
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            let appdir = dir.join("Staff.AppDir");
            std::fs::create_dir_all(appdir.join("usr/bin")).unwrap();
            assert!(!in_appdir(&appdir.join("usr/bin/staff")), "no AppRun: not an AppDir");
            std::fs::write(appdir.join("AppRun"), "#!/bin/sh\n").unwrap();
            assert!(in_appdir(&appdir.join("usr/bin/staff")));
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn query_strings() {
        assert_eq!(query_args("?a&b%20c&&d+e"), vec!["a", "b c", "d e"]);
        assert_eq!(query_args(""), Vec::<String>::new());
        assert_eq!(query_args("x%2"), vec!["x%2"]);
        assert_eq!(query_args("%C3%A9"), vec!["é"]);
    }

    /// The current folder's file first, then the program's own copy (this
    /// test executable's folder: no program file is set in tests).
    #[test]
    fn the_programs_own_copy_when_the_current_folder_has_none() {
        let dir = files_folder().unwrap().to_string_lossy().replace('\\', "/");
        let own = format!("{}/art/logo.svg", dir.trim_end_matches('/'));
        assert_eq!(programs_own("art\\logo.svg").as_deref(), Some(own.as_str()));
        for not in ["/etc/hosts", "C:\\x\\y.png", "https://x/y.png", ""] {
            assert_eq!(programs_own(not), None, "{not}");
        }
        let only_own = |f: &str| if f == own { Ok("own") } else { Err(format!("no {f}")) };
        assert_eq!(read_or_programs_own("art/logo.svg", only_own), Ok("own"));
        let both = |f: &str| Ok::<_, String>(if f == own { "own" } else { "here" });
        assert_eq!(read_or_programs_own("art/logo.svg", both), Ok("here"), "the current folder's file wins");
        assert_eq!(read_or_programs_own("/abs/logo.svg", only_own), Err("no /abs/logo.svg".to_string()), "the first read's error");
        assert_eq!(read_or_programs_own("gone.png", only_own), Err("no gone.png".to_string()));
    }
}

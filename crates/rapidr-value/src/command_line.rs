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
    fn query_strings() {
        assert_eq!(query_args("?a&b%20c&&d+e"), vec!["a", "b c", "d e"]);
        assert_eq!(query_args(""), Vec::<String>::new());
        assert_eq!(query_args("x%2"), vec!["x%2"]);
        assert_eq!(query_args("%C3%A9"), vec!["é"]);
    }
}

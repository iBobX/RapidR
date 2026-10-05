//! The program as it was started: its file (Application.ExeName,
//! Application.Path, Application.Title) and its arguments (COMMAND$).
//!
//! A built executable is its own program. The RapidR Runtime running a
//! program file (`rapidr run prog.rrbc a b`, a double-clicked `.rrbc`, a
//! `#!/usr/bin/env rapidr` script) sets the program to that file and its
//! arguments, so the program finds the files next to it as it would built.

use std::sync::OnceLock;

static PROGRAM: OnceLock<(String, Vec<String>)> = OnceLock::new();

/// The program file and its arguments, once, before it runs.
pub fn set(path: &str, args: Vec<String>) {
    let _ = PROGRAM.set((path.to_string(), args));
}

/// The program's file.
pub fn path() -> String {
    match PROGRAM.get() {
        Some((path, _)) => path.clone(),
        None => std::env::current_exe().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default(),
    }
}

/// The program's arguments (without the program).
pub fn args() -> Vec<String> {
    match PROGRAM.get() {
        Some((_, args)) => args.clone(),
        None => std::env::args().skip(1).collect(),
    }
}

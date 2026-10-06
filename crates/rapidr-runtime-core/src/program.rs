//! The program as it was started: its file (Application.ExeName,
//! Application.Path, Application.Title, COMMAND$(0)) and its arguments
//! (COMMAND$(n), CommandCount, COMMAND$).
//!
//! A built executable is its own program. The RapidR Runtime running a
//! program file (`rapidr run prog.rrbc a b`, `rapidr run-bc`, a
//! double-clicked `.rrbc`, a `#!/usr/bin/env rapidr` script) sets the
//! program to that file and its arguments, so the program finds the files
//! next to it, and sees only its own arguments, as it would built. (Shared:
//! rapidr_value::command_line.)

pub use crate::value::command_line::{args, path, set};

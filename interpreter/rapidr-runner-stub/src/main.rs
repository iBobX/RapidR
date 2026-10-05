//! `rapidrintr-runner` — bytecode runtime stub for self-contained
//! native executables.
//!
//! At build time the RapidR CLI copies this binary to the user's
//! desired output path and **appends** the program's `.rrbc` bytes plus
//! a 12-byte footer of the form:
//!
//! ```text
//! ... [stub elf/macho bytes] [rrbc bytes] [magic 8B "RRBCEXE1"] [u32 LE length]
//! ```
//!
//! At startup the runner opens its own executable, seeks to the end,
//! reads the footer, and passes the payload bytes to
//! [`rapidr_vm_host_native::run_bytes`].
//!
//! When invoked with no payload (i.e. a freshly built stub) it falls
//! back to `rapidrintr-runner --bytecode <file.rrbc>` for testing /
//! development.

mod runner;

fn main() -> std::process::ExitCode {
    runner::main()
}

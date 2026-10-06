//! `rapidr dap`: the Debug Adapter Protocol (stdio) for VS Code and other
//! editors, mapped onto the program session protocol (docs/ide-plan.md, I6).
//!
//! The editor speaks DAP to `rapidr dap` ([`adapter`]); the adapter runs
//! the program as a child process — its end of the session — and speaks the
//! session protocol to it ([`session_wire`]: JSON requests on its standard
//! input, events framed on its standard output among what it prints). The
//! IDE's own debugger is another client of the same protocol.

use std::path::Path;
use std::process::ExitCode;

mod adapter;
pub mod dap;
mod interim;
pub mod session_wire;

/// `rapidr dap`: serves one debug session on stdin / stdout.
pub fn run_stdio() -> ExitCode {
    adapter::run_stdio()
}

/// `rapidr __debuggee <file> [args]`: the program end of a debug session
/// (interim, until `rapidr run --session` lands).
pub fn debuggee_main(args: &[String]) -> ExitCode {
    interim::debuggee_main(args)
}

/// The command that runs `program` with `args` as the program's end of a
/// session: this `rapidr`. When rapidr-session lands, the one change is
/// `__debuggee` → `run --session`.
pub fn program_end_command(program: &Path, args: &[String]) -> std::io::Result<std::process::Command> {
    let mut command = std::process::Command::new(std::env::current_exe()?);
    command.arg("__debuggee").arg(program).args(args);
    Ok(command)
}

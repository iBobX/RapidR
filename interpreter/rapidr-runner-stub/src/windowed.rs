//! `rapidrintr-runnerw`: the runner windowed programs for Windows are built
//! from (`rapidr build --interp` picks it when the program isn't a console
//! one): a Windows GUI executable, so no console window opens with it —
//! as `$APPTYPE GUI` programs built by RapidQ. Elsewhere it is the same as
//! `rapidrintr-runner` (main.rs).
#![cfg_attr(windows, windows_subsystem = "windows")]

mod runner;

fn main() -> std::process::ExitCode {
    runner::main()
}

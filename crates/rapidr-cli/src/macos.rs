//! macOS executables are universal (arm64 + x86_64): RapidR ships one runner
//! for both (`runners/macos/`), native builds make both slices when Rust has
//! both targets (joined by rapidr_package::macho::universal), and `--target
//! macos-arm64` / `macos-x64` takes one slice out of the universal runner
//! (the packer). Apple silicon runs the arm64 slice, Intel Macs (up to
//! macOS 26) the x86_64 one — nothing Intel-only is needed where Rosetta is
//! going away (macOS 28).

use std::path::Path;
use std::process::Command;

/// The oldest macOS RapidR's executables run on: `MACOSX_DEPLOYMENT_TARGET`
/// for both slices (Rust raises it to 11.0 for arm64, the first macOS there
/// was for Apple silicon; 10.13 is what winit and Metal need on Intel) —
/// the apps' LSMinimumSystemVersion (rapidr_package::macos::MINIMUM_SYSTEM).
pub const DEPLOYMENT_TARGET: &str = rapidr_package::macos::MINIMUM_SYSTEM;

/// The two Rust targets of a universal executable.
pub const TRIPLES: [&str; 2] = ["aarch64-apple-darwin", "x86_64-apple-darwin"];

/// Rust has both macOS targets (`rustup target add` both): a native build
/// makes a universal executable.
pub fn rust_has_both_targets(mut rustc: Command) -> bool {
    let sysroot = rustc.args(["--print", "sysroot"]).output().ok().filter(|o| o.status.success());
    let Some(sysroot) = sysroot.map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()) else { return false };
    TRIPLES.iter().all(|t| Path::new(&sysroot).join("lib/rustlib").join(t).join("lib").is_dir())
}

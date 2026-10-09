//! `rapidr setup`: what an installed RapidR needs beyond itself.
//!
//! - Rust, for native builds (`rapidr build`): the exact toolchain the
//!   install was tested with (`Home::rust_toolchain`: `1.98.1`, on Windows
//!   `1.98.1-<arch>-pc-windows-gnullvm`), which builds always name
//!   (RUSTUP_TOOLCHAIN). Interpreted programs and `--interp` executables need
//!   none.
//!   - **The user's Rust is theirs.** With rustup already there, setup only
//!     installs that toolchain (and macOS' two targets) *beside* the user's
//!     own: never `rustup default`, `set default-host`, `update`, overrides,
//!     or removing a toolchain ([`rustup_steps`]; tested).
//!   - With no Rust at all, rustup is installed (MIT / Apache-2.0) after
//!     saying so and asking, with this machine's native architecture as its
//!     host (aarch64 on Windows on ARM, also from the emulated x64 rapidr).
//!   - With a Rust that isn't rustup's: reported, nothing changed.
//! - On Windows, the gnullvm toolchain links with the LLVM-MinGW the SDK
//!   ships (`--toolchain gnullvm`, the default: open source, no Visual
//!   Studio), or `--toolchain msvc` with Microsoft's C++ Build Tools (native
//!   builds then need RAPIDR_TOOLCHAIN=msvc).
//! - The `rapidr` command on PATH, when it isn't (the macOS app, a
//!   `.tar.gz`): a link in `/usr/local/bin` or `~/.local/bin` (`--no-path`:
//!   not offered).
//!
//! `rapidr setup --check` only reports; `rapidr setup --rust` answers only
//! whether native builds can run (exit 0 yes, 1 no: Studio asks it).

use std::env;
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use crate::home::{native_arch, rust_tool, toolchain_name, Home};

/// The Rust version `rustc --version` reports — `toolchain`'s, when named.
fn rustc_version(toolchain: Option<&str>) -> Option<String> {
    let mut rustc = Command::new(rust_tool("rustc"));
    if let Some(tc) = toolchain {
        rustc.env("RUSTUP_TOOLCHAIN", tc);
    }
    let out = rustc.arg("--version").output().ok().filter(|o| o.status.success())?;
    let text = String::from_utf8_lossy(&out.stdout);
    text.split_whitespace().nth(1).map(str::to_string)
}

/// Whether native builds can run here: the Rust they use answers (an
/// install's own toolchain, else the user's `rustc`). `RAPIDR_NO_RUST=1`
/// says "no Rust" without touching the machine, for testing the paths a
/// machine without it takes (Studio's dialog, the build's message).
pub fn native_ready() -> bool {
    if no_rust_forced(env::var("RAPIDR_NO_RUST").ok().as_deref()) {
        return false;
    }
    let home = Home::find();
    let msvc = env::var("RAPIDR_TOOLCHAIN").is_ok_and(|t| t == "msvc");
    let rust = home.as_ref().and_then(|h| h.release.as_ref()).map(|r| r.rust.clone()).unwrap_or_default();
    let tc = (!rust.is_empty()).then(|| toolchain_name(&rust, env::consts::OS, env::consts::ARCH, msvc));
    rustc_version(tc.as_deref()).is_some()
}

/// RAPIDR_NO_RUST is set to something but "" or 0.
fn no_rust_forced(value: Option<&str>) -> bool {
    value.is_some_and(|v| !v.is_empty() && v != "0")
}

/// What a native build says when there is no Rust.
pub const NEEDS_RUST: &str = "Native builds need Rust (it is free). Run `rapidr setup` to install it, or build without it: add --interp (an interpreted app runs the program's bytecode).";

/// The toolchains rustup has (`rustup toolchain list`), or None: no rustup.
fn rustup_toolchains() -> Option<Vec<String>> {
    let out = Command::new(rust_tool("rustup")).args(["toolchain", "list"]).output().ok()?;
    if !out.status.success() {
        // (rustup with no toolchain at all says so on stdout and succeeds; a
        // failure is a rustup that doesn't work)
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).lines().filter_map(|l| l.split_whitespace().next()).map(str::to_string).collect())
}

/// `tc` is among `installed`: `1.98.1` is listed with its host
/// (`1.98.1-aarch64-apple-darwin`).
fn has_toolchain(installed: &[String], tc: &str) -> bool {
    installed.iter().any(|t| t == tc || (!tc.contains("-pc-") && t.starts_with(&format!("{tc}-"))))
}

/// The rustup commands that give an install its toolchain — only ever
/// installing, beside what is there.
fn rustup_steps(tc: &str, installed: &[String], tc_arch: &str, native: &str, os: &str) -> Vec<Vec<String>> {
    let mut steps = Vec::new();
    if !has_toolchain(installed, tc) {
        let mut s: Vec<String> = ["toolchain", "install", tc, "--profile", "minimal", "--no-self-update"].map(String::from).to_vec();
        // (the x64 SDK on Windows on ARM: its toolchain runs emulated)
        if os == "windows" && tc_arch != native {
            s.push("--force-non-host".into());
        }
        steps.push(s);
    }
    if os == "macos" {
        // (universal native builds: both slices)
        steps.push(["target", "add", "--toolchain", tc, "aarch64-apple-darwin", "x86_64-apple-darwin"].map(String::from).to_vec());
    }
    steps
}

/// rustup has a default toolchain (`rustup default` succeeds).
fn has_default_toolchain() -> bool {
    Command::new(rust_tool("rustup")).arg("default").output().is_ok_and(|o| o.status.success())
}

/// The one `rustup default` setup ever runs: back to "none" when the user
/// had no default and rustup made RapidR's toolchain the default on its own
/// (it does that for the first toolchain installed).
fn restore_default(had_default: bool, has_default: bool) -> Option<Vec<String>> {
    (!had_default && has_default).then(|| vec!["default".into(), "none".into()])
}

/// What a machine with no Rust at all is set up with: rustup for the
/// machine's native architecture, its default toolchain the install's own
/// when that runs natively (or the same Rust for the native host).
fn rustup_init_args(rust: &str, tc: &str, tc_arch: &str, native: &str, os: &str, msvc: bool) -> Vec<String> {
    let mut a: Vec<String> = ["-y", "--profile", "minimal"].map(String::from).to_vec();
    if os == "windows" {
        let native_tc = toolchain_name(rust, os, native, msvc);
        a.extend(["--default-host".into(), native_tc.trim_start_matches(&format!("{rust}-")).to_string()]);
        a.extend(["--default-toolchain".into(), if tc_arch == native { tc.to_string() } else { rust.to_string() }]);
    } else {
        a.extend(["--default-toolchain".into(), rust.to_string()]);
    }
    a
}

fn at_least(have: &str, need: &str) -> bool {
    need.is_empty() || rapidr_bytecode::parse_version(have) >= rapidr_bytecode::parse_version(need)
}

fn confirm(question: &str, yes: bool) -> bool {
    if yes {
        return true;
    }
    if !std::io::stdin().is_terminal() {
        eprintln!("{question} — not asked (no terminal): run `rapidr setup --yes` to agree");
        return false;
    }
    eprint!("{question} [y/N] ");
    let _ = std::io::stderr().flush();
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).is_ok() && matches!(line.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}

pub fn setup(args: &[String]) -> ExitCode {
    let check = args.iter().any(|a| a == "--check");
    let yes = args.iter().any(|a| a == "--yes" || a == "-y");
    let no_path = args.iter().any(|a| a == "--no-path");
    let toolchain = args.iter().position(|a| a == "--toolchain").and_then(|i| args.get(i + 1)).map(String::as_str).unwrap_or("gnullvm");
    if !["gnullvm", "msvc"].contains(&toolchain) {
        eprintln!("--toolchain {toolchain}: gnullvm (LLVM-MinGW, open source) or msvc (Microsoft's C++ Build Tools)");
        return ExitCode::from(2);
    }

    // `--rust`: only whether native builds are ready (exit 0), for Studio
    if args.iter().any(|a| a == "--rust") {
        return if native_ready() {
            println!("native builds: ready");
            ExitCode::SUCCESS
        } else {
            println!("native builds: need Rust (rapidr setup)");
            ExitCode::from(1)
        };
    }

    let home = Home::find();
    println!("RapidR {}", env!("CARGO_PKG_VERSION"));
    match &home {
        Some(h) => println!(
            "home: {} ({})",
            h.root.display(),
            match &h.release {
                Some(r) => format!("{} install {}", r.kind, r.version),
                None => "source checkout".into(),
            }
        ),
        None => println!("home: not found (set RAPIDR_HOME)"),
    }
    let rust = home.as_ref().and_then(|h| h.release.as_ref()).map(|r| r.rust.clone()).unwrap_or_default();
    let builds = home.as_ref().is_some_and(Home::can_build);
    let installed_home = home.as_ref().is_some_and(|h| h.release.is_some());
    let mut ok = true;

    // 1. Rust (native builds)
    let msvc = toolchain == "msvc" || env::var("RAPIDR_TOOLCHAIN").is_ok_and(|t| t == "msvc");
    let (os, arch, native) = (env::consts::OS, env::consts::ARCH, native_arch());
    let tc = (installed_home && !rust.is_empty()).then(|| toolchain_name(&rust, os, arch, msvc));
    let rustup = rustup_toolchains();
    match (&tc, &rustup) {
        _ if !builds => match rustc_version(None) {
            Some(v) => println!("rust: {v} (this runtime runs programs; the RapidR SDK builds them)"),
            None => println!("rust: not installed (this runtime runs programs; the RapidR SDK builds them)"),
        },
        // a checkout: the user's Rust, as it is
        (None, _) => match rustc_version(None) {
            Some(v) => println!("rust: {v}"),
            None => println!("rust: not found — native builds need it (https://rustup.rs)"),
        },
        // rustup is here: the install's toolchain beside the user's own
        (Some(tc), Some(installed)) => {
            let steps = rustup_steps(tc, installed, arch, native, os);
            let missing = !has_toolchain(installed, tc);
            if missing {
                println!("rust: rustup is here; native builds use {tc}, which it doesn't have yet (installed beside your toolchains: your default stays as it is)");
            }
            if !check && !steps.is_empty() && (!missing || confirm(&format!("Install {tc} (`rustup toolchain install {tc} --profile minimal`)?"), yes)) {
                let had_default = has_default_toolchain();
                for s in &steps {
                    ok &= run_ok(Command::new(rust_tool("rustup")).args(s));
                }
                // (rustup makes the first toolchain it installs the default when
                // none is set: put "none" back, as the user had it)
                if let Some(s) = restore_default(had_default, has_default_toolchain()) {
                    ok &= run_ok(Command::new(rust_tool("rustup")).args(&s));
                }
            }
            match rustc_version(Some(tc)) {
                Some(v) => println!("rust: {v} ({tc}: native builds ready)"),
                None => {
                    println!("rust: {tc} isn't installed (`rapidr setup`)");
                    ok &= check;
                }
            }
        }
        // a Rust that isn't rustup's: reported, left alone
        (Some(_), None) if rustc_version(None).is_some() => {
            let v = rustc_version(None).unwrap_or_default();
            println!(
                "rust: {v}, not managed by rustup: native builds use it{}",
                if at_least(&v, &rust) { "" } else { " — older than the Rust this RapidR was tested with, update it yourself" }
            );
        }
        // no Rust at all: rustup, for this machine's architecture
        (Some(tc), None) => {
            println!("rust: not installed — needed only for native builds (`rapidr build`); interpreted programs and `--interp` executables need none");
            if !check {
                println!(
                    "\nRust is installed with rustup (MIT / Apache-2.0) from https://rustup.rs into {} and ~/.rustup\n(about 1.5 GB; uninstall with `rustup self uninstall`).",
                    crate::home::cargo_home().map(|p| p.display().to_string()).unwrap_or_else(|| "~/.cargo".into())
                );
                if cfg!(windows) {
                    println!("{}", windows_toolchain_note(toolchain));
                }
                if confirm("Install Rust now?", yes) {
                    ok &= install_rust(&rustup_init_args(&rust, tc, arch, native, os, msvc), native);
                    // (what the default doesn't cover: a non-native toolchain, macOS' targets)
                    let installed = rustup_toolchains().unwrap_or_default();
                    for s in rustup_steps(tc, &installed, arch, native, os) {
                        ok &= run_ok(Command::new(rust_tool("rustup")).args(&s));
                    }
                } else {
                    ok = false;
                }
            }
        }
    }
    if cfg!(windows) && builds {
        match home.as_ref().and_then(Home::windows_toolchain) {
            Some(dir) if !msvc => println!("linker: LLVM-MinGW, shipped ({})", dir.display()),
            _ => check_windows_linker(toolchain),
        }
    }

    // 2. `rapidr` on PATH
    if let Ok(exe) = env::current_exe().and_then(|e| e.canonicalize()) {
        let on_path = env::var_os("PATH").is_some_and(|p| env::split_paths(&p).any(|d| d.join(exe.file_name().unwrap_or_default()).canonicalize().ok().as_deref() == Some(exe.as_path())));
        if on_path {
            println!("command: {} is on PATH", exe.display());
        } else if cfg!(unix) {
            println!("command: {} is not on PATH", exe.display());
            if !check && !no_path {
                ok &= link_on_path(&exe, yes);
            }
        }
    }
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn run_ok(cmd: &mut Command) -> bool {
    match cmd.status() {
        Ok(s) if s.success() => true,
        Ok(s) => {
            eprintln!("{cmd:?} failed ({s})");
            false
        }
        Err(e) => {
            eprintln!("{cmd:?}: {e}");
            false
        }
    }
}

/// rustup's installer, for this machine's architecture (`native`), with
/// `args` ([`rustup_init_args`]). Only run where there is no rustup.
fn install_rust(args: &[String], native: &str) -> bool {
    if cfg!(windows) {
        let url = format!("https://static.rust-lang.org/rustup/dist/{native}-pc-windows-msvc/rustup-init.exe");
        let init = env::temp_dir().join("rustup-init.exe");
        let ps = format!("Invoke-WebRequest -UseBasicParsing -Uri '{url}' -OutFile '{}'", init.display());
        if !run_ok(Command::new("powershell").args(["-NoProfile", "-Command", &ps])) {
            return false;
        }
        let done = run_ok(Command::new(&init).args(args));
        let _ = std::fs::remove_file(&init);
        done
    } else {
        let script = format!("curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- {}", args.join(" "));
        run_ok(Command::new("sh").args(["-c", &script]))
    }
}

fn windows_toolchain_note(toolchain: &str) -> &'static str {
    if toolchain == "gnullvm" {
        "On Windows Rust needs a linker: the RapidR SDK ships LLVM-MinGW (open source: Apache-2.0 with LLVM\n\
         exception, mingw-w64's permissive licences; what it links carries no obligations), used with\n\
         Rust's gnullvm toolchain — nothing else to install, no Visual Studio.\n\
         (Or `rapidr setup --toolchain msvc` for Microsoft's C++ Build Tools.)"
    } else {
        "On Windows the msvc toolchain links with Microsoft's C++ Build Tools (Visual Studio Build Tools,\n\
         \"Desktop development with C++\"): rustup offers to install them. They are Microsoft's,\n\
         under Visual Studio's licence terms — check they fit you (RapidR recommends --toolchain gnullvm)."
    }
}

/// The linker the toolchain needs, on PATH.
fn check_windows_linker(toolchain: &str) {
    let tool = if toolchain == "gnullvm" { "clang.exe" } else { "link.exe" };
    let found = env::var_os("PATH").is_some_and(|p| env::split_paths(&p).any(|d| d.join(tool).is_file()));
    if found {
        println!("linker: {tool} found ({toolchain})");
    } else {
        println!("linker: {tool} not on PATH — native builds need it\n{}", windows_toolchain_note(toolchain));
    }
}

/// A link to `exe` in `/usr/local/bin` (when writable) or `~/.local/bin`.
fn link_on_path(exe: &Path, yes: bool) -> bool {
    let system = PathBuf::from("/usr/local/bin");
    let writable = |d: &Path| d.is_dir() && tempfile_in(d);
    let dir = if writable(&system) { system } else { std::env::home_dir().map(|h| h.join(".local/bin")).unwrap_or(system) };
    let link = dir.join("rapidr");
    if !confirm(&format!("Link {} → {}?", link.display(), exe.display()), yes) {
        return false;
    }
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("{}: {e}", dir.display());
        return false;
    }
    let _ = std::fs::remove_file(&link);
    #[cfg(unix)]
    if let Err(e) = std::os::unix::fs::symlink(exe, &link) {
        eprintln!("{}: {e}", link.display());
        return false;
    }
    println!("linked {}", link.display());
    let on_path = env::var_os("PATH").is_some_and(|p| env::split_paths(&p).any(|d| d == dir));
    if !on_path {
        println!("add it to PATH:  echo 'export PATH=\"{}:$PATH\"' >> ~/.zshrc   (or ~/.bashrc)", dir.display());
    }
    true
}

/// Whether a file can be made in `dir`.
fn tempfile_in(dir: &Path) -> bool {
    let probe = dir.join(format!(".rapidr-probe-{}", std::process::id()));
    let ok = std::fs::write(&probe, b"").is_ok();
    let _ = std::fs::remove_file(&probe);
    ok
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Nothing setup asks of rustup changes the user's own Rust.
    fn only_installs(cmd: &[String]) {
        assert!(matches!(cmd.get(..2).map(|v| v.join(" ")).as_deref(), Some("toolchain install") | Some("target add")), "rustup {cmd:?}");
        assert!(!cmd.iter().any(|a| ["default", "set", "update", "uninstall", "remove", "override", "self", "--default-host"].contains(&a.as_str())), "rustup {cmd:?}");
    }

    #[test]
    fn no_rust_can_be_said_for_testing() {
        assert!(no_rust_forced(Some("1")) && no_rust_forced(Some("yes")));
        assert!(!no_rust_forced(None) && !no_rust_forced(Some("")) && !no_rust_forced(Some("0")));
        assert!(NEEDS_RUST.contains("--interp") && NEEDS_RUST.contains("rapidr setup"));
    }

    #[test]
    fn with_rustup_the_toolchain_goes_beside_the_users_own() {
        let users = vec!["1.98.1-aarch64-pc-windows-msvc".to_string(), "stable-aarch64-pc-windows-msvc".to_string()];
        let tc = toolchain_name("1.98.1", "windows", "aarch64", false);
        let steps = rustup_steps(&tc, &users, "aarch64", "aarch64", "windows");
        assert_eq!(steps, vec![["toolchain", "install", "1.98.1-aarch64-pc-windows-gnullvm", "--profile", "minimal", "--no-self-update"].map(String::from).to_vec()]);
        steps.iter().for_each(|s| only_installs(s));
        // the x64 SDK on Windows on ARM: its toolchain beside the native ones, emulated
        let x64 = toolchain_name("1.98.1", "windows", "x86_64", false);
        let steps = rustup_steps(&x64, &users, "x86_64", "aarch64", "windows");
        assert!(steps[0].contains(&"--force-non-host".to_string()));
        steps.iter().for_each(|s| only_installs(s));
        // already there: nothing at all
        assert!(rustup_steps(&tc, &[tc.clone()], "aarch64", "aarch64", "windows").is_empty());
    }

    #[test]
    fn macos_adds_both_targets_to_its_own_toolchain_only() {
        let users = vec!["1.98.1-aarch64-apple-darwin".to_string()];
        let steps = rustup_steps("1.98.1", &users, "aarch64", "aarch64", "macos");
        assert_eq!(steps, vec![["target", "add", "--toolchain", "1.98.1", "aarch64-apple-darwin", "x86_64-apple-darwin"].map(String::from).to_vec()]);
        steps.iter().for_each(|s| only_installs(s));
        let steps = rustup_steps("1.98.1", &["stable-aarch64-apple-darwin".to_string()], "aarch64", "aarch64", "macos");
        assert_eq!(steps.len(), 2);
        steps.iter().for_each(|s| only_installs(s));
    }

    #[test]
    fn a_fresh_install_is_for_the_native_architecture() {
        // the x64 rapidr, emulated on Windows on ARM: rustup for aarch64
        let a = rustup_init_args("1.98.1", "1.98.1-x86_64-pc-windows-gnullvm", "x86_64", "aarch64", "windows", false);
        assert_eq!(a, ["-y", "--profile", "minimal", "--default-host", "aarch64-pc-windows-gnullvm", "--default-toolchain", "1.98.1"]);
        let a = rustup_init_args("1.98.1", "1.98.1-aarch64-pc-windows-gnullvm", "aarch64", "aarch64", "windows", false);
        assert_eq!(a[3..], ["--default-host", "aarch64-pc-windows-gnullvm", "--default-toolchain", "1.98.1-aarch64-pc-windows-gnullvm"]);
        assert_eq!(rustup_init_args("1.98.1", "1.98.1", "aarch64", "aarch64", "linux", false), ["-y", "--profile", "minimal", "--default-toolchain", "1.98.1"]);
    }

    #[test]
    fn a_default_rustup_set_on_its_own_is_put_back() {
        assert_eq!(restore_default(false, true), Some(vec!["default".to_string(), "none".to_string()]));
        // the user's own default: never touched
        assert_eq!(restore_default(true, true), None);
        assert_eq!(restore_default(false, false), None);
    }

    #[test]
    fn listed_toolchains_match_with_their_host() {
        let l = vec!["1.98.1-aarch64-apple-darwin".to_string(), "stable-x86_64-pc-windows-gnullvm".to_string()];
        assert!(has_toolchain(&l, "1.98.1"));
        assert!(!has_toolchain(&l, "1.98"));
        assert!(!has_toolchain(&l, "1.98.1-x86_64-pc-windows-gnullvm"));
        assert!(has_toolchain(&l, "stable-x86_64-pc-windows-gnullvm"));
    }
}

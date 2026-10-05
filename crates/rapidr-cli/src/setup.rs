//! `rapidr setup`: what an installed RapidR needs beyond itself.
//!
//! - Rust, for native builds (`rapidr build`): installed with rustup (MIT /
//!   Apache-2.0) into the user's `~/.cargo` and `~/.rustup`, after saying so
//!   and asking. Interpreted programs and `--interp` executables need none.
//! - On Windows, the linker Rust uses: the open-source LLVM-MinGW toolchain
//!   (`--toolchain gnullvm`, RapidR's recommendation: no Microsoft licence)
//!   or Microsoft's C++ Build Tools (`--toolchain msvc`, what rustup offers).
//! - The `rapidr` command on PATH, when it isn't (the macOS app, a
//!   `.tar.gz`): a link in `/usr/local/bin` or `~/.local/bin`.
//!
//! `rapidr setup --check` only reports.

use std::env;
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use crate::home::{rust_tool, Home};

/// The Rust version `rustc --version` reports.
fn rustc_version() -> Option<String> {
    let out = Command::new(rust_tool("rustc")).arg("--version").output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    text.split_whitespace().nth(1).map(str::to_string)
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
    let toolchain = args.iter().position(|a| a == "--toolchain").and_then(|i| args.get(i + 1)).map(String::as_str).unwrap_or("gnullvm");
    if !["gnullvm", "msvc"].contains(&toolchain) {
        eprintln!("--toolchain {toolchain}: gnullvm (LLVM-MinGW, open source) or msvc (Microsoft's C++ Build Tools)");
        return ExitCode::from(2);
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
    let need = home.as_ref().and_then(|h| h.release.as_ref()).map(|r| r.rust.clone()).unwrap_or_default();
    let builds = home.as_ref().is_some_and(Home::can_build);
    let mut ok = true;

    // 1. Rust (native builds)
    match rustc_version() {
        Some(v) if at_least(&v, &need) => println!("rust: {v} (native builds: ready)"),
        Some(v) => {
            println!("rust: {v}, older than the {need} this RapidR was tested with");
            if !check && builds && confirm("Update Rust with `rustup update stable`?", yes) {
                ok &= run_ok(Command::new(rust_tool("rustup")).args(["update", "stable"]));
            }
        }
        None if !builds => println!("rust: not installed (this runtime runs programs; the RapidR SDK builds them)"),
        None => {
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
                    ok &= install_rust(toolchain);
                } else {
                    ok = false;
                }
            }
        }
    }
    if cfg!(windows) && builds {
        check_windows_linker(toolchain);
    }

    // 2. `rapidr` on PATH
    if let Ok(exe) = env::current_exe().and_then(|e| e.canonicalize()) {
        let on_path = env::var_os("PATH").is_some_and(|p| env::split_paths(&p).any(|d| d.join(exe.file_name().unwrap_or_default()).canonicalize().ok().as_deref() == Some(exe.as_path())));
        if on_path {
            println!("command: {} is on PATH", exe.display());
        } else if cfg!(unix) {
            println!("command: {} is not on PATH", exe.display());
            if !check {
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

/// rustup's installer: stable Rust (at least what the release was tested
/// with), the minimal profile (rustc, cargo, rust-std).
fn install_rust(toolchain: &str) -> bool {
    let version = "stable";
    if cfg!(windows) {
        let arch = if env::consts::ARCH == "aarch64" { "aarch64" } else { "x86_64" };
        let host = format!("{arch}-pc-windows-{toolchain}");
        let url = format!("https://static.rust-lang.org/rustup/dist/{arch}-pc-windows-msvc/rustup-init.exe");
        let init = env::temp_dir().join("rustup-init.exe");
        let ps = format!("Invoke-WebRequest -UseBasicParsing -Uri '{url}' -OutFile '{}'", init.display());
        if !run_ok(Command::new("powershell").args(["-NoProfile", "-Command", &ps])) {
            return false;
        }
        let done = run_ok(Command::new(&init).args(["-y", "--profile", "minimal", "--default-toolchain", version, "--default-host", &host]));
        let _ = std::fs::remove_file(&init);
        done
    } else {
        let script = format!("curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain {version}");
        run_ok(Command::new("sh").args(["-c", &script]))
    }
}

fn windows_toolchain_note(toolchain: &str) -> &'static str {
    if toolchain == "gnullvm" {
        "On Windows Rust needs a linker. RapidR uses LLVM-MinGW (open source: Apache-2.0 with LLVM exception,\n\
         mingw-w64's permissive licences; what it links carries no obligations): download the\n\
         llvm-mingw-<version>-ucrt-<x86_64|aarch64>.zip for this machine from\n\
         https://github.com/mstorsjo/llvm-mingw/releases, unzip it, and put its bin folder on PATH.\n\
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
    let dir = if writable(&system) { system } else { dirs::home_dir().map(|h| h.join(".local/bin")).unwrap_or(system) };
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

//! Where a build's runners come from: ONE function every build asks for a
//! runner's bytes, [`runner_bytes`], and one for the notices that go with
//! it, [`runner_notices`] (docs/build-core-design.md §3).
//!
//! The signature is the RUNNERS lane's (crates/rapidr-runners: the
//! install's runner, the cache, the verified download). Until that crate
//! is merged this file is the small stand-in with the same signature — an
//! install's own runners, and in a source checkout this machine's runner
//! built with cargo — so the two branches join here and nowhere else.
//!
//! `target` is an install's runner folder name (`windows-x86_64`,
//! `windows-aarch64`, `windows-x86`, `macos`, `linux-x86_64`,
//! `linux-aarch64`, `web`: rapidr_package::Target::runner_dir) or an alias
//! (`windows-x64`, `linux-arm64`, `macos-arm64` → the universal `macos`
//! runner: the packer takes the slice out). `windowed`: Windows' windowed
//! runner (`rapidrintr-runnerw.exe`); ignored elsewhere. (The packer sets
//! the subsystem itself too, so either Windows runner makes either app:
//! the two files differ only in that field, the time stamp and the
//! checksum.) The web's runner is the web runtime as one tar
//! (rapidr_package::web::runtime_tar). Errors are finished sentences for
//! the person building.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use rapidr_package::Target;

use crate::home::{self, Home};
use crate::notices;

/// The runner's file name in an install.
fn runner_file(target: Target, windowed: bool) -> String {
    match target.system() {
        Some(rapidr_package::System::Windows) => format!("rapidrintr-runner{}.exe", if windowed { "w" } else { "" }),
        _ => "rapidrintr-runner".into(),
    }
}

fn parse(target: &str) -> Result<Target, String> {
    Target::parse(target).map(|t| if matches!(t, Target::MacosArm64 | Target::MacosX64) { Target::MacosUniversal } else { t }).map_err(|e| format!("{}.", e.trim_end_matches('.')))
}

/// The runner apps for `target` start from, as the packer takes it
/// (rapidr_package::Runner's bytes).
pub fn runner_bytes(target: &str, windowed: bool) -> Result<Vec<u8>, String> {
    let target = parse(target)?;
    let home = Home::find().ok_or("RapidR's home wasn't found (RAPIDR_HOME, an install, or a checkout).")?;
    if target == Target::Web {
        return web_runtime(&home);
    }
    if home.release.is_some() {
        let path = home.root.join("runners").join(target.runner_dir()).join(runner_file(target, windowed));
        return match fs::read(&path) {
            Ok(b) => Ok(b),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let shipped = home.runner_targets();
                Err(if shipped.is_empty() {
                    "This is the RapidR Runtime: it runs programs (`rapidr run`) and builds none. Executables need the RapidR SDK.".to_string()
                } else {
                    format!("There is no runner for {} in this install ({}); it has: {}.", target.label(), home.root.display(), shipped.join(", "))
                })
            }
            Err(e) => Err(format!("{}: {e}.", path.display())),
        };
    }
    checkout_runner(&home, target, windowed)
}

/// The notices that go with `target`'s runner (its THIRD-PARTY-NOTICES.txt:
/// once crates/rapidr-runners is merged, the one in its bundle).
pub fn runner_notices(target: &str) -> Result<String, String> {
    let t = parse(target)?;
    let kind = match t {
        Target::Web => notices::Kind::Web,
        t => notices::Kind::Desktop(t.runner_dir().to_string()),
    };
    notices::text(&kind).map_err(|e| format!("The open-source notices for {}'s runner aren't here ({e}): an app isn't shipped without them.", t.label()))
}

/// Whether this machine is `target` (a checkout builds only its own runner).
fn is_host(target: Target) -> bool {
    match target {
        Target::MacosUniversal | Target::MacosArm64 | Target::MacosX64 => cfg!(target_os = "macos"),
        t => Target::host() == Some(t),
    }
}

/// A source checkout's runner: this machine's, built with cargo (the
/// stripped `runner` profile; a quick no-op when it is up to date, so it
/// never lags behind the CLI); one already built is used if cargo can't run.
fn checkout_runner(home: &Home, target: Target, windowed: bool) -> Result<Vec<u8>, String> {
    if !is_host(target) {
        return Err(format!(
            "There is no runner for {} here: a source checkout builds this machine's runner only ({}); an installed RapidR has the others.",
            target.label(),
            home::system_name(&home::host_arch_target())
        ));
    }
    let name = if windowed && cfg!(windows) { "rapidrintr-runnerw" } else { "rapidrintr-runner" };
    let exe_name = format!("{name}{}", env::consts::EXE_SUFFIX);
    let mut args = vec!["build", "--quiet", "-p", "rapidr-runner-stub", "--bin", name, "--profile", "runner"];
    // (a development `rapidr` builds a development runner: its test hooks
    // act without a token, SEC-24)
    if rapidr_testmode::DEV_BUILD {
        args.extend(["--features", "dev-hooks"]);
    }
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    // (where cargo puts it: CARGO_TARGET_DIR when set, relative to where
    // rapidr was run, else the workspace's target/)
    let target_dir = env::var_os("CARGO_TARGET_DIR").map(|t| cwd.join(t)).unwrap_or_else(|| home.root.join("target"));
    let built = process::Command::new(home::rust_tool("cargo")).args(&args).current_dir(&home.root).env("CARGO_TARGET_DIR", &target_dir).status();
    let path = target_dir.join("runner").join(&exe_name);
    match built {
        Ok(status) if status.success() && path.exists() => return read(&path),
        Ok(status) => eprintln!("warning: cargo build rapidr-runner-stub failed ({status}); using an existing runner if there is one"),
        Err(e) => eprintln!("warning: can't run cargo ({e}); using an existing runner if there is one"),
    }
    for profile in ["runner", "release", "debug"] {
        let candidate = target_dir.join(profile).join(&exe_name);
        if candidate.exists() {
            return read(&candidate);
        }
    }
    Err(format!("{exe_name} could not be built or found (cargo build -p rapidr-runner-stub)."))
}

fn read(path: &Path) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|e| format!("{}: {e}.", path.display()))
}

/// The web runtime as one file: an install's `web/` (or a checkout's
/// `target/web`) — rapidrintr.js, rapidrintr_bg.wasm, the fonts — in a tar.
fn web_runtime(home: &Home) -> Result<Vec<u8>, String> {
    let dirs: Vec<PathBuf> = if home.release.is_some() {
        vec![home.root.join("web")]
    } else {
        let target_dir = env::var_os("CARGO_TARGET_DIR").map(PathBuf::from);
        let mut d: Vec<PathBuf> = ["target/web", "target/web-bundle"].iter().map(|d| home.root.join(d)).collect();
        if let Some(t) = target_dir {
            d.insert(0, t.join("web"));
        }
        d
    };
    for dir in &dirs {
        let (js, wasm) = (dir.join(rapidr_package::web::RUNTIME_JS), dir.join(rapidr_package::web::RUNTIME_WASM));
        if !(js.is_file() && wasm.is_file()) {
            continue;
        }
        let js = fs::read_to_string(&js).map_err(|e| format!("{}: {e}.", js.display()))?;
        let wasm = read(&wasm)?;
        let mut fonts = Vec::new();
        for e in fs::read_dir(dir.join("fonts")).into_iter().flatten().flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.ends_with(".otf") || name.ends_with(".ttf") || name == "index.json" || name == "OFL.txt" {
                fonts.push((name, read(&e.path())?));
            }
        }
        if !fonts.iter().any(|(n, _)| n == "index.json") {
            eprintln!("warning: no fallback fonts in {} (tools/build_web_artifacts.sh makes them): characters the built-in fonts lack will show as boxes", dir.join("fonts").display());
            fonts.clear();
        }
        fonts.sort_by(|a, b| a.0.cmp(&b.0));
        return rapidr_package::web::runtime_tar(&js, &wasm, &fonts);
    }
    Err(format!(
        "There is no web runtime (rapidrintr.js and rapidrintr_bg.wasm) in {}{}.",
        dirs.iter().map(|d| d.display().to_string()).collect::<Vec<_>>().join(" or "),
        if home.release.is_some() { "" } else { ": tools/build_web_artifacts.sh makes it" }
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_messages() {
        assert_eq!(runner_file(Target::WindowsX86, false), "rapidrintr-runner.exe");
        assert_eq!(runner_file(Target::WindowsArm64, true), "rapidrintr-runnerw.exe");
        assert_eq!(runner_file(Target::LinuxX64, true), "rapidrintr-runner");
        // (the aliases: install names, the brief's names, the thin macOS ones)
        assert_eq!(parse("windows-x86_64").unwrap(), Target::WindowsX64);
        assert_eq!(parse("linux-arm64").unwrap(), Target::LinuxArm64);
        assert_eq!(parse("macos-arm64").unwrap(), Target::MacosUniversal);
        assert!(parse("beos").unwrap_err().ends_with('.'), "a finished sentence");
        // (a checkout: another system's runner is said plainly)
        let other = if cfg!(target_os = "linux") { Target::WindowsX64 } else { Target::LinuxArm64 };
        if let Some(home) = Home::find().filter(|h| h.release.is_none()) {
            let e = checkout_runner(&home, other, false).unwrap_err();
            assert!(e.contains(other.label()) && e.contains("this machine's runner only") && e.ends_with('.'), "{e}");
        }
    }
}

//! The targets a runner is made for, and what a runner is made of.
//!
//! A target is named as an install's `runners/<target>/` folder is
//! (crates/rapidr-cli/src/home.rs): `macos` (one universal runner: Apple
//! silicon and Intel; a thin `macos-arm64` / `macos-x86_64` is a slice of
//! it), `windows-x86_64`, `windows-aarch64`, `windows-x86` (Windows 7 and
//! later, 32-bit), `linux-x86_64`, `linux-aarch64`, and `web` (the web
//! interpreter). People and other tools say `x64`, `arm64`, `amd64`,
//! `i686`: [`canonical`] turns those into the one name.

/// Every target RapidR makes a runner for, in the order a dialog lists them.
pub const TARGETS: &[&str] = &["windows-x86_64", "windows-aarch64", "windows-x86", "macos", "linux-x86_64", "linux-aarch64", "web"];

/// The file in every bundle that carries the runner's third-party notices
/// (`THIRD-PARTY-NOTICES.txt`: an app built from the runner ships it).
pub const NOTICES_FILE: &str = "THIRD-PARTY-NOTICES.txt";

/// The target `name` means, or `None` when it is no target RapidR knows.
/// Case does not matter; `-` and `_` are alike.
///
/// ```
/// use rapidr_runners::names::canonical;
/// assert_eq!(canonical("Windows-x64"), Some("windows-x86_64"));
/// assert_eq!(canonical("linux_arm64"), Some("linux-aarch64"));
/// assert_eq!(canonical("macos-arm64"), Some("macos"));
/// assert_eq!(canonical("amiga"), None);
/// ```
pub fn canonical(name: &str) -> Option<&'static str> {
    let n = name.trim().to_ascii_lowercase().replace('_', "-");
    let (os, arch) = match n.split_once('-') {
        Some((os, arch)) => (os, Some(arch)),
        None => (n.as_str(), None),
    };
    match (os, arch) {
        ("web", None) => Some("web"),
        ("macos", _) => Some("macos"),
        ("windows", Some("x86-64" | "x64" | "amd64")) => Some("windows-x86_64"),
        ("windows", Some("aarch64" | "arm64")) => Some("windows-aarch64"),
        ("windows", Some("x86" | "i686" | "i386" | "x32" | "32")) => Some("windows-x86"),
        ("linux", Some("x86-64" | "x64" | "amd64")) => Some("linux-x86_64"),
        ("linux", Some("aarch64" | "arm64")) => Some("linux-aarch64"),
        _ => None,
    }
}

/// The target as people say it: "Windows 64-bit (x64)", "Linux ARM64", …
pub fn describe(target: &str) -> String {
    match target {
        "windows-x86_64" => "Windows 64-bit (x64)".into(),
        "windows-aarch64" => "Windows ARM64".into(),
        "windows-x86" => "Windows 32-bit (x86)".into(),
        "macos" => "macOS (Apple silicon and Intel)".into(),
        "linux-x86_64" => "Linux 64-bit (x64)".into(),
        "linux-aarch64" => "Linux ARM64".into(),
        "web" => "web (browser)".into(),
        other => other.to_string(),
    }
}

/// A Windows target: its runners are `.exe` files, and it has a console
/// runner and a windowed one.
pub fn is_windows(target: &str) -> bool {
    target.starts_with("windows-")
}

/// The runner program for `target`: `rapidrintr-runner`, with `.exe` on
/// Windows. There is ONE per target: Windows' console and windowed runners
/// (`rapidrintr-runner.exe` and `rapidrintr-runnerw.exe` in an install) differ
/// in four bytes of the PE header (the time stamp, the checksum and the
/// subsystem), and the packer sets the subsystem itself, so a download
/// carries the console one only. `windowed` is therefore ignored; it stays in
/// the call so a build says what it wants (and a later target may need it).
pub fn runner_file(target: &str, _windowed: bool) -> &'static str {
    if is_windows(target) {
        "rapidrintr-runner.exe"
    } else {
        "rapidrintr-runner"
    }
}

/// The files a bundle for `target` must have: its runner(s) and the notices.
/// (The web's interpreter is `rapidrintr.js` and `rapidrintr_bg.wasm`, and
/// its fallback fonts in `fonts/`.)
pub fn required_files(target: &str) -> Vec<&'static str> {
    let mut files = match target {
        "web" => vec!["rapidrintr.js", "rapidrintr_bg.wasm"],
        t if is_windows(t) => vec!["rapidrintr-runner.exe"],
        _ => vec!["rapidrintr-runner"],
    };
    files.push(NOTICES_FILE);
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_target_is_its_own_canonical_name() {
        for t in TARGETS {
            assert_eq!(canonical(t), Some(*t));
        }
    }

    #[test]
    fn the_names_people_use() {
        for (said, target) in [
            ("windows-x64", "windows-x86_64"),
            ("Windows-AMD64", "windows-x86_64"),
            ("windows_arm64", "windows-aarch64"),
            ("windows-i686", "windows-x86"),
            ("windows-32", "windows-x86"),
            ("macos-universal", "macos"),
            ("macos-x86_64", "macos"),
            ("macos-arm64", "macos"),
            ("linux-x64", "linux-x86_64"),
            ("linux-arm64", "linux-aarch64"),
            (" web ", "web"),
        ] {
            assert_eq!(canonical(said), Some(target), "{said}");
        }
        for nope in ["", "windows", "linux", "web-x64", "ios", "windows-ppc", "linux-x86"] {
            assert_eq!(canonical(nope), None, "{nope}");
        }
    }

    #[test]
    fn one_runner_per_target() {
        // (windowed or console: the same program; the packer sets the subsystem)
        assert_eq!(runner_file("windows-x86", true), "rapidrintr-runner.exe");
        assert_eq!(runner_file("windows-aarch64", false), "rapidrintr-runner.exe");
        assert_eq!(runner_file("linux-x86_64", true), "rapidrintr-runner");
        assert_eq!(runner_file("macos", false), "rapidrintr-runner");
        assert_eq!(required_files("windows-x86_64"), ["rapidrintr-runner.exe", NOTICES_FILE]);
        assert!(required_files("web").contains(&"rapidrintr_bg.wasm"));
        assert!(required_files("macos").contains(&NOTICES_FILE));
    }
}

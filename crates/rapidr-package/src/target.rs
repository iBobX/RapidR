//! What an app is built for: a system and its processor
//! (docs/build-core-design.md §2).

use crate::System;

/// A system and processor an app is built for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Target {
    /// Windows on x64 (most PCs since 2005).
    WindowsX64,
    /// Windows on ARM64 (Snapdragon PCs).
    WindowsArm64,
    /// 32-bit Windows (7, 8, 8.1 and 10/11 32-bit): the fast interpreter, no JIT.
    WindowsX86,
    /// One macOS app for Apple silicon and Intel Macs.
    MacosUniversal,
    /// Linux on x64.
    LinuxX64,
    /// Linux on ARM64 (also Raspberry Pi 4/5 with a 64-bit system).
    LinuxArm64,
    /// A web page, for any static web host.
    Web,
    /// A macOS app for Apple silicon only (one slice of the universal runner).
    MacosArm64,
    /// A macOS app for Intel Macs only.
    MacosX64,
}

impl Target {
    /// What `--all` builds (and the Build dialog's cards offer): the thin
    /// macOS ones are left out, the universal app runs on both.
    pub const ALL: [Target; 7] = [Target::WindowsX64, Target::WindowsArm64, Target::WindowsX86, Target::MacosUniversal, Target::LinuxX64, Target::LinuxArm64, Target::Web];

    /// The target a name says: `windows-x64`, `macos` … (the names RapidR's
    /// runners and older builds used are taken too: `windows-x86_64`,
    /// `linux-aarch64`, `macos-x86_64`).
    pub fn parse(name: &str) -> Result<Target, String> {
        let n = name.trim().to_ascii_lowercase().replace('_', "-");
        let t = match n.as_str() {
            "windows-x64" | "windows-x86-64" | "windows-amd64" | "win-x64" | "win64" => Target::WindowsX64,
            "windows-arm64" | "windows-aarch64" | "win-arm64" => Target::WindowsArm64,
            "windows-x86" | "windows-i686" | "windows-i386" | "windows-32" | "win-x86" | "win32" => Target::WindowsX86,
            "macos" | "macos-universal" | "mac" | "macos-universal2" => Target::MacosUniversal,
            "macos-arm64" | "macos-aarch64" => Target::MacosArm64,
            "macos-x64" | "macos-x86-64" | "macos-intel" => Target::MacosX64,
            "linux-x64" | "linux-x86-64" | "linux-amd64" => Target::LinuxX64,
            "linux-arm64" | "linux-aarch64" => Target::LinuxArm64,
            "web" | "wasm" | "browser" => Target::Web,
            _ => {
                let names: Vec<&str> = Target::ALL.iter().map(|t| t.name()).collect();
                return Err(format!("\"{name}\" is not a system RapidR builds for: {} (or all)", names.join(", ")));
            }
        };
        Ok(t)
    }

    /// A list of targets: `windows-x64,macos,web` (commas or spaces), or
    /// `all`. Each once, in [`Target::ALL`]'s order.
    pub fn parse_list(list: &str) -> Result<Vec<Target>, String> {
        let mut out = Vec::new();
        for part in list.split([',', ' ', ';']).map(str::trim).filter(|p| !p.is_empty()) {
            if part.eq_ignore_ascii_case("all") {
                out.extend(Target::ALL);
            } else {
                out.push(Target::parse(part)?);
            }
        }
        if out.is_empty() {
            return Err("no system named: windows-x64, macos, linux-x64, web … (or all)".into());
        }
        out.sort();
        out.dedup();
        Ok(out)
    }

    /// Its name in file names and on the command line: `windows-x64`.
    pub fn name(self) -> &'static str {
        match self {
            Target::WindowsX64 => "windows-x64",
            Target::WindowsArm64 => "windows-arm64",
            Target::WindowsX86 => "windows-x86",
            Target::MacosUniversal => "macos-universal",
            Target::MacosArm64 => "macos-arm64",
            Target::MacosX64 => "macos-x64",
            Target::LinuxX64 => "linux-x64",
            Target::LinuxArm64 => "linux-arm64",
            Target::Web => "web",
        }
    }

    /// As people say it: `Windows (64-bit, x64)`.
    pub fn label(self) -> &'static str {
        match self {
            Target::WindowsX64 => "Windows (64-bit, x64)",
            Target::WindowsArm64 => "Windows (ARM64)",
            Target::WindowsX86 => "Windows (32-bit, x86)",
            Target::MacosUniversal => "macOS (Apple silicon and Intel)",
            Target::MacosArm64 => "macOS (Apple silicon)",
            Target::MacosX64 => "macOS (Intel)",
            Target::LinuxX64 => "Linux (x64)",
            Target::LinuxArm64 => "Linux (ARM64)",
            Target::Web => "the web",
        }
    }

    /// The desktop system it is, `None` for the web.
    pub fn system(self) -> Option<System> {
        match self {
            Target::WindowsX64 | Target::WindowsArm64 | Target::WindowsX86 => Some(System::Windows),
            Target::MacosUniversal | Target::MacosArm64 | Target::MacosX64 => Some(System::MacOs),
            Target::LinuxX64 | Target::LinuxArm64 => Some(System::Linux),
            Target::Web => None,
        }
    }

    /// The folder an install keeps its runner in (`runners/<this>/`), and
    /// the notices' name (`notices/<this>.txt`): RapidR's release layout.
    /// The thin macOS targets take their slice out of the universal runner.
    pub fn runner_dir(self) -> &'static str {
        match self {
            Target::WindowsX64 => "windows-x86_64",
            Target::WindowsArm64 => "windows-aarch64",
            Target::WindowsX86 => "windows-x86",
            Target::MacosUniversal | Target::MacosArm64 | Target::MacosX64 => "macos",
            Target::LinuxX64 => "linux-x86_64",
            Target::LinuxArm64 => "linux-aarch64",
            Target::Web => "web",
        }
    }

    /// The archive it is shipped in: `zip`, or `tar.gz` for Linux (a zip
    /// loses the exec bit there).
    pub fn archive_ext(self) -> &'static str {
        match self.system() {
            Some(System::Linux) => "tar.gz",
            _ => "zip",
        }
    }

    /// Whether its runner carries the JIT (32-bit x86 has none: the fast
    /// interpreter only; the web runs bytecode in wasm).
    pub fn has_jit(self) -> bool {
        !matches!(self, Target::WindowsX86 | Target::Web)
    }

    /// The machine this computer is (`None` where RapidR has no runner).
    pub fn host() -> Option<Target> {
        match (std::env::consts::OS, std::env::consts::ARCH) {
            ("macos", _) => Some(Target::MacosUniversal),
            ("windows", "x86_64") => Some(Target::WindowsX64),
            ("windows", "aarch64") => Some(Target::WindowsArm64),
            ("windows", "x86") => Some(Target::WindowsX86),
            ("linux", "x86_64") => Some(Target::LinuxX64),
            ("linux", "aarch64") => Some(Target::LinuxArm64),
            _ => None,
        }
    }
}

impl std::fmt::Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip_and_old_names_are_taken() {
        for t in Target::ALL.into_iter().chain([Target::MacosArm64, Target::MacosX64]) {
            assert_eq!(Target::parse(t.name()).unwrap(), t);
            assert_eq!(Target::parse(t.runner_dir()).map(|p| p.runner_dir()).unwrap(), t.runner_dir());
        }
        assert_eq!(Target::parse("windows-x86_64").unwrap(), Target::WindowsX64);
        assert_eq!(Target::parse("Linux-AArch64").unwrap(), Target::LinuxArm64);
        assert_eq!(Target::parse("macos").unwrap(), Target::MacosUniversal);
        assert_eq!(Target::parse("macos-x86_64").unwrap(), Target::MacosX64);
        let e = Target::parse("android").unwrap_err();
        assert!(e.contains("windows-x64") && e.contains("web"), "{e}");
    }

    #[test]
    fn lists() {
        assert_eq!(Target::parse_list("web, windows-x64 macos").unwrap(), [Target::WindowsX64, Target::MacosUniversal, Target::Web]);
        assert_eq!(Target::parse_list("all").unwrap(), Target::ALL);
        assert_eq!(Target::parse_list("all,web").unwrap().len(), 7);
        assert!(Target::parse_list(" , ").is_err());
        assert!(Target::parse_list("windows-x64,beos").is_err());
    }

    #[test]
    fn archives_and_systems() {
        assert_eq!(Target::LinuxArm64.archive_ext(), "tar.gz");
        assert_eq!(Target::MacosUniversal.archive_ext(), "zip");
        assert_eq!(Target::Web.archive_ext(), "zip");
        assert_eq!(Target::Web.system(), None);
        assert!(!Target::WindowsX86.has_jit() && Target::WindowsArm64.has_jit());
    }
}

//! What `rapidr build` makes a program into, for each system
//! (docs/manual/building-apps.md):
//!
//! - **macOS** ([`macos`]): `Name.app` — Info.plist, the program in
//!   Contents/MacOS, its icon as `.icns`, signed ad hoc;
//! - **Windows** ([`windows`]): the `.exe` with its icon (every size
//!   Explorer shows) and version information as resources;
//! - **Linux** ([`linux`]): `Name.AppDir` — the program, a desktop entry and
//!   hicolor icons from 16 to 512 px.
//!
//! The icon ([`Icon`]) is RapidR's own for compiled programs unless the
//! program has one: an `.icns`, `.ico`, `.png` or `.svg`, converted to each
//! system's format here, in pure Rust, so cross builds make them too.

pub mod icns;
pub mod ico;
mod icon;
mod info;
pub mod linux;
pub mod macos;
pub mod picture;
pub mod windows;

pub use icon::{Icon, Look, FORMATS};
pub use info::{default_bundle_id, AppInfo};

/// The system a build is for, from a target `<os>-<arch>`
/// (`macos-arm64`, `windows-x86_64`, `linux-aarch64`, or just `macos`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum System {
    MacOs,
    Windows,
    Linux,
}

impl System {
    pub fn of_target(target: &str) -> Option<System> {
        match target.split('-').next()? {
            "macos" => Some(System::MacOs),
            "windows" => Some(System::Windows),
            "linux" => Some(System::Linux),
            _ => None,
        }
    }

    /// The system this is running on.
    pub fn host() -> Option<System> {
        if cfg!(target_os = "macos") {
            Some(System::MacOs)
        } else if cfg!(windows) {
            Some(System::Windows)
        } else if cfg!(target_os = "linux") {
            Some(System::Linux)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn systems_of_targets() {
        assert_eq!(System::of_target("macos-arm64"), Some(System::MacOs));
        assert_eq!(System::of_target("macos"), Some(System::MacOs));
        assert_eq!(System::of_target("windows-x86_64"), Some(System::Windows));
        assert_eq!(System::of_target("linux-aarch64"), Some(System::Linux));
        assert_eq!(System::of_target("web"), None);
    }
}

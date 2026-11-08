//! The packer: what `rapidr build` makes a program into, for every system
//! (docs/build-core-design.md, docs/manual/building-apps.md). Pure: no file
//! system, no processes, no clock — bytes in, bytes out — so the CLI, the
//! desktop Studio (which runs the CLI) and the web Studio (compiled to
//! wasm32) all build with this one implementation.
//!
//! - [`pack()`]: a program (its bytecode, its data files) and a runner made
//!   into an app for a [`Target`], as [`Files`]:
//!   - **macOS** ([`macos`]): `Name.app` — Info.plist, the program in
//!     Contents/MacOS, its icon as `.icns`, signed ad hoc by RapidR's own
//!     signer ([`macho_sign`]);
//!   - **Windows** ([`windows`]): the `.exe` with its icon (every size
//!     Explorer shows), version information and manifest as resources;
//!   - **Linux** ([`linux`]): `Name.AppDir` — the program, a desktop entry,
//!     hicolor icons from 16 to 512 px, `install.sh`;
//!   - **the web** ([`web`]): the bundle, a web manifest and its icons.
//! - [`archive()`]: the app and its first-launch `README.txt` as the zip (a
//!   tar.gz for Linux) that is shipped; [`sha256sums`] for beside them.
//!
//! The icon ([`Icon`]) is RapidR's own for compiled programs unless the
//! program has one: an `.icns`, `.ico`, `.png` or `.svg`'s bytes, converted
//! to each system's format here.

pub mod files;
pub mod icns;
pub mod ico;
mod icon;
mod info;
pub mod linux;
pub mod macho;
pub mod macho_sign;
pub mod macos;
mod pack;
pub mod picture;
pub mod readme;
mod target;
pub mod web;
pub mod windows;

pub use files::{File, Files};
pub use icon::{Icon, Look, FORMATS};
pub use info::{default_bundle_id, exe_name_for, AppInfo, Category, DEFAULT_VERSION};
pub use pack::{archive, archive_name, build, pack, sha256_hex, sha256sums, with_payload, App, Archive, Packed, Runner, NOTICES_FILE, README_FILE};
pub use target::Target;

/// The desktop system a build is for, from a target `<os>-<arch>`
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

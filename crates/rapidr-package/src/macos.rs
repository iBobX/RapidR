//! macOS: a program as `Name.app` —
//!
//! ```text
//! Name.app/Contents/Info.plist          name, identifier, version, icon, minimum macOS
//! Name.app/Contents/PkgInfo             APPL????
//! Name.app/Contents/MacOS/<exe>         the program
//! Name.app/Contents/Resources/AppIcon.icns
//! Name.app/Contents/Resources/…         what else it carries (an interpreted program's .rrbc, notices)
//! ```
//!
//! signed ad hoc (`codesign --force --sign -`) so it opens on Apple silicon.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::info::{xml, AppInfo};
use crate::Icon;

/// The icon's file in Contents/Resources (and Info.plist's CFBundleIconFile).
pub const ICON_FILE: &str = "AppIcon.icns";

/// An app to write.
pub struct MacApp<'a> {
    pub info: &'a AppInfo,
    pub icon: &'a Icon,
    /// The program (Mach-O, thin or universal).
    pub executable: &'a [u8],
    /// More files for Contents/Resources: (name, contents).
    pub resources: Vec<(String, Vec<u8>)>,
    /// LSMinimumSystemVersion.
    pub minimum_system: &'a str,
}

/// Info.plist's text.
pub fn info_plist(info: &AppInfo, minimum_system: &str) -> Result<String, String> {
    info.check()?;
    let version = info.mac_version()?;
    let mut keys: Vec<(&str, String)> = vec![
        ("CFBundleDevelopmentRegion", "en".into()),
        ("CFBundleDisplayName", info.name.clone()),
        ("CFBundleExecutable", info.exe.clone()),
        ("CFBundleIconFile", ICON_FILE.trim_end_matches(".icns").into()),
        ("CFBundleIdentifier", info.bundle_id.clone()),
        ("CFBundleInfoDictionaryVersion", "6.0".into()),
        ("CFBundleName", info.name.clone()),
        ("CFBundlePackageType", "APPL".into()),
        ("CFBundleShortVersionString", version.clone()),
        ("CFBundleVersion", version),
        ("LSMinimumSystemVersion", minimum_system.into()),
        ("NSPrincipalClass", "NSApplication".into()),
    ];
    let copyright = info.copyright_line();
    if !copyright.is_empty() {
        keys.push(("NSHumanReadableCopyright", copyright));
    }
    let mut out = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n<dict>\n",
    );
    for (k, v) in keys {
        out.push_str(&format!("\t<key>{k}</key>\n\t<string>{}</string>\n", xml(&v)));
    }
    out.push_str("\t<key>NSHighResolutionCapable</key>\n\t<true/>\n</dict>\n</plist>\n");
    Ok(out)
}

/// Writes `folder/<name>.app` (replacing an app that's there); its path.
pub fn write(folder: &Path, app: &MacApp) -> Result<PathBuf, String> {
    let plist = info_plist(app.info, app.minimum_system)?;
    let path = folder.join(format!("{}.app", app.info.name));
    if path.exists() {
        if !path.join("Contents/Info.plist").is_file() {
            return Err(format!("{} is there and isn't an app: move it away first", path.display()));
        }
        std::fs::remove_dir_all(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    let contents = path.join("Contents");
    let io = |p: &Path, e: std::io::Error| format!("{}: {e}", p.display());
    for dir in ["MacOS", "Resources"] {
        std::fs::create_dir_all(contents.join(dir)).map_err(|e| io(&contents.join(dir), e))?;
    }
    let write = |p: PathBuf, bytes: &[u8]| std::fs::write(&p, bytes).map_err(|e| io(&p, e));
    write(contents.join("Info.plist"), plist.as_bytes())?;
    write(contents.join("PkgInfo"), b"APPL????")?;
    let exe = contents.join("MacOS").join(&app.info.exe);
    write(exe.clone(), app.executable)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).map_err(|e| io(&exe, e))?;
    }
    write(contents.join("Resources").join(ICON_FILE), &app.icon.icns())?;
    for (name, bytes) in &app.resources {
        write(contents.join("Resources").join(name), bytes)?;
    }
    Ok(path)
}

/// The path as `codesign` is given it: absolute. `codesign --verify` reads an
/// argument starting with a digit as a process id ("3dcube.app: No such
/// process"), and one starting with `-` as an option; an absolute path starts
/// with neither (a relative one does when the app's folder is the current one:
/// `rapidr build 3dcube.bas` makes `3dcube.app`).
fn codesign_arg(app: &Path) -> PathBuf {
    std::path::absolute(app).unwrap_or_else(|_| Path::new(".").join(app))
}

/// Signs the app ad hoc (`codesign --force --sign -`: no identity, but a
/// valid signature, which Apple silicon needs to run it) and checks the
/// signature. `Ok(false)`: not on macOS (codesign is macOS'), left unsigned.
pub fn sign(app: &Path) -> Result<bool, String> {
    if !cfg!(target_os = "macos") {
        return Ok(false);
    }
    let app = codesign_arg(app);
    let run = |args: &[&str]| -> Result<(), String> {
        let out = Command::new("codesign").args(args).arg(&app).output().map_err(|e| format!("codesign: {e}"))?;
        if out.status.success() {
            Ok(())
        } else {
            Err(format!("codesign {}: {}", args.join(" "), String::from_utf8_lossy(&out.stderr).trim()))
        }
    };
    run(&["--force", "--sign", "-"])?;
    run(&["--verify", "--strict"])?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plist_has_every_key() {
        let mut info = AppInfo::new("Note & Pad", "notepad");
        info.version = "2.1.0.5".into();
        info.company = "Ruta Internet SRL".into();
        let p = info_plist(&info, "10.13").unwrap();
        for want in [
            "<key>CFBundleName</key>\n\t<string>Note &amp; Pad</string>",
            "<key>CFBundleDisplayName</key>",
            "<key>CFBundleIdentifier</key>\n\t<string>dev.rapidr.app.note-pad</string>",
            "<key>CFBundleExecutable</key>\n\t<string>notepad</string>",
            "<key>CFBundleIconFile</key>\n\t<string>AppIcon</string>",
            "<key>CFBundleShortVersionString</key>\n\t<string>2.1.0</string>",
            "<key>LSMinimumSystemVersion</key>\n\t<string>10.13</string>",
            "<key>NSHighResolutionCapable</key>\n\t<true/>",
            "<string>© Ruta Internet SRL</string>",
        ] {
            assert!(p.contains(want), "{want}\n{p}");
        }
        info.version = "one".into();
        assert!(info_plist(&info, "10.13").is_err());
    }

    #[test]
    fn codesign_is_given_an_absolute_path() {
        // (a relative `3dcube.app` is read by `codesign --verify` as process 3; `-x.app` as an option)
        for name in ["3dcube.app", "-x.app", "./3dcube.app", "7/8.app"] {
            let arg = codesign_arg(Path::new(name));
            assert!(arg.is_absolute(), "{name} -> {}", arg.display());
            assert!(arg.ends_with(name.trim_start_matches("./")), "{name} -> {}", arg.display());
        }
        let abs = std::env::temp_dir().join("3dcube.app");
        assert_eq!(codesign_arg(&abs), abs);
    }

    #[test]
    fn writes_the_bundle() {
        let dir = std::env::temp_dir().join(format!("rapidr-package-mac-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let info = AppInfo::new("Hello", "hello");
        let icon = Icon::rapidr_default();
        let app = MacApp { info: &info, icon: &icon, executable: b"#!/bin/sh\necho hi\n", resources: vec![("hello.rrbc".into(), b"RRBC".to_vec())], minimum_system: "10.13" };
        let path = write(&dir, &app).unwrap();
        assert_eq!(path, dir.join("Hello.app"));
        for f in ["Contents/Info.plist", "Contents/PkgInfo", "Contents/MacOS/hello", "Contents/Resources/AppIcon.icns", "Contents/Resources/hello.rrbc"] {
            assert!(path.join(f).is_file(), "{f}");
        }
        // again: replaced
        write(&dir, &app).unwrap();
        // something else with the name: left alone
        std::fs::create_dir_all(dir.join("Other.app")).unwrap();
        let other = AppInfo::new("Other", "other");
        assert!(write(&dir, &MacApp { info: &other, ..app }).unwrap_err().contains("isn't an app"));
        if cfg!(target_os = "macos") {
            assert!(sign(&path).unwrap());
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

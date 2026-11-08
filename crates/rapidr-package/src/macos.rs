//! macOS: a program as `Name.app` —
//!
//! ```text
//! Name.app/Contents/Info.plist          name, identifier, version, category, icon, minimum macOS
//! Name.app/Contents/PkgInfo             APPL????
//! Name.app/Contents/MacOS/<exe>         the program (0755)
//! Name.app/Contents/Resources/AppIcon.icns
//! Name.app/Contents/Resources/…         what else it carries (an interpreted program's .rrbc, data, notices)
//! ```
//!
//! made in memory ([`app_files`]) and signed ad hoc ([`sign_ad_hoc`]) so it
//! opens on Apple silicon.

use crate::files::Files;
use crate::info::{xml, AppInfo};
use crate::Icon;

/// The icon's file in Contents/Resources (and Info.plist's CFBundleIconFile).
pub const ICON_FILE: &str = "AppIcon.icns";

/// The oldest macOS RapidR's executables run on (LSMinimumSystemVersion):
/// macOS High Sierra on Intel; Apple silicon starts at macOS 11.
pub const MINIMUM_SYSTEM: &str = "10.13";

/// An app to make.
pub struct MacApp<'a> {
    pub info: &'a AppInfo,
    pub icon: &'a Icon,
    /// The program (Mach-O, thin or universal).
    pub executable: &'a [u8],
    /// More files for Contents/Resources: (path there, `/`-separated; contents).
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
        ("LSApplicationCategoryType", info.category.mac().into()),
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

/// The app's folder name: `Name.app`.
pub fn app_folder(info: &AppInfo) -> String {
    format!("{}.app", info.name)
}

/// The app's files, under `Name.app/` (not signed yet: [`sign_ad_hoc`]).
pub fn app_files(app: &MacApp) -> Result<Files, String> {
    let plist = info_plist(app.info, app.minimum_system)?;
    let root = app_folder(app.info);
    let mut f = Files::new();
    f.add(&format!("{root}/Contents/Info.plist"), plist.into_bytes())?;
    f.add(&format!("{root}/Contents/PkgInfo"), b"APPL????".to_vec())?;
    f.add_executable(&format!("{root}/Contents/MacOS/{}", app.info.exe), app.executable.to_vec())?;
    f.add(&format!("{root}/Contents/Resources/{ICON_FILE}"), app.icon.icns())?;
    for (name, bytes) in &app.resources {
        // (in their folders: `lang/es.po`, `art/logo.svg`)
        f.add(&format!("{root}/Contents/Resources/{name}"), bytes.clone())?;
    }
    Ok(f)
}

/// Whether an app's signature is made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signature {
    /// Signed ad hoc (no identity: what Apple silicon needs to run it).
    AdHoc,
    /// Not signed yet: the caller signs it (on a Mac, the CLI's
    /// `codesign_bridge`) — until RapidR's own signer is here.
    NotYet,
}

/// Signs the app at `app_root` (`Name.app`) in `files` ad hoc, in memory:
/// its main executable (every slice) and the bundle's
/// `_CodeSignature/CodeResources`.
///
/// THE CALL SITE for RapidR's own Mach-O signer (docs/build-core-design.md
/// §6, the MACHO-SIGN lane's `macho_sign.rs`): every macOS app any build
/// makes passes here, folder builds and archives alike, on every system
/// and in the browser. Until the signer is here it signs nothing and says
/// so ([`Signature::NotYet`]).
pub fn sign_ad_hoc(files: &mut Files, app_root: &str, info: &AppInfo) -> Result<Signature, String> {
    let _ = (files, app_root, info);
    Ok(Signature::NotYet)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plist_has_every_key() {
        let mut info = AppInfo::new("Note & Pad", "notepad");
        info.version = "2.1.0.5".into();
        info.company = "Ruta Internet SRL".into();
        info.category = crate::Category::Games;
        let p = info_plist(&info, "10.13").unwrap();
        for want in [
            "<key>CFBundleName</key>\n\t<string>Note &amp; Pad</string>",
            "<key>CFBundleDisplayName</key>",
            "<key>CFBundleIdentifier</key>\n\t<string>dev.rapidr.app.note-pad</string>",
            "<key>CFBundleExecutable</key>\n\t<string>notepad</string>",
            "<key>CFBundleIconFile</key>\n\t<string>AppIcon</string>",
            "<key>CFBundleShortVersionString</key>\n\t<string>2.1.0</string>",
            "<key>LSApplicationCategoryType</key>\n\t<string>public.app-category.games</string>",
            "<key>LSMinimumSystemVersion</key>\n\t<string>10.13</string>",
            "<key>NSHighResolutionCapable</key>\n\t<true/>",
            "<string>© Ruta Internet SRL</string>",
        ] {
            assert!(p.contains(want), "{want}\n{p}");
        }
        info.version = "one".into();
        assert!(info_plist(&info, "10.13").is_err());
    }

    /// The whole Info.plist of a typical app, byte for byte: what Finder's
    /// Get Info and the Dock read.
    #[test]
    fn plist_golden() {
        let mut info = AppInfo::new("Tip Calculator", "Tip-Calculator");
        info.version = "1.2.0".into();
        info.bundle_id = "com.example.tips".into();
        info.copyright = "(c) 2026 Example Ltd".into();
        info.category = crate::Category::Finance;
        let golden = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">
<plist version=\"1.0\">
<dict>
\t<key>CFBundleDevelopmentRegion</key>
\t<string>en</string>
\t<key>CFBundleDisplayName</key>
\t<string>Tip Calculator</string>
\t<key>CFBundleExecutable</key>
\t<string>Tip-Calculator</string>
\t<key>CFBundleIconFile</key>
\t<string>AppIcon</string>
\t<key>CFBundleIdentifier</key>
\t<string>com.example.tips</string>
\t<key>CFBundleInfoDictionaryVersion</key>
\t<string>6.0</string>
\t<key>CFBundleName</key>
\t<string>Tip Calculator</string>
\t<key>CFBundlePackageType</key>
\t<string>APPL</string>
\t<key>CFBundleShortVersionString</key>
\t<string>1.2.0</string>
\t<key>CFBundleVersion</key>
\t<string>1.2.0</string>
\t<key>LSApplicationCategoryType</key>
\t<string>public.app-category.finance</string>
\t<key>LSMinimumSystemVersion</key>
\t<string>10.13</string>
\t<key>NSPrincipalClass</key>
\t<string>NSApplication</string>
\t<key>NSHumanReadableCopyright</key>
\t<string>(c) 2026 Example Ltd</string>
\t<key>NSHighResolutionCapable</key>
\t<true/>
</dict>
</plist>
";
        assert_eq!(info_plist(&info, MINIMUM_SYSTEM).unwrap(), golden);
    }

    #[test]
    fn the_bundle_in_memory() {
        let info = AppInfo::new("Hello", "hello");
        let icon = Icon::rapidr_default();
        let app = MacApp { info: &info, icon: &icon, executable: b"\xcf\xfa\xed\xfe", resources: vec![("hello.rrbc".into(), b"RRBC".to_vec()), ("lang/es.po".into(), b"msgid".to_vec())], minimum_system: "10.13" };
        let f = app_files(&app).unwrap();
        for p in ["Hello.app/Contents/Info.plist", "Hello.app/Contents/PkgInfo", "Hello.app/Contents/MacOS/hello", "Hello.app/Contents/Resources/AppIcon.icns", "Hello.app/Contents/Resources/hello.rrbc", "Hello.app/Contents/Resources/lang/es.po"] {
            assert!(f.contains(p), "{p}");
        }
        assert!(f.get("Hello.app/Contents/MacOS/hello").unwrap().executable);
        assert!(!f.get("Hello.app/Contents/Info.plist").unwrap().executable);
        // (a resource that would leave the bundle: refused)
        let bad = MacApp { resources: vec![("../../evil".into(), Vec::new())], ..app };
        assert!(app_files(&bad).is_err());
    }
}

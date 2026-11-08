//! macOS: a program as `Name.app` —
//!
//! ```text
//! Name.app/Contents/Info.plist          name, identifier, version, category, icon, minimum macOS
//! Name.app/Contents/PkgInfo             APPL????
//! Name.app/Contents/MacOS/<exe>         the program (0755)
//! Name.app/Contents/Resources/AppIcon.icns
//! Name.app/Contents/Resources/…         what else it carries (an interpreted program's .rrbc, data, notices)
//! Name.app/Contents/_CodeSignature/CodeResources   the seal of Contents/Resources
//! ```
//!
//! made in memory ([`app_files`]) and signed ad hoc ([`sign_ad_hoc`]) so it
//! opens on Apple silicon: by RapidR's own signer ([`crate::macho_sign`]),
//! the same on every system and in the browser.

use crate::files::Files;
use crate::info::{xml, AppInfo};
use crate::{macho_sign, Icon};

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

/// How an app is signed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signature {
    /// Ad hoc: no identity, no certificate; what Apple silicon needs to run
    /// it (a Developer ID signature is made later, on a Mac, with the
    /// user's own Apple account).
    AdHoc,
}

/// Signs the app at `app_root` (`Name.app`) in `files` ad hoc, in memory,
/// with RapidR's own signer ([`macho_sign::sign_app`]): its main executable
/// (every slice, bound to its Info.plist and the seal) and the bundle's
/// `_CodeSignature/CodeResources` (an old seal replaced). Every macOS app
/// any build makes passes here, folder builds and archives alike, on every
/// system and in the browser (docs/build-core-design.md §6,
/// docs/macho-signer-design.md).
///
/// Refused, with `files` as they were: no program at
/// `Contents/MacOS/<info.exe>`, a file outside Contents/Resources besides
/// Info.plist, PkgInfo and the program (nested code), a program that isn't
/// a Mach-O one or is damaged.
pub fn sign_ad_hoc(files: &mut Files, app_root: &str, info: &AppInfo) -> Result<Signature, String> {
    let root = app_root.trim_end_matches('/');
    if !files.contains(&format!("{root}/Contents/MacOS/{}", info.exe)) {
        return Err(format!("{root} can't be signed: it has no program Contents/MacOS/{}", info.exe));
    }
    // (taken out, not copied: a runner is tens of megabytes; put back as they were on an error)
    let paths: Vec<String> = files.below(root).map(|(rest, _)| rest.to_string()).collect();
    let mut executable = std::collections::HashSet::new();
    let mut app: Vec<(String, Vec<u8>)> = Vec::with_capacity(paths.len());
    for rest in paths {
        let f = files.remove(&format!("{root}/{rest}")).expect("listed just now");
        if f.executable {
            executable.insert(rest.clone());
        }
        app.push((rest, f.bytes));
    }
    let signed = macho_sign::sign_app(&mut app);
    for (rest, bytes) in app {
        let path = format!("{root}/{rest}");
        if executable.contains(&rest) {
            files.add_executable(&path, bytes)?;
        } else {
            files.add(&path, bytes)?;
        }
    }
    signed.map_err(|e| format!("{root} can't be signed: {e}"))?;
    Ok(Signature::AdHoc)
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

    /// Signed in memory by RapidR's signer: the program (still executable)
    /// and the seal; what it can't sign is refused with the files as they were.
    #[test]
    fn signed_ad_hoc_in_memory() {
        use crate::macho::{CPU_TYPE_ARM64, CPU_TYPE_X86_64};
        use crate::macho_sign::synth;
        let info = AppInfo::new("Hello", "hello");
        let icon = Icon::rapidr_default();
        let a = synth::program(CPU_TYPE_ARM64, b"arm");
        let x = synth::program(CPU_TYPE_X86_64, b"intel");
        let fat = crate::macho::universal(&[&a, &x]).unwrap();
        let app = MacApp { info: &info, icon: &icon, executable: &fat, resources: vec![("hello.rrbc".into(), b"RRBC".to_vec())], minimum_system: MINIMUM_SYSTEM };
        let mut f = app_files(&app).unwrap();
        f.add("README.txt", b"beside the app: not in it".to_vec()).unwrap();
        assert_eq!(sign_ad_hoc(&mut f, "Hello.app", &info).unwrap(), Signature::AdHoc);
        let exe = f.get("Hello.app/Contents/MacOS/hello").unwrap();
        assert!(exe.executable, "still executable");
        let slices = macho_sign::check(&exe.bytes).unwrap();
        assert_eq!(slices.len(), 2);
        assert!(slices.iter().all(|s| s.identifier == "dev.rapidr.app.hello" && s.flags == macho_sign::CS_ADHOC));
        let seal = f.get("Hello.app/Contents/_CodeSignature/CodeResources").unwrap();
        assert!(!seal.executable && String::from_utf8_lossy(&seal.bytes).contains("<key>Resources/hello.rrbc</key>"));
        assert!(f.contains("README.txt"), "files outside the app untouched");
        // signed again: the same bytes
        let before = f.clone();
        sign_ad_hoc(&mut f, "Hello.app/", &info).unwrap();
        assert_eq!(f.sorted(), before.sorted());

        // nested code, a script for a program, no program: refused, nothing changed
        let mut nested = app_files(&app).unwrap();
        nested.add_executable("Hello.app/Contents/MacOS/helper", b"x".to_vec()).unwrap();
        let was = nested.clone();
        let e = sign_ad_hoc(&mut nested, "Hello.app", &info).unwrap_err();
        assert!(e.starts_with("Hello.app can't be signed: Contents/MacOS/helper:"), "{e}");
        assert_eq!(nested.sorted(), was.sorted());
        let script = MacApp { executable: b"#!/bin/sh\necho hi\n", resources: Vec::new(), ..app };
        let mut s = app_files(&script).unwrap();
        let was = s.clone();
        assert!(sign_ad_hoc(&mut s, "Hello.app", &info).unwrap_err().contains("not a Mach-O program (it starts with 23212f62: a script?"));
        assert_eq!(s.sorted(), was.sorted());
        let other = AppInfo::new("Hello", "other");
        assert!(sign_ad_hoc(&mut s, "Hello.app", &other).unwrap_err().contains("it has no program Contents/MacOS/other"));
    }
}

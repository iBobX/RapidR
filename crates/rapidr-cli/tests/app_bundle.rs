//! `rapidr build` makes RapidQ's notepad (examples/rapidq/notepad.bas) into a
//! macOS app and opens it (docs/manual/building-apps.md). macOS only, and
//! slow the first time (the interpreter's runner is built), so on request:
//!
//!   cargo test -p rapidr-cli --test app_bundle -- --ignored

#![cfg(target_os = "macos")]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

fn rapidr() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_rapidr"))
}

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn plist(app: &Path, key: &str) -> String {
    let out = Command::new("plutil").args(["-extract", key, "raw", "-o", "-"]).arg(app.join("Contents/Info.plist")).output().unwrap();
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// The pixel sizes in an .icns (each entry's PNG or ARGB picture).
fn icns_sizes(path: &Path) -> Vec<u32> {
    // (outside the app: a file added inside would break its seal)
    let dir = std::env::temp_dir().join(format!("rapidr-app-bundle-{}.iconset", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    assert!(Command::new("iconutil").args(["-c", "iconset"]).arg(path).arg("-o").arg(&dir).status().unwrap().success(), "iconutil reads it");
    let mut sizes: Vec<u32> = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            // icon_16x16@2x.png
            let side: u32 = name.trim_start_matches("icon_").split('x').next().unwrap().parse().unwrap();
            if name.contains("@2x") { side * 2 } else { side }
        })
        .collect();
    sizes.sort();
    sizes
}

fn build(dir: &Path, args: &[&str]) {
    let out = Command::new(rapidr()).arg("build").arg("notepad.bas").args(args).current_dir(dir).output().unwrap();
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(out.status.success(), "rapidr build {args:?}:\n{text}");
}

#[test]
#[ignore]
fn notepad_becomes_an_app_that_opens() {
    let dir = std::env::temp_dir().join(format!("rapidr-app-bundle-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::copy(repo("examples/rapidq/notepad.bas"), dir.join("notepad.bas")).unwrap();
    std::fs::copy(repo("tests/fixtures/studio_app/note.svg"), dir.join("note.svg")).unwrap();

    // RapidR's icon, the defaults
    build(&dir, &["--interp"]);
    let app = dir.join("notepad.app");
    for f in ["Contents/Info.plist", "Contents/PkgInfo", "Contents/MacOS/notepad", "Contents/Resources/AppIcon.icns", "Contents/Resources/notepad.rrbc", "Contents/Resources/THIRD-PARTY-NOTICES.txt"] {
        assert!(app.join(f).is_file(), "{f}");
    }
    assert!(!dir.join("notepad").exists(), "no bare executable beside the app");
    assert_eq!(plist(&app, "CFBundleIdentifier"), "dev.rapidr.app.notepad");
    assert_eq!(plist(&app, "CFBundleExecutable"), "notepad");
    assert_eq!(plist(&app, "CFBundleIconFile"), "AppIcon");
    assert_eq!(plist(&app, "NSHighResolutionCapable"), "true");
    assert_eq!(plist(&app, "LSMinimumSystemVersion"), "10.13");
    assert_eq!(icns_sizes(&app.join("Contents/Resources/AppIcon.icns")), [16, 32, 32, 64, 128, 256, 256, 512, 512, 1024]);
    assert!(Command::new("codesign").args(["--verify", "--strict"]).arg(&app).status().unwrap().success(), "codesign -v");

    // its own icon and details
    build(&dir, &["--interp", "--icon", "note.svg", "--name", "Notepad", "--bundle-id", "com.example.notepad", "--app-version", "2.1.0", "--company", "Example Ltd"]);
    let app = dir.join("Notepad.app");
    assert_eq!(plist(&app, "CFBundleName"), "Notepad");
    assert_eq!(plist(&app, "CFBundleDisplayName"), "Notepad");
    assert_eq!(plist(&app, "CFBundleIdentifier"), "com.example.notepad");
    assert_eq!(plist(&app, "CFBundleShortVersionString"), "2.1.0");
    assert_eq!(plist(&app, "NSHumanReadableCopyright"), "© Example Ltd");
    assert!(Command::new("codesign").args(["--verify", "--strict"]).arg(&app).status().unwrap().success());

    // `open` starts it: its window comes up
    assert!(Command::new("open").arg("-n").arg(&app).env("RAPIDR_PRINT_TO", dir.join("prints")).status().unwrap().success());
    let exe = app.join("Contents/MacOS/notepad");
    let started = Instant::now();
    let mut title = String::new();
    while started.elapsed() < Duration::from_secs(60) && !title.contains("Notepad") {
        std::thread::sleep(Duration::from_millis(500));
        let out = Command::new("osascript")
            .args(["-e", "tell application \"System Events\" to get name of every window of (every process whose unix id is (do shell script \"pgrep -f -n '\" & \"Notepad.app/Contents/MacOS/notepad\" & \"'\") as integer)"])
            .output()
            .unwrap();
        title = String::from_utf8_lossy(&out.stdout).trim().to_string();
    }
    let _ = Command::new("pkill").arg("-f").arg(exe.to_string_lossy().as_ref()).status();
    assert!(title.contains("Notepad - untitled"), "the app's window: {title:?}");

    // a program named with a digit (3dcube.bas -> 3dcube.app): codesign took the relative
    // `3dcube.app` for a process id ("No such process")
    std::fs::copy(dir.join("notepad.bas"), dir.join("3dnote.bas")).unwrap();
    let out = Command::new(rapidr()).args(["build", "3dnote.bas", "--interp"]).current_dir(&dir).output().unwrap();
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(out.status.success(), "rapidr build 3dnote.bas:\n{text}");
    assert!(text.contains("Signed (ad hoc)"), "{text}");
    assert!(Command::new("codesign").args(["--verify", "--strict"]).arg(dir.join("3dnote.app")).status().unwrap().success(), "codesign -v 3dnote.app");

    // a console program stays a plain executable
    std::fs::write(dir.join("hello.bas"), "$APPTYPE CONSOLE\nPRINT \"hi\"\n").unwrap();
    let out = Command::new(rapidr()).args(["build", "hello.bas", "--interp"]).current_dir(&dir).output().unwrap();
    assert!(out.status.success());
    assert!(dir.join("hello").is_file() && !dir.join("hello.app").exists());
    let ran = Command::new(dir.join("hello")).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&ran.stdout).trim(), "hi");
    let _ = std::fs::remove_dir_all(&dir);
}

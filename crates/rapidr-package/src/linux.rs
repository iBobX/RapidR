//! Linux: a program as an AppDir (the layout AppImage packs, and that runs
//! where it is) —
//!
//! ```text
//! Name.AppDir/AppRun                          starts usr/bin/<exe>
//! Name.AppDir/<id>.desktop                    the desktop entry (name, icon, terminal or not)
//! Name.AppDir/<id>.png, .DirIcon              the icon, 256 px
//! Name.AppDir/usr/bin/<exe>                   the program
//! Name.AppDir/usr/share/applications/<id>.desktop
//! Name.AppDir/usr/share/icons/hicolor/<n>x<n>/apps/<id>.png   16 to 512 px (and scalable/ for an SVG)
//! ```
//!
//! `<id>` is the bundle ID. [`install`] puts it in the user's applications
//! menu (`~/.local/share`), where the launcher shows it with its icon.

use std::path::{Path, PathBuf};

use crate::info::AppInfo;
use crate::Icon;

/// The hicolor sizes an AppDir carries.
pub const SIZES: [u32; 9] = [16, 22, 24, 32, 48, 64, 128, 256, 512];

/// An AppDir to write.
pub struct AppDir<'a> {
    pub info: &'a AppInfo,
    pub icon: &'a Icon,
    pub executable: &'a [u8],
    /// More files beside the program in usr/bin: (name, contents).
    pub files: Vec<(String, Vec<u8>)>,
}

/// A desktop entry's value: one line, `\` and line breaks escaped.
fn entry(text: &str) -> String {
    text.replace('\\', "\\\\").replace('\n', "\\n").replace('\r', "")
}

/// The `.desktop` text; `exec` is the Exec line's program, `icon` the Icon.
pub fn desktop_entry(info: &AppInfo, exec: &str, icon: &str) -> String {
    let mut out = String::from("[Desktop Entry]\nType=Application\n");
    out.push_str(&format!("Name={}\n", entry(&info.name)));
    if !info.description.trim().is_empty() {
        out.push_str(&format!("Comment={}\n", entry(info.description.trim())));
    }
    out.push_str(&format!("Exec={exec} %F\n"));
    out.push_str(&format!("Icon={icon}\n"));
    out.push_str(&format!("Terminal={}\n", info.console));
    out.push_str("Categories=Utility;\n");
    // (the windows a program opens are its: GNOME and KDE match them by
    // their app ID / WM_CLASS, the executable's name)
    out.push_str(&format!("StartupWMClass={}\n", entry(&info.exe)));
    out.push_str(&format!("X-AppImage-Version={}\n", entry(info.version.trim())));
    out
}

/// A path quoted for a desktop entry's Exec line.
fn exec_quoted(path: &Path) -> String {
    let s = path.to_string_lossy();
    if s.chars().any(|c| c.is_whitespace() || "\"'\\$`".contains(c)) {
        format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"").replace('$', "\\$").replace('`', "\\`"))
    } else {
        s.into_owned()
    }
}

const APPRUN: &str = "#!/bin/sh\n# Starts the program inside this AppDir (made by `rapidr build`).\nHERE=\"$(dirname \"$(readlink -f \"$0\")\")\"\nexec \"$HERE/usr/bin/@EXE@\" \"$@\"\n";

#[cfg(unix)]
fn executable(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(not(unix))]
fn executable(_path: &Path) -> Result<(), String> {
    Ok(())
}

/// Writes `folder/<name>.AppDir` (replacing an AppDir that's there); its path.
pub fn write(folder: &Path, app: &AppDir) -> Result<PathBuf, String> {
    let info = app.info;
    info.check()?;
    let id = &info.bundle_id;
    let path = folder.join(format!("{}.AppDir", info.name));
    if path.exists() {
        if !path.join("AppRun").is_file() {
            return Err(format!("{} is there and isn't an AppDir: move it away first", path.display()));
        }
        std::fs::remove_dir_all(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    let io = |p: &Path, e: std::io::Error| format!("{}: {e}", p.display());
    let write = |p: PathBuf, bytes: &[u8]| -> Result<(), String> {
        if let Some(dir) = p.parent() {
            std::fs::create_dir_all(dir).map_err(|e| io(dir, e))?;
        }
        std::fs::write(&p, bytes).map_err(|e| io(&p, e))
    };
    let bin = path.join("usr/bin").join(&info.exe);
    write(bin.clone(), app.executable)?;
    executable(&bin)?;
    for (name, bytes) in &app.files {
        write(path.join("usr/bin").join(name), bytes)?;
    }
    let apprun = path.join("AppRun");
    write(apprun.clone(), APPRUN.replace("@EXE@", &info.exe).as_bytes())?;
    executable(&apprun)?;
    let desktop = desktop_entry(info, &info.exe, id);
    write(path.join(format!("{id}.desktop")), desktop.as_bytes())?;
    write(path.join(format!("usr/share/applications/{id}.desktop")), desktop.as_bytes())?;
    let png256 = app.icon.png(256);
    write(path.join(format!("{id}.png")), &png256)?;
    write(path.join(".DirIcon"), &png256)?;
    for px in SIZES {
        write(path.join(format!("usr/share/icons/hicolor/{px}x{px}/apps/{id}.png")), &app.icon.png(px))?;
    }
    if let Some(svg) = app.icon.svg_text() {
        write(path.join(format!("usr/share/icons/hicolor/scalable/apps/{id}.svg")), svg.as_bytes())?;
    }
    Ok(path)
}

/// Puts an AppDir in the user's applications menu: its desktop entry in
/// `~/.local/share/applications` (Exec its AppRun, where the AppDir is) and
/// its icons in `~/.local/share/icons/hicolor`. The paths written.
pub fn install(appdir: &Path, data_home: &Path) -> Result<Vec<PathBuf>, String> {
    let appdir = appdir.canonicalize().map_err(|e| format!("{}: {e}", appdir.display()))?;
    let entry = std::fs::read_dir(&appdir)
        .map_err(|e| format!("{}: {e}", appdir.display()))?
        .flatten()
        .map(|e| e.path())
        .find(|p| p.extension().is_some_and(|x| x == "desktop"))
        .ok_or_else(|| format!("{}: no desktop entry: not an AppDir `rapidr build` made", appdir.display()))?;
    let id = entry.file_stem().unwrap_or_default().to_string_lossy().into_owned();
    let text = std::fs::read_to_string(&entry).map_err(|e| format!("{}: {e}", entry.display()))?;
    let exec = exec_quoted(&appdir.join("AppRun"));
    let text: String = text
        .lines()
        .map(|l| if l.starts_with("Exec=") { format!("Exec={exec} %F") } else { l.to_string() })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    let mut written = Vec::new();
    let put = |to: PathBuf, bytes: &[u8], written: &mut Vec<PathBuf>| -> Result<(), String> {
        if let Some(dir) = to.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        std::fs::write(&to, bytes).map_err(|e| format!("{}: {e}", to.display()))?;
        written.push(to);
        Ok(())
    };
    let icons = appdir.join("usr/share/icons/hicolor");
    for size in std::fs::read_dir(&icons).map_err(|e| format!("{}: {e}", icons.display()))?.flatten() {
        for ext in ["png", "svg"] {
            let from = size.path().join("apps").join(format!("{id}.{ext}"));
            if let Ok(bytes) = std::fs::read(&from) {
                put(data_home.join("icons/hicolor").join(size.file_name()).join("apps").join(format!("{id}.{ext}")), &bytes, &mut written)?;
            }
        }
    }
    put(data_home.join("applications").join(format!("{id}.desktop")), text.as_bytes(), &mut written)?;
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_an_appdir_and_installs_it() {
        let dir = std::env::temp_dir().join(format!("rapidr-package-linux-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut info = AppInfo::new("Note Pad", "notepad");
        info.description = "Edits text".into();
        let icon = Icon::rapidr_default();
        let path = write(&dir.join(""), &AppDir { info: &info, icon: &icon, executable: b"\x7fELF", files: vec![] }).unwrap();
        let id = "dev.rapidr.app.note-pad";
        for f in ["AppRun", ".DirIcon", "usr/bin/notepad", &format!("{id}.desktop"), &format!("{id}.png"),
                  &format!("usr/share/icons/hicolor/512x512/apps/{id}.png"), &format!("usr/share/icons/hicolor/scalable/apps/{id}.svg")] {
            assert!(path.join(f).is_file(), "{f}");
        }
        let desktop = std::fs::read_to_string(path.join(format!("{id}.desktop"))).unwrap();
        for want in ["Name=Note Pad\n", "Comment=Edits text\n", "Exec=notepad %F\n", &format!("Icon={id}\n"), "Terminal=false\n", "StartupWMClass=notepad\n"] {
            assert!(desktop.contains(want), "{want}\n{desktop}");
        }
        assert!(std::fs::read_to_string(path.join("AppRun")).unwrap().contains("usr/bin/notepad"));

        let home = dir.join("home");
        let written = install(&path, &home).unwrap();
        assert!(written.len() >= SIZES.len() + 1);
        let installed = std::fs::read_to_string(home.join(format!("applications/{id}.desktop"))).unwrap();
        assert!(installed.contains("AppRun\" %F") && installed.contains("Exec=\""), "{installed}");
        assert!(home.join(format!("icons/hicolor/48x48/apps/{id}.png")).is_file());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

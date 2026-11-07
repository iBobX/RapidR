//! Linux: a program as an AppDir (the layout AppImage packs, and that runs
//! where it is) —
//!
//! ```text
//! Name.AppDir/AppRun                          starts usr/bin/<exe>
//! Name.AppDir/<id>.desktop                    the desktop entry (name, icon, category, terminal or not)
//! Name.AppDir/<id>.png, .DirIcon              the icon, 256 px
//! Name.AppDir/install.sh                      puts it in the applications menu (~/.local)
//! Name.AppDir/usr/bin/<exe>                   the program
//! Name.AppDir/usr/share/applications/<id>.desktop
//! Name.AppDir/usr/share/icons/hicolor/<n>x<n>/apps/<id>.png   16 to 512 px (and scalable/ for an SVG)
//! ```
//!
//! `<id>` is the bundle ID. Made in memory ([`appdir_files`]); `install.sh`
//! ([`install_script`]) or `rapidr install-app` put it in the user's menu.

use crate::files::Files;
use crate::info::{sh_quote, AppInfo};
use crate::Icon;

/// The hicolor sizes an AppDir carries.
pub const SIZES: [u32; 9] = [16, 22, 24, 32, 48, 64, 128, 256, 512];

/// An AppDir to make.
pub struct AppDir<'a> {
    pub info: &'a AppInfo,
    pub icon: &'a Icon,
    pub executable: &'a [u8],
    /// More files beside the program in usr/bin: (path there, `/`-separated; contents).
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
    out.push_str(&format!("Categories={}\n", info.category.linux()));
    // (the windows a program opens are its: GNOME and KDE match them by
    // their app ID / WM_CLASS, which AppRun sets to the bundle ID)
    out.push_str(&format!("StartupWMClass={}\n", entry(&info.bundle_id)));
    out.push_str(&format!("X-AppImage-Version={}\n", entry(info.version.trim())));
    out
}

const APPRUN: &str = "#!/bin/sh\n# Starts the program inside this AppDir (made by `rapidr build`).\nHERE=\"$(dirname \"$(readlink -f \"$0\")\")\"\n# (the windows' app ID: the desktop entry's, so the dock shows the app's icon)\nexport RAPIDR_APP_ID=@ID@\nexec \"$HERE/usr/bin/\"@EXE@ \"$@\"\n";

/// The AppDir's folder name: `Name.AppDir`.
pub fn appdir_folder(info: &AppInfo) -> String {
    format!("{}.AppDir", info.name)
}

/// The AppDir's files, under `Name.AppDir/`.
pub fn appdir_files(app: &AppDir) -> Result<Files, String> {
    let info = app.info;
    info.check()?;
    let id = &info.bundle_id;
    let root = appdir_folder(info);
    let mut f = Files::new();
    f.add_executable(&format!("{root}/usr/bin/{}", info.exe), app.executable.to_vec())?;
    for (name, bytes) in &app.files {
        f.add(&format!("{root}/usr/bin/{name}"), bytes.clone())?;
    }
    f.add_executable(&format!("{root}/AppRun"), APPRUN.replace("@EXE@", &sh_quote(&info.exe)).replace("@ID@", &sh_quote(id)).into_bytes())?;
    f.add_executable(&format!("{root}/install.sh"), install_script(info).into_bytes())?;
    let desktop = desktop_entry(info, &info.exe, id);
    f.add(&format!("{root}/{id}.desktop"), desktop.clone().into_bytes())?;
    f.add(&format!("{root}/usr/share/applications/{id}.desktop"), desktop.into_bytes())?;
    let png256 = app.icon.png(256);
    f.add(&format!("{root}/{id}.png"), png256.clone())?;
    f.add(&format!("{root}/.DirIcon"), png256)?;
    for px in SIZES {
        f.add(&format!("{root}/usr/share/icons/hicolor/{px}x{px}/apps/{id}.png"), app.icon.png(px))?;
    }
    if let Some(svg) = app.icon.svg_text() {
        f.add(&format!("{root}/usr/share/icons/hicolor/scalable/apps/{id}.svg"), svg.into_bytes())?;
    }
    Ok(f)
}

/// `install.sh`: the app for this user alone, no root — its folder copied
/// to `~/.local/lib/<id>`, its desktop entry and icons into
/// `~/.local/share` (it then shows in the applications menu with its icon),
/// a console program also as `~/.local/bin/<exe>` when no such file is
/// there; `install.sh --uninstall` removes what it put. Every value the app
/// gives is quoted for the shell ([`sh_quote`]): its name is only ever text.
pub fn install_script(info: &AppInfo) -> String {
    let script = r##"#!/bin/sh
# Installs @NAME_TEXT@ for you alone, no root needed (made by RapidR):
#   ./install.sh               the app into ~/.local, and into your applications menu
#   ./install.sh --uninstall   removes it again
# PREFIX (default ~/.local) and XDG_DATA_HOME are honoured.
# (The folder also runs as it is, without installing: ./AppRun)
set -eu
NAME=@NAME@
ID=@ID@
EXE=@EXE@
CONSOLE=@CONSOLE@
HERE="$(cd "$(dirname "$0")" && pwd)"
PREFIX="${PREFIX:-$HOME/.local}"
DATA="${XDG_DATA_HOME:-$HOME/.local/share}"
DEST="$PREFIX/lib/$ID"
LIST="$DEST/installed.txt"

uninstall() {
    if [ -f "$LIST" ]; then
        while IFS= read -r p; do
            [ -n "$p" ] && [ "$p" != "$DEST" ] && rm -f "$p"
        done < "$LIST"
    fi
    rm -rf "$DEST"
}

if [ "${1:-}" = "--uninstall" ]; then
    uninstall
    command -v update-desktop-database >/dev/null && update-desktop-database "$DATA/applications" || true
    echo "$NAME removed"
    exit 0
fi

[ -d "$DEST" ] && uninstall
mkdir -p "$PREFIX/lib" "$DATA/applications"
cp -R "$HERE" "$DEST"
chmod 755 "$DEST/AppRun" "$DEST/usr/bin/$EXE"
: > "$LIST"
# (the desktop entry, started from where the app is now — the path quoted as
# desktop entries quote: \ " ` $ escaped —; its icons)
DEST_Q="$(printf '%s' "$DEST" | sed -e 's/\\/\\\\\\\\/g' -e 's/["`$]/\\\\&/g')"
while IFS= read -r line; do
    case "$line" in
        Exec=*) printf 'Exec="%s/AppRun" %%F\n' "$DEST_Q" ;;
        *) printf '%s\n' "$line" ;;
    esac
done < "$HERE/usr/share/applications/$ID.desktop" > "$DATA/applications/$ID.desktop"
echo "$DATA/applications/$ID.desktop" >> "$LIST"
for f in "$HERE"/usr/share/icons/hicolor/*/apps/"$ID".*; do
    [ -f "$f" ] || continue
    size="$(basename "$(dirname "$(dirname "$f")")")"
    mkdir -p "$DATA/icons/hicolor/$size/apps"
    cp "$f" "$DATA/icons/hicolor/$size/apps/"
    echo "$DATA/icons/hicolor/$size/apps/$(basename "$f")" >> "$LIST"
done
if [ "$CONSOLE" = yes ] && [ ! -e "$PREFIX/bin/$EXE" ]; then
    mkdir -p "$PREFIX/bin"
    # (the path in single quotes: nothing in it is expanded when it runs)
    DEST_SH="$(printf '%s' "$DEST" | sed "s/'/'\\\\''/g")"
    printf "#!/bin/sh\nexec '%s/AppRun' \"\$@\"\n" "$DEST_SH" > "$PREFIX/bin/$EXE"
    chmod 755 "$PREFIX/bin/$EXE"
    echo "$PREFIX/bin/$EXE" >> "$LIST"
fi
command -v update-desktop-database >/dev/null && update-desktop-database "$DATA/applications" || true
command -v gtk-update-icon-cache >/dev/null && gtk-update-icon-cache -q -t "$DATA/icons/hicolor" || true
echo "$NAME installed in $DEST: it is in your applications menu"
[ "$CONSOLE" = yes ] && [ -f "$PREFIX/bin/$EXE" ] && echo "run it in a terminal: $EXE"
exit 0
"##;
    // (the comment line: the name as plain text, nothing a shell reads)
    let comment_name: String = info.name.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    script
        .replace("@NAME_TEXT@", &comment_name)
        .replace("@NAME@", &sh_quote(&info.name))
        .replace("@ID@", &sh_quote(&info.bundle_id))
        .replace("@EXE@", &sh_quote(&info.exe))
        .replace("@CONSOLE@", if info.console { "yes" } else { "no" })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_appdir_in_memory() {
        let mut info = AppInfo::new("Note Pad", "notepad");
        info.description = "Edits text".into();
        info.category = crate::Category::Productivity;
        let icon = Icon::rapidr_default();
        let f = appdir_files(&AppDir { info: &info, icon: &icon, executable: b"\x7fELF", files: vec![("art/logo.svg".into(), b"<svg/>".to_vec())] }).unwrap();
        let id = "dev.rapidr.app.note-pad";
        let r = "Note Pad.AppDir";
        for p in ["AppRun", ".DirIcon", "install.sh", "usr/bin/notepad", "usr/bin/art/logo.svg", &format!("{id}.desktop"), &format!("{id}.png"), &format!("usr/share/icons/hicolor/512x512/apps/{id}.png"), &format!("usr/share/icons/hicolor/scalable/apps/{id}.svg")] {
            assert!(f.contains(&format!("{r}/{p}")), "{p}");
        }
        for exe in ["AppRun", "install.sh", "usr/bin/notepad"] {
            assert!(f.get(&format!("{r}/{exe}")).unwrap().executable, "{exe}");
        }
        assert!(!f.get(&format!("{r}/{id}.desktop")).unwrap().executable);
        let desktop = String::from_utf8(f.get(&format!("{r}/{id}.desktop")).unwrap().bytes.clone()).unwrap();
        for want in ["Name=Note Pad\n", "Comment=Edits text\n", "Exec=notepad %F\n", &format!("Icon={id}\n"), "Terminal=false\n", "Categories=Office;\n", "StartupWMClass=dev.rapidr.app.note-pad\n"] {
            assert!(desktop.contains(want), "{want}\n{desktop}");
        }
        let apprun = String::from_utf8(f.get(&format!("{r}/AppRun")).unwrap().bytes.clone()).unwrap();
        assert!(apprun.contains("\"$HERE/usr/bin/\"'notepad'") && apprun.contains("RAPIDR_APP_ID='dev.rapidr.app.note-pad'"), "{apprun}");
    }

    /// Whatever the app is called, install.sh only ever uses it as text.
    #[test]
    fn install_sh_quotes_every_value() {
        let mut info = AppInfo::new("Bob's $(rm -rf ~) `x` \"App\"", "bobs");
        info.console = true;
        let s = install_script(&info);
        assert!(s.contains("NAME='Bob'\\''s $(rm -rf ~) `x` \"App\"'"), "{s}");
        assert!(s.contains("EXE='bobs'") && s.contains("CONSOLE=yes"), "{s}");
        assert!(s.starts_with("#!/bin/sh\n"));
        // (no placeholder left)
        for p in ["@NAME@", "@ID@", "@EXE@", "@CONSOLE@", "@NAME_TEXT@"] {
            assert!(!s.contains(p), "{p}: {s}");
        }
    }

    /// The script really installs and uninstalls (sh, where there is one).
    #[cfg(unix)]
    #[test]
    fn install_sh_installs_and_uninstalls() {
        let base = std::env::temp_dir().join(format!("rapidr-install-sh-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let mut info = AppInfo::new("Say Hi", "say-hi");
        info.console = true;
        let icon = Icon::rapidr_default();
        let files = appdir_files(&AppDir { info: &info, icon: &icon, executable: b"#!/bin/sh\necho hi\n", files: vec![] }).unwrap();
        for f in files.iter() {
            let p = base.join("unpacked").join(&f.path);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, &f.bytes).unwrap();
            if f.executable {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
            }
        }
        // (a home with a space and a $: the Exec line quotes both)
        let home = base.join("my home $USER");
        let run = |arg: &str| {
            std::process::Command::new("sh")
                .arg(base.join("unpacked/Say Hi.AppDir/install.sh"))
                .arg(arg)
                .env("HOME", &home)
                .env_remove("XDG_DATA_HOME")
                .env_remove("PREFIX")
                .output()
                .unwrap()
        };
        let o = run("");
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        let id = "dev.rapidr.app.say-hi";
        let desktop = std::fs::read_to_string(home.join(format!(".local/share/applications/{id}.desktop"))).unwrap();
        let quoted = home.display().to_string().replace('$', "\\\\$");
        assert!(desktop.contains(&format!("Exec=\"{quoted}/.local/lib/{id}/AppRun\" %F\n")), "{desktop}");
        assert!(desktop.contains("Name=Say Hi\n") && desktop.contains("Terminal=true\n"), "{desktop}");
        assert!(home.join(format!(".local/share/icons/hicolor/48x48/apps/{id}.png")).is_file());
        assert!(home.join(".local/bin/say-hi").is_file());
        let out = std::process::Command::new(home.join(".local/bin/say-hi")).output().unwrap();
        assert_eq!(String::from_utf8_lossy(&out.stdout), "hi\n");
        let o = run("--uninstall");
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        assert!(!home.join(format!(".local/lib/{id}")).exists());
        assert!(!home.join(format!(".local/share/applications/{id}.desktop")).exists());
        assert!(!home.join(".local/bin/say-hi").exists());
        let _ = std::fs::remove_dir_all(&base);
    }
}

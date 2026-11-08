//! `rapidr build`'s last step: the program made into an app for its system
//! (rapidr-package) — `Name.app` on macOS, the `.exe` with its icon and
//! version on Windows, `Name.AppDir` on Linux — with its icon and details
//! from, highest first: the command line (`--icon`, `--name`, …), the
//! project (`.rrproj`'s `[build]`), the source (`$OPTION ICON`), RC.EXE's
//! `-g<icon>`; else RapidR's own icon.

use std::fs;
use std::path::{Path, PathBuf};

use rapidr_package::{AppInfo, Icon, System, Target};

/// What the command line asked for.
#[derive(Debug, Default, Clone)]
pub struct Options {
    /// `--icon <file>`
    pub icon: Option<String>,
    /// `-g<file>` (RapidQ's RC.EXE option: below `$OPTION ICON`)
    pub rc_icon: Option<String>,
    /// `--name <app name>`
    pub name: Option<String>,
    /// `--bundle-id <id>`
    pub bundle_id: Option<String>,
    /// `--app-version <version>`
    pub version: Option<String>,
    /// `--company <name>`
    pub company: Option<String>,
    /// `--copyright <line>`
    pub copyright: Option<String>,
    /// `--description <one line>`
    pub description: Option<String>,
    /// `--website <https://…>`
    pub website: Option<String>,
    /// `--category <games|education|…>`
    pub category: Option<String>,
    /// `--kind <window|console>` (else the program's own)
    pub kind: Option<String>,
    /// `--theme-color <#RRGGBB>` (the web app's)
    pub theme_color: Option<String>,
    /// `--language <en|es>`: the README's in an archive
    pub language: Option<String>,
    /// `--include-source`: the program's source in the archives (`source/`)
    pub include_source: bool,
    /// `--bundle` (a console program as an app too) / `--no-bundle` (a
    /// plain executable, no icon)
    pub bundle: Option<bool>,
    /// `--project <file.rrproj>` (else the one beside the source naming it
    /// as its main file)
    pub project: Option<String>,
    /// `--output <folder>` / `-o <folder>`: where the app goes
    pub output: Option<String>,
    /// `--keep-rust` / `--no-keep-rust`: a native build's generated Rust
    /// left beside the app (`<program>-rust-source`), or deleted
    pub keep_rust: Option<bool>,
}

impl Options {
    /// Takes the option at `args[*i]` (and its value) if it's one of these;
    /// whether it was. An error for a missing value.
    pub fn take(&mut self, args: &[String], i: &mut usize) -> Result<bool, String> {
        let arg = args[*i].as_str();
        let mut value = |name: &str| -> Result<String, String> {
            if let Some(v) = arg.strip_prefix(&format!("{name}=")) {
                return Ok(v.to_string());
            }
            *i += 1;
            args.get(*i).cloned().ok_or_else(|| format!("{name} needs a value"))
        };
        let flag = arg.split('=').next().unwrap_or(arg);
        match flag {
            "--icon" => self.icon = Some(value("--icon")?),
            "--name" => self.name = Some(value("--name")?),
            "--bundle-id" => self.bundle_id = Some(value("--bundle-id")?),
            "--app-version" => self.version = Some(value("--app-version")?),
            "--company" => self.company = Some(value("--company")?),
            "--copyright" => self.copyright = Some(value("--copyright")?),
            "--description" => self.description = Some(value("--description")?),
            "--website" => self.website = Some(value("--website")?),
            "--category" => self.category = Some(value("--category")?),
            "--kind" => self.kind = Some(value("--kind")?),
            "--theme-color" | "--theme-colour" => self.theme_color = Some(value(flag)?),
            "--language" => self.language = Some(value("--language")?),
            "--include-source" => self.include_source = true,
            "--project" => self.project = Some(value("--project")?),
            "--output" | "-o" => self.output = Some(value(flag)?),
            "--keep-rust" => self.keep_rust = Some(true),
            "--no-keep-rust" => self.keep_rust = Some(false),
            "--bundle" => self.bundle = Some(true),
            "--no-bundle" => self.bundle = Some(false),
            _ if (arg.starts_with("-g") || arg.starts_with("-G")) && !arg.starts_with("--") => {
                // RC.EXE: `-g<file>` or `-g <file>`; nothing after it is its
                // "No icon specified"
                let rest = &arg[2..];
                let file = if rest.is_empty() {
                    *i += 1;
                    args.get(*i).cloned().ok_or("ERROR: No icon specified")?
                } else {
                    rest.to_string()
                };
                self.rc_icon = Some(file);
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}

/// The project a source belongs to: `--project`, a `.rrproj` itself, or the
/// one `.rrproj` beside it whose main file it is — or in a folder above it
/// (up to two), when its `main` is exactly this file (`src/main.rr` of a
/// project kept one folder up). (its folder, the project)
pub fn project_for(source: &Path, explicit: Option<&str>) -> Result<Option<(PathBuf, rapidr_project::Project)>, String> {
    // (SEC-33: a `main`, `icon` or `output` naming a place outside the
    // project's folder is set aside unless this computer's user chose it
    // in RapidR Studio — the same record Studio's Open reads; the build
    // says so, once)
    let load = |p: &Path| -> Result<(PathBuf, rapidr_project::Project, String), String> {
        let mut project = rapidr_project::Project::load(p).map_err(|e| format!("{}: {e}", p.display()))?;
        let found = rapidr_studio::project::set_aside_outside(&p.to_string_lossy().replace('\\', "/"), &mut project);
        Ok((p.parent().unwrap_or(Path::new(".")).to_path_buf(), project, rapidr_studio::project::outside_words(&found)))
    };
    let warned = |(dir, project, words): (PathBuf, rapidr_project::Project, String)| {
        static SAID: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
        if !words.is_empty() {
            if let Ok(mut said) = SAID.lock() {
                if !said.contains(&words) {
                    eprintln!("Warning: {words}");
                    said.push(words);
                }
            }
        }
        (dir, project)
    };
    if let Some(p) = explicit {
        return load(Path::new(p)).map(|l| Some(warned(l)));
    }
    let folder = source.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let Some(name) = source.file_name() else { return Ok(None) };
    // (the source's folder, then the two above it: the nearest one with a
    // project of this file decides; two projects of it there are none)
    let absolute = fs::canonicalize(folder).unwrap_or_else(|_| folder.to_path_buf());
    let searched = std::iter::once(folder.to_path_buf()).chain(absolute.ancestors().skip(1).take(2).map(Path::to_path_buf));
    for (level, searching) in searched.enumerate() {
        let mut found = None;
        for entry in fs::read_dir(&searching).into_iter().flatten().flatten() {
            let p = entry.path();
            if !p.extension().is_some_and(|e| e.eq_ignore_ascii_case("rrproj")) {
                continue;
            }
            // (a web IDE v1 project, or something else: not this one's)
            let Ok((dir, project, words)) = load(&p) else { continue };
            let mine = if level == 0 {
                Path::new(&project.main).file_name().is_some_and(|m| m.eq_ignore_ascii_case(name)) && dir.join(&project.main).is_file()
            } else {
                // (above the file: only a project whose main is exactly this file)
                rapidr_project::program_files::same_path(&dir.join(&project.main), source)
            };
            if mine {
                if found.is_some() {
                    return Ok(None);
                }
                found = Some((dir, project, words));
            }
        }
        if found.is_some() {
            return Ok(found.map(warned));
        }
    }
    Ok(None)
}

/// What the build will make.
pub struct Plan {
    /// The system and processor it is for.
    pub target: Target,
    pub system: System,
    pub info: AppInfo,
    pub icon: Icon,
    /// An app (macOS' .app, Linux' AppDir; Windows' resources) rather than a
    /// plain executable.
    pub bundle: bool,
    /// `--bundle` / `--no-bundle` as asked (`None`: the system's way).
    pub bundle_asked: Option<bool>,
    /// The data files the program names (`rapidr_project::program_files`:
    /// "staff.csv" beside it, "art/logo.svg" below it) and its translations
    /// (`lang/es.po`), put with what the build makes so the app finds them
    /// wherever it's started: in a macOS app's Contents/Resources, an
    /// AppDir's usr/bin, beside an executable — each as (its path there,
    /// `/`-separated; the file).
    pub data: Vec<(String, PathBuf)>,
    /// The README's language in an archive.
    pub language: String,
    /// The program's source in the archives (`--include-source`).
    pub include_source: bool,
}

/// An icon file read (`.icns`, `.ico`, `.png` or `.svg`).
pub fn load_icon(path: &Path) -> Result<Icon, String> {
    let shown = path.display();
    let bytes = fs::read(path).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => format!("{shown}: the icon file isn't there"),
        _ => format!("{shown}: can't read the icon ({e})"),
    })?;
    Icon::from_bytes(&bytes, &shown.to_string())
}

/// Decides the app's name, details and icon. `stem` is the executable's
/// name; `option_icon` the last `$OPTION ICON`'s file (found already: the
/// compiler checked it); `console` what the program is (a project's or the
/// command line's `kind` decides over it).
pub fn plan(stem: &str, target: Target, console: bool, opts: &Options, project: Option<&(PathBuf, rapidr_project::Project)>, option_icon: Option<&Path>) -> Result<Plan, String> {
    let system = target.system().ok_or_else(|| format!("{}: a desktop system's app (the web's is a web build)", target.name()))?;
    let build = project.map(|(_, p)| &p.build);
    let pick = |cli: &Option<String>, proj: Option<&String>| cli.clone().filter(|s| !s.trim().is_empty()).or_else(|| proj.filter(|s| !s.trim().is_empty()).cloned());
    let name = pick(&opts.name, build.map(|b| &b.app_name))
        .or_else(|| project.map(|(_, p)| p.name.clone()).filter(|n| !n.trim().is_empty()))
        .unwrap_or_else(|| stem.to_string());
    // The executable is named after the project (its `.rrproj`'s name, made
    // a file name: spaces and the like become `-`) when a project is built,
    // after the main file only for a single file — so an app's
    // Contents/MacOS/<exe> (Info.plist's CFBundleExecutable), a Windows
    // `.exe` and an AppDir's usr/bin/<exe> are the project's, never `main`.
    let exe = project.and_then(|(_, p)| rapidr_package::exe_name_for(&p.name)).unwrap_or_else(|| stem.to_string());
    let mut info = AppInfo::new(name.trim(), &exe);
    if let Some(id) = pick(&opts.bundle_id, build.map(|b| &b.bundle_id)) {
        info.bundle_id = id.trim().to_string();
    }
    if let Some(v) = pick(&opts.version, build.map(|b| &b.version)) {
        info.version = v.trim().to_string();
    }
    if let Some(c) = pick(&opts.company, build.map(|b| &b.company)) {
        info.company = c.trim().to_string();
    }
    if let Some(c) = pick(&opts.copyright, build.map(|b| &b.copyright)) {
        info.copyright = c.trim().to_string();
    }
    if let Some(d) = pick(&opts.description, build.map(|b| &b.description)) {
        info.description = d.trim().to_string();
    }
    if let Some(w) = pick(&opts.website, build.map(|b| &b.website)) {
        info.website = w.trim().to_string();
    }
    if let Some(c) = pick(&opts.category, build.map(|b| &b.category)) {
        info.category = rapidr_package::Category::parse(&c)?;
    }
    if let Some(c) = pick(&opts.theme_color, build.map(|b| &b.theme_color)) {
        info.theme_color = c.trim().to_string();
    }
    info.console = match pick(&opts.kind, build.map(|b| &b.kind)).map(|k| k.trim().to_ascii_lowercase()) {
        None => console,
        Some(k) if k == "console" => true,
        Some(k) if k == "window" || k == "windowed" || k == "gui" => false,
        Some(k) => return Err(format!("kind \"{k}\": window (an app with windows) or console")),
    };
    info.check()?;
    let language = pick(&opts.language, build.map(|b| &b.language)).unwrap_or_else(|| "en".into());

    // RC.EXE's -g is checked even when $OPTION ICON wins over it
    let rc_icon = match &opts.rc_icon {
        Some(f) => {
            let p = PathBuf::from(f);
            if !p.is_file() {
                return Err(format!("ERROR: Can't find icon {f}"));
            }
            Some(p)
        }
        None => None,
    };
    let icon_path = if let Some(f) = opts.icon.as_ref().filter(|f| !f.trim().is_empty()) {
        Some(PathBuf::from(f))
    } else if let Some((dir, p)) = project.filter(|(_, p)| !p.build.icon.trim().is_empty()) {
        Some(dir.join(&p.build.icon))
    } else {
        option_icon.map(Path::to_path_buf).or(rc_icon)
    };
    let icon = match icon_path {
        Some(p) => load_icon(&p)?,
        None => Icon::rapidr_default(),
    };
    let bundle = opts.bundle.unwrap_or(match system {
        System::Windows => true,
        System::MacOs | System::Linux => !info.console,
    });
    Ok(Plan { target, system, info, icon, bundle, bundle_asked: opts.bundle, data: Vec::new(), language, include_source: opts.include_source })
}

/// The plan's data files read, as (path in the app, bytes): a link is
/// refused (it could point anywhere: the build never follows one).
pub fn read_data(plan: &Plan) -> Result<Vec<(String, Vec<u8>)>, String> {
    plan.data
        .iter()
        .map(|(rel, f)| {
            let meta = fs::symlink_metadata(f).map_err(|e| format!("{}: {e}", f.display()))?;
            if meta.file_type().is_symlink() {
                return Err(format!("{}: a link, not a file: the build never packs links (copy the file there instead)", f.display()));
            }
            Ok((rel.clone(), fs::read(f).map_err(|e| format!("{}: {e}", f.display()))?))
        })
        .collect()
}

/// The packer's input for `plan`.
pub fn app<'a>(plan: &'a Plan, bytecode: Option<&'a [u8]>, data: &'a [(String, Vec<u8>)], source: &'a [(String, Vec<u8>)], mtime: i64) -> rapidr_package::App<'a> {
    rapidr_package::App {
        info: &plan.info,
        icon: &plan.icon,
        bytecode,
        data,
        bundle: Some(plan.bundle),
        web_needs: None,
        source,
        mtime,
        language: &plan.language,
    }
}

/// The time an archive's entries get: `SOURCE_DATE_EPOCH` when set (a
/// reproducible build), else now.
pub fn build_time() -> i64 {
    std::env::var("SOURCE_DATE_EPOCH")
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or_else(|| std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64))
}

/// Writes one file (its folders first), executable when asked.
fn write_file(path: &Path, bytes: &[u8], executable: bool) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    fs::write(path, bytes).map_err(|e| format!("write {}: {e}", path.display()))?;
    if executable {
        make_executable(path);
    }
    Ok(())
}

/// Writes an app's files into `dest_dir`: an app folder already there
/// (`Name.app`, `Name.AppDir`) is replaced — when it is one, never
/// something else of that name —; a loose file that already holds the same
/// bytes is left as it is (a data file built beside its own program, RC.EXE's
/// way, is itself).
pub fn write_tree(dest_dir: &Path, packed: &rapidr_package::Packed) -> Result<(), String> {
    fs::create_dir_all(dest_dir).map_err(|e| format!("{}: {e}", dest_dir.display()))?;
    let main = dest_dir.join(&packed.main);
    let marker = if packed.main.ends_with(".app") {
        Some("Contents/Info.plist")
    } else if packed.main.ends_with(".AppDir") {
        Some("AppRun")
    } else {
        None
    };
    if let Some(marker) = marker.filter(|_| main.exists()) {
        if !main.join(marker).is_file() {
            return Err(format!("{} is there and isn't an app: move it away first", main.display()));
        }
        fs::remove_dir_all(&main).map_err(|e| format!("{}: {e}", main.display()))?;
    }
    for f in packed.files.iter() {
        let to = dest_dir.join(&f.path);
        if fs::read(&to).is_ok_and(|old| old == f.bytes) {
            continue;
        }
        write_file(&to, &f.bytes, f.executable)?;
    }
    Ok(())
}

/// What was made.
pub struct Made {
    /// The file that runs (inside the app).
    pub executable: PathBuf,
}

#[cfg(unix)]
fn make_executable(p: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = fs::set_permissions(p, fs::Permissions::from_mode(0o755));
}

#[cfg(not(unix))]
fn make_executable(_p: &Path) {}

/// Makes the program into its app in `dest_dir` (rapidr_package::pack, the
/// one packer): `binary` is the executable (a native build's, or a runner),
/// `rrbc` an interpreted program's bytecode, `notices` the
/// THIRD-PARTY-NOTICES.txt it carries. Prints what it did.
pub fn finish(plan: &Plan, binary: &[u8], rrbc: Option<&[u8]>, dest_dir: &Path, notices: &str) -> Result<Made, String> {
    let data = read_data(plan)?;
    let app = app(plan, rrbc, &data, &[], build_time());
    let packed = rapidr_package::pack(plan.target, &app, &rapidr_package::Runner { bytes: binary, notices })?;
    for note in &packed.notes {
        eprintln!("note: {note}");
    }
    // (a Mac app: signed ad hoc in pack, by RapidR's own signer, on every system)
    if packed.signature.is_some() {
        println!("Signed (ad hoc): {}", dest_dir.join(&packed.main).display());
    }
    write_tree(dest_dir, &packed)?;
    let path = dest_dir.join(&packed.main);
    let executable = packed.executable.as_ref().map(|e| dest_dir.join(e)).unwrap_or_else(|| path.clone());
    let what = match (plan.system, packed.bundle) {
        (System::MacOs, true) => "App",
        (System::Linux, true) => "AppDir",
        _ => "Executable",
    };
    println!("{what}: {}", path.display());
    if plan.bundle {
        println!("  {} {} ({}), icon: {}", plan.info.name, plan.info.version, plan.info.bundle_id, plan.icon.origin);
    }
    if !data.is_empty() {
        let names: Vec<&str> = plan.data.iter().map(|(rel, _)| rel.as_str()).collect();
        let place = match (plan.system, plan.bundle) {
            (System::MacOs, true) => "in the app's Contents/Resources",
            (System::Linux, true) => "in the AppDir's usr/bin",
            (System::Windows, _) => "beside the .exe",
            _ => "beside the executable",
        };
        println!("  data files {place}: {} (started from Finder, a launcher or Explorer, it finds them)", names.join(", "));
    }
    Ok(Made { executable })
}

/// `rapidr install-app <Name.AppDir>` (Linux): the app in the user's
/// applications menu — its desktop entry and icons under
/// `$XDG_DATA_HOME` (`~/.local/share`), started from where the AppDir is.
pub fn install_app(appdir: &Path) -> std::process::ExitCode {
    let data_home = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")));
    let Some(data_home) = data_home else {
        eprintln!("install-app: no home folder (HOME)");
        return std::process::ExitCode::from(1);
    };
    match install_appdir(appdir, &data_home) {
        Ok(written) => {
            for p in &written {
                println!("{}", p.display());
            }
            // (menus that cache icons hear of the new one; optional tools)
            let _ = std::process::Command::new("gtk-update-icon-cache").arg("-q").arg("-t").arg(data_home.join("icons/hicolor")).status();
            let _ = std::process::Command::new("update-desktop-database").arg("-q").arg(data_home.join("applications")).status();
            println!("Installed: {} is in your applications menu (move the AppDir and install it again)", appdir.display());
            std::process::ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("install-app: {e}");
            std::process::ExitCode::from(1)
        }
    }
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

/// Puts an AppDir in the user's applications menu: its desktop entry in
/// `<data_home>/applications` (Exec its AppRun, where the AppDir is) and
/// its icons in `<data_home>/icons/hicolor`. The paths written.
pub fn install_appdir(appdir: &Path, data_home: &Path) -> Result<Vec<PathBuf>, String> {
    let appdir = appdir.canonicalize().map_err(|e| format!("{}: {e}", appdir.display()))?;
    let entry = fs::read_dir(&appdir)
        .map_err(|e| format!("{}: {e}", appdir.display()))?
        .flatten()
        .map(|e| e.path())
        .find(|p| p.extension().is_some_and(|x| x == "desktop"))
        .ok_or_else(|| format!("{}: no desktop entry: not an AppDir `rapidr build` made", appdir.display()))?;
    let id = entry.file_stem().unwrap_or_default().to_string_lossy().into_owned();
    let text = fs::read_to_string(&entry).map_err(|e| format!("{}: {e}", entry.display()))?;
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
            fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        fs::write(&to, bytes).map_err(|e| format!("{}: {e}", to.display()))?;
        written.push(to);
        Ok(())
    };
    let icons = appdir.join("usr/share/icons/hicolor");
    for size in fs::read_dir(&icons).map_err(|e| format!("{}: {e}", icons.display()))?.flatten() {
        for ext in ["png", "svg"] {
            let from = size.path().join("apps").join(format!("{id}.{ext}"));
            if let Ok(bytes) = fs::read(&from) {
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

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    /// The executable is named after the project when one is built and after
    /// the main file only for a single file — on every system: an app's
    /// Contents/MacOS/<exe> and its Info.plist's CFBundleExecutable, a
    /// Windows .exe, an AppDir's usr/bin/<exe> (and its desktop entry's Exec).
    #[test]
    fn the_executable_is_named_after_the_project() {
        let project = (PathBuf::from("."), rapidr_project::Project::new("Database Studio", "src/main.rr"));
        let o = Options::default();
        for system in [Target::MacosUniversal, Target::WindowsX64, Target::LinuxX64] {
            // (a project: its name, the spaces a dash; `main` is the main file's)
            let p = plan("main", system, false, &o, Some(&project), None).unwrap();
            assert_eq!((p.info.name.as_str(), p.info.exe.as_str()), ("Database Studio", "Database-Studio"), "{system:?}");
            // (a single file: the file's own name)
            let p = plan("main", system, false, &o, None, None).unwrap();
            assert_eq!((p.info.name.as_str(), p.info.exe.as_str()), ("main", "main"), "{system:?}");
        }
        let p = plan("main", Target::MacosUniversal, false, &o, Some(&project), None).unwrap();
        let plist = rapidr_package::macos::info_plist(&p.info, "10.13").unwrap();
        assert!(plist.contains("<key>CFBundleExecutable</key>\n\t<string>Database-Studio</string>"), "{plist}");
        assert!(plist.contains("<key>CFBundleName</key>\n\t<string>Database Studio</string>"), "{plist}");
        let p = plan("main", Target::LinuxX64, false, &o, Some(&project), None).unwrap();
        assert!(rapidr_package::linux::desktop_entry(&p.info, &p.info.exe, &p.info.bundle_id).contains("Exec=Database-Studio %F"));
        // (a project whose name leaves nothing usable: the main file's)
        let blank = (PathBuf::from("."), rapidr_project::Project::new("***", "src/main.rr"));
        assert_eq!(plan("main", Target::LinuxX64, false, &o, Some(&blank), None).unwrap().info.exe, "main");
        // (--name changes the name people see, not the executable)
        let named = Options { name: Some("Shown As".into()), ..Options::default() };
        let p = plan("main", Target::MacosUniversal, false, &named, Some(&project), None).unwrap();
        assert_eq!((p.info.name.as_str(), p.info.exe.as_str()), ("Shown As", "Database-Studio"));
    }

    /// A project kept one folder above its main file (`src/main.rr`) is the
    /// file's project; one whose main is another file is not.
    #[test]
    fn a_project_above_its_main_file_is_found() {
        let dir = std::env::temp_dir().join(format!("rapidr-cli-project-for-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(dir.join("src/main.rr"), "PRINT 1\n").unwrap();
        fs::write(dir.join("src/other.rr"), "PRINT 2\n").unwrap();
        rapidr_project::Project::new("Database Studio", "src/main.rr").save(&dir.join("Studio.rrproj")).unwrap();
        let found = project_for(&dir.join("src/main.rr"), None).unwrap().expect("src/main.rr's project");
        assert_eq!(found.1.name, "Database Studio");
        // (another file of the folder: not its main)
        assert!(project_for(&dir.join("src/other.rr"), None).unwrap().is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn options_from_the_command_line() {
        let a = args(&["--icon", "a.png", "--name=Note Pad", "-gc:\\x.ico", "--no-bundle", "--app-version", "2.0", "other", "-o", "dist", "--keep-rust"]);
        let mut o = Options::default();
        let mut i = 0;
        let mut rest = Vec::new();
        while i < a.len() {
            if !o.take(&a, &mut i).unwrap() {
                rest.push(a[i].clone());
            }
            i += 1;
        }
        assert_eq!(o.icon.as_deref(), Some("a.png"));
        assert_eq!(o.name.as_deref(), Some("Note Pad"));
        assert_eq!(o.rc_icon.as_deref(), Some("c:\\x.ico"));
        assert_eq!(o.version.as_deref(), Some("2.0"));
        assert_eq!(o.bundle, Some(false));
        assert_eq!((o.output.as_deref(), o.keep_rust), (Some("dist"), Some(true)));
        assert_eq!(rest, ["other"]);
        // RC.EXE: `-g` with nothing after it
        let mut i = 0;
        assert_eq!(Options::default().take(&args(&["-g"]), &mut i).unwrap_err(), "ERROR: No icon specified");
        let mut i = 0;
        assert!(Options::default().take(&args(&["--icon"]), &mut i).is_err());
    }

    #[test]
    fn priorities() {
        let dir = std::env::temp_dir().join(format!("rapidr-cli-package-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let png = |name: &str| {
            let p = dir.join(name);
            fs::write(&p, rapidr_package::picture::encode_png(&rapidr_package::picture::blank(64, 64))).unwrap();
            p
        };
        let (cli, proj, src, rc) = (png("cli.png"), png("proj.png"), png("src.png"), png("rc.png"));
        let mut project = rapidr_project::Project::new("Demo", "demo.rr");
        project.build.icon = "proj.png".into();
        project.build.version = "3.1".into();
        let project = (dir.clone(), project);
        let mut o = Options { rc_icon: Some(rc.to_string_lossy().into()), ..Options::default() };

        let p = plan("demo", Target::MacosUniversal, false, &o, Some(&project), Some(&src)).unwrap();
        assert!(p.icon.origin.ends_with("proj.png") && p.bundle && p.info.name == "Demo" && p.info.version == "3.1");
        o.icon = Some(cli.to_string_lossy().into());
        o.name = Some("Cli".into());
        let p = plan("demo", Target::MacosUniversal, false, &o, Some(&project), Some(&src)).unwrap();
        assert!(p.icon.origin.ends_with("cli.png") && p.info.name == "Cli" && p.info.bundle_id == "dev.rapidr.app.cli");
        o.icon = None;
        o.name = None;
        // without a project: $OPTION ICON over -g, then -g, then RapidR's
        let p = plan("demo", Target::LinuxX64, false, &o, None, Some(&src)).unwrap();
        assert!(p.icon.origin.ends_with("src.png") && p.info.name == "demo");
        assert!(plan("demo", Target::LinuxX64, false, &o, None, None).unwrap().icon.origin.ends_with("rc.png"));
        o.rc_icon = None;
        let p = plan("demo", Target::LinuxX64, true, &o, None, None).unwrap();
        assert!(p.icon.is_default() && !p.bundle, "a console program stays an executable");
        assert!(plan("demo", Target::WindowsX64, true, &o, None, None).unwrap().bundle, "a Windows .exe always gets its icon");
        // RC.EXE's -g message
        o.rc_icon = Some("nowhere.ico".into());
        assert_eq!(plan("demo", Target::MacosUniversal, false, &o, None, None).err().unwrap(), "ERROR: Can't find icon nowhere.ico");
        o.rc_icon = None;
        o.version = Some("x".into());
        assert!(plan("demo", Target::MacosUniversal, false, &o, None, None).err().unwrap().contains("version"));

        // the project beside a source names it as its main file
        project.1.save(&dir.join("demo.rrproj")).unwrap();
        fs::write(dir.join("demo.rr"), "PRINT 1\n").unwrap();
        fs::write(dir.join("other.rr"), "PRINT 2\n").unwrap();
        assert_eq!(project_for(&dir.join("demo.rr"), None).unwrap().unwrap().1.name, "Demo");
        assert!(project_for(&dir.join("other.rr"), None).unwrap().is_none());
        fs::remove_dir_all(&dir).unwrap();
    }
}

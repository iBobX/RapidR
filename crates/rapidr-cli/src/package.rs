//! `rapidr build`'s last step: the program made into an app for its system
//! (rapidr-package) — `Name.app` on macOS, the `.exe` with its icon and
//! version on Windows, `Name.AppDir` on Linux — with its icon and details
//! from, highest first: the command line (`--icon`, `--name`, …), the
//! project (`.rrproj`'s `[build]`), the source (`$OPTION ICON`), RC.EXE's
//! `-g<icon>`; else RapidR's own icon.

use std::fs;
use std::path::{Path, PathBuf};

use rapidr_package::{AppInfo, Icon, System};

use crate::notices;

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
    /// `--bundle` (a console program as an app too) / `--no-bundle` (a
    /// plain executable, no icon)
    pub bundle: Option<bool>,
    /// `--project <file.rrproj>` (else the one beside the source naming it
    /// as its main file)
    pub project: Option<String>,
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
            "--project" => self.project = Some(value("--project")?),
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
/// one `.rrproj` beside it whose main file it is. (its folder, the project)
pub fn project_for(source: &Path, explicit: Option<&str>) -> Result<Option<(PathBuf, rapidr_project::Project)>, String> {
    let load = |p: &Path| -> Result<(PathBuf, rapidr_project::Project), String> {
        let project = rapidr_project::Project::load(p).map_err(|e| format!("{}: {e}", p.display()))?;
        Ok((p.parent().unwrap_or(Path::new(".")).to_path_buf(), project))
    };
    if let Some(p) = explicit {
        return load(Path::new(p)).map(Some);
    }
    let folder = source.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let Some(name) = source.file_name() else { return Ok(None) };
    let mut found = None;
    for entry in fs::read_dir(folder).into_iter().flatten().flatten() {
        let p = entry.path();
        if !p.extension().is_some_and(|e| e.eq_ignore_ascii_case("rrproj")) {
            continue;
        }
        // (a web IDE v1 project, or something else: not this one's)
        let Ok((dir, project)) = load(&p) else { continue };
        if Path::new(&project.main).file_name().is_some_and(|m| m.eq_ignore_ascii_case(name)) && dir.join(&project.main).is_file() {
            if found.is_some() {
                return Ok(None);
            }
            found = Some((dir, project));
        }
    }
    Ok(found)
}

/// What the build will make.
pub struct Plan {
    pub system: System,
    pub info: AppInfo,
    pub icon: Icon,
    /// An app (macOS' .app, Linux' AppDir; Windows' resources) rather than a
    /// plain executable.
    pub bundle: bool,
}

/// Decides the app's name, details and icon. `stem` is the executable's
/// name; `option_icon` the last `$OPTION ICON`'s file (found already: the
/// compiler checked it).
pub fn plan(stem: &str, system: System, console: bool, opts: &Options, project: Option<&(PathBuf, rapidr_project::Project)>, option_icon: Option<&Path>) -> Result<Plan, String> {
    let build = project.map(|(_, p)| &p.build);
    let pick = |cli: &Option<String>, proj: Option<&String>| cli.clone().filter(|s| !s.trim().is_empty()).or_else(|| proj.filter(|s| !s.trim().is_empty()).cloned());
    let name = pick(&opts.name, build.map(|b| &b.app_name))
        .or_else(|| project.map(|(_, p)| p.name.clone()).filter(|n| !n.trim().is_empty()))
        .unwrap_or_else(|| stem.to_string());
    let mut info = AppInfo::new(name.trim(), stem);
    if let Some(id) = pick(&opts.bundle_id, build.map(|b| &b.bundle_id)) {
        info.bundle_id = id.trim().to_string();
    }
    if let Some(v) = pick(&opts.version, build.map(|b| &b.version)) {
        info.version = v.trim().to_string();
    }
    if let Some(c) = pick(&opts.company, build.map(|b| &b.company)) {
        info.company = c.trim().to_string();
    }
    info.console = console;
    info.check()?;

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
        Some(p) => Icon::load(&p)?,
        None => Icon::rapidr_default(),
    };
    let bundle = opts.bundle.unwrap_or(match system {
        System::Windows => true,
        System::MacOs | System::Linux => !console,
    });
    Ok(Plan { system, info, icon, bundle })
}

/// Appends a program's bytecode to a runner: `[rrbc][RRBCEXE1][u32 length]`
/// (the runner reads it from its own end).
pub fn with_payload(stub: &[u8], rrbc: &[u8]) -> Result<Vec<u8>, String> {
    let len = u32::try_from(rrbc.len()).map_err(|_| "bytecode payload exceeds 4 GiB".to_string())?;
    let mut out = Vec::with_capacity(stub.len() + rrbc.len() + 12);
    out.extend_from_slice(stub);
    out.extend_from_slice(rrbc);
    out.extend_from_slice(b"RRBCEXE1");
    out.extend_from_slice(&len.to_le_bytes());
    Ok(out)
}

/// What was made.
pub struct Made {
    /// The app or executable.
    pub path: PathBuf,
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

/// Makes the program into its app in `dest_dir`: `binary` is the executable
/// (a native build's, or a runner), `rrbc` an interpreted program's
/// bytecode; `notices` the notices it carries. Prints what it did.
pub fn finish(plan: &Plan, binary: &[u8], rrbc: Option<&[u8]>, dest_dir: &Path, notices: &notices::Kind) -> Result<Made, String> {
    fs::create_dir_all(dest_dir).map_err(|e| format!("{}: {e}", dest_dir.display()))?;
    let exe_suffix = if plan.system == System::Windows { ".exe" } else { "" };
    let whole = |b: &[u8]| -> Result<Vec<u8>, String> {
        match rrbc {
            Some(code) => with_payload(b, code),
            None => Ok(b.to_vec()),
        }
    };
    for note in plan.icon.notes() {
        eprintln!("note: {note}");
    }
    let made = match (plan.system, plan.bundle) {
        (System::MacOs, true) => {
            // (the bytecode in Resources: data after the Mach-O would break its signature)
            let mut resources = Vec::new();
            if let Some(code) = rrbc {
                resources.push((format!("{}.rrbc", plan.info.exe), code.to_vec()));
            }
            let app = rapidr_package::macos::write(
                dest_dir,
                &rapidr_package::macos::MacApp { info: &plan.info, icon: &plan.icon, executable: binary, resources, minimum_system: crate::macos::DEPLOYMENT_TARGET },
            )?;
            let notices_at = app.join("Contents/Resources");
            notices::write(&notices_at, notices).map_err(|e| notices::missing(&e))?;
            match rapidr_package::macos::sign(&app)? {
                true => println!("Signed (ad hoc): {}", app.display()),
                false => println!("note: {} is not signed: codesign is macOS' (sign it there, or Apple silicon Macs won't open it)", app.display()),
            }
            let executable = app.join("Contents/MacOS").join(&plan.info.exe);
            Made { path: app, executable }
        }
        (System::Linux, true) => {
            let dir = rapidr_package::linux::write(
                dest_dir,
                &rapidr_package::linux::AppDir { info: &plan.info, icon: &plan.icon, executable: &whole(binary)?, files: Vec::new() },
            )?;
            notices::write(&dir, notices).map_err(|e| notices::missing(&e))?;
            let executable = dir.join("usr/bin").join(&plan.info.exe);
            Made { path: dir, executable }
        }
        (system, bundle) => {
            let mut bytes = binary.to_vec();
            if system == System::Windows && bundle {
                bytes = rapidr_package::windows::stamp(&bytes, &plan.info, &plan.icon)?;
            }
            let path = dest_dir.join(format!("{}{exe_suffix}", plan.info.exe));
            fs::write(&path, whole(&bytes)?).map_err(|e| format!("write {}: {e}", path.display()))?;
            make_executable(&path);
            notices::write(dest_dir, notices).map_err(|e| notices::missing(&e))?;
            Made { executable: path.clone(), path }
        }
    };
    let what = match (plan.system, plan.bundle) {
        (System::MacOs, true) => "App",
        (System::Linux, true) => "AppDir",
        _ => "Executable",
    };
    println!("{what}: {}", made.path.display());
    if plan.bundle {
        println!("  {} {} ({}), icon: {}", plan.info.name, plan.info.version, plan.info.bundle_id, plan.icon.origin);
    }
    Ok(made)
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
    match rapidr_package::linux::install(appdir, &data_home) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn options_from_the_command_line() {
        let a = args(&["--icon", "a.png", "--name=Note Pad", "-gc:\\x.ico", "--no-bundle", "--app-version", "2.0", "other"]);
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

        let p = plan("demo", System::MacOs, false, &o, Some(&project), Some(&src)).unwrap();
        assert!(p.icon.origin.ends_with("proj.png") && p.bundle && p.info.name == "Demo" && p.info.version == "3.1");
        o.icon = Some(cli.to_string_lossy().into());
        o.name = Some("Cli".into());
        let p = plan("demo", System::MacOs, false, &o, Some(&project), Some(&src)).unwrap();
        assert!(p.icon.origin.ends_with("cli.png") && p.info.name == "Cli" && p.info.bundle_id == "dev.rapidr.app.cli");
        o.icon = None;
        o.name = None;
        // without a project: $OPTION ICON over -g, then -g, then RapidR's
        let p = plan("demo", System::Linux, false, &o, None, Some(&src)).unwrap();
        assert!(p.icon.origin.ends_with("src.png") && p.info.name == "demo");
        assert!(plan("demo", System::Linux, false, &o, None, None).unwrap().icon.origin.ends_with("rc.png"));
        o.rc_icon = None;
        let p = plan("demo", System::Linux, true, &o, None, None).unwrap();
        assert!(p.icon.is_default() && !p.bundle, "a console program stays an executable");
        assert!(plan("demo", System::Windows, true, &o, None, None).unwrap().bundle, "a Windows .exe always gets its icon");
        // RC.EXE's -g message
        o.rc_icon = Some("nowhere.ico".into());
        assert_eq!(plan("demo", System::MacOs, false, &o, None, None).err().unwrap(), "ERROR: Can't find icon nowhere.ico");
        o.rc_icon = None;
        o.version = Some("x".into());
        assert!(plan("demo", System::MacOs, false, &o, None, None).err().unwrap().contains("version"));

        // the project beside a source names it as its main file
        project.1.save(&dir.join("demo.rrproj")).unwrap();
        fs::write(dir.join("demo.rr"), "PRINT 1\n").unwrap();
        fs::write(dir.join("other.rr"), "PRINT 2\n").unwrap();
        assert_eq!(project_for(&dir.join("demo.rr"), None).unwrap().unwrap().1.name, "Demo");
        assert!(project_for(&dir.join("other.rr"), None).unwrap().is_none());
        fs::remove_dir_all(&dir).unwrap();
    }
}

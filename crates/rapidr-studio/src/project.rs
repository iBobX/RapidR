//! RPROJECT: a project as RapidR Studio opens it (docs/ide-components.md
//! §3.9), over `rapidr-project`. Its files are read and written through the
//! runtime's file hooks — the disk on the desktop, the page's store on the
//! web (the files the user picked, written back to them).
//!
//! | Member | |
//! |---|---|
//! | `Open(Path)` → True / False | a `.rrproj` (format 2, or the web IDE's v1 JSON, imported: its files written beside it where none is there yet), or a `.bas` / `.rr` / `.inc` file with the files it `$INCLUDE`s (RapidQ's way: no project file) |
//! | `Save([Path])` → True / False | the `.rrproj` (an implicit project becomes one: `<folder>/<Name>.rrproj` or Path) |
//! | `New(Template, Name, Folder)` → True / False | `console` or `gui`: `main.rr` and `<Name>.rrproj` written in Folder |
//! | `AddFile(Path [, Kind])`, `RemoveFile(Path)` → True / False | paths relative to the project's folder (or absolute inside it) |
//! | `Close` | no project |
//! | `File(i)`, `FileKind(i)`, `FullPath(i)` | file i (from 0): its project path, kind (`module`, `form`, `include`, `resource`, `asset`, `data`), path to open |
//! | `FileName`, `Folder`, `Kind` (`project`, `file`, `v1`, "" none), `Error` | read-only |
//! | `Name`, `MainFile`, `CompatMode` (`"rapidq"` or "") | read / write |
//! | `FileCount` | read-only |
//! | `OnChange` | the project opened, closed, saved, or its files changed |

use std::cell::RefCell;
use std::collections::HashMap;

use rapidr_project::{import_v1, is_v1_json, kind_for_path, normalize_path, FileKind, Project};
use rapidr_value::Value;

use crate::{flag, folder_of, int_arg, join, read_text, slashes, text_arg, write_text, Host};

#[derive(Default)]
struct Model {
    project: Project,
    kind: &'static str,
    file_name: String,
    folder: String,
    error: String,
    /// What the last Build made (an app or executable; "" none)
    built: String,
}

thread_local! {
    static PROJECTS: RefCell<HashMap<String, Model>> = RefCell::new(HashMap::new());
}

fn with<R>(name: &str, f: impl FnOnce(&mut Model) -> R) -> R {
    PROJECTS.with(|p| f(p.borrow_mut().entry(name.to_ascii_lowercase()).or_default()))
}

fn kind_name(k: FileKind) -> &'static str {
    k.as_str()
}

fn kind_parse(s: &str) -> Option<FileKind> {
    Some(match s.to_ascii_lowercase().as_str() {
        "module" => FileKind::Module,
        "form" => FileKind::Form,
        "include" => FileKind::Include,
        "resource" => FileKind::Resource,
        "asset" => FileKind::Asset,
        "data" => FileKind::Data,
        _ => return None,
    })
}

pub fn get(name: &str, prop: &str) -> Option<Value> {
    with(name, |m| {
        Some(match prop {
            "filename" => Value::String(m.file_name.clone()),
            "folder" => Value::String(m.folder.clone()),
            "kind" => Value::String(m.kind.to_string()),
            "error" => Value::String(m.error.clone()),
            "name" => Value::String(m.project.name.clone()),
            "mainfile" => Value::String(m.project.main.clone()),
            "filecount" => Value::Integer(if m.kind.is_empty() { 0 } else { m.project.file_count() as i64 }),
            "compatmode" => Value::String(m.project.compat_mode().to_string()),
            "icon" => Value::String(m.project.build.icon.clone()),
            "appname" => Value::String(m.project.build.app_name.clone()),
            "bundleid" => Value::String(m.project.build.bundle_id.clone()),
            "version" => Value::String(m.project.build.version.clone()),
            "company" => Value::String(m.project.build.company.clone()),
            "buildkind" => Value::String(build_kind(&m.project).to_string()),
            "building" => flag(crate::build::building(name)),
            "builtpath" => Value::String(m.built.clone()),
            // (Reveal's: what the system calls its file manager)
            "filemanager" => Value::String(if cfg!(target_arch = "wasm32") { "" } else if cfg!(target_os = "macos") { "Finder" } else if cfg!(windows) { "File Explorer" } else { "Files" }.into()),
            _ => return None,
        })
    })
}

/// `native` (compiled with Rust) or `interpreted` (the runner and the
/// program's bytecode: no Rust needed), from `[build] targets`.
fn build_kind(p: &Project) -> &'static str {
    if p.build.targets.iter().any(|t| matches!(t.to_ascii_lowercase().as_str(), "bytecode" | "interpreted" | "interp")) {
        "interpreted"
    } else {
        "native"
    }
}

pub fn set(name: &str, prop: &str, v: &Value) -> bool {
    with(name, |m| {
        match prop {
            "name" => m.project.name = v.to_string_val(),
            "mainfile" => m.project.main = normalize_path(&v.to_string_val()),
            "compatmode" => m.project.compat.rapidq_compatible = v.to_string_val().eq_ignore_ascii_case("rapidq"),
            "icon" => m.project.build.icon = project_path(&m.folder, v.to_string_val().trim()),
            "appname" => m.project.build.app_name = v.to_string_val().trim().to_string(),
            "bundleid" => m.project.build.bundle_id = v.to_string_val().trim().to_string(),
            "version" => m.project.build.version = v.to_string_val().trim().to_string(),
            "company" => m.project.build.company = v.to_string_val().trim().to_string(),
            "buildkind" => {
                let kind = if v.to_string_val().trim().eq_ignore_ascii_case("interpreted") { "bytecode" } else { "native" };
                m.project.build.targets.retain(|t| !matches!(t.to_ascii_lowercase().as_str(), "native" | "bytecode" | "interpreted" | "interp"));
                m.project.build.targets.insert(0, kind.to_string());
            }
            "filename" | "folder" | "kind" | "error" | "filecount" | "building" | "builtpath" | "filemanager" => {}
            _ => return false,
        }
        true
    })
}

/// A project path for `path` (relative to the project's folder; an
/// absolute path inside it made relative).
fn project_path(folder: &str, path: &str) -> String {
    let p = slashes(path);
    if !folder.is_empty() {
        let prefix = format!("{}/", folder.trim_end_matches('/'));
        if let Some(rest) = p.strip_prefix(&prefix) {
            return normalize_path(rest);
        }
    }
    normalize_path(&p)
}

/// The file a folder opens as a project: its `.rrproj` (the one named as
/// the folder first), else its main source — `main.rr` / `main.bas`, the
/// source named as the folder, the one that makes a form, the first.
pub fn main_of_folder(folder: &str, files: &[String]) -> Option<String> {
    let base = folder.rsplit('/').next().unwrap_or(folder).to_lowercase();
    let ext = |f: &str| f.rsplit('.').next().unwrap_or("").to_lowercase();
    let stem = |f: &str| f.rsplit_once('.').map_or(f, |(s, _)| s).to_lowercase();
    let mut projects: Vec<&String> = files.iter().filter(|f| ext(f) == "rrproj").collect();
    projects.sort_by_key(|f| (stem(f) != base, f.to_lowercase()));
    if let Some(p) = projects.first() {
        return Some((*p).clone());
    }
    let mut sources: Vec<&String> = files.iter().filter(|f| matches!(ext(f).as_str(), "rr" | "bas")).collect();
    sources.sort_by_key(|f| {
        let s = stem(f);
        let form = read_text(&join(folder, f)).map(|t| rapidr_project::defines_form(&t)).unwrap_or(false);
        (s != "main", s != base, !form, f.to_lowercase())
    });
    sources.first().map(|f| (*f).clone())
}

/// A program's files to compile it where there is no file system (the web):
/// its main file's name and every (path, text) — the main file and what it
/// `$INCLUDE`s, paths relative to its folder.
pub fn program_files(program: &str) -> Result<(String, Vec<(String, String)>), String> {
    let program = slashes(program);
    let folder = folder_of(&program);
    let main = program.rsplit('/').next().unwrap_or(&program).to_string();
    let main_text = read_text(&program)?;
    let resolve = |rel: &str| -> Option<(String, String)> {
        if rel.eq_ignore_ascii_case(&main) {
            return Some((main.clone(), main_text.clone()));
        }
        read_text(&join(&folder, rel)).ok().map(|t| (rel.to_string(), t))
    };
    let project = rapidr_project::implicit_from_resolver(&main, &resolve);
    let mut files = vec![(main.clone(), main_text.clone())];
    for f in project.files.iter().filter(|f| !f.path.eq_ignore_ascii_case(&main)) {
        if let Ok(t) = read_text(&join(&folder, &f.path)) {
            files.push((f.path.clone(), t));
        }
    }
    Ok((main, files))
}

fn exists(path: &str) -> bool {
    rapidr_value::objects::read_file(path).is_ok()
}

fn open(m: &mut Model, path: &str) -> Result<(), String> {
    let path = slashes(path);
    let folder = folder_of(&path);
    let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    let (project, kind, file_name) = match ext.as_str() {
        "rrproj" | "json" => {
            let text = read_text(&path)?;
            if is_v1_json(&text) {
                let imported = import_v1(&text).map_err(|e| e.to_string())?;
                // (its forms, modules and assets as files beside it — only
                // where no file has that name yet)
                for (rel, bytes) in &imported.files {
                    let to = join(&folder, rel);
                    if !exists(&to) {
                        rapidr_value::objects::write_file(&to, bytes)?;
                    }
                }
                let file = join(&folder, &format!("{}.rrproj", imported.project.name));
                (imported.project, "v1", file)
            } else if text.trim_start().starts_with('{') {
                return Err(format!("{path}: not a RapidR project"));
            } else {
                (Project::from_toml(&text).map_err(|e| e.to_string())?, "project", path.clone())
            }
        }
        "bas" | "rr" | "inc" => {
            let main = path.rsplit('/').next().unwrap_or(&path).to_string();
            let resolve = |rel: &str| -> Option<(String, String)> { read_text(&join(&folder, rel)).ok().map(|t| (rel.to_string(), t)) };
            if !exists(&path) {
                return Err(format!("{path}: no such file"));
            }
            (rapidr_project::implicit_from_resolver(&main, &resolve), "file", String::new())
        }
        _ => return Err(format!("{path}: not a project or a source file")),
    };
    *m = Model { project, kind, file_name, folder, ..Model::default() };
    Ok(())
}

fn save(m: &mut Model, to: &str) -> Result<(), String> {
    if m.kind.is_empty() {
        return Err("no project is open".into());
    }
    let file = if !to.is_empty() {
        slashes(to)
    } else if !m.file_name.is_empty() {
        m.file_name.clone()
    } else {
        join(&m.folder, &format!("{}.rrproj", if m.project.name.is_empty() { "project" } else { &m.project.name }))
    };
    write_text(&file, &m.project.to_toml())?;
    m.folder = folder_of(&file);
    m.file_name = file;
    m.kind = "project";
    Ok(())
}

fn new(m: &mut Model, template: &str, name: &str, folder: &str) -> Result<(), String> {
    let (project, files) = Project::new_from_template(name, if template.is_empty() { "gui" } else { template }).map_err(|e| e.to_string())?;
    let folder = slashes(folder).trim_end_matches('/').to_string();
    for (rel, text) in &files {
        write_text(&join(&folder, rel), text)?;
    }
    let file = join(&folder, &format!("{name}.rrproj"));
    write_text(&file, &project.to_toml())?;
    *m = Model { project, kind: "project", file_name: file, folder, ..Model::default() };
    Ok(())
}

/// `rapidr build`'s arguments for the open project (and the folder it runs
/// in): the main file, the `.rrproj`, what the project says that isn't
/// saved yet, `--interp` for an interpreted build (`kind`, else the
/// project's BuildKind).
fn build_args(name: &str, kind: &str) -> Result<(Vec<String>, String), String> {
    with(name, |m| {
        if m.kind.is_empty() {
            return Err("no project is open".to_string());
        }
        let main = join(&m.folder, &m.project.main);
        let mut args = vec!["build".to_string(), main];
        if m.kind == "project" && !m.file_name.is_empty() {
            args.extend(["--project".to_string(), m.file_name.clone()]);
        }
        let b = &m.project.build;
        let mut put = |flag: &str, v: &str| {
            if !v.trim().is_empty() {
                args.extend([flag.to_string(), v.trim().to_string()]);
            }
        };
        put("--name", &b.app_name);
        put("--bundle-id", &b.bundle_id);
        put("--app-version", &b.version);
        put("--company", &b.company);
        if !b.icon.trim().is_empty() {
            put("--icon", &join(&m.folder, &b.icon));
        }
        let kind = if kind.is_empty() { build_kind(&m.project) } else { kind };
        if kind.eq_ignore_ascii_case("interpreted") {
            args.push("--interp".into());
        }
        Ok((args, m.folder.clone()))
    })
}

/// The project's icon (its own, else RapidR's default for programs) as a
/// PNG of `px`, written under the project's `.rapidr/` folder: its path ("" when
/// it can't be made — Error says why).
fn icon_preview(name: &str, px: u32) -> String {
    let made = with(name, |m| -> Result<String, String> {
        let icon = if m.project.build.icon.trim().is_empty() {
            rapidr_package::Icon::rapidr_default()
        } else {
            let path = join(&m.folder, &m.project.build.icon);
            let bytes = rapidr_value::objects::read_file(&path).map_err(|e| format!("{path}: {e}"))?;
            rapidr_package::Icon::from_bytes(&bytes, &path)?
        };
        let out = join(&join(&m.folder, rapidr_project::STATE_DIR), &format!("icon-preview-{px}.png"));
        #[cfg(not(target_arch = "wasm32"))]
        let _ = std::fs::create_dir_all(folder_of(&out));
        rapidr_value::objects::write_file(&out, &icon.png(px))?;
        Ok(out)
    });
    with(name, |m| match made {
        Ok(p) => {
            m.error.clear();
            p
        }
        Err(e) => {
            m.error = e;
            String::new()
        }
    })
}

/// A finished build's app, remembered for Reveal and BuiltPath.
pub(crate) fn set_built(name: &str, path: &str) {
    with(name, |m| m.built = path.to_string());
}

pub fn call<H: Host>(host: H, name: &str, method: &str, args: &[Value]) -> Option<Value> {
    let s = |i: usize| text_arg(args, i);
    let i = int_arg(args, 0, -1);
    // (what changes the project answers True / False, says why in Error,
    // and fires OnChange)
    let changing = |f: &dyn Fn(&mut Model) -> Result<(), String>| -> Value {
        let r = with(name, |m| {
            let r = f(m);
            m.error = r.clone().err().unwrap_or_default();
            r
        });
        if r.is_ok() {
            host.fire(name, "onchange", &[]);
        }
        flag(r.is_ok())
    };
    Some(match method {
        "open" => changing(&|m| open(m, &s(0))),
        "openfolder" => {
            let folder = slashes(&s(0)).trim_end_matches('/').to_string();
            let files = host.list_files(&folder);
            match main_of_folder(&folder, &files) {
                Some(f) => changing(&|m| open(m, &join(&folder, &f))),
                None => {
                    with(name, |m| m.error = format!("{folder}: no RapidR project or source in it"));
                    flag(false)
                }
            }
        }
        "save" => changing(&|m| save(m, &s(0))),
        "new" => changing(&|m| new(m, &s(0), &s(1), &s(2))),
        "close" => changing(&|m| {
            *m = Model::default();
            Ok(())
        }),
        "addfile" => changing(&|m| {
            if m.kind.is_empty() {
                return Err("no project is open".into());
            }
            let rel = project_path(&m.folder, &s(0));
            let kind = kind_parse(&s(1)).unwrap_or_else(|| kind_for_path(&rel));
            if m.project.add_file(&rel, kind) { Ok(()) } else { Err(format!("{rel} is in the project already")) }
        }),
        "removefile" => changing(&|m| {
            let rel = project_path(&m.folder, &s(0));
            if m.project.remove_file(&rel) { Ok(()) } else { Err(format!("{rel} isn't in the project")) }
        }),
        "file" => Value::String(with(name, |m| m.project.file(i.max(0) as usize).filter(|_| i >= 0).map(|f| f.path.clone()).unwrap_or_default())),
        "filekind" => Value::String(with(name, |m| m.project.file(i.max(0) as usize).filter(|_| i >= 0).map(|f| kind_name(f.kind).to_string()).unwrap_or_default())),
        "build" => {
            let started = build_args(name, &s(0)).and_then(|(args, cwd)| {
                let rapidr = host.rapidr().ok_or("Build makes apps in RapidR Studio on the desktop (macOS, Windows, Linux); on the web, Run shows the program")?;
                crate::build::start(name, rapidr, &args, &cwd)
            });
            let ok = started.is_ok();
            with(name, |m| {
                m.error = started.err().unwrap_or_default();
                if ok {
                    m.built.clear();
                }
            });
            flag(ok)
        }
        "stopbuild" => {
            crate::build::stop(name);
            Value::Null
        }
        "reveal" => {
            let path = match s(0) {
                p if p.is_empty() => with(name, |m| m.built.clone()),
                p => p,
            };
            let r = if path.is_empty() { Err("nothing built yet".to_string()) } else { host.reveal(&path) };
            let ok = r.is_ok();
            with(name, |m| m.error = r.err().unwrap_or_default());
            flag(ok)
        }
        "iconpreview" => Value::String(icon_preview(name, int_arg(args, 0, 128).clamp(16, 1024) as u32)),
        // (Find in Files) Find(Pattern, Options, Text): the matches of one
        // text, a line each; Replace(Pattern, Options, Text, With): the text
        // replaced; FileText(Path): a file's text (the disk's, the web's store)
        "find" | "replace" => {
            let r = if method == "find" { crate::find::find(&s(2), &s(0), &s(1)) } else { crate::find::replace(&s(2), &s(0), &s(1), &s(3)).map(|(t, _)| t) };
            let ok = r.is_ok();
            with(name, |m| m.error = r.as_ref().err().cloned().unwrap_or_default());
            Value::String(if ok { r.unwrap() } else if method == "find" { String::new() } else { s(2) })
        }
        "filetext" => {
            let r = crate::read_text(&s(0));
            with(name, |m| m.error = r.as_ref().err().cloned().unwrap_or_default());
            Value::String(r.unwrap_or_default())
        }
        "fullpath" => Value::String(with(name, |m| m.project.file(i.max(0) as usize).filter(|_| i >= 0).map(|f| join(&m.folder, &f.path)).unwrap_or_default())),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy)]
    struct Quiet;
    impl Host for Quiet {
        fn fire(self, _name: &str, _event: &str, _args: &[Value]) {}
        fn launch(self, _p: &str, _a: &[String], _t: &str) -> Result<Box<dyn crate::Transport>, String> {
            Err("no".into())
        }
        fn list_files(self, folder: &str) -> Vec<String> {
            std::fs::read_dir(folder).map(|d| d.filter_map(|e| e.ok()).filter(|e| e.path().is_file()).map(|e| e.file_name().to_string_lossy().into_owned()).collect()).unwrap_or_default()
        }
    }

    #[test]
    fn open_a_file_new_save_add() {
        let dir = std::env::temp_dir().join(format!("rapidr-studio-project-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let d = slashes(&dir.to_string_lossy());
        std::fs::write(dir.join("main.bas"), "$INCLUDE \"lib.inc\"\nPRINT 1\n").unwrap();
        std::fs::write(dir.join("lib.inc"), "SUB X\nEND SUB\n").unwrap();
        let call = |m: &str, args: &[Value]| call(Quiet, "p1", m, args).unwrap();
        let get = |p: &str| get("p1", p).unwrap().to_string_val();
        assert_eq!(call("open", &[Value::String(format!("{d}/main.bas"))]).to_i64(), 1);
        assert_eq!((get("kind"), get("filecount"), get("mainfile")), ("file".into(), "2".into(), "main.bas".into()));
        assert_eq!(call("file", &[Value::Integer(1)]).to_string_val(), "lib.inc");
        assert_eq!(call("filekind", &[Value::Integer(1)]).to_string_val(), "include");
        assert_eq!(call("fullpath", &[Value::Integer(0)]).to_string_val(), format!("{d}/main.bas"));
        assert_eq!(call("save", &[]).to_i64(), 1);
        assert!(dir.join("main.rrproj").is_file());
        assert_eq!(call("open", &[Value::String(format!("{d}/main.rrproj"))]).to_i64(), 1);
        assert_eq!(get("kind"), "project");
        assert_eq!(call("addfile", &[Value::String(format!("{d}/data.csv"))]).to_i64(), 1);
        assert_eq!(call("filekind", &[Value::Integer(2)]).to_string_val(), "data");
        assert_eq!(call("addfile", &[Value::String("data.csv".into())]).to_i64(), 0);
        assert!(get("error").contains("already"));
        let new_dir = format!("{d}/fresh");
        std::fs::create_dir_all(&new_dir).unwrap();
        assert_eq!(call("new", &[Value::String("gui".into()), Value::String("Fresh".into()), Value::String(new_dir.clone())]).to_i64(), 1);
        assert!(std::path::Path::new(&new_dir).join("main.rr").is_file() && std::path::Path::new(&new_dir).join("Fresh.rrproj").is_file());
        // the app's settings, the build's arguments, the icon's preview
        assert_eq!(call("open", &[Value::String(format!("{d}/main.rrproj"))]).to_i64(), 1);
        let set = |p: &str, v: &str| assert!(set("p1", p, &Value::String(v.into())));
        set("appname", "Main App");
        set("version", "2.0");
        set("icon", &format!("{d}/art/app.svg"));
        set("buildkind", "interpreted");
        assert_eq!((get("icon"), get("buildkind")), ("art/app.svg".into(), "interpreted".into()));
        let (args, cwd) = build_args("p1", "").unwrap();
        assert_eq!(cwd, d);
        assert_eq!(args[..2], ["build".to_string(), format!("{d}/main.bas")]);
        for want in ["--project", "--name", "Main App", "--app-version", "2.0", "--icon", &format!("{d}/art/app.svg"), "--interp"] {
            assert!(args.iter().any(|a| a == want), "{want}: {args:?}");
        }
        assert!(!build_args("p1", "native").unwrap().0.contains(&"--interp".to_string()));
        // (no icon file yet: Error says so; RapidR's default draws)
        assert_eq!(call("iconpreview", &[Value::Integer(64)]).to_string_val(), "");
        assert!(!get("error").is_empty());
        set("icon", "");
        let preview = call("iconpreview", &[Value::Integer(64)]).to_string_val();
        assert!(preview.ends_with(".rapidr/icon-preview-64.png") && std::path::Path::new(&preview).is_file(), "{preview}");
        assert_eq!(call("build", &[]).to_i64(), 0, "the quiet host has no rapidr");
        assert!(get("error").contains("desktop"));
        assert_eq!(call("open", &[Value::String(format!("{d}/nope.rr"))]).to_i64(), 0);
        assert!(get("error").contains("no such file"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

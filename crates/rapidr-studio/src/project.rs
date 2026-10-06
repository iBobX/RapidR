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
            _ => return None,
        })
    })
}

pub fn set(name: &str, prop: &str, v: &Value) -> bool {
    with(name, |m| {
        match prop {
            "name" => m.project.name = v.to_string_val(),
            "mainfile" => m.project.main = normalize_path(&v.to_string_val()),
            "compatmode" => m.project.compat.rapidq_compatible = v.to_string_val().eq_ignore_ascii_case("rapidq"),
            "filename" | "folder" | "kind" | "error" | "filecount" => {}
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
    *m = Model { project, kind, file_name, folder, error: String::new() };
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
    *m = Model { project, kind: "project", file_name: file, folder, error: String::new() };
    Ok(())
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
        fn launch(self, _p: &str, _a: &[String]) -> Result<Box<dyn crate::Transport>, String> {
            Err("no".into())
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
        assert_eq!(call("open", &[Value::String(format!("{d}/main.bas"))]).to_i64(), -1);
        assert_eq!((get("kind"), get("filecount"), get("mainfile")), ("file".into(), "2".into(), "main.bas".into()));
        assert_eq!(call("file", &[Value::Integer(1)]).to_string_val(), "lib.inc");
        assert_eq!(call("filekind", &[Value::Integer(1)]).to_string_val(), "include");
        assert_eq!(call("fullpath", &[Value::Integer(0)]).to_string_val(), format!("{d}/main.bas"));
        assert_eq!(call("save", &[]).to_i64(), -1);
        assert!(dir.join("main.rrproj").is_file());
        assert_eq!(call("open", &[Value::String(format!("{d}/main.rrproj"))]).to_i64(), -1);
        assert_eq!(get("kind"), "project");
        assert_eq!(call("addfile", &[Value::String(format!("{d}/data.csv"))]).to_i64(), -1);
        assert_eq!(call("filekind", &[Value::Integer(2)]).to_string_val(), "data");
        assert_eq!(call("addfile", &[Value::String("data.csv".into())]).to_i64(), 0);
        assert!(get("error").contains("already"));
        let new_dir = format!("{d}/fresh");
        std::fs::create_dir_all(&new_dir).unwrap();
        assert_eq!(call("new", &[Value::String("gui".into()), Value::String("Fresh".into()), Value::String(new_dir.clone())]).to_i64(), -1);
        assert!(std::path::Path::new(&new_dir).join("main.rr").is_file() && std::path::Path::new(&new_dir).join("Fresh.rrproj").is_file());
        assert_eq!(call("open", &[Value::String(format!("{d}/nope.rr"))]).to_i64(), 0);
        assert!(get("error").contains("no such file"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

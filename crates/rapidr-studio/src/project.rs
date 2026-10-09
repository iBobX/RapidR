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
//! | `ImportRapidQ(Source [, Dest])` → the copy's project file | File ▸ Import RapidQ Project or File… (`crate::import`): a copy of a RapidQ program or folder with RapidR's names, its report and project; "" (Error says why) when it can't |
//! | `ImportSummary`, `ImportReport` | the last import's summary line and its report's file |
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
    /// The last import's (summary, report file), by component: kept when a
    /// project is opened (the copy is, next).
    static IMPORTED: RefCell<HashMap<String, (String, String)>> = RefCell::new(HashMap::new());
}

fn imported(name: &str) -> (String, String) {
    IMPORTED.with(|i| i.borrow().get(&name.to_ascii_lowercase()).cloned().unwrap_or_default())
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
            "outputfolder" => Value::String(m.project.build.output.clone()),
            "keeprust" => flag(m.project.build.keep_rust),
            "building" => flag(crate::build::building(name)),
            "builtpath" => Value::String(m.built.clone()),
            "importsummary" => Value::String(imported(name).0),
            "importreport" => Value::String(imported(name).1),
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

/// The project builds interpreted (`bytecode`) or native.
fn set_build_kind(p: &mut Project, interpreted: bool) {
    p.build.targets.retain(|t| !matches!(t.to_ascii_lowercase().as_str(), "native" | "bytecode" | "interpreted" | "interp"));
    p.build.targets.insert(0, if interpreted { "bytecode" } else { "native" }.to_string());
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
            "outputfolder" => m.project.build.output = project_path(&m.folder, v.to_string_val().trim()),
            "keeprust" => m.project.build.keep_rust = v.to_i64() != 0 || v.to_string_val().eq_ignore_ascii_case("true"),
            "buildkind" => set_build_kind(&mut m.project, v.to_string_val().trim().eq_ignore_ascii_case("interpreted")),
            "filename" | "folder" | "kind" | "error" | "filecount" | "building" | "builtpath" | "filemanager" | "importsummary" | "importreport" => {}
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
    let mut sources: Vec<&String> = files.iter().filter(|f| matches!(ext(f).as_str(), "rr" | "bas" | "rqw" | "rqb" | "rq")).collect();
    sources.sort_by_key(|f| {
        let s = stem(f);
        let form = read_text(&join(folder, f)).map(|t| rapidr_project::defines_form(&t)).unwrap_or(false);
        (s != "main", s != base, !form, f.to_lowercase())
    });
    sources.first().map(|f| (*f).clone())
}

/// A program's files to run it where there is no file system (the web):
/// its main file's name, every source (path, text) — the main file and what
/// it `$INCLUDE`s — and every data file (path, bytes) its sources name that
/// is there (`rapidr_project::named_files`: a `$RESOURCE`'s file, a CSV it
/// loads …), paths relative to its folder. The compiler builds the
/// `$RESOURCE`s in; the program reads the others as files beside it.
pub fn program_files(program: &str) -> Result<ProgramFiles, String> {
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
    let mut sources = vec![(main.clone(), main_text.clone())];
    let mut data = Vec::new();
    // (what a browser's program gets: its data, up to this much)
    let mut room: usize = 64 << 20;
    for f in project.files.iter().filter(|f| !f.path.eq_ignore_ascii_case(&main)) {
        match f.kind {
            FileKind::Module | FileKind::Form | FileKind::Include => {
                if let Ok(t) = read_text(&join(&folder, &f.path)) {
                    sources.push((f.path.clone(), t));
                }
            }
            _ => {
                if let Ok(bytes) = rapidr_value::objects::read_file(&join(&folder, &f.path)) {
                    if bytes.len() <= room {
                        room -= bytes.len();
                        data.push((f.path.clone(), bytes));
                    }
                }
            }
        }
    }
    Ok(ProgramFiles { main, sources, data })
}

/// [`program_files`]' answer.
pub struct ProgramFiles {
    /// The main file's name.
    pub main: String,
    /// (path, text) of each source.
    pub sources: Vec<(String, String)>,
    /// (path, bytes) of each data file.
    pub data: Vec<(String, Vec<u8>)>,
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
        // (RapidR's and RapidQ's sources: rapidr_preprocessor::SOURCE_EXTENSIONS)
        "bas" | "rr" | "inc" | "rqw" | "rqb" | "rq" => {
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

/// A new project; `interpreted`: it builds interpreted (a computer without
/// Rust: the build that works there; Project Options can change it).
fn new(m: &mut Model, template: &str, name: &str, folder: &str, interpreted: bool) -> Result<(), String> {
    let (mut project, files) = Project::new_from_template(name, if template.is_empty() { "gui" } else { template }).map_err(|e| e.to_string())?;
    if interpreted {
        set_build_kind(&mut project, true);
    }
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
        // (in full: the build runs in the project's folder, and Studio may
        // have opened it by a relative path)
        let folder = full(&m.folder);
        let main = join(&folder, &m.project.main);
        let mut args = vec!["build".to_string(), main];
        if m.kind == "project" && !m.file_name.is_empty() {
            args.extend(["--project".to_string(), full(&m.file_name)]);
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
            put("--icon", &join(&folder, &b.icon));
        }
        let kind = if kind.is_empty() { build_kind(&m.project) } else { kind };
        if kind.eq_ignore_ascii_case("interpreted") {
            args.push("--interp".into());
        } else if kind.eq_ignore_ascii_case("web") {
            // (a web app: the bytecode and the web runtime, a .zip)
            args.extend(["--web".to_string(), "--interp".to_string()]);
        }
        // (the app in the project's output folder, `build` by default, a
        // single file's project too; its generated Rust kept or not)
        args.extend(["--output".to_string(), join(&folder, b.output_folder())]);
        args.push(if b.keep_rust { "--keep-rust" } else { "--no-keep-rust" }.to_string());
        Ok((args, folder))
    })
}

/// The web build the page makes (Build Web App): the project's main file, as
/// `<name>-web.zip` — the name of its project's file, else of the program.
fn web_build<H: Host>(host: H, name: &str) -> Result<(), String> {
    let (main, stem) = with(name, |m| {
        if m.kind.is_empty() {
            return Err("no project is open".to_string());
        }
        let main = join(&full(&m.folder), &m.project.main);
        let named = if m.kind == "project" && !m.file_name.is_empty() { m.file_name.as_str() } else { main.as_str() };
        let file = slashes(named);
        let file = file.rsplit('/').next().unwrap_or("").to_string();
        let stem = file.rsplit_once('.').map_or(file.clone(), |(s, _)| s.to_string());
        Ok((main, if stem.is_empty() { "program".to_string() } else { stem }))
    })?;
    crate::build::web_start(name)?;
    host.build_web(name, &main, &stem).inspect_err(|_| crate::build::stop(name))
}

/// A path in full (relative ones from the current folder); as it is where
/// there is no file system (the web).
fn full(path: &str) -> String {
    #[cfg(not(target_arch = "wasm32"))]
    if let Ok(p) = std::path::absolute(path) {
        return slashes(&p.to_string_lossy());
    }
    path.to_string()
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
        // (the desktop only asks whether Rust is there; the web builds nothing)
        "new" => {
            let interpreted = host.rapidr().is_some_and(|r| !crate::build::rust_ready(&r));
            changing(&|m| new(m, &s(0), &s(1), &s(2), interpreted))
        }
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
            let kind = s(0);
            let started = if kind.eq_ignore_ascii_case("web") && host.rapidr().is_none() {
                // (the web page builds the web app itself)
                web_build(host, name)
            } else {
                build_args(name, &kind).and_then(|(args, cwd)| {
                    let rapidr = host.rapidr().ok_or("Build Native App and Build Interpreted App make apps in RapidR Studio on the desktop (macOS, Windows, Linux); on the web, Build Web App makes a web app and Run shows the program")?;
                    crate::build::start(name, rapidr, &args, &cwd)
                })
            };
            let ok = started.is_ok();
            with(name, |m| {
                m.error = started.err().unwrap_or_default();
                if ok {
                    m.built.clear();
                }
            });
            flag(ok)
        }
        // (the desktop asks `rapidr setup --rust`; the web has nothing to ask,
        // and Build says on its own that it is the desktop's)
        "rustready" => flag(host.rapidr().is_none_or(|r| crate::build::rust_ready(&r))),
        // (Rust installed by `rapidr setup`, run like a build: its lines come
        // back as OnBuildOutput, its end as OnBuildDone; nothing but Rust is
        // set up: no `rapidr` link on the PATH)
        "installrust" => {
            let started = host
                .rapidr()
                .ok_or("Installing Rust is the desktop's: run `rapidr setup` on the computer".to_string())
                .and_then(|rapidr| crate::build::start(name, rapidr, &["setup".to_string(), "--yes".to_string(), "--no-path".to_string()], ""));
            let ok = started.is_ok();
            with(name, |m| m.error = started.err().unwrap_or_default());
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
        // (File > Import RapidQ Project or File: a copy, never the original)
        "importrapidq" => {
            let r = crate::import::import(host, &s(0), &s(1));
            with(name, |m| m.error = r.as_ref().err().cloned().unwrap_or_default());
            match r {
                Ok(i) => {
                    IMPORTED.with(|x| x.borrow_mut().insert(name.to_ascii_lowercase(), (i.summary.clone(), i.report.clone())));
                    Value::String(i.project)
                }
                Err(_) => Value::String(String::new()),
            }
        }
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
        // (Project > Add Form / Add Module) NewFileText(Path, Kind,
        // MainText): a new file's text in the program's names;
        // IncludeEdit(MainText, Path): the `$INCLUDE` the main program needs
        // for it, as one patch for RCODEEDITOR.ApplyPatches ("" when it has it)
        "newfiletext" => Value::String(with(name, |m| new_file_text(m, &s(0), &s(1), &s(2)))),
        "includeedit" => Value::String(with(name, |m| include_edit(m, &s(0), &s(1)))),
        // IncludeRenameEdit(MainText, OldPath, NewPath): the main program's
        // $INCLUDE of a file renamed or moved, following it ("" none)
        "includerenameedit" => Value::String(with(name, |m| include_rename_edit(m, &s(0), &s(1), &s(2)))),
        // IncludeRemoveEdit(MainText, Path): its $INCLUDE line taken out ("" none)
        "includeremoveedit" => Value::String(with(name, |m| match rapidr_project::forms::include_line(&s(0), &from_main(m, &s(1))) {
            Some(line) => format!("{line}\t0\t{}\t0\t", line + 1),
            None => String::new(),
        })),
        _ => return None,
    })
}

/// A file's name without its folder and extension.
fn stem(path: &str) -> String {
    let file = slashes(path).rsplit('/').next().unwrap_or("").to_string();
    match file.rfind('.') {
        Some(i) if i > 0 => file[..i].to_string(),
        _ => file,
    }
}

/// The text a new project file starts with: a form (`CREATE <its name> AS
/// RForm`, or QFORM in a program written with RapidQ's names or a
/// RapidQ-compatible project), a module, or nothing.
fn new_file_text(m: &Model, path: &str, kind: &str, main_text: &str) -> String {
    use rapidr_project::forms;
    let eol = forms::line_end(main_text);
    let title = slashes(path).rsplit('/').next().unwrap_or("").to_string();
    match kind.to_ascii_lowercase().as_str() {
        // ("formmdi": an MDI main window — the toolbox's RFormMDI)
        "form" | "formmdi" => {
            let rapidq = m.project.compat.rapidq_compatible || forms::uses_rapidq_names(main_text, &m.project.main);
            forms::new_form_text(&title, &stem(path), rapidq, kind.eq_ignore_ascii_case("formmdi"), eol)
        }
        "module" => forms::new_module_text(&title, eol),
        _ => String::new(),
    }
}

/// The `$INCLUDE` of `path` (a project path) the main program's text needs,
/// as an ApplyPatches line (`StartLine⇥StartCol⇥EndLine⇥EndCol⇥Text`); ""
/// when it includes it already. The path is written relative to the main
/// file's folder.
fn include_edit(m: &Model, main_text: &str, path: &str) -> String {
    match rapidr_project::forms::include_insertion(main_text, &from_main(m, path)) {
        Some((line, text)) => format!("{line}\t0\t{line}\t0\t{}", patch_escaped(&text)),
        None => String::new(),
    }
}

/// `path` (a project path) as the main file's `$INCLUDE` writes it: relative
/// to the main file's folder.
fn from_main(m: &Model, path: &str) -> String {
    let rel = project_path(&m.folder, path);
    match folder_of(&m.project.main).as_str() {
        "" => rel,
        f => rel.strip_prefix(&format!("{f}/")).map_or_else(|| format!("{}{rel}", "../".repeat(f.split('/').count())), str::to_string),
    }
}

/// The main program's `$INCLUDE` of `old` changed to name `new` (a file
/// renamed or moved), as an ApplyPatches line; "" when it doesn't include it.
fn include_rename_edit(m: &Model, main_text: &str, old: &str, new: &str) -> String {
    match rapidr_project::forms::include_rename(main_text, &from_main(m, old), &from_main(m, new)) {
        Some((line, start, end, text)) => format!("{line}\t{start}\t{line}\t{end}\t{}", patch_escaped(&text)),
        None => String::new(),
    }
}

/// A patch's text as ApplyPatches reads it: `\`, line breaks and tabs
/// escaped.
fn patch_escaped(text: &str) -> String {
    text.replace('\\', "\\\\").replace('\n', "\\n").replace('\r', "\\r").replace('\t', "\\t")
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

    /// A host whose `rapidr` is a program that exits with a code of its own.
    #[derive(Clone, Copy)]
    struct Rapidr(&'static str);
    impl Host for Rapidr {
        fn fire(self, _name: &str, _event: &str, _args: &[Value]) {}
        fn launch(self, _p: &str, _a: &[String], _t: &str) -> Result<Box<dyn crate::Transport>, String> {
            Err("no".into())
        }
        fn list_files(self, _folder: &str) -> Vec<String> {
            Vec::new()
        }
        fn rapidr(self) -> Option<std::path::PathBuf> {
            Some(self.0.into())
        }
    }

    /// Without Rust (`rapidr setup --rust` says no) a new project builds
    /// interpreted, saved so; with it, native; and RustReady says which.
    #[cfg(unix)]
    #[test]
    fn a_new_project_without_rust_builds_interpreted() {
        let dir = std::env::temp_dir().join(format!("rapidr-studio-norust-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let d = slashes(&dir.to_string_lossy());
        for (rapidr, ready, kind) in [("/usr/bin/false", 0, "interpreted"), ("/usr/bin/true", 1, "native")] {
            let make = |host: Rapidr, folder: &str| {
                std::fs::create_dir_all(folder).unwrap();
                call(host, "pn", "new", &[Value::String("console".into()), Value::String("Fresh".into()), Value::String(folder.into())]).unwrap().to_i64()
            };
            let folder = format!("{d}/{kind}");
            assert_eq!(call(Rapidr(rapidr), "pn", "rustready", &[]).unwrap().to_i64(), ready, "{rapidr}");
            assert_eq!(make(Rapidr(rapidr), &folder), 1);
            assert_eq!(get("pn", "buildkind").unwrap().to_string_val(), kind);
            // (and in the file: opened again, it builds the same)
            assert_eq!(call(Rapidr(rapidr), "pn", "open", &[Value::String(format!("{folder}/Fresh.rrproj"))]).unwrap().to_i64(), 1);
            assert_eq!(get("pn", "buildkind").unwrap().to_string_val(), kind);
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_new_form_file_and_its_include() {
        let main = "$APPTYPE GUI\n\nCREATE Form1 AS RForm\nEND CREATE\n\nForm1.ShowModal\n";
        let m = Model { project: Project::new("Demo", "main.rr"), kind: "project", folder: "/work/Demo".into(), ..Model::default() };
        let form = new_file_text(&m, "Form2.rr", "form", main);
        assert!(form.contains("\nCREATE Form2 AS RForm\n    Caption = \"Form2\"\n"), "{form}");
        assert!(new_file_text(&m, "Form2.rr", "form", "CREATE Main AS QFORM\nEND CREATE\n").contains("CREATE Form2 AS QFORM"), "the program's own names");
        assert!(new_file_text(&m, "Module1.rr", "module", main).starts_with("' Module1.rr"));
        assert!(new_file_text(&m, "Form3.rr", "formmdi", main).contains("CREATE Form3 AS RFormMDI"));
        // the main program's $INCLUDE, as one patch (an absolute path too)
        assert_eq!(include_edit(&m, main, "Form2.rr"), "1\t0\t1\t0\t$INCLUDE \"Form2.rr\"\\n");
        assert_eq!(include_edit(&m, main, "/work/Demo/forms/About.rr"), "1\t0\t1\t0\t$INCLUDE \"forms/About.rr\"\\n");
        assert_eq!(include_edit(&m, "$INCLUDE \"Form2.rr\"\n", "Form2.rr"), "", "there already");
        // a main file in a folder of its own
        let sub = Model { project: Project::new("Demo", "src/main.rr"), kind: "project", folder: "/work/Demo".into(), ..Model::default() };
        assert_eq!(include_edit(&sub, main, "src/Form2.rr"), "1\t0\t1\t0\t$INCLUDE \"Form2.rr\"\\n");
        assert_eq!(include_edit(&sub, main, "Shared.rr"), "1\t0\t1\t0\t$INCLUDE \"../Shared.rr\"\\n");
        // renamed: the include follows
        let with = "$APPTYPE GUI\n$INCLUDE \"Form2.rr\"\n";
        assert_eq!(include_rename_edit(&m, with, "/work/Demo/Form2.rr", "/work/Demo/About.rr"), "1\t10\t1\t18\tAbout.rr");
        assert_eq!(include_rename_edit(&m, with, "Other.rr", "X.rr"), "");
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
        for want in ["--project", "--name", "Main App", "--app-version", "2.0", "--icon", &format!("{d}/art/app.svg"), "--interp", "--output", &format!("{d}/build"), "--no-keep-rust"] {
            assert!(args.iter().any(|a| a == want), "{want}: {args:?}");
        }
        assert!(!build_args("p1", "native").unwrap().0.contains(&"--interp".to_string()));
        // (the output folder and the Rust source: the project's)
        set("outputfolder", &format!("{d}/dist/mac"));
        assert!(super::set("p1", "keeprust", &Value::Integer(-1)));
        assert_eq!((get("outputfolder"), get("keeprust")), ("dist/mac".into(), "1".into()));
        let args = build_args("p1", "").unwrap().0;
        assert!(args.windows(2).any(|w| w[0] == "--output" && w[1] == format!("{d}/dist/mac")) && args.contains(&"--keep-rust".to_string()), "{args:?}");
        set("outputfolder", "");
        assert!(super::set("p1", "keeprust", &Value::Integer(0)));
        // (no icon file yet: Error says so; RapidR's default draws)
        assert_eq!(call("iconpreview", &[Value::Integer(64)]).to_string_val(), "");
        assert!(!get("error").is_empty());
        set("icon", "");
        let preview = call("iconpreview", &[Value::Integer(64)]).to_string_val();
        assert!(preview.ends_with(".rapidr/icon-preview-64.png") && std::path::Path::new(&preview).is_file(), "{preview}");
        assert_eq!(call("build", &[]).to_i64(), 0, "the quiet host has no rapidr");
        assert!(get("error").contains("desktop"));
        // (nothing to ask on the web: native is not refused for want of Rust)
        assert_eq!(call("rustready", &[]).to_i64(), 1);
        assert_eq!(call("open", &[Value::String(format!("{d}/nope.rr"))]).to_i64(), 0);
        assert!(get("error").contains("no such file"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

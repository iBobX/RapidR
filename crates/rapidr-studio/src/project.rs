//! RPROJECT: a project as RapidR Studio opens it (docs/ide-components.md
//! §3.9), over `rapidr-project`. Its files are read and written through the
//! runtime's file hooks — the disk on the desktop, the page's store on the
//! web (the files the user picked, written back to them).
//!
//! | Member | |
//! |---|---|
//! | `Open(Path)` → True / False | a `.rrproj` (format 2, or the web IDE's v1 JSON, imported: its files written beside it where none is there yet), or a `.bas` / `.rr` / `.inc` file with the files it `$INCLUDE`s (RapidQ's way: no project file) |
//! | `Save([Path])` → True / False | the `.rrproj` (an implicit project becomes one: `<folder>/<Name>.rrproj` or Path), its files' kinds as their code says |
//! | `FollowCode` → text | the files' kinds as their code says now (forms come from the code, several in a file too): the files whose kind changed, a line each (path, kind) |
//! | `New(Template, Name, Folder)` → True / False | `console` or `gui`: `main.rr` and `<Name>.rrproj` written in Folder |
//! | `AddFile(Path [, Kind])`, `RemoveFile(Path)` → True / False | paths relative to the project's folder (or absolute inside it) |
//! | `Close` | no project |
//! | `File(i)`, `FileKind(i)`, `FullPath(i)` | file i (from 0): its project path, kind (`module`, `form`, `include`, `resource`, `asset`, `data`), path to open |
//! | `ImportRapidQ(Source [, Dest])` → the copy's project file | File ▸ Import RapidQ Project or File… (`crate::import`): a copy of a RapidQ program or folder with RapidR's names, its report and project; "" (Error says why) when it can't |
//! | `ImportSummary`, `ImportReport` | the last import's summary line and its report's file |
//! | `ImportMissing` | the last import's includes found nowhere, a line each: `file:line`, the name, what to do (tabs between) |
//! | `IncludeFolders` | Studio's include folders, a line each: where `$INCLUDE` looks after the program's folder, for everything compiled here (and the programs it runs) |
//! | `FileName`, `Folder`, `Kind` (`project`, `file`, `v1`, "" none), `Error` | read-only |
//! | `Name`, `MainFile`, `CompatMode` (`"rapidq"` or "") | read / write |
//! | `Targets` | where the program runs: `desktop`, `web` or `both` (`[build] runs_on`; unset: `both` when it builds a web app too, else `desktop`) — the Problems pane's portability notes follow it |
//! | `RunArgs` | the arguments Run gives the program, one string (`[run] args`) |
//! | `DesignerGrid`, `DesignerSnap`, `DesignerGuides` | the form designer's grid step, snapping to it, alignment guides (`[designer]`; 8, True, True unless the project says) |
//! | `ThemeOf(MainText)` → look, `ThemeEdit(MainText, Look)` → patch | the look the main program names with `$THEME` (RapidR, RapidR Light, RapidR Dark, RapidR High Contrast, Classic), and the one-line edit that makes it name another: Studio's project properties keep the look in the program, not the project file |
//! | `MacKeys` → True / False | the keyboard is a Mac's: Cmd is the shortcut key, Option+arrows move by words (the desktop on macOS, a page on a Mac) |
//! | `FilesIn(Folder)` → text | the text files under Folder, a line each (full paths; hidden folders, `build`, `target` and pictures, sounds, archives left out; at most 2,000): Search's "A folder" scope |
//! | `FileCount` | read-only |
//! | `OnChange` | the project opened, closed, saved, or its files changed |

use std::cell::RefCell;
use std::collections::HashMap;

use rapidr_project::{import_v1, is_v1_json, kind_for_path, kind_for_source, normalize_path, FileKind, Project};
use rapidr_value::i18n::{tr_ui, tr_ui_fill};
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
    /// What the last ExampleToOpen did, by component: "made" (a copy now),
    /// "kept" (the copy that was there), "" (the example itself).
    static EXAMPLE_COPY: RefCell<HashMap<String, &'static str>> = RefCell::new(HashMap::new());
    static PROJECTS: RefCell<HashMap<String, Model>> = RefCell::new(HashMap::new());
    /// The last import's (summary, report file), by component: kept when a
    /// project is opened (the copy is, next).
    static IMPORTED: RefCell<HashMap<String, (String, String, String)>> = RefCell::new(HashMap::new());
}

/// The project's file (relative, as listed) whose code `$INCLUDE`s the main
/// file — "" when none does (the main file is where the program starts).
fn main_included_by(m: &Model) -> String {
    let main = normalize_path(&m.project.main).to_ascii_lowercase();
    if main.is_empty() {
        return String::new();
    }
    for f in &m.project.files {
        if !matches!(f.kind, FileKind::Module | FileKind::Form | FileKind::Include) || normalize_path(&f.path).eq_ignore_ascii_case(&main) {
            continue;
        }
        let Ok(text) = read_text(&join(&m.folder, &f.path)) else { continue };
        let includes = text.lines().filter_map(|l| {
            let l = l.trim_start();
            let rest = l.get(..8).filter(|k| k.eq_ignore_ascii_case("$include")).map(|_| &l[8..])?;
            let q = rest.trim_start().strip_prefix('"')?;
            Some(q.split('"').next().unwrap_or("").to_string())
        });
        let dir = folder_of(&f.path);
        if includes.map(|inc| normalize_path(&join(&dir, &slashes(&inc))).to_ascii_lowercase()).any(|inc| inc == main) {
            return f.path.clone();
        }
    }
    String::new()
}

fn imported(name: &str) -> (String, String, String) {
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
            "filename" => Value::String(m.file_name.clone().into()),
            "folder" => Value::String(m.folder.clone().into()),
            "kind" => Value::String(m.kind.to_string().into()),
            "error" => Value::String(m.error.clone().into()),
            "name" => Value::String(m.project.name.clone().into()),
            "mainfile" => Value::String(m.project.main.clone().into()),
            "filecount" => Value::Integer(if m.kind.is_empty() { 0 } else { m.project.file_count() as i64 }),
            "compatmode" => Value::String(m.project.compat_mode().to_string().into()),
            "icon" => Value::String(m.project.build.icon.clone().into()),
            "appname" => Value::String(m.project.build.app_name.clone().into()),
            "bundleid" => Value::String(m.project.build.bundle_id.clone().into()),
            "version" => Value::String(m.project.build.version.clone().into()),
            "company" => Value::String(m.project.build.company.clone().into()),
            "buildkind" => Value::String(build_kind(&m.project).to_string().into()),
            "targets" => Value::String(m.project.build.runs_on_or_default().to_string().into()),
            "outputfolder" => Value::String(m.project.build.output.clone().into()),
            "keeprust" => flag(m.project.build.keep_rust),
            "runargs" => Value::String(join_args(&m.project.run.args).into()),
            "designergrid" => Value::Integer(i64::from(m.project.designer.grid)),
            "designersnap" => flag(m.project.designer.snap),
            "designerguides" => flag(m.project.designer.guides),
            "building" => flag(crate::build::building(name)),
            "builtpath" => Value::String(m.built.clone().into()),
            "importsummary" => Value::String(imported(name).0.into()),
            "importreport" => Value::String(imported(name).1.into()),
            "importmissing" => Value::String(imported(name).2.into()),
            "includefolders" => Value::String(include_folders().into()),
            "examplecopy" => Value::String(EXAMPLE_COPY.with(|e| e.borrow().get(&name.to_ascii_lowercase()).copied().unwrap_or("")).to_string().into()),
            // (Reveal's: what the system calls its file manager)
            "filemanager" => Value::String(if cfg!(target_arch = "wasm32") { "" } else if cfg!(target_os = "macos") { "Finder" } else if cfg!(windows) { "File Explorer" } else { "Files" }.into()),
            _ => return None,
        })
    })
}

/// A True / False however a program writes it.
fn truth(v: &Value) -> bool {
    v.to_i64() != 0 || matches!(v.to_string_val().trim().to_ascii_lowercase().as_str(), "true" | "yes" | "on")
}

/// The arguments as one string: spaces between them, "quoted" when one has
/// a space (what [`crate::session::split_args`] reads back).
fn join_args(args: &[String]) -> String {
    args.iter().map(|a| if a.is_empty() || a.contains(' ') { format!("\"{a}\"") } else { a.clone() }).collect::<Vec<_>>().join(" ")
}

/// The look a `$THEME` name asks for, as RapidR Studio names the looks.
fn look_label(name: &str) -> &'static str {
    use rapidr_value::theme::{choose, Choice};
    match choose(name) {
        Choice::Theme(t) => match t.name {
            "rapidr light" => "RapidR Light",
            "rapidr dark" => "RapidR Dark",
            "rapidr high contrast" => "RapidR High Contrast",
            _ => "Classic",
        },
        _ => "RapidR",
    }
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
            "runargs" => m.project.run.args = crate::session::split_args(&v.to_string_val()),
            "designergrid" => m.project.designer.grid = v.to_i64().clamp(1, 64) as u32,
            "designersnap" => m.project.designer.snap = truth(v),
            "designerguides" => m.project.designer.guides = truth(v),
            "buildkind" => set_build_kind(&mut m.project, v.to_string_val().trim().eq_ignore_ascii_case("interpreted")),
            "targets" => {
                let t = v.to_string_val().trim().to_ascii_lowercase();
                m.project.build.runs_on = if matches!(t.as_str(), "desktop" | "web" | "both") { t } else { "desktop".into() };
            }
            "includefolders" => set_include_folders(&v.to_string_val()),
            "filename" | "folder" | "kind" | "error" | "filecount" | "building" | "builtpath" | "filemanager" | "importsummary" | "importreport" | "importmissing" | "examplecopy" => {}
            _ => return false,
        }
        true
    })
}

/// Studio's include folders (IncludeFolders), a line each: those set for
/// every program compiled here ([`rapidr_preprocessor::set_include_dirs`]).
fn include_folders() -> String {
    INCLUDE_FOLDERS.with(|f| f.borrow().clone())
}

thread_local! {
    static INCLUDE_FOLDERS: RefCell<String> = const { RefCell::new(String::new()) };
}

/// `RAPIDR_INCLUDE_PATH` for a program Studio starts (Run, Build): every
/// include folder this Studio compiles with — IncludeFolders, then its own
/// `RAPIDR_INCLUDE_PATH`'s. `None` when there are none.
pub fn include_path_env() -> Option<String> {
    let dirs = rapidr_preprocessor::configured_include_dirs();
    if dirs.is_empty() {
        return None;
    }
    rapidr_preprocessor::include_path_value(&dirs).map(|v| v.to_string_lossy().into_owned())
}

/// IncludeFolders set: a folder a line (blank lines left out), for every
/// program compiled from now on — Open, the language service, Run and
/// Build (the programs Studio starts get them in `RAPIDR_INCLUDE_PATH`),
/// ImportRapidQ.
fn set_include_folders(text: &str) {
    let dirs: Vec<std::path::PathBuf> = text.lines().map(str::trim).filter(|l| !l.is_empty()).map(std::path::PathBuf::from).collect();
    let shown = dirs.iter().map(|d| d.to_string_lossy().into_owned()).collect::<Vec<_>>().join("\n");
    INCLUDE_FOLDERS.with(|f| *f.borrow_mut() = shown);
    rapidr_preprocessor::set_include_dirs(dirs);
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
/// loads …), paths relative to its folder (`art/logo.svg` too, never one
/// outside it); and its translations when it translates (`lang/*.po`, the
/// names `list` gives for a folder). The compiler builds the `$RESOURCE`s
/// in; the program reads the others as files beside it.
pub fn program_files(program: &str, list: &dyn Fn(&str) -> Vec<String>) -> Result<ProgramFiles, String> {
    let program = slashes(program);
    let folder = folder_of(&program);
    let main = program.rsplit('/').next().unwrap_or(&program).to_string();
    let main_text = read_text(&program)?;
    let resolve = |rel: &str| -> Option<(String, String)> {
        if rel.eq_ignore_ascii_case(&main) {
            return Some((main.clone(), main_text.clone()));
        }
        read_project_file(&folder, rel)
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
    // (its translations, as a build takes them: rapidr_project::ship)
    let lang = join(&folder, rapidr_project::ship::LANG_FOLDER);
    let mut names: Vec<String> = list(&lang).into_iter().filter(|n| rapidr_project::ship::is_catalog(n)).collect();
    names.sort();
    let translates = !names.is_empty() || sources.iter().any(|(_, t)| rapidr_project::ship::uses_translations(t));
    let catalogs = translates.then(|| {
        let mut found = Vec::new();
        for name in names {
            if let Ok(bytes) = rapidr_value::objects::read_file(&join(&lang, &name)) {
                if bytes.len() <= room {
                    room -= bytes.len();
                    found.push((name, bytes));
                }
            }
        }
        found
    });
    Ok(ProgramFiles { main, sources, data, catalogs })
}

/// [`program_files`]' answer.
pub struct ProgramFiles {
    /// The main file's name.
    pub main: String,
    /// (path, text) of each source.
    pub sources: Vec<(String, String)>,
    /// (path, bytes) of each data file.
    pub data: Vec<(String, Vec<u8>)>,
    /// (file name, bytes) of each translation catalog (`lang/es.po`) when
    /// the program translates; `None` when it doesn't.
    pub catalogs: Option<Vec<(String, Vec<u8>)>>,
}

fn exists(path: &str) -> bool {
    rapidr_value::objects::read_file(path).is_ok()
}

/// Project file `rel` of `folder` — its names in any case, as `$INCLUDE`
/// finds it (`Inc\MyLib.INC` → `inc/mylib.inc`) — : its project path as
/// the files name it, and its text.
fn read_project_file(folder: &str, rel: &str) -> Option<(String, String)> {
    if let Ok(t) = read_text(&join(folder, rel)) {
        return Some((rel.to_string(), t));
    }
    let actual = rapidr_project::actual_path(folder, rel)?;
    read_text(&join(folder, &actual)).ok().map(|t| (actual, t))
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
                return Err(tr_ui_fill("{path}: not a RapidR project", &[("path", &path)]));
            } else {
                (Project::from_toml(&text).map_err(|e| e.to_string())?, "project", path.clone())
            }
        }
        // (RapidR's and RapidQ's sources: rapidr_preprocessor::SOURCE_EXTENSIONS)
        "bas" | "rr" | "inc" | "rqw" | "rqb" | "rq" => {
            let main = path.rsplit('/').next().unwrap_or(&path).to_string();
            let resolve = |rel: &str| -> Option<(String, String)> { read_project_file(&folder, rel) };
            if !exists(&path) {
                return Err(tr_ui_fill("{path}: no such file", &[("path", &path)]));
            }
            (rapidr_project::implicit_from_resolver(&main, &resolve), "file", String::new())
        }
        _ => return Err(tr_ui_fill("{path}: not a project or a source file", &[("path", &path)])),
    };
    *m = Model { project, kind, file_name, folder, ..Model::default() };
    Ok(())
}

/// The project's files' kinds as their code on the disk (the page's store
/// on the web) says now — forms come from the code: the files whose kind
/// changed, a line each, its path and new kind tab-separated.
fn follow_code(m: &mut Model) -> String {
    let folder = m.folder.clone();
    m.project.follow_code(&|rel: &str| read_text(&join(&folder, rel)).ok()).iter().map(|(p, k)| format!("{p}\t{}\n", kind_name(*k))).collect()
}

fn save(m: &mut Model, to: &str) -> Result<(), String> {
    if m.kind.is_empty() {
        return Err(tr_ui("no project is open"));
    }
    // (the kinds written are the code's: S1)
    follow_code(m);
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
            return Err(tr_ui("no project is open"));
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
            return Err(tr_ui("no project is open"));
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

/// ExampleToOpen(Path, Examples): the file Studio opens for `path` — when
/// it's one of RapidR's examples (in folder `examples`) installed where it
/// can't be written, its copy in Documents/RapidR/Examples/<name>/ (made
/// now, "made", or the one there, "kept": never overwritten); else `path`
/// itself (""). The web's store keeps what the user saves over the file it
/// opened: there, always the example itself.
fn example_to_open(path: &str, examples: &str) -> Result<(String, &'static str), String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use rapidr_project::program_files::{self, ExampleOpen};
        use std::path::Path;
        let p = Path::new(path);
        if path.is_empty() || examples.is_empty() || !program_files::is_inside(p, Path::new(examples)) || !p.is_file() {
            return Ok((path.to_string(), ""));
        }
        // (an install's home has its release.toml beside examples/: its
        // examples are always copied — a writable app bundle's too)
        let installed = Path::new(examples).parent().is_some_and(|home| home.join("release.toml").is_file());
        let (file, what) = program_files::example_to_open(p, installed)?;
        let what = match what {
            ExampleOpen::InPlace => return Ok((path.to_string(), "")),
            ExampleOpen::Copied => "made",
            ExampleOpen::Existing => "kept",
        };
        Ok((slashes(&file.to_string_lossy()), what))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = examples;
        Ok((path.to_string(), ""))
    }
}

/// ProjectsFolder(Home): where New Project puts projects by default —
/// `<home>projects` while Studio runs from a folder of the user's (the
/// repository), `Documents/RapidR/Projects` when Studio is installed (a
/// home with its release.toml: the macOS app, /usr/lib/rapidr, Program
/// Files) or its home can't be written: never inside RapidR itself.
fn projects_folder(home: &str) -> String {
    let own = format!("{home}projects");
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::path::Path;
        let h = Path::new(home);
        let installed = h.join("release.toml").is_file();
        let writable = !home.is_empty() && h.is_dir() && {
            let probe = h.join(".rapidr-write-probe");
            let ok = std::fs::write(&probe, b"").is_ok();
            let _ = std::fs::remove_file(&probe);
            ok
        };
        if installed || !writable {
            if let Some(d) = rapidr_project::program_files::documents_folder() {
                return slashes(&d.join("RapidR").join("Projects").to_string_lossy());
            }
        }
    }
    own
}

/// ExampleOf(Path, Examples): the example (in folder `examples`) that
/// `path` is the user's copy of; "" for any other file.
fn example_of(path: &str, examples: &str) -> String {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::path::Path;
        if path.is_empty() || examples.is_empty() {
            return String::new();
        }
        rapidr_project::program_files::example_of_copy(Path::new(path), Path::new(examples)).map(|p| slashes(&p.to_string_lossy())).unwrap_or_default()
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (path, examples);
        String::new()
    }
}

/// CopyExampleAgain(Path, Examples): the copy `path` (or the example
/// `path` itself) made again from the example as RapidR has it — its
/// program, includes and data over the copy's; files the user added stay.
/// The copy's main file.
fn copy_example_again(path: &str, examples: &str) -> Result<String, String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::path::Path;
        let original = match example_of(path, examples) {
            o if !o.is_empty() => o,
            _ if !path.is_empty() && rapidr_project::program_files::is_inside(Path::new(path), Path::new(examples)) => path.to_string(),
            _ => return Err(tr_ui_fill("{path} isn't a copy of one of RapidR's examples", &[("path", &path)])),
        };
        rapidr_project::program_files::copy_example_again(Path::new(&original)).map(|p| slashes(&p.to_string_lossy()))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (path, examples);
        Err(tr_ui("the web keeps the examples you change in the page's store"))
    }
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
                    with(name, |m| m.error = tr_ui_fill("{folder}: no RapidR project or source in it", &[("folder", &folder)]));
                    flag(false)
                }
            }
        }
        "save" => changing(&|m| save(m, &s(0))),
        // FollowCode → the files whose kind changed ("" none): the kinds
        // as the code says now, not saved yet (Save writes them)
        "followcode" => {
            let changed = with(name, |m| if m.kind.is_empty() { String::new() } else { follow_code(m) });
            if !changed.is_empty() {
                host.fire(name, "onchange", &[]);
            }
            Value::String(changed.into())
        }
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
                return Err(tr_ui("no project is open"));
            }
            let rel = project_path(&m.folder, &s(0));
            // (no kind given: its code says — a file that creates a form is
            // a form file, whoever wrote it: S1)
            let kind = kind_parse(&s(1)).unwrap_or_else(|| read_text(&join(&m.folder, &rel)).map_or_else(|_| kind_for_path(&rel), |t| kind_for_source(&rel, &t)));
            if m.project.add_file(&rel, kind) { Ok(()) } else { Err(tr_ui_fill("{file} is in the project already", &[("file", &rel)])) }
        }),
        "removefile" => changing(&|m| {
            let rel = project_path(&m.folder, &s(0));
            if m.project.remove_file(&rel) { Ok(()) } else { Err(tr_ui_fill("{file} isn't in the project", &[("file", &rel)])) }
        }),
        "file" => Value::String(with(name, |m| m.project.file(i.max(0) as usize).filter(|_| i >= 0).map(|f| f.path.clone()).unwrap_or_default()).into()),
        "filekind" => Value::String(with(name, |m| m.project.file(i.max(0) as usize).filter(|_| i >= 0).map(|f| kind_name(f.kind).to_string()).unwrap_or_default()).into()),
        "build" => {
            let kind = s(0);
            let started = if kind.eq_ignore_ascii_case("web") && host.rapidr().is_none() {
                // (the web page builds the web app itself)
                web_build(host, name)
            } else {
                build_args(name, &kind).and_then(|(args, cwd)| {
                    let rapidr = host.rapidr().ok_or_else(|| tr_ui("Build Native App and Build Interpreted App make apps in RapidR Studio on the desktop (macOS, Windows, Linux); on the web, Build Web App makes a web app and Run shows the program"))?;
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
                .ok_or_else(|| tr_ui("Installing Rust is the desktop's: run `rapidr setup` on the computer"))
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
            let r = if path.is_empty() { Err(tr_ui("nothing built yet")) } else { host.reveal(&path) };
            let ok = r.is_ok();
            with(name, |m| m.error = r.err().unwrap_or_default());
            flag(ok)
        }
        "iconpreview" => Value::String(icon_preview(name, int_arg(args, 0, 128).clamp(16, 1024) as u32).into()),
        // (an example installed where it can't be changed opens as the
        // user's copy: rapidr_project::program_files)
        "exampletoopen" => {
            let r = example_to_open(&s(0), &s(1));
            let (path, what) = match r {
                Ok(x) => x,
                Err(e) => {
                    with(name, |m| m.error = e);
                    (String::new(), "")
                }
            };
            EXAMPLE_COPY.with(|e| e.borrow_mut().insert(name.to_ascii_lowercase(), what));
            Value::String(path.into())
        }
        "projectsfolder" => Value::String(projects_folder(&s(0)).into()),
        // (Studio keeps your work: crate::recovery — on the web it asks the
        // page's store whether a file is there before it reads it)
        m if cfg!(target_arch = "wasm32") && (m.starts_with("recovery") || m.starts_with("log") || m.ends_with("newfile")) && !crate::recovery::store_known() => {
            crate::recovery::set_store_has(Box::new(move |p: &str| {
                let p = slashes(p);
                let (folder, file) = p.rsplit_once('/').unwrap_or(("", p.as_str()));
                host.list_files(folder).iter().any(|f| f == file)
            }));
            return call(host, name, method, args);
        }
        "studiofolder" => Value::String(crate::recovery::studio_folder().into()),
        "recoverykeep" => {
            let r = crate::recovery::keep(&s(0), &s(1), &s(2), &s(3));
            with(name, |m| m.error = r.clone().err().unwrap_or_default());
            flag(r.is_ok())
        }
        "recoveryforget" => {
            crate::recovery::forget(&s(0));
            Value::Null
        }
        "recoveryend" => {
            crate::recovery::end();
            Value::Null
        }
        "recoveryfound" => Value::String(crate::recovery::found().into()),
        "recoverytext" => Value::String(crate::recovery::text(&s(0), &s(1)).into()),
        "recoverydiscard" => {
            crate::recovery::discard(&s(0));
            Value::Null
        }
        "logerror" => {
            rapidr_crash::note(&s(0));
            flag(crate::recovery::log(&s(0)).is_ok())
        }
        // (RapidR's crash reports: crate::crash — on this computer only)
        "crashpending" => Value::String(crate::crash::pending().into()),
        "crashtext" => Value::String(crate::crash::text(&s(0)).into()),
        "crashpath" => Value::String(crate::crash::path(&s(0)).into()),
        "crashfield" => Value::String(crate::crash::field(&s(0), &s(1)).into()),
        "crashseen" => {
            rapidr_crash::mark_seen(&s(0));
            Value::Null
        }
        "crashsave" => {
            let r = crate::crash::save(&s(0), &s(1));
            with(name, |m| m.error = r.clone().err().unwrap_or_default());
            flag(r.is_ok())
        }
        "crashissueurl" => Value::String(crate::crash::issue(&s(0)).0.into()),
        "crashissuewhole" => flag(crate::crash::issue(&s(0)).1),
        "crashmark" => Value::String(crate::crash::mark().into()),
        "crashforprogram" => Value::String(crate::crash::for_program(&s(0), &s(1)).into()),
        "crashnote" => {
            rapidr_crash::note(&s(0));
            Value::Null
        }
        "crashcall" => {
            rapidr_crash::call(&s(0));
            Value::Null
        }
        "crashdoing" => {
            rapidr_crash::doing(&s(0));
            Value::Null
        }
        "crashwatch" => flag(crate::crash::watch(args.first().map(Value::to_i64).unwrap_or(5), &s(1), &s(2), &s(3), &s(4), &s(5), &s(6))),
        "crashtest" => flag(crate::crash::test(&s(0))),
        "crashfolder" => Value::String(crate::crash::folder().into()),
        "crashlist" => Value::String(rapidr_crash::list().iter().map(|n| format!("{n}\n")).collect::<String>().into()),
        "logfile" => Value::String(crate::recovery::log_file().into()),
        // (the main file is a module another file of the project
        // includes: that file is where the program likely starts — Robert's
        // ShopInvoices, main = lang.rr while main.rr includes it)
        "mainincludedby" => Value::String(with(name, |m| main_included_by(m)).into()),
        // (RAI's Apply, all or nothing: crate::recovery)
        "applybegin" => {
            crate::recovery::apply_begin();
            Value::Null
        }
        "canwritenewfile" => {
            let r = crate::recovery::can_write_new(&s(0));
            with(name, |m| m.error = r.clone().err().unwrap_or_default());
            flag(r.is_ok())
        }
        "writenewfile" => {
            let r = crate::recovery::write_new(&s(0), &s(1));
            with(name, |m| m.error = r.clone().err().unwrap_or_default());
            flag(r.is_ok())
        }
        "applyundo" => Value::String(crate::recovery::apply_undo().into()),
        "exampleof" => Value::String(example_of(&s(0), &s(1)).into()),
        "copyexampleagain" => {
            let r = copy_example_again(&s(0), &s(1));
            with(name, |m| m.error = r.as_ref().err().cloned().unwrap_or_default());
            Value::String(r.unwrap_or_default().into())
        }
        // (File > Import RapidQ Project or File: a copy, never the original)
        "importrapidq" => {
            let r = crate::import::import(host, &s(0), &s(1));
            with(name, |m| m.error = r.as_ref().err().cloned().unwrap_or_default());
            match r {
                Ok(i) => {
                    IMPORTED.with(|x| x.borrow_mut().insert(name.to_ascii_lowercase(), (i.summary.clone(), i.report.clone(), i.missing.clone())));
                    Value::String(i.project.into())
                }
                Err(_) => Value::String(String::new().into()),
            }
        }
        // (Studio's keys) MacKeys: a Mac's keyboard (Option moves by words)
        "mackeys" => flag(host.mac_keys()),
        // (Search's "A folder" scope) FilesIn(Folder): its text files
        "filesin" => Value::String(files_in(host, &s(0)).iter().map(|p| format!("{p}\n")).collect()),
        // (Find in Files) Find(Pattern, Options, Text): the matches of one
        // text, a line each; Replace(Pattern, Options, Text, With): the text
        // replaced; FileText(Path): a file's text (the disk's, the web's store)
        "find" | "replace" => {
            let r = if method == "find" { crate::find::find(&s(2), &s(0), &s(1)) } else { crate::find::replace(&s(2), &s(0), &s(1), &s(3)).map(|(t, _)| t) };
            let ok = r.is_ok();
            with(name, |m| m.error = r.as_ref().err().cloned().unwrap_or_default());
            Value::String(if ok { r.unwrap().into() } else if method == "find" { String::new().into() } else { s(2).into() })
        }
        // (files changed outside Studio: crate::external) Watch(Path): the
        // disk's file as Studio's own now (read or written); Unwatch(Path),
        // UnwatchAll; ExternalChanges: what changed outside since the last
        // look, a line each ("changed" / "deleted" / "back", a tab, the
        // path), each once; ChangedOnDisk(Path): the disk differs from what
        // Studio last read or wrote (Save asks first)
        "watch" => flag(crate::external::with(|w| w.watch(&s(0)))),
        "unwatch" => {
            crate::external::with(|w| w.unwatch(&s(0)));
            Value::Null
        }
        "unwatchall" => {
            crate::external::with(crate::external::Watcher::clear);
            Value::Null
        }
        "externalchanges" => Value::String(crate::external::changes_text(&crate::external::with(crate::external::Watcher::check)).into()),
        "changedondisk" => flag(crate::external::with(|w| w.changed_since_base(&s(0)))),
        "missingondisk" => flag(crate::external::with(|w| w.missing(&s(0)))),
        "filetext" => {
            let r = crate::read_text(&s(0));
            with(name, |m| m.error = r.as_ref().err().cloned().unwrap_or_default());
            Value::String(r.unwrap_or_default().into())
        }
        "fullpath" => Value::String(with(name, |m| m.project.file(i.max(0) as usize).filter(|_| i >= 0).map(|f| join(&m.folder, &f.path)).unwrap_or_default()).into()),
        // (Project > Add Form / Add Module) NewFileText(Path, Kind,
        // MainText): a new file's text in the program's names;
        // IncludeEdit(MainText, Path): the `$INCLUDE` the main program needs
        // for it, as one patch for RCODEEDITOR.ApplyPatches ("" when it has it)
        "newfiletext" => Value::String(with(name, |m| new_file_text(m, &s(0), &s(1), &s(2))).into()),
        "includeedit" => Value::String(with(name, |m| include_edit(m, &s(0), &s(1))).into()),
        // IncludeRenameEdit(MainText, OldPath, NewPath): the main program's
        // $INCLUDE of a file renamed or moved, following it ("" none)
        "includerenameedit" => Value::String(with(name, |m| include_rename_edit(m, &s(0), &s(1), &s(2))).into()),
        // ThemeOf(MainText): the look the main program names; ThemeEdit(
        // MainText, Look): the edit that makes it name Look ("" none)
        "themeof" => Value::String(look_label(&rapidr_project::forms::theme_of(&s(0))).to_string().into()),
        "themeedit" => {
            let (text, want) = (s(0), look_label(&s(1)));
            // (the line says the look as RapidR spells it; RapidR's default is no line)
            if look_label(&rapidr_project::forms::theme_of(&text)) == want {
                return Some(Value::String(String::new().into()));
            }
            let named = if want == "RapidR" { "" } else { want };
            Value::String(match rapidr_project::forms::theme_edit(&text, named) {
                Some((a, b, c, d, t)) => format!("{a}	{b}	{c}	{d}	{}", patch_escaped(&t)).into(),
                None => String::new().into(),
            })
        }
        // IncludeRemoveEdit(MainText, Path): its $INCLUDE line taken out ("" none)
        "includeremoveedit" => Value::String(with(name, |m| match rapidr_project::forms::include_line(&s(0), &from_main(m, &s(1))) {
            Some(line) => format!("{line}\t0\t{}\t0\t", line + 1),
            None => String::new(),
        }).into()),
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

/// The most files Search's "A folder" scope reads.
const FOLDER_FILES: usize = 2000;

/// Whether a file under a folder is one Search reads: not in a hidden
/// folder or a build's output, not a picture, sound, video, font, archive
/// or program.
fn searchable(rel: &str) -> bool {
    let parts: Vec<&str> = rel.split('/').collect();
    if parts.iter().any(|p| p.starts_with('.')) || parts[..parts.len().saturating_sub(1)].iter().any(|p| matches!(p.to_ascii_lowercase().as_str(), "build" | "target" | "node_modules")) {
        return false;
    }
    let ext = rel.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    !matches!(
        ext.as_str(),
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" | "icns" | "webp" | "tif" | "tiff" | "wav" | "mp3" | "ogg" | "flac" | "mid" | "midi" | "mp4" | "mov" | "avi" | "mkv" | "webm"
            | "ttf" | "otf" | "woff" | "woff2" | "zip" | "gz" | "tar" | "7z" | "rar" | "exe" | "dll" | "dylib" | "so" | "app" | "bin" | "pdf" | "db" | "sqlite" | "parquet" | "arrow" | "xlsx" | "rrb" | "class" | "o" | "a"
    )
}

/// Search's "A folder" scope: the text files under `folder` (full paths,
/// sorted; at most [`FOLDER_FILES`]) — the disk's on the desktop, the
/// page's store on the web.
fn files_in<H: Host>(host: H, folder: &str) -> Vec<String> {
    let folder = slashes(folder).trim_end_matches('/').to_string();
    if folder.is_empty() {
        return Vec::new();
    }
    let mut rels: Vec<String> = Vec::new();
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = host;
        let mut todo = vec![std::path::PathBuf::from(&folder)];
        while let Some(dir) = todo.pop() {
            let Ok(rd) = std::fs::read_dir(&dir) else { continue };
            for e in rd.flatten() {
                let p = e.path();
                let name = e.file_name().to_string_lossy().to_string();
                if name.starts_with('.') {
                    continue;
                }
                if p.is_dir() {
                    if !matches!(name.to_ascii_lowercase().as_str(), "build" | "target" | "node_modules") {
                        todo.push(p);
                    }
                } else if let Some(rel) = slashes(&p.to_string_lossy()).strip_prefix(&format!("{folder}/")) {
                    rels.push(rel.to_string());
                }
            }
            if rels.len() > FOLDER_FILES * 4 {
                break;
            }
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        rels = host.list_tree(&folder);
    }
    rels.retain(|r| searchable(r));
    rels.sort_by_key(|r| r.to_ascii_lowercase());
    rels.truncate(FOLDER_FILES);
    rels.into_iter().map(|r| format!("{folder}/{r}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The looks `$THEME` names, as Studio's project properties name them.
    #[test]
    fn the_looks_are_named_as_studio_names_them() {
        for (written, label) in [("", "RapidR"), ("rapidr", "RapidR"), ("RapidR Light", "RapidR Light"), ("modern", "RapidR Light"), ("Dark", "RapidR Dark"), ("rapidr high contrast", "RapidR High Contrast"), ("Win98", "Classic"), ("nonsense", "RapidR")] {
            assert_eq!(look_label(written), label, "{written:?}");
        }
        assert_eq!(join_args(&["--dry".into(), "a b".into()]), "--dry \"a b\"");
        assert_eq!(crate::session::split_args(&join_args(&["--dry".into(), "a b".into()])), ["--dry", "a b"]);
    }

    /// New Project's default folder is never inside an installed RapidR
    /// (the final audit: it was the app bundle's, /usr/lib/rapidr's).
    #[test]
    fn projects_go_to_documents_when_installed() {
        let base = std::env::temp_dir().join(format!("rapidr-projects-folder-{}", std::process::id()));
        let (home, docs) = (base.join("home"), base.join("docs"));
        std::fs::create_dir_all(&home).unwrap();
        // (this test's alone: no other test here reads RAPIDR_DOCUMENTS)
        std::env::set_var("RAPIDR_DOCUMENTS", &docs);
        let h = format!("{}/", slashes(&home.to_string_lossy()));
        assert_eq!(projects_folder(&h), format!("{h}projects"), "a folder of the user's keeps its own projects folder");
        std::fs::write(home.join("release.toml"), "").unwrap();
        assert_eq!(projects_folder(&h), slashes(&docs.join("RapidR").join("Projects").to_string_lossy()), "an install's go to Documents");
        std::env::remove_var("RAPIDR_DOCUMENTS");
        let _ = std::fs::remove_dir_all(&base);
    }

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
            assert_eq!(call(Rapidr(rapidr), "pn", "open", &[Value::String(format!("{folder}/Fresh.rrproj").into())]).unwrap().to_i64(), 1);
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
        assert_eq!(call("open", &[Value::String(format!("{d}/main.bas").into())]).to_i64(), 1);
        assert_eq!((get("kind"), get("filecount"), get("mainfile")), ("file".into(), "2".into(), "main.bas".into()));
        assert_eq!(call("file", &[Value::Integer(1)]).to_string_val(), "lib.inc");
        assert_eq!(call("filekind", &[Value::Integer(1)]).to_string_val(), "include");
        assert_eq!(call("fullpath", &[Value::Integer(0)]).to_string_val(), format!("{d}/main.bas"));
        assert_eq!(call("save", &[]).to_i64(), 1);
        assert!(dir.join("main.rrproj").is_file());
        assert_eq!(call("open", &[Value::String(format!("{d}/main.rrproj").into())]).to_i64(), 1);
        assert_eq!(get("kind"), "project");
        assert_eq!(call("addfile", &[Value::String(format!("{d}/data.csv").into())]).to_i64(), 1);
        assert_eq!(call("filekind", &[Value::Integer(2)]).to_string_val(), "data");
        assert_eq!(call("addfile", &[Value::String("data.csv".into())]).to_i64(), 0);
        assert!(get("error").contains("already"));
        let new_dir = format!("{d}/fresh");
        std::fs::create_dir_all(&new_dir).unwrap();
        assert_eq!(call("new", &[Value::String("gui".into()), Value::String("Fresh".into()), Value::String(new_dir.clone().into())]).to_i64(), 1);
        assert!(std::path::Path::new(&new_dir).join("main.rr").is_file() && std::path::Path::new(&new_dir).join("Fresh.rrproj").is_file());
        // the app's settings, the build's arguments, the icon's preview
        assert_eq!(call("open", &[Value::String(format!("{d}/main.rrproj").into())]).to_i64(), 1);
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
        assert_eq!(call("open", &[Value::String(format!("{d}/nope.rr").into())]).to_i64(), 0);
        assert!(get("error").contains("no such file"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    // (S1, Robert's 7-file program) The project file says main.rr is the
    // form and ui.rr a module; the code says ui.rr makes three forms and
    // main.rr none. FollowCode says what changes; Save writes the code's
    // kinds; a form file added without a kind (RAI's new file) is a form.
    #[test]
    fn the_project_file_follows_the_code() {
        let dir = std::env::temp_dir().join(format!("rapidr-studio-follow-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let d = slashes(&dir.to_string_lossy());
        let mut p = Project::new("Web", "main.rr");
        p.files.clear();
        p.add_file("main.rr", FileKind::Form);
        p.add_file("ui.rr", FileKind::Module);
        std::fs::write(dir.join("Web.rrproj"), p.to_toml()).unwrap();
        std::fs::write(dir.join("main.rr"), "$INCLUDE \"ui.rr\"\nForm.ShowModal\n").unwrap();
        std::fs::write(dir.join("ui.rr"), "CREATE Form AS RForm\nEND CREATE\nCREATE NewForm AS RForm\nEND CREATE\nCREATE CfgForm AS RForm\nEND CREATE\n").unwrap();
        let call = |m: &str, args: &[Value]| call(Quiet, "pf", m, args).unwrap();
        assert_eq!(call("open", &[Value::String(format!("{d}/Web.rrproj").into())]).to_i64(), 1);
        // (as the file says, until the code is followed)
        assert_eq!(call("filekind", &[Value::Integer(0)]).to_string_val(), "form");
        assert_eq!(call("followcode", &[]).to_string_val(), "main.rr\tmodule\nui.rr\tform\n");
        assert_eq!(call("followcode", &[]).to_string_val(), "");
        assert_eq!(call("save", &[]).to_i64(), 1);
        let saved = std::fs::read_to_string(dir.join("Web.rrproj")).unwrap();
        assert!(saved.contains("path = \"main.rr\"\nkind = \"module\"") && saved.contains("path = \"ui.rr\"\nkind = \"form\""), "{saved}");
        // (a form written by the assistant, added with no kind)
        std::fs::write(dir.join("about.rr"), "CREATE About AS RForm\nEND CREATE\n").unwrap();
        assert_eq!(call("addfile", &[Value::String(format!("{d}/about.rr").into())]).to_i64(), 1);
        assert_eq!(call("filekind", &[Value::Integer(2)]).to_string_val(), "form");
        // (the form taken out of the code, saved: a module again when the project is saved)
        std::fs::write(dir.join("about.rr"), "SUB About\nEND SUB\n").unwrap();
        assert_eq!(call("save", &[]).to_i64(), 1);
        assert!(std::fs::read_to_string(dir.join("Web.rrproj")).unwrap().contains("path = \"about.rr\"\nkind = \"module\""));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

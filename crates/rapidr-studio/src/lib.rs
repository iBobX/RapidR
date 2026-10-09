//! RapidR Studio's services as public non-visual components
//! (docs/ide-plan.md I1, docs/ide-components.md §3.9) — one implementation
//! for every runtime:
//!
//! - [`project`]: **RPROJECT**, a project (`.rrproj` format 2, a web IDE v1
//!   project, or a plain `.bas` / `.rr` file and what it includes) over
//!   `rapidr-project`;
//! - [`langsvc`]: **RLANGUAGESERVICE**, the language service
//!   (`rapidr-langsvc`): a file's outline and its diagnostics, as text a
//!   program reads line by line;
//! - [`session`]: **RPROGRAMSESSION**, a run of the program under
//!   development (`rapidr-session`'s client): its own process on the
//!   desktop, a sandboxed frame on the web.
//!
//! A runtime gives them its [`Host`]: files go through `rapidr-value`'s
//! file hooks (the disk, the web's store), events through the runtime's
//! queue, and [`Host::launch`] starts the program under development.

use rapidr_value::Value;

pub mod build;
pub mod channel;
pub mod design;
pub mod find;
pub mod help;
pub mod import;
pub mod langsvc;
pub mod project;
pub mod session;

pub use rapidr_session::Transport;

/// What a runtime provides to Studio's components.
pub trait Host: Copy + 'static {
    /// Fires component `name`'s `event` (its handler queued, as the
    /// runtime's other events).
    fn fire(self, name: &str, event: &str, args: &[Value]);
    /// Starts `program` (a source file, as saved) for a session, with its
    /// arguments, drawn in `theme` when one is named (a program that names
    /// none takes it, as from `RAPIDR_THEME`; "": the default): the
    /// program's end of the session protocol, waiting for `start`.
    fn launch(self, program: &str, args: &[String], theme: &str) -> Result<Box<dyn Transport>, String>;
    /// The files directly in `folder` (their names; RPROJECT.OpenFolder):
    /// the disk's on the desktop, the page's store on the web.
    fn list_files(self, folder: &str) -> Vec<String>;
    /// Every file under `folder`, at any depth (paths relative to it, `/`
    /// separated): the page's store on the web (RPROJECT.ImportRapidQ reads
    /// a picked folder from it); the desktop reads the disk itself.
    fn list_tree(self, _folder: &str) -> Vec<String> {
        Vec::new()
    }
    /// The `rapidr` executable RPROJECT.Build runs (`rapidr build`); `None`
    /// where there is none to run (the web).
    fn rapidr(self) -> Option<std::path::PathBuf> {
        None
    }
    /// Shows `path` selected in the system's file manager (Finder,
    /// Explorer, the Linux one): RPROJECT.Reveal.
    fn reveal(self, _path: &str) -> Result<(), String> {
        Err("there is no file manager here".into())
    }
}

/// Whether `type_name` (a canonical R name) is one of Studio's components.
pub fn is_studio_type(type_name: &str) -> bool {
    matches!(type_name.to_ascii_uppercase().as_str(), "RPROJECT" | "RLANGUAGESERVICE" | "RPROGRAMSESSION")
}

/// A property of component `name` of `type_name` (`None`: not one of its
/// own; the runtime keeps it).
pub fn get(type_name: &str, name: &str, prop: &str) -> Option<Value> {
    let prop = prop.to_ascii_lowercase();
    match type_name.to_ascii_uppercase().as_str() {
        "RPROJECT" => project::get(name, &prop),
        "RLANGUAGESERVICE" => langsvc::get(name, &prop),
        "RPROGRAMSESSION" => session::get(name, &prop),
        _ => None,
    }
}

/// Sets a property; whether it was one of the component's own.
pub fn set<H: Host>(host: H, type_name: &str, name: &str, prop: &str, value: &Value) -> bool {
    let prop = prop.to_ascii_lowercase();
    match type_name.to_ascii_uppercase().as_str() {
        "RPROJECT" => project::set(name, &prop, value),
        "RLANGUAGESERVICE" => langsvc::set(name, &prop, value),
        "RPROGRAMSESSION" => session::set(host, name, &prop, value),
        _ => false,
    }
}

/// A method (`None`: not one of the component's own).
pub fn call<H: Host>(host: H, type_name: &str, name: &str, method: &str, args: &[Value]) -> Option<Value> {
    let method = method.to_ascii_lowercase();
    match type_name.to_ascii_uppercase().as_str() {
        "RPROJECT" => project::call(host, name, &method, args),
        "RLANGUAGESERVICE" => langsvc::call(name, &method, args),
        "RPROGRAMSESSION" => session::call(host, name, &method, args),
        _ => None,
    }
}

/// Each running session's news delivered as its events (OnOutput, OnStopped,
/// OnExit …): the runtime calls it between its host's turns (the desktop)
/// or when the program's frame sent something (the web). Whether a session
/// is running (the desktop then calls again soon).
pub fn poll<H: Host>(host: H) -> bool {
    let sessions = session::poll(host);
    let builds = build::poll(host, project::set_built);
    sessions || builds
}

// ---- helpers ---------------------------------------------------------------

pub(crate) fn text_arg(args: &[Value], i: usize) -> String {
    args.get(i).map(Value::to_string_val).unwrap_or_default()
}

pub(crate) fn int_arg(args: &[Value], i: usize, default: i64) -> i64 {
    args.get(i).map_or(default, Value::to_i64)
}

/// True / False as RapidQ's components read them (1 / 0: RAPIDQ.INC's
/// True, docs/rapidq-ground-truth.md "Values at creation").
pub(crate) fn flag(b: bool) -> Value {
    Value::Integer(i64::from(b))
}

/// A path with `/` separators.
pub(crate) fn slashes(path: &str) -> String {
    path.replace('\\', "/")
}

/// A path's folder ("" for a bare name), without the last `/`.
pub(crate) fn folder_of(path: &str) -> String {
    let p = slashes(path);
    match p.rfind('/') {
        Some(0) => "/".to_string(),
        Some(i) => p[..i].to_string(),
        None => String::new(),
    }
}

/// `rel` under `folder`.
pub(crate) fn join(folder: &str, rel: &str) -> String {
    let rel = slashes(rel);
    if folder.is_empty() || rel.starts_with('/') || rel.get(1..3) == Some(":/") {
        rel
    } else if folder.ends_with('/') {
        format!("{folder}{rel}")
    } else {
        format!("{folder}/{rel}")
    }
}

/// A file's text through the runtime's file hooks: UTF-8 (its BOM off) when
/// it is UTF-8, else a byte a character (Windows' ANSI, as RapidQ wrote it).
pub(crate) fn read_text(path: &str) -> Result<String, String> {
    let bytes = rapidr_value::objects::read_file(path)?;
    let body = bytes.strip_prefix(b"\xEF\xBB\xBF".as_slice()).unwrap_or(&bytes);
    Ok(match std::str::from_utf8(body) {
        Ok(s) => s.to_string(),
        Err(_) => bytes.iter().map(|&b| char::from(b)).collect(),
    })
}

pub(crate) fn write_text(path: &str, text: &str) -> Result<(), String> {
    rapidr_value::objects::write_file(path, text.as_bytes())
}

/// Where there is no disk (RapidR Studio in a browser), the language
/// service and the designer read a program's `$INCLUDE`d files through the
/// runtime's file hooks — the page's store, the site's files — as the
/// program's run does (`project::program_files`): the web runtime calls this
/// when it installs its file hooks.
pub fn install_page_sources() {
    rapidr_preprocessor::set_source_reader(Some(page_source));
}

fn page_source(path: &std::path::Path) -> Option<Vec<u8>> {
    rapidr_value::objects::read_file(&slashes(&path.to_string_lossy())).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths() {
        assert_eq!(folder_of("/a/b/c.rr"), "/a/b");
        assert_eq!(folder_of("c.rr"), "");
        assert_eq!(folder_of(r"C:\x\y.bas"), "C:/x");
        assert_eq!(join("/a/b", "c/d.rr"), "/a/b/c/d.rr");
        assert_eq!(join("", "d.rr"), "d.rr");
        assert_eq!(join("/a", "/abs.rr"), "/abs.rr");
        assert_eq!(join("/a", "C:/abs.rr"), "C:/abs.rr");
        assert!(is_studio_type("rproject") && !is_studio_type("RFORM"));
    }
}

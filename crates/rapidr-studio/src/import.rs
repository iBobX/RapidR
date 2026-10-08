//! RapidR Studio's File ▸ "Import RapidQ Project or File…"
//! (docs/ide-plan.md, R-NAMES): a **copy** of a RapidQ program — a `.bas`,
//! `.rqw`, `.rqb`, `.rq` or `.inc` file and what it `$INCLUDE`s, or a
//! folder of them (a `.rrproj`: its folder) — with RapidR's names, proved
//! to compile to the same bytecode (`rapidr_import`, the engine of `rapidr
//! import-rapidq`). The original is only read. The copy goes beside it
//! (`<name>-rapidr`, the next free name) with its report
//! (`rapidr-import-report.md`) and a project (`<name>.rrproj`, the
//! RapidQ-compatible setting off) that Studio opens.
//!
//! On the desktop the engine works on the disk; on the web on the page's
//! store (what the user picked: `rapidr_import::Memory`), the copy written
//! back into it — one engine, the same copy.

use std::path::Path;

use rapidr_import::{report, Options};
use rapidr_project::{kind_for_path, kind_for_source, Project};

use crate::{folder_of, join, slashes, Host};

/// What an import made: the copy's project file, the copy's folder, the
/// report's file and its one-line summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Imported {
    pub project: String,
    pub folder: String,
    pub report: String,
    pub summary: String,
}

fn ext(path: &str) -> String {
    path.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).filter(|e| !e.contains('/')).unwrap_or_default()
}

/// The copy's default folder: `<name>-rapidr` beside the source (a file's
/// name without its extension, a folder's name), the next free one
/// (`-rapidr-2` …) when that is taken.
pub fn default_dest(source: &str, exists: impl Fn(&str) -> bool) -> String {
    let source = slashes(source).trim_end_matches('/').to_string();
    let name = source.rsplit('/').next().unwrap_or(&source);
    let stem = if ext(name).is_empty() || exists_as_folder(&source, &exists) { name.to_string() } else { name.rsplit_once('.').map_or(name, |(s, _)| s).to_string() };
    let parent = folder_of(&source);
    (1..1000)
        .map(|n| join(&parent, &if n == 1 { format!("{stem}-rapidr") } else { format!("{stem}-rapidr-{n}") }))
        .find(|d| !exists(d))
        .unwrap_or_else(|| join(&parent, &format!("{stem}-rapidr")))
}

fn exists_as_folder(path: &str, exists: &impl Fn(&str) -> bool) -> bool {
    // (a folder named like a file, `prog.v2`: a folder still)
    !rapidr_value::objects::read_file(path).is_ok() && exists(path)
}

/// Imports `source` into `dest` ("": [`default_dest`]).
pub fn import<H: Host>(host: H, source: &str, dest: &str) -> Result<Imported, String> {
    let source = slashes(source).trim_end_matches('/').to_string();
    if source.is_empty() {
        return Err("nothing to import".into());
    }
    let options = Options { include_dirs: rapidr_import::import::env_include_dirs(), normalize_case: true };
    #[cfg(not(target_arch = "wasm32"))]
    let (r, dest) = {
        let _ = host;
        let dest = if dest.is_empty() { default_dest(&source, |p| Path::new(p).exists()) } else { slashes(dest) };
        (rapidr_import::import(Path::new(&source), Path::new(&dest), &options, true)?, dest)
    };
    #[cfg(target_arch = "wasm32")]
    let (r, dest) = {
        // (the page's store: the source's folder and what is under it)
        let is_file = rapidr_value::objects::read_file(&source).is_ok();
        let root = if is_file { folder_of(&source) } else { source.clone() };
        let stored = |folder: &str| !host.list_tree(folder).is_empty();
        let dest = if dest.is_empty() { default_dest(&source, |p| rapidr_value::objects::read_file(p).is_ok() || stored(p)) } else { slashes(dest) };
        let files: Vec<(std::path::PathBuf, Vec<u8>)> = host.list_tree(&root).into_iter().filter_map(|rel| {
            let p = join(&root, &rel);
            rapidr_value::objects::read_file(&p).ok().map(|b| (std::path::PathBuf::from(p), b))
        }).collect();
        if files.is_empty() {
            return Err(format!("{source}: nothing there to import"));
        }
        let mem = rapidr_import::Memory::new(files);
        let r = rapidr_import::import_with(&mem, Path::new(&source), Path::new(&dest), &options, true)?;
        for (p, bytes) in mem.written() {
            rapidr_value::objects::write_file(&p.to_string_lossy(), &bytes)?;
        }
        (r, dest)
    };
    let folder = slashes(&r.out_dir.to_string_lossy());
    let report_file = join(&folder, report::FILE_NAME);
    rapidr_value::objects::write_file(&report_file, report::markdown(&r).as_bytes())?;
    let project = write_project(&source, &folder, &r)?;
    let _ = dest;
    Ok(Imported { project, folder, report: report_file, summary: report::summary(&r) })
}

/// The copy's project: the `.rrproj` the import copied (a project was
/// imported), else a new one — its main file the first program's copy,
/// every file of the copy in it; the RapidQ-compatible setting off (the
/// copy writes RapidR's names).
fn write_project(source: &str, folder: &str, r: &rapidr_import::ImportReport) -> Result<String, String> {
    let rel = |p: &Path| -> String {
        let p = slashes(&p.to_string_lossy());
        p.strip_prefix(&format!("{}/", folder.trim_end_matches('/'))).map(str::to_string).unwrap_or(p)
    };
    let copied_project = r.files.iter().map(|f| rel(&f.dest)).find(|p| ext(p) == "rrproj" && !p.contains('/'));
    let name = {
        let n = source.rsplit('/').next().unwrap_or(source);
        if ext(n).is_empty() { n.to_string() } else { n.rsplit_once('.').map_or(n, |(s, _)| s).to_string() }
    };
    let (mut project, file) = match copied_project {
        Some(p) => {
            let file = join(folder, &p);
            let text = crate::read_text(&file)?;
            (Project::from_toml(&text).map_err(|e| e.to_string())?, file)
        }
        None => {
            // (an include file imported alone: itself)
            let main = r.programs.first().map(|p| rel(&p.copy)).or_else(|| r.files.iter().find(|f| f.is_source).map(|f| rel(&f.dest))).unwrap_or_default();
            let mut project = Project::new(name.clone(), "");
            project.main = rapidr_project::normalize_path(&main);
            for f in r.files.iter().filter(|f| f.is_source) {
                let p = rel(&f.dest);
                if !project.contains(&p) {
                    // (a file that makes a form is a form, as the tree shows it)
                    let kind = crate::read_text(&join(folder, &p)).map_or_else(|_| kind_for_path(&p), |t| kind_for_source(&p, &t));
                    project.add_file(&p, kind);
                }
            }
            (project, join(folder, &format!("{name}.rrproj")))
        }
    };
    project.compat.rapidq_compatible = false;
    crate::write_text(&file, &project.to_toml())?;
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_copy_goes_beside_under_a_free_name() {
        let taken = ["/p/notepad-rapidr", "/p/notepad-rapidr-2"];
        let exists = |p: &str| taken.contains(&p);
        assert_eq!(default_dest("/p/notepad.bas", exists), "/p/notepad-rapidr-3");
        assert_eq!(default_dest("/p/Calc.RQW", |_| false), "/p/Calc-rapidr");
        assert_eq!(default_dest("/p/games/", |_| false), "/p/games-rapidr");
    }
}

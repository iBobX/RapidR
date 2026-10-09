//! RapidR Studio's project model (docs/ide-plan.md, I0, decision D2).
//!
//! - [`Project`]: the `.rrproj` file, format 2, in TOML. Paths are relative
//!   to the project file's folder with `/` separators; every section is
//!   optional and unknown keys are ignored. The future `RProject` component
//!   wraps it (`Name`, `MainFile`, `FileCount`, `File(i)`, `CompatMode`;
//!   `Open`, `Save`, `New(Template)`, `AddFile`, `RemoveFile`).
//! - [`Workspace`]: per-user state (open documents, dock layout,
//!   breakpoints, watches) in `.rapidr/workspace.toml` beside the project,
//!   ignored by version control.
//! - [`import_v1`]: the web IDE's JSON projects (`"rapidr_project": 1`)
//!   become a format 2 project: one `.rr` file per form (its CREATE block),
//!   one per module, the inline data-URL assets as files.
//! - [`Project::implicit`]: a plain `.bas` / `.rr` file opened without a
//!   project, with the files it `$INCLUDE`s, RapidQ's way.
//!
//! Everything that parses or produces text is a pure function over strings
//! (usable on the web, `wasm32-unknown-unknown`); the functions that touch
//! the file system take paths and are named for it (`load`, `save`, `open`,
//! `import_v1_file`, `Project::implicit`).

mod error;
pub mod forms;
mod implicit;
mod project;
mod v1;
mod workspace;

use std::path::Path;

pub use error::ProjectError;
pub use implicit::{implicit_from_resolver, implicit_from_sources, include_targets};
pub use project::{
    basic_string, defines_form, kind_for_file, kind_for_path, kind_for_source, normalize_path, Ai,
    Build, Compat, CompatLevel, DataAccess, Designer, FileKind, Project, ProjectFile, Run,
    DEFAULT_OUTPUT, FORMAT, HEADER, TEMPLATES,
};
pub use v1::{
    decode_base64, decode_data_url, import_v1, import_v1_file, is_v1_json, sanitize_file_name,
    Imported,
};
pub use workspace::{workspace_path, Breakpoint, Document, Workspace, STATE_DIR, WORKSPACE_HEADER};

/// What [`open`] found.
#[derive(Debug, Clone, PartialEq)]
pub enum Opened {
    /// A format 2 `.rrproj`.
    Project(Project),
    /// A web IDE v1 project, imported in memory (nothing written).
    V1(Imported),
    /// A plain source file and what it includes.
    Implicit(Project),
}

impl Opened {
    /// The project, however it was opened.
    pub fn project(&self) -> &Project {
        match self {
            Opened::Project(p) | Opened::Implicit(p) => p,
            Opened::V1(imported) => &imported.project,
        }
    }

    /// Takes the project out.
    pub fn into_project(self) -> Project {
        match self {
            Opened::Project(p) | Opened::Implicit(p) => p,
            Opened::V1(imported) => imported.project,
        }
    }
}

/// Opens whatever the user picked: a `.rrproj` (format 2 TOML, or a web
/// IDE v1 JSON project, detected by content and imported in memory), or a
/// `.bas` / `.rr` / `.inc` file (an implicit project).
pub fn open(path: &Path) -> Result<Opened, ProjectError> {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "rrproj" | "json" => {
            let text = std::fs::read_to_string(path).map_err(|e| ProjectError::io(path, e))?;
            if is_v1_json(&text) {
                import_v1(&text).map(Opened::V1)
            } else if text.trim_start().starts_with('{') {
                Err(ProjectError::NotAProject)
            } else {
                Project::from_toml(&text).map(Opened::Project)
            }
        }
        "bas" | "rr" | "inc" | "rqw" | "rqb" | "rq" => Project::implicit(path).map(Opened::Implicit),
        _ => Err(ProjectError::UnsupportedFile(path.display().to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_dispatches() {
        let dir = std::env::temp_dir().join(format!("rapidr-project-open-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let v2 = dir.join("a.rrproj");
        Project::new("A", "a.rr").save(&v2).unwrap();
        assert!(matches!(open(&v2).unwrap(), Opened::Project(p) if p.name == "A"));

        let v1 = dir.join("b.rrproj");
        std::fs::write(&v1, r#"{"rapidr_project":1,"name":"B","forms":[]}"#).unwrap();
        let opened = open(&v1).unwrap();
        assert!(matches!(&opened, Opened::V1(_)));
        assert_eq!(opened.project().main, "B.rr");
        assert!(!dir.join("B.rr").exists(), "nothing written");

        let other_json = dir.join("c.rrproj");
        std::fs::write(&other_json, "{\"x\": 1}").unwrap();
        assert_eq!(open(&other_json).unwrap_err(), ProjectError::NotAProject);

        let src = dir.join("c.bas");
        std::fs::write(&src, "PRINT 1\n").unwrap();
        assert_eq!(open(&src).unwrap().into_project().main, "c.bas");

        assert!(matches!(
            open(&dir.join("x.exe")),
            Err(ProjectError::UnsupportedFile(_))
        ));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

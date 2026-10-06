//! Per-user workspace state: `.rapidr/workspace.toml` beside the project
//! file. Not shared: `.rapidr/` holds a `.gitignore` of `*`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::ProjectError;

/// The first line of every workspace file this crate writes.
pub const WORKSPACE_HEADER: &str = "# RapidR Studio workspace state (per user, not shared)";

/// The folder beside the project file that holds per-user state.
pub const STATE_DIR: &str = ".rapidr";

/// An open editor and where it was.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Document {
    /// A project path (relative to the project folder, `/` separators).
    pub path: String,
    /// 1-based.
    pub caret_line: u32,
    /// 1-based.
    pub caret_column: u32,
    /// The first visible line, 1-based.
    pub scroll_line: u32,
}

impl Default for Document {
    fn default() -> Self {
        Document {
            path: String::new(),
            caret_line: 1,
            caret_column: 1,
            scroll_line: 1,
        }
    }
}

/// A breakpoint (a project path and a 1-based line).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Breakpoint {
    pub file: String,
    pub line: u32,
    /// Break only when this expression is true.
    pub condition: Option<String>,
    /// Break only when the hit count matches (`5`, `>= 10`, `% 2`).
    pub hit_condition: Option<String>,
    /// A logpoint: print this message instead of breaking.
    pub log_message: Option<String>,
    pub enabled: bool,
}

impl Default for Breakpoint {
    fn default() -> Self {
        Breakpoint {
            file: String::new(),
            line: 1,
            condition: None,
            hit_condition: None,
            log_message: None,
            enabled: true,
        }
    }
}

/// What one user had open in one project.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Workspace {
    pub open_documents: Vec<Document>,
    pub active_document: Option<String>,
    /// The dock layout, as the IDE serializes it.
    pub dock_layout: Option<String>,
    pub breakpoints: Vec<Breakpoint>,
    pub watches: Vec<String>,
}

/// Where the workspace state of the project at `project_path` lives:
/// `<project folder>/.rapidr/workspace.toml`.
pub fn workspace_path(project_path: &Path) -> PathBuf {
    project_folder(project_path)
        .join(STATE_DIR)
        .join("workspace.toml")
}

fn project_folder(project_path: &Path) -> PathBuf {
    match project_path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => PathBuf::from("."),
    }
}

impl Workspace {
    /// Reads workspace TOML (missing keys take their defaults, unknown keys
    /// are ignored).
    pub fn from_toml(text: &str) -> Result<Workspace, ProjectError> {
        toml::from_str(text).map_err(|e| ProjectError::Toml(e.to_string()))
    }

    /// The workspace as TOML, starting with [`WORKSPACE_HEADER`].
    pub fn to_toml(&self) -> String {
        let body = toml::to_string(self).expect("a workspace always serializes to TOML");
        format!("{WORKSPACE_HEADER}\n\n{body}")
    }

    /// The workspace of the project at `project_path`; a missing file is an
    /// empty workspace.
    pub fn load(project_path: &Path) -> Result<Workspace, ProjectError> {
        let path = workspace_path(project_path);
        match std::fs::read_to_string(&path) {
            Ok(text) => Workspace::from_toml(&text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Workspace::default()),
            Err(e) => Err(ProjectError::io(&path, e)),
        }
    }

    /// Writes the workspace of the project at `project_path`, creating
    /// `.rapidr/` (with a `.gitignore` of `*`) when needed.
    pub fn save(&self, project_path: &Path) -> Result<(), ProjectError> {
        let dir = project_folder(project_path).join(STATE_DIR);
        std::fs::create_dir_all(&dir).map_err(|e| ProjectError::io(&dir, e))?;
        let ignore = dir.join(".gitignore");
        if !ignore.exists() {
            std::fs::write(&ignore, "*\n").map_err(|e| ProjectError::io(&ignore, e))?;
        }
        let path = dir.join("workspace.toml");
        std::fs::write(&path, self.to_toml()).map_err(|e| ProjectError::io(&path, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Workspace {
        Workspace {
            open_documents: vec![
                Document {
                    path: "main.rr".into(),
                    caret_line: 12,
                    caret_column: 5,
                    scroll_line: 1,
                },
                Document {
                    path: "Form1.rr".into(),
                    ..Document::default()
                },
            ],
            active_document: Some("main.rr".into()),
            dock_layout: Some("left:project,properties;bottom:output".into()),
            breakpoints: vec![
                Breakpoint {
                    file: "main.rr".into(),
                    line: 3,
                    ..Breakpoint::default()
                },
                Breakpoint {
                    file: "Form1.rr".into(),
                    line: 20,
                    condition: Some("i > 5".into()),
                    hit_condition: Some(">= 2".into()),
                    log_message: Some("i = {i}".into()),
                    enabled: false,
                },
            ],
            watches: vec!["i".into(), "Form1.Caption".into()],
        }
    }

    #[test]
    fn round_trip() {
        let w = sample();
        let text = w.to_toml();
        assert!(text.starts_with(WORKSPACE_HEADER));
        assert_eq!(Workspace::from_toml(&text).unwrap(), w);
        assert_eq!(Workspace::from_toml("").unwrap(), Workspace::default());
    }

    #[test]
    fn defaults() {
        let w = Workspace::from_toml(
            "[[breakpoints]]\nfile = \"a.rr\"\nline = 4\n[[open_documents]]\npath = \"a.rr\"\nzoom = 2\n",
        )
        .unwrap();
        assert!(w.breakpoints[0].enabled);
        assert_eq!(w.breakpoints[0].condition, None);
        assert_eq!(w.open_documents[0].caret_line, 1);
        assert_eq!(w.active_document, None);
    }

    #[test]
    fn path_and_disk() {
        assert_eq!(
            workspace_path(Path::new("/p/Demo.rrproj")),
            Path::new("/p/.rapidr/workspace.toml")
        );
        assert_eq!(
            workspace_path(Path::new("Demo.rrproj")),
            Path::new("./.rapidr/workspace.toml")
        );
        let dir = std::env::temp_dir().join(format!("rapidr-project-ws-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let project = dir.join("Demo.rrproj");
        assert_eq!(Workspace::load(&project).unwrap(), Workspace::default());
        let w = sample();
        w.save(&project).unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.join(".rapidr/.gitignore")).unwrap(),
            "*\n"
        );
        assert_eq!(Workspace::load(&project).unwrap(), w);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

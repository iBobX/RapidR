//! The one error type of the crate.

use std::fmt;

/// Why a project (or workspace) could not be read, written or imported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectError {
    /// The text has no `format` key (or is not a project file at all).
    NotAProject,
    /// The project file was written by a newer RapidR Studio (or names a
    /// format this version does not read).
    UnsupportedFormat(i64),
    /// The TOML could not be parsed or does not match the project model.
    Toml(String),
    /// The JSON could not be parsed.
    Json(String),
    /// The JSON parses but is not a web IDE v1 project.
    InvalidV1(String),
    /// `new_from_template` was given a template name it does not know.
    UnknownTemplate(String),
    /// `open` was given a file it cannot open (by extension).
    UnsupportedFile(String),
    /// Writing would replace a file that is already there.
    AlreadyExists(String),
    /// A file system operation failed.
    Io { path: String, message: String },
}

impl ProjectError {
    pub(crate) fn io(path: &std::path::Path, err: std::io::Error) -> Self {
        ProjectError::Io {
            path: path.display().to_string(),
            message: err.to_string(),
        }
    }
}

impl fmt::Display for ProjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProjectError::NotAProject => {
                write!(f, "not a RapidR project (the file has no `format` key)")
            }
            ProjectError::UnsupportedFormat(n) if *n > crate::FORMAT => write!(
                f,
                "this project uses format {n}, written by a newer RapidR Studio; \
                 this version reads format {} (update RapidR to open it)",
                crate::FORMAT
            ),
            ProjectError::UnsupportedFormat(n) => write!(
                f,
                "unsupported project format {n}: a TOML .rrproj is format {} \
                 (format 1 projects are the web IDE's JSON files)",
                crate::FORMAT
            ),
            ProjectError::Toml(m) => write!(f, "invalid project file: {m}"),
            ProjectError::Json(m) => write!(f, "invalid JSON: {m}"),
            ProjectError::InvalidV1(m) => write!(f, "not a web IDE project (v1): {m}"),
            ProjectError::UnknownTemplate(t) => write!(
                f,
                "unknown project template `{t}` (the templates are `console` and `gui`)"
            ),
            ProjectError::UnsupportedFile(p) => write!(
                f,
                "cannot open `{p}` as a project: expected a .rrproj, .bas, .rr or .inc file"
            ),
            ProjectError::AlreadyExists(p) => {
                write!(f, "`{p}` already exists; not replacing it")
            }
            ProjectError::Io { path, message } => write!(f, "{path}: {message}"),
        }
    }
}

impl std::error::Error for ProjectError {}

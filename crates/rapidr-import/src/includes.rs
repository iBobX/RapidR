//! A program's `$INCLUDE`s in an import: where each one found its file
//! (the report's "Include files"), and what the copy's line must say so
//! that it finds the file's copy — `$INCLUDE "..\shared\s.inc"` read beside
//! the copy would find the original, `C:\rapidq\inc\x.inc` is nowhere.
//!
//! The places are the compiler's own ([`rapidr_preprocessor::include_search`]:
//! RC.EXE's rules, then RapidR's), so the report says what the compiler
//! did, and the copy is planned the way the compiler will read it.

use std::ops::Range;
use std::path::{Component, Path, PathBuf};

use rapidr_preprocessor::include_search;

/// Where an `$INCLUDE` found its file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Place {
    /// In the main program's folder (RC.EXE: the folder it compiles in).
    ProgramFolder,
    /// In this include folder (RC.EXE's `-I`; Studio's Settings ▸ Include
    /// folders, `--include-dir`).
    IncludeFolder(PathBuf),
    /// Beside the file that includes it (RapidR's addition).
    BesideIncluder,
    /// By its file name (or a shorter ending of its path), in this folder:
    /// a path written on another computer (`C:\rapidq\inc\x.inc`).
    ByName(PathBuf),
    /// At the absolute path written.
    Absolute,
    /// In this other folder the search looks in (the folder above an
    /// include folder, the current folder).
    Elsewhere(PathBuf),
    /// RapidR supplies it: no file read ([`rapidr_preprocessor::is_builtin_include`]).
    BuiltIn,
    /// Nowhere: the program doesn't compile.
    Missing,
}

/// One `$INCLUDE` of a program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncludeRef {
    /// The file it is written in (as read), its line (from 1).
    pub from: PathBuf,
    pub line: usize,
    /// The name as written (`Inc\MyLib.INC`) and its bytes in the text of
    /// `from`.
    pub written: String,
    pub span: Range<usize>,
    /// The file it read (`None`: built in, or missing).
    pub file: Option<PathBuf>,
    pub place: Place,
}

/// `p` with `.` and `..` resolved as text and `/` separators: two names of
/// one place compare equal (in any case, [`same`]).
pub(crate) fn lexical(p: &Path) -> String {
    let text = p.to_string_lossy().replace('\\', "/");
    let absolute = text.starts_with('/');
    let mut parts: Vec<String> = Vec::new();
    for c in Path::new(&text).components() {
        match c {
            Component::ParentDir => {
                if parts.last().is_some_and(|l| l != "..") {
                    parts.pop();
                } else if !absolute {
                    parts.push("..".into());
                }
            }
            Component::Normal(n) => parts.push(n.to_string_lossy().into_owned()),
            Component::Prefix(p) => parts.push(p.as_os_str().to_string_lossy().into_owned()),
            _ => {}
        }
    }
    format!("{}{}", if absolute { "/" } else { "" }, parts.join("/"))
}

/// Whether two paths name one place (any case: RapidQ's names were written
/// on Windows).
pub(crate) fn same(a: &Path, b: &Path) -> bool {
    lexical(a).eq_ignore_ascii_case(&lexical(b))
}

/// The name as written, without a drive or a leading `\` (the part the
/// search's places join).
fn written_rel(written: &str) -> String {
    let t = written.trim().replace('\\', "/");
    let b = t.as_bytes();
    let rel = if b.len() >= 2 && b[1] == b':' && b[0].is_ascii_alphabetic() { &t[2..] } else { &t[..] };
    rel.trim_start_matches('/').to_string()
}

/// Where `found` was found for `$INCLUDE "written"` in a file of
/// `includer_dir` of the program in `main_dir`: the search's first place
/// that is `found`.
pub(crate) fn place_of(written: &str, includer_dir: &Path, main_dir: &Path, include_dirs: &[PathBuf], found: &Path) -> Place {
    let whole = written_rel(written);
    let absolute = {
        let t = written.trim().replace('\\', "/");
        t.starts_with('/') || (t.len() >= 2 && t.as_bytes()[1] == b':')
    };
    for (root, rel) in include_search(written, includer_dir, main_dir, include_dirs) {
        if !same(&root.join(&rel), found) {
            continue;
        }
        if rel != whole {
            return Place::ByName(found.parent().map(Path::to_path_buf).unwrap_or(root));
        }
        if absolute {
            return if root.parent().is_none() { Place::Absolute } else { Place::ByName(root) };
        }
        if same(&root, main_dir) {
            return Place::ProgramFolder;
        }
        if let Some(d) = include_dirs.iter().find(|d| same(d, &root)) {
            return Place::IncludeFolder(d.clone());
        }
        if same(&root, includer_dir) {
            return Place::BesideIncluder;
        }
        return Place::Elsewhere(root);
    }
    // (a file held in memory found by its name alone: the store's)
    Place::ByName(found.parent().map(Path::to_path_buf).unwrap_or_default())
}

/// `to`'s path from folder `from_dir`, `/`-separated (`../include/x.inc`).
pub(crate) fn relative(from_dir: &Path, to: &Path) -> String {
    let a = lexical(from_dir);
    let b = lexical(to);
    let a: Vec<&str> = a.split('/').filter(|p| !p.is_empty()).collect();
    let b: Vec<&str> = b.split('/').filter(|p| !p.is_empty()).collect();
    let common = a.iter().zip(&b).take_while(|(x, y)| x.eq_ignore_ascii_case(y)).count();
    let mut out: Vec<&str> = vec![".."; a.len() - common];
    out.extend(&b[common..]);
    out.join("/")
}

/// `rel` written as `like` writes paths: with `\` when it does (RapidQ's
/// way), else `/`.
pub(crate) fn written_like(like: &str, rel: &str) -> String {
    if like.contains('\\') {
        rel.replace('/', "\\")
    } else {
        rel.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_compare_and_relate() {
        assert_eq!(lexical(Path::new("/a/b/../c/./d.inc")), "/a/c/d.inc");
        assert!(same(Path::new("/A/Inc/X.INC"), Path::new("/a/inc/../inc/x.inc")));
        assert_eq!(relative(Path::new("/out/sub"), Path::new("/out/include/x.inc")), "../include/x.inc");
        assert_eq!(relative(Path::new("/out"), Path::new("/out/GL/qgl.inc")), "GL/qgl.inc");
        assert_eq!(written_like("Inc\\A.inc", "include/a.inc"), "include\\a.inc");
        assert_eq!(written_like("lib/a.inc", "include/a.inc"), "include/a.inc");
    }

    #[test]
    fn places() {
        let (main, inc) = (PathBuf::from("/p"), PathBuf::from("/rq/include"));
        let dirs = [inc.clone()];
        assert_eq!(place_of("x.inc", Path::new("/p/sub"), &main, &dirs, Path::new("/p/X.INC")), Place::ProgramFolder);
        assert_eq!(place_of("x.inc", Path::new("/p/sub"), &main, &dirs, Path::new("/rq/include/x.inc")), Place::IncludeFolder(inc.clone()));
        assert_eq!(place_of("x.inc", Path::new("/p/sub"), &main, &dirs, Path::new("/p/sub/x.inc")), Place::BesideIncluder);
        assert_eq!(place_of("C:\\rapidq\\inc\\x.inc", &main, &main, &dirs, Path::new("/rq/include/x.inc")), Place::ByName(inc.clone()));
        assert_eq!(place_of("Inc\\x.inc", &main, &main, &dirs, Path::new("/p/inc/x.inc")), Place::ProgramFolder);
    }
}

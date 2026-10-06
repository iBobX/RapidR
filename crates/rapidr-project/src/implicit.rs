//! Plain `.bas` / `.rr` files opened without a project file: an implicit
//! project made of the file and what it `$INCLUDE`s, RapidQ's way.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::error::ProjectError;
use crate::project::{kind_for_path, kind_for_source, normalize_path, Project};

/// The files a source `$INCLUDE`s (`$INCLUDE "x"` or `$INCLUDE <x>`, any
/// case), as written, in order.
pub fn include_targets(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim_start();
        let Some(head) = line.get(..8) else { continue };
        if !head.eq_ignore_ascii_case("$INCLUDE") {
            continue;
        }
        let rest = &line[8..];
        if !rest.starts_with(|c: char| c.is_whitespace() || c == '"' || c == '<') {
            continue; // `$INCLUDEX` is something else
        }
        let rest = rest.trim_start();
        let close = match rest.chars().next() {
            Some('"') => '"',
            Some('<') => '>',
            _ => continue,
        };
        if let Some(end) = rest[1..].find(close) {
            let target = rest[1..1 + end].trim();
            if !target.is_empty() {
                out.push(target.to_string());
            }
        }
    }
    out
}

fn is_absolute(path: &str) -> bool {
    let b = path.as_bytes();
    path.starts_with('/') || path.starts_with('\\') || (b.len() >= 2 && b[1] == b':')
}

fn folder_of(path: &str) -> &str {
    match path.rfind('/') {
        Some(i) => &path[..=i],
        None => "",
    }
}

fn stem_of(path: &str) -> String {
    let name = path.rsplit('/').next().unwrap_or(path);
    match name.rfind('.') {
        Some(i) if i > 0 => name[..i].to_string(),
        _ => name.to_string(),
    }
}

/// The implicit project of `main_name` (a path relative to the project
/// folder), reading files through `lookup` (project path → contents; `None`
/// when there is no such file). Includes resolve relative to the including
/// file's folder, each file once, recursively; missing ones are skipped.
pub fn implicit_from_sources(main_name: &str, lookup: &dyn Fn(&str) -> Option<String>) -> Project {
    implicit_from_resolver(main_name, &|path: &str| {
        lookup(path).map(|text| (path.to_string(), text))
    })
}

/// Like [`implicit_from_sources`], but `resolve` also returns the file's
/// real project path (for a lookup that matches names without regard to
/// case).
pub fn implicit_from_resolver(
    main_name: &str,
    resolve: &dyn Fn(&str) -> Option<(String, String)>,
) -> Project {
    let main = normalize_path(main_name);
    let mut project = Project {
        name: stem_of(&main),
        main: main.clone(),
        ..Project::default()
    };
    let mut seen = HashSet::new();
    seen.insert(main.to_lowercase());
    match resolve(&main) {
        Some((actual, text)) => {
            let actual = normalize_path(&actual);
            seen.insert(actual.to_lowercase());
            project.main = actual.clone();
            project.add_file(&actual, kind_for_source(&actual, &text));
            add_includes(&actual, &text, resolve, &mut project, &mut seen);
        }
        None => {
            project.add_file(&main, kind_for_path(&main));
        }
    }
    project
}

fn add_includes(
    from: &str,
    text: &str,
    resolve: &dyn Fn(&str) -> Option<(String, String)>,
    project: &mut Project,
    seen: &mut HashSet<String>,
) {
    for target in include_targets(text) {
        if is_absolute(&target) {
            continue;
        }
        let rel = normalize_path(&format!("{}{}", folder_of(from), target.replace('\\', "/")));
        if !seen.insert(rel.to_lowercase()) {
            continue;
        }
        let Some((actual, contents)) = resolve(&rel) else {
            continue; // RAPIDQ.INC and friends: not here, not an error
        };
        let actual = normalize_path(&actual);
        if actual.to_lowercase() != rel.to_lowercase() && !seen.insert(actual.to_lowercase()) {
            continue;
        }
        project.add_file(&actual, kind_for_source(&actual, &contents));
        add_includes(&actual, &contents, resolve, project, seen);
    }
}

/// The real name of `wanted` in `folder`: the exact name, else one that
/// differs only in case; a folder when `want_dir`, else a file.
fn match_entry(folder: &Path, wanted: &str, want_dir: bool) -> Option<String> {
    let lower = wanted.to_lowercase();
    let mut fallback = None;
    for entry in std::fs::read_dir(folder).ok()?.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let exact = name == wanted;
        if !exact && (fallback.is_some() || name.to_lowercase() != lower) {
            continue;
        }
        let path = entry.path();
        let fits = if want_dir {
            path.is_dir()
        } else {
            path.is_file()
        };
        if !fits {
            continue;
        }
        if exact {
            return Some(name);
        }
        fallback = Some(name);
    }
    fallback
}

/// Finds `rel` (a project path) under `root`, each name matched without
/// regard to case; returns the file's real project path and text.
fn resolve_on_disk(root: &Path, rel: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = rel.split('/').collect();
    let (file, dirs) = parts.split_last()?;
    let mut folder: PathBuf = root.to_path_buf();
    let mut actual = String::new();
    for dir in dirs {
        let name = if *dir == ".." {
            "..".to_string()
        } else {
            match_entry(&folder, dir, true)?
        };
        folder.push(&name);
        actual.push_str(&name);
        actual.push('/');
    }
    let name = match_entry(&folder, file, false)?;
    let bytes = std::fs::read(folder.join(&name)).ok()?;
    actual.push_str(&name);
    Some((actual, String::from_utf8_lossy(&bytes).into_owned()))
}

impl Project {
    /// The implicit project of a plain source file: named after it, with it
    /// as the main file and the files it `$INCLUDE`s (those that exist).
    pub fn implicit(main_path: &Path) -> Result<Project, ProjectError> {
        if !main_path.is_file() {
            return Err(ProjectError::Io {
                path: main_path.display().to_string(),
                message: "no such file".into(),
            });
        }
        let root = match main_path.parent() {
            Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
            _ => PathBuf::from("."),
        };
        let main_name = main_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        Ok(implicit_from_resolver(&main_name, &|rel: &str| {
            resolve_on_disk(&root, rel)
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::FileKind;
    use std::collections::HashMap;

    #[test]
    fn scans_includes() {
        let text = "$INCLUDE \"rapidq.inc\"\n  $include <Lib\\x.inc> ' std\n\
                    '$INCLUDE \"no.inc\"\n$INCLUDEX \"no\"\n$Include\"tight.inc\"\nPRINT 1\n$INCLUDE \"\"\n";
        assert_eq!(
            include_targets(text),
            ["rapidq.inc", "Lib\\x.inc", "tight.inc"]
        );
    }

    #[test]
    fn in_memory() {
        let files: HashMap<&str, &str> = HashMap::from([
            (
                "app.bas",
                "$INCLUDE \"RAPIDQ.INC\"\n$INCLUDE \"lib/a.inc\"\n$INCLUDE \"form.rr\"\nPRINT 1\n",
            ),
            ("lib/a.inc", "$INCLUDE \"b.inc\"\n$INCLUDE <../form.rr>\n"),
            ("lib/b.inc", "$INCLUDE \"a.inc\"\n"),
            ("form.rr", "CREATE F AS QFORM\nEND CREATE\n"),
        ]);
        let lookup = |p: &str| files.get(p).map(|s| s.to_string());
        let p = implicit_from_sources("app.bas", &lookup);
        assert_eq!(p.name, "app");
        assert_eq!(p.main, "app.bas");
        let got: Vec<(&str, FileKind)> =
            p.files.iter().map(|f| (f.path.as_str(), f.kind)).collect();
        assert_eq!(
            got,
            [
                ("app.bas", FileKind::Module),
                ("lib/a.inc", FileKind::Include),
                ("lib/b.inc", FileKind::Include),
                ("form.rr", FileKind::Form),
            ]
        );
        // A main file the lookup does not know: just the main file.
        let p = implicit_from_sources("gone.rr", &lookup);
        assert_eq!(p.file_count(), 1);
    }

    #[test]
    fn on_disk_nested_and_case_insensitive() {
        let dir = std::env::temp_dir().join(format!("rapidr-project-impl-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("inc")).unwrap();
        std::fs::write(
            dir.join("Main.bas"),
            "$INCLUDE \"RAPIDQ.INC\"\n$INCLUDE \"INC/UTIL.INC\"\n$include \"Main.bas\"\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("inc/Util.inc"),
            "$INCLUDE \"deeper.inc\"\n$INCLUDE \"..\\Win.rr\"\n",
        )
        .unwrap();
        std::fs::write(dir.join("inc/deeper.inc"), "' nothing\n").unwrap();
        std::fs::write(dir.join("Win.rr"), "create W as QFormMDI\nend create\n").unwrap();

        let p = Project::implicit(&dir.join("Main.bas")).unwrap();
        assert_eq!(p.name, "Main");
        assert_eq!(p.main, "Main.bas");
        let got: Vec<(&str, FileKind)> =
            p.files.iter().map(|f| (f.path.as_str(), f.kind)).collect();
        assert_eq!(
            got,
            [
                ("Main.bas", FileKind::Module),
                ("inc/Util.inc", FileKind::Include),
                ("inc/deeper.inc", FileKind::Include),
                ("Win.rr", FileKind::Form),
            ]
        );
        assert!(Project::implicit(&dir.join("missing.bas")).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

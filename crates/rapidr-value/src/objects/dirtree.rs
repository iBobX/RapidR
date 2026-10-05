//! QDIRTREE (RapidQ manual, Appendix A): a directory tree like Windows
//! Explorer's, from the file system's root. The runtimes show its visible
//! nodes as indented rows ([`DirTree::rows`]): a click selects a directory
//! (`Directory`, then OnChange), a double click opens or closes it.
//!
//! `InitialDir` / `Directory` (the current directory at first) select a
//! directory and open the ones above it; `FullCollapse` closes everything,
//! `FullExpand` opens every directory (up to [`MAX_ROWS`] rows), `Reload`
//! reads the directories again (rows are always read from the disk).
//! Hidden directories (a leading `.`) aren't shown. The web has no file
//! system: only the root.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::{v_int, v_str, Value};

/// Most rows a tree shows (so FullExpand of `/` stays bounded).
pub const MAX_ROWS: usize = 5_000;

#[derive(Clone, Debug)]
pub struct DirTree {
    pub root: PathBuf,
    /// The selected directory.
    pub directory: PathBuf,
    pub initial_dir: String,
    /// Open directories.
    pub expanded: BTreeSet<PathBuf>,
}

/// One visible node.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub depth: usize,
    pub path: PathBuf,
    pub name: String,
    pub expanded: bool,
    pub has_children: bool,
}

fn subdirs(dir: &Path) -> Vec<PathBuf> {
    #[cfg(not(target_arch = "wasm32"))]
    if let Ok(entries) = std::fs::read_dir(dir) {
        let mut dirs: Vec<PathBuf> = entries
            .flatten()
            .take(100_000)
            .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
            .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
            .map(|e| e.path())
            .collect();
        dirs.sort_by_key(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default());
        return dirs;
    }
    let _ = dir;
    Vec::new()
}

impl Default for DirTree {
    fn default() -> Self {
        let cwd = std::env::current_dir().unwrap_or_default();
        let root = cwd.ancestors().last().map(Path::to_path_buf).filter(|r| !r.as_os_str().is_empty()).unwrap_or_else(|| PathBuf::from("/"));
        let mut t = DirTree { root: root.clone(), directory: root, initial_dir: String::new(), expanded: BTreeSet::new() };
        t.expanded.insert(t.root.clone());
        t.go_to(&cwd.to_string_lossy());
        t
    }
}

/// `path` resolved (`..`, links) as a program shows it: on Windows without
/// the `\\?\` prefix `canonicalize` adds (`C:\Users`, `\\server\share`),
/// as RapidQ's paths were — and as the tree's root (`C:\`) is.
pub fn canonical(path: &Path) -> Option<PathBuf> {
    let p = path.canonicalize().ok()?;
    #[cfg(windows)]
    {
        let s = p.to_string_lossy();
        if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
            return Some(PathBuf::from(format!(r"\\{rest}")));
        }
        if let Some(rest) = s.strip_prefix(r"\\?\").filter(|r| r.as_bytes().get(1) == Some(&b':')) {
            return Some(PathBuf::from(rest));
        }
    }
    Some(p)
}

impl DirTree {
    /// Selects `dir` (if it exists), opening the directories above it.
    pub fn go_to(&mut self, dir: &str) -> bool {
        let path = PathBuf::from(dir.replace('\\', "/"));
        let path = if path.is_absolute() { path } else { std::env::current_dir().unwrap_or_default().join(path) };
        let path = canonical(&path).unwrap_or(path);
        if !cfg!(target_arch = "wasm32") && !path.is_dir() {
            return false;
        }
        for a in path.ancestors().skip(1) {
            self.expanded.insert(a.to_path_buf());
        }
        let changed = self.directory != path;
        self.directory = path;
        changed
    }

    /// The visible nodes, depth first.
    pub fn rows(&self) -> Vec<Row> {
        let mut out = Vec::new();
        self.walk(&self.root, 0, &mut out);
        out
    }

    fn walk(&self, dir: &Path, depth: usize, out: &mut Vec<Row>) {
        if out.len() >= MAX_ROWS {
            return;
        }
        let expanded = self.expanded.contains(dir);
        let children = if expanded { subdirs(dir) } else { Vec::new() };
        let name = dir.file_name().map_or_else(|| dir.to_string_lossy().into_owned(), |n| n.to_string_lossy().into_owned());
        // Unread (closed) directories are assumed to have children.
        let has_children = !expanded || !children.is_empty();
        out.push(Row { depth, path: dir.to_path_buf(), name, expanded, has_children });
        for c in children {
            self.walk(&c, depth + 1, out);
        }
    }

    /// A row as text: indented, `+` closed, `-` open.
    pub fn row_text(row: &Row) -> String {
        let mark = match (row.has_children, row.expanded) {
            (false, _) => "  ",
            (true, true) => "- ",
            (true, false) => "+ ",
        };
        format!("{}{mark}{}", "    ".repeat(row.depth), row.name)
    }

    /// The selected directory's row.
    pub fn selected_row(&self) -> Option<usize> {
        self.rows().iter().position(|r| r.path == self.directory)
    }

    /// The user clicked row `i`: it's selected. Returns whether the
    /// directory changed (OnChange).
    pub fn click(&mut self, i: usize) -> bool {
        let Some(row) = self.rows().into_iter().nth(i) else { return false };
        let changed = self.directory != row.path;
        self.directory = row.path;
        changed
    }

    /// The user double-clicked row `i`: it opens or closes.
    pub fn toggle(&mut self, i: usize) {
        let Some(row) = self.rows().into_iter().nth(i) else { return };
        if !self.expanded.remove(&row.path) {
            self.expanded.insert(row.path);
        }
    }

    pub fn get(&self, prop: &str) -> Option<Value> {
        Some(match prop {
            "directory" => v_str(&self.directory.to_string_lossy()),
            "initialdir" => v_str(&self.initial_dir),
            "itemcount" => v_int(self.rows().len() as i64),
            _ => return None,
        })
    }

    /// Sets a property; `Some(true)` if the directory changed (OnChange).
    pub fn set(&mut self, prop: &str, val: &Value) -> Option<bool> {
        match prop {
            "directory" => Some(self.go_to(&val.to_string_val())),
            "initialdir" => {
                self.initial_dir = val.to_string_val();
                Some(self.go_to(&self.initial_dir.clone()))
            }
            _ => None,
        }
    }

    pub fn call(&mut self, method: &str, _args: &[Value]) -> Option<Value> {
        match method {
            "fullcollapse" => {
                self.expanded.clear();
                self.expanded.insert(self.root.clone());
            }
            "fullexpand" => {
                // Open every directory, breadth first, until the rows are many.
                let mut todo = vec![self.root.clone()];
                while let Some(d) = todo.pop() {
                    if self.expanded.len() >= MAX_ROWS {
                        break;
                    }
                    self.expanded.insert(d.clone());
                    todo.extend(subdirs(&d));
                }
            }
            "reload" | "refresh" | "repaint" | "adddirtypes" | "adddrivetypes" | "deldirtypes" | "deldrivetypes" => {}
            _ => return None,
        }
        Some(Value::Null)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browses_directories() {
        let base = std::env::temp_dir().join(format!("rapidr_dirtree_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("b/inner")).unwrap();
        std::fs::create_dir_all(base.join("A")).unwrap();
        std::fs::create_dir_all(base.join(".hidden")).unwrap();
        let base = canonical(&base).unwrap();
        let mut t = DirTree::default();
        assert!(t.set("directory", &v_str(&base.to_string_lossy())).unwrap());
        let rows = t.rows();
        let at = t.selected_row().unwrap();
        // Its subdirectories aren't read until it's opened.
        assert_eq!(rows[at].path, base);
        assert!(!rows[at].expanded);
        t.toggle(at);
        let names: Vec<String> = t.rows()[at + 1..at + 3].iter().map(|r| r.name.clone()).collect();
        assert_eq!(names, ["A", "b"]);
        assert_eq!(DirTree::row_text(&t.rows()[at + 1]), format!("{}+ A", "    ".repeat(t.rows()[at].depth + 1)));
        // Selecting b: the directory changes; again: it doesn't.
        assert!(t.click(at + 2));
        assert!(!t.click(at + 2));
        assert_eq!(t.get("directory").unwrap().to_string_val(), base.join("b").to_string_lossy());
        // FullCollapse: only the root is open.
        t.call("fullcollapse", &[]);
        assert!(t.rows().iter().all(|r| r.depth <= 1));
        let _ = std::fs::remove_dir_all(&base);
    }
}

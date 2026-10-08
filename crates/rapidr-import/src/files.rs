//! Where an import reads and writes: the disk ([`Disk`]: `rapidr
//! import-rapidq`, RapidR Studio on the desktop) or files held in memory
//! ([`Memory`]: RapidR Studio on the web, whose files are the page's
//! store). One engine works on either ([`crate::import::import_with`]).

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

use rapidr_parser::{parse_file_for_tools, parse_source_for_tools, ToolsParse};
use rapidr_preprocessor::{PreprocessOptions, SourceEncoding};

/// The files an import reads and writes.
pub trait Files {
    /// A file's bytes.
    fn read(&self, path: &Path) -> Result<Vec<u8>, String>;
    /// Writes a file (its folders made).
    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), String>;
    /// Makes a folder (and those it is in).
    fn make_dir(&self, path: &Path) -> Result<(), String>;
    fn is_dir(&self, path: &Path) -> bool;
    fn is_file(&self, path: &Path) -> bool;
    /// A folder's files and folders, sorted.
    fn entries(&self, dir: &Path) -> Vec<PathBuf>;
    /// The path as these files name it (on the disk: links and `..`
    /// resolved); `None` when there's nothing there.
    fn canonical(&self, path: &Path) -> Option<PathBuf>;
    /// Program `entry` parsed for tools, its `$INCLUDE`s read from these
    /// files (and `options.include_dirs`).
    fn parse(&self, entry: &Path, options: PreprocessOptions) -> Result<ToolsParse, String>;
    /// What `$INCLUDE` finds without reading the disk (the preprocessor's
    /// `virtual_files`): nothing on the disk, every source file in memory.
    fn virtual_files(&self) -> Vec<(String, String)>;
    /// How a file's text is encoded (as read).
    fn encoding(&self, path: &Path) -> Option<SourceEncoding> {
        self.read(path).ok().map(|b| rapidr_preprocessor::decode_source(&b).1)
    }
}

/// The disk.
#[derive(Debug, Clone, Copy, Default)]
pub struct Disk;

impl Files for Disk {
    fn read(&self, path: &Path) -> Result<Vec<u8>, String> {
        fs::read(path).map_err(|e| format!("{}: {e}", path.display()))
    }

    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
    }

    fn make_dir(&self, path: &Path) -> Result<(), String> {
        fs::create_dir_all(path).map_err(|e| format!("{}: {e}", path.display()))
    }

    fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }

    fn entries(&self, dir: &Path) -> Vec<PathBuf> {
        let Ok(rd) = fs::read_dir(dir) else { return Vec::new() };
        let mut out: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
        out.sort();
        out
    }

    fn canonical(&self, path: &Path) -> Option<PathBuf> {
        fs::canonicalize(path).ok()
    }

    fn parse(&self, entry: &Path, options: PreprocessOptions) -> Result<ToolsParse, String> {
        parse_file_for_tools(entry, options).map_err(|d| d.to_string())
    }

    fn virtual_files(&self) -> Vec<(String, String)> {
        Vec::new()
    }
}

/// Files held in memory, by path ('/'-separated, any case, as the web's
/// store keeps them); what an import writes is kept apart
/// ([`Memory::written`]).
#[derive(Debug, Default)]
pub struct Memory {
    files: BTreeMap<String, (PathBuf, Vec<u8>)>,
    written: RefCell<BTreeMap<String, (PathBuf, Vec<u8>)>>,
}

/// A path as the store compares it: '/'-separated, `.` and `..` resolved,
/// lower case.
fn mem_key(path: &Path) -> String {
    normal(path).to_string_lossy().to_lowercase()
}

/// `path` with `.` and `..` resolved and '/' separators.
fn normal(path: &Path) -> PathBuf {
    let text = path.to_string_lossy().replace('\\', "/");
    let mut parts: Vec<String> = Vec::new();
    let absolute = text.starts_with('/');
    for c in Path::new(&text).components() {
        match c {
            Component::ParentDir => {
                parts.pop();
            }
            Component::Normal(n) => parts.push(n.to_string_lossy().into_owned()),
            _ => {}
        }
    }
    PathBuf::from(format!("{}{}", if absolute { "/" } else { "" }, parts.join("/")))
}

impl Memory {
    /// The files (path, bytes) an import reads.
    pub fn new(files: impl IntoIterator<Item = (PathBuf, Vec<u8>)>) -> Memory {
        let files = files.into_iter().map(|(p, b)| (mem_key(&p), (normal(&p), b))).collect();
        Memory { files, written: RefCell::new(BTreeMap::new()) }
    }

    /// What was written, by path.
    pub fn written(&self) -> Vec<(PathBuf, Vec<u8>)> {
        self.written.borrow().values().cloned().collect()
    }

    fn get(&self, path: &Path) -> Option<(PathBuf, Vec<u8>)> {
        let k = mem_key(path);
        self.written.borrow().get(&k).cloned().or_else(|| self.files.get(&k).cloned())
    }

    /// Every path held (read or written).
    fn all(&self) -> Vec<PathBuf> {
        let mut out: Vec<PathBuf> = self.files.values().map(|(p, _)| p.clone()).collect();
        out.extend(self.written.borrow().values().map(|(p, _)| p.clone()));
        out
    }
}

impl Files for Memory {
    fn read(&self, path: &Path) -> Result<Vec<u8>, String> {
        self.get(path).map(|(_, b)| b).ok_or_else(|| format!("{}: no such file", path.display()))
    }

    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), String> {
        self.written.borrow_mut().insert(mem_key(path), (normal(path), bytes.to_vec()));
        Ok(())
    }

    fn make_dir(&self, _path: &Path) -> Result<(), String> {
        Ok(())
    }

    fn is_dir(&self, path: &Path) -> bool {
        let prefix = format!("{}/", mem_key(path).trim_end_matches('/'));
        self.all().iter().any(|p| mem_key(p).starts_with(&prefix))
    }

    fn is_file(&self, path: &Path) -> bool {
        self.get(path).is_some()
    }

    fn entries(&self, dir: &Path) -> Vec<PathBuf> {
        let base = normal(dir);
        let prefix = format!("{}/", mem_key(dir).trim_end_matches('/'));
        let mut out: Vec<PathBuf> = Vec::new();
        for p in self.all() {
            let k = mem_key(&p);
            let Some(rest) = k.strip_prefix(&prefix) else { continue };
            // (the first part of what is below: a file, or a folder)
            let first_len = rest.find('/').unwrap_or(rest.len());
            let shown = p.to_string_lossy();
            let name = &shown[prefix.len()..prefix.len() + first_len];
            let entry = base.join(name);
            if !out.iter().any(|e| mem_key(e) == mem_key(&entry)) {
                out.push(entry);
            }
        }
        out.sort();
        out
    }

    fn canonical(&self, path: &Path) -> Option<PathBuf> {
        if let Some((p, _)) = self.get(path) {
            return Some(p);
        }
        self.is_dir(path).then(|| normal(path))
    }

    fn parse(&self, entry: &Path, mut options: PreprocessOptions) -> Result<ToolsParse, String> {
        let bytes = self.read(entry)?;
        let (text, _) = rapidr_preprocessor::decode_source(&bytes);
        options.virtual_files.extend(self.virtual_files());
        let base = entry.parent().map(Path::to_path_buf).unwrap_or_default();
        let mut tp = parse_source_for_tools(&text, base, Some(normal(entry)), options);
        // (each file's encoding as its bytes say: a copy keeps it)
        for f in &mut tp.preprocessed.origins.files {
            if let Some(enc) = f.path.as_deref().and_then(|p| self.encoding(p)) {
                f.encoding = enc;
            }
        }
        Ok(tp)
    }

    fn virtual_files(&self) -> Vec<(String, String)> {
        // (the program's own folder's first: an `$INCLUDE` is found by its
        // path, else by its name)
        self.files
            .values()
            .filter(|(p, _)| rapidr_preprocessor::is_source_path(p))
            .map(|(p, b)| (p.to_string_lossy().into_owned(), rapidr_preprocessor::decode_source(b).0))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_lists_folders_and_files() {
        let m = Memory::new([(PathBuf::from("/p/a.bas"), b"PRINT 1".to_vec()), (PathBuf::from("/p/inc/B.inc"), Vec::new()), (PathBuf::from("/q/c.bas"), Vec::new())]);
        assert_eq!(m.entries(Path::new("/p")), vec![PathBuf::from("/p/a.bas"), PathBuf::from("/p/inc")]);
        assert!(m.is_dir(Path::new("/p/inc")) && !m.is_dir(Path::new("/p/a.bas")));
        assert!(m.is_file(Path::new("/P/INC/b.inc")), "any case");
        assert_eq!(m.canonical(Path::new("/p/inc/../a.bas")), Some(PathBuf::from("/p/a.bas")));
        m.write(Path::new("/out/a.bas"), b"x").unwrap();
        assert_eq!(m.read(Path::new("/out/A.bas")).unwrap(), b"x");
        assert_eq!(m.written(), vec![(PathBuf::from("/out/a.bas"), b"x".to_vec())]);
        assert!(m.canonical(Path::new("/nothing")).is_none());
    }
}

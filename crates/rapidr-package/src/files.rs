//! An app's files in memory: what a build writes into a folder or packs
//! into an archive — each a path below the app's place (`/`-separated,
//! checked safe), its bytes, and whether it is a program (kept executable).

/// One file of an app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct File {
    /// `Name.app/Contents/Info.plist`, `staff.csv`: `/` between folders,
    /// never absolute, never `..`.
    pub path: String,
    pub bytes: Vec<u8>,
    /// A program or a script (Unix mode 0755).
    pub executable: bool,
}

/// An app's files, in the order they were added (an archive sorts them).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Files {
    list: Vec<File>,
}

/// `path` as a file of an app — or why it can't be one, in plain words:
/// absolute, a drive, `..`, a device's name, a control character
/// (rapidr-archive's [`rapidr_archive::safe_path`], the zip-slip guard).
pub fn safe(path: &str) -> Result<String, String> {
    rapidr_archive::safe_path(path).map(|parts| parts.join("/")).map_err(|e| {
        // (its words are an extractor's — `"x" goes up a folder (..): not
        // extracted (…)` —: said for a build)
        let shown = format!("\"{}\"", path.escape_debug());
        let why = e.strip_prefix(&shown).unwrap_or(&e).trim_start();
        let why = why.split(": not extracted").next().unwrap_or(why);
        format!("{shown} can't go in the app: it {why} (a file of the app is a path below its folder, like data/staff.csv)")
    })
}

impl Files {
    pub fn new() -> Files {
        Files::default()
    }

    /// Adds a file (one already at that path is replaced).
    pub fn add(&mut self, path: &str, bytes: impl Into<Vec<u8>>) -> Result<(), String> {
        self.put(path, bytes.into(), false)
    }

    /// Adds a program or a script (kept executable).
    pub fn add_executable(&mut self, path: &str, bytes: impl Into<Vec<u8>>) -> Result<(), String> {
        self.put(path, bytes.into(), true)
    }

    fn put(&mut self, path: &str, bytes: Vec<u8>, executable: bool) -> Result<(), String> {
        let path = safe(path)?;
        if let Some(clash) = self.list.iter().find(|f| f.path.starts_with(&format!("{path}/")) || path.starts_with(&format!("{}/", f.path))) {
            return Err(format!("\"{path}\" and \"{}\": a file can't also be a folder", clash.path));
        }
        self.list.retain(|f| f.path != path);
        self.list.push(File { path, bytes, executable });
        Ok(())
    }

    pub fn get(&self, path: &str) -> Option<&File> {
        self.list.iter().find(|f| f.path == path)
    }

    pub fn get_mut(&mut self, path: &str) -> Option<&mut File> {
        self.list.iter_mut().find(|f| f.path == path)
    }

    pub fn contains(&self, path: &str) -> bool {
        self.get(path).is_some()
    }

    pub fn remove(&mut self, path: &str) -> Option<File> {
        let i = self.list.iter().position(|f| f.path == path)?;
        Some(self.list.remove(i))
    }

    pub fn iter(&self) -> impl Iterator<Item = &File> {
        self.list.iter()
    }

    pub fn len(&self) -> usize {
        self.list.len()
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// Every byte of every file.
    pub fn size(&self) -> u64 {
        self.list.iter().map(|f| f.bytes.len() as u64).sum()
    }

    /// The files below folder `prefix` (`Name.app`), by their paths there.
    pub fn below(&self, prefix: &str) -> impl Iterator<Item = (&str, &File)> {
        let p = format!("{}/", prefix.trim_end_matches('/'));
        self.list.iter().filter_map(move |f| f.path.strip_prefix(p.as_str()).map(|rest| (rest, f)))
    }

    /// The files under another folder: `top/` before each path (`""`: as
    /// they are).
    pub fn moved_under(self, top: &str) -> Result<Files, String> {
        let top = top.trim_matches('/');
        if top.is_empty() {
            return Ok(self);
        }
        let mut out = Files::new();
        for f in self.list {
            out.put(&format!("{top}/{}", f.path), f.bytes, f.executable)?;
        }
        Ok(out)
    }

    /// Takes the files of `other` (replacing those at the same paths).
    pub fn extend(&mut self, other: Files) -> Result<(), String> {
        for f in other.list {
            self.put(&f.path, f.bytes, f.executable)?;
        }
        Ok(())
    }

    /// Sorted by path: the order an archive keeps them in (the same build,
    /// the same bytes).
    pub fn sorted(&self) -> Vec<&File> {
        let mut v: Vec<&File> = self.list.iter().collect();
        v.sort_by(|a, b| a.path.cmp(&b.path));
        v
    }
}

impl IntoIterator for Files {
    type Item = File;
    type IntoIter = std::vec::IntoIter<File>;
    fn into_iter(self) -> Self::IntoIter {
        self.list.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_are_checked_and_said_plainly() {
        let mut f = Files::new();
        f.add("data/staff.csv", b"a,b".to_vec()).unwrap();
        f.add("data\\more.csv", b"c".to_vec()).unwrap();
        assert!(f.contains("data/more.csv"), "backslashes are folders");
        for bad in ["../secret.txt", "/etc/passwd", "C:\\Windows\\x", "a/../../b", "con.txt", "a\u{7}b", ""] {
            let e = f.add(bad, Vec::new()).unwrap_err();
            assert!(e.contains("can't go in the app"), "{bad}: {e}");
            assert!(!e.contains("not extracted"), "{e}");
        }
        assert!(f.add("../x", Vec::new()).unwrap_err().contains(".."));
        // (a file and a folder of the same name)
        assert!(f.add("data", Vec::new()).unwrap_err().contains("folder"));
        assert!(f.add("data/staff.csv/x", Vec::new()).is_err());
        // (replaced, not twice)
        f.add("data/staff.csv", b"new".to_vec()).unwrap();
        assert_eq!(f.len(), 2);
        assert_eq!(f.get("data/staff.csv").unwrap().bytes, b"new");
    }

    #[test]
    fn moved_and_sorted() {
        let mut f = Files::new();
        f.add_executable("b/run", b"x".to_vec()).unwrap();
        f.add("a.txt", b"y".to_vec()).unwrap();
        let moved = f.moved_under("Top-1.0").unwrap();
        let names: Vec<&str> = moved.sorted().iter().map(|f| f.path.as_str()).collect();
        assert_eq!(names, ["Top-1.0/a.txt", "Top-1.0/b/run"]);
        assert!(moved.get("Top-1.0/b/run").unwrap().executable);
        assert_eq!(moved.below("Top-1.0").map(|(p, _)| p).collect::<Vec<_>>(), ["b/run", "a.txt"]);
        assert_eq!(moved.size(), 2);
    }
}

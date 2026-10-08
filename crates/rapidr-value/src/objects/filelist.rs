//! QFILELISTBOX (RapidQ manual, Appendix A): a list box of the files in a
//! directory ([`FileSource`] on the shared [`super::list::ItemList`], so the
//! runtimes draw it as a list box). `Directory` (the current directory at
//! first), `Mask` (`*.*`; several separated by `;`, case-insensitive as on
//! Windows), `AddFileTypes` / `DelFileTypes` (ftReadOnly = 0 … ftNormal = 6;
//! ftNormal alone at first), `Update`, `FileName` (the selected item's
//! path), `Drive`.
//!
//! Items as RapidQ's file list shows them (RC.EXE): the files, then the
//! directories (ftDirectory) in brackets with `[..]` and `[.]` among them,
//! each group in Windows' order ([`windows_order`]: case ignored, symbols
//! before digits before letters). Hidden files (a leading `.`) only with
//! ftHidden. A
//! directory that can't be read (or the web, which has no file system)
//! lists nothing.

pub const FT_READ_ONLY: u32 = 0;
pub const FT_HIDDEN: u32 = 1;
pub const FT_DIRECTORY: u32 = 4;
pub const FT_ARCHIVE: u32 = 5;
pub const FT_NORMAL: u32 = 6;

#[derive(Clone, Debug)]
pub struct FileSource {
    pub directory: String,
    pub mask: String,
    /// `1 << ftXxx` bits.
    pub types: u32,
}

impl Default for FileSource {
    fn default() -> Self {
        let directory = std::env::current_dir().map(|d| d.to_string_lossy().into_owned()).unwrap_or_default();
        FileSource { directory, mask: "*.*".into(), types: 1 << FT_NORMAL }
    }
}

/// Whether `name` matches a Windows wildcard (`*`, `?`), ignoring case;
/// `*.*` matches every name (with or without an extension).
pub fn wildcard_match(pattern: &str, name: &str) -> bool {
    let pattern = pattern.trim();
    if pattern.is_empty() || pattern == "*.*" || pattern == "*" {
        return true;
    }
    fn go(p: &[char], n: &[char]) -> bool {
        match (p.first(), n.first()) {
            (None, None) => true,
            (Some('*'), _) => go(&p[1..], n) || (!n.is_empty() && go(p, &n[1..])),
            (Some('?'), Some(_)) => go(&p[1..], &n[1..]),
            (Some(a), Some(b)) => a == b && go(&p[1..], &n[1..]),
            _ => false,
        }
    }
    let p: Vec<char> = pattern.to_lowercase().chars().collect();
    let n: Vec<char> = name.to_lowercase().chars().collect();
    go(&p, &n)
}

/// Two names in the order a Windows list box sorts them (RC.EXE's file
/// lists: `a.bas`, `z.txt`; `[..]`, `[.]`, `[_x]`, `[Adir]`, `[bdir]`):
/// case ignored, symbols before digits before letters.
pub fn windows_order(a: &str, b: &str) -> std::cmp::Ordering {
    let key = |s: &str| -> Vec<(u8, char)> {
        s.chars()
            .flat_map(char::to_lowercase)
            .map(|c| (if c.is_alphabetic() { 2 } else if c.is_ascii_digit() { 1 } else { 0 }, c))
            .collect()
    };
    key(a).cmp(&key(b)).then_with(|| a.cmp(b))
}

impl FileSource {
    /// The items for the directory as it is now.
    pub fn list(&self) -> Vec<String> {
        let mut dirs = Vec::new();
        let mut files = Vec::new();
        #[cfg(not(target_arch = "wasm32"))]
        let hidden_ok = self.types & (1 << FT_HIDDEN) != 0;
        #[cfg(not(target_arch = "wasm32"))]
        let files_ok = self.types & (1 << FT_NORMAL | 1 << FT_ARCHIVE | 1 << FT_READ_ONLY) != 0;
        #[cfg(not(target_arch = "wasm32"))]
        if let Ok(entries) = std::fs::read_dir(&self.directory) {
            for entry in entries.flatten().take(100_000) {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.starts_with('.') && !hidden_ok {
                    continue;
                }
                let is_dir = entry.file_type().is_ok_and(|t| t.is_dir()) || entry.path().is_dir();
                if is_dir {
                    dirs.push(name);
                } else if files_ok && self.mask.split(';').any(|m| wildcard_match(m, &name)) {
                    files.push(name);
                }
            }
        }
        files.sort_by(|a, b| windows_order(a, b));
        let mut items = Vec::with_capacity(dirs.len() + files.len() + 2);
        items.extend(files);
        if self.types & (1 << FT_DIRECTORY) != 0 {
            let mut shown: Vec<String> = dirs.into_iter().map(|d| format!("[{d}]")).collect();
            // (a drive's root has neither)
            if std::path::Path::new(&self.directory).parent().is_some() {
                shown.extend(["[..]".to_string(), "[.]".to_string()]);
            }
            shown.sort_by(|a, b| windows_order(a, b));
            items.extend(shown);
        }
        items
    }

    /// The path of an item (`[sub]` is the directory `sub`).
    pub fn path_of(&self, item: &str) -> String {
        let name = item.strip_prefix('[').and_then(|i| i.strip_suffix(']')).unwrap_or(item);
        std::path::Path::new(&self.directory).join(name).to_string_lossy().into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcards() {
        assert!(wildcard_match("*.EXE", "app.exe"));
        assert!(!wildcard_match("*.exe", "app.exe.txt"));
        assert!(wildcard_match("*.*", "README"));
        assert!(wildcard_match("a?c.*", "ABC.bas"));
        assert!(!wildcard_match("a?c", "ac"));
    }

    #[test]
    fn lists_a_directory() {
        let dir = std::env::temp_dir().join(format!("rapidr_filelist_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("Sub")).unwrap();
        for f in ["b.bas", "A.BAS", "c.txt", ".hidden.bas"] {
            std::fs::write(dir.join(f), "").unwrap();
        }
        let mut src = FileSource { directory: dir.to_string_lossy().into_owned(), mask: "*.bas".into(), types: 1 << FT_NORMAL };
        assert_eq!(src.list(), ["A.BAS", "b.bas"]);
        src.types |= 1 << FT_DIRECTORY | 1 << FT_HIDDEN;
        src.mask = "*.bas;*.txt".into();
        assert_eq!(src.list(), [".hidden.bas", "A.BAS", "b.bas", "c.txt", "[..]", "[.]", "[Sub]"]);
        // (RC.EXE's order)
        let mut names = ["[bdir]", "[Adir]", "[_x]", "[.]", "[..]"];
        names.sort_by(|a, b| windows_order(a, b));
        assert_eq!(names, ["[..]", "[.]", "[_x]", "[Adir]", "[bdir]"]);
        assert_eq!(src.path_of("[Sub]"), dir.join("Sub").to_string_lossy());
        let _ = std::fs::remove_dir_all(&dir);
    }
}

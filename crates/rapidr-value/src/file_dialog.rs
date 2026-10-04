//! File dialogs — QOPENDIALOG, QSAVEDIALOG and RAPIDQ2.INC's QFILEDIALOG —
//! the same on every runtime: RapidQ's `Filter` ("Pictures|*.bmp;*.ico|All
//! Files|*.*", `FilterIndex` counting from 1), and what the dialog answers:
//! `FileName` (with its folder), `FileTitle` (without), and for several
//! picked files `Files(0)` the folder, `Files(1…)` the names, `SelCount`.

/// One filter: its description and patterns (`*.bmp`, `*.ico`).
#[derive(Debug, Clone, PartialEq)]
pub struct Filter {
    pub name: String,
    pub patterns: Vec<String>,
}

/// RapidQ's `Filter` text: description|patterns pairs, patterns split by `;`.
pub fn parse_filter(text: &str) -> Vec<Filter> {
    let parts: Vec<&str> = text.split('|').collect();
    parts
        .chunks(2)
        .filter(|c| !c[0].trim().is_empty() || c.get(1).is_some_and(|p| !p.trim().is_empty()))
        .map(|c| {
            let pats = c.get(1).copied().unwrap_or(c[0]);
            Filter { name: c[0].trim().to_string(), patterns: pats.split(';').map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect() }
        })
        .collect()
}

/// The browser's `accept` for a file input: the patterns' extensions
/// (`.bmp,.ico`), or "" for any file.
pub fn html_accept(filters: &[Filter], index: usize) -> String {
    let Some(f) = filters.get(index).or_else(|| filters.first()) else { return String::new() };
    if f.patterns.iter().any(|p| p == "*.*" || p == "*") {
        return String::new();
    }
    f.patterns.iter().filter_map(|p| p.strip_prefix('*')).filter(|e| e.starts_with('.')).collect::<Vec<_>>().join(",")
}

/// Whether file `name` fits filter `index` (from 0; any file when there's
/// no such filter): `*`, `*.*`, `*.ext`, or an exact name.
pub fn fits(filters: &[Filter], index: usize, name: &str) -> bool {
    let Some(f) = filters.get(index).or_else(|| filters.first()) else { return true };
    let lower = name.to_ascii_lowercase();
    f.patterns.iter().any(|p| {
        let p = p.to_ascii_lowercase();
        p == "*" || p == "*.*" || p.strip_prefix('*').is_some_and(|ext| lower.ends_with(ext)) || p == lower
    })
}

/// `FileTitle`: the name without its folder.
pub fn file_title(path: &str) -> String {
    path.rsplit(['/', '\\']).next().unwrap_or("").to_string()
}

/// A saved file's name with `DefaultExt` added when it has no extension.
pub fn with_default_ext(path: &str, ext: &str) -> String {
    let ext = ext.trim().trim_start_matches('.');
    if ext.is_empty() || file_title(path).contains('.') {
        return path.to_string();
    }
    format!("{path}.{ext}")
}

/// What a dialog answered: `FileName`, `FileTitle`, `Files(…)`, `SelCount`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Picked {
    pub file_name: String,
    pub file_title: String,
    /// `Files(0)` the folder, then the names (several picked files).
    pub files: Vec<String>,
    pub sel_count: i64,
}

/// The answer for the files `paths` the user picked.
pub fn picked(paths: &[String]) -> Picked {
    let Some(first) = paths.first() else { return Picked::default() };
    let folder = match first.rfind(['/', '\\']) {
        Some(i) => first[..i].to_string(),
        None => String::new(),
    };
    let mut files = vec![folder];
    files.extend(paths.iter().map(|p| file_title(p)));
    Picked { file_name: first.clone(), file_title: file_title(first), files, sel_count: paths.len() as i64 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters() {
        let f = parse_filter("Picture files|*.BMP;*.ICO|All Files|*.*");
        assert_eq!(f.len(), 2);
        assert_eq!(f[0].patterns, vec!["*.BMP", "*.ICO"]);
        assert_eq!(html_accept(&f, 0), ".BMP,.ICO");
        assert_eq!(html_accept(&f, 1), "");
        assert!(parse_filter("").is_empty());
        assert!(fits(&f, 0, "a.ico") && !fits(&f, 0, "a.txt") && fits(&f, 1, "a.txt") && fits(&[], 0, "x"));
    }

    #[test]
    fn answers() {
        let p = picked(&["/home/me/a.txt".into(), "/home/me/b.txt".into()]);
        assert_eq!((p.file_name.as_str(), p.file_title.as_str(), p.sel_count), ("/home/me/a.txt", "a.txt", 2));
        assert_eq!(p.files, vec!["/home/me", "a.txt", "b.txt"]);
        assert_eq!(picked(&[]), Picked::default());
        assert_eq!(with_default_ext("/x/report", "txt"), "/x/report.txt");
        assert_eq!(with_default_ext("/x/report.csv", ".txt"), "/x/report.csv");
        assert_eq!(file_title("C:\\RapidQ\\test.txt"), "test.txt");
    }
}

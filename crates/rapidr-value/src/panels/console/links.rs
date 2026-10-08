//! The places in a console's text that are links: a file and a line, as
//! compilers and programs print them —
//!
//! - `Main.rr:12`, `src/app.bas:3:5: error: …` (GCC's, RapidR's own),
//! - `Main.rr(12)`, `C:\work\app.bas(3,5)` (RapidQ's and Microsoft's),
//!
//! a file being a name with an extension (letters or digits after a dot),
//! a path before it allowed (`/`, `\`, a drive's `C:`), so times
//! (`12:30:45`) and addresses (`localhost:8080`) aren't links.

/// A link in a line: where it is (characters, the end past its last) and
/// the place it names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    pub start: usize,
    pub end: usize,
    pub file: String,
    pub line: i64,
    pub col: i64,
}

fn is_path_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | '/' | '\\' | '~' | '$' | '+' | '@')
}

/// Whether `name` (a path) ends in a file name with an extension.
fn has_extension(name: &str) -> bool {
    let file = name.rsplit(['/', '\\']).next().unwrap_or(name);
    match file.rsplit_once('.') {
        Some((stem, ext)) => !stem.is_empty() && !ext.is_empty() && ext.len() <= 8 && ext.chars().all(|c| c.is_ascii_alphanumeric()) && ext.chars().any(|c| c.is_ascii_alphabetic()),
        None => false,
    }
}

/// Digits from `at` in `chars`: their value and where they end.
fn number(chars: &[char], at: usize) -> Option<(i64, usize)> {
    let mut j = at;
    while j < chars.len() && chars[j].is_ascii_digit() {
        j += 1;
    }
    if j == at || j - at > 9 {
        return None;
    }
    let n: String = chars[at..j].iter().collect();
    Some((n.parse().ok()?, j))
}

/// A screen line's links: its hyperlinks (OSC 8: `file` the URI, line 0),
/// and the places its text names outside them, in order.
pub fn of_line(line: &super::screen::Line) -> Vec<Link> {
    let mut out: Vec<Link> = line.links().iter().map(|(start, end, uri)| Link { start: *start, end: *end, file: uri.clone(), line: 0, col: 0 }).collect();
    let hyper = out.len();
    for l in find(line.text()) {
        if !out[..hyper].iter().any(|h| l.start < h.end && h.start < l.end) {
            out.push(l);
        }
    }
    out.sort_by_key(|l| l.start);
    out
}

/// The links in `text`, in order.
pub fn find(text: &str) -> Vec<Link> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if !is_path_char(chars[i]) || (i > 0 && is_path_char(chars[i - 1])) {
            i += 1;
            continue;
        }
        // (a path: its characters, a drive's `C:` allowed at its start)
        let start = i;
        let mut j = i;
        if j + 2 < chars.len() && chars[j].is_ascii_alphabetic() && chars[j + 1] == ':' && matches!(chars[j + 2], '\\' | '/') {
            j += 2;
        }
        while j < chars.len() && is_path_char(chars[j]) {
            j += 1;
        }
        // (a sentence's full stop isn't the file's)
        let mut name_end = j;
        while name_end > start && chars[name_end - 1] == '.' {
            name_end -= 1;
        }
        let name: String = chars[start..name_end].iter().collect();
        let found = if name_end == j && has_extension(&name) { place(&chars, j) } else { None };
        match found {
            Some((line, col, end)) if line > 0 => {
                out.push(Link { start, end, file: name, line, col });
                i = end;
            }
            _ => i = j.max(i + 1),
        }
    }
    out
}

/// `:line[:col]` or `(line[,col])` from `at`: line, column (0 without one)
/// and where it ends.
fn place(chars: &[char], at: usize) -> Option<(i64, i64, usize)> {
    match chars.get(at)? {
        ':' => {
            let (line, j) = number(chars, at + 1)?;
            if chars.get(j) == Some(&':') {
                if let Some((col, k)) = number(chars, j + 1) {
                    return Some((line, col, k));
                }
            }
            Some((line, 0, j))
        }
        '(' => {
            let (line, j) = number(chars, at + 1)?;
            match chars.get(j)? {
                ')' => Some((line, 0, j + 1)),
                ',' => {
                    let (col, k) = number(chars, j + 1)?;
                    (chars.get(k) == Some(&')')).then_some((line, col, k + 1))
                }
                _ => None,
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn places(text: &str) -> Vec<(String, i64, i64, String)> {
        find(text).into_iter().map(|l| (l.file, l.line, l.col, text.chars().skip(l.start).take(l.end - l.start).collect())).collect()
    }

    #[test]
    fn compilers_places_are_links() {
        assert_eq!(places("see Main.rr:12 for it"), vec![("Main.rr".into(), 12, 0, "Main.rr:12".into())]);
        assert_eq!(places("src/app.bas:3:5: error: x"), vec![("src/app.bas".into(), 3, 5, "src/app.bas:3:5".into())]);
        assert_eq!(places("Main.rr(12) Error"), vec![("Main.rr".into(), 12, 0, "Main.rr(12)".into())]);
        assert_eq!(places(r"C:\work\app.bas(3,5): warning"), vec![(r"C:\work\app.bas".into(), 3, 5, r"C:\work\app.bas(3,5)".into())]);
        assert_eq!(places("a.bas:1 and b.inc:20:2."), vec![("a.bas".into(), 1, 0, "a.bas:1".into()), ("b.inc".into(), 20, 2, "b.inc:20:2".into())]);
    }

    #[test]
    fn times_and_addresses_are_not() {
        assert!(find("12:30:45").is_empty());
        assert!(find("http://localhost:8080/x").is_empty());
        assert!(find("Main.rr: 12").is_empty());
        assert!(find("ratio 1.5:2").is_empty());
        assert!(find("Main.rr:0").is_empty());
        assert!(find("v1.2:3").is_empty(), "an extension of digits only");
    }
}

//! Find in Files (docs/studio-wow.md CMD-3), RPROJECT's `Find`: one text
//! searched with the code editor's own search (rapidr-editor: literal or
//! regular expression, case, whole word), its matches as lines a program
//! reads — line, column (from 1, in characters), length (characters), the
//! line's text; and `Replace`, the text with every match replaced (`$1`,
//! `${name}` in a regular expression's replacement). Studio runs them on
//! each project file's text (an open document's as its editor has it).

use rapidr_editor::search::{SearchQuery, Searcher};

/// At most this many matches a text (a search for "e" in a big file).
pub const MAX_MATCHES: usize = 2000;

/// The matches of `pattern` in `text`: `line\tcol\tlength\tline text` a
/// line; `Err`: the pattern can't be compiled (why).
pub fn find(text: &str, pattern: &str, options: &str) -> Result<String, String> {
    let s = Searcher::new(&SearchQuery::with_options(pattern, options)).map_err(|e| e.0)?;
    let mut out = String::new();
    // (line starts, to turn byte offsets into lines and columns)
    let starts: Vec<usize> = std::iter::once(0).chain(text.match_indices('\n').map(|(i, _)| i + 1)).collect();
    for m in s.find_all(text, 0..text.len()).into_iter().take(MAX_MATCHES) {
        let line = starts.partition_point(|&st| st <= m.start) - 1;
        let ls = starts[line];
        let le = starts.get(line + 1).map_or(text.len(), |e| e - 1);
        let line_text = text[ls..le].trim_end_matches('\r');
        let col = text[ls..m.start].chars().count() + 1;
        let len = text[m.start..m.end].chars().count();
        out.push_str(&format!("{}\t{col}\t{len}\t{}\n", line + 1, line_text.replace('\t', "    ")));
    }
    Ok(out)
}

/// `text` with every match of `pattern` replaced by `with`, and how many.
pub fn replace(text: &str, pattern: &str, options: &str, with: &str) -> Result<(String, usize), String> {
    let s = Searcher::new(&SearchQuery::with_options(pattern, options)).map_err(|e| e.0)?;
    let all = s.find_all(text, 0..text.len());
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    for m in &all {
        out.push_str(&text[at..m.start]);
        out.push_str(&s.replacement(text, m, with));
        at = m.end;
    }
    out.push_str(&text[at..]);
    Ok((out, all.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_come_back_as_lines_and_columns() {
        let text = "DIM x AS INTEGER\r\nx = 1: PRINT x\n' the end, x\n";
        let r = find(text, "x", "word").unwrap();
        assert_eq!(r.lines().collect::<Vec<_>>(), ["1\t5\t1\tDIM x AS INTEGER", "2\t1\t1\tx = 1: PRINT x", "2\t14\t1\tx = 1: PRINT x", "3\t12\t1\t' the end, x"]);
        assert_eq!(find(text, "PRINT", "case").unwrap().lines().count(), 1);
        assert_eq!(find(text, "print", "case").unwrap(), "");
        assert_eq!(find(text, "print", "").unwrap().lines().count(), 1);
        assert_eq!(find(text, r"x = \d", "regex").unwrap(), "2\t1\t5\tx = 1: PRINT x\n");
        assert!(find(text, "(", "regex").is_err());
        assert!(find(text, "", "").is_err());
        // (columns in characters)
        assert_eq!(find("é = \"ñ\"", "ñ", "").unwrap(), "1\t6\t1\té = \"ñ\"\n");
    }

    #[test]
    fn replace_every_match() {
        assert_eq!(replace("a b a", "a", "word", "x").unwrap(), ("x b x".to_string(), 2));
        assert_eq!(replace("Left = 10: Top = 20", r"(\w+) = (\d+)", "regex", "$1=$2").unwrap().0, "Left=10: Top=20");
    }
}

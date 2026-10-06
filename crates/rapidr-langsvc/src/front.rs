//! One parse of a program for the service: the compiler's preprocessor,
//! lexer and parser, never stopping at an error, and the map from the
//! AST's spans (bytes of the preprocessed text) back to the files.
//!
//! **Interim seam.** This is what `rapidr_parser::tools` (lane L-PARSE:
//! `parse_source_for_tools`, `ToolsParse::locate` / `to_preprocessed`, the
//! byte-level origin map, lexer recovery) provides; until it lands, this
//! module recovers by blanking the line in error and maps positions line by
//! line through the preprocessor's line map (exact for every line the
//! preprocessor copies; best effort on a line a `$DEFINE` or `$MACRO`
//! rewrote). When it lands, [`parse`] calls `parse_source_for_tools` and
//! [`Parsed::locate`] / [`Parsed::to_preprocessed`] forward to it.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rapidr_ast::Program;
use rapidr_diagnostics::TextSpan;
use rapidr_lexer::{Lexer, Token};
use rapidr_preprocessor::{preprocess_source, LineOrigin, PreprocessOptions};

use crate::text::LineIndex;

/// A place in a file: bytes of its text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    pub file: PathBuf,
    pub start: usize,
    pub end: usize,
}

/// One parse of a program.
#[derive(Debug, Clone)]
pub struct Parsed {
    /// The program's main file.
    pub root: PathBuf,
    /// The preprocessed text the AST's spans count (lines the service had
    /// to blank to recover are blank here).
    pub source: String,
    pub line_map: Vec<LineOrigin>,
    pre_lines: LineIndex,
    pub tokens: Vec<Token>,
    pub program: Program,
    /// The text of each file the program was made of (the main file's is
    /// the editor's), with its line index.
    pub files: HashMap<PathBuf, (String, LineIndex)>,
}

/// Parses `text`, the content of `path`, recovering from errors.
/// `overlay` gives the editor's text of other files (an `$INCLUDE`d file
/// open in the editor) for mapping positions; the preprocessor itself reads
/// included files from the disk (L-PARSE adds overlays).
pub fn parse(path: &Path, text: &str, options: &PreprocessOptions, overlay: &dyn Fn(&Path) -> Option<String>) -> Parsed {
    let base = path.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));
    let mut root_text = text.to_string();
    // The preprocessor stops at its first error (a missing $INCLUDE …):
    // blank the line it names and go again.
    let mut pre = None;
    for _ in 0..64 {
        match preprocess_source(&root_text, &base, Some(path.to_path_buf()), options.clone()) {
            Ok(p) => {
                pre = Some(p);
                break;
            }
            Err(e) => {
                let d = &e.diagnostic;
                let in_root = d.file_path.as_deref().is_none_or(|f| same_file(Path::new(f), path));
                let line = if in_root { d.location.line } else { include_line_of(&root_text, d.file_path.as_deref().unwrap_or("")).unwrap_or(0) };
                if line == 0 || !blank_line(&mut root_text, line) {
                    break;
                }
            }
        }
    }
    let (mut source, line_map) = match pre {
        Some(p) => (p.source, p.line_map),
        None => {
            // (unrecoverable: the text itself, line for line)
            let n = root_text.split('\n').count();
            (root_text.clone(), (1..=n).map(|l| (Some(path.to_path_buf()), l)).collect())
        }
    };
    // The lexer stops at its first error too: blank that line, again.
    let label = Some(path.display().to_string());
    let mut tokens = Vec::new();
    for _ in 0..256 {
        match Lexer::new(&source, label.clone()).tokenize() {
            Ok(t) => {
                tokens = t;
                break;
            }
            Err(e) => {
                if !blank_line(&mut source, e.diagnostic.location.line) {
                    break;
                }
            }
        }
    }
    let (program, _) = rapidr_parser::parse_tokens_recovering(&tokens);

    let mut files = HashMap::new();
    files.insert(path.to_path_buf(), (text.to_string(), LineIndex::new(text)));
    for (file, _) in &line_map {
        let Some(file) = file else { continue };
        if files.contains_key(file) || file.starts_with("<RapidR>") {
            continue;
        }
        let content = overlay(file).or_else(|| rapidr_preprocessor::read_source(file).ok());
        if let Some(content) = content {
            let index = LineIndex::new(&content);
            files.insert(file.clone(), (content, index));
        }
    }
    let pre_lines = LineIndex::new(&source);
    Parsed { root: path.to_path_buf(), source, line_map, pre_lines, tokens, program, files }
}

impl Parsed {
    /// Where a span of the AST is in the program's files.
    pub fn locate(&self, span: TextSpan) -> Option<Location> {
        let (line, col) = self.pre_lines.line_col(span.start);
        let (file, file_line) = self.line_map.get(line)?;
        let file = file.as_ref()?;
        let (text, index) = self.files.get(file)?;
        let pre_line = self.pre_lines.line_text(&self.source, line);
        let file_line_index = file_line.checked_sub(1)?;
        let line_start = index.line_start(file_line_index)?;
        let orig_line = index.line_text(text, file_line_index);
        let start = line_start + map_column(pre_line, orig_line, col);
        // (a span on one line maps its end the same way; a longer one keeps
        // its length)
        let (end_line, end_col) = self.pre_lines.line_col(span.end);
        let end = if end_line == line {
            line_start + map_column(pre_line, orig_line, end_col).max(start - line_start)
        } else {
            start + span.len()
        };
        Some(Location { file: file.clone(), start, end: end.min(text.len()) })
    }

    /// The preprocessed offset of byte `offset` of `file` (e.g. the caret).
    pub fn to_preprocessed(&self, file: &Path, offset: usize) -> Option<usize> {
        let key = self.file_key(file)?;
        let (text, index) = &self.files[key];
        let (line, col) = index.line_col(offset.min(text.len()));
        let orig_line = index.line_text(text, line);
        let pre_line = self.line_map.iter().position(|(f, l)| f.as_deref() == Some(key) && *l == line + 1)?;
        let pre_text = self.pre_lines.line_text(&self.source, pre_line);
        Some(self.pre_lines.line_start(pre_line)? + map_column(orig_line, pre_text, col))
    }

    /// The name this parse knows `file` by (as the preprocessor wrote it).
    pub fn file_key(&self, file: &Path) -> Option<&Path> {
        if let Some((k, _)) = self.files.get_key_value(file) {
            return Some(k);
        }
        self.files.keys().find(|f| same_file(f, file)).map(PathBuf::as_path)
    }

    /// The file and line (0-based) a preprocessed line came from.
    pub fn origin_of_line(&self, pre_line: usize) -> Option<(&Path, usize)> {
        let (file, line) = self.line_map.get(pre_line)?;
        Some((file.as_deref()?, line.checked_sub(1)?))
    }

    /// The preprocessed line (0-based) of a byte offset of the source.
    pub fn pre_line_of(&self, offset: usize) -> usize {
        self.pre_lines.line_col(offset).0
    }

    /// The text of a file of the program (the editor's, for the main file).
    pub fn file_text(&self, file: &Path) -> Option<&str> {
        self.files.get(file).map(|(t, _)| t.as_str())
    }

    /// The text of the AST's span.
    pub fn span_text(&self, span: TextSpan) -> &str {
        self.source.get(span.start..span.end).unwrap_or("")
    }
}

/// A column of `from` (bytes) on the corresponding position of `to`, for a
/// line the preprocessor may have rewritten: the same column while the two
/// agree, counted from the end after their last difference.
pub(crate) fn map_column(from: &str, to: &str, col: usize) -> usize {
    if from == to {
        return col.min(to.len());
    }
    let prefix = from.bytes().zip(to.bytes()).take_while(|(a, b)| a == b).count();
    if col <= prefix {
        return col;
    }
    let suffix = from.bytes().rev().zip(to.bytes().rev()).take_while(|(a, b)| a == b).count().min(from.len() - prefix).min(to.len() - prefix);
    if from.len() - col <= suffix {
        return to.len() - (from.len() - col);
    }
    prefix.min(to.len())
}

/// Replaces line `line` (1-based) of `text` by spaces; false if there's no
/// such line or it's blank already.
fn blank_line(text: &mut String, line: usize) -> bool {
    let Some(start) = (if line <= 1 { Some(0) } else { text.match_indices('\n').nth(line - 2).map(|(i, _)| i + 1) }) else { return false };
    let end = text[start..].find('\n').map_or(text.len(), |i| start + i);
    if text[start..end].trim().is_empty() {
        return false;
    }
    let blank: String = text[start..end].chars().map(|c| if c == '\r' { '\r' } else { ' ' }).collect();
    text.replace_range(start..end, &blank);
    true
}

/// The line (1-based) of `$INCLUDE` naming `file` in `text`.
fn include_line_of(text: &str, file: &str) -> Option<usize> {
    let name = Path::new(file).file_name()?.to_string_lossy().to_ascii_lowercase();
    text.lines().position(|l| {
        let l = l.trim_start().to_ascii_lowercase();
        l.starts_with("$include") && l.contains(&name)
    })
    .map(|i| i + 1)
}

/// Whether two paths name the same file (as written, or once resolved).
pub fn same_file(a: &Path, b: &Path) -> bool {
    a == b || matches!((a.canonicalize(), b.canonicalize()), (Ok(x), Ok(y)) if x == y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_map_through_a_rewritten_line() {
        assert_eq!(map_column("x = 10", "x = 10", 4), 4);
        // `$DEFINE TEN 10`: `x = TEN + y` became `x = 10 + y`
        assert_eq!(map_column("x = 10 + y", "x = TEN + y", 9), 10);
        assert_eq!(map_column("x = 10 + y", "x = TEN + y", 0), 0);
    }

    #[test]
    fn a_blanked_line_keeps_the_others_in_place() {
        let mut t = "a\n\"oops\nc".to_string();
        assert!(blank_line(&mut t, 2));
        assert_eq!(t, "a\n     \nc");
        assert!(!blank_line(&mut t, 2));
    }
}

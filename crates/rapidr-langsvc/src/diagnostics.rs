//! Diagnostics: the compiler's own, with the compiler's messages and
//! places (RapidQ's wording: `.reference/rapidq-compiler-messages.txt`).
//!
//! They come from the very pipeline `rapidr build-bc` runs
//! (`compile_to_bytecode` in crates/rapidr-cli: preprocessor → lexer →
//! parser → bytecode compiler, which stops at the first stage that fails),
//! run on the editor's text, so an editor shows exactly what a build
//! reports. The bytecode compiler reports its errors as text
//! (`LINE:COL: error: message`, lines of the preprocessed program); they
//! are read in one place, [`compiler_errors`], until it returns structured
//! diagnostics (a small change in rapidr-bcgen: its `errors` as
//! `Diagnostic`s with spans).

use std::path::{Path, PathBuf};

use rapidr_diagnostics::Severity;
use rapidr_preprocessor::{preprocess_source, PreprocessOptions, PreprocessResult};

use crate::text::{is_name_char, LineIndex};

/// A diagnostic in a file: bytes of its text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDiagnostic {
    pub file: PathBuf,
    pub start: usize,
    pub end: usize,
    pub severity: Severity,
    pub message: String,
    /// A code for code actions (`rapidq-compat` …).
    pub code: Option<String>,
}

/// The options the CLI's compiler uses (`preprocess_file` adds
/// `RAPIDR_INCLUDE_PATH`'s folders).
pub fn compiler_options(base: &PreprocessOptions) -> PreprocessOptions {
    let mut options = base.clone();
    if let Some(paths) = std::env::var_os("RAPIDR_INCLUDE_PATH") {
        options.include_dirs.extend(std::env::split_paths(&paths));
    }
    options
}

/// What compiling `text` (the content of `path`) reports, as a build
/// would. `file_text` gives the text of other files (editor or disk).
pub fn compile(path: &Path, text: &str, options: &PreprocessOptions, file_text: &dyn Fn(&Path) -> Option<String>) -> Vec<FileDiagnostic> {
    let base = path.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));
    let options = compiler_options(options);
    let label = path.display().to_string();
    let texts = Texts { root: path, root_text: text, file_text };
    let pre = match preprocess_source(text, &base, Some(path.to_path_buf()), options) {
        Ok(p) => p,
        Err(e) => {
            let d = &e.diagnostic;
            let file = d.file_path.as_deref().map(PathBuf::from).unwrap_or_else(|| path.to_path_buf());
            return texts.at_line_col(&file, d.location.line, d.location.column, d.severity, &d.message).into_iter().collect();
        }
    };
    let pre_lines = LineIndex::new(&pre.source);
    let tokens = match rapidr_lexer::Lexer::new(&pre.source, Some(label.clone())).tokenize() {
        Ok(t) => t,
        Err(e) => {
            let d = &e.diagnostic;
            return texts.at_pre(&pre, &pre_lines, d.span.start, d.span.end.max(d.span.start), d.severity, &d.message).into_iter().collect();
        }
    };
    let program = match rapidr_parser::parse_tokens(&tokens) {
        Ok(p) => p,
        Err(e) => {
            return e.diagnostics.iter().filter_map(|d| texts.at_pre(&pre, &pre_lines, d.span.start, d.span.end, d.severity, &d.message)).collect();
        }
    };
    let library_lines: Vec<bool> = pre.line_map.iter().map(|(file, _)| file.as_deref().is_some_and(|f| f != path)).collect();
    match rapidr_bcgen::compile_program_with_libraries(&program, Some(&pre.source), &library_lines) {
        Ok(compiled) => compiled
            .warnings
            .iter()
            .filter_map(|w| {
                let (line, col, message) = compiler_errors(w)?;
                texts.at_pre_line_col(&pre, &pre_lines, line, col, Severity::Warning, &message)
            })
            .collect(),
        Err(errors) => errors
            .lines()
            .filter_map(|e| {
                let (line, col, message) = compiler_errors(e)?;
                texts.at_pre_line_col(&pre, &pre_lines, line, col, Severity::Error, &message)
            })
            .collect(),
    }
}

/// One line of the bytecode compiler's errors: (line, column, message) —
/// `LINE:COL: error: message` (line 1 when it names no place).
pub fn compiler_errors(line: &str) -> Option<(usize, usize, String)> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    let mut parts = line.splitn(3, ':');
    let (a, b) = (parts.next()?, parts.next());
    if let (Ok(l), Some(Ok(c))) = (a.trim().parse::<usize>(), b.map(|b| b.trim().parse::<usize>())) {
        let rest = parts.next().unwrap_or("").trim();
        let message = rest.strip_prefix("error:").or_else(|| rest.strip_prefix("warning:")).unwrap_or(rest).trim();
        return Some((l, c, message.to_string()));
    }
    let message = line.strip_prefix("error:").or_else(|| line.strip_prefix("warning:")).unwrap_or(line).trim();
    Some((1, 1, message.to_string()))
}

struct Texts<'a> {
    root: &'a Path,
    root_text: &'a str,
    file_text: &'a dyn Fn(&Path) -> Option<String>,
}

impl Texts<'_> {
    fn text(&self, file: &Path) -> Option<String> {
        if file == self.root {
            return Some(self.root_text.to_string());
        }
        (self.file_text)(file).or_else(|| rapidr_preprocessor::read_source(file).ok())
    }

    /// A diagnostic at a 1-based line and column of a file, over the name
    /// the message is about (else the word there).
    fn at_line_col(&self, file: &Path, line: usize, col: usize, severity: Severity, message: &str) -> Option<FileDiagnostic> {
        let text = self.text(file).unwrap_or_default();
        let index = LineIndex::new(&text);
        let l = line.saturating_sub(1).min(index.line_count().saturating_sub(1));
        let start_of_line = index.line_start(l).unwrap_or(0);
        let line_text = index.line_text(&text, l);
        let col0 = col.saturating_sub(1).min(line_text.len());
        // (it starts where the compiler says, and runs over the name the
        // message is about)
        let (_, e) = focus(line_text, col0, message);
        let s = col0;
        let e = e.max(s);
        Some(FileDiagnostic { file: file.to_path_buf(), start: start_of_line + s, end: start_of_line + e, severity, message: message.to_string(), code: None })
    }

    /// A diagnostic at bytes of the preprocessed text.
    fn at_pre(&self, pre: &PreprocessResult, lines: &LineIndex, start: usize, end: usize, severity: Severity, message: &str) -> Option<FileDiagnostic> {
        let (line, col) = lines.line_col(start);
        let (end_line, end_col) = lines.line_col(end);
        let mut d = self.at_pre_line_col(pre, lines, line + 1, col + 1, severity, message)?;
        if end_line == line && end > start {
            // (the span as the parser gave it)
            let pre_line = lines.line_text(&pre.source, line);
            let text = self.text(&d.file).unwrap_or_default();
            let index = LineIndex::new(&text);
            let (fl, _) = index.line_col(d.start);
            let orig = index.line_text(&text, fl);
            let ls = index.line_start(fl).unwrap_or(0);
            d.start = ls + crate::front::map_column(pre_line, orig, col);
            d.end = (ls + crate::front::map_column(pre_line, orig, end_col)).max(d.start);
        }
        Some(d)
    }

    /// A diagnostic at a 1-based line and column of the preprocessed text.
    fn at_pre_line_col(&self, pre: &PreprocessResult, lines: &LineIndex, line: usize, col: usize, severity: Severity, message: &str) -> Option<FileDiagnostic> {
        let Some((file, file_line)) = pre.line_map.get(line.saturating_sub(1)) else {
            return self.at_line_col(self.root, line, col, severity, message);
        };
        let file = file.clone().unwrap_or_else(|| self.root.to_path_buf());
        if file.starts_with("<RapidR>") {
            return None;
        }
        let pre_line = lines.line_text(&pre.source, line.saturating_sub(1));
        let text = self.text(&file).unwrap_or_default();
        let index = LineIndex::new(&text);
        let orig = index.line_text(&text, file_line.saturating_sub(1));
        let col = crate::front::map_column(pre_line, orig, col.saturating_sub(1)) + 1;
        self.at_line_col(&file, *file_line, col, severity, message)
    }
}

/// The part of a line a message at column `col` is about: the name it
/// quotes or mentions, at or after the column; else the word there; else
/// the rest of the line. (The diagnostic runs from `col` to its end.)
fn focus(line: &str, col: usize, message: &str) -> (usize, usize) {
    let lower = line.to_ascii_lowercase();
    let mut candidates: Vec<String> = Vec::new();
    for quoted in message.split('\'').skip(1).step_by(2) {
        candidates.push(quoted.to_string());
    }
    for word in message.split(|c: char| !(is_name_char(c) || c == '.' || c == '$')) {
        let w = word.trim_matches('.');
        if !w.is_empty() && !COMMON.contains(&w.to_ascii_lowercase().as_str()) {
            candidates.push(w.to_string());
            if let Some((_, last)) = w.rsplit_once('.') {
                candidates.push(last.to_string());
            }
        }
    }
    for c in candidates {
        let c = c.to_ascii_lowercase();
        if c.is_empty() {
            continue;
        }
        let mut from = col.min(lower.len());
        while let Some(i) = lower[from..].find(&c) {
            let at = from + i;
            let end = at + c.len();
            let before = at == 0 || !is_name_char(lower.as_bytes()[at - 1] as char);
            let after = end >= lower.len() || !is_name_char(lower.as_bytes()[end] as char);
            if before && after {
                return (at, end);
            }
            from = end;
        }
    }
    if let Some((s, e)) = crate::text::word_at(line, col).filter(|(s, _)| *s >= col.saturating_sub(0)) {
        return (s, e);
    }
    let trimmed_end = line.trim_end().len();
    if col < trimmed_end {
        (col, trimmed_end)
    } else {
        (col.min(line.len()), line.len())
    }
}

/// Words of messages that aren't the subject.
const COMMON: &[&str] = &[
    "error", "unknown", "sub", "or", "function", "undeclared", "identifier", "member", "not", "part", "of", "class", "is", "a", "an", "the", "read",
    "only", "value", "type", "mismatch", "expecting", "but", "got", "label", "found", "in", "main", "program", "already", "used", "try", "another",
    "name", "too", "many", "few", "actual", "parameters", "for", "trying", "to", "assign", "return", "while", "array", "supported", "component",
    "assignment", "yet", "datatype", "struct", "property", "reserved", "word", "expected", "integer", "string", "double", "single", "long",
    "missing", "end", "if", "then", "with", "and", "at", "line", "statement", "cannot", "can", "be", "invalid", "syntax", "unexpected", "token",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiler_lines_are_read() {
        assert_eq!(compiler_errors("3:1: error: Unknown SUB or FUNCTION 'X'"), Some((3, 1, "Unknown SUB or FUNCTION 'X'".into())));
        assert_eq!(compiler_errors("error: something"), Some((1, 1, "something".into())));
    }

    #[test]
    fn the_name_a_message_is_about_is_underlined() {
        let line = "Pair$(1, 2, 3)";
        assert_eq!(focus(line, 0, "Too many actual parameters for Pair$"), (0, 5));
        assert_eq!(focus("PRINT b + 1", 0, "Undeclared identifier b"), (6, 7));
        assert_eq!(focus("G.Caption = \"x\"", 0, "Member CAPTION not part of class G"), (2, 9));
    }
}

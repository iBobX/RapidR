//! Diagnostics: the compiler's own, with the compiler's messages and
//! places (RapidQ's wording: `.reference/rapidq-compiler-messages.txt`).
//!
//! They come from the stages `rapidr build-bc` runs (preprocessor → lexer
//! → parser → bytecode compiler), reporting what the first stage that
//! fails reports — as a build does — on the editor's text (the program's
//! open files included, saved or not). Every stage gives structured
//! diagnostics with spans; the parser for tools' origin map puts them in
//! their files.

use std::path::{Path, PathBuf};

use rapidr_diagnostics::{Diagnostic, Severity};
use rapidr_preprocessor::PreprocessOptions;

use crate::front::Parsed;
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

/// What compiling the parsed program reports, as a build would.
pub fn compile(parsed: &Parsed) -> Vec<FileDiagnostic> {
    let tools = &parsed.tools;
    // 1. The preprocessor stops at its first error.
    if tools.preprocessor_diagnostics > 0 {
        let (location, d) = &tools.diagnostic_locations()[0];
        let at = location.as_ref().and_then(|l| Some((l.path.clone()?, l.start)));
        let (file, start) = at.unwrap_or_else(|| (parsed.root.clone(), 0));
        return place(parsed, &file, start, None, d).into_iter().collect();
    }
    let source = &tools.preprocessed.source;
    // 2. So does the lexer.
    if let Err(e) = rapidr_lexer::Lexer::new(source, None).tokenize() {
        return at_pre(parsed, &e.diagnostic).into_iter().collect();
    }
    // 3. The parser reports every statement it couldn't parse.
    let parse: Vec<&Diagnostic> = tools.diagnostics[tools.preprocessor_diagnostics..].iter().collect();
    if parse.iter().any(|d| d.severity == Severity::Error) {
        return parse.into_iter().filter_map(|d| at_pre(parsed, d)).collect();
    }
    // 4. The bytecode compiler, on the same program and text.
    let library_lines: Vec<bool> = tools.preprocessed.line_map.iter().map(|(file, _)| file.as_deref().is_some_and(|f| f != parsed.root)).collect();
    match rapidr_bcgen::compile_program_diagnostics(&parsed.program, Some(source), &library_lines) {
        Ok(_) => missing_icons(parsed),
        Err(errors) => errors.iter().filter_map(|d| at_pre(parsed, d)).collect(),
    }
}

/// 5. `$OPTION ICON` files that aren't there: RC.EXE's `ICON file x does
/// not exist.` at the directive, as the build reports it after compiling.
/// (Not on the web: there the project's assets hold the icon, checked when
/// the program is built.)
fn missing_icons(parsed: &Parsed) -> Vec<FileDiagnostic> {
    if cfg!(target_arch = "wasm32") {
        return Vec::new();
    }
    let resources = &parsed.tools.preprocessed.resources;
    resources
        .iter()
        .filter(|r| r.path.is_none())
        .filter_map(|r| {
            let (file, line) = r.icon_directive.as_ref()?;
            let file = PathBuf::from(file);
            let text = parsed.file_text(&file)?;
            let index = LineIndex::new(text);
            let start = index.line_start(line.saturating_sub(1)).unwrap_or(0);
            let end = start + index.line_text(text, line.saturating_sub(1)).len();
            Some(FileDiagnostic { file, start, end: end.max(start), severity: Severity::Error, message: format!("ICON file {} does not exist.", r.file), code: None })
        })
        .collect()
}

/// A diagnostic whose span counts bytes of the preprocessed text.
fn at_pre(parsed: &Parsed, d: &Diagnostic) -> Option<FileDiagnostic> {
    // (generated text — a $DEFINE's value — maps to the source it replaced)
    let Some(l) = parsed.tools.locate(d.span) else {
        return place(parsed, &parsed.root, 0, None, d);
    };
    let file = l.path?;
    if file.starts_with("<RapidR>") {
        return None;
    }
    let end = (l.exact && l.end > l.start).then_some(l.end);
    place(parsed, &file, l.start, end, d)
}

/// The diagnostic at byte `start` of `file`: from there over the name the
/// message is about (else the word there, else the rest of the line), not
/// past the compiler's own span when it gives one.
fn place(parsed: &Parsed, file: &Path, start: usize, end: Option<usize>, d: &Diagnostic) -> Option<FileDiagnostic> {
    let text = parsed.file_text(file).unwrap_or("");
    let start = start.min(text.len());
    let index = LineIndex::new(text);
    let (line, col) = index.line_col(start);
    let line_start = index.line_start(line).unwrap_or(0);
    let line_text = index.line_text(text, line);
    let (_, e) = focus(line_text, col.min(line_text.len()), &d.message);
    let mut end_at = line_start + e.max(col.min(line_text.len()));
    if let Some(span_end) = end.filter(|&x| x > start) {
        end_at = end_at.min(span_end.max(start + 1));
    }
    // (never empty on a line's text: the word focused may end at the place
    // — `Form.`'s dot — and then the character there is underlined)
    if end_at <= start {
        let line_end = line_start + line_text.len();
        end_at = (start + text[start..].chars().next().map_or(0, char::len_utf8)).min(line_end).max(start);
    }
    Some(FileDiagnostic { file: file.to_path_buf(), start, end: end_at, severity: d.severity, message: d.message.clone(), code: None })
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
        if c.is_empty() || !lower.is_char_boundary(col.min(lower.len())) {
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
    // (a word that ends at the column — `Form` before `Form.`'s dot — isn't
    // what the message at the dot is about)
    if let Some((s, e)) = crate::text::word_at(line, col).filter(|&(_, e)| e > col) {
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
    fn the_name_a_message_is_about_is_underlined() {
        let line = "Pair$(1, 2, 3)";
        assert_eq!(focus(line, 0, "Too many actual parameters for Pair$"), (0, 5));
        assert_eq!(focus("PRINT b + 1", 0, "Undeclared identifier b"), (6, 7));
        assert_eq!(focus("G.Caption = \"x\"", 0, "Member CAPTION not part of class G"), (2, 9));
        assert_eq!(focus("Form.", 4, "Member  not part of class FORM"), (4, 5));
    }
}

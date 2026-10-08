//! The parser for tools (docs/ide-plan.md, I0): everything an editor, the
//! designer or the language service needs from one parse of a program.
//!
//! [`parse_file_for_tools`] runs the compiler's own pipeline — preprocessor,
//! lexer, parser, so the [`Program`] is the one the backends compile — but
//! never stops at an error: a missing `$INCLUDE` is reported and skipped,
//! unreadable text becomes an error token, an unparseable line a
//! diagnostic. On top of the AST it gives
//!
//! * the origin map ([`ToolsParse::locate`]): any span of the AST (bytes of
//!   the preprocessed text) → (file, byte range) in the files the user
//!   edits, through `$INCLUDE`, `$DEFINE` and `$MACRO`;
//! * every file of the program losslessly ([`ToolsParse::files`]): tokens
//!   plus trivia (comments, blank lines, directives, inactive `$IFDEF`
//!   branches) that print the file back byte for byte, classified with the
//!   preprocessor's real `$IFDEF` decisions for this build.
//!
//! The AST is unchanged: trivia live in the side table only.

use std::path::{Path, PathBuf};

use rapidr_ast::Program;
use rapidr_diagnostics::{Diagnostic, TextSpan};
use rapidr_lexer::{Lexer, LosslessFile, Token};
use rapidr_preprocessor::{FileId, OriginSpan, PreprocessOptions, PreprocessResult};

/// One parse of a program, for tools.
#[derive(Debug, Clone)]
pub struct ToolsParse {
    /// The preprocessed text, its line map and its origin map (with every
    /// file's text: `preprocessed.origins.files`).
    pub preprocessed: PreprocessResult,
    /// Tokens of the preprocessed text (with error tokens).
    pub tokens: Vec<Token>,
    /// The AST, as the compiler gets it (spans count preprocessed bytes).
    pub program: Program,
    /// Preprocessor, lexer and parser diagnostics. Spans count preprocessed
    /// bytes, except the preprocessor's (bytes of the file named in
    /// `file_path`, the line in error).
    pub diagnostics: Vec<Diagnostic>,
    /// How many of `diagnostics` come first from the preprocessor.
    pub preprocessor_diagnostics: usize,
    /// Each file of `preprocessed.origins.files` (same index), lossless.
    pub files: Vec<LosslessFile>,
    /// The tokens the parser read as type names (`AS QBUTTON`, `EXTENDS
    /// QFORM`), as spans of the preprocessed text, in order
    /// ([`crate::parse_tokens_with_type_names`]).
    pub type_names: Vec<TextSpan>,
}

/// A span of a source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    pub file: FileId,
    pub path: Option<PathBuf>,
    /// Byte range in the file's (decoded) text.
    pub start: usize,
    pub end: usize,
    /// 1-based line and byte column of `start`.
    pub line: usize,
    pub column: usize,
    /// False when the span is (partly) generated text (a `$DEFINE`'s value,
    /// a macro's expansion): the range is the source it came from.
    pub exact: bool,
}

impl ToolsParse {
    /// Where a span of the AST is in the program's files.
    pub fn locate(&self, span: TextSpan) -> Option<Location> {
        let OriginSpan { file, start, end, exact } = self.preprocessed.origins.origin_span(span.start, span.end)?;
        let source = &self.preprocessed.origins.files[file];
        let (line, column) = source.line_col(start);
        Some(Location { file, path: source.path.clone(), start, end, line, column, exact })
    }

    /// The file read from `path`.
    pub fn file_id(&self, path: &Path) -> Option<FileId> {
        self.preprocessed.origins.find_file(path)
    }

    /// The preprocessed offset of byte `offset` of a file (e.g. the caret),
    /// to find the AST node there.
    pub fn to_preprocessed(&self, file: FileId, offset: usize) -> Option<usize> {
        self.preprocessed.origins.to_preprocessed(file, offset)
    }

    /// Where each diagnostic is (the preprocessor's are already in files).
    pub fn diagnostic_locations(&self) -> Vec<(Option<Location>, &Diagnostic)> {
        self.diagnostics
            .iter()
            .enumerate()
            .map(|(i, d)| {
                let location = if i < self.preprocessor_diagnostics {
                    let file = d.file_path.as_deref().and_then(|p| self.file_id(Path::new(p)));
                    file.map(|file| {
                        let source = &self.preprocessed.origins.files[file];
                        let (line, column) = source.line_col(d.span.start);
                        Location { file, path: source.path.clone(), start: d.span.start, end: d.span.end, line, column, exact: true }
                    })
                } else {
                    self.locate(d.span)
                };
                (location, d)
            })
            .collect()
    }
}

/// Parses the program in `path` for tools (see the module documentation).
/// Fails only when `path` can't be read.
pub fn parse_file_for_tools(path: impl AsRef<Path>, options: PreprocessOptions) -> Result<ToolsParse, Diagnostic> {
    let path = path.as_ref();
    let (preprocessed, errors) = rapidr_preprocessor::preprocess_file_recovering(path, options).map_err(|e| e.diagnostic)?;
    Ok(finish(preprocessed, errors, Some(path.display().to_string())))
}

/// Parses a program's text for tools; `$INCLUDE`s are looked for in
/// `base_dir`.
pub fn parse_source_for_tools(source: &str, base_dir: impl AsRef<Path>, file_path: Option<PathBuf>, options: PreprocessOptions) -> ToolsParse {
    let label = file_path.as_ref().map(|p| p.display().to_string());
    let (preprocessed, errors) = rapidr_preprocessor::preprocess_source_recovering(source, base_dir, file_path, options);
    finish(preprocessed, errors, label)
}

fn finish(preprocessed: PreprocessResult, errors: Vec<rapidr_preprocessor::PreprocessError>, label: Option<String>) -> ToolsParse {
    let (tokens, lex_errors) = Lexer::new(&preprocessed.source, label).tokenize_recovering();
    let (program, parse_diagnostics, type_names) = crate::parse_tokens_with_type_names(&tokens);
    let mut diagnostics: Vec<Diagnostic> = errors.into_iter().map(|e| e.diagnostic).collect();
    let preprocessor_diagnostics = diagnostics.len();
    diagnostics.extend(lex_errors.into_iter().map(|e| e.diagnostic));
    diagnostics.extend(parse_diagnostics);
    let files = preprocessed
        .origins
        .files
        .iter()
        .map(|f| LosslessFile::lex_with_line_kinds(&f.text, &f.lines, f.path.as_ref().map(|p| p.display().to_string())))
        .collect();
    ToolsParse { preprocessed, tokens, program, diagnostics, preprocessor_diagnostics, files, type_names }
}

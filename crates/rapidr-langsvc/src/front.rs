//! One parse of a program for the service: the parser for tools
//! (`rapidr_parser::tools`) — the compiler's preprocessor, lexer and
//! parser, recovering from every error — with the byte-level origin map
//! from the AST's spans (bytes of the preprocessed text) to the files.
//!
//! The editor's text of the program's other open files (an `$INCLUDE`d file
//! being edited) goes to the preprocessor as virtual files, so what the
//! service answers is what the editor shows, saved or not.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rapidr_ast::Program;
use rapidr_diagnostics::TextSpan;
use rapidr_parser::tools::{parse_source_for_tools, ToolsParse};
use rapidr_preprocessor::PreprocessOptions;

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
    pub tools: ToolsParse,
    /// The preprocessed text the AST's spans count.
    pub source: String,
    pub program: Program,
    /// Each file of the program by path → its index in the origin map.
    paths: HashMap<PathBuf, usize>,
    /// The origin map's exact segments by (file, source start): a byte of
    /// a file found by a binary search, not a walk through them all.
    by_source: Vec<(usize, usize, usize)>,
}

/// The program's other open files, for the preprocessor: those in the
/// program's own folder first (an `$INCLUDE` is matched by its name).
pub fn virtual_files(root: &Path, open: &[(PathBuf, String)]) -> Vec<(String, String)> {
    let dir = root.parent();
    let mut files: Vec<&(PathBuf, String)> = open.iter().filter(|(p, _)| p != root).collect();
    files.sort_by_key(|(p, _)| p.parent() != dir);
    files.into_iter().map(|(p, t)| (p.display().to_string(), t.clone())).collect()
}

/// Parses `text`, the content of `path`, recovering from errors; `open` is
/// the editor's text of other files.
pub fn parse(path: &Path, text: &str, options: &PreprocessOptions, open: &[(PathBuf, String)]) -> Parsed {
    let base = path.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));
    let mut options = crate::diagnostics::compiler_options(options);
    options.virtual_files = virtual_files(path, open);
    let tools = parse_source_for_tools(text, &base, Some(path.to_path_buf()), options);
    let paths = tools
        .preprocessed
        .origins
        .files
        .iter()
        .enumerate()
        .filter_map(|(i, f)| Some((f.path.clone()?, i)))
        .collect();
    let mut by_source: Vec<(usize, usize, usize)> =
        tools.preprocessed.origins.segments.iter().enumerate().filter(|(_, s)| s.exact).map(|(k, s)| (s.file, s.src_start, k)).collect();
    by_source.sort_unstable();
    Parsed { root: path.to_path_buf(), source: tools.preprocessed.source.clone(), program: tools.program.clone(), tools, paths, by_source }
}

impl Parsed {
    /// Where a span of the AST is in the program's files; `None` for text
    /// no file has (a directive's generated code, a macro's expansion,
    /// RapidR's own libraries).
    pub fn locate(&self, span: TextSpan) -> Option<Location> {
        let l = self.tools.locate(span)?;
        let path = l.path?;
        if !l.exact || path.starts_with("<RapidR>") {
            return None;
        }
        Some(Location { file: path, start: l.start, end: l.end })
    }

    /// The preprocessed offset of byte `offset` of `file` (e.g. the caret).
    pub fn to_preprocessed(&self, file: &Path, offset: usize) -> Option<usize> {
        let id = self.file_id(file)?;
        // (the end of a line or of the file: just after the byte before)
        self.pre_of(id, offset).or_else(|| self.pre_of(id, offset.checked_sub(1)?).map(|p| p + 1))
    }

    /// `OriginMap::to_preprocessed` through the index: the first segment
    /// (in the preprocessed text's order) of file `id` holding `offset`.
    fn pre_of(&self, id: usize, offset: usize) -> Option<usize> {
        let segs = &self.tools.preprocessed.origins.segments;
        let i = self.by_source.partition_point(|&(f, s, _)| (f, s) <= (id, offset));
        // (a file's segments don't overlap unless it's included twice: a
        // few before the place are enough)
        let k = self.by_source[..i]
            .iter()
            .rev()
            .take_while(|&&(f, _, _)| f == id)
            .take(16)
            .filter(|&&(_, _, k)| offset < segs[k].src_end.max(segs[k].src_start + 1))
            .map(|&(_, _, k)| k)
            .min()?;
        let s = &segs[k];
        Some(s.pp_start + (offset - s.src_start).min(s.pp_end - s.pp_start))
    }

    fn file_id(&self, file: &Path) -> Option<usize> {
        self.paths.get(file).copied().or_else(|| self.tools.file_id(file))
    }

    /// The name this parse knows `file` by.
    pub fn file_key(&self, file: &Path) -> Option<&Path> {
        let id = self.file_id(file)?;
        self.tools.preprocessed.origins.files[id].path.as_deref()
    }

    /// The text of a file of the program (the editor's when it's open).
    pub fn file_text(&self, file: &Path) -> Option<&str> {
        let id = self.file_id(file)?;
        Some(&self.tools.preprocessed.origins.files[id].text)
    }

    /// A file of the program as the compiler's lexer reads it: its tokens
    /// (spans count bytes of the file's text) and what each line was to the
    /// preprocessor.
    pub fn lossless(&self, file: &Path) -> Option<&rapidr_lexer::LosslessFile> {
        self.tools.files.get(self.file_id(file)?)
    }
}

/// Whether two paths name the same file (as written, or once resolved).
pub fn same_file(a: &Path, b: &Path) -> bool {
    a == b || matches!((a.canonicalize(), b.canonicalize()), (Ok(x), Ok(y)) if x == y)
}

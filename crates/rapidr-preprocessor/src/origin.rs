//! Where every byte of the preprocessed text came from (docs/ide-plan.md,
//! I0 "Parser for tools").
//!
//! The preprocessor's output is one text made of the program, its
//! `$INCLUDE` files and RapidR's libraries, with directives blanked and
//! `$DEFINE`s / `$MACRO`s substituted. [`OriginMap`] maps any byte of that
//! text back to (file, byte offset) in the files the user edits, so spans of
//! the AST — which count bytes of the preprocessed text — point at the right
//! place in the right file. It also keeps each file's text and what each of
//! its lines was to the preprocessor ([`LineKind`]: code, a directive, a
//! line of an inactive `$IFDEF` branch), which the lossless lexer
//! (`rapidr_lexer::lossless`) uses to keep directives as trivia.
//!
//! Offsets into a file count bytes of its *decoded* text (UTF-8, as
//! [`crate::read_source`] returns it: without a BOM, Windows-1252 decoded);
//! [`SourceFile::encoding`] and [`encode_source`] give the file's bytes back.

use std::path::{Path, PathBuf};

/// How a source file's bytes were turned into text ([`decode_source`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SourceEncoding {
    #[default]
    Utf8,
    /// UTF-8 starting with a byte order mark (left out of the text).
    Utf8Bom,
    /// Not valid UTF-8: read as Windows-1252 (RapidQ's ANSI sources).
    Windows1252,
    /// A UTF-8 byte order mark, then bytes that aren't UTF-8 (read as
    /// Windows-1252).
    Windows1252Bom,
}

/// Decodes a source file's bytes as RapidR reads programs: UTF-8 (a BOM is
/// dropped), or Windows-1252 when the bytes aren't valid UTF-8.
pub fn decode_source(bytes: &[u8]) -> (String, SourceEncoding) {
    let (body, bom) = match bytes.strip_prefix(b"\xEF\xBB\xBF") {
        Some(rest) => (rest, true),
        None => (bytes, false),
    };
    match std::str::from_utf8(body) {
        Ok(text) => (text.to_string(), if bom { SourceEncoding::Utf8Bom } else { SourceEncoding::Utf8 }),
        Err(_) => (
            body.iter().map(|&b| crate::windows_1252_char(b)).collect(),
            if bom { SourceEncoding::Windows1252Bom } else { SourceEncoding::Windows1252 },
        ),
    }
}

/// The bytes [`decode_source`] read `text` from: `encode_source(decode(b)) == b`
/// for every file.
pub fn encode_source(text: &str, encoding: SourceEncoding) -> Vec<u8> {
    match encoding {
        SourceEncoding::Utf8 => text.as_bytes().to_vec(),
        SourceEncoding::Utf8Bom => [b"\xEF\xBB\xBF".as_slice(), text.as_bytes()].concat(),
        SourceEncoding::Windows1252 => text.chars().map(crate::windows_1252_byte).collect(),
        SourceEncoding::Windows1252Bom => b"\xEF\xBB\xBF".iter().copied().chain(text.chars().map(crate::windows_1252_byte)).collect(),
    }
}

/// What one line of a source file was to the preprocessor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    /// Passed on to the compiler (possibly with `$DEFINE`s substituted).
    Code,
    /// A preprocessor directive, consumed (`$INCLUDE`, `$DEFINE`, `$IFDEF`,
    /// `$RESOURCE`, `#If`, …): the compiler sees an empty line or the
    /// constants it stands for.
    Directive,
    /// Inside an `$IFDEF` / `#If` branch that isn't taken: left out.
    Inactive,
    /// A script's `#!` first line.
    Shebang,
}

/// One file of a program, as the preprocessor read it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    /// `None` for a text preprocessed without a file name; RapidR's own
    /// libraries are `<RapidR>/<name>`.
    pub path: Option<PathBuf>,
    /// The decoded text (offsets in [`Segment`]s count its bytes).
    pub text: String,
    pub encoding: SourceEncoding,
    /// One entry per line (`text.split('\n')`), from the file's first
    /// inclusion.
    pub lines: Vec<LineKind>,
}

impl SourceFile {
    /// Byte offset of the start of each line.
    pub fn line_starts(&self) -> Vec<usize> {
        std::iter::once(0).chain(self.text.match_indices('\n').map(|(i, _)| i + 1)).collect()
    }

    /// 1-based (line, column) of a byte offset; the column counts bytes.
    pub fn line_col(&self, offset: usize) -> (usize, usize) {
        let offset = offset.min(self.text.len());
        let line_start = self.text[..offset].rfind('\n').map_or(0, |i| i + 1);
        (self.text[..offset].matches('\n').count() + 1, offset - line_start + 1)
    }

    /// The file's bytes as they are on disk.
    pub fn bytes(&self) -> Vec<u8> {
        encode_source(&self.text, self.encoding)
    }
}

/// Index of a file in [`OriginMap::files`].
pub type FileId = usize;

/// A run of preprocessed text and where it came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment {
    /// Bytes `pp_start..pp_end` of the preprocessed text …
    pub pp_start: usize,
    pub pp_end: usize,
    /// … came from bytes `src_start..src_end` of this file.
    pub file: FileId,
    pub src_start: usize,
    pub src_end: usize,
    /// True when the text is the file's own, byte for byte (same length);
    /// false for generated text (a `$DEFINE`'s value, a `$MACRO`'s
    /// expansion, `$RESOURCE` constants, a built-in include): all of it maps
    /// to the source range it replaced.
    pub exact: bool,
}

/// A position in a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Origin {
    pub file: FileId,
    pub offset: usize,
    /// False inside generated text: `offset` is where that text's source
    /// (the macro call, the `$INCLUDE` line) starts.
    pub exact: bool,
}

/// A range of a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OriginSpan {
    pub file: FileId,
    pub start: usize,
    pub end: usize,
    pub exact: bool,
}

/// Byte-level map from the preprocessed text to the files of the program.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OriginMap {
    pub files: Vec<SourceFile>,
    /// Sorted by `pp_start`, not overlapping; together they cover every byte
    /// of the preprocessed text.
    pub segments: Vec<Segment>,
}

impl OriginMap {
    /// The file read from `path` (compared as written, then canonicalized).
    pub fn find_file(&self, path: &Path) -> Option<FileId> {
        self.files.iter().position(|f| f.path.as_deref() == Some(path)).or_else(|| {
            let wanted = path.canonicalize().ok()?;
            self.files.iter().position(|f| f.path.as_ref().and_then(|p| p.canonicalize().ok()).as_ref() == Some(&wanted))
        })
    }

    fn segment_at(&self, pp: usize) -> Option<&Segment> {
        let i = self.segments.partition_point(|s| s.pp_end <= pp);
        self.segments.get(i).filter(|s| s.pp_start <= pp)
    }

    /// Where byte `pp` of the preprocessed text came from. The end of the
    /// text maps to the end of the last segment.
    pub fn origin(&self, pp: usize) -> Option<Origin> {
        if let Some(s) = self.segment_at(pp) {
            return Some(if s.exact {
                Origin { file: s.file, offset: s.src_start + (pp - s.pp_start), exact: true }
            } else {
                Origin { file: s.file, offset: s.src_start, exact: false }
            });
        }
        let before = self.segments.partition_point(|s| s.pp_end <= pp).checked_sub(1)?;
        let s = &self.segments[before];
        Some(Origin { file: s.file, offset: s.src_end, exact: s.exact && s.pp_end == pp })
    }

    /// Where a span of the preprocessed text came from: the file of its
    /// start, from its start to the end of the last byte of the span in
    /// that same file (a span can't leave its file through an `$INCLUDE`).
    pub fn origin_span(&self, start: usize, end: usize) -> Option<OriginSpan> {
        let first = self.origin(start)?;
        if end <= start {
            return Some(OriginSpan { file: first.file, start: first.offset, end: first.offset, exact: first.exact });
        }
        let mut src_end = first.offset;
        let mut exact = first.exact;
        let from = self.segments.partition_point(|s| s.pp_end <= start);
        for s in self.segments[from..].iter().take_while(|s| s.pp_start < end) {
            if s.file != first.file {
                continue;
            }
            let last = end.min(s.pp_end);
            let mapped_end = if s.exact { s.src_start + (last - s.pp_start) } else { s.src_end };
            src_end = src_end.max(mapped_end);
            exact &= s.exact;
        }
        Some(OriginSpan { file: first.file, start: first.offset, end: src_end.max(first.offset), exact })
    }

    /// The preprocessed offset of byte `offset` of a file (the first time
    /// it appears verbatim), e.g. to find the AST node under the caret.
    pub fn to_preprocessed(&self, file: FileId, offset: usize) -> Option<usize> {
        self.segments
            .iter()
            .find(|s| s.file == file && s.exact && s.src_start <= offset && offset < s.src_end.max(s.src_start + 1))
            .map(|s| s.pp_start + (offset - s.src_start).min(s.pp_end - s.pp_start))
    }
}

/// A piece of output text and the segments it maps through (offsets
/// relative to the text). Substitutions go through [`MappedText::replace`],
/// so the mapping follows every edit.
#[derive(Debug, Clone, Default)]
pub(crate) struct MappedText {
    pub text: String,
    pub segs: Vec<Segment>,
}

impl MappedText {
    /// `text` is the file's own bytes starting at `src_start`.
    pub fn exact(text: &str, file: FileId, src_start: usize) -> Self {
        let segs = if text.is_empty() {
            Vec::new()
        } else {
            vec![Segment { pp_start: 0, pp_end: text.len(), file, src_start, src_end: src_start + text.len(), exact: true }]
        };
        Self { text: text.to_string(), segs }
    }

    /// Generated `text` standing for bytes `src` of `file`.
    pub fn generated(text: String, file: FileId, src: std::ops::Range<usize>) -> Self {
        let segs = if text.is_empty() {
            Vec::new()
        } else {
            vec![Segment { pp_start: 0, pp_end: text.len(), file, src_start: src.start, src_end: src.end, exact: false }]
        };
        Self { text, segs }
    }

    /// Replaces bytes `range` of the text with `with`; the new text maps to
    /// the source the replaced bytes came from.
    pub fn replace(&mut self, range: std::ops::Range<usize>, with: &str) {
        if self.text[range.clone()] == *with {
            return;
        }
        let (start, end) = (range.start, range.end);
        let shift = |p: usize| p + with.len() - (end - start);
        let mut before = Vec::with_capacity(self.segs.len() + 2);
        let mut after = Vec::new();
        let mut covered: Option<(FileId, usize, usize)> = None;
        for s in &self.segs {
            let src_at = |p: usize| if s.exact { s.src_start + (p - s.pp_start) } else { s.src_start };
            // (an insertion point inside a segment splits it)
            let holds_point = start == end && s.pp_start < start && start < s.pp_end;
            if s.pp_end <= start && !holds_point {
                before.push(*s);
            } else if s.pp_start >= end && !holds_point {
                after.push(Segment { pp_start: shift(s.pp_start), pp_end: shift(s.pp_end), ..*s });
            } else {
                // Overlaps the replaced bytes: keep what's outside them.
                if s.pp_start < start {
                    before.push(Segment { pp_end: start, src_end: if s.exact { src_at(start) } else { s.src_end }, ..*s });
                }
                let (a, b) = if s.exact { (src_at(start.max(s.pp_start)), src_at(end.min(s.pp_end))) } else { (s.src_start, s.src_end) };
                covered = Some(match covered {
                    Some((f, x, y)) if f == s.file => (f, x.min(a), y.max(b)),
                    Some(c) => c,
                    None => (s.file, a, b),
                });
                if s.pp_end > end {
                    let src_start = if s.exact { src_at(end) } else { s.src_start };
                    after.push(Segment { pp_start: shift(end), pp_end: shift(s.pp_end), src_start, ..*s });
                }
            }
        }
        if covered.is_none() {
            // (an insertion at a segment's edge: where that segment ends)
            covered = before
                .last()
                .map(|s| (s.file, s.src_end, s.src_end))
                .or_else(|| after.first().map(|s| (s.file, s.src_start, s.src_start)));
        }
        if let (Some((file, a, b)), false) = (covered, with.is_empty()) {
            before.push(Segment { pp_start: start, pp_end: start + with.len(), file, src_start: a, src_end: b, exact: false });
        }
        before.extend(after);
        self.text.replace_range(range, with);
        self.segs = before;
    }
}

/// Builds the preprocessed text of one file (and what it includes): lines
/// joined by `\n`, with segments in absolute offsets of this output.
#[derive(Debug, Default)]
pub(crate) struct Output {
    pub text: String,
    pub segs: Vec<Segment>,
    pub line_map: Vec<crate::LineOrigin>,
    entries: usize,
    /// Where the `\n` after the last entry comes from: (file, offset of the
    /// source `\n`), or the end of its file when it had none.
    pending_newline: Option<(FileId, usize, bool)>,
}

impl Output {
    fn separator(&mut self) {
        if self.entries > 0 {
            let at = self.text.len();
            self.text.push('\n');
            if let Some((file, offset, exact)) = self.pending_newline {
                let src_end = if exact { offset + 1 } else { offset };
                self.segs.push(Segment { pp_start: at, pp_end: at + 1, file, src_start: offset, src_end, exact });
            }
        }
        self.entries += 1;
    }

    /// Appends one line; `newline` is where the `\n` after it would come from.
    pub fn push_line(&mut self, line: MappedText, origin: crate::LineOrigin, newline: (FileId, usize, bool)) {
        self.separator();
        let base = self.text.len();
        self.text.push_str(&line.text);
        self.segs.extend(line.segs.into_iter().map(|s| Segment { pp_start: s.pp_start + base, pp_end: s.pp_end + base, ..s }));
        self.line_map.push(origin);
        self.pending_newline = Some(newline);
    }

    /// Appends a nested output (an included file) as one entry.
    pub fn push_output(&mut self, nested: Output, newline: (FileId, usize, bool)) {
        self.separator();
        let base = self.text.len();
        self.text.push_str(&nested.text);
        self.segs.extend(nested.segs.into_iter().map(|s| Segment { pp_start: s.pp_start + base, pp_end: s.pp_end + base, ..s }));
        self.line_map.extend(nested.line_map);
        self.pending_newline = Some(newline);
    }
}

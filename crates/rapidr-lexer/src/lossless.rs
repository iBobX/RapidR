//! A source file as tokens plus trivia, losslessly (docs/ide-plan.md, I0
//! "Parser for tools").
//!
//! [`LosslessFile::lex`] reads one file the way the compiler does — the
//! same [`Lexer`], so the same tokens — and keeps everything the compiler
//! drops in a side table of [`Trivia`]: spaces, comments (`'` and `REM`),
//! `_` line continuations and the line breaks they join, preprocessor
//! directive lines (`$INCLUDE`, `$DEFINE`, `$IFDEF`, `#If`, …), the lines of
//! inactive `$IFDEF` branches and a script's `#!` line. Tokens and trivia
//! together cover every byte of the file exactly once, in order, so
//! [`LosslessFile::print`] gives the file back byte for byte.
//!
//! Blank lines are kept too: a blank line is its [`TokenType::Newline`]
//! token (the parser's statement separator) with nothing but whitespace
//! trivia before it; [`LosslessFile::lines`] says which lines are blank,
//! comments, directives or code. Directives the compiler itself reads
//! (`$TYPECHECK`, `$APPTYPE`, `$OPTION`, `$ESCAPECHARS`, …) stay
//! [`TokenType::Directive`] tokens, as the parser needs them.
//!
//! The lexer recovers from errors here ([`Lexer::tokenize_recovering`]):
//! unreadable text becomes a [`TokenType::Error`] token and lexing goes on.

use rapidr_diagnostics::TextSpan;
use rapidr_preprocessor::LineKind;

use crate::{LexError, Lexer, Token, TokenType};

/// What a piece of trivia is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TriviaKind {
    /// Spaces and tabs.
    Whitespace,
    /// `' …` or `REM …`, up to the end of the line (without the line break).
    Comment,
    /// The `_` that continues a statement on the next line.
    LineContinuation,
    /// A line break inside a statement (after `_`): not a statement end.
    LineBreak,
    /// A preprocessor directive line (the whole line but its line break).
    Directive,
    /// A line of an `$IFDEF` / `#If` branch that isn't compiled.
    Inactive,
    /// A script's `#!` first line.
    Shebang,
    /// Anything else between tokens (none in a file the lexer reads).
    Other,
}

/// A run of text the compiler skips.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Trivia {
    pub kind: TriviaKind,
    pub span: TextSpan,
}

/// One piece of a file, in order: a token or trivia.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Piece<'a> {
    Token(&'a Token),
    Trivia(&'a Trivia),
}

impl Piece<'_> {
    pub fn span(&self) -> TextSpan {
        match self {
            Piece::Token(t) => t.span,
            Piece::Trivia(t) => t.span,
        }
    }
}

/// What a whole line of a file is, for tools (formatters, the designer's
/// source sync).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LineClass {
    /// Nothing but spaces.
    Blank,
    /// Only a comment (and spaces).
    Comment,
    /// A preprocessor directive.
    Directive,
    /// In an inactive `$IFDEF` branch.
    Inactive,
    /// A script's `#!` line.
    Shebang,
    /// Has tokens.
    Code,
}

/// A file as tokens plus trivia.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LosslessFile {
    /// The file's text (as `rapidr_preprocessor::read_source` decodes it).
    pub text: String,
    /// The tokens, as the compiler's lexer makes them (with the line's own
    /// `$ESCAPECHARS` state), ending with [`TokenType::Eof`]; directive and
    /// inactive lines have none but their line break.
    pub tokens: Vec<Token>,
    /// Everything between the tokens, in order.
    pub trivia: Vec<Trivia>,
    /// Lexical errors (each also an `Error` token, or an unterminated string).
    pub errors: Vec<LexError>,
    /// What each line was to the preprocessor.
    pub line_kinds: Vec<LineKind>,
}

impl LosslessFile {
    /// Lexes `text` (one file; its `$INCLUDE`s aren't read), classifying
    /// its lines with `rapidr_preprocessor::scan_lines`.
    pub fn lex(text: &str, file_path: Option<String>) -> Self {
        let kinds = rapidr_preprocessor::scan_lines(text, Default::default());
        Self::lex_with_line_kinds(text, &kinds, file_path)
    }

    /// Lexes `text` whose lines the preprocessor classified as `kinds` (one
    /// per `\n`-separated line, e.g. `SourceFile::lines` of a program's
    /// origin map: the exact `$IFDEF` decisions of a real build).
    pub fn lex_with_line_kinds(text: &str, kinds: &[LineKind], file_path: Option<String>) -> Self {
        // Lines the compiler never sees are blanked (same length, so every
        // offset stays), their line breaks kept.
        let mut masked: Option<String> = None;
        let mut start = 0;
        for (i, line) in text.split('\n').enumerate() {
            let kind = kinds.get(i).copied().unwrap_or(LineKind::Code);
            if kind != LineKind::Code {
                let m = masked.get_or_insert_with(|| text.to_string());
                let body = line.strip_suffix('\r').unwrap_or(line);
                // (all ASCII spaces: still UTF-8)
                blank_range(m, start, start + body.len());
            }
            start += line.len() + 1;
        }
        let source = masked.as_deref().unwrap_or(text);
        let (tokens, errors) = Lexer::new(source, file_path).tokenize_recovering();

        let line_starts: Vec<usize> = std::iter::once(0).chain(text.match_indices('\n').map(|(i, _)| i + 1)).collect();
        let kind_at = |offset: usize| {
            let line = line_starts.partition_point(|&s| s <= offset) - 1;
            kinds.get(line).copied().unwrap_or(LineKind::Code)
        };
        let mut trivia = Vec::new();
        let mut at = 0;
        for token in tokens.iter().filter(|t| t.span.end > t.span.start) {
            if token.span.start < at {
                // (tokens never overlap; if one did, its bytes are already
                // covered)
                continue;
            }
            classify_gap(text, at, token.span.start, &kind_at, &mut trivia);
            at = token.span.end;
        }
        classify_gap(text, at, text.len(), &kind_at, &mut trivia);
        Self { text: text.to_string(), tokens, trivia, errors, line_kinds: kinds.to_vec() }
    }

    /// Tokens and trivia in file order (zero-length tokens, such as `Eof`,
    /// left out).
    pub fn pieces(&self) -> Vec<Piece<'_>> {
        let mut pieces = Vec::with_capacity(self.tokens.len() + self.trivia.len());
        let (mut t, mut v) = (self.tokens.iter().filter(|t| t.span.end > t.span.start).peekable(), self.trivia.iter().peekable());
        loop {
            match (t.peek(), v.peek()) {
                (Some(a), Some(b)) if a.span.start < b.span.start => pieces.push(Piece::Token(t.next().unwrap())),
                (_, Some(_)) => pieces.push(Piece::Trivia(v.next().unwrap())),
                (Some(_), None) => pieces.push(Piece::Token(t.next().unwrap())),
                (None, None) => break,
            }
        }
        pieces
    }

    /// The text of a piece.
    pub fn piece_text(&self, piece: Piece<'_>) -> &str {
        let span = piece.span();
        &self.text[span.start..span.end]
    }

    /// The file again, from its tokens and trivia: always equal to `text`.
    pub fn print(&self) -> String {
        let mut out = String::with_capacity(self.text.len());
        for piece in self.pieces() {
            out.push_str(self.piece_text(piece));
        }
        out
    }

    /// The trivia right before token `index` (since the previous token).
    pub fn leading_trivia(&self, index: usize) -> &[Trivia] {
        let Some(token) = self.tokens.get(index) else { return &[] };
        let prev_end = self.tokens[..index].iter().rev().find(|t| t.span.end > t.span.start).map_or(0, |t| t.span.end);
        let from = self.trivia.partition_point(|t| t.span.start < prev_end);
        let to = self.trivia.partition_point(|t| t.span.start < token.span.start);
        &self.trivia[from..to.max(from)]
    }

    /// What each line of the file is.
    pub fn lines(&self) -> Vec<LineClass> {
        let mut classes: Vec<LineClass> = self
            .line_kinds
            .iter()
            .map(|k| match k {
                LineKind::Directive => LineClass::Directive,
                LineKind::Inactive => LineClass::Inactive,
                LineKind::Shebang => LineClass::Shebang,
                LineKind::Code => LineClass::Blank,
            })
            .collect();
        classes.resize(self.text.split('\n').count(), LineClass::Blank);
        let line_starts: Vec<usize> = std::iter::once(0).chain(self.text.match_indices('\n').map(|(i, _)| i + 1)).collect();
        let line_of = |offset: usize| line_starts.partition_point(|&s| s <= offset) - 1;
        for piece in self.pieces() {
            let (kind, span) = match piece {
                Piece::Token(t) if t.kind == TokenType::Newline => continue,
                Piece::Token(t) => (None, t.span),
                Piece::Trivia(t) => (Some(t.kind), t.span),
            };
            // (a token may run over several lines: a continued string, RUSTSTART)
            let (first, last) = (line_of(span.start), line_of(span.end.saturating_sub(1).max(span.start)));
            for class in &mut classes[first..=last] {
                match (kind, *class) {
                    (_, LineClass::Directive | LineClass::Inactive | LineClass::Shebang) => {}
                    (None, _) => *class = LineClass::Code,
                    (Some(TriviaKind::Comment), LineClass::Blank) => *class = LineClass::Comment,
                    _ => {}
                }
            }
        }
        classes
    }
}

/// Blanks bytes `from..to` of `text` with spaces (they hold one line's text,
/// whole characters).
fn blank_range(text: &mut String, from: usize, to: usize) {
    let blanks = " ".repeat(to - from);
    text.replace_range(from..to, &blanks);
}

/// Splits the text between two tokens into trivia.
fn classify_gap(text: &str, from: usize, to: usize, kind_at: &dyn Fn(usize) -> LineKind, out: &mut Vec<Trivia>) {
    let bytes = text.as_bytes();
    let mut i = from;
    while i < to {
        let start = i;
        let line_kind = kind_at(i);
        let kind = if bytes[i] == b'\n' || (bytes[i] == b'\r' && bytes.get(i + 1) == Some(&b'\n')) {
            i += if bytes[i] == b'\r' { 2 } else { 1 };
            TriviaKind::LineBreak
        } else if line_kind != LineKind::Code {
            // The rest of a directive / inactive line, up to its line break.
            while i < to && bytes[i] != b'\n' && !(bytes[i] == b'\r' && bytes.get(i + 1) == Some(&b'\n')) {
                i += 1;
            }
            match line_kind {
                LineKind::Directive => TriviaKind::Directive,
                LineKind::Inactive => TriviaKind::Inactive,
                _ => TriviaKind::Shebang,
            }
        } else if bytes[i] == b' ' || bytes[i] == b'\t' {
            while i < to && (bytes[i] == b' ' || bytes[i] == b'\t') {
                i += 1;
            }
            TriviaKind::Whitespace
        } else if bytes[i] == b'\'' || bytes[i..to].get(..3).is_some_and(|w| w.eq_ignore_ascii_case(b"REM")) {
            while i < to && bytes[i] != b'\n' && !(bytes[i] == b'\r' && bytes.get(i + 1) == Some(&b'\n')) {
                i += 1;
            }
            TriviaKind::Comment
        } else if bytes[i] == b'_' {
            i += 1;
            TriviaKind::LineContinuation
        } else {
            // (a lone `\r`, or anything unexpected: up to the next known piece)
            i += text[i..].chars().next().map_or(1, char::len_utf8);
            while i < to && !matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r' | b'\'' | b'_') && !text.is_char_boundary(i) {
                i += 1;
            }
            TriviaKind::Other
        };
        // (merge with the previous piece of the same kind: `\r` runs)
        match out.last_mut() {
            Some(last) if last.kind == kind && last.span.end == start && kind == TriviaKind::Other => last.span.end = i,
            _ => out.push(Trivia { kind, span: TextSpan::new(start, i) }),
        }
    }
}

/// Lexes one file losslessly (see the module documentation).
pub fn lex_lossless(text: &str) -> LosslessFile {
    LosslessFile::lex(text, None)
}

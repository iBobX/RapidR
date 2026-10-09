//! Automatic case, as QuickBASIC and VB did it: the language's words —
//! keywords and statements, type names, directives, builtins — in one case
//! (`DIM x AS INTEGER`), and optionally the program's own names as their
//! declaration spells them.
//!
//! The words come from the language registry (`rapidr_lang`); the text is
//! read as the compiler's lexer reads it (the parser for tools' lossless
//! tokens), so strings, comments, inactive `$IFDEF` branches and what
//! follows a directive (an `$INCLUDE`'s path, a `$DEFINE`'s text) are never
//! touched, and a word is the language's only where it isn't one of the
//! program's names (the compiler's model says): `DIM Left AS INTEGER` keeps
//! `Left`. BASIC ignores case, so no edit changes what a program does.
//!
//! One function for every editor: `rapidr lsp` (on-type and document /
//! range formatting) and RapidR Studio's editor.

use std::collections::HashSet;
use std::path::Path;
use std::sync::OnceLock;

use rapidr_lexer::{Token, TokenType};
use rapidr_preprocessor::LineKind;

use crate::context::{self, chain_before, Ty};
use crate::text::{is_name_char, is_suffix_char, LineIndex};
use crate::{Snapshot, TextEdit};

/// How the language's words are written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KeywordCase {
    /// `DIM x AS INTEGER` (QuickBASIC's).
    #[default]
    Upper,
    /// `dim x as integer`.
    Lower,
    /// `Dim x As Integer` (VB's).
    Proper,
    /// As typed: off.
    Preserve,
}

/// How the program's own names are written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IdentifierCase {
    /// Each use as its declaration spells it (VB's), and a component's
    /// members as the registry spells them (`Form.caption` → `Form.Caption`).
    Declaration,
    /// As typed: off.
    #[default]
    Preserve,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CaseOptions {
    pub keywords: KeywordCase,
    pub identifiers: IdentifierCase,
}

impl KeywordCase {
    /// From a setting's value: `upper`, `lower`, `proper`, `preserve`.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "upper" => Some(Self::Upper),
            "lower" => Some(Self::Lower),
            "proper" => Some(Self::Proper),
            "preserve" => Some(Self::Preserve),
            _ => None,
        }
    }
}

impl IdentifierCase {
    /// From a setting's value: `declaration`, `preserve`.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "declaration" => Some(Self::Declaration),
            "preserve" => Some(Self::Preserve),
            _ => None,
        }
    }
}

/// Which words to case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseScope {
    /// The user typed `ch`, ending at byte `offset` (after it): the word
    /// just finished — or, for a line break, the whole line just left.
    Typed { offset: usize, ch: char },
    /// Every word in a range of bytes (formatting).
    Range { start: usize, end: usize },
}

/// The characters after which a word is finished (on-type formatting).
pub const TRIGGERS: &[char] = &[' ', '\n', '\t', '(', ')', ',', ':', '=', '+', '-', '*', '/', '\\', '^', '&', '<', '>', ';'];

/// The language's words, upper case, without type suffixes.
fn words() -> &'static HashSet<&'static str> {
    static WORDS: OnceLock<HashSet<&'static str>> = OnceLock::new();
    WORDS.get_or_init(|| ["control", "keyword", "type", "operator", "constant"].iter().flat_map(|k| rapidr_lang::words(k)).collect())
}

/// `word` in `case` (its type suffix kept).
fn cased(word: &str, case: KeywordCase) -> String {
    match case {
        KeywordCase::Upper => word.to_ascii_uppercase(),
        KeywordCase::Lower => word.to_ascii_lowercase(),
        KeywordCase::Proper => {
            let lower = word.to_ascii_lowercase();
            let mut c = lower.chars();
            match c.next() {
                Some(f) => f.to_ascii_uppercase().to_string() + c.as_str(),
                None => lower,
            }
        }
        KeywordCase::Preserve => word.to_string(),
    }
}

/// The edits that case `scope` of `file` (`text`: the editor's text, the
/// one `s` analysed).
pub(crate) fn case_edits(s: &Snapshot, file: &Path, text: &str, scope: CaseScope, options: CaseOptions) -> Vec<TextEdit> {
    if options.keywords == KeywordCase::Preserve && options.identifiers == IdentifierCase::Preserve {
        return Vec::new();
    }
    let index = LineIndex::new(text);
    let (from, to) = match scope {
        CaseScope::Range { start, end } => (start, end.min(text.len())),
        CaseScope::Typed { offset, ch } => {
            let offset = offset.min(text.len());
            let at = offset.saturating_sub(ch.len_utf8());
            if ch == '\n' {
                // (the line the caret left: the one before the caret's,
                // whatever the editor indented the new one with)
                let (line, _) = index.line_col(offset);
                let Some(prev) = line.checked_sub(1) else { return Vec::new() };
                (index.line_start(prev).unwrap_or(0), index.line_end(text, prev))
            } else {
                // (the word that ends where `ch` was typed)
                (at, at)
            }
        }
    };
    // (the analysis's tokens when it was made of this text; typing, from an
    // older one: today's lines being cased lexed again — the lexer alone, on
    // them only, so a key costs the same in a long file — and the older
    // model for the program's names)
    let fresh;
    let lf = match s.parsed.lossless(file) {
        Some(lf) if lf.text == text => lf,
        Some(_) if s.is_stale() => {
            let (first, _) = index.line_col(from);
            let (last, _) = index.line_col(to);
            let ws = index.line_start(first).unwrap_or(0);
            let we = index.line_end(text, last).min(text.len()).max(ws);
            let mut w = rapidr_lexer::lex_lossless(&text[ws..we]);
            for t in &mut w.tokens {
                t.span.start += ws;
                t.span.end += ws;
            }
            let mut kinds = vec![LineKind::Code; first];
            kinds.append(&mut w.line_kinds);
            w.line_kinds = kinds;
            fresh = w;
            &fresh
        }
        _ => return Vec::new(),
    };
    let word_only = matches!(scope, CaseScope::Typed { ch, .. } if ch != '\n');
    let mut c = Caser { s, file, text, index: &index, options, out: Vec::new() };
    c.directives(&lf.line_kinds, from, to, word_only);
    let toks: Vec<&Token> = lf.tokens.iter().filter(|t| t.kind != TokenType::Eof).collect();
    let mut statement_start = true;
    for (i, t) in toks.iter().enumerate() {
        if matches!(t.kind, TokenType::Newline | TokenType::Colon) {
            statement_start = true;
            continue;
        }
        let at_start = std::mem::replace(&mut statement_start, matches!(t.kind, TokenType::Then | TokenType::Else));
        // (a directive the compiler reads, `$TYPECHECK ON`: its word)
        if t.kind == TokenType::Directive && text[t.span.start..].starts_with('$') {
            let (start, end) = (t.span.start + 1, word_end(text, t.span.start + 1, t.span.end));
            let inside = if word_only { end == from } else { start >= from && end <= to };
            if end > start && inside && c.options.keywords != KeywordCase::Preserve && rapidr_lang::directive(&text[start..end]).is_some() {
                let new = cased(&text[start..end], c.options.keywords);
                c.replace(start, end, new);
            }
            continue;
        }
        // (the word: a RUSTSTART token spans its whole block)
        let start = t.span.start;
        let end = word_end(text, start, t.span.end);
        if end == start {
            continue;
        }
        let inside = if word_only { end == from } else { start >= from && end <= to };
        if !inside {
            continue;
        }
        let prev = i.checked_sub(1).map(|p| toks[p]);
        let next = toks.get(i + 1).copied();
        c.token(t, start, end, prev, next, at_start);
    }
    c.out.sort_by_key(|e| e.start);
    c.out
}

/// Where the name at `start` ends (its suffix included), not past `end`.
fn word_end(text: &str, start: usize, end: usize) -> usize {
    let b = text.as_bytes();
    let mut e = start;
    if e < end && (b[e] as char).is_ascii_alphabetic() || e < end && b[e] == b'_' {
        while e < end && is_name_char(b[e] as char) {
            e += 1;
        }
        while e < end && is_suffix_char(b[e] as char) {
            e += 1;
        }
    }
    e
}

struct Caser<'a> {
    s: &'a Snapshot,
    file: &'a Path,
    text: &'a str,
    index: &'a LineIndex,
    options: CaseOptions,
    out: Vec<TextEdit>,
}

impl Caser<'_> {
    fn replace(&mut self, start: usize, end: usize, new: String) {
        if self.text[start..end] != new {
            self.out.push(TextEdit { start, end, text: new });
        }
    }

    /// `$include` → `$INCLUDE` (only the directive's word).
    fn directives(&mut self, kinds: &[LineKind], from: usize, to: usize, word_only: bool) {
        if self.options.keywords == KeywordCase::Preserve {
            return;
        }
        for (line, kind) in kinds.iter().enumerate() {
            if *kind != LineKind::Directive {
                continue;
            }
            let Some(line_start) = self.index.line_start(line) else { continue };
            let lt = self.index.line_text(self.text, line);
            let lead = lt.len() - lt.trim_start().len();
            let Some(body) = lt[lead..].strip_prefix('$') else { continue };
            let len = body.find(|c: char| !is_name_char(c)).unwrap_or(body.len());
            let (start, end) = (line_start + lead + 1, line_start + lead + 1 + len);
            let inside = if word_only { end == from } else { start >= from && end <= to };
            if len == 0 || !inside || rapidr_lang::directive(&body[..len]).is_none() {
                continue;
            }
            let new = cased(&body[..len], self.options.keywords);
            self.replace(start, end, new);
        }
    }

    fn token(&mut self, t: &Token, start: usize, end: usize, prev: Option<&Token>, next: Option<&Token>, at_start: bool) {
        let word = &self.text[start..end];
        let after_dot = prev.is_some_and(|p| p.kind == TokenType::Dot);
        let pre = self.s.pre_offset(self.file, start);
        let model = &self.s.model;
        // The program's own name here (a member's too, when the model has it).
        let symbol = pre.and_then(|p| model.symbol_at(p)).or_else(|| {
            let p = pre?;
            (t.kind == TokenType::Identifier && !after_dot).then(|| model.lookup(word, model.scope_at(p))).flatten()
        });
        if let Some(id) = symbol {
            if self.options.identifiers == IdentifierCase::Declaration {
                if let Some(spelled) = self.declared(id) {
                    self.respell(start, end, &spelled);
                }
            }
            return;
        }
        if after_dot {
            if self.options.identifiers == IdentifierCase::Declaration {
                if let Some(name) = self.member_name(t, start) {
                    self.respell(start, end, name);
                }
            }
            return;
        }
        if self.options.keywords == KeywordCase::Preserve {
            return;
        }
        // (`Left = 12` in a CREATE body, `Color = 3`: a property, a variable)
        if t.kind == TokenType::Identifier && at_start && next.is_some_and(|n| n.kind == TokenType::Eq) {
            return;
        }
        let base = rapidr_ast::strip_type_suffix(word).to_ascii_uppercase();
        let is_word = (t.kind != TokenType::Identifier || base.len() == word.len()) && words().contains(base.as_str());
        if is_word || rapidr_lang::builtin(word).is_some() {
            let new = cased(word, self.options.keywords);
            self.replace(start, end, new);
        }
    }

    /// A symbol's name as its declaration writes it (without its suffix).
    fn declared(&self, id: crate::model::SymbolId) -> Option<String> {
        let sym = &self.s.model.symbols[id];
        let loc = self.s.locate(sym.decl?)?;
        let text = self.s.parsed.file_text(&loc.file)?;
        let at = &text[loc.start..loc.end.min(text.len())];
        let base = rapidr_ast::strip_type_suffix(at);
        (!base.is_empty() && base.chars().all(is_name_char)).then(|| base.to_string())
    }

    /// The registry's spelling of the member after a dot (`Form.caption`).
    fn member_name(&self, t: &Token, start: usize) -> Option<&'static str> {
        let dot = start.checked_sub(1)?;
        let (line, _) = self.index.line_col(dot);
        let chain = chain_before(self.text, self.index.line_start(line)?, dot);
        let pre = self.s.pre_offset(self.file, dot)?;
        let c = match context::resolve_chain(self.s, pre, &chain)? {
            Ty::Component(c) => c,
            Ty::User(u) => context::base_component(self.s, &u)?,
        };
        let name = &t.lexeme;
        c.property(name).map(|p| p.name).or_else(|| c.method(name).map(|m| m.name)).or_else(|| c.event(name).map(|e| e.name))
    }

    /// `word` (at start..end) spelled as `base`, its own suffix kept.
    fn respell(&mut self, start: usize, end: usize, base: &str) {
        let word = &self.text[start..end];
        let own = rapidr_ast::strip_type_suffix(word);
        if own.eq_ignore_ascii_case(base) {
            let new = format!("{base}{}", &word[own.len()..]);
            self.replace(start, end, new);
        }
    }
}

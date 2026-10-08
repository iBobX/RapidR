//! What the language registry (`rapidr_lang`) says about the names a file
//! uses, as diagnostics (docs/q-and-r-components.md §5):
//!
//! - always: what RapidQ has and RapidR doesn't answer yet (`missing`, a
//!   `planned` component), and what one runtime only answers (`only`: the
//!   desktop — native builds and the interpreter — or the web), as a note;
//! - in a RapidQ-compatible project: RapidR's extensions, which RapidQ's
//!   compiler refuses — components, members of RapidQ's components,
//!   builtins, statements, directives, types and constants of RapidR's
//!   origin — Q-prefixed names RapidQ doesn't have (`QPLOT`: RapidR
//!   reads it as RPLOT) and RapidR's names of RapidQ's components
//!   (`RButton`: RapidQ's compiler knows QBUTTON), with fixes.
//!
//! The file is read as the compiler's lexer reads it (the parser for
//! tools' lossless tokens); names are resolved by the compiler's model.

use std::path::Path;

use rapidr_diagnostics::Severity;
use rapidr_lang::{Component, Kind, Origin, Runtimes};
use rapidr_lexer::{Token, TokenType};
use rapidr_preprocessor::LineKind;

use crate::context::{self, chain_before, pretty_component, Ty};
use crate::model::SymbolKind;
use crate::text::{is_name_char, LineIndex};
use crate::{CodeAction, FileDiagnostic, Snapshot, TextEdit};

pub(crate) const RAPIDR_ONLY: &str = "rapidr-only";
pub(crate) const NOT_RAPIDQ_NAME: &str = "not-a-rapidq-name";
pub(crate) const RAPIDR_NAME: &str = "rapidr-name";
pub(crate) const NOT_IMPLEMENTED: &str = "not-implemented";
pub(crate) const ONE_RUNTIME: &str = "one-runtime";

const COMPAT: &str = "RapidQ's compiler refuses it (this project is RapidQ-compatible)";

/// The notes a doc gets from an entry's registry flags (empty when there's
/// nothing to say): not in RapidR yet, a RapidR extension of a RapidQ
/// thing (`owner`: the component's origin; RapidQ for the language's own
/// words), one runtime only, the RapidQ include file it comes from.
pub(crate) fn notes(origin: Origin, owner: Origin, missing: bool, runtimes: Runtimes, from: Option<&str>) -> String {
    let mut out: Vec<String> = Vec::new();
    if missing {
        out.push("*RapidQ has it; RapidR doesn't yet (not implemented).*".into());
    }
    if origin == Origin::RapidR && owner == Origin::RapidQ {
        out.push("*A RapidR extension: RapidQ's compiler refuses it.*".into());
    }
    match runtimes {
        Runtimes::All => {}
        Runtimes::Desktop => out.push("*Desktop only (native builds and the interpreter): the web runtime doesn't have it.*".into()),
        Runtimes::Web => out.push("*Web only: the desktop doesn't have it.*".into()),
    }
    if let Some(inc) = from {
        out.push(format!("*RapidQ's, from `{inc}`.*"));
    }
    out.join("\n\n")
}

/// The registry's diagnostics for `file` of the program `s`.
pub(crate) fn check(s: &Snapshot, file: &Path, rapidq_compatible: bool) -> Vec<FileDiagnostic> {
    let Some(lf) = s.parsed.lossless(file) else { return Vec::new() };
    let mut c = Checker { s, file, text: &lf.text, index: LineIndex::new(&lf.text), compat: rapidq_compatible, out: Vec::new() };
    c.directives(&lf.line_kinds);
    let toks: Vec<&Token> = lf.tokens.iter().filter(|t| t.kind != TokenType::Eof).collect();
    let mut statement_start = true;
    for (i, t) in toks.iter().enumerate() {
        if matches!(t.kind, TokenType::Newline | TokenType::Colon) {
            statement_start = true;
            continue;
        }
        let at_start = std::mem::replace(&mut statement_start, matches!(t.kind, TokenType::Then | TokenType::Else));
        let prev = i.checked_sub(1).map(|p| toks[p]);
        let next = toks.get(i + 1).copied();
        if prev.is_some_and(|p| p.kind == TokenType::Dot) {
            c.member(t);
            continue;
        }
        if prev.is_some_and(|p| p.kind == TokenType::As) {
            c.type_after_as(t);
            continue;
        }
        if at_start && c.statement(t, next) {
            continue;
        }
        if t.kind == TokenType::Identifier {
            // `Caption = …` in a CREATE body: a property of the component
            if at_start && next.is_some_and(|n| n.kind == TokenType::Eq) {
                if let Some(comp) = s.pre_offset(file, t.span.start).and_then(|pre| context::create_at(s, pre)) {
                    c.member_of(comp, t);
                    continue;
                }
            }
            c.name(t, next);
        }
    }
    c.out
}

struct Checker<'a> {
    s: &'a Snapshot,
    file: &'a Path,
    text: &'a str,
    index: LineIndex,
    compat: bool,
    out: Vec<FileDiagnostic>,
}

impl Checker<'_> {
    fn push(&mut self, start: usize, end: usize, severity: Severity, code: &str, message: String) {
        self.out.push(FileDiagnostic { file: self.file.to_path_buf(), start, end, severity, message, code: Some(code.into()) });
    }

    fn extension(&mut self, start: usize, end: usize, what: String) {
        if self.compat {
            self.push(start, end, Severity::Warning, RAPIDR_ONLY, format!("{what} is a RapidR extension: {COMPAT}"));
        }
    }

    fn not_implemented(&mut self, start: usize, end: usize, what: String) {
        self.push(start, end, Severity::Warning, NOT_IMPLEMENTED, format!("RapidQ's {what} is not implemented in RapidR yet"));
    }

    fn one_runtime(&mut self, start: usize, end: usize, what: String, runtimes: Runtimes) {
        let message = match runtimes {
            Runtimes::All => return,
            Runtimes::Desktop => format!("{what} works on the desktop only: the web runtime doesn't have it"),
            Runtimes::Web => format!("{what} works in the web runtime only: the desktop doesn't have it"),
        };
        self.push(start, end, Severity::Note, ONE_RUNTIME, message);
    }

    /// Whether `name` (at file offset `at`) is one of the program's own
    /// names there (a variable, a SUB …): then it isn't the language's.
    fn is_program_name(&self, name: &str, at: usize) -> bool {
        let Some(pre) = self.s.pre_offset(self.file, at) else { return false };
        let m = &self.s.model;
        m.symbol_at(pre).is_some() || m.lookup(name, m.scope_at(pre)).is_some_and(|id| m.symbols[id].kind != SymbolKind::Label)
    }

    /// `$THEME …` and the other directive lines (the lexer leaves them out).
    fn directives(&mut self, kinds: &[LineKind]) {
        let index = LineIndex::new(self.text);
        for (line, kind) in kinds.iter().enumerate() {
            if *kind != LineKind::Directive {
                continue;
            }
            let Some(start) = index.line_start(line) else { continue };
            let lt = index.line_text(self.text, line);
            let lead = lt.len() - lt.trim_start().len();
            let rest = &lt[lead..];
            let Some(body) = rest.strip_prefix('$') else { continue };
            let len = body.find(|c: char| !is_name_char(c)).unwrap_or(body.len());
            let Some(d) = rapidr_lang::directive(&body[..len]) else { continue };
            if d.origin == Origin::RapidR {
                let s = start + lead;
                self.extension(s, s + 1 + len, d.name.to_string());
            }
        }
    }

    /// A statement's first word(s) (`OPEN`, `LINE INPUT`, `PRINT #`):
    /// whether it is one of the registry's statements.
    fn statement(&mut self, t: &Token, next: Option<&Token>) -> bool {
        if !t.lexeme.starts_with(|c: char| c.is_ascii_alphabetic()) {
            return false;
        }
        // (an assignment or a member: `inc = 1`, `Seek.x`)
        if next.is_some_and(|n| matches!(n.kind, TokenType::Eq | TokenType::Dot)) {
            return false;
        }
        let two = next.and_then(|n| {
            let st = rapidr_lang::statement(&format!("{} {}", t.lexeme, n.lexeme))?;
            Some((st, n.span.end))
        });
        let Some((st, end)) = two.or_else(|| rapidr_lang::statement(&t.lexeme).map(|st| (st, t.span.end))) else { return false };
        if t.kind == TokenType::Identifier && self.is_program_name(&t.lexeme, t.span.start) {
            return false;
        }
        if st.origin == Origin::RapidR {
            self.extension(t.span.start, end, st.name.to_string());
        }
        true
    }

    /// The type after `AS`: a built-in type or a component.
    fn type_after_as(&mut self, t: &Token) {
        let (start, end) = (t.span.start, t.span.end);
        if let Some(ty) = rapidr_lang::type_name(&t.lexeme) {
            if ty.origin == Origin::RapidR {
                self.extension(start, end, ty.name.to_string());
            }
            return;
        }
        if t.kind != TokenType::Identifier || self.is_program_name(&t.lexeme, start) {
            return;
        }
        let written = t.lexeme.to_ascii_uppercase();
        let Some(c) = rapidr_lang::resolve_component(&written) else { return };
        let shown = pretty_component(&written);
        let is_rapidq_name = c.rapidq.is_some_and(|q| q.eq_ignore_ascii_case(&written)) || c.aliases.iter().any(|a| a.eq_ignore_ascii_case(&written));
        if c.kind == Kind::Planned {
            self.not_implemented(start, end, shown.clone());
        } else if written.starts_with('Q') && !is_rapidq_name {
            if self.compat {
                self.push(start, end, Severity::Warning, NOT_RAPIDQ_NAME, format!("RapidQ has no {written}: RapidR reads it as {}", c.spelling()));
            }
        } else if self.compat && c.origin == Origin::RapidQ && c.kind == Kind::Component && !is_rapidq_name {
            // (`RBUTTON` in a RapidQ-compatible project: RapidQ's compiler
            // knows the component by its RapidQ name only)
            let q = c.rapidq.unwrap_or(c.name);
            self.push(start, end, Severity::Warning, RAPIDR_NAME, format!("{shown} is RapidR's name: RapidQ's compiler knows it as {q}"));
        } else if c.origin == Origin::RapidR && self.compat {
            self.push(
                start,
                end,
                Severity::Warning,
                RAPIDR_ONLY,
                format!("{} is RapidR's own component: RapidQ doesn't have it (this project is RapidQ-compatible)", c.spelling()),
            );
        }
        self.one_runtime(start, end, shown, c.runtimes);
    }

    /// `Obj.Member`: the member, when the object's type is a component's.
    fn member(&mut self, t: &Token) {
        if !t.lexeme.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_') {
            return;
        }
        let dot = t.span.start - 1;
        let index = &self.index;
        let (line, _) = index.line_col(dot);
        let line_start = index.line_start(line).unwrap_or(0);
        let chain = chain_before(self.text, line_start, dot);
        let Some(pre) = self.s.pre_offset(self.file, dot) else { return };
        let comp = match context::resolve_chain(self.s, pre, &chain) {
            Some(Ty::Component(c)) => c,
            Some(Ty::User(u)) if context::user_member(self.s, &u, &t.lexeme).is_none() => match context::base_component(self.s, &u) {
                Some(c) => c,
                None => return,
            },
            _ => return,
        };
        self.member_of(comp, t);
    }

    fn member_of(&mut self, c: &'static Component, t: &Token) {
        let name = &t.lexeme;
        let (origin, missing, runtimes, member) = if let Some(p) = c.property(name) {
            (p.origin, p.missing, p.runtimes, p.name)
        } else if let Some(m) = c.method(name) {
            (m.origin, m.missing, m.runtimes, m.name)
        } else if let Some(e) = c.event(name) {
            (e.origin, e.missing, e.runtimes, e.name)
        } else {
            return;
        };
        // (in a RapidQ-compatible project, under RapidQ's name)
        let owner = if self.compat { c.rapidq_spelling().unwrap_or_else(|| c.spelling()) } else { c.spelling() };
        let what = format!("{owner}.{member}");
        let (start, end) = (t.span.start, t.span.end);
        if missing {
            self.not_implemented(start, end, what.clone());
        } else if origin == Origin::RapidR && c.origin == Origin::RapidQ {
            self.extension(start, end, what.clone());
        }
        self.one_runtime(start, end, what, runtimes);
    }

    /// A name that isn't the program's: a builtin, RapidR's constant.
    fn name(&mut self, t: &Token, next: Option<&Token>) {
        let (start, end) = (t.span.start, t.span.end);
        // (a label, `start:`, isn't a name used)
        if next.is_some_and(|n| n.kind == TokenType::Colon) && self.text[..start].rsplit('\n').next().is_some_and(|l| l.trim().is_empty()) {
            return;
        }
        if self.is_program_name(&t.lexeme, start) {
            return;
        }
        if let Some(b) = rapidr_lang::builtin(&t.lexeme) {
            let what = b.name.to_string();
            if b.missing {
                self.not_implemented(start, end, what.clone());
            } else if b.origin == Origin::RapidR {
                self.extension(start, end, what.clone());
            }
            self.one_runtime(start, end, what, b.runtimes);
            return;
        }
        if let Some((_, group)) = rapidr_lang::constant(&t.lexeme) {
            if group.origin == Origin::RapidR {
                self.extension(start, end, format!("{} (RapidR's constant)", t.lexeme));
            }
        }
    }
}

/// Fixes for compatibility diagnostics.
pub(crate) fn actions(diags: &[FileDiagnostic]) -> Vec<CodeAction> {
    let mut out = Vec::new();
    for d in diags {
        if matches!(d.code.as_deref(), Some(NOT_RAPIDQ_NAME | RAPIDR_NAME)) {
            if let Some(name) = d.message.rsplit(' ').next() {
                out.push(CodeAction {
                    title: format!("Write {name}"),
                    edit: vec![(d.file.clone(), vec![TextEdit { start: d.start, end: d.end, text: name.to_string() }])],
                    fixes: Some(d.clone()),
                    preferred: true,
                });
            }
        }
    }
    out
}

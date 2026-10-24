//! Hover: what a name is — the program's own (its declaration, kind and
//! type), a component's member, a builtin, a statement, a directive, a type
//! or a component (both its names and where it comes from).

use std::path::Path;

use rapidr_lang::Runtimes;

use crate::compat;
use crate::complete::{component_doc, with_notes};
use crate::context::{self, chain_before, Ty};
use crate::model::{name_key, ScopeKind, Symbol, SymbolKind};
use rapidr_lang::Origin;
use crate::text::{word_at, LineIndex};
use crate::{Hover, Snapshot};

pub(crate) fn hover(s: &Snapshot, file: &Path, text: &str, offset: usize) -> Option<Hover> {
    let (start, end) = word_at(text, offset)?;
    let word = &text[start..end];
    let index = LineIndex::new(text);
    let (line, _) = index.line_col(start);
    let line_start = index.line_start(line)?;
    let line_text = index.line_text(text, line);
    // (not in a comment or a string)
    let before = &text[line_start..start];
    let quotes = before.matches('"').count();
    if quotes % 2 == 1 || before.split('"').step_by(2).any(|part| part.contains('\'')) {
        return None;
    }
    let pre = s.pre_offset(file, start).unwrap_or(0);
    let hover = |markdown: String| Some(Hover { markdown, start, end });

    // `$INCLUDE`, `$APPTYPE` …
    if start > line_start && text.as_bytes()[start - 1] == b'$' && line_text.trim_start().starts_with('$') {
        if let Some(d) = rapidr_lang::directive(word) {
            return hover(format!("```rapidr\n{}\n```\n{}", d.syntax, with_notes(d.doc, &compat::notes(d.origin, Origin::RapidQ, false, Runtimes::All, None))));
        }
    }

    // A member: `Obj.Member`.
    if start > line_start && text.as_bytes()[start - 1] == b'.' {
        let chain = chain_before(text, line_start, start - 1);
        let ty = context::resolve_chain(s, pre, &chain)?;
        return member_hover(s, &ty, word).and_then(hover);
    }

    // The program's own names.
    if let Some(id) = s.model.symbol_at(pre).or_else(|| s.model.lookup(word, s.model.scope_at(pre))) {
        let sym = &s.model.symbols[id];
        if sym.kind != SymbolKind::Label || name_key(&sym.name) == name_key(word) {
            return hover(symbol_hover(s, sym));
        }
    }
    // A property set in a CREATE body (`Caption = "Hi"`).
    if let Some(c) = context::create_at(s, pre) {
        if let Some(md) = member_hover(s, &Ty::Component(c), word) {
            return hover(md);
        }
    }
    if let Some(c) = rapidr_lang::resolve_component(word) {
        return hover(format!("```rapidr\n{}\n```\n{}", c.spelling(), component_doc(c).unwrap_or_default()));
    }
    if let Some(g) = rapidr_lang::global(word) {
        let notes = compat::notes(g.origin, Origin::RapidQ, false, g.runtimes, g.from);
        return hover(format!("```rapidr\n{}\n```\n*global object*\n\n{}", g.name, with_notes(g.doc, &notes)));
    }
    if let Some(b) = rapidr_lang::builtin(word) {
        let mut md = format!("```rapidr\n{}\n```\n", b.syntax);
        md.push_str(&with_notes(b.doc, &compat::notes(b.origin, Origin::RapidQ, b.missing, b.runtimes, None)));
        return hover(md);
    }
    let lang = |syntax: &str, doc: &str, origin: Origin| {
        hover(format!("```rapidr\n{syntax}\n```\n{}", with_notes(doc, &compat::notes(origin, Origin::RapidQ, false, Runtimes::All, None))))
    };
    if let Some(t) = rapidr_lang::type_name(word) {
        return lang(t.name, t.doc, t.origin);
    }
    if let Some(k) = rapidr_lang::keyword(word) {
        return lang(k.name, k.doc, k.origin);
    }
    if let Some(st) = rapidr_lang::statement_starting(word) {
        return lang(st.syntax, st.doc, st.origin);
    }
    if let Some((value, group)) = rapidr_lang::constant(word) {
        let from = if group.origin == Origin::RapidR && rapidr_ast::builtin_constants::is_rapidr_only(word) {
            // (ctText …: built into a .rr program, nothing in a .bas)
            if s.rapidr_program() { "built in".to_string() } else { "RapidR's own: built into a .rr program only".to_string() }
        } else if group.origin == Origin::RapidR {
            "RapidR's own, always there".to_string()
        } else if s.rapidr_program() && rapidr_ast::builtin_constants::constant(word).is_some() {
            // (RAPIDQ.INC's, built into a RapidR program)
            "built in".to_string()
        } else {
            format!("from {}", group.source)
        };
        return hover(format!("```rapidr\nCONST {word} = {value}\n```\n*{}: {from}*\n\n{}", group.name, group.doc));
    }
    None
}

/// What the hover and the Help pane both say of one of the program's names.
pub(crate) struct Facts {
    /// Its declaration line (trimmed), the file it is in and the line (from 0).
    pub decl: Option<(String, std::path::PathBuf, usize)>,
    /// In words: "global variable", "SUB", "field of TYPE `T`" …
    pub what: String,
    /// What shows it: its declaration line; a SUB's or FUNCTION's signature.
    pub code: String,
    /// Uses besides the declaration.
    pub uses: usize,
}

pub(crate) fn symbol_facts(s: &Snapshot, sym: &Symbol) -> Facts {
    let decl = sym.decl.and_then(|d| s.locate(d)).and_then(|loc| {
        let text = s.parsed.file_text(&loc.file)?;
        let index = LineIndex::new(text);
        let (line, _) = index.line_col(loc.start);
        Some((index.line_text(text, line).trim().to_string(), loc.file.clone(), line))
    });
    let owner = match &s.model.scopes[sym.scope].kind {
        ScopeKind::Program => String::new(),
        ScopeKind::Routine(r) => format!(" in `{r}`"),
        ScopeKind::Type(t) => format!(" of TYPE `{t}`"),
    };
    let what = match sym.kind {
        // (RapidQ's implicit variables: the compiler declares one where it's
        // first used, not by a DIM)
        SymbolKind::Global if sym.implicit || decl.as_ref().is_some_and(|(line, _, _)| !declares(line)) => {
            "global variable (implicit: made by its first use)".to_string()
        }
        SymbolKind::Global => "global variable".to_string(),
        SymbolKind::Local => format!("local variable{owner}"),
        SymbolKind::Param => format!("parameter{owner}"),
        // (a SUB's own undeclared variable: RapidQ keeps it between calls)
        SymbolKind::Static if sym.implicit || decl.as_ref().is_some_and(|(line, _, _)| !declares(line)) => match &s.model.scopes[sym.scope].kind {
            ScopeKind::Routine(r) => format!("variable of `{r}` (implicit: its own, kept between calls)"),
            _ => format!("STATIC variable{owner}"),
        },
        SymbolKind::Static => format!("STATIC variable{owner}"),
        SymbolKind::Constant => match (&decl, rapidr_lang::constant(&sym.name)) {
            (None, Some((_, group))) => format!("constant of {}", group.source),
            _ => "constant".to_string(),
        },
        SymbolKind::Component => "component".to_string(),
        SymbolKind::Sub => format!("SUB{owner}"),
        SymbolKind::Function => format!("FUNCTION{owner}"),
        SymbolKind::External => "DLL routine (DECLARE … LIB)".to_string(),
        SymbolKind::Type => "TYPE".to_string(),
        SymbolKind::Field => format!("field{owner}"),
        SymbolKind::Label => "label".to_string(),
    };
    let code = match (&decl, sym.kind) {
        (Some((line, _, _)), SymbolKind::Sub | SymbolKind::Function | SymbolKind::External) => {
            context::routine_statement(&s.parsed.program, &sym.name).map(|st| crate::signature::routine_label(st).0).unwrap_or_else(|| line.clone())
        }
        (Some((line, _, _)), _) if !line.is_empty() => line.clone(),
        (None, SymbolKind::Constant) if rapidr_lang::constant(&sym.name).is_some() => {
            format!("CONST {} = {}", sym.name, rapidr_lang::constant(&sym.name).map_or(0, |(v, _)| v))
        }
        _ => match &sym.ty {
            Some(t) => format!("{} AS {t}", sym.name),
            None => sym.name.clone(),
        },
    };
    let uses = s
        .model
        .symbols
        .iter()
        .position(|x| std::ptr::eq(x, sym))
        .map_or(0, |id| s.model.references_to(id).filter(|r| r.access != crate::model::Access::Declare).count());
    Facts { decl, what, code, uses }
}

/// What RapidR Studio's Help pane (F1) says of one of the program's own
/// names: what it is, how it was declared, where, and its type (whose own
/// help the pane shows below).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolHelp {
    pub name: String,
    /// `variable`, `constant`, `parameter`, `sub`, `function`, `dll`, `type`,
    /// `field`, `component` or `label`.
    pub kind: &'static str,
    /// In words: "global variable", "local variable in `Go`" …
    pub what: String,
    /// Its declaration line; a SUB's or FUNCTION's signature.
    pub code: String,
    /// Its type's name (`RFONT`, `INTEGER`, a TYPE's), when it has one.
    pub ty: Option<String>,
    /// Where it is declared (`None`: it isn't, in the files).
    pub file: Option<std::path::PathBuf>,
    /// …its line, from 1 (0: none), and the column of its name there (from 1,
    /// in characters).
    pub line: usize,
    pub column: usize,
    /// Uses besides the declaration.
    pub uses: usize,
}

fn symbol_help_of(s: &Snapshot, sym: &Symbol) -> SymbolHelp {
    let Facts { decl, mut what, code, uses } = symbol_facts(s, sym);
    // (`DIM PickedFont AS RFont`: a variable that holds a component, made by
    // a DIM; a component of the form is made by CREATE)
    let dimmed = sym.kind == SymbolKind::Component && decl.as_ref().is_some_and(|(line, _, _)| declares(line));
    if dimmed {
        what = match &s.model.scopes[sym.scope].kind {
            ScopeKind::Program => "global variable".to_string(),
            ScopeKind::Routine(r) => format!("local variable in `{r}`"),
            ScopeKind::Type(t) => format!("field of TYPE `{t}`"),
        };
    }
    let kind = match sym.kind {
        SymbolKind::Global | SymbolKind::Local | SymbolKind::Static => "variable",
        SymbolKind::Param => "parameter",
        SymbolKind::Constant => "constant",
        SymbolKind::Component if dimmed => "variable",
        SymbolKind::Component => "component",
        SymbolKind::Sub => "sub",
        SymbolKind::Function => "function",
        SymbolKind::External => "dll",
        SymbolKind::Type => "type",
        SymbolKind::Field => "field",
        SymbolKind::Label => "label",
    };
    let (file, line) = match decl {
        Some((_, file, line)) => (Some(file), line + 1),
        None => (None, 0),
    };
    let column = sym
        .decl
        .and_then(|d| s.locate(d))
        .and_then(|loc| {
            let text = s.parsed.file_text(&loc.file)?;
            let index = LineIndex::new(text);
            let (line, _) = index.line_col(loc.start);
            Some(text.get(index.line_start(line)?..loc.start)?.chars().count() + 1)
        })
        .unwrap_or(1);
    SymbolHelp { name: rapidr_ast::strip_type_suffix(&sym.name).to_string(), kind, what, code, ty: sym.ty.clone(), file, line, column, uses }
}

/// The program's own name at byte `offset` of `file`'s `text` (the caret
/// in it, or just after it), if the word there is one.
pub(crate) fn help_at(s: &Snapshot, file: &Path, text: &str, offset: usize) -> Option<SymbolHelp> {
    let (start, end) = word_at(text, offset)?;
    // (a member of something: the registry's, not the program's)
    if start > 0 && text.as_bytes()[start - 1] == b'.' {
        return None;
    }
    let word = &text[start..end];
    let index = LineIndex::new(text);
    let (line, _) = index.line_col(start);
    let before = &text[index.line_start(line)?..start];
    if before.matches('"').count() % 2 == 1 || before.split('"').step_by(2).any(|part| part.contains('\'')) {
        return None;
    }
    let pre = s.pre_offset(file, start)?;
    let id = s.model.symbol_at(pre).or_else(|| s.model.lookup(word, s.model.scope_at(pre)))?;
    let sym = &s.model.symbols[id];
    if sym.kind == SymbolKind::Label && name_key(&sym.name) != name_key(word) {
        return None;
    }
    Some(symbol_help_of(s, sym))
}

/// The component the word at byte `offset` of `file`'s `text` is a member
/// of (`Hello.Font.Color`: the font's: `RFONT`), when it follows a dot and
/// the chain before it can be told; its registry name.
pub(crate) fn owner_at(s: &Snapshot, file: &Path, text: &str, offset: usize) -> Option<String> {
    let (start, _) = word_at(text, offset)?;
    if start == 0 || text.as_bytes()[start - 1] != b'.' {
        return None;
    }
    let index = LineIndex::new(text);
    let (line, _) = index.line_col(start);
    let line_start = index.line_start(line)?;
    let chain = chain_before(text, line_start, start - 1);
    let pre = s.pre_offset(file, start).unwrap_or(0);
    match context::resolve_chain(s, pre, &chain)? {
        Ty::Component(c) => Some(c.name.to_string()),
        Ty::User(t) => context::base_component(s, &t).map(|c| c.name.to_string()),
    }
}

/// The program's own name `name` (its main scope's: globals, constants,
/// components, SUBs, FUNCTIONs, TYPEs), as the Help pane's search finds it.
pub(crate) fn help_named(s: &Snapshot, name: &str) -> Option<SymbolHelp> {
    let id = s.model.lookup(name, 0)?;
    let sym = &s.model.symbols[id];
    (sym.kind != SymbolKind::Label).then(|| symbol_help_of(s, sym))
}

/// The names of the program's main scope, with their kinds.
pub(crate) fn program_names(s: &Snapshot) -> Vec<(String, &'static str)> {
    let mut out: Vec<(String, &'static str)> = Vec::new();
    for sym in s.model.symbols.iter().filter(|x| x.scope == 0 && x.kind != SymbolKind::Label) {
        let h = symbol_help_of(s, sym);
        if !out.iter().any(|(n, _)| n.eq_ignore_ascii_case(&h.name)) {
            out.push((h.name, h.kind));
        }
    }
    out
}

/// Markdown for one of the program's names.
pub(crate) fn symbol_hover(s: &Snapshot, sym: &Symbol) -> String {
    let Facts { decl, what, code, uses } = symbol_facts(s, sym);
    // (a top-level SUB / FUNCTION: its signature says what it is)
    let mut md = if what == "SUB" || what == "FUNCTION" { format!("```rapidr\n{code}\n```\n") } else { format!("```rapidr\n{code}\n```\n*{what}*") };
    if let Some(t) = &sym.ty {
        if let Some(c) = rapidr_lang::resolve_component(t) {
            md.push_str(&format!(" — {}", c.spelling()));
            if let Some(q) = c.rapidq {
                if !q.eq_ignore_ascii_case(c.name) {
                    md.push_str(&format!(" (RapidQ name: {q})"));
                }
            }
        } else if !matches!(sym.kind, SymbolKind::Sub | SymbolKind::Function | SymbolKind::External) && !code.to_ascii_uppercase().contains(&t.to_ascii_uppercase()) {
            md.push_str(&format!(" — `{t}`"));
        }
    }
    let used = match uses {
        0 => "not used yet".to_string(),
        1 => "used once".to_string(),
        n => format!("used {n} times"),
    };
    match decl {
        Some((_, file, line)) => {
            let name = file.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            md.push_str(&format!("\n\nDeclared in `{name}`, line {}; {used}.", line + 1));
        }
        None => md.push_str(&format!("\n\n{}{}.", used[..1].to_ascii_uppercase(), &used[1..])),
    }
    md
}

fn member_hover(s: &Snapshot, ty: &Ty, member: &str) -> Option<String> {
    match ty {
        Ty::Component(c) => {
            let owner = c.spelling();
            let (code, kind, doc, notes) = if let Some(p) = c.property(member) {
                let kind = match p.access {
                    rapidr_lang::Access::Read => format!("{} property (read only)", p.ty.as_str()),
                    rapidr_lang::Access::Write => format!("{} property (write only)", p.ty.as_str()),
                    rapidr_lang::Access::ReadWrite => format!("{} property", p.ty.as_str()),
                };
                // (RapidQ's value at creation after the owner: "string
                // property of QButton · default """)
                let kind = match p.default {
                    Some(d) => format!("{kind} of {owner}* · *{}", default_text(d)),
                    None => format!("{kind} of {owner}"),
                };
                let notes = compat::notes(p.origin, c.origin, p.missing, p.runtimes, p.from);
                (format!("{owner}.{}", p.name), kind, p.doc, notes)
            } else if let Some(m) = c.method(member) {
                (format!("{owner}.{}", m.signature()), format!("method of {owner}"), m.doc, compat::notes(m.origin, c.origin, m.missing, m.runtimes, m.from))
            } else {
                let e = c.event(member)?;
                (format!("{owner}.{}", e.signature()), format!("event of {owner}"), e.doc, compat::notes(e.origin, c.origin, e.missing, e.runtimes, e.from))
            };
            let mut md = format!("```rapidr\n{code}\n```\n*{kind}*");
            let text = with_notes(doc, &notes);
            if !text.is_empty() {
                md.push_str("\n\n");
                md.push_str(&text);
            }
            Some(md)
        }
        Ty::User(t) => {
            if let Some(sym) = context::user_member(s, t, member) {
                return Some(symbol_hover(s, &s.model.symbols[sym]));
            }
            let c = context::base_component(s, t)?;
            member_hover(s, &Ty::Component(c), member)
        }
    }
}

/// `default 0`, `default "text"`, `default alNone`: RapidQ's value at
/// creation (the registry's).
fn default_text(d: rapidr_lang::DefaultValue) -> String {
    use rapidr_lang::DefaultValue::*;
    match d {
        Int(n) => format!("default {n}"),
        Float(x) => format!("default {x}"),
        Bool(b) => format!("default {}", if b { "True" } else { "False" }),
        Str(s) => format!("default \"{s}\""),
        Expr(e) => format!("default {e}"),
    }
}

/// Whether a line declares names (DIM, CONST …) rather than using one.
fn declares(line: &str) -> bool {
    let first = line.split(|c: char| !c.is_ascii_alphanumeric()).next().unwrap_or("").to_ascii_uppercase();
    matches!(first.as_str(), "DIM" | "REDIM" | "STATIC" | "GLOBAL" | "CONST" | "PUBLIC" | "PRIVATE" | "SHARED" | "COMMON") || first.starts_with("DEF")
}

//! Hover: what a name is — the program's own (its declaration, kind and
//! type), a component's member, a builtin, a statement, a directive, a type
//! or a component (both its names and where it comes from).

use std::path::Path;

use rapidr_lang::Runtimes;

use crate::compat;
use crate::complete::{component_doc, with_notes};
use crate::context::{self, chain_before, pretty_component, Ty};
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
        return hover(format!("```rapidr\n{}\n```\n{}", pretty_component(&word.to_ascii_uppercase()), component_doc(c).unwrap_or_default()));
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
        let from = if group.origin == Origin::RapidR { "RapidR's own, always there".to_string() } else { format!("from {}", group.source) };
        return hover(format!("```rapidr\nCONST {word} = {value}\n```\n*{}: {from}*\n\n{}", group.name, group.doc));
    }
    None
}

/// Markdown for one of the program's names.
pub(crate) fn symbol_hover(s: &Snapshot, sym: &Symbol) -> String {
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
        SymbolKind::Static if decl.as_ref().is_some_and(|(line, _, _)| !declares(line)) => match &s.model.scopes[sym.scope].kind {
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
    // (a top-level SUB / FUNCTION: its signature says what it is)
    let mut md = if what == "SUB" || what == "FUNCTION" { format!("```rapidr\n{code}\n```\n") } else { format!("```rapidr\n{code}\n```\n*{what}*") };
    if let Some(t) = &sym.ty {
        if let Some(c) = rapidr_lang::resolve_component(t) {
            md.push_str(&format!(" — {}", pretty_component(c.written_name())));
            if let Some(q) = c.rapidq {
                if !q.eq_ignore_ascii_case(c.name) {
                    md.push_str(&format!(" (RapidR: {})", pretty_component(c.name)));
                }
            }
        } else if !matches!(sym.kind, SymbolKind::Sub | SymbolKind::Function | SymbolKind::External) && !code.to_ascii_uppercase().contains(&t.to_ascii_uppercase()) {
            md.push_str(&format!(" — `{t}`"));
        }
    }
    let uses = s
        .model
        .symbols
        .iter()
        .position(|x| std::ptr::eq(x, sym))
        .map_or(0, |id| s.model.references_to(id).filter(|r| r.access != crate::model::Access::Declare).count());
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
            let owner = pretty_component(c.written_name());
            let (code, kind, doc, notes) = if let Some(p) = c.property(member) {
                let mut kind = match p.access {
                    rapidr_lang::Access::Read => format!("{} property (read only)", p.ty.as_str()),
                    rapidr_lang::Access::Write => format!("{} property (write only)", p.ty.as_str()),
                    rapidr_lang::Access::ReadWrite => format!("{} property", p.ty.as_str()),
                };
                if let Some(d) = p.default {
                    kind.push_str(&format!(", {}", default_text(d)));
                }
                let notes = compat::notes(p.origin, c.origin, p.missing, p.runtimes, p.from);
                (format!("{owner}.{}", p.name), kind, p.doc, notes)
            } else if let Some(m) = c.method(member) {
                (format!("{owner}.{}", m.signature()), "method".to_string(), m.doc, compat::notes(m.origin, c.origin, m.missing, m.runtimes, m.from))
            } else {
                let e = c.event(member)?;
                (format!("{owner}.{}", e.signature()), "event".to_string(), e.doc, compat::notes(e.origin, c.origin, e.missing, e.runtimes, e.from))
            };
            let mut md = format!("```rapidr\n{code}\n```\n*{kind} of {owner}*");
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

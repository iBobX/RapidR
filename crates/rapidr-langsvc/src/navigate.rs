//! Go to definition, find references, rename, the outline and semantic
//! tokens: all from the semantic model's references (never a text search).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rapidr_ast::Statement;
use rapidr_diagnostics::TextSpan;

use crate::context::pretty_component;
use crate::model::{name_key, Access, SymbolId, SymbolKind};
use crate::text::{is_name_char, is_suffix_char, LineIndex};
use crate::{modifiers, Location, Options, OutlineItem, OutlineKind, SemanticToken, Snapshot, TextEdit, TokenKind, WorkspaceEdit};

/// The symbol whose name is at byte `offset` of `file`.
fn symbol_at(s: &Snapshot, file: &Path, offset: usize) -> Option<SymbolId> {
    let pre = s.pre_offset(file, offset)?;
    s.model.symbol_at(pre).or_else(|| pre.checked_sub(1).and_then(|p| s.model.symbol_at(p)))
}

pub(crate) fn definition(s: &Snapshot, file: &Path, text: &str, offset: usize, options: &Options) -> Vec<Location> {
    // `$INCLUDE "file"`: the file.
    let index = LineIndex::new(text);
    let (line, _) = index.line_col(offset);
    let line_text = index.line_text(text, line).trim_start();
    if line_text.get(..8).is_some_and(|w| w.eq_ignore_ascii_case("$include")) {
        let name = line_text[8..].trim().trim_matches(|c| c == '"' || c == '<' || c == '>' || c == '\'').trim();
        let base = file.parent().map(Path::to_path_buf).unwrap_or_default();
        let candidates = std::iter::once(base).chain(options.include_dirs.iter().cloned());
        for dir in candidates {
            let p = dir.join(name);
            if p.is_file() {
                return vec![Location { file: p, start: 0, end: 0 }];
            }
        }
        return Vec::new();
    }
    let Some(id) = symbol_at(s, file, offset) else { return Vec::new() };
    s.model.symbols[id].decl.and_then(|d| s.locate(d)).into_iter().collect()
}

pub(crate) fn references(s: &Snapshot, file: &Path, offset: usize, include_declaration: bool) -> Vec<Location> {
    let Some(id) = symbol_at(s, file, offset) else { return Vec::new() };
    s.model
        .references_to(id)
        .filter(|r| include_declaration || r.access != Access::Declare)
        .filter_map(|r| s.locate(r.span))
        .collect()
}

pub(crate) fn prepare_rename(s: &Snapshot, file: &Path, offset: usize) -> Result<Location, String> {
    let id = symbol_at(s, file, offset).ok_or("Nothing to rename here: put the caret on a name of the program")?;
    renamable(s, id)?;
    let pre = s.pre_offset(file, offset).ok_or("no position")?;
    let r = s.model.references.iter().find(|r| r.symbol == id && r.span.start <= pre && pre <= r.span.end).ok_or("no reference here")?;
    let mut loc = s.locate(r.span).ok_or("no position")?;
    // (the name without its type suffix)
    if let Some(text) = s.parsed.file_text(&loc.file) {
        while loc.end > loc.start && is_suffix_char(text.as_bytes()[loc.end - 1] as char) {
            loc.end -= 1;
        }
    }
    Ok(loc)
}

fn renamable(s: &Snapshot, id: SymbolId) -> Result<(), String> {
    let sym = &s.model.symbols[id];
    for r in s.model.references_to(id) {
        let loc = s.locate(r.span).ok_or_else(|| format!("{} is used where it can't be renamed", sym.name))?;
        if loc.file.starts_with("<RapidR>") {
            return Err(format!("{} belongs to a library RapidR supplies", sym.name));
        }
    }
    Ok(())
}

pub(crate) fn rename(s: &Snapshot, file: &Path, offset: usize, new_name: &str) -> Result<WorkspaceEdit, String> {
    let id = symbol_at(s, file, offset).ok_or("Nothing to rename here")?;
    renamable(s, id)?;
    let sym = &s.model.symbols[id];
    let new_base = rapidr_ast::strip_type_suffix(new_name);
    let valid = new_base.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && new_base.chars().all(is_name_char)
        && new_name[new_base.len()..].chars().all(is_suffix_char);
    if !valid {
        return Err(format!("'{new_name}' is not a valid name"));
    }
    if rapidr_lang::statement(new_base).is_some() || rapidr_lang::keyword(new_base).is_some() || rapidr_lang::type_name(new_base).is_some() {
        return Err(format!("'{new_name}' is a reserved word"));
    }
    if rapidr_lang::builtin(new_name).is_some() && !matches!(sym.kind, SymbolKind::Field | SymbolKind::Label) {
        return Err(format!("'{new_name}' is a builtin's name"));
    }
    if name_key(new_name) == name_key(&sym.name) {
        // (only the spelling changes)
    } else {
        // The new name must not mean something already wherever the symbol is used.
        let is_label = sym.kind == SymbolKind::Label;
        let mut scopes: Vec<usize> = s.model.references_to(id).map(|r| s.model.scope_at(r.span.start)).collect();
        scopes.push(sym.scope);
        for scope in scopes {
            let clash = if is_label {
                s.model.symbols.iter().position(|o| o.kind == SymbolKind::Label && (o.scope == scope || o.scope == 0) && name_key(&o.name) == name_key(new_name))
            } else {
                s.model.lookup(new_name, scope)
            };
            if let Some(other) = clash.filter(|&o| o != id) {
                let o = &s.model.symbols[other];
                return Err(format!("'{new_name}' is already the name of {} here", describe(o.kind, &o.name)));
            }
        }
        // …nor hide or be hidden by a name in a routine below it.
        if sym.scope == 0 {
            for (i, o) in s.model.symbols.iter().enumerate() {
                if i != id && o.scope != 0 && name_key(&o.name) == name_key(new_name) && s.model.references_to(id).any(|r| s.model.scope_at(r.span.start) == o.scope) {
                    return Err(format!("'{new_name}' would be hidden by {} in a SUB that uses it", describe(o.kind, &o.name)));
                }
            }
        }
    }
    let mut edits: BTreeMap<PathBuf, Vec<TextEdit>> = BTreeMap::new();
    for r in s.model.references_to(id) {
        let Some(loc) = s.locate(r.span) else { continue };
        let Some(text) = s.parsed.file_text(&loc.file) else { continue };
        let old = &text[loc.start..loc.end.min(text.len())];
        let old_base = rapidr_ast::strip_type_suffix(old);
        // (`n%` stays `x%`: the suffix written is kept unless the new name has one)
        let replacement = if new_name.len() > new_base.len() { new_name.to_string() } else { new_base.to_string() };
        let (start, end) = if new_name.len() > new_base.len() { (loc.start, loc.end) } else { (loc.start, loc.start + old_base.len()) };
        edits.entry(loc.file.clone()).or_default().push(TextEdit { start, end, text: replacement });
    }
    let mut out: WorkspaceEdit = edits.into_iter().collect();
    for (_, e) in &mut out {
        e.sort_by_key(|e| e.start);
        e.dedup_by_key(|e| e.start);
    }
    Ok(out)
}

fn describe(kind: SymbolKind, name: &str) -> String {
    let what = match kind {
        SymbolKind::Global => "the global variable",
        SymbolKind::Local => "the local variable",
        SymbolKind::Param => "the parameter",
        SymbolKind::Static => "the STATIC variable",
        SymbolKind::Constant => "the constant",
        SymbolKind::Component => "the component",
        SymbolKind::Sub => "the SUB",
        SymbolKind::Function => "the FUNCTION",
        SymbolKind::External => "the DLL routine",
        SymbolKind::Type => "the TYPE",
        SymbolKind::Field => "the field",
        SymbolKind::Label => "the label",
    };
    format!("{what} {name}")
}

// ---------------------------------------------------------------- outline

pub(crate) fn outline(s: &Snapshot, file: &Path) -> Vec<OutlineItem> {
    let Some(key) = s.parsed.file_key(file).map(Path::to_path_buf) else { return Vec::new() };
    let mut out = Vec::new();
    for st in &s.parsed.program.statements {
        outline_statement(s, &key, st, &mut out, true);
    }
    out
}

fn item(s: &Snapshot, file: &Path, span: TextSpan, name: &str, kind: OutlineKind, detail: Option<String>) -> Option<OutlineItem> {
    let range = s.locate_range(span)?;
    if range.file != file {
        return None;
    }
    let text = s.parsed.file_text(file)?;
    let name_span = find_name(text, range.start, range.end, name).unwrap_or((range.start, range.start));
    Some(OutlineItem {
        name: name.to_string(),
        detail,
        kind,
        start: range.start,
        end: range.end.max(name_span.1),
        name_start: name_span.0,
        name_end: name_span.1,
        children: Vec::new(),
    })
}

/// A name in a range of a file (not its first word: the keyword).
fn find_name(text: &str, start: usize, end: usize, name: &str) -> Option<(usize, usize)> {
    let hay = text.get(start..end)?.to_ascii_lowercase();
    let needle = rapidr_ast::strip_type_suffix(name).to_ascii_lowercase();
    let mut from = 0;
    while let Some(i) = hay[from..].find(&needle) {
        let at = from + i;
        let e = at + needle.len();
        let ok_before = at > 0 && !is_name_char(hay.as_bytes()[at - 1] as char);
        let ok_after = e >= hay.len() || !is_name_char(hay.as_bytes()[e] as char);
        if ok_before && ok_after {
            let mut e2 = e;
            while e2 < hay.len() && is_suffix_char(hay.as_bytes()[e2] as char) {
                e2 += 1;
            }
            return Some((start + at, start + e2));
        }
        from = e;
    }
    None
}

fn outline_statement(s: &Snapshot, file: &Path, st: &Statement, out: &mut Vec<OutlineItem>, top: bool) {
    match st {
        Statement::Subroutine(sub) => {
            let detail = crate::signature::routine_label(st).0;
            out.extend(item(s, file, sub.span, &sub.name, if top { OutlineKind::Sub } else { OutlineKind::Method }, Some(params_of(&detail))));
        }
        Statement::Function(f) => {
            let detail = crate::signature::routine_label(st).0;
            out.extend(item(s, file, f.span, &f.name, if top { OutlineKind::Function } else { OutlineKind::Method }, Some(params_of(&detail))));
        }
        Statement::Type(t) => {
            if let Some(mut it) = item(s, file, t.span, &t.name, OutlineKind::Type, t.extends.as_ref().map(|e| format!("EXTENDS {e}"))) {
                for f in &t.fields {
                    it.children.extend(item(s, file, f.span, &f.name, OutlineKind::Field, Some(f.type_name.clone())));
                }
                for m in &t.methods {
                    outline_statement(s, file, m, &mut it.children, false);
                }
                for e in &t.events {
                    it.children.extend(item(s, file, e.span, &e.name, OutlineKind::Event, None));
                }
                out.push(it);
            }
        }
        Statement::Create(c) => {
            // (RapidQ's components under RapidQ's names, RapidR's own under RapidR's)
            let shown = rapidr_lang::resolve_component(&c.type_name).map_or(c.type_name.clone(), |comp| pretty_component(comp.written_name()));
            if let Some(mut it) = item(s, file, c.span, &c.name, OutlineKind::Component, Some(shown)) {
                for inner in &c.body {
                    if let Statement::Create(_) = inner {
                        outline_statement(s, file, inner, &mut it.children, false);
                    }
                }
                out.push(it);
            }
        }
        Statement::Const(c) if top => out.extend(item(s, file, c.span, &c.name, OutlineKind::Constant, None)),
        Statement::Dim(d) if top => {
            for v in &d.declarators {
                let kind = if rapidr_lang::resolve_component(&d.type_name).is_some() { OutlineKind::Component } else { OutlineKind::Variable };
                let detail = (!d.type_name.is_empty()).then(|| d.type_name.clone());
                let span = if v.span.is_empty() { d.span } else { v.span };
                out.extend(item(s, file, span, &v.name, kind, detail));
            }
        }
        _ => {}
    }
}

/// `SUB Name(a AS INTEGER)` → `(a AS INTEGER)`.
fn params_of(label: &str) -> String {
    label.find('(').map_or(String::new(), |i| label[i..].to_string())
}

// ---------------------------------------------------------------- tokens

pub(crate) fn semantic_tokens(s: &Snapshot, file: &Path) -> Vec<SemanticToken> {
    let Some(key) = s.parsed.file_key(file).map(Path::to_path_buf) else { return Vec::new() };
    let mut out = Vec::new();
    for r in &s.model.references {
        let sym = &s.model.symbols[r.symbol];
        let Some(loc) = s.locate(r.span) else { continue };
        if loc.file != key || loc.end <= loc.start {
            continue;
        }
        let kind = match sym.kind {
            SymbolKind::Sub | SymbolKind::Function | SymbolKind::External => TokenKind::Function,
            SymbolKind::Global | SymbolKind::Local | SymbolKind::Static => TokenKind::Variable,
            SymbolKind::Param => TokenKind::Parameter,
            SymbolKind::Constant => TokenKind::Constant,
            SymbolKind::Component => TokenKind::Component,
            SymbolKind::Type => TokenKind::Type,
            SymbolKind::Field => TokenKind::Property,
            SymbolKind::Label => TokenKind::Label,
        };
        let mut m = 0;
        if r.access == Access::Declare {
            m |= modifiers::DECLARATION;
        }
        if sym.kind == SymbolKind::Constant {
            m |= modifiers::READONLY;
        }
        if sym.kind == SymbolKind::Static {
            m |= modifiers::STATIC;
        }
        if sym.scope == 0 && matches!(sym.kind, SymbolKind::Global) {
            m |= modifiers::GLOBAL;
        }
        out.push(SemanticToken { start: loc.start, end: loc.end, kind, modifiers: m });
    }
    out.sort_by_key(|t| (t.start, t.end));
    out.dedup_by(|b, a| b.start < a.end);
    out
}

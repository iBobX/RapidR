//! Signature help: the parameters of the SUB, FUNCTION, builtin or method
//! being called, and which one the caret is at — inside parentheses, or
//! after a SUB's name in a call without them (`MySub a, b`).

use std::path::Path;

use rapidr_ast::{Parameter, Statement};

use crate::context::{self, chain_before, Ty};
use crate::model::SymbolKind;
use rapidr_lang::Param;
use crate::text::{is_name_char, is_suffix_char, LineIndex};
use crate::{Signature, SignatureHelp, Snapshot};

pub(crate) fn signature(s: &Snapshot, file: &Path, text: &str, offset: usize) -> Option<SignatureHelp> {
    let offset = offset.min(text.len());
    let index = LineIndex::new(text);
    let (line, _) = index.line_col(offset);
    let line_start = index.line_start(line)?;
    let before = &text[line_start..offset];
    let bytes = text.as_bytes();

    // The innermost unclosed `(` before the caret (outside strings), and
    // the commas after it.
    let mut stack: Vec<(usize, usize)> = Vec::new(); // (paren offset, commas)
    let mut top_commas = 0;
    let mut in_string = false;
    for (i, c) in before.char_indices() {
        match c {
            '"' => in_string = !in_string,
            _ if in_string => {}
            '\'' => return None,
            '(' => stack.push((line_start + i, 0)),
            ')' => {
                stack.pop();
            }
            ',' => match stack.last_mut() {
                Some((_, n)) => *n += 1,
                None => top_commas += 1,
            },
            ':' if stack.is_empty() => top_commas = 0,
            _ => {}
        }
    }
    if in_string {
        // (a string argument being typed: still in the call)
    }
    let (name_end, active) = match stack.last() {
        Some(&(paren, commas)) => (paren, commas),
        None => {
            // `MySub a, b`: a statement starting with a SUB's name.
            let trimmed = before.trim_start();
            let lead = line_start + (before.len() - trimmed.len());
            let mut e = lead;
            while e < offset && (is_name_char(bytes[e] as char) || is_suffix_char(bytes[e] as char) || bytes[e] == b'.') {
                e += 1;
            }
            if e == lead || e >= offset || !bytes[e].is_ascii_whitespace() {
                return None;
            }
            (e, top_commas)
        }
    };
    // The callee: the name (chain) right before the parenthesis.
    let mut start = name_end;
    while start > line_start && bytes[start - 1] == b' ' {
        start -= 1;
    }
    let end = start;
    while start > line_start && (is_name_char(bytes[start - 1] as char) || is_suffix_char(bytes[start - 1] as char)) {
        start -= 1;
    }
    let name = &text[start..end];
    if name.is_empty() {
        return None;
    }
    let pre = s.pre_offset(file, start).unwrap_or(0);
    let sig = if start > line_start && bytes[start - 1] == b'.' {
        let chain = chain_before(text, line_start, start - 1);
        let ty = context::resolve_chain(s, pre, &chain)?;
        member_signature(s, &ty, name)?
    } else {
        let scope = s.model.scope_at(pre);
        match s.model.lookup(name, scope).map(|id| &s.model.symbols[id]) {
            Some(sym) if matches!(sym.kind, SymbolKind::Sub | SymbolKind::Function | SymbolKind::External) => {
                let st = context::routine_statement(&s.parsed.program, &sym.name)?;
                let (label, params) = routine_label(st);
                Signature { label, params, doc: None }
            }
            Some(_) => return None,
            None => {
                let b = rapidr_lang::builtin(name)?;
                let doc = crate::complete::with_notes(b.doc, &crate::compat::notes(b.origin, rapidr_lang::Origin::RapidQ, b.missing, b.runtimes, None));
                syntax_signature(b.syntax, doc)
            }
        }
    };
    let active_param = if sig.params.is_empty() { 0 } else { active.min(sig.params.len() - 1) };
    Some(SignatureHelp { signatures: vec![sig], active_signature: 0, active_param })
}

/// `SUB Name(a AS INTEGER, b$)` and each parameter's range in it.
pub(crate) fn routine_label(st: &Statement) -> (String, Vec<(usize, usize)>) {
    let (keyword, name, params, ret): (&str, &str, &[Parameter], Option<&str>) = match st {
        Statement::Subroutine(s) => ("SUB", &s.name, &s.params, None),
        Statement::Function(f) => ("FUNCTION", &f.name, &f.params, f.return_type.as_deref()),
        Statement::Declare(d) => (if d.is_function { "FUNCTION" } else { "SUB" }, &d.name, &d.params, d.return_type.as_deref()),
        _ => return (String::new(), Vec::new()),
    };
    let texts: Vec<String> = params
        .iter()
        .map(|p| {
            let mut t = String::new();
            if p.by_ref {
                t.push_str("BYREF ");
            }
            t.push_str(&p.name);
            if p.is_array {
                t.push_str("()");
            }
            if !p.type_name.is_empty() {
                t.push_str(" AS ");
                t.push_str(&p.type_name);
            }
            t
        })
        .collect();
    let mut label = format!("{keyword} {name}(");
    let ranges = push_params(&mut label, &texts);
    label.push(')');
    if let Some(r) = ret {
        label.push_str(" AS ");
        label.push_str(r);
    }
    (label, ranges)
}

fn push_params(label: &mut String, texts: &[String]) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    for (i, t) in texts.iter().enumerate() {
        if i > 0 {
            label.push_str(", ");
        }
        let start = label.len();
        label.push_str(t);
        ranges.push((start, label.len()));
    }
    ranges
}

/// A builtin's signature: the registry's syntax line, each parameter's
/// range found in it (`MID$(String, Position, Num)`, `LOCATE [Y%][, X%]`).
fn syntax_signature(syntax: &str, doc: String) -> Signature {
    let bytes = syntax.as_bytes();
    let name_end = syntax.find(['(', ' ', '[']).unwrap_or(syntax.len());
    let (inner_start, inner_end) = match syntax.find('(') {
        Some(open) if syntax[name_end..open].chars().all(|c| c == '[') => {
            let mut depth = 0;
            let mut close = syntax.len();
            for (i, &b) in bytes.iter().enumerate().skip(open) {
                match b {
                    b'(' => depth += 1,
                    b')' => {
                        depth -= 1;
                        if depth == 0 {
                            close = i;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            (open + 1, close)
        }
        _ => (name_end, syntax.len()),
    };
    let mut params = Vec::new();
    let mut piece_start = inner_start;
    let mut depth = 0;
    for i in inner_start..=inner_end {
        let at_end = i == inner_end;
        let b = if at_end { b',' } else { bytes[i] };
        match b {
            b'(' => depth += 1,
            b')' => depth -= 1,
            b',' if depth == 0 => {
                let piece = &syntax[piece_start..i];
                let lead = piece.len() - piece.trim_start_matches([' ', '[', ']']).len();
                let body = piece.trim_matches([' ', '[', ']']);
                if !body.is_empty() && body != "..." && body != "…" {
                    params.push((piece_start + lead, piece_start + lead + body.len()));
                }
                piece_start = i + 1;
            }
            _ => {}
        }
    }
    Signature { label: syntax.to_string(), params, doc: (!doc.is_empty()).then_some(doc) }
}

fn params_signature(syntax: &str, params: &[Param], doc: &str) -> Signature {
    let name = syntax.split('(').next().unwrap_or(syntax);
    let texts: Vec<String> = params.iter().map(Param::text).collect();
    let mut label = format!("{name}(");
    let ranges = push_params(&mut label, &texts);
    label.push(')');
    Signature { label, params: ranges, doc: (!doc.is_empty()).then(|| doc.to_string()) }
}

fn member_signature(s: &Snapshot, ty: &Ty, name: &str) -> Option<Signature> {
    match ty {
        Ty::Component(c) => {
            let m = c.method(name)?;
            let mut sig = params_signature(&format!("{}.{}", c.spelling(), m.name), m.params, m.doc);
            if let Some(r) = m.returns {
                sig.label.push_str(" AS ");
                sig.label.push_str(r);
            }
            Some(sig)
        }
        Ty::User(t) => {
            if let Some(sym) = context::user_member(s, t, name) {
                let owner = match &s.model.scopes[s.model.symbols[sym].scope].kind {
                    crate::model::ScopeKind::Type(o) => o.clone(),
                    _ => t.clone(),
                };
                let st = type_method(&s.parsed.program.statements, &owner, name)?;
                let (label, params) = routine_label(st);
                return Some(Signature { label, params, doc: None });
            }
            let c = context::base_component(s, t)?;
            member_signature(s, &Ty::Component(c), name)
        }
    }
}

fn type_method<'p>(statements: &'p [Statement], type_name: &str, name: &str) -> Option<&'p Statement> {
    statements.iter().find_map(|st| match st {
        Statement::Type(t) if t.name.eq_ignore_ascii_case(type_name) => t.methods.iter().find(|m| match m {
            Statement::Subroutine(s) => s.name.eq_ignore_ascii_case(name),
            Statement::Function(f) => f.name.eq_ignore_ascii_case(name),
            _ => false,
        }),
        _ => None,
    })
}

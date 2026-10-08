//! What the caret is in: the line's text around it (a member access, a
//! type after `AS`, a directive, a string …), the blocks around it (a
//! CREATE, a WITH, a SUB) and the types of names (`Form.` is a QFORM).

use rapidr_ast::{Expression, Program, Statement};

use crate::model::{name_key, ScopeId, ScopeKind, SymbolKind};
use rapidr_lang::Component;
use crate::text::{is_name_char, is_suffix_char, LineIndex};
use crate::Snapshot;

/// What the caret's line says about the place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Place {
    /// In a string or a comment.
    Nothing,
    /// `$IN|`: a directive.
    Directive,
    /// `DIM x AS Q|`: a type.
    AfterAs,
    /// `a.b.|`: members of what the names before the dot are; an empty
    /// first name is a WITH's `.`.
    Member(Vec<String>),
    /// `GOTO |`: a label.
    Label,
    /// Anywhere else; whether a statement starts here.
    Code { statement_start: bool },
}

#[derive(Debug, Clone)]
pub(crate) struct LineContext {
    pub place: Place,
    /// The word typed so far (file bytes).
    pub word_start: usize,
    pub word_end: usize,
}

/// The context of byte `offset` of `text`.
pub(crate) fn line_context(text: &str, offset: usize) -> LineContext {
    let offset = offset.min(text.len());
    let index = LineIndex::new(text);
    let (line, _) = index.line_col(offset);
    let line_start = index.line_start(line).unwrap_or(0);
    let before = &text[line_start..offset];
    let bytes = text.as_bytes();
    let mut word_start = offset;
    while word_start > line_start && is_suffix_char(bytes[word_start - 1] as char) {
        word_start -= 1;
    }
    while word_start > line_start && is_name_char(bytes[word_start - 1] as char) {
        word_start -= 1;
    }
    if word_start < offset && !is_name_char(bytes[word_start] as char) {
        // (a `$` alone is not a name's suffix)
        word_start = offset;
    }
    let mut word_end = offset;
    while word_end < bytes.len() && is_name_char(bytes[word_end] as char) {
        word_end += 1;
    }
    let ctx = |place| LineContext { place, word_start, word_end };

    // In a string or after a comment's quote?
    let mut in_string = false;
    for (i, c) in before.char_indices() {
        match c {
            '"' => in_string = !in_string,
            '\'' if !in_string => {
                let _ = i;
                return ctx(Place::Nothing);
            }
            _ => {}
        }
    }
    if in_string {
        return ctx(Place::Nothing);
    }
    let head = &text[line_start..word_start];
    let head_trim = head.trim_start();
    if head_trim.get(..4).is_some_and(|w| w.eq_ignore_ascii_case("rem ")) {
        return ctx(Place::Nothing);
    }
    if head_trim == "$" {
        return LineContext { place: Place::Directive, word_start: word_start - 1, word_end };
    }
    if word_start > line_start && bytes[word_start - 1] == b'.' {
        return ctx(Place::Member(chain_before(text, line_start, word_start - 1)));
    }
    let last_word = |s: &str| -> String {
        let t = s.trim_end();
        let start = t.rfind(|c: char| !is_name_char(c)).map_or(0, |i| i + 1);
        t[start..].to_ascii_uppercase()
    };
    let prev = last_word(head);
    if prev == "AS" && head.ends_with(|c: char| c.is_whitespace()) {
        return ctx(Place::AfterAs);
    }
    if matches!(prev.as_str(), "GOTO" | "GOSUB" | "RESTORE") && head.ends_with(|c: char| c.is_whitespace()) {
        return ctx(Place::Label);
    }
    let trimmed = head.trim_end();
    let statement_start = trimmed.trim_start().is_empty()
        || trimmed.ends_with(':')
        || matches!(prev.as_str(), "THEN" | "ELSE") && head.ends_with(|c: char| c.is_whitespace());
    ctx(Place::Code { statement_start })
}

/// The names before the dot at `dot` (`Form.Font.` → [Form, Font]; a
/// WITH's leading `.` → [""]). Index expressions are skipped
/// (`a(i).b` → [a, b]).
pub(crate) fn chain_before(text: &str, line_start: usize, dot: usize) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut chain = Vec::new();
    let mut i = dot;
    loop {
        let mut j = i;
        if j > line_start && bytes[j - 1] == b')' {
            let mut depth = 0;
            while j > line_start {
                j -= 1;
                match bytes[j] {
                    b')' => depth += 1,
                    b'(' => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
            }
        }
        let end = j;
        while j > line_start && (is_name_char(bytes[j - 1] as char) || is_suffix_char(bytes[j - 1] as char)) {
            j -= 1;
        }
        let name = &text[j..end];
        if name.is_empty() {
            chain.insert(0, String::new());
            break;
        }
        chain.insert(0, name.to_string());
        if j > line_start && bytes[j - 1] == b'.' {
            i = j - 1;
        } else {
            break;
        }
    }
    chain
}

/// A type the service knows the members of.
#[derive(Debug, Clone)]
pub(crate) enum Ty {
    Component(&'static Component),
    /// A TYPE of the program (its name as declared).
    User(String),
}

impl Ty {
    pub fn from_name(s: &Snapshot, name: &str) -> Option<Ty> {
        if let Some(c) = rapidr_lang::resolve_component(name) {
            return Some(Ty::Component(c));
        }
        let key = name_key(name);
        s.model.symbols.iter().find(|sym| sym.kind == SymbolKind::Type && name_key(&sym.name) == key).map(|sym| Ty::User(sym.name.clone()))
    }
}

/// The scope of a TYPE (by name).
pub(crate) fn type_scope(s: &Snapshot, name: &str) -> Option<ScopeId> {
    let key = name_key(name);
    s.model.scopes.iter().position(|sc| matches!(&sc.kind, ScopeKind::Type(t) if name_key(t) == key))
}

/// A TYPE and the TYPEs it extends, nearest first.
pub(crate) fn type_chain(s: &Snapshot, type_name: &str) -> Vec<ScopeId> {
    let mut out = Vec::new();
    let mut at = type_scope(s, type_name);
    while let Some(scope) = at {
        if out.contains(&scope) {
            break;
        }
        out.push(scope);
        at = s.model.scopes[scope].parent.filter(|&p| p != 0);
    }
    out
}

/// A field or method of a TYPE or of the TYPEs it extends.
pub(crate) fn user_member(s: &Snapshot, type_name: &str, member: &str) -> Option<crate::model::SymbolId> {
    let key = name_key(member);
    type_chain(s, type_name)
        .into_iter()
        .find_map(|scope| s.model.symbols.iter().position(|sym| sym.scope == scope && name_key(&sym.name) == key))
}

/// The component a TYPE extends (directly or through other TYPEs).
pub(crate) fn base_component(s: &Snapshot, type_name: &str) -> Option<&'static Component> {
    let mut name = type_name.to_string();
    for _ in 0..16 {
        let key = name_key(&name);
        let sym = s.model.symbols.iter().find(|sym| sym.kind == SymbolKind::Type && name_key(&sym.name) == key)?;
        let base = sym.ty.clone()?;
        if let Some(c) = rapidr_lang::resolve_component(&base) {
            return Some(c);
        }
        name = base;
    }
    None
}

/// The type of a member of a type.
pub(crate) fn member_type(s: &Snapshot, ty: &Ty, member: &str) -> Option<Ty> {
    match ty {
        // (the registry's property types: a font, a component, the item
        // object an indexed property gives — `Tree.Item(i).`)
        Ty::Component(c) => {
            let p = c.property(member)?;
            match p.ty {
                _ if member.eq_ignore_ascii_case("font") => rapidr_lang::component("RFONT").map(Ty::Component),
                rapidr_lang::Type::Font => rapidr_lang::component("RFONT").map(Ty::Component),
                rapidr_lang::Type::Component | rapidr_lang::Type::Item => {
                    let kind = p.kinds.first()?;
                    rapidr_lang::component(kind).or_else(|| rapidr_lang::item(kind)).map(Ty::Component)
                }
                _ => None,
            }
        }
        Ty::User(t) => {
            if let Some(sym) = user_member(s, t, member) {
                return Ty::from_name(s, s.model.symbols[sym].ty.as_deref()?);
            }
            base_component(s, t).and_then(|c| member_type(s, &Ty::Component(c), member))
        }
    }
}

/// The type of what a chain of names means at a preprocessed offset.
pub(crate) fn resolve_chain(s: &Snapshot, pre: usize, chain: &[String]) -> Option<Ty> {
    let first = chain.first()?;
    let scope = s.model.scope_at(pre);
    let mut ty = if first.is_empty() {
        with_type(s, pre)?
    } else if first.eq_ignore_ascii_case("this") {
        Ty::User(enclosing_type(s, scope)?)
    } else if first.eq_ignore_ascii_case("super") {
        let t = enclosing_type(s, scope)?;
        let key = name_key(&t);
        let base = s.model.symbols.iter().find(|sym| sym.kind == SymbolKind::Type && name_key(&sym.name) == key)?.ty.clone()?;
        Ty::from_name(s, &base)?
    } else {
        match s.model.lookup(first, scope) {
            Some(sym) => Ty::from_name(s, s.model.symbols[sym].ty.as_deref()?)?,
            // RapidQ's global objects (`Screen.`, `Printer.`)
            None => Ty::Component(rapidr_lang::global(first)?),
        }
    };
    for m in &chain[1..] {
        ty = member_type(s, &ty, m)?;
    }
    Some(ty)
}

/// The TYPE whose method a scope is (or is in).
pub(crate) fn enclosing_type(s: &Snapshot, scope: ScopeId) -> Option<String> {
    let mut at = Some(scope);
    while let Some(sc) = at {
        if let ScopeKind::Type(t) = &s.model.scopes[sc].kind {
            return Some(t.clone());
        }
        at = s.model.scopes[sc].parent;
    }
    None
}

/// The innermost block statements holding a preprocessed offset, outer
/// first (SUBs, FUNCTIONs, TYPEs, CREATEs, WITHs and the blocks between).
pub(crate) fn blocks_at(program: &Program, pre: usize) -> Vec<&Statement> {
    let mut out = Vec::new();
    collect_blocks(&program.statements, pre, &mut out);
    out
}

fn collect_blocks<'p>(statements: &'p [Statement], pre: usize, out: &mut Vec<&'p Statement>) {
    for st in statements {
        let span = rapidr_parser::statement_span(st);
        if !(span.start <= pre && pre <= span.end) {
            continue;
        }
        let bodies: Vec<&[Statement]> = match st {
            Statement::Subroutine(s) => vec![&s.body],
            Statement::Function(f) => vec![&f.body],
            Statement::Create(c) => vec![&c.body],
            Statement::With(w) => vec![&w.body],
            Statement::Type(t) => {
                let mut v: Vec<&[Statement]> = vec![&t.methods, &t.constructor];
                v.extend(t.events.iter().map(|e| e.body.as_slice()));
                v
            }
            Statement::If(i) => {
                let mut v: Vec<&[Statement]> = vec![&i.then_body, &i.else_body];
                v.extend(i.elseif_branches.iter().map(|b| b.body.as_slice()));
                v
            }
            Statement::For(f) => vec![&f.body],
            Statement::While(w) => vec![&w.body],
            Statement::DoLoop(d) => vec![&d.body],
            Statement::SelectCase(s) => {
                let mut v: Vec<&[Statement]> = s.cases.iter().map(|c| c.body.as_slice()).collect();
                v.push(&s.case_else);
                v
            }
            _ => continue,
        };
        out.push(st);
        for b in bodies {
            collect_blocks(b, pre, out);
        }
    }
}

/// The type of the innermost WITH's object at a preprocessed offset.
pub(crate) fn with_type(s: &Snapshot, pre: usize) -> Option<Ty> {
    let blocks = blocks_at(&s.parsed.program, pre);
    let w = blocks.iter().rev().find_map(|b| match b {
        Statement::With(w) => Some(w),
        _ => None,
    })?;
    expr_type(s, &w.object, pre)
}

/// The type of an expression (names, members, `This`).
pub(crate) fn expr_type(s: &Snapshot, e: &Expression, pre: usize) -> Option<Ty> {
    let chain = expr_chain(e)?;
    resolve_chain(s, pre, &chain)
}

/// `a.b(i).c` → [a, b, c].
fn expr_chain(e: &Expression) -> Option<Vec<String>> {
    match e {
        Expression::Identifier(id) => Some(vec![if id.name == "_with_" { String::new() } else { id.name.clone() }]),
        Expression::MemberAccess(m) => {
            let mut c = expr_chain(&m.object)?;
            c.push(m.member.clone());
            Some(c)
        }
        Expression::ArrayAccess(a) => expr_chain(&a.array),
        Expression::FunctionCall(f) => expr_chain(&f.callee),
        _ => None,
    }
}

/// The component a CREATE body at the caret sets properties of (only when
/// the caret is directly in the body, not in a nested block).
pub(crate) fn create_at(s: &Snapshot, pre: usize) -> Option<&'static Component> {
    let blocks = blocks_at(&s.parsed.program, pre);
    match blocks.last()? {
        Statement::Create(c) => rapidr_lang::resolve_component(&c.type_name),
        _ => None,
    }
}

/// Pretty spelling of a component name (`RBUTTON` → `RButton`,
/// `QFORMMDI` → `QFormMDI`): the registry's mixed-case spelling
/// (`rapidr_lang::Component::display`).
pub(crate) fn pretty_component(upper: &str) -> String {
    if let Some(c) = rapidr_lang::resolve_component(upper) {
        let p = c.pretty(upper);
        if p != upper {
            return p;
        }
    }
    if upper.len() < 2 {
        return upper.to_string();
    }
    // (an alias, or a name the registry doesn't know: its first letter
    // after the Q / R kept)
    let (first, rest) = upper.split_at(1);
    if rest.is_ascii() {
        format!("{first}{}{}", &rest[..1], rest[1..].to_ascii_lowercase())
    } else {
        upper.to_string()
    }
}

/// The routine statement a SUB / FUNCTION symbol was declared by (params,
/// return type), searched in the program and its TYPEs.
pub(crate) fn routine_statement<'p>(program: &'p Program, name: &str) -> Option<&'p Statement> {
    fn find<'p>(statements: &'p [Statement], key: &str) -> Option<&'p Statement> {
        for st in statements {
            match st {
                Statement::Subroutine(s) if name_key(&s.name) == key => return Some(st),
                Statement::Function(f) if name_key(&f.name) == key => return Some(st),
                Statement::Declare(d) if name_key(&d.name) == key && d.lib.is_some() => return Some(st),
                Statement::Type(t) => {
                    if let Some(found) = find(&t.methods, key) {
                        return Some(found);
                    }
                }
                _ => {}
            }
        }
        None
    }
    find(&program.statements, &name_key(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn places_on_a_line() {
        let t = "  Form.Cap";
        assert_eq!(line_context(t, t.len()).place, Place::Member(vec!["Form".into()]));
        let t = "x = a(i).b.";
        assert_eq!(line_context(t, t.len()).place, Place::Member(vec!["a".into(), "b".into()]));
        let t = "  .Cap";
        assert_eq!(line_context(t, t.len()).place, Place::Member(vec![String::new()]));
        let t = "DIM b AS QBu";
        assert_eq!(line_context(t, t.len()).place, Place::AfterAs);
        let t = "PRINT \"a.b";
        assert_eq!(line_context(t, t.len()).place, Place::Nothing);
        let t = "x = 1 ' Form.";
        assert_eq!(line_context(t, t.len()).place, Place::Nothing);
        let t = "$INC";
        assert_eq!(line_context(t, t.len()).place, Place::Directive);
        let t = "  Pri";
        assert_eq!(line_context(t, t.len()).place, Place::Code { statement_start: true });
        let t = "x = Le";
        assert_eq!(line_context(t, t.len()).place, Place::Code { statement_start: false });
    }

    #[test]
    fn component_names_are_spelled_as_written() {
        assert_eq!(pretty_component("QBUTTON"), "QButton");
        assert_eq!(pretty_component("RFORMMDI"), "RFormMDI");
        assert_eq!(pretty_component("RXYZ"), "RXyz");
    }
}

//! The program's names: where each is declared and where it's used.
//!
//! **Interim seam.** The types below are exactly those of
//! `rapidr_bcgen::semantic` (lane L-PARSE: the semantic model exported from
//! the compiler's own scope tables, so the compiler and the IDE can't
//! disagree). Until it lands, [`analyze`] builds the same model by walking
//! the AST with the compiler's rules for names (a component, then a local,
//! then a global; `DIM` in the main program is global, in a SUB local; a
//! name never declared is RapidQ's implicit global). When it lands this
//! file becomes `pub use rapidr_bcgen::semantic::*;` and nothing else in
//! the service changes.

use std::collections::HashMap;

use rapidr_ast::{CaseValue, Expression, Program, Statement};
use rapidr_diagnostics::TextSpan;

use crate::registry;

pub type SymbolId = usize;
pub type ScopeId = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    Global,
    Local,
    Param,
    /// `STATIC` in a SUB / FUNCTION: one variable for every call.
    Static,
    Constant,
    Component,
    Sub,
    Function,
    /// `DECLARE … LIB`: a routine of a DLL.
    External,
    Type,
    Field,
    Label,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Access {
    /// The declaration itself.
    Declare,
    Read,
    Write,
    Call,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    /// As first written.
    pub name: String,
    pub kind: SymbolKind,
    /// The declared type (`INTEGER`, `RFORM`, a TYPE's name …), or the one
    /// its suffix gives; `None`: a variant.
    pub ty: Option<String>,
    /// The name in its declaration; `None` when it's only used (an
    /// implicit global).
    pub decl: Option<TextSpan>,
    pub scope: ScopeId,
    /// Never declared: made by its first use (RapidQ's implicit variables).
    pub implicit: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeKind {
    /// The main program: globals, constants, components, routines, TYPEs.
    Program,
    /// A SUB or FUNCTION (its name).
    Routine(String),
    /// A TYPE's fields.
    Type(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    pub kind: ScopeKind,
    pub parent: Option<ScopeId>,
    /// The SUB / FUNCTION / TYPE statement (the whole program for scope 0).
    pub span: TextSpan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reference {
    pub span: TextSpan,
    pub symbol: SymbolId,
    pub access: Access,
}

/// Every name of a program, where it's declared and where it's used.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SemanticModel {
    pub scopes: Vec<Scope>,
    pub symbols: Vec<Symbol>,
    /// Sorted by span start; declarations included (`Access::Declare`).
    pub references: Vec<Reference>,
}

impl SemanticModel {
    /// The symbol whose name is at byte `offset`.
    pub fn symbol_at(&self, offset: usize) -> Option<SymbolId> {
        let i = self.references.partition_point(|r| r.span.end <= offset);
        self.references.get(i).filter(|r| r.span.start <= offset && offset < r.span.end).map(|r| r.symbol)
    }

    /// Every use of a symbol (its declaration among them).
    pub fn references_to(&self, symbol: SymbolId) -> impl Iterator<Item = &Reference> {
        self.references.iter().filter(move |r| r.symbol == symbol)
    }

    /// The innermost scope holding byte `offset`.
    pub fn scope_at(&self, offset: usize) -> ScopeId {
        self.scopes
            .iter()
            .enumerate()
            .skip(1)
            .filter(|(_, s)| s.span.start <= offset && offset < s.span.end)
            .min_by_key(|(_, s)| s.span.len())
            .map_or(0, |(i, _)| i)
    }

    /// What `name` means in a scope: the scope's own symbol, else the
    /// program's (case-insensitive, suffixes ignored).
    pub fn lookup(&self, name: &str, scope: ScopeId) -> Option<SymbolId> {
        let key = name_key(name);
        let find = |scope: ScopeId| self.symbols.iter().position(|s| s.scope == scope && s.kind != SymbolKind::Label && name_key(&s.name) == key);
        let mut at = Some(scope);
        while let Some(scope) = at {
            if let Some(found) = find(scope) {
                return Some(found);
            }
            at = self.scopes.get(scope).and_then(|s| s.parent);
        }
        None
    }

    /// The symbols visible in a scope (its own, then the program's).
    pub fn visible(&self, scope: ScopeId) -> Vec<SymbolId> {
        let mut out: Vec<SymbolId> = (0..self.symbols.len()).filter(|&i| self.symbols[i].scope == scope).collect();
        let mut at = self.scopes.get(scope).and_then(|s| s.parent);
        while let Some(outer) = at {
            out.extend((0..self.symbols.len()).filter(|&i| self.symbols[i].scope == outer && self.lookup(&self.symbols[i].name, scope) == Some(i)));
            at = self.scopes.get(outer).and_then(|s| s.parent);
        }
        out
    }
}

/// BASIC names are case-insensitive and may carry a type suffix.
pub fn name_key(name: &str) -> String {
    rapidr_ast::strip_type_suffix(&name.to_ascii_lowercase()).to_string()
}

/// The semantic model of a parsed program. `source` is the text it was
/// parsed from (its spans count its bytes).
pub fn analyze(program: &Program, source: Option<&str>) -> SemanticModel {
    let mut b = Builder { source: source.unwrap_or(""), model: SemanticModel::default(), names: HashMap::new(), types: HashMap::new() };
    let whole = TextSpan::new(0, source.map_or(program.span.end, str::len).max(program.span.end));
    b.model.scopes.push(Scope { kind: ScopeKind::Program, parent: None, span: whole });
    b.declare_main(&program.statements);
    let ctx = Ctx { scope: 0, in_main: true, function: None, this_type: None, with: Vec::new() };
    b.walk_body(&program.statements, &ctx, false);
    b.model.references.sort_by_key(|r| (r.span.start, r.span.end));
    b.model.references.dedup_by_key(|r| (r.span.start, r.span.end));
    b.model
}

// ---------------------------------------------------------------- building

struct Builder<'a> {
    source: &'a str,
    model: SemanticModel,
    /// (scope, name key, is a label) → symbol
    names: HashMap<(ScopeId, String, bool), SymbolId>,
    /// TYPE name key → its scope
    types: HashMap<String, ScopeId>,
}

#[derive(Clone)]
struct Ctx {
    scope: ScopeId,
    in_main: bool,
    /// The FUNCTION being walked (its symbol: `Result` and its own name).
    function: Option<SymbolId>,
    /// The TYPE whose method is being walked (`This`).
    this_type: Option<String>,
    /// The types of the open WITH blocks' objects, innermost last.
    with: Vec<Option<String>>,
}

/// Words a statement starts with: a name searched in its span is never the
/// first word.
const LEADING_KEYWORDS: &[&str] = &[
    "dim", "redim", "static", "const", "sub", "function", "subi", "functioni", "create", "type", "declare", "defint", "defstr", "defdbl", "defsng",
    "deflng", "defbyte", "defword", "defdword", "for", "goto", "gosub", "callback", "call", "private", "public", "property", "event", "extends",
];

impl Builder<'_> {
    fn declare(&mut self, scope: ScopeId, kind: SymbolKind, name: &str, ty: Option<String>, decl: Option<TextSpan>, implicit: bool) -> SymbolId {
        let key = (scope, name_key(name), kind == SymbolKind::Label);
        if let Some(&id) = self.names.get(&key) {
            let s = &mut self.model.symbols[id];
            if s.decl.is_none() && decl.is_some() {
                s.decl = decl;
                s.implicit = false;
                if let Some(d) = decl {
                    self.model.references.push(Reference { span: d, symbol: id, access: Access::Declare });
                }
            }
            return id;
        }
        let id = self.model.symbols.len();
        self.model.symbols.push(Symbol { name: name.to_string(), kind, ty: ty.filter(|t| !t.is_empty()), decl, scope, implicit });
        self.names.insert(key, id);
        if let Some(d) = decl {
            self.model.references.push(Reference { span: d, symbol: id, access: Access::Declare });
        }
        id
    }

    fn reference(&mut self, span: TextSpan, symbol: SymbolId, access: Access) {
        if span.is_empty() || span.end > self.source.len() {
            return;
        }
        self.model.references.push(Reference { span, symbol, access });
    }

    /// The name inside `span` (whole words; a statement's first word is
    /// its keyword), with its type suffix.
    fn name_span(&self, span: TextSpan, name: &str) -> Option<TextSpan> {
        let text = self.source.get(span.start..span.end)?;
        let base = rapidr_ast::strip_type_suffix(name).to_ascii_lowercase();
        if base.is_empty() {
            return None;
        }
        let lower = text.to_ascii_lowercase();
        let first_word_end = lower.find(|c: char| !crate::text::is_name_char(c)).unwrap_or(lower.len());
        let skip_first = LEADING_KEYWORDS.contains(&&lower[..first_word_end]);
        let bytes = lower.as_bytes();
        let mut from = 0;
        while let Some(i) = lower[from..].find(&base) {
            let at = from + i;
            let end = at + base.len();
            let before_ok = at == 0 || !crate::text::is_name_char(bytes[at - 1] as char) && bytes[at - 1] != b'.';
            let after_ok = end >= bytes.len() || !crate::text::is_name_char(bytes[end] as char);
            // (inside a string or a comment: not the name)
            let in_text = lower[..at].matches('"').count() % 2 == 1 || lower[..at].contains('\'');
            if before_ok && after_ok && !in_text && !(skip_first && at == 0) {
                let mut e = end;
                while e < bytes.len() && crate::text::is_suffix_char(bytes[e] as char) {
                    e += 1;
                }
                return Some(TextSpan::new(span.start + at, span.start + e));
            }
            from = end;
        }
        None
    }

    /// The span of an identifier if the source there really holds it
    /// (the parser also makes names of its own, with empty spans).
    fn ident_span(&self, span: TextSpan, name: &str) -> Option<TextSpan> {
        let text = self.source.get(span.start..span.end)?;
        let base = rapidr_ast::strip_type_suffix(name);
        if span.is_empty() || base.is_empty() {
            return None;
        }
        if text.eq_ignore_ascii_case(name) || rapidr_ast::strip_type_suffix(text).eq_ignore_ascii_case(base) {
            return Some(span);
        }
        self.name_span(span, name)
    }

    /// A member's name at the end of an `Obj.Member` expression.
    fn member_span(&self, span: TextSpan, member: &str) -> Option<TextSpan> {
        let text = self.source.get(span.start..span.end)?.to_ascii_lowercase();
        let m = member.to_ascii_lowercase();
        let mut at = text.rfind(&format!(".{m}"))?;
        // (the member of the outermost access: the last one written)
        while let Some(next) = text[at + 1..].find(&format!(".{m}")) {
            at += 1 + next;
        }
        let start = span.start + at + 1;
        let mut end = start + m.len();
        let bytes = self.source.as_bytes();
        while end < span.end && crate::text::is_suffix_char(bytes[end] as char) {
            end += 1;
        }
        Some(TextSpan::new(start, end))
    }

    fn type_of_decl(&self, type_name: &str, name: &str) -> Option<String> {
        if type_name.is_empty() {
            return rapidr_ast::suffix_type(name).map(str::to_string);
        }
        Some(canonical_type(type_name))
    }

    // ------------------------------------------------ program declarations

    fn declare_main(&mut self, statements: &[Statement]) {
        for st in statements {
            match st {
                Statement::Dim(d) => {
                    for v in &d.declarators {
                        let ty = self.type_of_decl(&d.type_name, &v.name);
                        let kind = if is_component(&d.type_name) { SymbolKind::Component } else { SymbolKind::Global };
                        let decl = self.name_span(v.span, &v.name).or_else(|| self.name_span(d.span, &v.name));
                        self.declare(0, kind, &v.name, ty, decl, false);
                    }
                }
                Statement::Const(c) => {
                    let ty = c.declared_type.clone().or_else(|| rapidr_ast::suffix_type(&c.name).map(str::to_string));
                    let decl = self.name_span(c.span, &c.name);
                    self.declare(0, SymbolKind::Constant, &c.name, ty, decl, false);
                }
                Statement::Create(c) => self.declare_create(c),
                Statement::Subroutine(s) => {
                    let decl = self.name_span(s.span, &s.name);
                    self.declare(0, SymbolKind::Sub, &s.name, None, decl, false);
                }
                Statement::Function(f) => {
                    let decl = self.name_span(f.span, &f.name);
                    let ty = f.return_type.clone().or_else(|| rapidr_ast::suffix_type(&f.name).map(str::to_string));
                    self.declare(0, SymbolKind::Function, &f.name, ty, decl, false);
                }
                Statement::Declare(d) => {
                    if d.lib.is_some() {
                        let decl = self.name_span(d.span, &d.name);
                        self.declare(0, SymbolKind::External, &d.name, d.return_type.clone(), decl, false);
                    } else {
                        // (a forward declaration: the SUB / FUNCTION itself declares it)
                        let kind = if d.is_function { SymbolKind::Function } else { SymbolKind::Sub };
                        self.declare(0, kind, &d.name, d.return_type.clone(), None, false);
                    }
                }
                Statement::Type(t) => self.declare_type(t),
                Statement::Label(l) => {
                    let decl = self.name_span(l.span, &l.name);
                    self.declare(0, SymbolKind::Label, &l.name, None, decl, false);
                }
                _ => {}
            }
            // (declarations in blocks of the main program are the program's too)
            for body in child_bodies(st) {
                self.declare_main(body);
            }
        }
    }

    fn declare_create(&mut self, c: &rapidr_ast::CreateStatement) {
        let decl = self.name_span(c.span, &c.name);
        self.declare(0, SymbolKind::Component, &c.name, Some(canonical_type(&c.type_name)), decl, false);
        for st in &c.body {
            if let Statement::Create(inner) = st {
                self.declare_create(inner);
            }
        }
    }

    fn declare_type(&mut self, t: &rapidr_ast::TypeStatement) {
        let decl = self.name_span(t.span, &t.name);
        let ty = t.extends.as_deref().map(canonical_type);
        self.declare(0, SymbolKind::Type, &t.name, ty, decl, false);
        // (a TYPE extending another sees its fields: the base's scope is the parent)
        let parent = t.extends.as_deref().and_then(|b| self.types.get(&name_key(b)).copied()).unwrap_or(0);
        let scope = self.model.scopes.len();
        self.model.scopes.push(Scope { kind: ScopeKind::Type(t.name.clone()), parent: Some(parent), span: t.span });
        self.types.insert(name_key(&t.name), scope);
        for f in &t.fields {
            let decl = self.name_span(f.span, &f.name);
            let ty = self.type_of_decl(&f.type_name, &f.name);
            self.declare(scope, SymbolKind::Field, &f.name, ty, decl, false);
        }
        for m in &t.methods {
            match m {
                Statement::Subroutine(s) => {
                    let decl = self.name_span(s.span, &s.name);
                    self.declare(scope, SymbolKind::Sub, &s.name, None, decl, false);
                }
                Statement::Function(f) => {
                    let decl = self.name_span(f.span, &f.name);
                    self.declare(scope, SymbolKind::Function, &f.name, f.return_type.clone(), decl, false);
                }
                _ => {}
            }
        }
    }

    // ------------------------------------------------ walking

    fn walk_body(&mut self, statements: &[Statement], ctx: &Ctx, in_create: bool) {
        for st in statements {
            self.walk_statement(st, ctx, in_create);
        }
    }

    fn walk_statement(&mut self, st: &Statement, ctx: &Ctx, in_create: bool) {
        match st {
            Statement::Assignment(a) => {
                if in_create {
                    // (a property of the component being created)
                    if let Expression::MemberAccess(_) | Expression::Identifier(_) = &a.target {
                    } else {
                        self.expr(&a.target, ctx, Access::Write);
                    }
                } else {
                    self.expr(&a.target, ctx, Access::Write);
                }
                self.expr(&a.value, ctx, Access::Read);
            }
            Statement::Bind(b) => {
                self.expr(&b.target, ctx, Access::Write);
                self.expr(&b.handler, ctx, Access::Read);
            }
            Statement::Call(c) => {
                match &c.callee {
                    Expression::Identifier(id) => self.call_name(id, ctx),
                    other => self.expr(other, ctx, Access::Call),
                }
                for a in &c.args {
                    self.expr(a, ctx, Access::Read);
                }
            }
            Statement::Close(c) => self.expr(&c.file_number, ctx, Access::Read),
            Statement::Const(c) => {
                if !ctx.in_main {
                    let decl = self.name_span(c.span, &c.name);
                    let ty = c.declared_type.clone().or_else(|| rapidr_ast::suffix_type(&c.name).map(str::to_string));
                    self.declare(ctx.scope, SymbolKind::Constant, &c.name, ty, decl, false);
                }
                self.expr(&c.value, ctx, Access::Read);
            }
            Statement::Create(c) => {
                if !ctx.in_main {
                    let decl = self.name_span(c.span, &c.name);
                    self.declare(0, SymbolKind::Component, &c.name, Some(canonical_type(&c.type_name)), decl, false);
                }
                self.walk_body(&c.body, ctx, true);
            }
            Statement::Dim(d) => {
                for v in &d.declarators {
                    if !ctx.in_main {
                        let ty = self.type_of_decl(&d.type_name, &v.name);
                        let decl = self.name_span(v.span, &v.name).or_else(|| self.name_span(d.span, &v.name));
                        let has_local = self.local(ctx.scope, &v.name).is_some();
                        let known_global = self.local(0, &v.name).is_some();
                        if d.is_redim && !has_local && known_global {
                            // (REDIM of the module's array: resizes the global)
                            if let (Some(id), Some(span)) = (self.local(0, &v.name), decl) {
                                self.reference(span, id, Access::Write);
                            }
                        } else {
                            let kind = if d.is_static {
                                SymbolKind::Static
                            } else if is_component(&d.type_name) {
                                SymbolKind::Component
                            } else {
                                SymbolKind::Local
                            };
                            self.declare(ctx.scope, kind, &v.name, ty, decl, false);
                        }
                    }
                    for dim in &v.dimensions {
                        match dim {
                            rapidr_ast::ArrayDimension::Single(e) => self.expr(e, ctx, Access::Read),
                            rapidr_ast::ArrayDimension::Range { start, end } => {
                                self.expr(start, ctx, Access::Read);
                                self.expr(end, ctx, Access::Read);
                            }
                        }
                    }
                }
            }
            Statement::DoLoop(d) => {
                if let Some(c) = &d.condition {
                    self.expr(c, ctx, Access::Read);
                }
                self.walk_body(&d.body, ctx, false);
            }
            Statement::For(f) => {
                let span = self.name_span(f.span, &f.variable);
                let id = match self.resolve(&f.variable, ctx) {
                    Some(id) => Some(id),
                    None if ctx.in_main => Some(self.declare(0, SymbolKind::Global, &f.variable, rapidr_ast::suffix_type(&f.variable).map(str::to_string), None, true)),
                    None => Some(self.declare(ctx.scope, SymbolKind::Local, &f.variable, rapidr_ast::suffix_type(&f.variable).map(str::to_string), span, false)),
                };
                if let (Some(id), Some(span)) = (id, span) {
                    self.reference(span, id, Access::Write);
                }
                self.expr(&f.start, ctx, Access::Read);
                self.expr(&f.end, ctx, Access::Read);
                if let Some(s) = &f.step {
                    self.expr(s, ctx, Access::Read);
                }
                self.walk_body(&f.body, ctx, false);
            }
            Statement::Function(f) => self.walk_routine(f.span, &f.name, &f.params, &f.body, true, None),
            Statement::Subroutine(s) => self.walk_routine(s.span, &s.name, &s.params, &s.body, false, None),
            Statement::Gosub(j) | Statement::Goto(j) => {
                if let Some(span) = self.name_span(j.span, &j.label) {
                    let key = name_key(&j.label);
                    let id = self.names.get(&(ctx.scope, key.clone(), true)).or_else(|| self.names.get(&(0, key, true))).copied();
                    if let Some(id) = id {
                        self.reference(span, id, Access::Read);
                    }
                }
            }
            Statement::Label(l) => {
                if !ctx.in_main {
                    let decl = self.name_span(l.span, &l.name);
                    self.declare(ctx.scope, SymbolKind::Label, &l.name, None, decl, false);
                }
            }
            Statement::If(i) => {
                self.expr(&i.condition, ctx, Access::Read);
                self.walk_body(&i.then_body, ctx, false);
                for b in &i.elseif_branches {
                    self.expr(&b.condition, ctx, Access::Read);
                    self.walk_body(&b.body, ctx, false);
                }
                self.walk_body(&i.else_body, ctx, false);
            }
            Statement::Input(i) => {
                if let Some(p) = &i.prompt {
                    self.expr(p, ctx, Access::Read);
                }
                self.expr(&i.target, ctx, Access::Write);
            }
            Statement::Open(o) => {
                self.expr(&o.filename, ctx, Access::Read);
                self.expr(&o.file_number, ctx, Access::Read);
            }
            Statement::Print(p) => {
                for e in &p.items {
                    self.expr(e, ctx, Access::Read);
                }
            }
            Statement::PrintHash(p) => {
                self.expr(&p.file_number, ctx, Access::Read);
                for e in &p.items {
                    self.expr(e, ctx, Access::Read);
                }
            }
            Statement::WriteHash(p) => {
                self.expr(&p.file_number, ctx, Access::Read);
                for e in &p.items {
                    self.expr(e, ctx, Access::Read);
                }
            }
            Statement::Return(r) => {
                if let Some(v) = &r.value {
                    self.expr(v, ctx, Access::Read);
                }
            }
            Statement::Seek(s) => {
                self.expr(&s.file_number, ctx, Access::Read);
                self.expr(&s.position, ctx, Access::Read);
            }
            Statement::SelectCase(s) => {
                self.expr(&s.expression, ctx, Access::Read);
                for c in &s.cases {
                    for v in &c.values {
                        match v {
                            CaseValue::Value(e) | CaseValue::Is(_, e) => self.expr(e, ctx, Access::Read),
                            CaseValue::Range(a, b) => {
                                self.expr(a, ctx, Access::Read);
                                self.expr(b, ctx, Access::Read);
                            }
                            CaseValue::IsLogic(_, e, rest) => {
                                self.expr(e, ctx, Access::Read);
                                for (_, e) in rest {
                                    self.expr(e, ctx, Access::Read);
                                }
                            }
                        }
                    }
                    self.walk_body(&c.body, ctx, false);
                }
                self.walk_body(&s.case_else, ctx, false);
            }
            Statement::Type(t) => self.walk_type(t),
            Statement::While(w) => {
                self.expr(&w.condition, ctx, Access::Read);
                self.walk_body(&w.body, ctx, false);
            }
            Statement::With(w) => {
                self.expr(&w.object, ctx, Access::Read);
                let mut inner = ctx.clone();
                inner.with.push(self.type_of_expr(&w.object, ctx));
                self.walk_body(&w.body, &inner, false);
            }
            Statement::Declare(_)
            | Statement::Comment(_)
            | Statement::Directive(_)
            | Statement::Exit(_)
            | Statement::Import(_)
            | Statement::Line(_)
            | Statement::RustBlock(_) => {}
        }
    }

    fn walk_type(&mut self, t: &rapidr_ast::TypeStatement) {
        let Some(&scope) = self.types.get(&name_key(&t.name)) else { return };
        for m in &t.methods {
            match m {
                Statement::Subroutine(s) => self.walk_routine(s.span, &s.name, &s.params, &s.body, false, Some((scope, &t.name))),
                Statement::Function(f) => self.walk_routine(f.span, &f.name, &f.params, &f.body, true, Some((scope, &t.name))),
                _ => {}
            }
        }
        let ctx = Ctx { scope, in_main: false, function: None, this_type: Some(t.name.clone()), with: Vec::new() };
        self.walk_body(&t.constructor, &ctx, false);
        for e in &t.events {
            let routine = self.model.scopes.len();
            self.model.scopes.push(Scope { kind: ScopeKind::Routine(e.name.clone()), parent: Some(scope), span: e.span });
            self.declare_params(routine, &e.params);
            let ctx = Ctx { scope: routine, in_main: false, function: None, this_type: Some(t.name.clone()), with: Vec::new() };
            self.walk_body(&e.body, &ctx, false);
        }
    }

    fn declare_params(&mut self, scope: ScopeId, params: &[rapidr_ast::Parameter]) {
        for p in params {
            let decl = self.name_span(p.span, &p.name);
            let ty = self.type_of_decl(&p.type_name, &p.name);
            self.declare(scope, SymbolKind::Param, &p.name, ty, decl, false);
        }
    }

    fn walk_routine(&mut self, span: TextSpan, name: &str, params: &[rapidr_ast::Parameter], body: &[Statement], is_function: bool, owner: Option<(ScopeId, &str)>) {
        let parent = owner.map_or(0, |(s, _)| s);
        let scope = self.model.scopes.len();
        self.model.scopes.push(Scope { kind: ScopeKind::Routine(name.to_string()), parent: Some(parent), span });
        self.declare_params(scope, params);
        let function = if is_function { self.names.get(&(parent, name_key(name), false)).copied() } else { None };
        let ctx = Ctx { scope, in_main: false, function, this_type: owner.map(|(_, t)| t.to_string()), with: Vec::new() };
        self.walk_body(body, &ctx, false);
    }

    /// A name's symbol in a context (no new symbol).
    fn resolve(&self, name: &str, ctx: &Ctx) -> Option<SymbolId> {
        let key = name_key(name);
        let mut at = Some(ctx.scope);
        while let Some(scope) = at {
            if let Some(&id) = self.names.get(&(scope, key.clone(), false)) {
                return Some(id);
            }
            at = self.model.scopes.get(scope).and_then(|s| s.parent);
        }
        None
    }

    fn local(&self, scope: ScopeId, name: &str) -> Option<SymbolId> {
        self.names.get(&(scope, name_key(name), false)).copied()
    }

    fn call_name(&mut self, id: &rapidr_ast::Identifier, ctx: &Ctx) {
        let Some(span) = self.ident_span(id.span, &id.name) else { return };
        if let Some(sym) = self.resolve(&id.name, ctx) {
            self.reference(span, sym, Access::Call);
        }
    }

    /// A bare name in an expression.
    fn name(&mut self, id: &rapidr_ast::Identifier, ctx: &Ctx, access: Access) {
        if id.name.starts_with("__") || id.name == "_with_" {
            return;
        }
        let Some(span) = self.ident_span(id.span, &id.name) else { return };
        let key = name_key(&id.name);
        if key == "result" && self.resolve(&id.name, ctx).is_none() {
            if let Some(f) = ctx.function {
                self.reference(span, f, access);
            }
            return;
        }
        if matches!(key.as_str(), "this" | "super" | "me") {
            return;
        }
        match self.resolve(&id.name, ctx) {
            Some(sym) => self.reference(span, sym, access),
            None => {
                if is_predefined(&id.name) {
                    return;
                }
                // RapidQ's implicit variable: a global made by its first use.
                let ty = rapidr_ast::suffix_type(&id.name).map(str::to_string);
                let sym = self.declare(0, SymbolKind::Global, &id.name, ty, None, true);
                self.reference(span, sym, access);
            }
        }
    }

    fn expr(&mut self, e: &Expression, ctx: &Ctx, access: Access) {
        match e {
            Expression::Identifier(id) => self.name(id, ctx, access),
            Expression::Literal(_) => {}
            Expression::Unary(u) => self.expr(&u.operand, ctx, Access::Read),
            Expression::Binary(b) => {
                self.expr(&b.left, ctx, Access::Read);
                self.expr(&b.right, ctx, Access::Read);
            }
            Expression::ArrayAccess(a) => {
                self.expr(&a.array, ctx, access);
                for i in &a.indices {
                    self.expr(i, ctx, Access::Read);
                }
            }
            Expression::FunctionCall(f) => {
                match &*f.callee {
                    Expression::Identifier(id) => {
                        let resolved = self.resolve(&id.name, ctx);
                        let is_routine = resolved.is_some_and(|s| matches!(self.model.symbols[s].kind, SymbolKind::Sub | SymbolKind::Function | SymbolKind::External));
                        if is_routine {
                            self.call_name(id, ctx);
                        } else if resolved.is_some() || !registry::is_builtin_name(&id.name) {
                            self.name(id, ctx, access);
                        }
                    }
                    other => self.expr(other, ctx, Access::Call),
                }
                for a in &f.args {
                    self.expr(a, ctx, Access::Read);
                }
            }
            Expression::MemberAccess(m) => {
                self.expr(&m.object, ctx, Access::Read);
                self.member(e, &m.object, &m.member, ctx, access);
            }
            Expression::MethodCall(m) => {
                self.expr(&m.object, ctx, Access::Read);
                self.member(e, &m.object, &m.method, ctx, Access::Call);
                for a in &m.args {
                    self.expr(a, ctx, Access::Read);
                }
            }
        }
    }

    /// `Obj.Member` where Obj is a TYPE's instance: the field or method.
    fn member(&mut self, whole: &Expression, object: &Expression, member: &str, ctx: &Ctx, access: Access) {
        let Some(ty) = self.type_of_expr(object, ctx) else { return };
        let Some(sym) = self.type_member(&ty, member) else { return };
        if let Some(span) = self.member_span(expression_span(whole), member) {
            self.reference(span, sym, access);
        }
    }

    /// A field or method of a TYPE, or of the TYPEs it extends.
    fn type_member(&self, ty: &str, member: &str) -> Option<SymbolId> {
        let mut scope = *self.types.get(&name_key(ty))?;
        loop {
            if let Some(&sym) = self.names.get(&(scope, name_key(member), false)) {
                return Some(sym);
            }
            match self.model.scopes[scope].parent {
                Some(p) if p != 0 => scope = p,
                _ => return None,
            }
        }
    }

    /// The declared type of an expression, when the model knows it.
    fn type_of_expr(&self, e: &Expression, ctx: &Ctx) -> Option<String> {
        match e {
            Expression::Identifier(id) => {
                let key = name_key(&id.name);
                if id.name == "_with_" {
                    return ctx.with.last().cloned().flatten();
                }
                if key == "this" {
                    return ctx.this_type.clone();
                }
                self.resolve(&id.name, ctx).and_then(|s| self.model.symbols[s].ty.clone())
            }
            Expression::ArrayAccess(a) => self.type_of_expr(&a.array, ctx),
            Expression::FunctionCall(f) => self.type_of_expr(&f.callee, ctx),
            Expression::MemberAccess(m) => {
                let ty = self.type_of_expr(&m.object, ctx)?;
                let sym = self.type_member(&ty, &m.member)?;
                self.model.symbols[sym].ty.clone()
            }
            _ => None,
        }
    }
}

/// The statement lists inside a block statement of the main program (not a
/// SUB's or a TYPE's: those have scopes of their own).
fn child_bodies(st: &Statement) -> Vec<&[Statement]> {
    match st {
        Statement::If(i) => {
            let mut v: Vec<&[Statement]> = vec![&i.then_body, &i.else_body];
            v.extend(i.elseif_branches.iter().map(|b| b.body.as_slice()));
            v
        }
        Statement::For(f) => vec![&f.body],
        Statement::While(w) => vec![&w.body],
        Statement::DoLoop(d) => vec![&d.body],
        Statement::With(w) => vec![&w.body],
        Statement::SelectCase(s) => {
            let mut v: Vec<&[Statement]> = s.cases.iter().map(|c| c.body.as_slice()).collect();
            v.push(&s.case_else);
            v
        }
        _ => Vec::new(),
    }
}

/// Names a program uses without declaring that aren't variables: builtins,
/// constants, `True` / `False`.
fn is_predefined(name: &str) -> bool {
    let key = name_key(name);
    matches!(key.as_str(), "true" | "false" | "nothing" | "null")
        || registry::is_builtin_name(name)
        || registry::constant(name).is_some()
        || registry::component(name).is_some()
}

/// RapidR's name of a component type (`QFORM` → `RFORM`); other types upper case.
pub fn canonical_type(type_name: &str) -> String {
    let c = rapidr_ast::canonical_type_name(type_name);
    if registry::component(&c).is_some() {
        c.to_ascii_uppercase()
    } else {
        c
    }
}

fn is_component(type_name: &str) -> bool {
    !type_name.is_empty() && registry::component(type_name).is_some()
}

pub fn expression_span(e: &Expression) -> TextSpan {
    rapidr_parser::expression_span(e)
}

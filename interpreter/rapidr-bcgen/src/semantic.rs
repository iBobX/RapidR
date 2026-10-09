//! The program's names as the compiler resolves them: a public semantic
//! model (docs/ide-plan.md, I0 "semantic model export"; I3 builds
//! IntelliSense on it).
//!
//! [`analyze`] compiles the program with the bytecode compiler and records,
//! as it goes, every declaration it makes (globals, locals, parameters,
//! STATICs, constants, components, SUBs and FUNCTIONs, labels) and how it
//! reads each name (a local slot, a global, a component, a FUNCTION called
//! without parentheses, a builtin …). The rules that decide are the
//! functions below ([`resolve_name`], [`dim_target`], [`for_target`],
//! [`store_target`]): the compiler itself calls them, so the compiler and
//! the IDE can't disagree. TYPEs, their fields, and names the compiler
//! handles through other paths (an object's name in `Obj.Member`, a
//! routine's name in a call) are added from the program as written, looked
//! up in the same tables.
//!
//! Spans count bytes of the text the program was parsed from (the
//! preprocessed text: map them to files with
//! `rapidr_preprocessor::OriginMap`, e.g. `rapidr_parser::ToolsParse::locate`).
//! Names the compiler generates (its `__` temporaries, TYPE methods'
//! internal names) are left out: every span in the model holds the name it
//! stands for — except `RESULT` inside a FUNCTION, a reference to the
//! FUNCTION (its result), which renaming tools must leave alone.
//!
//! Component types are the parser's: RapidR's name for a RapidQ component
//! (`QFORM` → `RFORM`), one model for both names (docs/q-and-r-components.md).

use std::collections::HashMap;

use rapidr_ast::{Expression, Program, Statement};
use rapidr_diagnostics::TextSpan;

// ---------------------------------------------------------------- rules

/// What a bare name in an expression is, by the compiler's precedence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NameUse {
    /// A component (CREATE / DIM … AS a component), unless a local hides it.
    Component,
    /// A local slot of the routine (parameter, DIM in a SUB, FOR variable,
    /// a FUNCTION's result).
    Local,
    /// A builtin written without parentheses (`TIMER`).
    BareBuiltin,
    /// `True` / `False` without RAPIDQ.INC's constants.
    BoolConstant,
    /// One of RapidR's own constants (`akLeft` …).
    RapidrConstant,
    /// A FUNCTION named without parentheses: called.
    FunctionCall,
    /// A global variable (or a STATIC of the routine), declared or implicit.
    Global,
}

/// What the compiler knows about a name where it's read.
#[derive(Debug, Clone, Copy, Default)]
pub struct NameFacts {
    pub is_component: bool,
    pub is_local: bool,
    pub is_known_global: bool,
    pub is_bare_builtin: bool,
    pub is_bool_name: bool,
    pub is_rapidr_constant: bool,
    pub is_function: bool,
}

/// The compiler's rule for a bare name in an expression: a component, then
/// a local, then (unless the program has a global of that name) a bare
/// builtin, `True`/`False`, a RapidR constant, a FUNCTION; else a global.
pub fn resolve_name(f: &NameFacts) -> NameUse {
    if f.is_component {
        NameUse::Component
    } else if f.is_local {
        NameUse::Local
    } else if !f.is_known_global && f.is_bare_builtin {
        NameUse::BareBuiltin
    } else if !f.is_known_global && f.is_bool_name {
        NameUse::BoolConstant
    } else if !f.is_known_global && f.is_rapidr_constant {
        NameUse::RapidrConstant
    } else if !f.is_known_global && f.is_function {
        NameUse::FunctionCall
    } else {
        NameUse::Global
    }
}

/// Where a `DIM` / `REDIM` puts a variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DimTarget {
    /// A module-level variable (any DIM in the main program).
    Global,
    /// A local of the SUB / FUNCTION.
    Local,
    /// `REDIM` in a SUB of an array the module declares (and the SUB
    /// doesn't): resizes the global (QBasic / RapidQ).
    ResizeGlobal,
}

/// The compiler's rule for a DIM / REDIM declarator.
pub fn dim_target(in_main: bool, is_redim: bool, has_local: bool, is_known_global: bool) -> DimTarget {
    if in_main {
        DimTarget::Global
    } else if is_redim && !has_local && is_known_global {
        DimTarget::ResizeGlobal
    } else {
        DimTarget::Local
    }
}

/// The compiler's rule for a FOR variable: global in the main program
/// (unless a local of that name exists), a local in a SUB / FUNCTION.
pub fn for_target(in_main: bool, has_local: bool) -> NameUse {
    if in_main && !has_local {
        NameUse::Global
    } else {
        NameUse::Local
    }
}

/// The compiler's rule for assigning a bare name: its local slot if the
/// routine has one, else the global (an assignment in the main program
/// declares it).
pub fn store_target(has_local: bool) -> NameUse {
    if has_local {
        NameUse::Local
    } else {
        NameUse::Global
    }
}

// ---------------------------------------------------------------- model

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
    /// The declared type (`INTEGER`, `QFORM`, a TYPE's name …), or the one
    /// its suffix gives; `None`: a variant.
    pub ty: Option<String>,
    /// The name in its declaration; `None` when it's only used (an
    /// implicit global) or declared in generated code.
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
        // (name_key's comparison without a String per symbol)
        let key = rapidr_ast::strip_type_suffix(name);
        let find = |scope: ScopeId| self.symbols.iter().position(|s| s.scope == scope && s.kind != SymbolKind::Label && rapidr_ast::strip_type_suffix(&s.name).eq_ignore_ascii_case(key));
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
        // (an outer scope's symbol shows when `lookup` from `scope` finds
        // it: no scope nearer has its name, and it's its scope's first of
        // that name — one pass, the names taken so far in a set)
        let mut out: Vec<SymbolId> = (0..self.symbols.len()).filter(|&i| self.symbols[i].scope == scope).collect();
        let mut taken: std::collections::HashSet<String> = out.iter().filter(|&&i| self.symbols[i].kind != SymbolKind::Label).map(|&i| name_key(&self.symbols[i].name)).collect();
        let mut at = self.scopes.get(scope).and_then(|s| s.parent);
        while let Some(outer) = at {
            let mut here = std::collections::HashSet::new();
            for (i, sym) in self.symbols.iter().enumerate() {
                if sym.scope != outer || sym.kind == SymbolKind::Label {
                    continue;
                }
                let key = name_key(&sym.name);
                if !taken.contains(&key) && here.insert(key) {
                    out.push(i);
                }
            }
            taken.extend(here);
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
/// parsed from (its spans count its bytes): with it, every span is checked
/// to hold its name and narrowed to the name in declarations.
pub fn analyze(program: &Program, source: Option<&str>) -> SemanticModel {
    let recorder = crate::record_program(program, source);
    build(program, source, recorder)
}

// ---------------------------------------------------------------- recording

/// What the compiler decided, as it compiled.
#[derive(Debug, Default)]
pub(crate) struct Recorder {
    pub events: Vec<Event>,
}

#[derive(Debug, Clone)]
pub(crate) enum Event {
    Declare { routine: Option<String>, kind: SymbolKind, name: String, ty: Option<String>, span: TextSpan },
    Use { routine: Option<String>, name: String, span: TextSpan, target: NameUse, access: Access },
    /// The compiler starts a routine's body (its key, the statement's span:
    /// how a TYPE's method, renamed by the compiler, is found).
    Routine { routine: String, span: TextSpan, is_function: bool },
    Label { routine: Option<String>, name: String, span: TextSpan, access: Access },
}

impl Recorder {
    pub fn declare(&mut self, routine: &Option<String>, kind: SymbolKind, name: &str, ty: Option<&str>, span: TextSpan) {
        self.events.push(Event::Declare {
            routine: routine.clone(),
            kind,
            name: name.to_string(),
            ty: ty.filter(|t| !t.is_empty()).map(str::to_string),
            span,
        });
    }

    pub fn use_name(&mut self, routine: &Option<String>, name: &str, span: TextSpan, target: NameUse, access: Access) {
        self.events.push(Event::Use { routine: routine.clone(), name: name.to_string(), span, target, access });
    }
}

// ---------------------------------------------------------------- building

struct Builder<'a> {
    source: Option<&'a str>,
    model: SemanticModel,
    /// (scope, name key, is a label) → symbol
    names: HashMap<(ScopeId, String, bool), SymbolId>,
    routines: HashMap<String, ScopeId>,
    functions: HashMap<String, SymbolId>,
    /// Each span with a reference: its index in the model's references.
    used: HashMap<(usize, usize), usize>,
    /// Routine scopes and FUNCTION symbols by the routine's span (the
    /// compiler renames a TYPE's methods; their spans stay).
    span_scopes: HashMap<(usize, usize), ScopeId>,
    span_functions: HashMap<(usize, usize), SymbolId>,
}

impl<'a> Builder<'a> {
    /// The name inside `span` (the span itself if there's no source); `None`
    /// when the source there doesn't hold the name (generated code).
    fn name_span(&self, span: TextSpan, name: &str) -> Option<TextSpan> {
        let Some(source) = self.source else { return Some(span) };
        let text = source.get(span.start..span.end.min(source.len()))?;
        let key = name_key(name);
        if key.is_empty() || name.starts_with("__") {
            return None;
        }
        if name_key(text) == key && is_name(text) {
            return Some(span);
        }
        // The first whole word in the span that is the name.
        let bytes = text.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if is_name_byte(bytes[i]) && (i == 0 || !is_name_byte(bytes[i - 1])) {
                let mut j = i;
                while j < bytes.len() && (is_name_byte(bytes[j]) || (j > i && matches!(bytes[j], b'$' | b'%' | b'&' | b'!' | b'#' | b'?'))) {
                    j += 1;
                }
                if text.is_char_boundary(i) && text.is_char_boundary(j) && name_key(&text[i..j]) == key {
                    return Some(TextSpan::new(span.start + i, span.start + j));
                }
                i = j.max(i + 1);
            } else {
                i += 1;
            }
        }
        None
    }

    fn scope_of(&mut self, routine: &Option<String>) -> ScopeId {
        match routine {
            None => 0,
            Some(r) => match self.routines.get(r) {
                Some(&s) => s,
                None => {
                    // (a routine the compiler made: TYPE methods, `type:` code)
                    self.model.scopes.push(Scope { kind: ScopeKind::Routine(r.clone()), parent: Some(0), span: TextSpan::default() });
                    let id = self.model.scopes.len() - 1;
                    self.routines.insert(r.clone(), id);
                    id
                }
            },
        }
    }

    fn symbol(&mut self, scope: ScopeId, name: &str, kind: SymbolKind, ty: Option<String>, decl: Option<TextSpan>, implicit: bool) -> SymbolId {
        let label = kind == SymbolKind::Label;
        let key = (scope, name_key(name), label);
        if let Some(&id) = self.names.get(&key) {
            let s = &mut self.model.symbols[id];
            if s.decl.is_none() && decl.is_some() && !implicit {
                s.decl = decl;
                s.implicit = false;
                s.kind = kind;
                if ty.is_some() {
                    s.ty = ty;
                }
            }
            return id;
        }
        let ty = ty.filter(|t| !t.is_empty()).or_else(|| rapidr_ast::suffix_type(name).map(str::to_string));
        let display = decl.and_then(|d| self.source.and_then(|s| s.get(d.start..d.end))).filter(|t| is_name(t)).unwrap_or(name).to_string();
        self.model.symbols.push(Symbol { name: display, kind, ty, decl, scope, implicit });
        let id = self.model.symbols.len() - 1;
        self.names.insert(key, id);
        id
    }

    fn reference(&mut self, span: TextSpan, symbol: SymbolId, access: Access) {
        match self.used.get(&(span.start, span.end)) {
            // (the same name read, then stored, as INPUT x and SWAP a, b do:
            // it is written)
            Some(&i) => {
                if access == Access::Write && self.model.references[i].access == Access::Read && self.model.references[i].symbol == symbol {
                    self.model.references[i].access = Access::Write;
                }
            }
            None => {
                self.used.insert((span.start, span.end), self.model.references.len());
                self.model.references.push(Reference { span, symbol, access });
            }
        }
    }

    fn declare(&mut self, scope: ScopeId, kind: SymbolKind, name: &str, ty: Option<String>, span: TextSpan) {
        let decl = self.name_span(span, name);
        let id = self.symbol(scope, name, kind, ty, decl, false);
        if let Some(d) = decl {
            self.reference(d, id, Access::Declare);
        }
    }

    /// A routine's own undeclared variable, as the implicit-scope pass
    /// renamed it (`S5__q`: RapidQ keeps it between calls, a STATIC —
    /// rapidr_ast::implicit_scope): its routine's scope and its name.
    fn owned(&self, name: &str) -> Option<(ScopeId, String)> {
        let key = name_key(name);
        self.routines
            .iter()
            .filter(|(r, _)| key.len() > r.len() + 2 && key.starts_with(r.as_str()) && key[r.len()..].starts_with("__"))
            .max_by_key(|(r, _)| r.len())
            .map(|(r, &scope)| (scope, name[r.len() + 2..].to_string()))
    }

    fn use_name(&mut self, routine: &Option<String>, name: &str, span: TextSpan, target: NameUse, access: Access) {
        if let (NameUse::Global, Some((scope, var))) = (target, self.owned(name)) {
            let Some(at) = self.name_span(span, &var) else { return };
            let id = self.symbol(scope, &var, SymbolKind::Static, None, Some(at), true);
            self.reference(at, id, access);
            return;
        }
        let Some(at) = self.name_span(span, name) else { return };
        let scope = self.scope_of(routine);
        let symbol = match target {
            NameUse::Local => {
                // A FUNCTION's own name (or RESULT) inside it: its result.
                let key = name_key(name);
                match routine.as_ref().and_then(|r| self.functions.get(r).copied().filter(|_| &key == r || key == "result")) {
                    Some(f) => f,
                    None => self.symbol(scope, name, SymbolKind::Local, None, Some(at), true),
                }
            }
            NameUse::Global => {
                if let Some(&id) = self.names.get(&(scope, name_key(name), false)).filter(|_| scope != 0) {
                    // (a STATIC of the routine)
                    id
                } else {
                    self.symbol(0, name, SymbolKind::Global, None, Some(at), true)
                }
            }
            // (a component the routine DIMs itself: the compiler reaches it
            // by name, the IDE by its declaration)
            NameUse::Component => match self.names.get(&(scope, name_key(name), false)).filter(|_| scope != 0) {
                Some(&id) => id,
                None => self.symbol(0, name, SymbolKind::Component, None, Some(at), true),
            },
            NameUse::FunctionCall => match self.names.get(&(0, name_key(name), false)) {
                Some(&id) => id,
                None => return,
            },
            NameUse::BareBuiltin | NameUse::BoolConstant | NameUse::RapidrConstant => return,
        };
        let access = if target == NameUse::FunctionCall { Access::Call } else { access };
        self.reference(at, symbol, access);
    }
}

fn is_name_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b >= 0x80
}

fn is_name(text: &str) -> bool {
    !text.is_empty() && text.bytes().enumerate().all(|(i, b)| is_name_byte(b) || (i > 0 && matches!(b, b'$' | b'%' | b'&' | b'!' | b'#' | b'?')))
}

fn build(program: &Program, source: Option<&str>, recorder: Recorder) -> SemanticModel {
    let mut b = Builder {
        source,
        model: SemanticModel::default(),
        names: HashMap::new(),
        routines: HashMap::new(),
        functions: HashMap::new(),
        used: Default::default(),
        span_scopes: HashMap::new(),
        span_functions: HashMap::new(),
    };
    b.model.scopes.push(Scope { kind: ScopeKind::Program, parent: None, span: program.span });

    // Routines first (calls may come before them), from the program as
    // written, then the TYPEs.
    for stmt in &program.statements {
        match stmt {
            Statement::Subroutine(s) => declare_routine(&mut b, 0, &s.name, false, None, s.span, &s.params),
            Statement::Function(f) => declare_routine(&mut b, 0, &f.name, true, f.return_type.clone(), f.span, &f.params),
            Statement::Declare(d) if d.lib.is_some() => b.declare(0, SymbolKind::External, &d.name, d.return_type.clone(), d.span),
            Statement::Type(t) => {
                b.declare(0, SymbolKind::Type, &t.name, t.extends.clone(), t.span);
                b.model.scopes.push(Scope { kind: ScopeKind::Type(t.name.clone()), parent: Some(0), span: t.span });
                let scope = b.model.scopes.len() - 1;
                for f in &t.fields {
                    b.declare(scope, SymbolKind::Field, &f.name, Some(f.type_name.clone()), f.span);
                }
                // Its methods: in the TYPE's scope.
                for m in &t.methods {
                    match m {
                        Statement::Subroutine(s) => declare_routine(&mut b, scope, &s.name, false, None, s.span, &s.params),
                        Statement::Function(f) => declare_routine(&mut b, scope, &f.name, true, f.return_type.clone(), f.span, &f.params),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    // `DECLARE SUB Greet` before its SUB: a declaration of the same routine
    // (find references lists it, a rename changes it)
    for stmt in &program.statements {
        if let Statement::Declare(d) = stmt {
            if d.lib.is_none() {
                if let (Some(&id), Some(span)) = (b.names.get(&(0, name_key(&d.name), false)), b.name_span(d.span, &d.name)) {
                    b.reference(span, id, Access::Declare);
                }
            }
        }
    }

    // DIM types as written (the compiler's passes turn a TYPE's instances
    // into variants), by where they're declared.
    let mut written: HashMap<usize, String> = HashMap::new();
    rapidr_ast::walk(
        &program.statements,
        &mut |s| {
            if let Statement::Dim(d) = s {
                for v in &d.declarators {
                    written.insert(v.span.start, d.type_name.clone());
                }
            }
        },
        &mut |_| {},
    );

    // The compiler's own decisions.
    for event in recorder.events {
        match event {
            Event::Declare { routine, kind, name, ty, span } => {
                // (RapidQ's implicit variables: the compiler's own `DIM name
                // AS DOUBLE` for a name never declared, spanning the whole
                // program — rapidr_ast's default-type pass. Not a
                // declaration the program wrote: the symbol is implicit,
                // its uses recorded as they are, Read or Write)
                if kind == SymbolKind::Global && span == program.span && program.span.len() > 0 {
                    match b.owned(&name) {
                        Some((scope, var)) => b.symbol(scope, &var, SymbolKind::Static, ty, None, true),
                        None => b.symbol(0, &name, SymbolKind::Global, ty, None, true),
                    };
                    continue;
                }
                if let (SymbolKind::Global, Some((scope, var))) = (kind, b.owned(&name)) {
                    b.declare(scope, SymbolKind::Static, &var, ty, span);
                    continue;
                }
                let scope = if matches!(kind, SymbolKind::Global | SymbolKind::Constant | SymbolKind::Component) { 0 } else { b.scope_of(&routine) };
                let ty = written.get(&span.start).cloned().or(ty);
                b.declare(scope, kind, &name, ty, span);
            }
            Event::Use { routine, name, span, target, access } => b.use_name(&routine, &name, span, target, access),
            Event::Routine { routine, span, is_function } => {
                if !b.routines.contains_key(&routine) {
                    let key = (span.start, span.end);
                    let scope = match b.span_scopes.get(&key) {
                        Some(&scope) => scope,
                        None => {
                            // (a routine the compiler made — a TYPE's
                            // constructor or event code: no place of its own)
                            b.model.scopes.push(Scope { kind: ScopeKind::Routine(routine.clone()), parent: Some(0), span: TextSpan::default() });
                            b.model.scopes.len() - 1
                        }
                    };
                    b.routines.insert(routine.clone(), scope);
                    if let (true, Some(&f)) = (is_function, b.span_functions.get(&key)) {
                        b.functions.insert(routine, f);
                    }
                }
            }
            Event::Label { routine, name, span, access } => {
                let scope = b.scope_of(&routine);
                let Some(at) = b.name_span(span, &name) else { continue };
                let id = b.symbol(scope, &name, SymbolKind::Label, None, (access == Access::Declare).then_some(at), access != Access::Declare);
                b.reference(at, id, access);
            }
        }
    }

    // Names the compiler reads through other paths (`Obj.Member`'s object,
    // a routine called by name), from the program as written.
    // Each name is looked up in the innermost scope holding it, then outwards.
    let mut idents: Vec<(TextSpan, String)> = Vec::new();
    rapidr_ast::walk(
        &program.statements,
        &mut |_| {},
        &mut |e| {
            if let Expression::Identifier(id) = e {
                idents.push((id.span, id.name.clone()));
            }
        },
    );
    for (span, name) in idents {
        if b.used.contains_key(&(span.start, span.end)) {
            continue;
        }
        let Some(at) = b.name_span(span, &name) else { continue };
        let key = name_key(&name);
        let mut scope = Some(b.model.scope_at(at.start));
        let mut found = None;
        while let (Some(s), None) = (scope, found) {
            found = b.names.get(&(s, key.clone(), false)).copied();
            scope = b.model.scopes[s].parent;
        }
        if let Some(id) = found {
            let access = if matches!(b.model.symbols[id].kind, SymbolKind::Sub | SymbolKind::Function | SymbolKind::External) { Access::Call } else { Access::Read };
            b.reference(at, id, access);
        }
    }

    // A TYPE that extends another of the program's sees its fields and
    // methods: the base's scope is its parent.
    let type_scopes: HashMap<String, ScopeId> = b
        .model
        .scopes
        .iter()
        .enumerate()
        .filter_map(|(i, s)| match &s.kind {
            ScopeKind::Type(t) => Some((name_key(t), i)),
            _ => None,
        })
        .collect();
    for sym in b.model.symbols.clone() {
        if sym.kind != SymbolKind::Type {
            continue;
        }
        let (Some(&scope), Some(&base)) = (type_scopes.get(&name_key(&sym.name)), sym.ty.as_deref().and_then(|t| type_scopes.get(&name_key(t)))) else { continue };
        if base != scope && !extends(&b.model, base, scope) {
            b.model.scopes[scope].parent = Some(base);
        }
    }

    members(&mut b, program, &type_scopes);

    let mut model = b.model;
    model.references.sort_by_key(|r| (r.span.start, r.span.end));
    model
}

/// Whether `scope`'s parent chain reaches `ancestor` (no cycles).
fn extends(model: &SemanticModel, scope: ScopeId, ancestor: ScopeId) -> bool {
    let mut at = model.scopes[scope].parent;
    while let Some(s) = at {
        if s == ancestor {
            return true;
        }
        at = model.scopes[s].parent;
    }
    false
}

/// `Obj.Member` where Obj is an instance of one of the program's TYPEs
/// (a variable, a parameter, a field, `This`, a WITH's `.`): a reference
/// to the field or method (the compiler lowers these to its own calls, so
/// they come from the program as written).
fn members(b: &mut Builder, program: &Program, type_scopes: &HashMap<String, ScopeId>) {
    if type_scopes.is_empty() {
        return;
    }
    // (what a name's span refers to so far, and the WITH blocks)
    let at_span: HashMap<usize, SymbolId> = b.model.references.iter().map(|r| (r.span.start, r.symbol)).collect();
    let mut withs: Vec<(TextSpan, Expression)> = Vec::new();
    let mut writes: std::collections::HashSet<usize> = Default::default();
    let mut found: Vec<(Expression, Access)> = Vec::new();
    rapidr_ast::walk(
        &program.statements,
        &mut |s| match s {
            Statement::With(w) => withs.push((w.span, w.object.clone())),
            Statement::Assignment(a) => {
                writes.insert(expr_span(&a.target).start);
            }
            _ => {}
        },
        &mut |e| match e {
            Expression::MemberAccess(_) => found.push((e.clone(), Access::Read)),
            Expression::MethodCall(_) => found.push((e.clone(), Access::Call)),
            _ => {}
        },
    );
    let ctx = MemberCtx { at_span: &at_span, withs: &withs, type_scopes };
    for (e, access) in found {
        let (object, member) = match &e {
            Expression::MemberAccess(m) => (&*m.object, m.member.as_str()),
            Expression::MethodCall(m) => (&*m.object, m.method.as_str()),
            _ => continue,
        };
        let Some(ty) = ctx.type_of(b, object, 0) else { continue };
        let Some(symbol) = ctx.member(b, &ty, member) else { continue };
        // (the member's name: right after the object and its dot)
        let Some(source) = b.source else { continue };
        let after = expr_span(object).end;
        let rest = source.get(after..).unwrap_or("");
        let lead = rest.len() - rest.trim_start_matches(['.', ' ', '\t']).len();
        let start = after + lead;
        let Some(at) = b.name_span(TextSpan::new(start, (start + member.len() + 3).min(source.len())), member) else { continue };
        if at.start != start {
            continue;
        }
        let access = if access == Access::Read && writes.contains(&expr_span(&e).start) { Access::Write } else { access };
        b.reference(at, symbol, access);
    }
}

struct MemberCtx<'c> {
    at_span: &'c HashMap<usize, SymbolId>,
    withs: &'c [(TextSpan, Expression)],
    type_scopes: &'c HashMap<String, ScopeId>,
}

impl MemberCtx<'_> {
    /// The TYPE an expression's value is an instance of.
    fn type_of(&self, b: &Builder, e: &Expression, depth: usize) -> Option<String> {
        if depth > 16 {
            return None;
        }
        match e {
            Expression::Identifier(id) => {
                let key = name_key(&id.name);
                if id.name == "_with_" {
                    let w = self.withs.iter().filter(|(span, _)| span.start <= id.span.start && id.span.start < span.end).min_by_key(|(span, _)| span.len())?;
                    return self.type_of(b, &w.1, depth + 1);
                }
                if key == "this" || key == "me" {
                    let mut at = Some(b.model.scope_at(id.span.start));
                    while let Some(s) = at {
                        if let ScopeKind::Type(t) = &b.model.scopes[s].kind {
                            return Some(t.clone());
                        }
                        at = b.model.scopes[s].parent;
                    }
                    return None;
                }
                let at = b.name_span(id.span, &id.name)?;
                let sym = *self.at_span.get(&at.start)?;
                let ty = b.model.symbols[sym].ty.clone()?;
                self.type_scopes.contains_key(&name_key(&ty)).then_some(ty)
            }
            Expression::MemberAccess(m) => {
                let ty = self.type_of(b, &m.object, depth + 1)?;
                let sym = self.member(b, &ty, &m.member)?;
                let ty = b.model.symbols[sym].ty.clone()?;
                self.type_scopes.contains_key(&name_key(&ty)).then_some(ty)
            }
            Expression::ArrayAccess(a) => self.type_of(b, &a.array, depth + 1),
            Expression::FunctionCall(f) => self.type_of(b, &f.callee, depth + 1),
            _ => None,
        }
    }

    /// A field or method of a TYPE, or of the TYPEs it extends.
    fn member(&self, b: &Builder, ty: &str, member: &str) -> Option<SymbolId> {
        let mut scope = Some(*self.type_scopes.get(&name_key(ty))?);
        let key = name_key(member);
        while let Some(s) = scope.filter(|&s| s != 0) {
            if let Some(&id) = b.names.get(&(s, key.clone(), false)) {
                return Some(id);
            }
            scope = b.model.scopes[s].parent;
        }
        None
    }
}

/// The span of an expression.
fn expr_span(e: &Expression) -> TextSpan {
    match e {
        Expression::ArrayAccess(x) => x.span,
        Expression::Binary(x) => x.span,
        Expression::FunctionCall(x) => x.span,
        Expression::Identifier(x) => x.span,
        Expression::Literal(x) => x.span,
        Expression::MemberAccess(x) => x.span,
        Expression::MethodCall(x) => x.span,
        Expression::Unary(x) => x.span,
    }
}

/// A SUB / FUNCTION (in the program, or a TYPE's method in its scope), its
/// scope and parameters.
fn declare_routine(b: &mut Builder, parent: ScopeId, name: &str, is_function: bool, ty: Option<String>, span: TextSpan, params: &[rapidr_ast::Parameter]) {
    let kind = if is_function { SymbolKind::Function } else { SymbolKind::Sub };
    b.declare(parent, kind, name, ty, span);
    let key = name_key(name);
    let symbol = b.names.get(&(parent, key.clone(), false)).copied();
    b.model.scopes.push(Scope { kind: ScopeKind::Routine(name.to_string()), parent: Some(parent), span });
    let scope = b.model.scopes.len() - 1;
    b.span_scopes.insert((span.start, span.end), scope);
    if let (true, Some(id)) = (is_function, symbol) {
        b.span_functions.insert((span.start, span.end), id);
        if parent == 0 {
            b.functions.insert(key.clone(), id);
        }
    }
    if parent == 0 {
        b.routines.insert(key, scope);
    }
    for p in params {
        b.declare(scope, SymbolKind::Param, &p.name, Some(p.type_name.clone()), p.span);
    }
}

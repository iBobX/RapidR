//! Typed arrays in native builds (the speed release): an array declared
//! with a numeric type — `DIM VPhase(31) AS DOUBLE` in the main program, a
//! SUB's own `DIM buf(255) AS INTEGER`, a parameter `voices() AS DOUBLE` —
//! is a `TypedArray<f64>` (`rapidr_value::typed_array`: a plain vector of
//! numbers) instead of a `Value` holding a `BasicArray`, and its elements
//! are read and written as Rust numbers, so `ph = VPhase(v) + VInc(v)` is
//! native arithmetic, as `typed` makes it for scalars.
//!
//! An array is typed only where nothing could see it another way: every
//! DIM / REDIM of it declares the same numeric type, and it is only used by
//! element (`a(i)`, `a(i, j) = x`), as LBOUND / UBOUND's array, or passed
//! whole to a parameter that is itself a typed array of the same type (each
//! call of that routine passing one). Anything else — the array as a value,
//! passed to a builtin, to a DLL, to a parameter of another type, its
//! address, SWAP of whole arrays, inline Rust anywhere — leaves it a `Value`
//! array, as before, so no builtin, interpreter or debugger ever meets a
//! second copy. A routine with GOTO / GOSUB / labels keeps its own arrays
//! and parameters as `Value`s (it still uses typed global arrays); so does
//! one whose name is used as a pointer (CODEPTR, an event, BIND): those are
//! called with `Value` arguments.
//!
//! Main-program arrays live in thread-local statics (`TA_<NAME>`, one
//! `TypedArray` each, made once); each routine using one takes it at its
//! start (`__ta_<name>`), so an element access is a vector index. A DIM or
//! REDIM changes the array in place: whoever holds it sees the new one, as
//! with a `Value` array (a main-program DIM runs with no routine active, so
//! nothing else holds it then).

use std::collections::{HashMap, HashSet};

use rapidr_ast::{ArrayDimension, Expression, Parameter, Statement};

use crate::typed::{Kind, Ty};
use crate::{strip_type_suffix, to_snake, RustCodegen};

/// A typed array's declared type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ArrKind {
    Byte,
    Word,
    Short,
    Long,
    Dword,
    Single,
    Double,
    Int64,
    UInt64,
}

impl ArrKind {
    pub(crate) fn of_type(type_name: &str) -> Option<ArrKind> {
        Some(match type_name.trim().to_ascii_uppercase().as_str() {
            "BYTE" => ArrKind::Byte,
            "WORD" => ArrKind::Word,
            "SHORT" => ArrKind::Short,
            "INTEGER" | "LONG" => ArrKind::Long,
            "DWORD" => ArrKind::Dword,
            "SINGLE" => ArrKind::Single,
            "DOUBLE" => ArrKind::Double,
            "INT64" => ArrKind::Int64,
            "UINT64" => ArrKind::UInt64,
            _ => return None,
        })
    }

    /// The Rust type of an element.
    pub(crate) fn elem(self) -> &'static str {
        match self {
            ArrKind::Byte => "u8",
            ArrKind::Word => "u16",
            ArrKind::Short => "i16",
            ArrKind::Long | ArrKind::Dword => "i32",
            ArrKind::Single => "f32",
            ArrKind::Double => "f64",
            ArrKind::Int64 => "i64",
            ArrKind::UInt64 => "u64",
        }
    }

    /// The typed scalar kind a store converts with (`typed::typed_store`).
    fn scalar(self) -> Option<Kind> {
        Some(match self {
            ArrKind::Byte => Kind::Byte,
            ArrKind::Word => Kind::Word,
            ArrKind::Short => Kind::Short,
            ArrKind::Long => Kind::Long,
            ArrKind::Dword => Kind::Dword,
            ArrKind::Double => Kind::Double,
            ArrKind::Single | ArrKind::Int64 | ArrKind::UInt64 => return None,
        })
    }

    /// The typed expression type an element reads as: none for INT64 /
    /// UINT64, whose arithmetic is `rapidr_value::wide`'s (on `Value`s).
    fn read_ty(self) -> Option<Ty> {
        match self {
            ArrKind::Single | ArrKind::Double => Some(Ty::Float),
            ArrKind::Int64 | ArrKind::UInt64 => None,
            _ => Some(Ty::Int),
        }
    }

    /// The conversion builtin `rapidr_ast::numeric` wraps a store in.
    fn conversion(self) -> &'static str {
        match self {
            ArrKind::Byte => "__to_byte",
            ArrKind::Word => "__to_word",
            ArrKind::Short => "__to_short",
            ArrKind::Long => "__to_long",
            ArrKind::Dword => "__to_dword",
            ArrKind::Single => "__to_single",
            ArrKind::Double => "__to_double",
            ArrKind::Int64 => "__to_int64",
            ArrKind::UInt64 => "__to_uint64",
        }
    }
}

fn key(name: &str) -> String {
    strip_type_suffix(name).to_lowercase()
}

/// The static holding main-program typed array `name`.
pub(crate) fn global_static(name: &str) -> String {
    format!("TA_{}", to_snake(&key(name)).trim_end_matches('_').to_uppercase())
}

/// The local a routine takes main-program typed array `name` into.
fn global_local(name: &str) -> String {
    format!("__ta_{}", to_snake(&key(name)))
}

/// Which arrays are typed (see the module docs).
#[derive(Default, Debug)]
pub(crate) struct TypedArrays {
    /// The main program's (lowercase name → type).
    pub(crate) globals: HashMap<String, ArrKind>,
    /// Each routine's own (lowercase routine → name → type), parameters
    /// included.
    pub(crate) locals: HashMap<String, HashMap<String, ArrKind>>,
    /// Each routine's typed array parameters, by position.
    pub(crate) params: HashMap<String, Vec<Option<ArrKind>>>,
    /// The main-program typed arrays each routine (and the main program,
    /// under "") names.
    pub(crate) uses_globals: HashMap<String, Vec<String>>,
}

impl TypedArrays {
    pub(crate) fn is_empty(&self) -> bool {
        self.globals.is_empty() && self.locals.values().all(|l| l.is_empty())
    }
}

/// A user routine's call: the callee and, per argument, the name passed
/// when it is a bare variable.
struct UserCall {
    callee: String,
    args: Vec<Option<String>>,
}

/// What one scope (the main program, or a routine) does with names.
#[derive(Default)]
struct Scope {
    /// Lowercase routine name; "" for the main program.
    name: String,
    /// Names the routine declares itself (parameters, DIM, STATIC): not
    /// the main program's.
    own: HashSet<String>,
    /// Declarations: (name, type, has dimensions, REDIM, STATIC).
    decls: Vec<(String, Option<ArrKind>, bool, bool, bool)>,
    /// Names used bare (anything but an element, LBOUND / UBOUND's array or
    /// a user routine's argument).
    bare: Vec<String>,
    /// Every name used as an array (element, bound or argument).
    named: HashSet<String>,
    calls: Vec<UserCall>,
    /// GOTO / GOSUB / labels / inline Rust.
    blocked: bool,
}

struct Collect<'a> {
    scope: &'a mut Scope,
    is_routine: &'a dyn Fn(&str) -> bool,
}

impl Collect<'_> {
    fn bare(&mut self, name: &str) {
        self.scope.bare.push(key(name));
    }

    fn user_call(&mut self, callee: &str, args: &[Expression]) {
        let mut passed = Vec::with_capacity(args.len());
        for a in args {
            match a {
                Expression::Identifier(id) => {
                    self.scope.named.insert(key(&id.name));
                    passed.push(Some(key(&id.name)));
                }
                other => {
                    self.expr(other);
                    passed.push(None);
                }
            }
        }
        self.scope.calls.push(UserCall { callee: key(callee), args: passed });
    }

    /// An element: `a(i, …)` (as a value or a store's target).
    fn element(&mut self, name: &str, indices: &[Expression]) {
        self.scope.named.insert(key(name));
        for i in indices {
            self.expr(i);
        }
    }

    fn expr(&mut self, e: &Expression) {
        match e {
            Expression::Identifier(id) => self.bare(&id.name),
            Expression::Literal(_) => {}
            Expression::FunctionCall(fc) => match fc.callee.as_ref() {
                Expression::Identifier(id) => {
                    let name = key(&id.name);
                    let user = (self.is_routine)(&name);
                    if !user && matches!(name.as_str(), "lbound" | "ubound") && (1..=2).contains(&fc.args.len()) {
                        if let Some(Expression::Identifier(a)) = fc.args.first() {
                            self.scope.named.insert(key(&a.name));
                            for rest in &fc.args[1..] {
                                self.expr(rest);
                            }
                            return;
                        }
                    }
                    if user {
                        // (an array of the routine's name is still read as
                        // an element there: the callee counts as both)
                        self.scope.named.insert(name.clone());
                        self.user_call(&id.name, &fc.args);
                    } else {
                        self.element(&id.name, &fc.args);
                    }
                }
                other => {
                    self.expr(other);
                    for a in &fc.args {
                        self.expr(a);
                    }
                }
            },
            Expression::ArrayAccess(aa) => match aa.array.as_ref() {
                Expression::Identifier(id) => self.element(&id.name, &aa.indices),
                other => {
                    self.expr(other);
                    for i in &aa.indices {
                        self.expr(i);
                    }
                }
            },
            Expression::Binary(b) => {
                self.expr(&b.left);
                self.expr(&b.right);
            }
            Expression::Unary(u) => self.expr(&u.operand),
            Expression::MemberAccess(m) => self.expr(&m.object),
            Expression::MethodCall(m) => {
                self.expr(&m.object);
                for a in &m.args {
                    self.expr(a);
                }
            }
        }
    }

    /// A store's target.
    fn target(&mut self, t: &Expression) {
        match t {
            Expression::FunctionCall(fc) => match fc.callee.as_ref() {
                Expression::Identifier(id) => self.element(&id.name, &fc.args),
                _ => self.expr(t),
            },
            Expression::ArrayAccess(aa) => match aa.array.as_ref() {
                Expression::Identifier(id) => self.element(&id.name, &aa.indices),
                _ => self.expr(t),
            },
            other => self.expr(other),
        }
    }

    fn dims(&mut self, dims: &[ArrayDimension]) {
        for d in dims {
            match d {
                ArrayDimension::Single(e) => self.expr(e),
                ArrayDimension::Range { start, end } => {
                    self.expr(start);
                    self.expr(end);
                }
            }
        }
    }

    fn stmts(&mut self, body: &[Statement]) {
        for s in body {
            self.stmt(s);
        }
    }

    fn stmt(&mut self, s: &Statement) {
        match s {
            Statement::Assignment(a) => {
                self.target(&a.target);
                self.expr(&a.value);
            }
            Statement::Input(i) => {
                if let Some(p) = &i.prompt {
                    self.expr(p);
                }
                self.target(&i.target);
            }
            Statement::Call(c) => match &c.callee {
                Expression::Identifier(id) if (self.is_routine)(&key(&id.name)) => {
                    self.bare(&id.name);
                    self.user_call(&id.name, &c.args);
                }
                other => {
                    self.expr(other);
                    for a in &c.args {
                        self.expr(a);
                    }
                }
            },
            Statement::Dim(d) => {
                for v in &d.declarators {
                    let k = key(&v.name);
                    self.scope.decls.push((k.clone(), ArrKind::of_type(&d.type_name), !v.dimensions.is_empty(), d.is_redim, d.is_static));
                    if !self.scope.name.is_empty() && (!d.is_redim || d.is_static) {
                        self.scope.own.insert(k);
                    }
                    self.dims(&v.dimensions);
                }
            }
            Statement::For(f) => {
                self.bare(&f.variable);
                self.expr(&f.start);
                self.expr(&f.end);
                if let Some(st) = &f.step {
                    self.expr(st);
                }
                self.stmts(&f.body);
            }
            // (`Name:` naming a SUB is a call of it without arguments)
            Statement::Label(l) if (self.is_routine)(&key(&l.name)) => {
                self.scope.blocked = true;
                self.user_call(&l.name, &[]);
            }
            Statement::Goto(_) | Statement::Gosub(_) | Statement::Label(_) | Statement::RustBlock(_) => self.scope.blocked = true,
            Statement::If(i) => {
                self.expr(&i.condition);
                self.stmts(&i.then_body);
                for b in &i.elseif_branches {
                    self.expr(&b.condition);
                    self.stmts(&b.body);
                }
                self.stmts(&i.else_body);
            }
            Statement::While(w) => {
                self.expr(&w.condition);
                self.stmts(&w.body);
            }
            Statement::DoLoop(d) => {
                if let Some(c) = &d.condition {
                    self.expr(c);
                }
                self.stmts(&d.body);
            }
            Statement::SelectCase(c) => {
                self.expr(&c.expression);
                for case in &c.cases {
                    for v in &case.values {
                        match v {
                            rapidr_ast::CaseValue::Value(e) | rapidr_ast::CaseValue::Is(_, e) => self.expr(e),
                            rapidr_ast::CaseValue::Range(a, b) => {
                                self.expr(a);
                                self.expr(b);
                            }
                            rapidr_ast::CaseValue::IsLogic(_, e, rest) => {
                                self.expr(e);
                                for (_, x) in rest {
                                    self.expr(x);
                                }
                            }
                        }
                    }
                    self.stmts(&case.body);
                }
                self.stmts(&c.case_else);
            }
            Statement::With(w) => {
                self.expr(&w.object);
                self.stmts(&w.body);
            }
            Statement::Create(c) => self.stmts(&c.body),
            Statement::Print(p) => p.items.iter().for_each(|e| self.expr(e)),
            Statement::PrintHash(p) => {
                self.expr(&p.file_number);
                p.items.iter().for_each(|e| self.expr(e));
            }
            Statement::WriteHash(w) => {
                self.expr(&w.file_number);
                w.items.iter().for_each(|e| self.expr(e));
            }
            Statement::Open(o) => {
                self.expr(&o.filename);
                self.expr(&o.file_number);
            }
            Statement::Close(c) => self.expr(&c.file_number),
            Statement::Const(c) => self.expr(&c.value),
            Statement::Return(r) => r.value.iter().for_each(|e| self.expr(e)),
            Statement::Seek(s) => {
                self.expr(&s.file_number);
                self.expr(&s.position);
            }
            Statement::Bind(b) => {
                self.expr(&b.target);
                self.expr(&b.handler);
            }
            Statement::Subroutine(_) | Statement::Function(_) => {}
            // (lowered before code generation: `rapidr_ast::objects`) Every
            // name in one is used bare.
            Statement::Type(_) => rapidr_ast::walk(
                std::slice::from_ref(s),
                &mut |_| {},
                &mut |e| {
                    if let Expression::Identifier(id) = e {
                        self.scope.bare.push(key(&id.name));
                    }
                },
            ),
            Statement::Comment(_) | Statement::Declare(_) | Statement::Directive(_) | Statement::Exit(_) | Statement::Import(_) | Statement::Line(_) => {}
        }
    }
}

/// Routines named as a pointer: CODEPTR / CALLBACK's argument, an event's
/// handler (`OnClick = Clicked`, `Form.OnClose = …`), BIND's.
fn pointer_routines(program: &[Statement]) -> HashSet<String> {
    fn handler(e: &Expression, into: &mut HashSet<String>) {
        match e {
            Expression::Identifier(id) => {
                into.insert(key(&id.name));
            }
            Expression::MemberAccess(m) => {
                if let Expression::Identifier(o) = m.object.as_ref() {
                    into.insert(format!("{}.{}", o.name, m.member).to_lowercase());
                }
            }
            _ => {}
        }
    }
    fn on_event(t: &Expression) -> bool {
        let name = match t {
            Expression::Identifier(id) => &id.name,
            Expression::MemberAccess(m) => &m.member,
            _ => return false,
        };
        name.len() > 2 && name[..2].eq_ignore_ascii_case("on")
    }
    let mut out = HashSet::new();
    let mut found: Vec<Expression> = Vec::new();
    let mut in_calls: Vec<Expression> = Vec::new();
    rapidr_ast::walk(
        program,
        &mut |s| match s {
            Statement::Assignment(a) if on_event(&a.target) => found.push(a.value.clone()),
            Statement::Bind(b) => found.push(b.handler.clone()),
            _ => {}
        },
        &mut |e| {
            if let Expression::FunctionCall(fc) = e {
                if let Expression::Identifier(id) = fc.callee.as_ref() {
                    if matches!(id.name.to_ascii_lowercase().as_str(), "codeptr" | "callback" | "varptr" | "bind") {
                        in_calls.extend(fc.args.iter().cloned());
                    }
                }
            }
        },
    );
    for e in found.iter().chain(&in_calls) {
        handler(e, &mut out);
    }
    out
}

/// One type for every declaration of `name` in `decls` that has
/// dimensions, and none without (a scalar of the same name), none STATIC.
fn declared_kind(decls: &[(String, Option<ArrKind>, bool, bool, bool)], name: &str, redim_only_ok: bool) -> Option<ArrKind> {
    let mut kind = None;
    let mut dimmed = false;
    for (n, k, has_dims, redim, is_static) in decls.iter().filter(|d| d.0 == name) {
        let _ = n;
        if !has_dims || *is_static {
            return None;
        }
        let k = (*k)?;
        if kind.is_some_and(|prev| prev != k) {
            return None;
        }
        kind = Some(k);
        dimmed |= !redim;
    }
    kind.filter(|_| dimmed || redim_only_ok)
}

/// Which of `program`'s arrays are typed (see the module docs).
pub(crate) fn analyze(program: &[Statement], is_routine: &dyn Fn(&str) -> bool) -> TypedArrays {
    let mut inline_rust = false;
    rapidr_ast::walk(
        program,
        &mut |s| {
            if matches!(s, Statement::RustBlock(_)) {
                inline_rust = true;
            }
        },
        &mut |_| {},
    );
    if inline_rust {
        return TypedArrays::default();
    }
    let pointers = pointer_routines(program);

    // Each scope's uses.
    let mut main = Scope::default();
    let main_stmts: Vec<Statement> = program.iter().filter(|s| !matches!(s, Statement::Subroutine(_) | Statement::Function(_))).cloned().collect();
    Collect { scope: &mut main, is_routine }.stmts(&main_stmts);
    let mut routines: Vec<(Scope, Vec<Parameter>)> = Vec::new();
    for s in program {
        let (name, params, body) = match s {
            Statement::Subroutine(r) => (&r.name, &r.params, &r.body),
            Statement::Function(f) => (&f.name, &f.params, &f.body),
            _ => continue,
        };
        let mut scope = Scope { name: key(name), own: params.iter().map(|p| key(&p.name)).collect(), ..Default::default() };
        Collect { scope: &mut scope, is_routine }.stmts(body);
        routines.push((scope, params.clone()));
    }

    // Candidates: the main program's arrays…
    let mut globals: HashMap<String, ArrKind> = HashMap::new();
    let main_arrays: HashSet<String> = main.decls.iter().filter(|d| d.2).map(|d| d.0.clone()).collect();
    for name in &main_arrays {
        // (with the REDIMs of the routines that don't declare it)
        let mut decls = main.decls.clone();
        for (r, _) in &routines {
            if !r.own.contains(name) {
                decls.extend(r.decls.iter().filter(|d| &d.0 == name).cloned());
            }
        }
        if let Some(k) = declared_kind(&decls, name, true) {
            globals.insert(name.clone(), k);
        }
    }
    // …each routine's own, and its array parameters.
    let mut locals: HashMap<String, HashMap<String, ArrKind>> = HashMap::new();
    let mut params: HashMap<String, Vec<Option<ArrKind>>> = HashMap::new();
    for (r, ps) in &routines {
        let mut mine = HashMap::new();
        let mut typed_params = vec![None; ps.len()];
        if !r.blocked {
            let param_names: HashSet<String> = ps.iter().map(|p| key(&p.name)).collect();
            for (name, _, has_dims, redim, _) in &r.decls {
                if *has_dims && !*redim && !param_names.contains(name) {
                    if let Some(k) = declared_kind(&r.decls, name, false) {
                        mine.insert(name.clone(), k);
                    }
                }
            }
            // (a routine with a dotted name, `SUB Draw.Box`, is called as
            // `Draw.Box …`, a member's form: its parameters stay Values)
            if !pointers.contains(&r.name) && !r.name.contains('.') {
                for (i, p) in ps.iter().enumerate() {
                    let k = key(&p.name);
                    // (a DIM of it in the routine makes a new array; a
                    // REDIM of another type converts)
                    let redims_ok = r.decls.iter().filter(|d| d.0 == k).all(|d| d.3 && d.2 && !d.4 && d.1 == ArrKind::of_type(&p.type_name));
                    if let (true, Some(kind), true) = (p.is_array, ArrKind::of_type(&p.type_name), redims_ok) {
                        if ps.iter().filter(|q| key(&q.name) == k).count() == 1 {
                            typed_params[i] = Some(kind);
                            mine.insert(k, kind);
                        }
                    }
                }
            }
        }
        locals.insert(r.name.clone(), mine);
        params.insert(r.name.clone(), typed_params);
    }

    // Until nothing changes: drop every candidate used another way.
    let resolve = |scope: &Scope, name: &str, globals: &HashMap<String, ArrKind>, locals: &HashMap<String, HashMap<String, ArrKind>>| -> Option<(bool, ArrKind)> {
        if !scope.name.is_empty() && scope.own.contains(name) {
            return locals.get(&scope.name).and_then(|l| l.get(name)).map(|k| (false, *k));
        }
        globals.get(name).map(|k| (true, *k))
    };
    loop {
        let mut drop_global: HashSet<String> = HashSet::new();
        let mut drop_local: HashSet<(String, String)> = HashSet::new();
        let mut drop_param: HashSet<(String, usize)> = HashSet::new();
        let scopes = std::iter::once(&main).chain(routines.iter().map(|(s, _)| s));
        for scope in scopes {
            let mut drop = |name: &str, scope: &Scope| match resolve(scope, name, &globals, &locals) {
                Some((true, _)) => {
                    drop_global.insert(name.to_string());
                }
                Some((false, _)) => {
                    drop_local.insert((scope.name.clone(), name.to_string()));
                }
                None => {}
            };
            for name in &scope.bare {
                drop(name, scope);
            }
            for call in &scope.calls {
                let callee_params = params.get(&call.callee).cloned().unwrap_or_default();
                for i in 0..callee_params.len().max(call.args.len()) {
                    let want = callee_params.get(i).copied().flatten();
                    let passed = call.args.get(i).cloned().flatten();
                    let got = passed.as_deref().and_then(|n| resolve(scope, n, &globals, &locals)).map(|(_, k)| k);
                    match (want, got) {
                        (Some(w), Some(g)) if w == g => {}
                        _ => {
                            if want.is_some() {
                                drop_param.insert((call.callee.clone(), i));
                            }
                            if let Some(n) = &passed {
                                drop(n, scope);
                            }
                        }
                    }
                }
            }
        }
        if drop_global.is_empty() && drop_local.is_empty() && drop_param.is_empty() {
            break;
        }
        for g in drop_global {
            globals.remove(&g);
        }
        for (r, n) in drop_local {
            if let Some(l) = locals.get_mut(&r) {
                l.remove(&n);
            }
            // (a parameter dropped as a local is dropped as a parameter)
            if let (Some(ps), Some((_, rps))) = (params.get_mut(&r), routines.iter().find(|(s, _)| s.name == r)) {
                for (i, p) in rps.iter().enumerate() {
                    if key(&p.name) == n {
                        ps[i] = None;
                    }
                }
            }
        }
        for (r, i) in drop_param {
            if let Some(ps) = params.get_mut(&r) {
                if let Some(slot) = ps.get_mut(i) {
                    *slot = None;
                }
            }
            if let (Some(l), Some((_, rps))) = (locals.get_mut(&r), routines.iter().find(|(s, _)| s.name == r)) {
                if let Some(p) = rps.get(i) {
                    l.remove(&key(&p.name));
                }
            }
        }
    }

    // The main-program arrays each scope names, for its start.
    let mut uses_globals: HashMap<String, Vec<String>> = HashMap::new();
    for scope in std::iter::once(&main).chain(routines.iter().map(|(s, _)| s)) {
        let mut used: Vec<String> = scope
            .named
            .iter()
            .chain(scope.decls.iter().map(|d| &d.0))
            .filter(|n| globals.contains_key(*n) && (scope.name.is_empty() || !scope.own.contains(*n)))
            .cloned()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        used.sort();
        uses_globals.insert(scope.name.clone(), used);
    }
    TypedArrays { globals, locals, params, uses_globals }
}

/// Where a typed array lives in the routine being emitted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Place {
    /// The routine's own (`let buf = TypedArray::new()`).
    Owned,
    /// A parameter (`buf: &TypedArray<f64>`).
    Param,
}

impl RustCodegen {
    /// The typed array `name` names here, as its type and a Rust expression
    /// of type `&TypedArray<_>`.
    pub(crate) fn typed_array(&self, name: &str) -> Option<(ArrKind, String)> {
        if self.typed_arrays.is_empty() {
            return None;
        }
        let k = key(name);
        if let Some((kind, place)) = self.ta_scope.get(&k) {
            let snake = to_snake(&k);
            return Some((*kind, if *place == Place::Owned { format!("(&{snake})") } else { snake }));
        }
        if self.in_sub_or_function && self.shadowed.contains(&k) {
            return None;
        }
        self.typed_arrays.globals.get(&k).map(|kind| (*kind, global_local(&k)))
    }

    /// The typed array an element access's callee names.
    pub(crate) fn typed_array_of(&self, callee: &Expression) -> Option<(ArrKind, String)> {
        match callee {
            Expression::Identifier(id) => self.typed_array(&id.name),
            _ => None,
        }
    }

    /// An index as an `i64`, as `Value::to_i64` reads it.
    fn index_i64(&self, e: &Expression) -> String {
        match self.typed_expr(e) {
            Some((c, Ty::Int)) => c,
            Some((c, Ty::Float)) => format!("(({c}) as i64)"),
            Some((c, Ty::Bool)) => format!("(if {c} {{ -1_i64 }} else {{ 0_i64 }})"),
            None => format!("({}).to_i64()", self.expr_to_string(e)),
        }
    }

    /// `get1(i)` / `get2(i, j)` / `get(&[…])` — `method` the prefix.
    fn element_call(&self, array: &str, method: &str, indices: &[Expression], value: Option<String>) -> String {
        let idx: Vec<String> = indices.iter().map(|e| self.index_i64(e)).collect();
        let tail = value.map(|v| format!(", {v}")).unwrap_or_default();
        match idx.len() {
            1 => format!("{array}.{method}1({}{tail})", idx[0]),
            2 if method != "value" => format!("{array}.{method}2({}, {}{tail})", idx[0], idx[1]),
            _ => format!("{array}.{method}(&[{}]{tail})", idx.join(", ")),
        }
    }

    /// An element read as a typed (native) expression.
    pub(crate) fn typed_element(&self, kind: ArrKind, array: &str, indices: &[Expression]) -> Option<(String, Ty)> {
        let ty = kind.read_ty()?;
        let call = self.element_call(array, "get", indices, None);
        Some(match ty {
            Ty::Float if kind == ArrKind::Double => (call, Ty::Float),
            Ty::Float => (format!("({call} as f64)"), Ty::Float),
            _ => (format!("({call} as i64)"), Ty::Int),
        })
    }

    /// An element read as a `Value`.
    pub(crate) fn element_value(&self, array: &str, indices: &[Expression]) -> String {
        self.element_call(array, "value", indices, None)
    }

    /// `a(i, …) = value` into a typed array.
    pub(crate) fn element_store(&self, kind: ArrKind, array: &str, indices: &[Expression], value: &Expression) -> String {
        // (the value is the store's conversion, `rapidr_ast::numeric`; made
        // here when it isn't)
        let inner = match value {
            Expression::FunctionCall(fc) if fc.args.len() == 1 => match fc.callee.as_ref() {
                Expression::Identifier(id) if id.name.eq_ignore_ascii_case(kind.conversion()) => &fc.args[0],
                _ => value,
            },
            _ => value,
        };
        let number = match kind.scalar() {
            Some(Kind::Double) => self.typed_store(Kind::Double, inner),
            Some(k) => format!("({}) as {}", self.typed_store(k, inner), kind.elem()),
            None if kind == ArrKind::Single => match self.typed_expr(inner) {
                Some((c, Ty::Float)) => format!("({c}) as f32"),
                Some((c, Ty::Int)) => format!("(({c}) as f64) as f32"),
                Some((c, Ty::Bool)) => format!("(if {c} {{ -1.0_f32 }} else {{ 0.0_f32 }})"),
                None => format!("(numeric::double_of(&{})) as f32", self.expr_to_string(inner)),
            },
            None => {
                let (wide, cast) = if kind == ArrKind::UInt64 { ("UInt64", " as u64") } else { ("Int64", "") };
                format!("numeric::int_of(&{}, numeric::NumKind::{wide}){cast}", self.expr_to_string(inner))
            }
        };
        format!("{};", self.element_call(array, "set", indices, Some(number)))
    }

    /// `LBOUND(a [, d])` / `UBOUND(…)` (`name`, lowercase) of a typed array:
    /// the array and whether it is the upper bound.
    pub(crate) fn typed_bound_call(&self, name: &str, args: &[Expression]) -> Option<(String, bool)> {
        let upper = match name {
            "lbound" => false,
            "ubound" => true,
            _ => return None,
        };
        if self.defined_functions.contains(name) || !(1..=2).contains(&args.len()) {
            return None;
        }
        let Expression::Identifier(id) = &args[0] else { return None };
        let (_, array) = self.typed_array(&id.name)?;
        Some((array, upper))
    }

    /// LBOUND / UBOUND of a typed array, as a `Value`.
    pub(crate) fn typed_bound(&self, array: &str, dim: Option<&Expression>, upper: bool) -> String {
        let d = dim.map(|e| self.owned_expr(e)).unwrap_or_else(|| "v_null()".into());
        format!("{array}.bound(&{d}, {upper})")
    }

    /// LBOUND / UBOUND as a typed integer, when its dimension is.
    pub(crate) fn typed_bound_i64(&self, array: &str, dim: Option<&Expression>, upper: bool) -> Option<(String, Ty)> {
        let d = match dim {
            None => "1_i64".to_string(),
            Some(e) => match self.typed_expr(e)? {
                (c, Ty::Int) => c,
                (c, Ty::Float) => format!("(({c}) as i64)"),
                (c, Ty::Bool) => format!("(if {c} {{ -1_i64 }} else {{ 0_i64 }})"),
            },
        };
        Some((format!("{array}.bound_of({d}, {upper})"), Ty::Int))
    }

    /// `(callee)(…, name, …)`: the typed array passed at position `i` of a
    /// call to the program's routine `callee`, when that parameter is one.
    pub(crate) fn typed_argument(&self, callee: &str, i: usize, arg: &Expression) -> Option<String> {
        let kind = self.typed_arrays.params.get(&key(callee))?.get(i).copied().flatten()?;
        let Expression::Identifier(id) = arg else { return None };
        let (k, r) = self.typed_array(&id.name)?;
        (k == kind).then_some(r)
    }

    /// The routine `name`'s typed array parameter at `i`.
    pub(crate) fn typed_param(&self, routine: &str, i: usize) -> Option<ArrKind> {
        self.typed_arrays.params.get(&key(routine))?.get(i).copied().flatten()
    }

    /// The thread-local statics of the main program's typed arrays.
    pub(crate) fn emit_typed_array_statics(&mut self) {
        let mut names: Vec<(String, ArrKind)> = self.typed_arrays.globals.iter().map(|(n, k)| (n.clone(), *k)).collect();
        if names.is_empty() {
            return;
        }
        names.sort_by(|a, b| a.0.cmp(&b.0));
        // (in blocks of at most 64: one `thread_local!` expands its items
        // recursively, and 128 or more of them reach rustc's macro recursion
        // limit - a program with that many typed arrays didn't build)
        for chunk in names.chunks(64) {
            self.line("thread_local! {");
            for (name, kind) in chunk {
                // (made once, never freed: a routine takes it as `&'static`)
                self.line(&format!(
                    "    static {}: &'static TypedArray<{}> = Box::leak(Box::new(TypedArray::new()));",
                    global_static(name),
                    kind.elem()
                ));
            }
            self.line("}");
        }
    }

    /// At a routine's (or the main program's, `""`) start: the main
    /// program's typed arrays it uses, and its own.
    pub(crate) fn emit_typed_array_locals(&mut self, routine: &str, params: &[Parameter]) {
        let used = self.typed_arrays.uses_globals.get(&key(routine)).cloned().unwrap_or_default();
        for name in used {
            if self.in_sub_or_function && self.shadowed.contains(&name) {
                continue;
            }
            let kind = self.typed_arrays.globals[&name];
            self.line(&format!("let {}: &'static TypedArray<{}> = {}.with(|a| *a);", global_local(&name), kind.elem(), global_static(&name)));
        }
        if routine.is_empty() {
            return;
        }
        let param_names: HashSet<String> = params.iter().map(|p| key(&p.name)).collect();
        let mut own: Vec<(String, ArrKind)> = self.ta_scope.iter().filter(|(n, (_, p))| *p == Place::Owned && !param_names.contains(*n)).map(|(n, (k, _))| (n.clone(), *k)).collect();
        own.sort_by(|a, b| a.0.cmp(&b.0));
        for (name, kind) in own {
            self.line(&format!("let {}: TypedArray<{}> = TypedArray::new();", to_snake(&name), kind.elem()));
        }
    }

    /// The current routine's typed arrays (`ta_scope`), from the analysis.
    pub(crate) fn enter_typed_arrays(&mut self, routine: &str, params: &[Parameter]) {
        self.ta_scope.clear();
        let Some(mine) = self.typed_arrays.locals.get(&key(routine)) else { return };
        let param_names: HashSet<String> = params.iter().enumerate().filter(|(i, _)| self.typed_param(routine, *i).is_some()).map(|(_, p)| key(&p.name)).collect();
        for (name, kind) in mine {
            let place = if param_names.contains(name) { Place::Param } else { Place::Owned };
            self.ta_scope.insert(name.clone(), (*kind, place));
        }
    }

    /// `DIM` / `REDIM` of typed array `name`: its statement, if it is one.
    pub(crate) fn typed_dim(&self, name: &str, bounds: &[String], redim: bool) -> Option<String> {
        let (_, array) = self.typed_array(name)?;
        Some(format!("{array}.{}(&[{}]);", if redim { "redim" } else { "dim" }, bounds.join(", ")))
    }
}

#[cfg(test)]
mod tests {
    fn gen(code: &str) -> String {
        let tokens = rapidr_lexer::Lexer::new(code, None).tokenize().unwrap();
        let program = rapidr_parser::parse_tokens(&tokens).expect("test source should parse");
        crate::generate(&program)
    }

    #[test]
    fn a_global_double_array_used_by_element_is_a_vector() {
        let rust = gen("DIM v(3) AS DOUBLE\nDIM x AS DOUBLE\nSUB S\n  v(1) = v(0) + 2\nEND SUB\nv(0) = 1.5\nS\nx = v(1) * 2\nPRINT x; UBOUND(v)\n");
        assert!(rust.contains("static TA_V: &'static TypedArray<f64>"), "{rust}");
        assert!(rust.contains("__ta_v.set1(1_i64, ((__ta_v.get1(0_i64)) + ((2_i64) as f64)));"), "{rust}");
        assert!(rust.contains("__ta_v.dim(&["), "{rust}");
        assert!(!rust.contains("rp_new_array"), "{rust}");
    }

    #[test]
    fn an_array_used_whole_stays_a_value() {
        // Passed to a builtin, assigned whole, its address taken, swapped.
        for use_it in ["PRINT LEN(a)", "b = a", "p = VARPTR(a(0))", "SWAP a, c"] {
            let rust = gen(&format!("DIM a(3) AS INTEGER\nDIM c(3) AS INTEGER\na(1) = 2\n{use_it}\n"));
            assert!(rust.contains("rp_new_array(&[(0, (v_int(3)).to_i64())], v_int(0))"), "{use_it}: {rust}");
            assert!(!rust.contains("TA_A"), "{use_it}: {rust}");
        }
    }

    #[test]
    fn parameters_are_typed_when_every_call_passes_one() {
        let typed = gen("DIM a(3) AS DOUBLE\nSUB Fill(x() AS DOUBLE)\n  x(0) = 1\nEND SUB\nFill a\n");
        assert!(typed.contains("fn fill(x: &TypedArray<f64>)"), "{typed}");
        assert!(typed.contains("fill(__ta_a)"), "{typed}");
        // An INTEGER array passed to a DOUBLE parameter: both stay Values.
        let mixed = gen("DIM a(3) AS DOUBLE\nDIM b(3) AS INTEGER\nSUB Fill(x() AS DOUBLE)\n  x(0) = 1\nEND SUB\nFill a\nFill b\n");
        assert!(mixed.contains("fn fill(mut x: Value)"), "{mixed}");
        assert!(!mixed.contains("TA_B") && !mixed.contains("TA_A"), "{mixed}");
        // A routine named as a pointer is called with Values.
        let pointer = gen("DIM a(3) AS DOUBLE\nSUB Fill(x() AS DOUBLE)\n  x(0) = 1\nEND SUB\np = CODEPTR(Fill)\nFill a\n");
        assert!(pointer.contains("fn fill(mut x: Value)"), "{pointer}");
        // A dotted name is called in a member's form; `Name:` calls with
        // no arguments.
        let dotted = gen("DIM a(3) AS DOUBLE\nSUB Draw.Fill(x() AS DOUBLE)\n  x(0) = 1\nEND SUB\nDraw.Fill a\n");
        assert!(dotted.contains("fn draw_fill(mut x: Value)"), "{dotted}");
        let label = gen("DIM a(3) AS DOUBLE\nSUB Fill(x() AS DOUBLE)\n  x(0) = 1\nEND SUB\nFill a\nFill:\n");
        assert!(label.contains("fn fill(mut x: Value)"), "{label}");
    }

    #[test]
    fn a_routines_own_array_and_one_with_goto() {
        let rust = gen("SUB S\n  DIM buf(7) AS BYTE\n  buf(1) = 300\n  PRINT buf(1)\nEND SUB\nSUB J\n  DIM own(2) AS DOUBLE\nagain:\n  own(0) = 1\n  IF own(0) = 0 THEN GOTO again\nEND SUB\nS\nJ\n");
        assert!(rust.contains("let buf: TypedArray<u8> = TypedArray::new();"), "{rust}");
        assert!(rust.contains("rp_new_array(&[(0, (v_int(2)).to_i64())], v_dbl(0.0))"), "{rust}");
    }

    #[test]
    fn inline_rust_turns_typed_arrays_off() {
        let rust = gen("DIM a(3) AS DOUBLE\na(1) = 2\nRUSTSTART\nlet _x = 1;\nRUSTEND\n");
        assert!(!rust.contains("TypedArray"), "{rust}");
    }
}

//! `FOR i AS INTEGER = 1 TO n [STEP s] … NEXT` — a RapidR extension
//! (docs/rai-mistake-ledger.md): the loop most BASICs (and every model
//! trained on them) write, which RapidQ's compiler refuses. Additive: a
//! plain `FOR i = …` is unchanged.
//!
//! The counter is declared where it isn't yet, as a `DIM` would:
//! - in a SUB or FUNCTION, a local of the routine (at its top — the same
//!   place [`crate::for_locals`] puts an undeclared counter), unless the
//!   routine declares it itself;
//! - in the main program, a global, declared just before the loop, unless
//!   the main program (any of its files) declares it.
//!
//! A counter already declared with the same type is that variable (`DIM i
//! AS INTEGER` … `FOR i AS INTEGER`, or two loops on `i`); one declared
//! with another type, as an array, a constant or a component is an error,
//! and so is a type that isn't a number ([`errors`]). This is the first pass
//! both backends run ([`lower`]), so the bytecode compiler, native builds
//! and the web see the same program.

use std::collections::HashMap;

use rapidr_diagnostics::TextSpan;

use crate::*;

/// The types a FOR counter may have: numbers.
pub const COUNTER_TYPES: &[&str] = &["BYTE", "SHORT", "WORD", "INTEGER", "DWORD", "LONG", "SINGLE", "DOUBLE", "INT64", "CURRENCY"];

fn key(name: &str) -> String {
    strip_type_suffix(name).to_ascii_lowercase()
}

/// What a scope declares a name as.
#[derive(Clone, Debug, PartialEq)]
enum Decl {
    /// A variable of this type (upper case); `array`: with dimensions.
    Var { ty: String, array: bool, at: usize },
    Const { at: usize },
    Component { at: usize },
}

/// What a scope (the main program, or one routine) declares itself:
/// DIMs (STATIC, REDIM), CONSTs, CREATEs, not its nested routines'.
fn declared(stmts: &[Statement], into: &mut HashMap<String, Decl>) {
    for s in stmts {
        match s {
            Statement::Dim(d) => {
                for v in &d.declarators {
                    into.entry(key(&v.name)).or_insert(Decl::Var { ty: d.type_name.to_ascii_uppercase(), array: !v.dimensions.is_empty(), at: v.span.start });
                }
            }
            Statement::Const(c) => {
                into.entry(key(&c.name)).or_insert(Decl::Const { at: c.span.start });
            }
            Statement::Create(c) => {
                into.entry(key(&c.name)).or_insert(Decl::Component { at: c.span.start });
                declared(&c.body, into);
            }
            Statement::Subroutine(_) | Statement::Function(_) | Statement::Type(_) => {}
            other => {
                for body in child_bodies(other) {
                    declared(body, into);
                }
            }
        }
    }
}

/// The statement lists directly inside a block statement (not routines').
fn child_bodies(s: &Statement) -> Vec<&Vec<Statement>> {
    match s {
        Statement::If(i) => {
            let mut v = vec![&i.then_body];
            v.extend(i.elseif_branches.iter().map(|b| &b.body));
            v.push(&i.else_body);
            v
        }
        Statement::For(f) => vec![&f.body],
        Statement::While(w) => vec![&w.body],
        Statement::DoLoop(d) => vec![&d.body],
        Statement::With(w) => vec![&w.body],
        Statement::SelectCase(c) => {
            let mut v: Vec<&Vec<Statement>> = c.cases.iter().map(|b| &b.body).collect();
            v.push(&c.case_else);
            v
        }
        _ => Vec::new(),
    }
}

/// The error for `FOR var AS ty` given what the scope has, if any.
fn conflict(var: &str, ty: &str, had: Option<&Decl>) -> Option<String> {
    let ty_up = ty.to_ascii_uppercase();
    if !COUNTER_TYPES.contains(&ty_up.as_str()) {
        return Some(format!("FOR {var} AS {ty}: a FOR counter is a number — write FOR {var} AS INTEGER (or LONG, DOUBLE …)"));
    }
    let plain = format!("use another name for the counter, or write FOR {var} = … (without AS) to count with it");
    match had? {
        Decl::Var { ty: t, array: false, .. } if *t == ty_up => None,
        Decl::Var { array: true, at, .. } => Some(format!("FOR {var} AS {ty}: {var} is an array (declared in {}) — use another name for the counter", rapidr_diagnostics::place_mark(*at))),
        Decl::Var { ty: t, at, .. } => Some(format!("FOR {var} AS {ty}: {var} is already declared AS {t} (in {}) — {plain}", rapidr_diagnostics::place_mark(*at))),
        Decl::Const { at } => Some(format!("FOR {var} AS {ty}: {var} is a constant (declared in {}) — use another name for the counter", rapidr_diagnostics::place_mark(*at))),
        Decl::Component { at } => Some(format!("FOR {var} AS {ty}: {var} is a component (made in {}) — use another name for the counter", rapidr_diagnostics::place_mark(*at))),
    }
}

/// The new DIM of a counter.
fn dim(var: &str, ty: &str, span: TextSpan) -> Statement {
    Statement::Dim(DimStatement {
        span,
        declarators: vec![VariableDeclarator { span, name: var.to_string(), dimensions: Vec::new() }],
        type_name: ty.to_ascii_uppercase(),
        fixed_len: None,
        is_static: false,
        is_redim: false,
    })
}

struct Pass {
    errors: Vec<(TextSpan, String)>,
}

impl Pass {
    /// One scope's block: each `FOR … AS` resolved, in order. `top`: a
    /// routine's DIMs for its top (`None` in the main program, where the
    /// DIM goes just before the loop).
    fn block(&mut self, stmts: &mut Vec<Statement>, scope: &mut HashMap<String, Decl>, top: &mut Option<Vec<Statement>>) {
        let mut out = Vec::with_capacity(stmts.len());
        for mut s in std::mem::take(stmts) {
            if let Statement::For(f) = &mut s {
                if let Some(ty) = f.declared_type.take() {
                    let k = key(&f.variable);
                    // (`FOR obj.x AS …` isn't a counter of its own: as written)
                    if f.variable.contains('.') {
                        self.errors.push((f.span, format!("FOR {} AS {ty}: AS declares a new counter, not a field — write FOR {} = …", f.variable, f.variable)));
                    } else if let Some(e) = conflict(&f.variable, &ty, scope.get(&k)) {
                        self.errors.push((f.span, e));
                    } else if !scope.contains_key(&k) {
                        scope.insert(k, Decl::Var { ty: ty.to_ascii_uppercase(), array: false, at: f.span.start });
                        match top {
                            Some(t) => t.push(dim(&f.variable, &ty, f.span)),
                            None => out.push(dim(&f.variable, &ty, f.span)),
                        }
                    }
                }
            }
            match &mut s {
                Statement::Subroutine(_) | Statement::Function(_) | Statement::Type(_) => {}
                Statement::If(i) => {
                    self.block(&mut i.then_body, scope, top);
                    for b in &mut i.elseif_branches {
                        self.block(&mut b.body, scope, top);
                    }
                    self.block(&mut i.else_body, scope, top);
                }
                Statement::For(f) => self.block(&mut f.body, scope, top),
                Statement::While(w) => self.block(&mut w.body, scope, top),
                Statement::DoLoop(d) => self.block(&mut d.body, scope, top),
                Statement::With(w) => self.block(&mut w.body, scope, top),
                Statement::SelectCase(c) => {
                    for b in &mut c.cases {
                        self.block(&mut b.body, scope, top);
                    }
                    self.block(&mut c.case_else, scope, top);
                }
                _ => {}
            }
            out.push(s);
        }
        *stmts = out;
    }

    /// A routine's body: its parameters and own declarations are its scope.
    fn routine(&mut self, params: &[Parameter], body: &mut Vec<Statement>) {
        let mut scope: HashMap<String, Decl> = HashMap::new();
        for p in params {
            scope.insert(key(&p.name), Decl::Var { ty: p.type_name.to_ascii_uppercase(), array: p.is_array, at: p.span.start });
        }
        declared(body, &mut scope);
        let mut top = Some(Vec::new());
        self.block(body, &mut scope, &mut top);
        let dims = top.unwrap_or_default();
        if !dims.is_empty() {
            body.splice(0..0, dims);
        }
    }

    /// A TYPE's methods, constructor and events.
    fn type_routines(&mut self, t: &mut TypeStatement) {
        for m in &mut t.methods {
            match m {
                Statement::Subroutine(r) => self.routine(&r.params.clone(), &mut r.body),
                Statement::Function(f) => self.routine(&f.params.clone(), &mut f.body),
                _ => {}
            }
        }
        self.routine(&[], &mut t.constructor);
        for e in &mut t.events {
            self.routine(&e.params.clone(), &mut e.body);
        }
    }
}

fn has_for_as(stmts: &[Statement]) -> bool {
    let mut found = false;
    walk(
        stmts,
        &mut |s| {
            if let Statement::For(f) = s {
                found |= f.declared_type.is_some();
            }
        },
        &mut |_| {},
    );
    found
}

/// [`lower`] and [`errors`] at once.
pub fn lower_with_errors(program: &Program) -> (Program, Vec<(TextSpan, String)>) {
    let mut program = program.clone();
    let mut pass = Pass { errors: Vec::new() };
    if !has_for_as(&program.statements) {
        return (program, pass.errors);
    }
    let mut main: HashMap<String, Decl> = HashMap::new();
    declared(&program.statements, &mut main);
    for s in &mut program.statements {
        match s {
            Statement::Subroutine(r) => pass.routine(&r.params.clone(), &mut r.body),
            Statement::Function(f) => pass.routine(&f.params.clone(), &mut f.body),
            Statement::Type(t) => pass.type_routines(t),
            _ => {}
        }
    }
    pass.block(&mut program.statements, &mut main, &mut None);
    (program, pass.errors)
}

/// The program with every `FOR … AS` counter declared (see the module
/// docs) and the loops plain FORs; a loop in error is left a plain FOR
/// ([`errors`] says why).
pub fn lower(program: &Program) -> Program {
    lower_with_errors(program).0
}

/// What is wrong with the program's `FOR … AS` loops: a type that isn't a
/// number, a name declared otherwise. Both compilers report these.
pub fn errors(program: &Program) -> Vec<(TextSpan, String)> {
    lower_with_errors(program).1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sp(a: usize) -> TextSpan {
        TextSpan::new(a, a + 1)
    }

    fn lit(n: i64) -> Expression {
        Expression::Literal(Literal { span: sp(0), value: LiteralValue::Integer(n) })
    }

    fn for_as(var: &str, ty: Option<&str>, at: usize) -> Statement {
        Statement::For(ForStatement { span: sp(at), variable: var.into(), declared_type: ty.map(String::from), start: lit(1), end: lit(3), step: None, body: Vec::new() })
    }

    fn dim_of(var: &str, ty: &str, at: usize) -> Statement {
        dim(var, ty, sp(at))
    }

    fn prog(statements: Vec<Statement>) -> Program {
        Program { span: TextSpan::default(), statements }
    }

    fn dims_in(stmts: &[Statement]) -> Vec<(String, String)> {
        stmts.iter().filter_map(|s| if let Statement::Dim(d) = s { Some((d.declarators[0].name.clone(), d.type_name.clone())) } else { None }).collect()
    }

    #[test]
    fn main_program_counter_is_declared_before_its_loop() {
        let p = prog(vec![for_as("i", Some("INTEGER"), 10), for_as("i", Some("INTEGER"), 20)]);
        assert!(errors(&p).is_empty());
        let l = lower(&p);
        // (one DIM, before the first loop; the second loop counts with it)
        assert_eq!(dims_in(&l.statements), vec![("i".to_string(), "INTEGER".to_string())]);
        assert!(matches!(&l.statements[0], Statement::Dim(_)));
        assert!(l.statements.iter().all(|s| !matches!(s, Statement::For(f) if f.declared_type.is_some())));
    }

    #[test]
    fn same_type_is_the_same_variable_another_is_an_error() {
        let p = prog(vec![dim_of("i", "INTEGER", 1), for_as("i", Some("integer"), 10)]);
        assert!(errors(&p).is_empty());
        assert_eq!(dims_in(&lower(&p).statements).len(), 1);
        let p = prog(vec![dim_of("i", "STRING", 1), for_as("i", Some("INTEGER"), 10)]);
        let e = errors(&p);
        assert_eq!(e.len(), 1);
        assert!(e[0].1.starts_with("FOR i AS INTEGER: i is already declared AS STRING (in {{at:1}})"), "{e:?}");
        let p = prog(vec![for_as("s", Some("STRING"), 10)]);
        assert!(errors(&p)[0].1.contains("a FOR counter is a number"));
    }

    #[test]
    fn in_a_routine_the_counter_is_a_local_at_its_top() {
        let sub = Statement::Subroutine(SubroutineStatement { span: sp(5), name: "Count".into(), params: Vec::new(), body: vec![Statement::Comment(CommentStatement { span: sp(6), text: String::new() }), for_as("k", Some("LONG"), 10)] });
        // (a global of the same name doesn't matter: the routine's own)
        let p = prog(vec![dim_of("k", "STRING", 1), sub]);
        assert!(errors(&p).is_empty(), "{:?}", errors(&p));
        let l = lower(&p);
        let Statement::Subroutine(r) = &l.statements[1] else { panic!() };
        assert_eq!(dims_in(&r.body), vec![("k".to_string(), "LONG".to_string())]);
        assert!(matches!(&r.body[0], Statement::Dim(_)));
        // (a parameter of another type: an error)
        let sub = Statement::Subroutine(SubroutineStatement {
            span: sp(5),
            name: "Count".into(),
            params: vec![Parameter { span: sp(7), name: "k".into(), type_name: "STRING".into(), by_ref: false, is_array: false }],
            body: vec![for_as("k", Some("INTEGER"), 10)],
        });
        assert_eq!(errors(&prog(vec![sub])).len(), 1);
    }

    #[test]
    fn plain_for_is_unchanged() {
        let p = prog(vec![for_as("i", None, 10)]);
        assert_eq!(lower(&p), p);
        assert!(errors(&p).is_empty());
    }

    #[test]
    fn the_nearest_names_are_suggested() {
        use crate::{closest_names, unknown_routine_fix};
        assert_eq!(closest_names("ShowNumbr", ["ShowNumber", "Hide"]), vec!["ShowNumber"]);
        assert_eq!(closest_names("zzz", ["ShowNumber"]), Vec::<&str>::new());
        assert_eq!(unknown_routine_fix("ShowNumbr", &["ShowNumber"]), " — did you mean ShowNumber?");
        // (a tie: both, honestly)
        let f = unknown_routine_fix("NStr", &[]);
        assert!(f.contains("STR$") && f.contains(" or "), "{f}");
        assert!(unknown_routine_fix("srvOn", &["StartServer"]).contains("no file of the program has a SUB or FUNCTION of that name"));
    }
}

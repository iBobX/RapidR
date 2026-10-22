//! RapidQ's standard constants in RapidR's own programs: built in.
//!
//! A RapidQ program (`.bas`, `.rqw` …) gets `mbYes`, `clRed`, `crHandPoint`
//! and the rest of RAPIDQ.INC's constants, and its `QBColor` array, only
//! from `$INCLUDE "RAPIDQ.INC"` — without the line, RapidQ reads such a
//! name as an undeclared variable, 0, and so does RapidR there (unchanged).
//! A RapidR program (`.rr`: the preprocessor marks it with
//! [`rapidr_preprocessor::RAPIDR_PROGRAM`]) has them with no include, as
//! VB and Delphi programs have theirs. This pass, the first both backends
//! run, gives them:
//!
//! - **The constants** become their numbers where the program reads them
//!   (only those it uses: nothing is declared, the debugger lists none).
//! - **Lowest priority**: a name the program makes its own — a DIM, CONST,
//!   CREATE, SUB, FUNCTION, DECLARE, TYPE, a parameter or a TYPE's field,
//!   or a variable it stores into (`=`, INPUT, FOR, SWAP, READ, INC) — is
//!   the program's in that place: the main program's own names everywhere,
//!   a routine's own in that routine (`DIM mbYes AS INTEGER = 5` prints 5).
//! - **`QBColor`**, an assignable `INTEGER` array `(0 TO 15)`, is declared
//!   and filled at the program's start, as the include does it, when the
//!   program uses it and doesn't declare its own.
//! - **`True` and `False` stay RapidR's** (-1 and 0, what a comparison
//!   gives): RAPIDQ.INC's `True` is 1, which a RapidR program gets only
//!   with the include, as before.
//!
//! `$INCLUDE "RAPIDQ.INC"` in a RapidR program still works: its constants
//! and array are then the program's own, and nothing here applies.
//!
//! What the debugger evaluates (Immediate, Watch, data tips) goes through
//! the same pass ([`lower_snippet`]), so `?mbYes` there is what it is in
//! the program.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use rapidr_diagnostics::TextSpan;

use crate::*;

/// The name of RAPIDQ.INC's array of QBasic's 16 colours.
pub const QBCOLOR: &str = "QBColor";

fn key(name: &str) -> String {
    strip_type_suffix(name).to_ascii_lowercase()
}

/// RAPIDQ.INC's `True` / `False`: not built in (a RapidR program's own are -1 / 0).
fn rapidq_only(name: &str) -> bool {
    name.eq_ignore_ascii_case("true") || name.eq_ignore_ascii_case("false")
}

fn table() -> &'static HashMap<String, i64> {
    static TABLE: OnceLock<HashMap<String, i64>> = OnceLock::new();
    TABLE.get_or_init(|| constants().map(|(n, v)| (n.to_ascii_lowercase(), v)).collect())
}

/// The constants a RapidR program has built in (RAPIDQ.INC's, in its
/// order, but `True` and `False`), with RAPIDQ.INC's spelling.
pub fn constants() -> impl Iterator<Item = (&'static str, i64)> {
    rapidr_preprocessor::RAPIDQ_INC_CONSTANTS.iter().filter(|(n, _)| !rapidq_only(n)).map(|&(n, v)| (n, v))
}

/// The value of built-in constant `name` (any case, a type suffix ignored),
/// if it is one.
pub fn constant(name: &str) -> Option<i64> {
    table().get(&key(name)).copied()
}

/// Whether `name` is one of the names a RapidR program has built in: a
/// constant or `QBColor`.
pub fn is_builtin_name(name: &str) -> bool {
    constant(name).is_some() || key(name) == "qbcolor"
}

fn is_mark(s: &Statement) -> bool {
    let value = rapidr_preprocessor::RAPIDR_PROGRAM.trim_start_matches("$OPTION").trim();
    matches!(s, Statement::Directive(d) if d.name.eq_ignore_ascii_case("$OPTION") && d.value.as_deref().is_some_and(|v| v.trim().eq_ignore_ascii_case(value)))
}

/// The preprocessor's mark of a RapidR program, if `program` has it
/// (anywhere: a block left open while the program is being typed holds it).
fn mark(program: &Program) -> Option<TextSpan> {
    let mut found = None;
    walk(
        &program.statements,
        &mut |s| {
            if found.is_none() && is_mark(s) {
                if let Statement::Directive(d) = s {
                    found = Some(d.span);
                }
            }
        },
        &mut |_| {},
    );
    found
}

/// Whether `program` is a RapidR program (a `.rr` file): its constants
/// are built in.
pub fn is_rapidr_program(program: &Program) -> bool {
    mark(program).is_some()
}

/// The program with its built-in constants and `QBColor` (see the module);
/// a RapidQ program as it is. The mark itself is taken out: it is no code,
/// and a line of its own (after the file's last) would be a place a
/// breakpoint could move to.
pub fn lower(program: &Program) -> Program {
    let Some(at) = mark(program) else { return program.clone() };
    let mut program = program.clone();
    program.statements.retain(|s| !is_mark(s));
    let mut own = HashSet::new();
    own_names(&program.statements, &mut own, false);
    let mut qbcolor = false;
    rewrite_block(&mut program.statements, &own, &mut qbcolor);
    if qbcolor && !own.contains("qbcolor") {
        // (on the program's first line of code, as the include's are on
        // its `$INCLUDE` line: never on a line after the file's last)
        let first = program.statements.iter().filter_map(code_start).filter(|s| *s < at.start).min();
        let mut init = qbcolor_statements(first.map_or(at, |s| TextSpan::new(s, s)));
        init.append(&mut program.statements);
        program.statements = init;
    }
    program
}

/// [`lower`] for what the debugger compiles against a running RapidR
/// program (a Watch, the Immediate window, a data tip): `statements` have no
/// mark and no program around them, and `known(name)` says whether the
/// running program has `name` as its own at that place (a local of the
/// frame, a global, a routine: a DIM mbYes in it, so `?mbYes` shows that).
/// `QBColor`, when the snippet uses it and the program has no such array
/// yet, is declared and filled at the snippet's start.
pub fn lower_snippet(statements: &[Statement], known: &dyn Fn(&str) -> bool) -> Vec<Statement> {
    let mut statements = statements.to_vec();
    let mut own = HashSet::new();
    own_names(&statements, &mut own, false);
    own.extend(constants().map(|(n, _)| key(n)).filter(|k| known(k)));
    if known(QBCOLOR) {
        own.insert(key(QBCOLOR));
    }
    let mut qbcolor = false;
    rewrite_block(&mut statements, &own, &mut qbcolor);
    if qbcolor && !own.contains("qbcolor") {
        let mut init = qbcolor_statements(TextSpan::default());
        init.append(&mut statements);
        statements = init;
    }
    statements
}

/// Where a statement of the main program's code starts: `None` for what
/// runs no code there (directives, comments, routines, TYPEs, DECLAREs).
fn code_start(s: &Statement) -> Option<usize> {
    let span = match s {
        Statement::Directive(_) | Statement::Comment(_) | Statement::Subroutine(_) | Statement::Function(_) | Statement::Type(_) | Statement::Declare(_) | Statement::RustBlock(_) => return None,
        Statement::Assignment(a) => a.span,
        Statement::Bind(b) => b.span,
        Statement::Call(c) => c.span,
        Statement::Close(c) => c.span,
        Statement::Const(c) => c.span,
        Statement::Create(c) => c.span,
        Statement::Dim(d) => d.span,
        Statement::DoLoop(d) => d.span,
        Statement::Exit(e) => e.span,
        Statement::For(f) => f.span,
        Statement::If(i) => i.span,
        Statement::Import(i) => i.span,
        Statement::Input(i) => i.span,
        Statement::Label(l) => l.span,
        Statement::Goto(j) | Statement::Gosub(j) => j.span,
        Statement::Line(l) => l.span,
        Statement::Open(o) => o.span,
        Statement::Print(p) => p.span,
        Statement::PrintHash(p) => p.span,
        Statement::Return(r) => r.span,
        Statement::Seek(s) => s.span,
        Statement::SelectCase(s) => s.span,
        Statement::While(w) => w.span,
        Statement::With(w) => w.span,
        Statement::WriteHash(w) => w.span,
    };
    Some(span.start)
}

/// `DIM QBColor(0 TO 15) AS INTEGER` and its 16 values, as the include
/// gives them (rapidr_preprocessor's RAPIDQ.INC text), at `span`.
pub fn qbcolor_statements(span: TextSpan) -> Vec<Statement> {
    let int = |n: i64| Expression::Literal(Literal { span, value: LiteralValue::Integer(n) });
    let name = || Expression::Identifier(Identifier { span, name: QBCOLOR.to_string() });
    let mut out = vec![Statement::Dim(DimStatement {
        span,
        declarators: vec![VariableDeclarator { span, name: QBCOLOR.to_string(), dimensions: vec![ArrayDimension::Range { start: int(0), end: int(15) }] }],
        type_name: "INTEGER".to_string(),
        fixed_len: None,
        is_static: false,
        is_redim: false,
    })];
    for (i, v) in rapidr_preprocessor::RAPIDQ_INC_QBCOLOR.iter().enumerate() {
        let target = Expression::FunctionCall(FunctionCallExpression { span, callee: Box::new(name()), args: vec![int(i as i64)] });
        out.push(Statement::Assignment(AssignmentStatement { span, target, value: int(*v) }));
    }
    out
}

/// The names `stmts` makes its own: what it declares and the bare names
/// it stores into — in nested blocks too, not in its SUBs, FUNCTIONs and
/// TYPEs (only their names). In a CREATE body (`in_create`) a bare name
/// set is a property, not a variable.
fn own_names(stmts: &[Statement], out: &mut HashSet<String>, in_create: bool) {
    let stored = |e: &Expression, out: &mut HashSet<String>| {
        if let Expression::Identifier(id) = e {
            out.insert(key(&id.name));
        }
    };
    for s in stmts {
        match s {
            Statement::Dim(d) => out.extend(d.declarators.iter().map(|v| key(&v.name))),
            Statement::Const(c) => {
                out.insert(key(&c.name));
            }
            Statement::Create(c) => {
                out.insert(key(&c.name));
                own_names(&c.body, out, true);
            }
            Statement::Declare(d) => {
                out.insert(key(&d.name));
            }
            Statement::Type(t) => {
                out.insert(key(&t.name));
            }
            Statement::Subroutine(r) => {
                out.insert(key(&r.name));
            }
            Statement::Function(f) => {
                out.insert(key(&f.name));
            }
            Statement::Assignment(a) if !in_create => stored(&a.target, out),
            Statement::Input(i) => stored(&i.target, out),
            Statement::For(f) => {
                out.insert(key(&f.variable));
                own_names(&f.body, out, in_create);
            }
            // (`INC x` / `DEC x`: a store)
            Statement::Call(c) if matches!(&c.callee, Expression::Identifier(i) if i.name.eq_ignore_ascii_case("INC") || i.name.eq_ignore_ascii_case("DEC")) => {
                if let Some(first) = c.args.first() {
                    stored(first, out);
                }
            }
            Statement::If(i) => {
                own_names(&i.then_body, out, in_create);
                for b in &i.elseif_branches {
                    own_names(&b.body, out, in_create);
                }
                own_names(&i.else_body, out, in_create);
            }
            Statement::While(w) => own_names(&w.body, out, in_create),
            Statement::DoLoop(d) => own_names(&d.body, out, in_create),
            Statement::With(w) => own_names(&w.body, out, in_create),
            Statement::SelectCase(c) => {
                for case in &c.cases {
                    own_names(&case.body, out, in_create);
                }
                own_names(&c.case_else, out, in_create);
            }
            _ => {}
        }
    }
}

/// `own` and a routine's own names: its parameters, what its body
/// declares and stores into (`more`: a TYPE's fields).
fn routine_scope(own: &HashSet<String>, params: &[Parameter], body: &[Statement], more: &HashSet<String>) -> HashSet<String> {
    let mut scope = own.clone();
    scope.extend(more.iter().cloned());
    scope.extend(params.iter().map(|p| key(&p.name)));
    own_names(body, &mut scope, false);
    scope
}

fn rewrite_block(stmts: &mut [Statement], own: &HashSet<String>, qbcolor: &mut bool) {
    rewrite_block_in(stmts, own, &HashSet::new(), qbcolor);
}

/// Rewrites `stmts` where `own` are the program's names; `fields`: the
/// names of the TYPE whose members these are (none elsewhere).
fn rewrite_block_in(stmts: &mut [Statement], own: &HashSet<String>, fields: &HashSet<String>, qbcolor: &mut bool) {
    for s in stmts {
        match s {
            Statement::Subroutine(r) => {
                let scope = routine_scope(own, &r.params, &r.body, fields);
                rewrite_block_in(&mut r.body, &scope, fields, qbcolor);
            }
            Statement::Function(f) => {
                let scope = routine_scope(own, &f.params, &f.body, fields);
                rewrite_block_in(&mut f.body, &scope, fields, qbcolor);
            }
            Statement::Type(t) => {
                let names: HashSet<String> = t.fields.iter().map(|f| key(&f.name)).collect();
                for f in &mut t.fields {
                    for e in f.array_size.iter_mut().chain(f.array_lower.iter_mut()) {
                        subst(e, own, qbcolor);
                    }
                    for (a, b) in &mut f.more_dims {
                        subst(a, own, qbcolor);
                        subst(b, own, qbcolor);
                    }
                }
                rewrite_block_in(&mut t.methods, own, &names, qbcolor);
                let scope = routine_scope(own, &[], &t.constructor, &names);
                rewrite_block_in(&mut t.constructor, &scope, &names, qbcolor);
                for e in &mut t.events {
                    let scope = routine_scope(own, &e.params, &e.body, &names);
                    rewrite_block_in(&mut e.body, &scope, &names, qbcolor);
                }
            }
            Statement::Assignment(a) => {
                stored(&mut a.target, own, qbcolor);
                subst(&mut a.value, own, qbcolor);
            }
            Statement::Input(i) => {
                if let Some(p) = &mut i.prompt {
                    subst(p, own, qbcolor);
                }
                stored(&mut i.target, own, qbcolor);
            }
            Statement::Call(c) => {
                named(&mut c.callee, own, qbcolor);
                for a in &mut c.args {
                    subst(a, own, qbcolor);
                }
            }
            // (`Obj.OnClick = Handler`: the handler is a SUB's name)
            Statement::Bind(b) => {
                stored(&mut b.target, own, qbcolor);
                named(&mut b.handler, own, qbcolor);
            }
            Statement::With(w) => {
                named(&mut w.object, own, qbcolor);
                rewrite_block_in(&mut w.body, own, fields, qbcolor);
            }
            _ => {
                let (exprs, bodies) = statement_parts_mut(s, true);
                for e in exprs {
                    subst(e, own, qbcolor);
                }
                for body in bodies {
                    rewrite_block_in(body, own, fields, qbcolor);
                }
            }
        }
    }
}

/// An expression read for its value: a built-in constant there becomes
/// its number (unless `own` has the name); `QBColor` used is noted.
fn subst(e: &mut Expression, own: &HashSet<String>, qbcolor: &mut bool) {
    match e {
        Expression::Identifier(id) => {
            let k = key(&id.name);
            if own.contains(&k) {
                return;
            }
            if k == "qbcolor" {
                *qbcolor = true;
            } else if let Some(&value) = table().get(&k) {
                *e = Expression::Literal(Literal { span: id.span, value: LiteralValue::Integer(value) });
            }
        }
        Expression::Literal(_) => {}
        Expression::Binary(b) => {
            subst(&mut b.left, own, qbcolor);
            subst(&mut b.right, own, qbcolor);
        }
        Expression::Unary(u) => subst(&mut u.operand, own, qbcolor),
        Expression::FunctionCall(c) => {
            named(&mut c.callee, own, qbcolor);
            for a in &mut c.args {
                subst(a, own, qbcolor);
            }
        }
        Expression::ArrayAccess(a) => {
            named(&mut a.array, own, qbcolor);
            for i in &mut a.indices {
                subst(i, own, qbcolor);
            }
        }
        Expression::MemberAccess(m) => named(&mut m.object, own, qbcolor),
        Expression::MethodCall(m) => {
            named(&mut m.object, own, qbcolor);
            for a in &mut m.args {
                subst(a, own, qbcolor);
            }
        }
    }
}

/// An expression in a routine's, an array's or an object's place: a bare
/// name there is never a constant (`QBColor(12)` uses the array).
fn named(e: &mut Expression, own: &HashSet<String>, qbcolor: &mut bool) {
    match e {
        Expression::Identifier(id) => {
            if key(&id.name) == "qbcolor" && !own.contains("qbcolor") {
                *qbcolor = true;
            }
        }
        _ => subst(e, own, qbcolor),
    }
}

/// A place stored into: a bare name is a variable (the program's own: it
/// is in `own`), anything else is read for its parts (`a(mbYes) = 1`).
fn stored(e: &mut Expression, own: &HashSet<String>, qbcolor: &mut bool) {
    if !matches!(e, Expression::Identifier(_)) {
        subst(e, own, qbcolor);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mark_stmt() -> Statement {
        Statement::Directive(DirectiveStatement { span: TextSpan::new(90, 104), name: "$OPTION".into(), value: Some(" RAPIDR".into()) })
    }
    fn id(name: &str) -> Expression {
        Expression::Identifier(Identifier { span: TextSpan::new(1, 2), name: name.into() })
    }
    fn print(e: Expression) -> Statement {
        Statement::Print(PrintStatement { span: TextSpan::default(), items: vec![e], zones: vec![false], append_newline: true })
    }
    fn printed(s: &Statement) -> &Expression {
        match s {
            Statement::Print(p) => &p.items[0],
            other => panic!("{other:?}"),
        }
    }
    fn int(e: &Expression) -> Option<i64> {
        match e {
            Expression::Literal(Literal { value: LiteralValue::Integer(n), .. }) => Some(*n),
            _ => None,
        }
    }

    #[test]
    fn only_a_rapidr_program_has_them() {
        let bas = Program { span: TextSpan::default(), statements: vec![print(id("mbYes"))] };
        assert_eq!(lower(&bas), bas);
        let rr = Program { span: TextSpan::default(), statements: vec![print(id("mbYes")), print(id("crHandPoint")), print(id("MB_OK")), print(id("True")), mark_stmt()] };
        let out = lower(&rr);
        assert_eq!(int(printed(&out.statements[0])), Some(1));
        assert_eq!(int(printed(&out.statements[1])), Some(-21));
        assert_eq!(int(printed(&out.statements[2])), Some(0));
        // (True stays RapidR's)
        assert!(matches!(printed(&out.statements[3]), Expression::Identifier(_)));
        assert!(is_rapidr_program(&rr) && !is_rapidr_program(&bas));
    }

    #[test]
    fn qbcolor_is_declared_when_used() {
        let call = Expression::FunctionCall(FunctionCallExpression { span: TextSpan::default(), callee: Box::new(id("qbcolor")), args: vec![Expression::Literal(Literal { span: TextSpan::default(), value: LiteralValue::Integer(12) })] });
        let rr = Program { span: TextSpan::default(), statements: vec![print(call), mark_stmt()] };
        let out = lower(&rr);
        assert!(matches!(&out.statements[0], Statement::Dim(d) if d.declarators[0].name == "QBColor" && d.type_name == "INTEGER"));
        // (the 17 statements and the PRINT; the mark is gone: no code)
        assert_eq!(out.statements.len(), 17 + 1);
        assert!(!out.statements.iter().any(is_mark));
        // (on the program's first line of code, never after its last)
        assert!(matches!(&out.statements[1], Statement::Assignment(a) if a.span == TextSpan::new(0, 0)));
        // (not used: nothing added)
        let rr = Program { span: TextSpan::default(), statements: vec![print(id("clRed")), mark_stmt()] };
        assert_eq!(lower(&rr).statements.len(), 1);
    }

    #[test]
    fn what_the_debugger_evaluates_has_them_too() {
        // (no mark, no program: the snippet alone; `known`: what the
        // running program has as its own)
        let known = |n: &str| n.eq_ignore_ascii_case("mbNo");
        let out = lower_snippet(&[print(id("mbYes")), print(id("mbNo")), print(id("True"))], &known);
        assert_eq!(int(printed(&out[0])), Some(1));
        assert!(matches!(printed(&out[1]), Expression::Identifier(_)));
        assert!(matches!(printed(&out[2]), Expression::Identifier(_)));
        // (a snippet's own store: its own name)
        let set = Statement::Assignment(AssignmentStatement { span: TextSpan::default(), target: id("mbYes"), value: Expression::Literal(Literal { span: TextSpan::default(), value: LiteralValue::Integer(5) }) });
        let out = lower_snippet(&[set, print(id("mbYes"))], &|_| false);
        assert!(matches!(printed(&out[1]), Expression::Identifier(_)));
        // (QBColor: declared and filled in front when the program has none)
        let call = Expression::FunctionCall(FunctionCallExpression { span: TextSpan::default(), callee: Box::new(id("QBColor")), args: vec![Expression::Literal(Literal { span: TextSpan::default(), value: LiteralValue::Integer(12) })] });
        assert_eq!(lower_snippet(&[print(call.clone())], &|_| false).len(), 17 + 1);
        assert_eq!(lower_snippet(&[print(call)], &|n| n.eq_ignore_ascii_case("QBColor")).len(), 1);
    }

    #[test]
    fn the_programs_own_names_win() {
        let dim = |name: &str| Statement::Dim(DimStatement { span: TextSpan::default(), declarators: vec![VariableDeclarator { span: TextSpan::default(), name: name.into(), dimensions: Vec::new() }], type_name: "INTEGER".into(), fixed_len: None, is_static: false, is_redim: false });
        let sub = Statement::Subroutine(SubroutineStatement {
            span: TextSpan::default(),
            name: "Show".into(),
            params: vec![Parameter { span: TextSpan::default(), name: "clRed".into(), type_name: "INTEGER".into(), by_ref: false, is_array: false }],
            body: vec![dim("clBlue"), print(id("clRed")), print(id("clBlue")), print(id("mbYes")), print(id("clGreen"))],
        });
        let rr = Program { span: TextSpan::default(), statements: vec![dim("mbYes"), print(id("mbYes")), print(id("clBlue")), print(id("clRed")), sub, mark_stmt()] };
        let out = lower(&rr);
        // (main: its own mbYes; the SUB's clBlue and parameter are the SUB's)
        assert!(matches!(printed(&out.statements[1]), Expression::Identifier(_)));
        assert_eq!(int(printed(&out.statements[2])), Some(0xFF0000));
        assert_eq!(int(printed(&out.statements[3])), Some(0x0000FF));
        let Statement::Subroutine(s) = &out.statements[4] else { panic!() };
        assert!(matches!(printed(&s.body[1]), Expression::Identifier(_)));
        assert!(matches!(printed(&s.body[2]), Expression::Identifier(_)));
        // (main's DIM mbYes is the program's everywhere)
        assert!(matches!(printed(&s.body[3]), Expression::Identifier(_)));
        assert_eq!(int(printed(&s.body[4])), Some(0x00FF00));
    }
}

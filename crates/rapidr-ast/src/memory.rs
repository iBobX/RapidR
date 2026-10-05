//! RapidQ's memory functions for both backends (`rapidr_value::memory` runs
//! them, memory-safe):
//!
//! * `VARPTR(a(i))` / `VARPTR(a)` of an array → `__varptr_elem(a, "TYPE", i…)`
//!   (a live view of the array's elements);
//! * `VARPTR(x)` / `UDTPTR(x)` of anything else → `__varptr_var(key, x,
//!   "TYPE")`: a TYPE instance is a live view; a plain variable gets a
//!   *mirror* of its bytes. Around every statement that reads or writes
//!   memory (or calls a SUB / FUNCTION, which may), the mirrors of the
//!   variables in scope are refreshed first (`__mem_refresh`) and copied
//!   back after (`x = __mem_sync(key, x, "TYPE")`), so `MEMCPY(VARPTR(i),
//!   VARPTR(j), 4)` changes `i`;
//! * `SIZEOF(INTEGER)`, `SIZEOF(TMyType)`, `SIZEOF(x)` of a declared scalar
//!   → the size as RapidQ stores it; of anything else → `__sizeof(x, "TYPE")`
//!   at run time (a STRING: its length; a TYPE: its fields, packed);
//! * `RTLMOVEMEMORY(dest, src, n)` (variables passed by reference) →
//!   `MEMCPY(VARPTR(dest), VARPTR(src), n)`.
//!
//! A program without any of these is left untouched.

use std::collections::{HashMap, HashSet};

use crate::*;
use rapidr_diagnostics::TextSpan;

/// Builtins that read or write memory: statements with them refresh and
/// copy back the mirrors in scope.
const MEMORY_CALLS: &[&str] = &["memcpy", "memset", "memcmp", "__cstring"];

fn key(name: &str) -> String {
    crate::strip_type_suffix(&name.to_ascii_lowercase()).to_string()
}

fn ident(span: TextSpan, name: &str) -> Expression {
    Expression::Identifier(Identifier { span, name: name.into() })
}

fn text(span: TextSpan, s: &str) -> Expression {
    Expression::Literal(Literal { span, value: LiteralValue::String(s.into()) })
}

fn int(span: TextSpan, n: i64) -> Expression {
    Expression::Literal(Literal { span, value: LiteralValue::Integer(n) })
}

fn call(span: TextSpan, name: &str, args: Vec<Expression>) -> Expression {
    Expression::FunctionCall(FunctionCallExpression { span, callee: Box::new(ident(span, name)), args })
}

fn callee_name(e: &Expression) -> Option<String> {
    match e {
        Expression::Identifier(id) => Some(id.name.to_ascii_lowercase()),
        _ => None,
    }
}

/// What a name is declared as in a scope: its type (`INTEGER`,
/// `STRING*8`, a TYPE name) and whether it's an array.
#[derive(Clone, Debug)]
struct Decl {
    type_name: String,
    is_array: bool,
}

type Decls = HashMap<String, Decl>;

fn type_of(d: &DimStatement) -> String {
    match d.fixed_len {
        Some(n) => format!("STRING*{n}"),
        None => d.type_name.clone(),
    }
}

/// The DIMs of `stmts` (not inside SUB / FUNCTION definitions).
fn declare(stmts: &[Statement], into: &mut Decls) {
    walk(
        stmts,
        &mut |s| {
            if let Statement::Dim(d) = s {
                for v in &d.declarators {
                    into.insert(key(&v.name), Decl { type_name: type_of(d), is_array: !v.dimensions.is_empty() });
                }
            }
        },
        &mut |_| {},
    );
}

/// The TYPEs' fields, for `SIZEOF(TMyType)` at compile time.
struct Types(HashMap<String, Vec<TypeField>>);

impl Types {
    /// The size of `type_name` as RapidQ stores it, when known at compile
    /// time (fixed-size fields, array fields with constant bounds).
    fn size(&self, type_name: &str, depth: usize) -> Option<i64> {
        let t = type_name.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_ascii_uppercase();
        if let Some(n) = t.strip_prefix("STRING*") {
            return n.parse().ok();
        }
        Some(match t.as_str() {
            "BYTE" => 1,
            "WORD" | "SHORT" => 2,
            "INTEGER" | "LONG" | "DWORD" | "SINGLE" | "STRING" => 4,
            "DOUBLE" => 8,
            // (RapidQ's data types: RC.EXE's SIZEOF — QNOTIFYICONDATA's six
            // numbers, its tip not counted)
            "QRECT" | "RRECT" => 16,
            "QNOTIFYICONDATA" | "RNOTIFYICONDATA" => 24,
            _ => {
                let fields = self.0.get(&t)?;
                if depth > 16 {
                    return None;
                }
                let mut total = 0;
                for f in fields {
                    let ft = match f.fixed_len {
                        Some(n) => format!("STRING*{n}"),
                        None => f.type_name.clone(),
                    };
                    let one = self.size(&ft, depth + 1).unwrap_or(4);
                    let count = match (&f.array_lower, &f.array_size) {
                        (_, None) => 1,
                        (lower, Some(upper)) => {
                            let lit = |e: &Expression| match e {
                                Expression::Literal(Literal { value: LiteralValue::Integer(n), .. }) => Some(*n),
                                _ => None,
                            };
                            let lo = match lower {
                                Some(l) => lit(l)?,
                                None => 0,
                            };
                            let mut n = lit(upper)? - lo + 1;
                            for (l, u) in &f.more_dims {
                                n *= lit(u)? - lit(l)? + 1;
                            }
                            n
                        }
                    };
                    total += one * count;
                }
                total
            }
        })
    }

    fn is_type(&self, name: &str) -> bool {
        self.0.contains_key(&name.to_ascii_uppercase())
    }
}

/// A variable whose VARPTR was taken: its mirror's key, the expression
/// that reads / assigns it, and its type.
#[derive(Clone)]
struct Mirror {
    key: String,
    target: Expression,
    type_name: String,
}

struct Pass<'a> {
    types: &'a Types,
    globals: &'a Decls,
    locals: Decls,
    /// "" for the main program, else the routine's name (lowercase).
    routine: String,
    routines: &'a HashSet<String>,
    /// Functions of DLLs (DECLARE … LIB): a call may write memory.
    dlls: &'a HashSet<String>,
    /// Mirrors of this scope (its locals) and of the globals, by key.
    mirrors: Vec<Mirror>,
}

impl Pass<'_> {
    fn decl(&self, name: &str) -> Option<&Decl> {
        let k = key(name);
        self.locals.get(&k).or_else(|| self.globals.get(&k))
    }

    fn is_local(&self, name: &str) -> bool {
        self.locals.contains_key(&key(name))
    }

    /// A name's declared type, or its suffix's, or VARIANT.
    fn type_name(&self, name: &str) -> String {
        match self.decl(name) {
            Some(d) => d.type_name.clone(),
            None => crate::suffix_type(name).unwrap_or("VARIANT").to_string(),
        }
    }

    fn is_array(&self, name: &str) -> bool {
        self.decl(name).is_some_and(|d| d.is_array)
    }

    fn mirror_key(&self, name: &str) -> String {
        if self.is_local(name) && !self.routine.is_empty() {
            format!("{}:{}", self.routine, key(name))
        } else {
            format!(":{}", key(name))
        }
    }

    /// `VARPTR(arg)` (or `UDTPTR`).
    fn varptr(&mut self, span: TextSpan, arg: Expression) -> Expression {
        match &arg {
            Expression::Identifier(id) if self.is_array(&id.name) => {
                let t = self.type_name(&id.name);
                call(span, "__varptr_elem", vec![arg.clone(), text(span, &t)])
            }
            Expression::FunctionCall(FunctionCallExpression { callee, args, .. })
            | Expression::ArrayAccess(ArrayAccessExpression { array: callee, indices: args, .. })
                if matches!(callee.as_ref(), Expression::Identifier(id) if self.is_array(&id.name)) =>
            {
                let Expression::Identifier(id) = callee.as_ref() else { unreachable!() };
                let t = self.type_name(&id.name);
                let mut a = vec![ident(span, &id.name), text(span, &t)];
                a.extend(args.iter().cloned());
                call(span, "__varptr_elem", a)
            }
            Expression::Identifier(id) => {
                let (k, t) = (self.mirror_key(&id.name), self.type_name(&id.name));
                self.add_mirror(Mirror { key: k.clone(), target: arg.clone(), type_name: t.clone() });
                call(span, "__varptr_var", vec![text(span, &k), arg, text(span, &t)])
            }
            // `VARPTR(r.Field)` (RapidQ refuses it; RapidR mirrors the field).
            Expression::MemberAccess(_) => {
                let k = format!("{}:{}", self.routine, member_path(&arg));
                self.add_mirror(Mirror { key: k.clone(), target: arg.clone(), type_name: "VARIANT".into() });
                call(span, "__varptr_var", vec![text(span, &k), arg, text(span, "VARIANT")])
            }
            _ => call(span, "__varptr_var", vec![text(span, ""), arg, text(span, "VARIANT")]),
        }
    }

    fn add_mirror(&mut self, m: Mirror) {
        if !self.mirrors.iter().any(|x| x.key == m.key) {
            self.mirrors.push(m);
        }
    }

    /// `SIZEOF(arg)`.
    fn sizeof(&self, span: TextSpan, arg: Expression) -> Expression {
        let named = match &arg {
            // `SIZEOF(INTEGER)`: the parser gives a type keyword as its name.
            Expression::Literal(Literal { value: LiteralValue::String(s), .. }) => Some(s.clone()),
            // `SIZEOF(SHORT)`, `SIZEOF(TMyType)`: a type's name (not a variable).
            Expression::Identifier(id) if self.decl(&id.name).is_none() && (self.types.is_type(&id.name) || is_builtin_type(&id.name)) => Some(id.name.clone()),
            _ => None,
        };
        if let Some(t) = named {
            return match self.types.size(&t, 0) {
                Some(n) => int(span, n),
                None => call(span, "__sizeof_type", vec![text(span, &t)]),
            };
        }
        if let Expression::Identifier(id) = &arg {
            let t = self.type_name(&id.name);
            let fixed = !self.is_array(&id.name) && !t.eq_ignore_ascii_case("STRING") && !t.eq_ignore_ascii_case("VARIANT");
            if fixed {
                if let Some(n) = self.types.size(&t, 0) {
                    return int(span, n);
                }
            }
            return call(span, "__sizeof", vec![arg.clone(), text(span, &t)]);
        }
        call(span, "__sizeof", vec![arg, text(span, "VARIANT")])
    }

    /// Rewrites the memory calls inside `e`.
    fn expr(&mut self, e: &mut Expression) {
        // (`walk_expressions_mut` has rewritten the children already.)
        let Expression::FunctionCall(fc) = e else { return };
        let Some(name) = callee_name(&fc.callee) else { return };
        let span = fc.span;
        // `VARPTR$(addr)`: the text at an address (not VARPTR).
        if name == "varptr$" && fc.args.len() == 1 {
            let arg = fc.args.remove(0);
            *e = call(span, "__cstring", vec![arg]);
            return;
        }
        match key(&name).as_str() {
            "varptr" | "udtptr" if fc.args.len() == 1 && !self.routines.contains(&key(&name)) => {
                let arg = fc.args.remove(0);
                *e = self.varptr(span, arg);
            }
            "sizeof" if fc.args.len() == 1 && !self.routines.contains("sizeof") => {
                let arg = fc.args.remove(0);
                *e = self.sizeof(span, arg);
            }
            _ => {}
        }
    }

    /// Whether `s` (a simple statement, or a compound statement's own
    /// expressions) uses memory or calls a routine.
    fn touches(&self, s: &Statement) -> bool {
        let mut found = false;
        let check = |e: &Expression, found: &mut bool| {
            walk_expression(e, &mut |x| {
                if let Expression::FunctionCall(fc) = x {
                    if let Some(n) = callee_name(&fc.callee) {
                        let n = key(&n);
                        *found |= MEMORY_CALLS.contains(&n.as_str()) || self.routines.contains(&n) || self.dlls.contains(&n);
                    }
                }
            });
        };
        match s {
            Statement::Call(c) => {
                if let Some(n) = callee_name(&c.callee) {
                    let n = key(&n);
                    found |= MEMORY_CALLS.contains(&n.as_str()) || self.routines.contains(&n) || self.dlls.contains(&n);
                }
                for a in &c.args {
                    check(a, &mut found);
                }
            }
            Statement::Assignment(a) => {
                check(&a.target, &mut found);
                check(&a.value, &mut found);
            }
            Statement::Print(_) | Statement::Return(_) => {
                walk(std::slice::from_ref(s), &mut |_| {}, &mut |e| check(e, &mut found));
            }
            Statement::If(i) => check(&i.condition, &mut found),
            Statement::While(w) => check(&w.condition, &mut found),
            Statement::SelectCase(c) => check(&c.expression, &mut found),
            Statement::For(f) => {
                check(&f.start, &mut found);
                check(&f.end, &mut found);
            }
            _ => {}
        }
        found
    }

    /// Rewrites a block: memory calls, then the refresh / copy-back around
    /// the statements that use memory.
    fn block(&mut self, stmts: &mut Vec<Statement>) {
        self.address_refs(stmts);
        // Expressions everywhere (nested blocks included).
        walk_expressions_mut(stmts, true, &mut |e| self.expr(e));
        // `RTLMOVEMEMORY dest, src, n` → `MEMCPY VARPTR(dest), VARPTR(src), n`.
        walk_statements_mut(stmts, &mut |s| {
            if let Statement::Call(c) = s {
                if callee_name(&c.callee).is_some_and(|n| n == "rtlmovememory") && c.args.len() == 3 {
                    let span = c.span;
                    c.callee = ident(span, "MEMCPY");
                    for i in 0..2 {
                        let a = c.args[i].clone();
                        c.args[i] = self.varptr(span, a);
                    }
                }
            }
        });
    }

    /// `@x` passed to anything but the program's own SUB / FUNCTION (a DLL
    /// function, a builtin): the address of `x`, i.e. `VARPTR(x)`. (To the
    /// program's own routines `@x` passes `x` by reference; that stays.)
    fn address_refs(&self, stmts: &mut Vec<Statement>) {
        const KEEP: &str = "__keep_ref";
        let routines = self.routines;
        let keep = |args: &mut Vec<Expression>| {
            for a in args.iter_mut() {
                if let Expression::Unary(u) = a {
                    if u.operator == UnaryOperator::Ref {
                        let span = u.span;
                        *a = call(span, KEEP, vec![(*u.operand).clone()]);
                    }
                }
            }
        };
        walk_statements_mut(stmts, &mut |s| {
            if let Statement::Call(c) = s {
                if callee_name(&c.callee).is_some_and(|n| routines.contains(&key(&n))) {
                    keep(&mut c.args);
                }
            }
        });
        walk_expressions_mut(stmts, true, &mut |e| {
            if let Expression::FunctionCall(fc) = e {
                if callee_name(&fc.callee).is_some_and(|n| routines.contains(&key(&n))) {
                    keep(&mut fc.args);
                }
            }
        });
        walk_expressions_mut(stmts, true, &mut |e| match e {
            Expression::Unary(u) if u.operator == UnaryOperator::Ref => {
                let span = u.span;
                *e = call(span, "VARPTR", vec![(*u.operand).clone()]);
            }
            Expression::FunctionCall(fc) if callee_name(&fc.callee).is_some_and(|n| n == KEEP) => {
                let span = fc.span;
                let operand = fc.args.remove(0);
                *e = Expression::Unary(UnaryExpression { span, operator: UnaryOperator::Ref, operand: Box::new(operand) });
            }
            _ => {}
        });
    }

    /// Adds the refresh / copy-back around the statements of every block.
    fn sync_blocks(&mut self, stmts: &mut Vec<Statement>) {
        // A global's mirror doesn't apply where a local has its name.
        let routine = self.routine.clone();
        let locals = self.locals.clone();
        self.mirrors.retain(|m| {
            !m.key.starts_with(':') || routine.is_empty() || !matches!(&m.target, Expression::Identifier(id) if locals.contains_key(&key(&id.name)))
        });
        if self.mirrors.is_empty() {
            return;
        }
        for_each_block_mut(stmts, &mut |block| {
            let mut out = Vec::with_capacity(block.len());
            for s in block.drain(..) {
                if !self.touches(&s) {
                    out.push(s);
                    continue;
                }
                let span = statement_span(&s);
                let compound = matches!(s, Statement::If(_) | Statement::While(_) | Statement::SelectCase(_) | Statement::For(_));
                for m in &self.mirrors {
                    out.push(Statement::Call(CallStatement {
                        span,
                        callee: ident(span, "__mem_refresh"),
                        args: vec![text(span, &m.key), m.target.clone(), text(span, &m.type_name)],
                    }));
                }
                out.push(s);
                if !compound {
                    for m in &self.mirrors {
                        out.push(Statement::Assignment(AssignmentStatement {
                            span,
                            target: m.target.clone(),
                            value: call(span, "__mem_sync", vec![text(span, &m.key), m.target.clone(), text(span, &m.type_name)]),
                        }));
                    }
                }
            }
            *block = out;
        });
    }
}

/// Where a statement is (for the statements added around it).
fn statement_span(s: &Statement) -> TextSpan {
    match s {
        Statement::Call(c) => c.span,
        Statement::Assignment(a) => a.span,
        Statement::If(i) => i.span,
        Statement::While(w) => w.span,
        Statement::For(f) => f.span,
        Statement::SelectCase(c) => c.span,
        Statement::Print(p) => p.span,
        Statement::Return(r) => r.span,
        _ => TextSpan::default(),
    }
}

fn is_builtin_type(name: &str) -> bool {
    matches!(
        name.to_ascii_uppercase().as_str(),
        "BYTE" | "WORD" | "SHORT" | "INTEGER" | "LONG" | "DWORD" | "SINGLE" | "DOUBLE" | "STRING" | "QRECT" | "QNOTIFYICONDATA"
    )
}

fn member_path(e: &Expression) -> String {
    match e {
        Expression::Identifier(id) => key(&id.name),
        Expression::MemberAccess(m) => format!("{}.{}", member_path(&m.object), key(&m.member)),
        _ => "?".into(),
    }
}

/// Whether `program` uses any of the memory functions.
fn uses_memory(program: &Program) -> bool {
    let found = std::cell::Cell::new(false);
    walk(
        &program.statements,
        &mut |s| {
            if let Statement::Call(c) = s {
                if callee_name(&c.callee).is_some_and(|n| n == "rtlmovememory") {
                    found.set(true);
                }
            }
        },
        &mut |e| match e {
            Expression::FunctionCall(fc) if callee_name(&fc.callee).is_some_and(|n| matches!(key(&n).as_str(), "varptr" | "udtptr" | "sizeof")) => found.set(true),
            Expression::Unary(u) if u.operator == UnaryOperator::Ref => found.set(true),
            _ => {}
        },
    );
    found.get()
}

/// A body of code with its own locals: a SUB / FUNCTION, a TYPE's method,
/// CONSTRUCTOR or EVENT. Calls `f(routine name, parameters, body)`.
fn for_each_routine(stmts: &mut [Statement], f: &mut dyn FnMut(&str, &[Parameter], &mut Vec<Statement>)) {
    for s in stmts {
        match s {
            Statement::Subroutine(r) => f(&key(&r.name), &r.params, &mut r.body),
            Statement::Function(x) => f(&key(&x.name), &x.params, &mut x.body),
            Statement::Type(t) => {
                let tn = key(&t.name);
                f(&format!("{tn}__constructor"), &[], &mut t.constructor);
                for m in &mut t.methods {
                    match m {
                        Statement::Subroutine(r) => f(&format!("{tn}__{}", key(&r.name)), &r.params, &mut r.body),
                        Statement::Function(x) => f(&format!("{tn}__{}", key(&x.name)), &x.params, &mut x.body),
                        _ => {}
                    }
                }
                for e in &mut t.events {
                    f(&format!("{tn}__{}", key(&e.name)), &e.params, &mut e.body);
                }
            }
            _ => {}
        }
    }
}

/// Lowers the memory functions of `program` (see the module docs).
pub fn lower(program: &Program) -> Program {
    if !uses_memory(program) {
        return program.clone();
    }
    let mut program = program.clone();
    let mut types = HashMap::new();
    let mut routines = HashSet::new();
    let mut dlls = HashSet::new();
    let mut main = Vec::new();
    for s in &program.statements {
        match s {
            Statement::Type(t) => {
                types.insert(t.name.to_ascii_uppercase(), t.fields.clone());
            }
            Statement::Declare(d) if d.lib.is_some() => {
                dlls.insert(key(&d.name));
                main.push(s.clone());
            }
            Statement::Subroutine(r) => {
                routines.insert(key(&r.name));
            }
            Statement::Function(f) => {
                routines.insert(key(&f.name));
            }
            other => main.push(other.clone()),
        }
    }
    let types = Types(types);
    let mut globals = Decls::new();
    declare(&main, &mut globals);
    let locals_of = |params: &[Parameter], body: &[Statement]| {
        let mut locals = Decls::new();
        for p in params {
            locals.insert(key(&p.name), Decl { type_name: p.type_name.clone(), is_array: p.is_array });
        }
        declare(body, &mut locals);
        locals
    };

    // 1. The memory calls everywhere; the mirrors each body made, and the
    //    globals' (which matter in every body).
    let mut global_mirrors: Vec<Mirror> = Vec::new();
    let mut local_mirrors: HashMap<String, Vec<Mirror>> = HashMap::new();
    let rewrite = |routine: &str, locals: Decls, body: &mut Vec<Statement>, global_mirrors: &mut Vec<Mirror>| {
        let mut pass = Pass { types: &types, globals: &globals, locals, routine: routine.to_string(), routines: &routines, dlls: &dlls, mirrors: Vec::new() };
        pass.block(body);
        let (global, local): (Vec<Mirror>, Vec<Mirror>) = pass.mirrors.into_iter().partition(|m| m.key.starts_with(':'));
        for m in global {
            if !global_mirrors.iter().any(|x| x.key == m.key) {
                global_mirrors.push(m);
            }
        }
        local
    };
    for_each_routine(&mut program.statements, &mut |routine, params, body| {
        let locals = locals_of(params, body);
        let m = rewrite(routine, locals, body, &mut global_mirrors);
        local_mirrors.insert(routine.to_string(), m);
    });
    for s in program.statements.iter_mut() {
        if !matches!(s, Statement::Subroutine(_) | Statement::Function(_) | Statement::Type(_)) {
            let mut one = vec![s.clone()];
            let _ = rewrite("", Decls::new(), &mut one, &mut global_mirrors);
            *s = one.remove(0);
        }
    }

    // 2. The refresh / copy-back around the statements that use memory.
    let sync = |mirrors: Vec<Mirror>, locals: Decls, routine: &str, body: &mut Vec<Statement>| {
        let mut pass = Pass { types: &types, globals: &globals, locals, routine: routine.into(), routines: &routines, dlls: &dlls, mirrors };
        pass.sync_blocks(body);
    };
    for_each_routine(&mut program.statements, &mut |routine, params, body| {
        let mut m = local_mirrors.remove(routine).unwrap_or_default();
        m.extend(global_mirrors.iter().cloned());
        let locals = locals_of(params, body);
        sync(m, locals, routine, body);
    });
    // The main program's statements (between the routines) in runs.
    let statements = std::mem::take(&mut program.statements);
    let mut out: Vec<Statement> = Vec::with_capacity(statements.len());
    let mut run: Vec<Statement> = Vec::new();
    let flush = |run: &mut Vec<Statement>, out: &mut Vec<Statement>| {
        if !run.is_empty() {
            let mut block = std::mem::take(run);
            sync(global_mirrors.clone(), Decls::new(), "", &mut block);
            out.extend(block);
        }
    };
    for s in statements {
        match s {
            Statement::Subroutine(_) | Statement::Function(_) | Statement::Type(_) => {
                flush(&mut run, &mut out);
                out.push(s);
            }
            other => run.push(other),
        }
    }
    flush(&mut run, &mut out);
    program.statements = out;
    program
}

//! Where an undeclared variable lives, as RapidQ's one-pass compiler decides
//! it (RC.EXE, docs/rapidq-ground-truth.md): a name the main program has
//! already used (above the SUB in the source) is that global inside a
//! SUB/FUNCTION; a name the SUB uses first is the SUB's own — kept between
//! calls, as a STATIC — and the main program's same name, used further down,
//! is another variable:
//!
//! ```text
//! k = 4                 ' main uses k first: global
//! SUB S4 : k = 7 : END SUB      ' sets the global k
//! SUB S5 : q = q + 1 : PRINT q : END SUB   ' S5's own q: 1, 2, 3 …
//! q = 100               ' main's q, not S5's
//! ```
//!
//! Each SUB's own names are renamed (`S5__q`), so both backends, which make
//! every undeclared name one global, keep them apart. Declared names (DIM,
//! CONST, …, anywhere at the top), parameters, locals, arrays, objects,
//! called names, builtins and names with a type suffix's twin are left
//! alone. Runs in [`crate::option_dim`], before its declarations.

use std::collections::HashSet;

use crate::*;

/// The undeclared names `stmts` stores into or reads (not the CREATE and
/// TYPE bodies': their bare names are properties), in order of first use.
fn uses(stmts: &[Statement], skip: &dyn Fn(&str) -> bool) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut not_vars: HashSet<String> = HashSet::new();
    let mut seen = Vec::new();
    let mut called = Vec::new();
    let mut counters = Vec::new();
    for s in stmts {
        if matches!(s, Statement::Type(_) | Statement::Create(_) | Statement::Subroutine(_) | Statement::Function(_)) {
            continue;
        }
        walk(
            std::slice::from_ref(s),
            &mut |x| match x {
                Statement::Call(c) => {
                    if let Expression::Identifier(i) = &c.callee {
                        called.push(key(&i.name));
                    }
                }
                Statement::For(f) => counters.push(f.variable.clone()),
                _ => {}
            },
            &mut |e| match e {
                Expression::MemberAccess(m) => {
                    if let Expression::Identifier(i) = m.object.as_ref() {
                        not_vars.insert(key(&i.name));
                    }
                }
                Expression::FunctionCall(c) => {
                    if let Expression::Identifier(i) = c.callee.as_ref() {
                        not_vars.insert(key(&i.name));
                    }
                }
                Expression::ArrayAccess(a) => {
                    if let Expression::Identifier(i) = a.array.as_ref() {
                        not_vars.insert(key(&i.name));
                    }
                }
                Expression::Identifier(i) => seen.push(i.name.clone()),
                _ => {}
            },
        );
    }
    not_vars.extend(called);
    seen.extend(counters);
    // (CREATE bodies below other blocks set properties too)
    let mut in_creates: HashSet<String> = HashSet::new();
    walk(
        stmts,
        &mut |s| {
            if let Statement::Create(c) = s {
                walk(&c.body, &mut |_| {}, &mut |e| {
                    if let Expression::Identifier(i) = e {
                        in_creates.insert(key(&i.name));
                    }
                });
            }
        },
        &mut |_| {},
    );
    for n in seen {
        let k = key(&n);
        if n.starts_with("__") || n.contains('.') || not_vars.contains(&k) || in_creates.contains(&k) || skip(&k) {
            continue;
        }
        if !out.iter().any(|o| key(o) == k) {
            out.push(n);
        }
    }
    out
}

/// A name without its type suffix, lowercase (`Q$` → `q`): both backends
/// key variables so.
fn key(name: &str) -> String {
    strip_type_suffix(name).to_ascii_lowercase()
}

/// Names a routine declares itself: its parameters, its name (a FUNCTION's
/// result), RESULT, DIM / STATIC / CONST inside it.
fn locals(name: &str, params: &[Parameter], body: &[Statement]) -> HashSet<String> {
    let mut l: HashSet<String> = params.iter().map(|p| key(&p.name)).collect();
    l.insert(key(name));
    l.insert("result".into());
    walk(
        body,
        &mut |s| match s {
            Statement::Dim(d) => l.extend(d.declarators.iter().map(|v| key(&v.name))),
            Statement::Const(c) => {
                l.insert(key(&c.name));
            }
            Statement::Label(lb) => {
                l.insert(key(&lb.name));
            }
            _ => {}
        },
        &mut |_| {},
    );
    l
}

/// `name` → `new` in a routine's body (its own expressions and FOR
/// counters, not CREATE bodies).
fn rename(body: &mut [Statement], name: &str, new_base: &str) {
    let k = key(name);
    let renamed = |n: &str| -> Option<String> {
        (key(n) == k).then(|| format!("{new_base}{}", &n[strip_type_suffix(n).len()..]))
    };
    fn visit(stmts: &mut [Statement], f: &dyn Fn(&str) -> Option<String>) {
        for s in stmts.iter_mut() {
            if matches!(s, Statement::Create(_) | Statement::Type(_)) {
                continue;
            }
            if let Statement::For(fs) = s {
                if let Some(n) = f(&fs.variable) {
                    fs.variable = n;
                }
            }
            let (exprs, bodies) = crate::statement_parts_mut(s, true);
            for e in exprs {
                crate::walk_expression_mut(e, &mut |e| {
                    if let Expression::Identifier(i) = e {
                        if let Some(n) = f(&i.name) {
                            i.name = n;
                        }
                    }
                });
            }
            for b in bodies {
                visit(b, f);
            }
        }
    }
    visit(body, &renamed);
}

/// See the module docs. `skip(key)`: names that aren't variables (builtins,
/// declared names, RapidR's constants, …).
pub fn lower(program: &Program, skip: &dyn Fn(&str) -> bool) -> Program {
    let mut program = program.clone();
    let mut known: HashSet<String> = HashSet::new();
    for s in program.statements.iter_mut() {
        let (name, params, body) = match s {
            Statement::Subroutine(r) => (r.name.clone(), r.params.clone(), &mut r.body),
            Statement::Function(f) => (f.name.clone(), f.params.clone(), &mut f.body),
            other => {
                known.extend(uses(std::slice::from_ref(other), skip).iter().map(|n| key(n)));
                continue;
            }
        };
        let own = locals(&name, &params, body);
        for n in uses(body, &|k| skip(k) || own.contains(k)) {
            if !known.contains(&key(&n)) {
                rename(body, &n, &format!("{}__{}", strip_type_suffix(&name), strip_type_suffix(&n)));
            }
        }
    }
    program
}

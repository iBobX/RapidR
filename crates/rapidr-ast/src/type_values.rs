//! A component type's name used as a value (`Parent = QFORM`): RapidQ's
//! most recently created object of that type. RapidQ's own examples rely
//! on it — a toolbar or main menu written in an include file sets `Parent =
//! QFORM` to land on the form the program created just before — and it's
//! the same rule as inside a TYPE … EXTENDS constructor, where the type's
//! own name is the object being constructed (the newest of its type).
//!
//! Such a name becomes `__lastoftype("RFORM")` (RapidQ's Q names
//! canonicalised; a TYPE extending a component keeps its own name), which
//! every runtime answers from `rapidr_value::rp_last_of_type` — "" while
//! none exists yet, as the bare name was before. Only names the program
//! doesn't declare (variable, constant, routine, parameter, component) and
//! only as a value: not `QFORM.Caption`, a call's name or an assignment's
//! target. Inside a TYPE's own code its name (and its TYPE ancestors')
//! stays the instance (`rapidr_ast::objects`).

use std::collections::{HashMap, HashSet};

use crate::*;

/// The builtin a component type's name used as a value becomes.
pub const LAST_OF_TYPE: &str = "__lastoftype";

fn key(name: &str) -> String {
    strip_type_suffix(name).to_ascii_lowercase()
}

/// Every name the program declares or assigns anywhere (lowercase).
fn declared_names(program: &Program) -> HashSet<String> {
    let mut names = HashSet::new();
    let add_params = |names: &mut HashSet<String>, params: &[Parameter]| names.extend(params.iter().map(|p| key(&p.name)));
    walk(
        &program.statements,
        &mut |s| match s {
            Statement::Dim(d) => names.extend(d.declarators.iter().map(|v| key(&v.name))),
            Statement::Const(c) => {
                names.insert(key(&c.name));
            }
            Statement::Create(c) => {
                names.insert(key(&c.name));
            }
            Statement::Subroutine(r) => {
                names.insert(key(&r.name));
                add_params(&mut names, &r.params);
            }
            Statement::Function(f) => {
                names.insert(key(&f.name));
                add_params(&mut names, &f.params);
            }
            Statement::Declare(d) => {
                names.insert(key(&d.name));
            }
            Statement::For(f) => {
                names.insert(key(&f.variable));
            }
            Statement::Assignment(AssignmentStatement { target, .. }) | Statement::Input(InputStatement { target, .. }) => {
                let base = match target {
                    Expression::ArrayAccess(a) => a.array.as_ref(),
                    Expression::FunctionCall(c) => c.callee.as_ref(),
                    other => other,
                };
                if let Expression::Identifier(id) = base {
                    names.insert(key(&id.name));
                }
            }
            _ => {}
        },
        &mut |_| {},
    );
    names
}

/// Rewrites the program's component type names used as values (see the
/// module docs).
pub fn lower(program: &Program) -> Program {
    let declared = declared_names(program);
    // User TYPEs (lowercase → (name as declared, the TYPE it extends)).
    let types: HashMap<String, (String, Option<String>)> = program
        .statements
        .iter()
        .filter_map(|s| match s {
            Statement::Type(t) => Some((key(&t.name), (t.name.clone(), t.extends.clone()))),
            _ => None,
        })
        .collect();
    // A TYPE's chain of TYPE ancestors (itself first) and whether it ends
    // at a component.
    let chain = |t: &str| {
        let mut names = Vec::new();
        let mut cur = Some(t.to_string());
        while let Some(name) = cur.take() {
            match types.get(&key(&name)) {
                Some((declared_name, extends)) if names.len() < 32 => {
                    names.push(key(declared_name));
                    cur = extends.clone();
                }
                Some(_) => return (names, false), // cyclic EXTENDS
                None => return (names, is_rapidq_object_type(&name)),
            }
        }
        (names, false)
    };
    // The type a bare name stands for, if it's one used as a value.
    let type_of = |name: &str| -> Option<String> {
        let k = key(name);
        if declared.contains(&k) || name.len() != strip_type_suffix(name).len() {
            return None;
        }
        if let Some((declared_name, _)) = types.get(&k) {
            return chain(declared_name).1.then(|| declared_name.clone());
        }
        is_rapidq_object_type(name).then(|| canonical_type_name(name).to_ascii_uppercase())
    };
    let mut program = program.clone();
    for s in &mut program.statements {
        if let Statement::Type(t) = s {
            let own: HashSet<String> = chain(&t.name).0.into_iter().collect();
            let type_of = |name: &str| if own.contains(&key(name)) { None } else { type_of(name) };
            for body in [&mut t.methods, &mut t.constructor].into_iter().chain(t.events.iter_mut().map(|e| &mut e.body)) {
                rewrite_statements(body, &type_of);
            }
        } else {
            rewrite_statements(std::slice::from_mut(s), &type_of);
        }
    }
    program
}

fn rewrite_statements(stmts: &mut [Statement], type_of: &dyn Fn(&str) -> Option<String>) {
    for s in stmts {
        match s {
            // (a call's name and an assignment's target aren't values)
            Statement::Call(c) => {
                rewrite(&mut c.callee, false, type_of);
                c.args.iter_mut().for_each(|a| rewrite(a, true, type_of));
            }
            Statement::Assignment(a) => {
                rewrite(&mut a.target, false, type_of);
                rewrite(&mut a.value, true, type_of);
            }
            _ => {
                let (exprs, bodies) = statement_parts_mut(s, true);
                exprs.into_iter().for_each(|e| rewrite(e, true, type_of));
                bodies.into_iter().for_each(|b| rewrite_statements(b, type_of));
            }
        }
    }
}

/// `value`: `e` is read as a value (not a member's object or a call's name).
fn rewrite(e: &mut Expression, value: bool, type_of: &dyn Fn(&str) -> Option<String>) {
    match e {
        Expression::Identifier(id) => {
            if let Some(t) = value.then(|| type_of(&id.name)).flatten() {
                let span = id.span;
                let callee = Box::new(Expression::Identifier(Identifier { span, name: LAST_OF_TYPE.into() }));
                let arg = Expression::Literal(Literal { span, value: LiteralValue::String(t) });
                *e = Expression::FunctionCall(FunctionCallExpression { span, callee, args: vec![arg] });
            }
        }
        Expression::ArrayAccess(a) => {
            rewrite(&mut a.array, false, type_of);
            a.indices.iter_mut().for_each(|i| rewrite(i, true, type_of));
        }
        Expression::Binary(b) => {
            rewrite(&mut b.left, true, type_of);
            rewrite(&mut b.right, true, type_of);
        }
        Expression::FunctionCall(c) => {
            rewrite(&mut c.callee, false, type_of);
            c.args.iter_mut().for_each(|a| rewrite(a, true, type_of));
        }
        Expression::MemberAccess(m) => rewrite(&mut m.object, false, type_of),
        Expression::MethodCall(m) => {
            rewrite(&mut m.object, false, type_of);
            m.args.iter_mut().for_each(|a| rewrite(a, true, type_of));
        }
        Expression::Unary(u) => rewrite(&mut u.operand, true, type_of),
        Expression::Literal(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rapidr_diagnostics::TextSpan;

    fn span() -> TextSpan {
        TextSpan::default()
    }
    fn ident(n: &str) -> Expression {
        Expression::Identifier(Identifier { span: span(), name: n.into() })
    }
    fn assign(target: Expression, value: Expression) -> Statement {
        Statement::Assignment(AssignmentStatement { span: span(), target, value })
    }
    fn last_of(e: &Expression) -> Option<String> {
        match e {
            Expression::FunctionCall(c) => match (c.callee.as_ref(), c.args.first()) {
                (Expression::Identifier(id), Some(Expression::Literal(Literal { value: LiteralValue::String(t), .. }))) if id.name == LAST_OF_TYPE => Some(t.clone()),
                _ => None,
            },
            _ => None,
        }
    }
    fn member(o: Expression, m: &str) -> Expression {
        Expression::MemberAccess(MemberAccessExpression { span: span(), object: Box::new(o), member: m.into() })
    }

    #[test]
    fn type_names_as_values_are_the_last_of_their_type() {
        let ty = Statement::Type(TypeStatement {
            span: span(),
            name: "QCoolForm".into(),
            extends: Some("RFORM".into()),
            object_base: true,
            fields: vec![],
            methods: vec![],
            constructor: vec![assign(member(ident("b"), "Parent"), ident("QCoolForm")), assign(ident("x"), ident("QFORM"))],
            events: vec![],
            template_params: vec![],
        });
        let program = Program {
            span: span(),
            statements: vec![
                ty,
                assign(member(ident("p"), "Parent"), ident("QFORM")),
                assign(ident("y"), ident("qcoolform")),
                assign(ident("z"), member(ident("QFORM"), "Caption")),
                assign(ident("w"), ident("QPanelX")),
                assign(ident("QButton"), Expression::Literal(Literal { span: span(), value: LiteralValue::Integer(1) })),
                assign(ident("v"), ident("QBUTTON")),
            ],
        };
        let out = lower(&program).statements;
        let value = |i: usize| match &out[i] {
            Statement::Assignment(a) => a.value.clone(),
            _ => unreachable!(),
        };
        assert_eq!(last_of(&value(1)).as_deref(), Some("RFORM"));
        assert_eq!(last_of(&value(2)).as_deref(), Some("QCoolForm"));
        assert_eq!(last_of(&value(3)), None, "a member's object stays");
        assert_eq!(value(4), ident("QPanelX"), "not a type");
        assert_eq!(value(6), ident("QBUTTON"), "a variable of that name");
        let Statement::Type(t) = &out[0] else { unreachable!() };
        let ctor = |i: usize| match &t.constructor[i] {
            Statement::Assignment(a) => a.value.clone(),
            _ => unreachable!(),
        };
        assert_eq!(ctor(0), ident("QCoolForm"), "inside its own TYPE: the instance");
        assert_eq!(last_of(&ctor(1)).as_deref(), Some("RFORM"));
    }
}

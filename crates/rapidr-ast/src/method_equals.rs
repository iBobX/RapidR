//! RapidQ's `Obj.Method = x`: a method "assigned" is called, with `= x` as
//! its argument (RC.EXE, probes 2026-10-08: `B.FillRect = 10, 10, 20, 20,
//! &HFF` fills, `L.AddItems = "a"` adds an empty line, `v.Bar = 3, 4` calls
//! the TYPE's SUB Bar with 3 and 4). With a comma after the value the parser
//! already makes it a call (any member: RapidQ refuses `L.Sorted = 1, 2`
//! with `Expected end-of-line but got ,`, which [`crate::rapidq_checks`]
//! gives); alone it's an assignment, told apart here by the object's type —
//! a component's method that isn't also a property (the language registry),
//! or a TYPE's SUB / FUNCTION that isn't one of its fields.

use std::collections::HashMap;

use crate::*;

/// `name → type` of every variable, component and parameter (lowercase
/// names; the type as written).
fn declared_types(program: &Program) -> HashMap<String, String> {
    let mut out = HashMap::new();
    walk(
        &program.statements,
        &mut |s| match s {
            Statement::Dim(d) => {
                for v in &d.declarators {
                    out.insert(strip_type_suffix(&v.name).to_ascii_lowercase(), d.type_name.clone());
                }
            }
            Statement::Create(c) => {
                out.insert(c.name.to_ascii_lowercase(), c.type_name.clone());
            }
            Statement::Subroutine(r) => {
                for p in &r.params {
                    out.entry(p.name.to_ascii_lowercase()).or_insert_with(|| p.type_name.clone());
                }
            }
            Statement::Function(f) => {
                for p in &f.params {
                    out.entry(p.name.to_ascii_lowercase()).or_insert_with(|| p.type_name.clone());
                }
            }
            _ => {}
        },
        &mut |_| {},
    );
    out
}

/// Whether `member` of an object of type `type_name` is a method taking
/// arguments and not a property or field (one taking none is RC.EXE's
/// `Expected end-of-line but got =`: [`crate::rapidq_checks`]).
pub fn is_method_only(program: &Program, type_name: &str, member: &str) -> bool {
    method_params(program, type_name, member).is_some_and(|n| n > 0)
}

/// How many parameters `member` takes when it's a method (of a TYPE or a
/// component) and not a property or field of `type_name`.
pub fn method_params(program: &Program, type_name: &str, member: &str) -> Option<usize> {
    let user = program.statements.iter().find_map(|s| match s {
        Statement::Type(t) if t.name.eq_ignore_ascii_case(type_name.trim()) => Some(t),
        _ => None,
    });
    if let Some(t) = user {
        let params = t.methods.iter().find_map(|m| match m {
            Statement::Subroutine(r) if r.name.eq_ignore_ascii_case(member) => Some(r.params.len()),
            Statement::Function(f) if f.name.eq_ignore_ascii_case(member) => Some(f.params.len()),
            _ => None,
        });
        return params.filter(|_| !t.fields.iter().any(|f| f.name.eq_ignore_ascii_case(member)));
    }
    let c = rapidr_lang::component(&canonical_type_name(type_name.trim()))?;
    c.method(member).filter(|_| c.property(member).is_none()).map(|m| m.params.len())
}

pub fn lower(program: &Program) -> Program {
    let types = declared_types(program);
    let mut out = program.clone();
    walk_statements_mut(&mut out.statements, &mut |s| {
        let Statement::Assignment(a) = s else { return };
        let Expression::MemberAccess(m) = &a.target else { return };
        let Expression::Identifier(root) = m.object.as_ref() else { return };
        let Some(t) = types.get(&strip_type_suffix(&root.name).to_ascii_lowercase()) else { return };
        if !is_method_only(program, t, &m.member) {
            return;
        }
        let value = a.value.clone();
        *s = Statement::Call(CallStatement { span: a.span, callee: a.target.clone(), args: vec![lone_equals(a.span, value)] });
    });
    out
}

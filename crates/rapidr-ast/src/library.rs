//! RapidQ's library objects that RapidR implements itself, and two
//! statements around them — one pass both backends run first.
//!
//! - **The libraries' TYPEs.** QCGI, QMIDI, QWAVE, QVIDEO, QCDAUDIO,
//!   QDOWNLOAD (RapidQ's include files: `qcgi.inc`, `QMidi.inc`, …) and
//!   RAPIDQ2.INC's COMPORT (its QCOMPORT: `$DEFINE QCOMPORT COMPORT`) are
//!   `TYPE … EXTENDS QOBJECT` written on Windows' MCI, WinINet-era sockets
//!   and kernel32's serial calls. A program that includes one gets
//!   RapidR's own object of that name instead (rapidr_value::objects::rqlib):
//!   the TYPE is left out here; the file's constants stay.
//! - **TYPEs no DIM makes** keep their fields but lose their methods,
//!   constructor and events: RapidQ's compiler never compiles them
//!   ([`crate::instantiated_types`]), so what they call needn't exist.
//! - **`Obj.Method = x`** is a call of the method ([`crate::method_equals`]).
//! - **`ENVIRON "name=text"`** (the statement) is `__environ_set(…)`;
//!   `ENVIRON$(name)` stays the function.
//! - **`CGI.Get(Name, Value)`** sets Value by reference: written
//!   `Value = CGI.__get(Name, Value)` before the statement, the call itself
//!   read as `CGI.__found` (whether that Get found it).

use std::collections::HashSet;

use crate::*;

/// The TYPE names of RapidQ's libraries RapidR implements (uppercase), and
/// RapidR's component for each.
pub const LIBRARY_TYPES: &[(&str, &str)] = &[("QCGI", "RCGI"), ("QDOWNLOAD", "RDOWNLOAD"), ("COMPORT", "RCOMPORT"), ("QCOMPORT", "RCOMPORT"), ("QMIDI", "RMIDI"), ("QWAVE", "RWAVE"), ("QVIDEO", "RVIDEO"), ("QCDAUDIO", "RCDAUDIO")];

/// RapidR's component for library TYPE `name` (any case), if it's one.
pub fn library_component(name: &str) -> Option<&'static str> {
    let upper = name.to_ascii_uppercase();
    LIBRARY_TYPES.iter().find(|(t, _)| *t == upper).map(|(_, r)| *r)
}

fn is_library_type(s: &Statement) -> bool {
    matches!(s, Statement::Type(t) if library_component(&t.name).is_some() && t.extends.as_deref().is_some_and(|e| e.trim().eq_ignore_ascii_case("QOBJECT")))
}

pub fn lower(program: &Program) -> Program {
    let mut program = crate::method_equals::lower(program);
    program.statements.retain(|s| !is_library_type(s));
    let made = instantiated_types(&program);
    for s in &mut program.statements {
        if let Statement::Type(t) = s {
            if !made.contains(&t.name.to_ascii_uppercase()) {
                t.methods.clear();
                t.constructor.clear();
                t.events.clear();
            }
        }
    }
    let cgis = names_of_type(&program.statements, "RCGI");
    rewrite_blocks(&mut program.statements, &cgis);
    program
}

/// Variables and components declared AS a type RapidR calls `component`.
fn names_of_type(stmts: &[Statement], component: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    let is = |t: &str| canonical_type_name(t.trim()).eq_ignore_ascii_case(component);
    walk(
        stmts,
        &mut |s| match s {
            Statement::Dim(d) if is(&d.type_name) => out.extend(d.declarators.iter().map(|v| strip_type_suffix(&v.name).to_ascii_lowercase())),
            Statement::Create(c) if is(&c.type_name) => {
                out.insert(c.name.to_ascii_lowercase());
            }
            _ => {}
        },
        &mut |_| {},
    );
    out
}

fn rewrite_blocks(block: &mut Vec<Statement>, cgis: &HashSet<String>) {
    for s in block.iter_mut() {
        for body in child_bodies_mut(s) {
            rewrite_blocks(body, cgis);
        }
    }
    let mut out = Vec::with_capacity(block.len());
    for mut s in block.drain(..) {
        if let Statement::Call(c) = &mut s {
            if let Expression::Identifier(id) = &c.callee {
                if matches!(id.name.to_ascii_lowercase().as_str(), "environ" | "environ$") && c.args.len() == 1 {
                    c.callee = Expression::Identifier(Identifier { span: id.span, name: "__environ_set".into() });
                }
            }
        }
        if !cgis.is_empty() {
            // `CGI.Get(Name, Value)` alone: `Value = CGI.__get(Name, Value)`.
            if let Statement::Call(c) = &s {
                if let Some(hoisted) = cgi_get(&c.callee, &c.args, cgis) {
                    out.push(hoisted.0);
                    continue;
                }
            }
            let mut hoisted = Vec::new();
            let (exprs, _) = statement_parts_mut(&mut s, false);
            for e in exprs {
                walk_expression_mut(e, &mut |e| {
                    let found = match e {
                        Expression::FunctionCall(f) => cgi_get(&f.callee, &f.args, cgis),
                        Expression::MethodCall(m) => {
                            let callee = Expression::MemberAccess(MemberAccessExpression { span: m.span, object: m.object.clone(), member: m.method.clone() });
                            cgi_get(&callee, &m.args, cgis)
                        }
                        _ => None,
                    };
                    if let Some((assign, read)) = found {
                        hoisted.push(assign);
                        *e = read;
                    }
                });
            }
            out.extend(hoisted);
        }
        out.push(s);
    }
    *block = out;
}

/// `CGI.Get(Name, Value)` on a QCGI: the assignment that sets Value, and
/// what the call reads as.
fn cgi_get(callee: &Expression, args: &[Expression], cgis: &HashSet<String>) -> Option<(Statement, Expression)> {
    let Expression::MemberAccess(m) = callee else { return None };
    let Expression::Identifier(obj) = m.object.as_ref() else { return None };
    if !m.member.eq_ignore_ascii_case("get") || args.len() != 2 || !cgis.contains(&strip_type_suffix(&obj.name).to_ascii_lowercase()) {
        return None;
    }
    let target = &args[1];
    if !matches!(target, Expression::Identifier(_) | Expression::ArrayAccess(_) | Expression::FunctionCall(_) | Expression::MemberAccess(_)) {
        return None;
    }
    let span = m.span;
    let call = Expression::FunctionCall(FunctionCallExpression {
        span,
        callee: Box::new(Expression::MemberAccess(MemberAccessExpression { span, object: m.object.clone(), member: "__get".into() })),
        args: args.to_vec(),
    });
    let assign = Statement::Assignment(AssignmentStatement { span, target: target.clone(), value: call });
    let read = Expression::MemberAccess(MemberAccessExpression { span, object: m.object.clone(), member: "__found".into() });
    Some((assign, read))
}

//! An event's Sender reaches only an object parameter, as in RapidQ.
//!
//! The runtimes hand an event handler the event's arguments and then the
//! firing component, the Sender (`SUB ButtonClick (Sender AS QBUTTON)`).
//! RapidQ (RC.EXE, 2026-10-22, docs/rapidq-ground-truth.md) gives the Sender
//! only to an object parameter: a handler's STRING parameter in its place
//! reads "" and a number one 0 on the handler's first call (RapidQ's
//! parameters keep their place between calls, so after a direct call
//! `H "direct"` the next event finds "direct" there; RapidR gives "" and 0
//! every time). Both backends run this pass: each SUB / FUNCTION bound to
//! an event (`OnClick = H` in a CREATE block, `Obj.OnClick = H`) starts with
//!
//! ```text
//! T = __sender_param(0, T, "S")   ' a BYVAL STRING parameter
//! N = __sender_param(1, N, "N")   ' a BYVAL number parameter
//! __event_done                    ' every handler: the event's mark taken
//! ```
//!
//! `rapidr_value::events` answers: the runtime marks the Sender's place
//! just before it starts a handler for an event (a native build's compiled
//! handler in `events::call`, the interpreter's queued one in the VM), the
//! parameter in that place becomes "" or 0, and `__event_done` takes the
//! mark, so a routine the handler calls directly keeps its arguments. A
//! direct call of the handler has no mark: nothing changes.

use std::collections::HashMap;

use crate::*;

fn key(name: &str) -> String {
    strip_type_suffix(name).to_ascii_lowercase()
}

/// "S" for a STRING parameter, "N" for a number one; `None` for anything
/// else (an object, a VARIANT, no type: the Sender reaches it).
fn kind(p: &Parameter) -> Option<&'static str> {
    if p.by_ref || p.is_array {
        return None;
    }
    let declared = p.type_name.trim().to_ascii_uppercase();
    let t = if declared.is_empty() { suffix_type(&p.name).unwrap_or("").to_string() } else { declared };
    match t.split('*').next().unwrap_or("").trim() {
        "STRING" => Some("S"),
        "BYTE" | "WORD" | "SHORT" | "INTEGER" | "LONG" | "DWORD" | "SINGLE" | "DOUBLE" | "INT64" | "CURRENCY" => Some("N"),
        _ => None,
    }
}

/// The routines `stmts` binds to events (`OnX = Name`, `Obj.OnX = Name`).
fn handlers(stmts: &[Statement], routines: &HashMap<String, usize>, out: &mut Vec<usize>) {
    walk(
        stmts,
        &mut |s| {
            let Statement::Assignment(a) = s else { return };
            let event = match &a.target {
                Expression::Identifier(id) => id.name.as_str(),
                Expression::MemberAccess(m) => m.member.as_str(),
                _ => return,
            };
            // (OnMessage is WndProc's other name — a window procedure gets the
            // message's four numbers and no Sender — rapidr_ast::tray_calls)
            if event.len() <= 2 || !event[..2].eq_ignore_ascii_case("on") || event.eq_ignore_ascii_case("onmessage") {
                return;
            }
            if let Expression::Identifier(h) = &a.value {
                if let Some(&i) = routines.get(&key(&h.name)) {
                    if !out.contains(&i) {
                        out.push(i);
                    }
                }
            }
        },
        &mut |_| {},
    );
}

fn call(span: TextSpan, name: &str, args: Vec<Expression>) -> Expression {
    Expression::FunctionCall(FunctionCallExpression { span, callee: Box::new(Expression::Identifier(Identifier { span, name: name.into() })), args })
}

/// The prologue of a handler with `params` (see the module docs).
fn prologue(span: TextSpan, params: &[Parameter]) -> Vec<Statement> {
    let mut out = Vec::new();
    for (i, p) in params.iter().enumerate() {
        let Some(k) = kind(p) else { continue };
        let target = Expression::Identifier(Identifier { span, name: p.name.clone() });
        let args = vec![
            Expression::Literal(Literal { span, value: LiteralValue::Integer(i as i64) }),
            target.clone(),
            Expression::Literal(Literal { span, value: LiteralValue::String(k.into()) }),
        ];
        out.push(Statement::Assignment(AssignmentStatement { span, target, value: call(span, "__sender_param", args) }));
    }
    out.push(Statement::Call(CallStatement { span, callee: Expression::Identifier(Identifier { span, name: "__event_done".into() }), args: Vec::new() }));
    out
}

/// See the module docs.
pub fn lower(program: &Program) -> Program {
    let mut routines: HashMap<String, usize> = HashMap::new();
    for (i, s) in program.statements.iter().enumerate() {
        match s {
            Statement::Subroutine(r) => {
                routines.entry(key(&r.name)).or_insert(i);
            }
            Statement::Function(f) => {
                routines.entry(key(&f.name)).or_insert(i);
            }
            _ => {}
        }
    }
    let mut bound = Vec::new();
    if !routines.is_empty() {
        handlers(&program.statements, &routines, &mut bound);
    }
    if bound.is_empty() {
        return program.clone();
    }
    let mut program = program.clone();
    for i in bound {
        match &mut program.statements[i] {
            Statement::Subroutine(r) => {
                let entry = prologue(r.span, &r.params);
                r.body.splice(0..0, entry);
            }
            Statement::Function(f) => {
                let entry = prologue(f.span, &f.params);
                f.body.splice(0..0, entry);
            }
            _ => {}
        }
    }
    program
}


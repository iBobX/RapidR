//! `Stream.SaveArray(A(i), n)` and `Stream.LoadArray(A(i), n)` (QFILESTREAM /
//! QMEMORYSTREAM, manual: "saves/loads n elements of an array, starting at
//! element A(i)") as loops both backends compile alike:
//!
//! ```text
//! FOR __sa1 = 0 TO n - 1 : Stream.WriteNum(A(i + __sa1), kind) : NEXT
//! FOR __sa1 = 0 TO n - 1 : A(i + __sa1) = Stream.ReadNum(kind) : NEXT
//! ```
//!
//! `kind` is the storage of the array's declared element type (the `Num_*`
//! constants: BYTE 1, SHORT 2, WORD 3, LONG / INTEGER 4, DWORD 5, SINGLE 6,
//! DOUBLE 8), so the bytes are RapidQ's. The last index steps, as elements
//! are laid out in memory (`vertex(i, j, 0), 3` is `vertex(i, j, 0..2)`).
//! An array of strings, or of an undeclared type, uses the generic
//! `Write` / `Read(var)`.
//!
//! `Stream.Read(x)` / `Stream.Write(x)` of a declared numeric `x` likewise
//! read and write that type's bytes (a SHORT: 2).

use std::collections::HashMap;

use crate::*;

/// `Num_*` storage of a declared type, or `None` (string / variant / object).
fn kind_of_type(type_name: &str) -> Option<i64> {
    Some(match type_name.to_ascii_uppercase().as_str() {
        "BYTE" => 1,
        "SHORT" => 2,
        "WORD" => 3,
        "LONG" | "INTEGER" => 4,
        "DWORD" => 5,
        "SINGLE" => 6,
        "DOUBLE" => 8,
        _ => return None,
    })
}

fn kind_of_suffix(name: &str) -> Option<i64> {
    match name.chars().last()? {
        '%' | '&' => Some(4),
        '!' => Some(6),
        '#' => Some(8),
        _ => None,
    }
}

/// Declared element types: lowercase name → type name.
type Scope = HashMap<String, String>;

fn declare(scope: &mut Scope, stmts: &[Statement]) {
    walk(
        stmts,
        &mut |s| {
            if let Statement::Dim(d) = s {
                for v in &d.declarators {
                    scope.insert(v.name.to_ascii_lowercase(), d.type_name.clone());
                }
            }
        },
        &mut |_| {},
    );
}

struct Pass {
    globals: Scope,
    /// TYPE fields by name (`This.vertex(i, j, 0)`).
    fields: Scope,
    counter: usize,
}

impl Pass {
    fn kind(&self, local: &Scope, name: &str) -> Option<i64> {
        let k = name.to_ascii_lowercase();
        let t = local.get(&k).or_else(|| self.globals.get(&k)).or_else(|| self.fields.get(&k));
        t.and_then(|t| kind_of_type(t)).or_else(|| kind_of_suffix(name))
    }

    fn block(&mut self, stmts: &mut Vec<Statement>, scope: &Scope) {
        for stmt in stmts.iter_mut() {
            match stmt {
                Statement::Subroutine(s) => {
                    let inner = routine_scope(&s.params, &s.body);
                    self.block(&mut s.body, &inner);
                }
                Statement::Function(f) => {
                    let inner = routine_scope(&f.params, &f.body);
                    self.block(&mut f.body, &inner);
                }
                Statement::Call(c) => {
                    if let Some(s) = self.rewrite(c, scope) {
                        *stmt = s;
                    }
                }
                _ => {
                    for body in child_bodies_mut(stmt) {
                        self.block(body, scope);
                    }
                }
            }
        }
    }

    /// `Stream.Read(x)` / `Stream.Write(x)` of a variable, element or field
    /// of a declared numeric type: as many bytes as that type takes
    /// (`x = Stream.ReadNum(kind)` / `Stream.WriteNum(x, kind)`), where the
    /// generic ones go by the value (4 bytes, or 8 for a fraction).
    fn read_write(&self, c: &CallStatement, m: &MemberAccessExpression, scope: &Scope) -> Option<Statement> {
        let Expression::Identifier(stream) = m.object.as_ref() else { return None };
        let k = stream.name.to_ascii_lowercase();
        let t = scope.get(&k).or_else(|| self.globals.get(&k))?;
        if !matches!(crate::canonical_type_name(t).to_ascii_uppercase().as_str(), "RFILESTREAM" | "RMEMORYSTREAM") {
            return None;
        }
        let [x] = c.args.as_slice() else { return None };
        let name = match x {
            Expression::Identifier(id) => &id.name,
            Expression::MemberAccess(ma) => &ma.member,
            Expression::FunctionCall(fc) => match fc.callee.as_ref() {
                Expression::Identifier(id) => &id.name,
                Expression::MemberAccess(ma) => &ma.member,
                _ => return None,
            },
            Expression::ArrayAccess(a) => match a.array.as_ref() {
                Expression::Identifier(id) => &id.name,
                _ => return None,
            },
            _ => return None,
        };
        let kind = self.kind(scope, name)?;
        let span = c.span;
        let int = Expression::Literal(Literal { span, value: LiteralValue::Integer(kind) });
        let method = |member: &str| Expression::MemberAccess(MemberAccessExpression { span, object: m.object.clone(), member: member.into() });
        Some(if m.member.eq_ignore_ascii_case("read") {
            Statement::Assignment(AssignmentStatement {
                span,
                target: x.clone(),
                value: Expression::FunctionCall(FunctionCallExpression { span, callee: Box::new(method("ReadNum")), args: vec![int] }),
            })
        } else {
            Statement::Call(CallStatement { span, callee: method("WriteNum"), args: vec![x.clone(), int] })
        })
    }

    fn rewrite(&mut self, c: &CallStatement, scope: &Scope) -> Option<Statement> {
        let Expression::MemberAccess(m) = &c.callee else { return None };
        let load = match m.member.to_ascii_lowercase().as_str() {
            "savearray" => false,
            "loadarray" => true,
            "read" | "write" => return self.read_write(c, m, scope),
            _ => return None,
        };
        let [element, count] = c.args.as_slice() else { return None };
        // `A(i, …)`: the array expression and its indices.
        let (array, indices) = match element {
            Expression::FunctionCall(fc) if !fc.args.is_empty() => (fc.callee.as_ref(), fc.args.as_slice()),
            Expression::ArrayAccess(a) if !a.indices.is_empty() => (a.array.as_ref(), a.indices.as_slice()),
            _ => return None,
        };
        let name = match array {
            Expression::Identifier(id) => &id.name,
            Expression::MemberAccess(ma) => &ma.member,
            _ => return None,
        };
        let kind = self.kind(scope, name);
        let span = c.span;
        self.counter += 1;
        let var = format!("__sa{}", self.counter);
        let ident = |n: &str| Expression::Identifier(Identifier { span, name: n.into() });
        let int = |n: i64| Expression::Literal(Literal { span, value: LiteralValue::Integer(n) });
        let binary = |l: Expression, operator, r: Expression| Expression::Binary(BinaryExpression { span, left: Box::new(l), operator, right: Box::new(r) });
        let mut idx = indices.to_vec();
        let last = idx.pop()?;
        idx.push(binary(last, BinaryOperator::Add, ident(&var)));
        let index = idx.last().cloned()?;
        let target = Expression::FunctionCall(FunctionCallExpression { span, callee: Box::new(array.clone()), args: idx });
        let method = |member: &str| Expression::MemberAccess(MemberAccessExpression { span, object: m.object.clone(), member: member.into() });
        let body = match (load, kind) {
            (false, Some(k)) => Statement::Call(CallStatement { span, callee: method("WriteNum"), args: vec![target, int(k)] }),
            (false, None) => Statement::Call(CallStatement { span, callee: method("Write"), args: vec![target] }),
            (true, Some(k)) => Statement::Assignment(AssignmentStatement {
                span,
                target,
                value: Expression::FunctionCall(FunctionCallExpression { span, callee: Box::new(method("ReadNum")), args: vec![int(k)] }),
            }),
            // `Read(var)` (rapidr_ast::stream_read_assignment).
            (true, None) => Statement::Call(CallStatement { span, callee: method("Read"), args: vec![target] }),
        };
        // Past the array's last element (RapidQ wrote over whatever memory
        // followed it) the loop stops.
        let mut body = vec![body];
        if let Expression::Identifier(_) = array {
            let last = Expression::FunctionCall(FunctionCallExpression {
                span,
                callee: Box::new(ident("UBOUND")),
                args: vec![array.clone(), int(indices.len() as i64)],
            });
            let past = binary(index, BinaryOperator::GreaterThan, last);
            body.insert(
                0,
                Statement::If(IfStatement {
                    span,
                    condition: past,
                    then_body: vec![Statement::Exit(ExitStatement { span, exit_type: "FOR".into() })],
                    elseif_branches: Vec::new(),
                    else_body: Vec::new(),
                }),
            );
        }
        Some(Statement::For(ForStatement { span, variable: var, start: int(0), end: binary(count.clone(), BinaryOperator::Subtract, int(1)), step: None, body }))
    }
}

fn routine_scope(params: &[Parameter], body: &[Statement]) -> Scope {
    let mut scope: Scope = params.iter().map(|p| (p.name.to_ascii_lowercase(), p.type_name.clone())).collect();
    declare(&mut scope, body);
    scope
}

/// Rewrites every `SaveArray` / `LoadArray` call of `program` into a loop
/// (see the module docs). Programs without them are returned unchanged.
pub fn lower(program: &Program) -> Program {
    let mut found = false;
    walk(
        &program.statements,
        &mut |s| {
            if let Statement::Call(CallStatement { callee: Expression::MemberAccess(m), .. }) = s {
                found |= ["savearray", "loadarray", "read", "write"].iter().any(|n| m.member.eq_ignore_ascii_case(n));
            }
        },
        &mut |_| {},
    );
    if !found {
        return program.clone();
    }
    let mut globals = Scope::new();
    let mut fields = Scope::new();
    let mut main = Vec::new();
    for s in &program.statements {
        match s {
            Statement::Subroutine(_) | Statement::Function(_) => {}
            Statement::Type(t) => {
                for f in &t.fields {
                    fields.insert(f.name.to_ascii_lowercase(), f.type_name.clone());
                }
            }
            other => main.push(other.clone()),
        }
    }
    declare(&mut globals, &main);
    let mut pass = Pass { globals, fields, counter: 0 };
    let mut program = program.clone();
    pass.block(&mut program.statements, &Scope::new());
    program
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds() {
        assert_eq!(kind_of_type("short"), Some(2));
        assert_eq!(kind_of_type("SINGLE"), Some(6));
        assert_eq!(kind_of_type("STRING"), None);
        assert_eq!(kind_of_suffix("x#"), Some(8));
    }
}

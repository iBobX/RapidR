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
//! When the array is a variable of a declared numeric type and its indices
//! are plain (numbers and declared variables, which the loop would read
//! the same each time round), the loop is one call that does the same work
//! at once — `Stream.__ArrayIO(A, load, kind, n, i…)`
//! (`rapidr_value::objects::stream_ops`): the same bytes, the same stop at
//! the array's last element, the same Position, without a method call per
//! element.
//!
//! `Stream.Read(x)` / `Stream.Write(x)` of a declared numeric `x` likewise
//! read and write that type's bytes (a SHORT: 2). `Stream.SaveUDTArray(t.f)`
//! / `LoadUDTArray(t.f)` get the array field's declared type as a second
//! argument.

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

/// `STRING * n`'s type name as `rapidr_value::memory::Kind::of` reads it.
fn with_len(type_name: &str, fixed_len: Option<usize>) -> String {
    match fixed_len {
        Some(n) => format!("STRING*{n}"),
        None => type_name.to_string(),
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
                    scope.insert(v.name.to_ascii_lowercase(), with_len(&d.type_name, d.fixed_len));
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
    /// TYPE fields by TYPE and name (lowercase `type.field`), for a field
    /// two TYPEs both have.
    typed_fields: Scope,
    counter: usize,
    /// The program's SUBs and FUNCTIONs (lowercase): a name that calls one
    /// isn't a plain index.
    routines: std::collections::HashSet<String>,
}

impl Pass {
    /// An index the loop would work out the same each time round: numbers,
    /// declared variables (or ones with a type suffix) and arithmetic on
    /// them — no call, which might give another value or do something.
    fn plain(&self, scope: &Scope, e: &Expression) -> bool {
        match e {
            Expression::Literal(l) => matches!(l.value, LiteralValue::Integer(_) | LiteralValue::Float(_)),
            Expression::Identifier(id) => {
                let k = id.name.to_ascii_lowercase();
                !self.routines.contains(&k) && (scope.contains_key(&k) || self.globals.contains_key(&k) || kind_of_suffix(&id.name).is_some())
            }
            Expression::Binary(b) => {
                matches!(b.operator, BinaryOperator::Add | BinaryOperator::Subtract | BinaryOperator::Multiply)
                    && self.plain(scope, &b.left)
                    && self.plain(scope, &b.right)
            }
            Expression::Unary(u) => u.operator == UnaryOperator::Negate && self.plain(scope, &u.operand),
            _ => false,
        }
    }

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

    /// `Stream.SaveUDTArray(t.field)` / `LoadUDTArray(t.field)` (RC.EXE:
    /// one argument, a TYPE's array field): the field's declared type added
    /// as a second argument, so the runtime lays the elements out as RapidQ
    /// does (`rapidr_value::objects::stream_ops`).
    fn udt_array(&self, c: &CallStatement, scope: &Scope) -> Option<Statement> {
        let [x] = c.args.as_slice() else { return None };
        let name = match x {
            Expression::Identifier(id) => &id.name,
            Expression::MemberAccess(ma) => &ma.member,
            _ => return None,
        };
        let k = name.to_ascii_lowercase();
        let declared = |n: &str| scope.get(n).or_else(|| self.globals.get(n));
        let t = match x {
            Expression::MemberAccess(ma) => {
                let owner = match ma.object.as_ref() {
                    Expression::Identifier(id) => declared(&id.name.to_ascii_lowercase()),
                    _ => None,
                };
                owner.and_then(|o| self.typed_fields.get(&format!("{}.{k}", o.to_ascii_lowercase()))).or_else(|| self.fields.get(&k))
            }
            _ => declared(&k),
        }?;
        let mut call = c.clone();
        call.args.push(Expression::Literal(Literal { span: c.span, value: LiteralValue::String(t.clone()) }));
        Some(Statement::Call(call))
    }

    fn rewrite(&mut self, c: &CallStatement, scope: &Scope) -> Option<Statement> {
        let Expression::MemberAccess(m) = &c.callee else { return None };
        let load = match m.member.to_ascii_lowercase().as_str() {
            "savearray" => false,
            "loadarray" => true,
            "read" | "write" => return self.read_write(c, m, scope),
            "saveudtarray" | "loadudtarray" => return self.udt_array(c, scope),
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
        // (a numeric array variable, plain indices: one call — module docs)
        if let (Some(k), Expression::Identifier(_)) = (kind, array) {
            if indices.iter().all(|i| self.plain(scope, i)) {
                let int = |n: i64| Expression::Literal(Literal { span, value: LiteralValue::Integer(n) });
                let mut args = vec![array.clone(), int(load as i64), int(k), count.clone()];
                args.extend(indices.iter().cloned());
                let callee = Expression::MemberAccess(MemberAccessExpression { span, object: m.object.clone(), member: "__ArrayIO".into() });
                return Some(Statement::Call(CallStatement { span, callee, args }));
            }
        }
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
        Some(Statement::For(ForStatement { span, variable: var, declared_type: None, start: int(0), end: binary(count.clone(), BinaryOperator::Subtract, int(1)), step: None, body }))
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
                found |= ["savearray", "loadarray", "read", "write", "saveudtarray", "loadudtarray"].iter().any(|n| m.member.eq_ignore_ascii_case(n));
            }
        },
        &mut |_| {},
    );
    if !found {
        return program.clone();
    }
    let mut globals = Scope::new();
    let mut fields = Scope::new();
    let mut typed_fields = Scope::new();
    let mut routines = std::collections::HashSet::new();
    let mut main = Vec::new();
    for s in &program.statements {
        match s {
            Statement::Subroutine(r) => {
                routines.insert(r.name.to_ascii_lowercase());
            }
            Statement::Function(f) => {
                routines.insert(f.name.to_ascii_lowercase());
            }
            Statement::Type(t) => {
                for f in &t.fields {
                    fields.insert(f.name.to_ascii_lowercase(), with_len(&f.type_name, f.fixed_len));
                    typed_fields.insert(format!("{}.{}", t.name.to_ascii_lowercase(), f.name.to_ascii_lowercase()), with_len(&f.type_name, f.fixed_len));
                }
            }
            other => main.push(other.clone()),
        }
    }
    declare(&mut globals, &main);
    let mut pass = Pass { globals, fields, typed_fields, counter: 0, routines };
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

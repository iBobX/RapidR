//! Declared numeric types (RapidQ manual, Appendix C), shared by both
//! backends: every store into something declared BYTE, WORD, SHORT,
//! INTEGER/LONG, DWORD, SINGLE or DOUBLE goes through a conversion builtin
//! (`__to_long(v)` & co., `rapidr_value::numeric`), so `DIM n AS INTEGER :
//! n = 2.5` holds 2 and a BYTE wraps at 256 — in the interpreter and in
//! native builds alike. It runs after `objects::lower` (which converts
//! stores into typed TYPE fields itself).
//!
//! Converted: assignments to typed variables and array elements, `INPUT`
//! into them, a FOR counter's start value, BYVAL typed parameters on entry,
//! and a typed FUNCTION's result (`f = …`, `Result = …`, `RETURN …`). A
//! local declaration (or parameter) of another type shadows a typed global
//! of the same name. Not converted: a FOR counter's own increments (an
//! integer counter with an integer STEP stays an integer), type suffixes
//! (`n%`).

use std::collections::HashMap;

use crate::*;
use rapidr_diagnostics::TextSpan;

/// The conversion builtin for a declared type name (`AS INTEGER` →
/// `__to_long`), if numeric. Mirrors `rapidr_value::numeric::NumKind`.
pub fn conversion_for(type_name: &str) -> Option<&'static str> {
    Some(match type_name.trim().to_ascii_uppercase().as_str() {
        "BYTE" => "__to_byte",
        "WORD" => "__to_word",
        "SHORT" => "__to_short",
        "INTEGER" | "LONG" => "__to_long",
        "DWORD" => "__to_dword",
        "SINGLE" | "DOUBLE" => "__to_double",
        _ => return None,
    })
}

/// The conversion builtins this pass (and `objects`) emit.
pub const CONVERSION_BUILTINS: &[&str] = &["__to_byte", "__to_word", "__to_short", "__to_long", "__to_dword", "__to_double", "__to_fixed"];

/// A conversion applied to a store: a numeric builtin, or `__to_fixed(v, n)`
/// for a `STRING * n` (the text cut to `n` characters).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Conv {
    name: &'static str,
    len: Option<usize>,
}

/// The conversion for a declared type (`AS INTEGER`, `AS STRING * 8`), if
/// stores into it convert.
pub fn conv_for(type_name: &str, fixed_len: Option<usize>) -> Option<Conv> {
    match (conversion_for(type_name), fixed_len) {
        (Some(name), _) => Some(Conv { name, len: None }),
        (None, Some(len)) if type_name.trim().eq_ignore_ascii_case("STRING") => Some(Conv { name: "__to_fixed", len: Some(len) }),
        _ => None,
    }
}

/// What a name holds in a scope: a typed scalar, a typed array, or anything
/// else (which shadows a typed global).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Slot {
    Scalar(Conv),
    Array(Conv),
    Other,
}

type Scope = HashMap<String, Slot>;

impl Slot {
    /// A numeric scalar only (a FOR counter isn't a string).
    fn filter_numeric(self) -> Slot {
        match self {
            Slot::Scalar(c) if c.len.is_some() => Slot::Other,
            s => s,
        }
    }
}

/// Names are keyed without their type suffix, as both backends do (`n%`
/// is `n`).
fn key(name: &str) -> String {
    crate::strip_type_suffix(&name.to_ascii_lowercase()).to_string()
}

fn slot_for(type_name: &str, fixed_len: Option<usize>, is_array: bool) -> Slot {
    match (conv_for(type_name, fixed_len), is_array) {
        (Some(c), false) => Slot::Scalar(c),
        (Some(c), true) => Slot::Array(c),
        (None, _) => Slot::Other,
    }
}

/// `conv(value)` (`__to_fixed(value, n)` for a fixed-length string).
pub fn convert_at(span: TextSpan, conv: Conv, value: Expression) -> Expression {
    // Already converted (a literal of the right kind stays as written).
    if let Expression::FunctionCall(fc) = &value {
        if matches!(fc.callee.as_ref(), Expression::Identifier(id) if id.name == conv.name) {
            return value;
        }
    }
    let mut args = vec![value];
    if let Some(len) = conv.len {
        args.push(Expression::Literal(Literal { span, value: LiteralValue::Integer(len as i64) }));
    }
    Expression::FunctionCall(FunctionCallExpression {
        span,
        callee: Box::new(Expression::Identifier(Identifier { span, name: conv.name.into() })),
        args,
    })
}

/// Declarations in `stmts` (DIM, STATIC, REDIM), into `scope`, not
/// descending into SUBs/FUNCTIONs.
fn declare(stmts: &[Statement], scope: &mut Scope) {
    walk(
        stmts,
        &mut |s| {
            if let Statement::Dim(d) = s {
                for v in &d.declarators {
                    scope.insert(key(&v.name), slot_for(&d.type_name, d.fixed_len, !v.dimensions.is_empty()));
                }
            }
        },
        &mut |_| {},
    );
}

struct Pass<'a> {
    globals: &'a Scope,
    locals: Scope,
    /// A typed FUNCTION being converted: its name (lowercase) and conversion.
    result: Option<(String, Conv)>,
    /// Inside a CREATE block, bare names are the component's properties.
    in_create: usize,
}

impl Pass<'_> {
    fn lookup(&self, name: &str) -> Slot {
        let k = key(name);
        if let Some(s) = self.locals.get(&k) {
            return *s;
        }
        if let Some((f, conv)) = &self.result {
            if k == *f || k == "result" {
                return Slot::Scalar(*conv);
            }
        }
        // An undeclared `q% = 40000` stores as its suffix's type.
        self.globals.get(&k).copied().unwrap_or_else(|| slot_for(crate::suffix_type(name).unwrap_or(""), None, false))
    }

    /// The conversion a store into `target` needs.
    fn target_conversion(&self, target: &Expression) -> Option<Conv> {
        match target {
            Expression::Identifier(id) if self.in_create == 0 => match self.lookup(&id.name) {
                Slot::Scalar(c) => Some(c),
                _ => None,
            },
            Expression::FunctionCall(FunctionCallExpression { callee, .. })
            | Expression::ArrayAccess(ArrayAccessExpression { array: callee, .. }) => match callee.as_ref() {
                Expression::Identifier(id) => match self.lookup(&id.name) {
                    Slot::Array(c) => Some(c),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        }
    }

    fn body(&mut self, stmts: &mut Vec<Statement>) {
        let mut out = Vec::with_capacity(stmts.len());
        for mut s in stmts.drain(..) {
            let after = self.stmt(&mut s);
            out.push(s);
            out.extend(after);
        }
        *stmts = out;
    }

    /// Converts `s` in place; returns statements to add after it.
    fn stmt(&mut self, s: &mut Statement) -> Vec<Statement> {
        match s {
            Statement::Assignment(a) => {
                if let Some(conv) = self.target_conversion(&a.target) {
                    let value = std::mem::replace(&mut a.value, Expression::Literal(Literal { span: a.span, value: LiteralValue::Integer(0) }));
                    a.value = convert_at(a.span, conv, value);
                }
            }
            Statement::Input(i) => {
                if let Some(conv) = self.target_conversion(&i.target) {
                    let target = i.target.clone();
                    return vec![Statement::Assignment(AssignmentStatement { span: i.span, value: convert_at(i.span, conv, target.clone()), target })];
                }
            }
            Statement::Return(r) => {
                if let (Some(v), Some((_, conv))) = (r.value.as_mut(), self.result.as_ref()) {
                    *v = convert_at(r.span, *conv, v.clone());
                }
            }
            Statement::If(i) => {
                self.body(&mut i.then_body);
                for b in &mut i.elseif_branches {
                    self.body(&mut b.body);
                }
                self.body(&mut i.else_body);
            }
            Statement::For(f) => {
                // The start value is a store into the counter; its own
                // increments aren't converted (as in QBasic's FOR).
                if let Slot::Scalar(conv) = self.lookup(&f.variable).filter_numeric() {
                    if self.in_create == 0 {
                        f.start = convert_at(f.span, conv, f.start.clone());
                    }
                }
                self.body(&mut f.body)
            }
            Statement::While(w) => self.body(&mut w.body),
            Statement::DoLoop(d) => self.body(&mut d.body),
            Statement::SelectCase(c) => {
                for b in &mut c.cases {
                    self.body(&mut b.body);
                }
                self.body(&mut c.case_else);
            }
            Statement::With(w) => self.body(&mut w.body),
            Statement::Create(c) => {
                self.in_create += 1;
                self.body(&mut c.body);
                self.in_create -= 1;
            }
            _ => {}
        }
        Vec::new()
    }

    /// A SUB/FUNCTION: its own scope, BYVAL typed parameters converted on
    /// entry.
    fn routine(globals: &Scope, params: &[Parameter], body: &mut Vec<Statement>, result: Option<(String, Conv)>, span: TextSpan) {
        let mut locals = Scope::new();
        for p in params {
            locals.insert(key(&p.name), slot_for(&p.type_name, None, p.is_array));
        }
        declare(body, &mut locals);
        // A local of the function's own name would shadow its result.
        let result = result.filter(|(f, _)| !locals.contains_key(f));
        let mut pass = Pass { globals, locals, result, in_create: 0 };
        pass.body(body);
        let entry: Vec<Statement> = params
            .iter()
            .filter(|p| !p.by_ref && !p.is_array)
            .filter_map(|p| {
                let conv = conv_for(&p.type_name, None)?;
                let target = Expression::Identifier(Identifier { span, name: p.name.clone() });
                Some(Statement::Assignment(AssignmentStatement { span, value: convert_at(span, conv, target.clone()), target }))
            })
            .collect();
        if !entry.is_empty() {
            body.splice(0..0, entry);
        }
    }
}

/// Inserts the conversions (see the module docs).
pub fn lower(mut program: Program) -> Program {
    let mut globals = Scope::new();
    let main: Vec<Statement> = program.statements.iter().filter(|s| !matches!(s, Statement::Subroutine(_) | Statement::Function(_))).cloned().collect();
    declare(&main, &mut globals);
    let globals = globals;
    let mut main_pass = Pass { globals: &globals, locals: Scope::new(), result: None, in_create: 0 };
    let mut out = Vec::with_capacity(program.statements.len());
    for mut s in program.statements.drain(..) {
        match &mut s {
            Statement::Subroutine(sub) => {
                let span = sub.span;
                Pass::routine(&globals, &sub.params, &mut sub.body, None, span);
                out.push(s);
            }
            Statement::Function(f) => {
                let span = f.span;
                let result = f.return_type.as_deref().and_then(|t| conv_for(t, None)).map(|c| (key(&f.name), c));
                Pass::routine(&globals, &f.params, &mut f.body, result, span);
                out.push(s);
            }
            _ => {
                let after = main_pass.stmt(&mut s);
                out.push(s);
                out.extend(after);
            }
        }
    }
    program.statements = out;
    program
}

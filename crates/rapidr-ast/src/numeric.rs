//! Declared numeric types (RapidQ manual, Appendix C), shared by both
//! backends: every store into something declared BYTE, WORD, SHORT,
//! INTEGER/LONG, DWORD, SINGLE, DOUBLE, INT64 or UINT64 goes through a
//! conversion builtin
//! (`__to_long(v)` & co., `rapidr_value::numeric`), so `DIM n AS INTEGER :
//! n = 2.5` holds 2 and a BYTE wraps at 256 — in the interpreter and in
//! native builds alike. It runs after `objects::lower` (which converts
//! stores into typed TYPE fields itself).
//!
//! Converted: assignments to typed variables and array elements, `INPUT`
//! into them, a FOR counter's start value, BYVAL typed parameters on entry
//! (an integer one rounded half to even first, `__arg_round`, as RapidQ's
//! RC.EXE does). A local declaration (or parameter) of another type shadows
//! a typed global of the same name. Not converted: a typed FUNCTION's
//! result (RapidQ returns `F = 2.7` from a `FUNCTION F AS INTEGER` as 2.7), a
//! FOR counter's own increments (an integer counter with an integer STEP
//! stays an integer), type suffixes (`n%`).
//!
//! INT64 / UINT64 (RapidR's) are also *read* as 64-bit numbers: see the
//! end of this file.

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
        "DOUBLE" => "__to_double",
        "SINGLE" => "__to_single",
        "INT64" => "__to_int64",
        "UINT64" => "__to_uint64",
        _ => return None,
    })
}

/// The conversion builtins this pass (and `objects`) emit.
pub const CONVERSION_BUILTINS: &[&str] =
    &["__to_byte", "__to_word", "__to_short", "__to_long", "__to_dword", "__to_double", "__to_single", "__to_int64", "__to_uint64", "__to_fixed", "__arg_round"];

/// A conversion applied to a store: a numeric builtin, or `__to_fixed(v, n)`
/// for a `STRING * n` (the text cut to `n` characters).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Conv {
    name: &'static str,
    len: Option<usize>,
}

impl Conv {
    /// An INT64 / UINT64's (`__to_int64` / `__to_uint64`).
    pub fn is_wide(&self) -> bool {
        wide_conv(*self).is_some()
    }
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
        &mut |s| match s {
            Statement::Dim(d) => {
                for v in &d.declarators {
                    scope.insert(key(&v.name), slot_for(&d.type_name, d.fixed_len, !v.dimensions.is_empty()));
                }
            }
            // (`CONST BIG AS INT64 = …`: read as one; nothing stores into it)
            Statement::Const(c) if c.declared_type.as_deref().and_then(wide_type).is_some() => {
                scope.insert(key(&c.name), slot_for(c.declared_type.as_deref().unwrap_or(""), None, false));
            }
            _ => {}
        },
        &mut |_| {},
    );
}

struct Pass<'a> {
    globals: &'a Scope,
    /// INT64 / UINT64 FUNCTIONs (lowercase names); `None` when the program
    /// has no INT64 / UINT64 at all (nothing is rewritten then).
    wide: Option<&'a HashMap<String, Wide>>,
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
            // `DIM s AS STRING * n` starts as n spaces (RC.EXE prints them).
            Statement::Dim(d) if d.fixed_len.is_some() && !d.is_static && !d.is_redim && d.type_name.trim().eq_ignore_ascii_case("STRING") => {
                let conv = Conv { name: "__to_fixed", len: d.fixed_len };
                let empty = Expression::Literal(Literal { span: d.span, value: LiteralValue::String(String::new()) });
                return d
                    .declarators
                    .iter()
                    .filter(|v| v.dimensions.is_empty())
                    .map(|v| {
                        let target = Expression::Identifier(Identifier { span: v.span, name: v.name.clone() });
                        Statement::Assignment(AssignmentStatement { span: d.span, target, value: convert_at(d.span, conv, empty.clone()) })
                    })
                    .collect();
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
    #[allow(clippy::too_many_arguments)]
    fn routine(globals: &Scope, wide: Option<&HashMap<String, Wide>>, params: &[Parameter], body: &mut Vec<Statement>, result: Option<(String, Conv)>, span: TextSpan, function: bool) {
        let mut locals = Scope::new();
        for p in params {
            locals.insert(key(&p.name), slot_for(&p.type_name, None, p.is_array));
        }
        declare(body, &mut locals);
        // (a FUNCTION's RESULT is its value, never a global named Result:
        // not converted — crate::function_result)
        if function {
            locals.insert("result".to_string(), Slot::Other);
        }
        // A local of the function's own name would shadow its result.
        let result = result.filter(|(f, _)| !locals.contains_key(f));
        let mut pass = Pass { globals, wide, locals, result, in_create: 0 };
        pass.body(body);
        let entry: Vec<Statement> = params
            .iter()
            .filter(|p| !p.by_ref && !p.is_array)
            .filter_map(|p| {
                let conv = conv_for(&p.type_name, None)?;
                let target = Expression::Identifier(Identifier { span, name: p.name.clone() });
                // An integer parameter rounds half to even first (RC.EXE:
                // `P 2.5` gets 2, `P 2.7` 3), where a store truncates.
                let value = if conv.len.is_none() && !matches!(conv.name, "__to_double" | "__to_single") {
                    Expression::FunctionCall(FunctionCallExpression {
                        span,
                        callee: Box::new(Expression::Identifier(Identifier { span, name: "__arg_round".into() })),
                        args: vec![target.clone()],
                    })
                } else {
                    target.clone()
                };
                Some(Statement::Assignment(AssignmentStatement { span, value: convert_at(span, conv, value), target }))
            })
            .collect();
        if !entry.is_empty() {
            body.splice(0..0, entry);
        }
        pass.widen(body);
    }
}

/// Inserts the conversions (see the module docs).
pub fn lower(mut program: Program) -> Program {
    let mut globals = Scope::new();
    let main: Vec<Statement> = program.statements.iter().filter(|s| !matches!(s, Statement::Subroutine(_) | Statement::Function(_))).cloned().collect();
    declare(&main, &mut globals);
    let globals = globals;
    let wide_functions = wide_functions(&program);
    let wide = wide_functions.as_ref();
    let mut main_pass = Pass { globals: &globals, wide, locals: Scope::new(), result: None, in_create: 0 };
    let mut out = Vec::with_capacity(program.statements.len());
    for mut s in program.statements.drain(..) {
        match &mut s {
            Statement::Subroutine(sub) => {
                let span = sub.span;
                Pass::routine(&globals, wide, &sub.params, &mut sub.body, None, span, false);
                out.push(s);
            }
            Statement::Function(f) => {
                let span = f.span;
                // (its result isn't converted: RapidQ returns what was
                // stored, see the module docs)
                Pass::routine(&globals, wide, &f.params, &mut f.body, None, span, true);
                out.push(s);
            }
            _ => {
                let mut after = main_pass.stmt(&mut s);
                main_pass.widen(std::slice::from_mut(&mut s));
                main_pass.widen(&mut after);
                out.push(s);
                out.extend(after);
            }
        }
    }
    program.statements = out;
    program
}

// ---- INT64 / UINT64 (RapidR's 64-bit integers, rapidr_value::wide) ----
//
// A `Value::Integer` holds 64 bits and `+ - *` already wrap at 64, but
// PRINT, STR$, HEX$, `\`, MOD, the comparisons and AND/OR/XOR/NOT/SHL/SHR
// see 32 bits (or a float), as RapidQ does. An expression with an INT64 or
// UINT64 in it (a variable, an array element, a TYPE field, a FUNCTION's
// result) is rewritten into `rapidr_value::wide`'s builtins instead — so
// the interpreter, native builds and the web do the same — and nothing
// else is: a program without INT64 / UINT64 compiles exactly as before.
//
// The rules: `+ - * \ MOD` and the bit operators give an INT64, a UINT64
// when either side is one; `\`, MOD and the comparisons work on the exact
// values (a UINT64 above 2^63 is above any INT64; `u > -1` is true), the
// result wrapped to 64 bits; a DOUBLE operand makes a DOUBLE as usual (for
// `+ - *`; `/` and `^` are always DOUBLE). A whole-number literal in such
// an expression is exact up to 18446744073709551615 (a UINT64 above
// 9223372036854775807), `&H` up to 16 digits.

/// An expression's 64-bit type.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Wide {
    Signed,
    Unsigned,
}

/// INT64 / UINT64 for a declared type name.
pub fn wide_type(type_name: &str) -> Option<Wide> {
    match type_name.trim().to_ascii_uppercase().as_str() {
        "INT64" => Some(Wide::Signed),
        "UINT64" => Some(Wide::Unsigned),
        _ => None,
    }
}

fn wide_conv(conv: Conv) -> Option<Wide> {
    match conv.name {
        "__to_int64" => Some(Wide::Signed),
        "__to_uint64" => Some(Wide::Unsigned),
        _ => None,
    }
}

/// UINT64 when either side is one.
fn combine(a: Option<Wide>, b: Option<Wide>) -> Option<Wide> {
    match (a, b) {
        (None, None) => None,
        (Some(Wide::Unsigned), _) | (_, Some(Wide::Unsigned)) => Some(Wide::Unsigned),
        _ => Some(Wide::Signed),
    }
}

fn unsigned_bit(w: Option<Wide>) -> i64 {
    (w == Some(Wide::Unsigned)) as i64
}

/// The program's INT64 / UINT64 FUNCTIONs, or `None` when it has no INT64 /
/// UINT64 anywhere (then nothing is rewritten).
fn wide_functions(program: &Program) -> Option<HashMap<String, Wide>> {
    let any = std::cell::Cell::new(false);
    let mut functions = HashMap::new();
    let is_wide = |t: &str| wide_type(t).is_some();
    walk(
        &program.statements,
        &mut |s| match s {
            Statement::Dim(d) if is_wide(&d.type_name) => any.set(true),
            Statement::Const(c) if c.declared_type.as_deref().is_some_and(is_wide) => any.set(true),
            Statement::Subroutine(r) if r.params.iter().any(|p| is_wide(&p.type_name)) => any.set(true),
            Statement::Function(f) => {
                if f.params.iter().any(|p| is_wide(&p.type_name)) {
                    any.set(true);
                }
                if let Some(w) = f.return_type.as_deref().and_then(wide_type) {
                    any.set(true);
                    functions.insert(key(&f.name), w);
                }
            }
            _ => {}
        },
        &mut |e| {
            if let Expression::FunctionCall(fc) = e {
                if matches!(fc.callee.as_ref(), Expression::Identifier(id) if id.name == "__to_int64" || id.name == "__to_uint64") {
                    any.set(true);
                }
            }
        },
    );
    any.get().then_some(functions)
}

fn call(span: TextSpan, name: &str, args: Vec<Expression>) -> Expression {
    Expression::FunctionCall(FunctionCallExpression { span, callee: Box::new(Expression::Identifier(Identifier { span, name: name.into() })), args })
}

fn int(span: TextSpan, n: i64) -> Expression {
    Expression::Literal(Literal { span, value: LiteralValue::Integer(n) })
}

fn span_of(e: &Expression) -> TextSpan {
    match e {
        Expression::ArrayAccess(n) => n.span,
        Expression::Binary(n) => n.span,
        Expression::FunctionCall(n) => n.span,
        Expression::Identifier(n) => n.span,
        Expression::Literal(n) => n.span,
        Expression::MemberAccess(n) => n.span,
        Expression::MethodCall(n) => n.span,
        Expression::Unary(n) => n.span,
    }
}

/// A whole-number literal's 64 bits, exact, when it needs them: digits
/// beyond 9223372036854775807 (the parser leaves them text), an `&H` of 9
/// to 16 digits (its AST value is the low 32 bits), negated too. With its
/// type: UINT64 above 9223372036854775807.
fn wide_literal(e: &Expression) -> Option<(i64, Option<Wide>)> {
    match e {
        Expression::Literal(Literal { value: LiteralValue::String(s), .. }) if !s.is_empty() && s.len() <= 20 && s.bytes().all(|b| b.is_ascii_digit()) => {
            let n = s.parse::<u64>().ok()?;
            Some((n as i64, (n > i64::MAX as u64).then_some(Wide::Unsigned)))
        }
        Expression::Literal(Literal { span, value: LiteralValue::Integer(low) }) => {
            let full = crate::wide_literals::full(*span, *low)?;
            Some((full as i64, (full > i64::MAX as u64).then_some(Wide::Unsigned)))
        }
        Expression::Unary(UnaryExpression { operator: UnaryOperator::Negate, operand, .. }) => {
            let (n, _) = wide_literal(operand)?;
            Some((n.wrapping_neg(), None))
        }
        _ => None,
    }
}

/// `e` with a [`wide_literal`] made its exact number.
fn fix_literal(e: &mut Expression) {
    if let Some((n, _)) = wide_literal(e) {
        *e = int(span_of(e), n);
    }
}

fn take(e: &mut Expression) -> Expression {
    std::mem::replace(e, int(span_of(e), 0))
}

impl Pass<'_> {
    /// The 64-bit type of `e` (already rewritten): a variable, element,
    /// field or FUNCTION result of INT64 / UINT64, or arithmetic on one.
    fn wide_of(&self, e: &Expression) -> Option<Wide> {
        let functions = self.wide?;
        match e {
            Expression::Identifier(id) => match self.lookup(&id.name) {
                Slot::Scalar(c) if self.in_create == 0 => wide_conv(c),
                Slot::Other => functions.get(&key(&id.name)).copied(),
                _ => None,
            },
            Expression::FunctionCall(FunctionCallExpression { callee, args, .. }) | Expression::ArrayAccess(ArrayAccessExpression { array: callee, indices: args, .. }) => {
                let Expression::Identifier(id) = callee.as_ref() else { return None };
                let mode = |i: usize| match args.get(i) {
                    Some(Expression::Literal(Literal { value: LiteralValue::Integer(n), .. })) => *n,
                    _ => 0,
                };
                match id.name.as_str() {
                    "__to_int64" => Some(Wide::Signed),
                    "__to_uint64" => Some(Wide::Unsigned),
                    "__idiv64" | "__mod64" => Some(if mode(2) != 0 { Wide::Unsigned } else { Wide::Signed }),
                    "__bits64" => Some(if mode(0) & 8 != 0 { Wide::Unsigned } else { Wide::Signed }),
                    _ => match self.lookup(&id.name) {
                        Slot::Array(c) => wide_conv(c),
                        Slot::Other => functions.get(&key(&id.name)).copied(),
                        Slot::Scalar(_) => None,
                    },
                }
            }
            Expression::Binary(b) if matches!(b.operator, BinaryOperator::Add | BinaryOperator::Subtract | BinaryOperator::Multiply) => {
                let (l, r) = (self.wide_of(&b.left), self.wide_of(&b.right));
                (l.is_some() || r.is_some()).then(|| combine(l.or(wide_literal(&b.left).and_then(|x| x.1)), r.or(wide_literal(&b.right).and_then(|x| x.1))))?
            }
            Expression::Unary(u) if matches!(u.operator, UnaryOperator::Negate | UnaryOperator::Positive) => self.wide_of(&u.operand),
            _ => None,
        }
    }

    /// An operand's type, its literal's too (once the other side is wide).
    fn operand(&self, e: &Expression) -> Option<Wide> {
        self.wide_of(e).or_else(|| wide_literal(e).and_then(|x| x.1))
    }

    /// One expression (its parts already done, `walk_expressions_mut`).
    fn widen_expr(&self, e: &mut Expression) {
        match e {
            Expression::Binary(b) => {
                // (`/` and `^` are DOUBLE, `&` joins or adds as ever)
                if matches!(b.operator, BinaryOperator::Divide | BinaryOperator::Power | BinaryOperator::Concat) {
                    return;
                }
                let (lw, rw) = (self.wide_of(&b.left), self.wide_of(&b.right));
                if lw.is_none() && rw.is_none() {
                    return;
                }
                let (lw, rw) = (self.operand(&b.left), self.operand(&b.right));
                fix_literal(&mut b.left);
                fix_literal(&mut b.right);
                let span = b.span;
                let mode = unsigned_bit(lw) | unsigned_bit(rw) << 1;
                let u = unsigned_bit(combine(lw, rw)) << 3;
                let (l, r) = (take(&mut b.left), take(&mut b.right));
                *e = match b.operator {
                    BinaryOperator::IntegerDivide => call(span, "__idiv64", vec![l, r, int(span, mode)]),
                    BinaryOperator::Modulo => call(span, "__mod64", vec![l, r, int(span, mode)]),
                    BinaryOperator::And => call(span, "__bits64", vec![int(span, u), l, r]),
                    BinaryOperator::Or => call(span, "__bits64", vec![int(span, 1 | u), l, r]),
                    BinaryOperator::Xor => call(span, "__bits64", vec![int(span, 2 | u), l, r]),
                    op @ (BinaryOperator::Equal
                    | BinaryOperator::NotEqual
                    | BinaryOperator::LessThan
                    | BinaryOperator::LessThanOrEqual
                    | BinaryOperator::GreaterThan
                    | BinaryOperator::GreaterThanOrEqual) => Expression::Binary(BinaryExpression {
                        span,
                        left: Box::new(call(span, "__cmp64", vec![l, r, int(span, mode)])),
                        operator: op,
                        right: Box::new(int(span, 0)),
                    }),
                    op => Expression::Binary(BinaryExpression { span, left: Box::new(l), operator: op, right: Box::new(r) }),
                };
            }
            Expression::Unary(u) if u.operator == UnaryOperator::Not => {
                if let Some(w) = self.wide_of(&u.operand) {
                    let span = u.span;
                    let x = take(&mut u.operand);
                    *e = call(span, "__bits64", vec![int(span, 3 | unsigned_bit(Some(w)) << 3), x, int(span, 0)]);
                }
            }
            Expression::FunctionCall(fc) => {
                let Expression::Identifier(id) = fc.callee.as_ref() else { return };
                let name = key(&id.name);
                let span = fc.span;
                match name.as_str() {
                    // A store into an INT64 / UINT64: its literal exact.
                    "__to_int64" | "__to_uint64" => {
                        if let Some(a) = fc.args.first_mut() {
                            fix_literal(a);
                        }
                    }
                    "str" | "hex" if fc.args.len() == 1 => {
                        if let Some(w) = self.wide_of(&fc.args[0]) {
                            let a = take(&mut fc.args[0]);
                            *e = if name == "str" { call(span, "__str64", vec![a, int(span, 2 | unsigned_bit(Some(w)))]) } else { call(span, "__hex64", vec![a]) };
                        }
                    }
                    "shl" | "shr" if fc.args.len() == 2 => {
                        if let Some(w) = self.wide_of(&fc.args[0]) {
                            fix_literal(&mut fc.args[1]);
                            let (a, n) = (take(&mut fc.args[0]), take(&mut fc.args[1]));
                            let op = if name == "shl" { 4 } else { 5 };
                            *e = call(span, "__bits64", vec![int(span, op | unsigned_bit(Some(w)) << 3), a, n]);
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    /// The INT64 / UINT64 rewrite of `stmts` (see above).
    fn widen(&self, stmts: &mut [Statement]) {
        if self.wide.is_none() {
            return;
        }
        walk_expressions_mut(stmts, true, &mut |e| self.widen_expr(e));
        let show = |items: &mut Vec<Expression>| {
            for item in items.iter_mut() {
                if let Some(w) = self.wide_of(item) {
                    let span = span_of(item);
                    let x = take(item);
                    *item = call(span, "__str64", vec![x, int(span, unsigned_bit(Some(w)))]);
                }
            }
        };
        walk_statements_mut(stmts, &mut |s| match s {
            Statement::Print(p) => show(&mut p.items),
            Statement::PrintHash(p) => show(&mut p.items),
            Statement::Const(c) if c.declared_type.as_deref().and_then(wide_type).is_some() => fix_literal(&mut c.value),
            _ => {}
        });
    }
}

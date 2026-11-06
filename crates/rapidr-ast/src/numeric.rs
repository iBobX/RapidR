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
//! INT64 / UINT64 / UINT32 (RapidR's): their literals are read whole and a
//! 64-bit FUNCTION's result is converted: see the end of this file.

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
        "UINT32" => "__to_uint32",
        _ => return None,
    })
}

/// The conversion builtins this pass (and `objects`) emit.
pub const CONVERSION_BUILTINS: &[&str] =
    &["__to_byte", "__to_word", "__to_short", "__to_long", "__to_dword", "__to_double", "__to_single", "__to_int64", "__to_uint64", "__to_uint32", "__to_fixed", "__arg_round"];

/// A conversion applied to a store: a numeric builtin, or `__to_fixed(v, n)`
/// for a `STRING * n` (the text cut to `n` characters).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Conv {
    name: &'static str,
    len: Option<usize>,
}

impl Conv {
    /// An INT64 / UINT64 / UINT32's (`__to_int64`, `__to_uint64`,
    /// `__to_uint32`).
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
    /// The program's 64-bit routines; `None` when it has no INT64 / UINT64
    /// / UINT32 at all (nothing is touched then).
    wide: Option<&'a WideInfo>,
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
    fn routine(globals: &Scope, wide: Option<&WideInfo>, params: &[Parameter], body: &mut Vec<Statement>, result: Option<(String, Conv)>, span: TextSpan, function: bool) {
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
    let wide_info = wide_info(&program);
    let wide = wide_info.as_ref();
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
                // stored, see the module docs — except a 64-bit one's)
                let result = wide.and_then(|_| wide_result(f));
                Pass::routine(&globals, wide, &f.params, &mut f.body, result, span, true);
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

// ---- INT64 / UINT64 / UINT32 (RapidR's 64-bit numbers) ----
//
// Their values carry their type (`rapidr_value::Value::Int64` / `UInt64`,
// docs/big-numbers-design.md): PRINT, STR$, `&`, the operators, a VARIANT,
// an array, FORMAT$, the debugger all see a 64-bit number as one at run
// time, the same in the interpreter, native builds and the web. What only
// the compiler can do is read a *literal* whole: the parser keeps RapidQ's
// reading in the AST (`&H123456789` is its low 32 bits; a decimal beyond
// 9223372036854775807 a DOUBLE) and notes the full value
// (`crate::wide_literals`). Where a literal meets a 64-bit or UINT32 value
// — an operator's other side, a store into one, a FOR counter's start, end
// and step, a SELECT CASE on one, a CONST AS INT64, an argument to a
// 64-bit parameter — it is read as written: `&HFFFFFFFF` is 4294967295
// there, `&H` up to 16 digits, a decimal up to 18446744073709551615 (a
// UINT64 above 9223372036854775807). A program without INT64 / UINT64 /
// UINT32 isn't touched at all. Also: a 64-bit FUNCTION's result is
// converted (RapidQ's rule of returning what was stored is for its own
// types), and `__to_int64(VAL(x))` reads `x` whole.

/// An expression's 64-bit type: INT64 (UINT32's values are INT64s too) or
/// UINT64.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Wide {
    Signed,
    Unsigned,
}

/// INT64 / UINT64 / UINT32 for a declared type name.
pub fn wide_type(type_name: &str) -> Option<Wide> {
    match type_name.trim().to_ascii_uppercase().as_str() {
        "INT64" | "UINT32" => Some(Wide::Signed),
        "UINT64" => Some(Wide::Unsigned),
        _ => None,
    }
}

fn wide_conv(conv: Conv) -> Option<Wide> {
    match conv.name {
        "__to_int64" | "__to_uint32" => Some(Wide::Signed),
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

/// What the pass knows of the program's 64-bit routines.
#[derive(Default)]
struct WideInfo {
    /// 64-bit FUNCTIONs' result types (lowercase names).
    functions: HashMap<String, Wide>,
    /// Routines with a 64-bit parameter: each parameter's type (lowercase
    /// names).
    params: HashMap<String, Vec<Option<Wide>>>,
}

/// The program's 64-bit routines, or `None` when it has no INT64 / UINT64 /
/// UINT32 anywhere (then nothing is touched).
fn wide_info(program: &Program) -> Option<WideInfo> {
    let any = std::cell::Cell::new(false);
    let mut info = WideInfo::default();
    let is_wide = |t: &str| wide_type(t).is_some();
    walk(
        &program.statements,
        &mut |s| match s {
            Statement::Dim(d) if is_wide(&d.type_name) => any.set(true),
            Statement::Const(c) if c.declared_type.as_deref().is_some_and(is_wide) => any.set(true),
            Statement::Subroutine(r) => {
                if r.params.iter().any(|p| is_wide(&p.type_name)) {
                    any.set(true);
                    info.params.insert(key(&r.name), r.params.iter().map(|p| if p.is_array { None } else { wide_type(&p.type_name) }).collect());
                }
            }
            Statement::Function(f) => {
                if f.params.iter().any(|p| is_wide(&p.type_name)) {
                    any.set(true);
                    info.params.insert(key(&f.name), f.params.iter().map(|p| if p.is_array { None } else { wide_type(&p.type_name) }).collect());
                }
                if let Some(w) = f.return_type.as_deref().and_then(wide_type) {
                    any.set(true);
                    info.functions.insert(key(&f.name), w);
                }
            }
            _ => {}
        },
        &mut |e| {
            if let Expression::FunctionCall(fc) = e {
                if matches!(fc.callee.as_ref(), Expression::Identifier(id) if matches!(id.name.as_str(), "__to_int64" | "__to_uint64" | "__to_uint32")) {
                    any.set(true);
                }
            }
        },
    );
    any.get().then_some(info)
}

/// A 64-bit FUNCTION's result conversion (a store into its name or RESULT
/// converts, as a variable's does).
fn wide_result(f: &FunctionStatement) -> Option<(String, Conv)> {
    let t = f.return_type.as_deref()?;
    wide_type(t)?;
    Some((key(&f.name), conv_for(t, None)?))
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

/// A literal's whole number as written, when the AST holds RapidQ's
/// reading of it: an `&H` literal (its unsigned value, up to 16 digits), a
/// decimal beyond 9223372036854775807 (the AST has a DOUBLE), negated too.
/// With its type: UINT64 above 9223372036854775807.
fn wide_literal(e: &Expression) -> Option<(i64, Option<Wide>)> {
    match e {
        Expression::Literal(Literal { span, value: LiteralValue::Integer(low) }) => {
            let full = crate::wide_literals::full(*span, *low)?;
            Some((full as i64, (full > i64::MAX as u64).then_some(Wide::Unsigned)))
        }
        Expression::Literal(Literal { span, value: LiteralValue::Float(f) }) => {
            let full = crate::wide_literals::full(*span, f.to_bits() as i64)?;
            Some((full as i64, (full > i64::MAX as u64).then_some(Wide::Unsigned)))
        }
        Expression::Unary(UnaryExpression { operator: UnaryOperator::Negate, operand, .. }) => {
            let (n, _) = wide_literal(operand)?;
            Some((n.wrapping_neg(), None))
        }
        _ => None,
    }
}

/// `e` with a [`wide_literal`] made its whole number: a UINT64 one as
/// `__to_uint64(bits)` (so it compares as the UINT64 it is), one beyond 32
/// bits as `__to_int64(n)`, the rest as a plain whole number.
fn fix_literal(e: &mut Expression) {
    if let Some((n, w)) = wide_literal(e) {
        *e = whole_literal(span_of(e), n, w);
    }
}

/// The literal `n` of type `w` as the value it is.
fn whole_literal(span: TextSpan, n: i64, w: Option<Wide>) -> Expression {
    match w {
        Some(Wide::Unsigned) => call(span, "__to_uint64", vec![int(span, n)]),
        _ if !(i32::MIN as i64..=i32::MAX as i64).contains(&n) => call(span, "__to_int64", vec![int(span, n)]),
        _ => int(span, n),
    }
}

/// A literal standing alone (not beside a 64-bit value) in a program with
/// 64-bit types: a whole number beyond 32 bits is an INT64 (a UINT64 above
/// 9223372036854775807), so it shows and works in full — `PRINT
/// 3000000000`, `v = 3000000000 \ 2`; `&H` up to 8 digits keeps RapidQ's
/// 32-bit reading there (`&HFFFFFFFF` is -1), longer ones their 64 bits.
fn alone_literal(e: &mut Expression) {
    let span = span_of(e);
    match e {
        Expression::Literal(Literal { value: LiteralValue::Integer(n), .. }) if !(i32::MIN as i64..=i32::MAX as i64).contains(n) => {
            *e = whole_literal(span, *n, None);
        }
        Expression::Literal(Literal { value: LiteralValue::Integer(low), .. }) => {
            if let Some(full) = crate::wide_literals::full(span, *low).filter(|f| *f > u32::MAX as u64) {
                *e = whole_literal(span, full as i64, (full > i64::MAX as u64).then_some(Wide::Unsigned));
            }
        }
        Expression::Literal(Literal { value: LiteralValue::Float(_), .. }) => fix_literal(e),
        // (`-9223372036854775808`: the negated UINT64 literal above is the
        // smallest INT64)
        Expression::Unary(UnaryExpression { operator: UnaryOperator::Negate, operand, .. }) => {
            if let Expression::FunctionCall(fc) = operand.as_ref() {
                let made = matches!(fc.callee.as_ref(), Expression::Identifier(id) if id.name == "__to_uint64");
                if let (true, [Expression::Literal(Literal { value: LiteralValue::Integer(bits), .. })]) = (made, fc.args.as_slice()) {
                    let n = -((*bits as u64) as i128);
                    if n >= i64::MIN as i128 {
                        *e = whole_literal(span, n as i64, None);
                    }
                }
            }
        }
        _ => {}
    }
}

impl Pass<'_> {
    /// The 64-bit type of `e`: a variable, element, field or FUNCTION
    /// result of INT64 / UINT64 / UINT32, a conversion to one, arithmetic
    /// on one, a builtin that keeps its argument's type.
    fn wide_of(&self, e: &Expression) -> Option<Wide> {
        let info = self.wide?;
        match e {
            Expression::Identifier(id) => match self.lookup(&id.name) {
                Slot::Scalar(c) if self.in_create == 0 => wide_conv(c),
                Slot::Other => info.functions.get(&key(&id.name)).copied(),
                _ => None,
            },
            Expression::FunctionCall(FunctionCallExpression { callee, args, .. }) | Expression::ArrayAccess(ArrayAccessExpression { array: callee, indices: args, .. }) => {
                let Expression::Identifier(id) = callee.as_ref() else { return None };
                match key(&id.name).as_str() {
                    "__to_int64" | "__to_uint32" => Some(Wide::Signed),
                    "__to_uint64" => Some(Wide::Unsigned),
                    // (they keep their first argument's type)
                    "abs" | "int" | "fix" | "getbits" | "setbits" | "shl" | "shr" if self.lookup(&id.name) == Slot::Other && !info.functions.contains_key(&key(&id.name)) => args.first().and_then(|a| self.wide_of(a)),
                    _ => match self.lookup(&id.name) {
                        Slot::Array(c) => wide_conv(c),
                        Slot::Other => info.functions.get(&key(&id.name)).copied(),
                        Slot::Scalar(_) => None,
                    },
                }
            }
            Expression::Binary(b) => match b.operator {
                BinaryOperator::Add
                | BinaryOperator::Subtract
                | BinaryOperator::Multiply
                | BinaryOperator::IntegerDivide
                | BinaryOperator::Modulo
                | BinaryOperator::And
                | BinaryOperator::Or
                | BinaryOperator::Xor => {
                    let (l, r) = (self.wide_of(&b.left), self.wide_of(&b.right));
                    (l.is_some() || r.is_some()).then(|| combine(l.or(wide_literal(&b.left).and_then(|x| x.1)), r.or(wide_literal(&b.right).and_then(|x| x.1))))?
                }
                _ => None,
            },
            Expression::Unary(u) if matches!(u.operator, UnaryOperator::Negate | UnaryOperator::Positive | UnaryOperator::Not) => self.wide_of(&u.operand),
            _ => None,
        }
    }

    /// One expression (its parts already done, `walk_expressions_mut`):
    /// literals beside a 64-bit value read whole.
    fn widen_expr(&self, e: &mut Expression) {
        match e {
            Expression::Literal(_) | Expression::Unary(_) => alone_literal(e),
            Expression::Binary(b) => {
                // (`&` joins text: a literal there is what it is)
                if b.operator == BinaryOperator::Concat {
                    return;
                }
                if self.wide_of(&b.left).is_some() || self.wide_of(&b.right).is_some() {
                    fix_literal(&mut b.left);
                    fix_literal(&mut b.right);
                }
            }
            Expression::FunctionCall(fc) => {
                let Expression::Identifier(id) = fc.callee.as_ref() else { return };
                let name = key(&id.name);
                match name.as_str() {
                    // A store into a 64-bit one: its literal whole, and
                    // `VAL(x)`'s text read whole (VAL is a DOUBLE's).
                    "__to_int64" | "__to_uint64" | "__to_uint32" => {
                        if let Some(a) = fc.args.first_mut() {
                            if let Expression::FunctionCall(inner) = a {
                                if inner.args.len() == 1 && matches!(inner.callee.as_ref(), Expression::Identifier(v) if key(&v.name) == "val" && self.lookup(&v.name) == Slot::Other) {
                                    *a = inner.args.remove(0);
                                }
                            }
                            fix_literal(a);
                        }
                    }
                    // A call of a routine with 64-bit parameters.
                    _ => self.fix_arguments(&name, &mut fc.args),
                }
            }
            _ => {}
        }
    }

    /// The literals passed to a routine's 64-bit parameters, read whole.
    fn fix_arguments(&self, routine: &str, args: &mut [Expression]) {
        let Some(info) = self.wide else { return };
        let Some(params) = info.params.get(routine) else { return };
        if self.lookup(routine) != Slot::Other {
            return;
        }
        for (a, p) in args.iter_mut().zip(params) {
            if p.is_some() {
                fix_literal(a);
            }
        }
    }

    /// The 64-bit literal rules of `stmts` (see above).
    fn widen(&self, stmts: &mut [Statement]) {
        if self.wide.is_none() {
            return;
        }
        walk_expressions_mut(stmts, true, &mut |e| self.widen_expr(e));
        walk_statements_mut(stmts, &mut |s| match s {
            // (`CONST BIG AS INT64 = …`: the value of its type)
            Statement::Const(c) => {
                if let Some(conv) = c.declared_type.as_deref().filter(|t| wide_type(t).is_some()).and_then(|t| conv_for(t, None)) {
                    fix_literal(&mut c.value);
                    let value = take(&mut c.value);
                    c.value = convert_at(c.span, conv, value);
                }
            }
            Statement::For(f) => {
                if let Slot::Scalar(conv) = self.lookup(&f.variable) {
                    if wide_conv(conv).is_some() && self.in_create == 0 {
                        fix_literal(&mut f.end);
                        if let Some(step) = f.step.as_mut() {
                            fix_literal(step);
                        }
                    }
                }
            }
            Statement::SelectCase(c) => {
                if self.wide_of(&c.expression).is_some() {
                    for b in &mut c.cases {
                        for v in &mut b.values {
                            match v {
                                CaseValue::Value(e) | CaseValue::Is(_, e) | CaseValue::IsLogic(_, e, _) => fix_literal(e),
                                CaseValue::Range(a, z) => {
                                    fix_literal(a);
                                    fix_literal(z);
                                }
                            }
                        }
                    }
                }
            }
            Statement::Call(c) => {
                if let Expression::Identifier(id) = &c.callee {
                    let name = key(&id.name);
                    if name == "__data_add" {
                        // (DATA's whole numbers beyond 64 bits signed: a
                        // UINT64 each, for READ; the rest as RapidQ reads them)
                        for a in &mut c.args {
                            if matches!(a, Expression::Literal(Literal { value: LiteralValue::Float(_), .. })) {
                                fix_literal(a);
                            }
                        }
                    } else {
                        self.fix_arguments(&name, &mut c.args);
                    }
                }
            }
            _ => {}
        });
    }
}

fn take(e: &mut Expression) -> Expression {
    std::mem::replace(e, int(span_of(e), 0))
}

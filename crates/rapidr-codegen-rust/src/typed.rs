//! Typed variables in native builds: a SUB/FUNCTION's local or BYVAL
//! parameter declared with a numeric type (`DIM n AS LONG`, `x AS DOUBLE`)
//! is a Rust `i64` / `f64` instead of a `Value`, and so is such a variable of
//! the main program (in an atomic static, [`analyze_globals`]), and arithmetic, comparisons and FOR loops on such
//! variables compile to plain Rust — with exactly `Value`'s semantics
//! (wrapping integer `+ - *`, `/` and `\` by zero giving 0, comparisons as
//! floats), so a program prints the same as in the interpreter.
//!
//! It relies on `rapidr_ast::numeric`: every store into a typed variable
//! converts to its type (a FOR counter's start too), so the variable always
//! holds a number of its kind. A local is only typed when nothing else can
//! store into it: it isn't passed to a user SUB/FUNCTION (a possible BYREF),
//! its address isn't taken, it isn't used as an array or object, an integer
//! FOR counter steps by an integer, and the routine has no GOTO/GOSUB (their
//! state machine) or inline Rust.

use std::collections::{HashMap, HashSet};

use rapidr_ast::{BinaryOperator, Expression, LiteralValue, Parameter, Statement, UnaryOperator};

use crate::{strip_type_suffix, RustCodegen};

/// The kind of a typed variable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Kind {
    Byte,
    Word,
    Short,
    Long,
    Dword,
    Double,
}

impl Kind {
    pub(crate) fn of_type(type_name: &str) -> Option<Kind> {
        Some(match type_name.trim().to_ascii_uppercase().as_str() {
            "BYTE" => Kind::Byte,
            "WORD" => Kind::Word,
            "SHORT" => Kind::Short,
            "INTEGER" | "LONG" => Kind::Long,
            "DWORD" => Kind::Dword,
            // (SINGLE stays a `Value`: its 32-bit rounding is
            // `numeric::to_single`)
            "DOUBLE" => Kind::Double,
            _ => return None,
        })
    }

    fn of_conversion(builtin: &str) -> Option<Kind> {
        Some(match builtin.to_ascii_lowercase().as_str() {
            "__to_byte" => Kind::Byte,
            "__to_word" => Kind::Word,
            "__to_short" => Kind::Short,
            "__to_long" => Kind::Long,
            "__to_dword" => Kind::Dword,
            "__to_double" => Kind::Double,
            _ => return None,
        })
    }

    fn ty(self) -> Ty {
        if self == Kind::Double {
            Ty::Float
        } else {
            Ty::Int
        }
    }

    /// `rapidr_value::numeric::NumKind` for runtime calls.
    pub(crate) fn runtime(self) -> &'static str {
        match self {
            Kind::Byte => "numeric::NumKind::Byte",
            Kind::Word => "numeric::NumKind::Word",
            Kind::Short => "numeric::NumKind::Short",
            Kind::Long => "numeric::NumKind::Long",
            Kind::Dword => "numeric::NumKind::Dword",
            Kind::Double => "numeric::NumKind::Double",
        }
    }

    /// A 32-bit `i64` expression wrapped to this integer kind's width
    /// (INTEGER, LONG and DWORD are all 32-bit signed in RapidQ).
    fn wrap(self, code: &str) -> String {
        let via = match self {
            Kind::Byte => "u8",
            Kind::Word => "u16",
            Kind::Short => "i16",
            Kind::Long | Kind::Dword => return code.to_string(),
            Kind::Double => return code.to_string(),
        };
        format!("(({code}) as {via} as i64)")
    }

    pub(crate) fn rust_type(self) -> &'static str {
        if self == Kind::Double {
            "f64"
        } else {
            "i64"
        }
    }

    pub(crate) fn zero(self) -> &'static str {
        if self == Kind::Double {
            "0.0_f64"
        } else {
            "0_i64"
        }
    }
}

/// The Rust type of a typed expression.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Ty {
    Int,
    Float,
    /// A comparison or logical result (`Value::Boolean`).
    Bool,
}

fn key(name: &str) -> String {
    strip_type_suffix(name).to_lowercase()
}

/// Routines whose arguments may be passed BYREF or by address.
const ADDRESS_TAKERS: &[&str] = &["varptr", "varptr$", "callfunc", "codeptr", "callback", "swap", "bind"];

fn is_int_literal(e: &Expression) -> bool {
    match e {
        Expression::Literal(l) => matches!(l.value, LiteralValue::Integer(_)),
        Expression::Unary(u) if matches!(u.operator, UnaryOperator::Negate | UnaryOperator::Positive) => is_int_literal(&u.operand),
        _ => false,
    }
}

/// What a body does with its variables that stops them being typed.
struct Uses {
    /// GOTO / GOSUB / labels / inline Rust: nothing in it is typed.
    blocked: bool,
    /// Passed to a user SUB/FUNCTION or an address taker, used as an array,
    /// called, used as an object or with `@`.
    excluded: HashSet<String>,
    /// FOR counters and whether they step by an integer.
    for_steps: Vec<(String, bool)>,
}

fn uses(body: &[Statement], is_user_routine: &dyn Fn(&str) -> bool) -> Uses {
    let mut blocked = false;
    let mut excluded: HashSet<String> = HashSet::new();
    let mut for_steps: Vec<(String, bool)> = Vec::new();
    let mut on_expr_excluded: HashSet<String> = HashSet::new();
    let exclude_bare_args = |args: &[Expression], excluded: &mut HashSet<String>| {
        for a in args {
            if let Expression::Identifier(id) = a {
                excluded.insert(key(&id.name));
            }
        }
    };
    rapidr_ast::walk(
        body,
        &mut |s| match s {
            Statement::Goto(_) | Statement::Gosub(_) | Statement::Label(_) | Statement::RustBlock(_) => blocked = true,
            Statement::For(f) => for_steps.push((key(&f.variable), f.step.as_ref().is_none_or(is_int_literal))),
            Statement::Call(c) => {
                if let Expression::Identifier(id) = &c.callee {
                    let name = key(&id.name);
                    if is_user_routine(&name) || ADDRESS_TAKERS.contains(&name.as_str()) {
                        exclude_bare_args(&c.args, &mut excluded);
                    }
                }
            }
            _ => {}
        },
        &mut |e| match e {
            Expression::FunctionCall(fc) => {
                if let Expression::Identifier(id) = fc.callee.as_ref() {
                    let name = key(&id.name);
                    if is_user_routine(&name) || ADDRESS_TAKERS.contains(&name.as_str()) {
                        for a in &fc.args {
                            if let Expression::Identifier(arg) = a {
                                on_expr_excluded.insert(key(&arg.name));
                            }
                        }
                    }
                    // `n(1)`: used as an array or called.
                    on_expr_excluded.insert(name);
                }
            }
            Expression::ArrayAccess(a) => {
                if let Expression::Identifier(id) = a.array.as_ref() {
                    on_expr_excluded.insert(key(&id.name));
                }
            }
            Expression::Unary(u) if u.operator == UnaryOperator::Ref => {
                if let Expression::Identifier(id) = u.operand.as_ref() {
                    on_expr_excluded.insert(key(&id.name));
                }
            }
            Expression::MemberAccess(m) => {
                if let Expression::Identifier(id) = m.object.as_ref() {
                    on_expr_excluded.insert(key(&id.name));
                }
            }
            _ => {}
        },
    );
    excluded.extend(on_expr_excluded);
    Uses { blocked, excluded, for_steps }
}

/// Drops the integer FOR counters that step by a fraction (it makes them
/// floats, as `Value` addition does).
fn drop_fractional_counters(typed: &mut HashMap<String, Kind>, for_steps: &[(String, bool)]) {
    for (var, int_step) in for_steps {
        if !int_step && typed.get(var).is_some_and(|k| *k != Kind::Double) {
            typed.remove(var);
        }
    }
}

/// The locals — and BYVAL parameters — of a SUB/FUNCTION that can be typed
/// (see the module docs). A parameter is typed from the `Value` passed in.
pub(crate) fn analyze(params: &[Parameter], body: &[Statement], function: Option<&str>, is_user_routine: &dyn Fn(&str) -> bool) -> HashMap<String, Kind> {
    let mut declared: HashMap<String, Option<Kind>> = HashMap::new();
    let mut excluded: HashSet<String> = HashSet::new();
    for p in params {
        match Kind::of_type(&p.type_name).filter(|_| !p.by_ref && !p.is_array) {
            Some(k) => {
                declared.insert(key(&p.name), Some(k));
            }
            None => {
                excluded.insert(key(&p.name));
            }
        }
    }
    excluded.insert("result".into());
    if let Some(f) = function {
        excluded.insert(key(f));
    }
    rapidr_ast::walk(
        body,
        &mut |s| {
            if let Statement::Dim(d) = s {
                for v in &d.declarators {
                    let kind = if d.is_static || d.is_redim || !v.dimensions.is_empty() { None } else { Kind::of_type(&d.type_name) };
                    let entry = declared.entry(key(&v.name)).or_insert(kind);
                    if *entry != kind {
                        *entry = None;
                    }
                }
            }
        },
        &mut |_| {},
    );
    let u = uses(body, is_user_routine);
    if u.blocked {
        return HashMap::new();
    }
    excluded.extend(u.excluded);
    let mut typed: HashMap<String, Kind> = declared.into_iter().filter_map(|(n, k)| Some((n, k?))).filter(|(n, _)| !excluded.contains(n)).collect();
    drop_fractional_counters(&mut typed, &u.for_steps);
    typed
}

/// The main program's variables that can be typed: declared once with a
/// numeric type (`DIM n AS LONG` outside any SUB), and used nowhere — in
/// the main program or a SUB/FUNCTION that doesn't declare its own — in a
/// way that could store something else into them (see the module docs). A
/// GOTO / GOSUB doesn't matter here: the variable lives in a static, not in
/// a routine's state machine. Inline Rust anywhere turns this off (it may
/// use the variables' `Value` slots).
pub(crate) fn analyze_globals(program: &[Statement], is_user_routine: &dyn Fn(&str) -> bool) -> HashMap<String, Kind> {
    let mut declared: HashMap<String, Option<Kind>> = HashMap::new();
    let mut excluded: HashSet<String> = HashSet::new();
    let main: Vec<Statement> =
        program.iter().filter(|s| !matches!(s, Statement::Subroutine(_) | Statement::Function(_))).cloned().collect();
    let mut inline_rust = false;
    rapidr_ast::walk(
        program,
        &mut |s| {
            if matches!(s, Statement::RustBlock(_)) {
                inline_rust = true;
            }
        },
        &mut |_| {},
    );
    if inline_rust {
        return HashMap::new();
    }
    rapidr_ast::walk(
        &main,
        &mut |s| match s {
            Statement::Dim(d) => {
                for v in &d.declarators {
                    let kind = if d.is_static || d.is_redim || !v.dimensions.is_empty() { None } else { Kind::of_type(&d.type_name) };
                    let entry = declared.entry(key(&v.name)).or_insert(kind);
                    if *entry != kind {
                        *entry = None;
                    }
                }
            }
            Statement::Const(c) => {
                excluded.insert(key(&c.name));
            }
            _ => {}
        },
        &mut |_| {},
    );
    let mut for_steps = Vec::new();
    let main_uses = uses(&main, is_user_routine);
    excluded.extend(main_uses.excluded);
    for_steps.extend(main_uses.for_steps);
    for s in program {
        let (params, body) = match s {
            Statement::Subroutine(sub) => (&sub.params, &sub.body),
            Statement::Function(f) => (&f.params, &f.body),
            _ => continue,
        };
        // Names the routine declares itself are its own, not the globals.
        let own = crate::shadowing_names(params, body);
        let u = uses(body, is_user_routine);
        excluded.extend(u.excluded.into_iter().filter(|n| !own.contains(n)));
        for_steps.extend(u.for_steps.into_iter().filter(|(n, _)| !own.contains(n)));
    }
    let mut typed: HashMap<String, Kind> = declared.into_iter().filter_map(|(n, k)| Some((n, k?))).filter(|(n, _)| !excluded.contains(n)).collect();
    drop_fractional_counters(&mut typed, &for_steps);
    typed
}

/// The static holding typed global `name` (an `AtomicI64`, or an `AtomicU64`
/// with a DOUBLE's bits).
pub(crate) fn global_static(name: &str) -> String {
    format!("TG_{}", crate::to_snake(&key(name)).trim_end_matches('_').to_uppercase())
}

impl RustCodegen {
    /// The typed local `name`, if it is one.
    pub(crate) fn typed_local(&self, name: &str) -> Option<Kind> {
        if self.typed_locals.is_empty() {
            return None;
        }
        self.typed_locals.get(&key(name)).copied()
    }

    /// A typed variable `name` — a typed local, or a typed global the
    /// current routine doesn't shadow — as its kind and the Rust code
    /// reading it.
    pub(crate) fn typed_var(&self, name: &str) -> Option<(Kind, String)> {
        if let Some(k) = self.typed_local(name) {
            return Some((k, crate::to_snake(&strip_type_suffix(name))));
        }
        // (Inside CREATE a value like `Width = bx` still reads the variable;
        // only an assignment's target there can be the component's property,
        // which the assignment checks itself.)
        if self.typed_globals.is_empty() {
            return None;
        }
        let k = key(name);
        if self.shadowed.contains(&k) {
            return None;
        }
        let kind = *self.typed_globals.get(&k)?;
        let load = format!("{}.load(std::sync::atomic::Ordering::Relaxed)", global_static(&k));
        Some((kind, if kind == Kind::Double { format!("f64::from_bits({load})") } else { load }))
    }

    /// The Rust statement storing `value` (code of the variable's Rust type)
    /// into typed variable `name`.
    pub(crate) fn typed_write(&self, name: &str, value: &str) -> String {
        if self.typed_local(name).is_some() {
            format!("{} = {value};", crate::to_snake(&strip_type_suffix(name)))
        } else if self.typed_globals.get(&key(name)) == Some(&Kind::Double) {
            format!("{}.store(({value}).to_bits(), std::sync::atomic::Ordering::Relaxed);", global_static(name))
        } else {
            format!("{}.store({value}, std::sync::atomic::Ordering::Relaxed);", global_static(name))
        }
    }

    /// `e` as plain Rust (`i64` / `f64` / `bool`) when every part of it is
    /// typed: typed variables, number literals, and the operators below.
    pub(crate) fn typed_expr(&self, e: &Expression) -> Option<(String, Ty)> {
        match e {
            Expression::Literal(l) => match l.value {
                LiteralValue::Integer(n) if n == i64::MIN => Some(("i64::MIN".into(), Ty::Int)),
                LiteralValue::Integer(n) if n < 0 => Some((format!("(-{}_i64)", -n), Ty::Int)),
                LiteralValue::Integer(n) => Some((format!("{n}_i64"), Ty::Int)),
                LiteralValue::Float(f) if f.is_finite() => Some((format!("({f:?}_f64)"), Ty::Float)),
                _ => None,
            },
            Expression::Identifier(id) => {
                let (k, code) = self.typed_var(&id.name)?;
                Some((code, k.ty()))
            }
            Expression::Unary(u) => {
                let (a, t) = self.typed_expr(&u.operand)?;
                match (u.operator, t) {
                    (UnaryOperator::Positive, _) => Some((a, t)),
                    (UnaryOperator::Negate, Ty::Int) => Some((format!("({a}).wrapping_neg()"), Ty::Int)),
                    (UnaryOperator::Negate, Ty::Float) => Some((format!("(-({a}))"), Ty::Float)),
                    (UnaryOperator::Not, Ty::Int) => Some((format!("(!numeric::int32_of({a}))"), t)),
                    (UnaryOperator::Not, Ty::Bool) => Some((format!("(!({a}))"), t)),
                    _ => None,
                }
            }
            Expression::Binary(b) => {
                let (a, ta) = self.typed_expr(&b.left)?;
                let (c, tc) = self.typed_expr(&b.right)?;
                let numeric = ta != Ty::Bool && tc != Ty::Bool;
                let both_int = ta == Ty::Int && tc == Ty::Int;
                let fa = || if ta == Ty::Int { format!("(({a}) as f64)") } else { format!("({a})") };
                let fc = || if tc == Ty::Int { format!("(({c}) as f64)") } else { format!("({c})") };
                // (`\`'s operands: RapidQ rounds a real as its CINT does)
                let ia = || if ta == Ty::Int { format!("({a})") } else { format!("numeric::idiv_operand({a})") };
                let ic = || if tc == Ty::Int { format!("({c})") } else { format!("numeric::idiv_operand({c})") };
                use BinaryOperator as B;
                Some(match b.operator {
                    B::Add | B::Subtract | B::Multiply if both_int => {
                        let m = match b.operator {
                            B::Add => "wrapping_add",
                            B::Subtract => "wrapping_sub",
                            _ => "wrapping_mul",
                        };
                        (format!("({a}).{m}({c})"), Ty::Int)
                    }
                    B::Add | B::Subtract | B::Multiply if numeric => {
                        let op = match b.operator {
                            B::Add => "+",
                            B::Subtract => "-",
                            _ => "*",
                        };
                        (format!("({} {op} {})", fa(), fc()), Ty::Float)
                    }
                    B::Divide if numeric => (format!("numeric::fdiv({}, {})", fa(), fc()), Ty::Float),
                    B::IntegerDivide if numeric => (format!("numeric::idiv({}, {})", ia(), ic()), Ty::Int),
                    B::Modulo if both_int => (format!("numeric::imod({a}, {c})"), Ty::Int),
                    B::Modulo if numeric => (format!("numeric::fmod({}, {})", fa(), fc()), Ty::Int),
                    B::Power if numeric => (format!("{}.powf({})", fa(), fc()), Ty::Float),
                    // (integers compare plainly; floats as `Value` does,
                    // a NaN as RapidQ's FCOM leaves it — numeric::eq & co.)
                    B::Equal if both_int => (format!("({} == {})", fa(), fc()), Ty::Bool),
                    B::NotEqual if both_int => (format!("({} != {})", fa(), fc()), Ty::Bool),
                    B::LessThan if both_int => (format!("({} < {})", fa(), fc()), Ty::Bool),
                    B::GreaterThan if both_int => (format!("({} > {})", fa(), fc()), Ty::Bool),
                    B::Equal if numeric => (format!("numeric::eq({}, {})", fa(), fc()), Ty::Bool),
                    B::NotEqual if numeric => (format!("numeric::ne({}, {})", fa(), fc()), Ty::Bool),
                    B::LessThan if numeric => (format!("numeric::lt({}, {})", fa(), fc()), Ty::Bool),
                    B::GreaterThan if numeric => (format!("numeric::gt({}, {})", fa(), fc()), Ty::Bool),
                    // Plain `<=` / `>=` where NaN can't occur; `Value`'s
                    // NaN-as-equal rule otherwise.
                    B::LessThanOrEqual if both_int => (format!("({} <= {})", fa(), fc()), Ty::Bool),
                    B::GreaterThanOrEqual if both_int => (format!("({} >= {})", fa(), fc()), Ty::Bool),
                    B::LessThanOrEqual if numeric => (format!("numeric::le({}, {})", fa(), fc()), Ty::Bool),
                    B::GreaterThanOrEqual if numeric => (format!("numeric::ge({}, {})", fa(), fc()), Ty::Bool),
                    B::And | B::Or | B::Xor if both_int || (ta == Ty::Bool && tc == Ty::Bool) => {
                        let op = match b.operator {
                            B::And => "&",
                            B::Or => "|",
                            _ => "^",
                        };
                        if ta == Ty::Int {
                            // (on RapidQ's 32-bit integers, as `Value`'s)
                            (format!("(numeric::int32_of({a}) {op} numeric::int32_of({c}))"), ta)
                        } else {
                            (format!("(({a}) {op} ({c}))"), ta)
                        }
                    }
                    _ => return None,
                })
            }
            // `__to_long(e)` & co. (rapidr_ast::numeric) on a typed value.
            Expression::FunctionCall(fc) if fc.args.len() == 1 => {
                let Expression::Identifier(id) = fc.callee.as_ref() else { return None };
                // A BYVAL integer parameter's rounding (half to even).
                if id.name.eq_ignore_ascii_case("__arg_round") {
                    let (code, ty) = self.typed_expr(&fc.args[0])?;
                    return Some(if ty == Ty::Float { (format!("({code}).round_ties_even()"), ty) } else { (code, ty) });
                }
                let kind = Kind::of_conversion(&id.name)?;
                let typed = self.typed_expr(&fc.args[0])?;
                Some((convert_typed(kind, typed), kind.ty()))
            }
            _ => None,
        }
    }

    /// `e` as a `Value`, when it's typed (so the arithmetic is native).
    pub(crate) fn boxed_typed(&self, e: &Expression) -> Option<String> {
        if matches!(e, Expression::Literal(_)) || (self.typed_locals.is_empty() && self.typed_globals.is_empty() && !matches!(e, Expression::Binary(_))) {
            return None;
        }
        let (code, ty) = self.typed_expr(e)?;
        Some(match ty {
            Ty::Int => format!("v_int({code})"),
            Ty::Float => format!("v_dbl({code})"),
            Ty::Bool => format!("v_bool({code})"),
        })
    }

    /// A condition as a Rust `bool`.
    pub(crate) fn cond_to_string(&self, e: &Expression) -> String {
        match self.typed_expr(e) {
            Some((code, Ty::Bool)) => code,
            Some((code, Ty::Int)) => format!("(({code}) != 0)"),
            Some((code, Ty::Float)) => format!("(({code}) != 0.0)"),
            None => format!("({}).to_bool()", self.expr_to_string(e)),
        }
    }

    /// The Rust expression storing `value` into a typed local of `kind`
    /// (`value` may already be the conversion `rapidr_ast::numeric` added).
    pub(crate) fn typed_store(&self, kind: Kind, value: &Expression) -> String {
        let inner = match value {
            Expression::FunctionCall(fc) if fc.args.len() == 1 => match fc.callee.as_ref() {
                Expression::Identifier(id) if Kind::of_conversion(&id.name) == Some(kind) => &fc.args[0],
                _ => value,
            },
            _ => value,
        };
        if let Some(typed) = self.typed_expr(inner) {
            return convert_typed(kind, typed);
        }
        let v = self.expr_to_string(inner);
        if kind == Kind::Double {
            format!("numeric::double_of(&{v})")
        } else {
            format!("numeric::int_of(&{v}, {})", kind.runtime())
        }
    }

    /// A value for a typed FOR bound or step: `f64` (or `i64` when `int`).
    pub(crate) fn typed_number(&self, e: &Expression, int: bool) -> String {
        match (self.typed_expr(e), int) {
            (Some((code, Ty::Int)), true) => code,
            (Some((code, Ty::Int)), false) => format!("(({code}) as f64)"),
            (Some((code, Ty::Float)), false) => code,
            _ if int => format!("({}).to_i64()", self.expr_to_string(e)),
            _ => format!("({}).to_f64()", self.expr_to_string(e)),
        }
    }
}

impl RustCodegen {
    /// `FOR n = a TO b [STEP s]` over a typed local, as the `Value` loop
    /// does it: `b` and `s` evaluated once, the direction from the sign of
    /// `s`, comparisons as floats, `n` stepped without conversion.
    pub(crate) fn emit_typed_for(&mut self, f: &rapidr_ast::ForStatement, kind: Kind) {
        use std::fmt::Write;
        let var = crate::to_snake(&strip_type_suffix(&f.variable));
        let (_, read) = self.typed_var(&f.variable).expect("a typed FOR counter");
        let int = kind != Kind::Double;
        let end = self.typed_number(&f.end, false);
        let step = match &f.step {
            Some(e) => self.typed_number(e, int),
            None if int => "1_i64".to_string(),
            None => "1.0_f64".to_string(),
        };
        let start = self.typed_store(kind, &f.start);
        let lbl = self.open_loop("FOR");
        let (end_tmp, step_tmp) = (format!("__for_end_{var}"), format!("__for_step_{var}"));
        self.write_indent();
        let _ = writeln!(self.output, "let {end_tmp}: f64 = {end};");
        self.write_indent();
        let _ = writeln!(self.output, "let {step_tmp}: {} = {step};", kind.rust_type());
        self.write_indent();
        let _ = writeln!(self.output, "{}", self.typed_write(&f.variable, &start));
        let (cur, up) = if int {
            (format!("({read} as f64)"), format!("{step_tmp} >= 0"))
        } else {
            (read.clone(), format!("numeric::ge({step_tmp}, 0.0)"))
        };
        self.write_indent();
        let _ = writeln!(
            self.output,
            "{lbl}: while (if {up} {{ numeric::le({cur}, {end_tmp}) }} else {{ numeric::ge({cur}, {end_tmp}) }}) {{"
        );
        self.indent += 1;
        for s in &f.body {
            self.emit_statement(s);
        }
        self.write_indent();
        let next = if int { format!("{read}.wrapping_add({step_tmp})") } else { format!("{read} + {step_tmp}") };
        let _ = writeln!(self.output, "{}", self.typed_write(&f.variable, &next));
        self.indent -= 1;
        self.line("}");
        self.loop_labels.pop();
    }
}

/// A typed value converted for a store into `kind` (truncated, beyond 32
/// bits -2147483648, wrapped to the width — `rapidr_value::numeric`).
fn convert_typed(kind: Kind, (code, ty): (String, Ty)) -> String {
    match (kind, ty) {
        (Kind::Double, Ty::Int) => format!("(({code}) as f64)"),
        (Kind::Double, Ty::Float) => code,
        (Kind::Double, Ty::Bool) => format!("(if {code} {{ -1.0_f64 }} else {{ 0.0_f64 }})"),
        (k, Ty::Int) => k.wrap(&format!("numeric::int32_of({code})")),
        (k, Ty::Float) => k.wrap(&format!("numeric::trunc_to_int({code})")),
        (k, Ty::Bool) => k.wrap(&format!("(if {code} {{ -1_i64 }} else {{ 0_i64 }})")),
    }
}

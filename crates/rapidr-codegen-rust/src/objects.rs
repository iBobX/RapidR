//! Object-oriented TYPEs in native builds (RapidQ manual ch. 10).
//!
//! Before code generation, this pass rewrites objects into plain procedural
//! BASIC that the Rust backend already compiles, with the same model as the
//! bytecode compiler (`rapidr-bcgen`), so both backends behave alike:
//!
//! * an instance is its id (`DIM c AS TCounter` → `c` holds `"c"`); its
//!   fields live in the runtime's object registry (`rp_comp_get/set`), the
//!   same one components use, so a TYPE that EXTENDS QFORM *is* the form;
//! * each method becomes a routine `Type__Method(This, …)`; calls resolve by
//!   the declared type, the nearest ancestor defining the method winning;
//! * `Type__init(This)` sets every field (base TYPE first), gives fields of
//!   TYPE or component type their own object (`<id>.<field>`, composition),
//!   then runs the CONSTRUCTORs, base first;
//! * inside a TYPE's code, `This`, `Me`, the type's own name (or an
//!   ancestor's) and a leading `.` stand for the instance, and bare field
//!   names are its fields; assigning a field with a PROPERTY SET calls the
//!   setter (except inside the setter itself).
//!
//! EVENT blocks become routines `Type__ev<n>(This, …)`; every instance gets
//! a small handler `__ev_<instance>_<n>` bound to its own component (or to
//! its field's, for `EVENT Panel.OnClick`), so `This` is that instance.
//! `CREATE x AS TType` sets the instance up, then applies the block's
//! settings to it. An array field of objects (`image(10) AS QBITMAP`) holds
//! the ids `<instance>.image(i)`; `DIM a(n) AS TType` is a plain array, as
//! in the interpreter.

use std::collections::{HashMap, HashSet};

use rapidr_ast::*;
use rapidr_diagnostics::TextSpan;

/// A TYPE the pass lowers to objects.
struct TypeDef {
    name: String,
    extends: Option<String>,
    fields: Vec<TypeField>,
    /// Method name (lowercase) → (name as written, is FUNCTION).
    methods: HashMap<String, (String, bool)>,
    has_ctor: bool,
    /// EVENT blocks: (event, parameter count); handler `Type__ev<i>`.
    events: Vec<(String, usize)>,
}

struct Types {
    map: HashMap<String, TypeDef>,
}

fn key(name: &str) -> String {
    name.to_ascii_lowercase()
}

/// `Type__Method`: the routine a method compiles to.
fn mangle(type_name: &str, member: &str) -> String {
    format!("{type_name}__{member}")
}

impl Types {
    fn get(&self, t: &str) -> Option<&TypeDef> {
        self.map.get(&key(t))
    }

    /// The TYPE and its ancestors that are TYPEs, base first.
    fn chain(&self, t: &str) -> Vec<&TypeDef> {
        let mut out = Vec::new();
        let mut cur = self.get(t);
        while let Some(def) = cur {
            if out.len() > 32 {
                break; // cyclic EXTENDS
            }
            out.push(def);
            cur = def.extends.as_deref().and_then(|e| self.get(e));
        }
        out.reverse();
        out
    }

    /// The component the TYPE (or an ancestor) extends, e.g. RFORM.
    fn base_component(&self, t: &str) -> Option<String> {
        self.chain(t)
            .iter()
            .filter_map(|d| d.extends.as_deref())
            .map(canonical_type_name)
            .find(|e| is_component_type_name(e))
            .map(|e| e.to_ascii_uppercase())
    }

    /// (defining TYPE, method as written, is FUNCTION).
    fn find_method(&self, t: &str, m: &str) -> Option<(String, String, bool)> {
        self.chain(t)
            .iter()
            .rev()
            .find_map(|d| d.methods.get(&key(m)).map(|(name, f)| (d.name.clone(), name.clone(), *f)))
    }

    fn field(&self, t: &str, f: &str) -> Option<&TypeField> {
        self.chain(t).into_iter().rev().find_map(|d| d.fields.iter().find(|x| x.name.eq_ignore_ascii_case(f)))
    }

    /// Type of a (non-array) field that holds an object: a lowered TYPE or a
    /// component.
    fn field_object_type(&self, t: &str, f: &str) -> Option<String> {
        let field = self.field(t, f)?;
        if field.array_size.is_some() {
            return None;
        }
        object_kind(self, &field.type_name)
    }

    fn setter(&self, t: &str, f: &str) -> Option<(String, String)> {
        let setter = self.field(t, f)?.setter.clone()?;
        self.find_method(t, &setter).map(|(def, name, _)| (def, name))
    }

    /// Whether `name` is the TYPE `t` or one of its ancestors.
    fn is_self_or_ancestor(&self, t: &str, name: &str) -> bool {
        self.chain(t).iter().any(|d| d.name.eq_ignore_ascii_case(name))
    }
}

/// A lowered TYPE's name, or a component type (canonical, uppercase).
fn object_kind(types: &Types, type_name: &str) -> Option<String> {
    if let Some(d) = types.get(type_name) {
        return Some(d.name.clone());
    }
    let c = canonical_type_name(type_name);
    is_component_type_name(&c).then(|| c.to_ascii_uppercase())
}

/// Whether a TYPE has object-oriented parts (then it's lowered, together
/// with the TYPEs it extends or holds).
fn is_oop(t: &TypeStatement) -> bool {
    t.extends.is_some()
        || !t.methods.is_empty()
        || !t.constructor.is_empty()
        || !t.events.is_empty()
        || t.fields.iter().any(|f| f.setter.is_some())
}

fn collect_types(program: &Program) -> Types {
    let all: HashMap<String, &TypeStatement> = program
        .statements
        .iter()
        .filter_map(|s| match s {
            Statement::Type(t) => Some((key(&t.name), t)),
            _ => None,
        })
        .collect();
    // OOP TYPEs, plus the TYPEs they extend or hold (objects too).
    let mut lowered: HashSet<String> = all.iter().filter(|(_, t)| is_oop(t)).map(|(k, _)| k.clone()).collect();
    loop {
        let mut more = Vec::new();
        for k in &lowered {
            let t = all[k];
            let refs = t.extends.iter().cloned().chain(t.fields.iter().map(|f| f.type_name.clone()));
            for r in refs {
                if all.contains_key(&key(&r)) && !lowered.contains(&key(&r)) {
                    more.push(key(&r));
                }
            }
        }
        if more.is_empty() {
            break;
        }
        lowered.extend(more);
    }
    let map = lowered
        .into_iter()
        .map(|k| {
            let t = all[&k];
            let methods = t
                .methods
                .iter()
                .filter_map(|m| match m {
                    Statement::Subroutine(s) => Some((key(&s.name), (s.name.clone(), false))),
                    Statement::Function(f) => Some((key(&f.name), (f.name.clone(), true))),
                    _ => None,
                })
                .collect();
            (
                k,
                TypeDef {
                    name: t.name.clone(),
                    extends: t.extends.clone(),
                    fields: t.fields.clone(),
                    methods,
                    has_ctor: !t.constructor.is_empty(),
                    events: t.events.iter().map(|e| (e.name.clone(), e.params.len())).collect(),
                },
            )
        })
        .collect();
    Types { map }
}

/// What this pass can't lower yet, as a message for the user: nothing
/// today; kept so `rapidr build` has one place to report gaps.
pub fn unsupported(_program: &Program) -> Option<String> {
    None
}

// ---------- building blocks ----------

fn sp() -> TextSpan {
    TextSpan::default()
}

fn ident(name: &str) -> Expression {
    Expression::Identifier(Identifier { span: sp(), name: name.into() })
}

fn text(s: &str) -> Expression {
    Expression::Literal(Literal { span: sp(), value: LiteralValue::String(s.into()) })
}

fn call(name: &str, args: Vec<Expression>) -> Expression {
    Expression::FunctionCall(FunctionCallExpression { span: sp(), callee: Box::new(ident(name)), args })
}

fn call_stmt(name: &str, args: Vec<Expression>) -> Statement {
    Statement::Call(CallStatement { span: sp(), callee: ident(name), args })
}

/// `This + "." + field`: the id of an instance's sub-object.
fn sub_id(obj: Expression, field: &str) -> Expression {
    Expression::Binary(BinaryExpression {
        span: sp(),
        left: Box::new(obj),
        operator: BinaryOperator::Add,
        right: Box::new(text(&format!(".{field}"))),
    })
}

fn field_fill(type_name: &str) -> Expression {
    match type_name.to_ascii_uppercase().as_str() {
        "STRING" => text(""),
        "INTEGER" | "LONG" | "SHORT" | "BYTE" | "WORD" | "DWORD" | "SINGLE" | "DOUBLE" | "CURRENCY" => {
            Expression::Literal(Literal { span: sp(), value: LiteralValue::Integer(0) })
        }
        _ => call("__null", Vec::new()),
    }
}

// ---------- the rewrite ----------

/// Where the code being rewritten is.
#[derive(Clone, Default)]
struct Ctx {
    /// Object variables of the routine (params and DIMs), lowercase → TYPE.
    local_types: HashMap<String, String>,
    /// Every parameter and DIMmed local of the routine (lowercase).
    locals: HashSet<String>,
    in_main: bool,
    /// Inside a TYPE's method/CONSTRUCTOR: that TYPE.
    current_type: Option<String>,
    /// The method being rewritten: (name as written, routine name, is FUNCTION).
    current_method: Option<(String, String, bool)>,
}

struct Lowering<'a> {
    types: &'a Types,
    /// Object variables of the main program, lowercase → TYPE.
    global_types: HashMap<String, String>,
    /// Main-program variable and constant names (lowercase).
    globals: HashSet<String>,
    /// User SUB/FUNCTION names (lowercase).
    routines: HashSet<String>,
    ctx: Ctx,
    /// Generated routines (EVENT handler trampolines).
    extra: Vec<Statement>,
    defined_trampolines: HashSet<String>,
}

impl Lowering<'_> {
    fn is_local(&self, name: &str) -> bool {
        self.ctx.locals.contains(&key(name))
    }

    /// Inside a TYPE's code: `This`, `Me`, a leading `.` and the type's own
    /// (or an ancestor's) name stand for the instance.
    fn is_this(&self, name: &str) -> bool {
        let Some(t) = &self.ctx.current_type else { return false };
        let k = key(name);
        if matches!(k.as_str(), "this" | "me" | "_with_") {
            return true;
        }
        !self.is_local(name) && !self.globals.contains(&k) && self.types.is_self_or_ancestor(t, name)
    }

    /// A bare name inside a TYPE's code that's a member of the instance: a
    /// field, or a property of the component the TYPE extends.
    fn implicit_member(&self, name: &str) -> bool {
        let Some(t) = &self.ctx.current_type else { return false };
        if self.is_local(name) || matches!(key(name).as_str(), "this" | "me" | "true" | "false") {
            return false;
        }
        self.types.field(t, name).is_some()
            || (self.types.base_component(t).is_some()
                && !self.globals.contains(&key(name))
                && !self.routines.contains(&key(name))
                && super::builtin_function_call(&key(name), &[]).is_none())
    }

    fn var_type(&self, name: &str) -> Option<String> {
        if self.is_this(name) {
            return self.ctx.current_type.clone();
        }
        if let Some(t) = self.ctx.local_types.get(&key(name)) {
            return Some(t.clone());
        }
        if !self.ctx.in_main && self.is_local(name) {
            return None;
        }
        self.global_types.get(&key(name)).cloned()
    }

    /// The object type an expression holds: a lowered TYPE (by name) or a
    /// component type (uppercase), for fields reached through an object.
    fn object_type(&self, e: &Expression) -> Option<String> {
        match e {
            Expression::Identifier(id) => self.var_type(&id.name).or_else(|| {
                let t = self.ctx.current_type.as_ref()?;
                if self.is_local(&id.name) {
                    return None;
                }
                self.types.field_object_type(t, &id.name)
            }),
            Expression::MemberAccess(m) => {
                let t = self.object_type(&m.object)?;
                self.types.get(&t)?;
                self.types.field_object_type(&t, &m.member)
            }
            // `obj.image(i)` / `image(i)`: an element of an array field of objects.
            Expression::FunctionCall(fc) if fc.args.len() == 1 => match fc.callee.as_ref() {
                Expression::MemberAccess(m) => {
                    let t = self.object_type(&m.object)?;
                    self.array_field_type(&t, &m.member)
                }
                Expression::Identifier(id) if !self.is_local(&id.name) => {
                    let t = self.ctx.current_type.clone()?;
                    self.array_field_type(&t, &id.name)
                }
                _ => None,
            },
            _ => None,
        }
    }

    fn array_field_type(&self, t: &str, field: &str) -> Option<String> {
        let f = self.types.field(t, field)?;
        f.array_size.as_ref()?;
        object_kind(self.types, &f.type_name)
    }

    /// `x` becomes an instance of `t`: its id, set-up, and its EVENT
    /// handlers bound to it.
    fn instance(&mut self, name: &str, t: &str) -> Vec<Statement> {
        let mut out = vec![
            Statement::Assignment(AssignmentStatement { span: sp(), target: ident(name), value: text(name) }),
            call_stmt(&format!("{t}__init"), vec![text(name)]),
        ];
        for def in self.types.chain(t) {
            for (i, (event, n)) in def.events.iter().enumerate() {
                let (target, event_name) = match event.rsplit_once('.') {
                    Some((field, ev)) => (format!("{name}.{field}"), ev.to_string()),
                    None => (name.to_string(), event.clone()),
                };
                let tramp = format!("__ev_{}_{}_{i}", name.replace(['.', '(', ')'], "_"), def.name);
                if self.defined_trampolines.insert(key(&tramp)) {
                    let params: Vec<Parameter> = (0..*n)
                        .map(|k| Parameter { span: sp(), name: format!("arg{k}"), type_name: String::new(), by_ref: false, is_array: false })
                        .collect();
                    let args = std::iter::once(text(name)).chain((0..*n).map(|k| ident(&format!("arg{k}")))).collect();
                    self.extra.push(Statement::Subroutine(SubroutineStatement {
                        span: sp(),
                        name: tramp.clone(),
                        params,
                        body: vec![call_stmt(&format!("{}__ev{i}", def.name), args)],
                    }));
                }
                let bind = match n {
                    0 => "rp_bind_event".to_string(),
                    n => format!("rp_bind_event_{}", n.min(&5)),
                };
                out.push(Statement::RustBlock(RustBlockStatement {
                    span: sp(),
                    code: format!("{bind}(\"{}\", \"{}\", {});", target.to_lowercase(), event_name.to_lowercase(), super::to_snake(&tramp)),
                }));
            }
        }
        out
    }

    fn is_user_type(&self, t: &str) -> bool {
        self.types.get(t).is_some()
    }

    fn expr(&self, e: &Expression) -> Expression {
        match e {
            Expression::Identifier(id) => {
                if self.is_this(&id.name) {
                    return ident("This");
                }
                if let Some((name, routine, _)) = &self.ctx.current_method {
                    if id.name.eq_ignore_ascii_case(name) {
                        return ident(routine);
                    }
                }
                if self.implicit_member(&id.name) {
                    let t = self.ctx.current_type.clone().unwrap_or_default();
                    if let Some((def, m, true)) = self.types.find_method(&t, &id.name) {
                        return call(&mangle(&def, &m), vec![ident("This")]);
                    }
                    return call("__objget", vec![ident("This"), text(&id.name)]);
                }
                e.clone()
            }
            Expression::MemberAccess(m) => {
                if let Some(t) = self.object_type(&m.object) {
                    let o = self.expr(&m.object);
                    if self.is_user_type(&t) {
                        if let Some((def, name, _)) = self.types.find_method(&t, &m.member) {
                            return call(&mangle(&def, &name), vec![o]);
                        }
                    }
                    return call("__objget", vec![o, text(&m.member)]);
                }
                Expression::MemberAccess(MemberAccessExpression { span: m.span, object: Box::new(self.expr(&m.object)), member: m.member.clone() })
            }
            Expression::FunctionCall(fc) => {
                let args: Vec<Expression> = fc.args.iter().map(|a| self.expr(a)).collect();
                match fc.callee.as_ref() {
                    Expression::MemberAccess(m) => {
                        if let Some(t) = self.object_type(&m.object) {
                            let o = self.expr(&m.object);
                            if self.is_user_type(&t) {
                                if let Some((def, name, _)) = self.types.find_method(&t, &m.member) {
                                    return call(&mangle(&def, &name), std::iter::once(o).chain(args).collect());
                                }
                                if self.types.field(&t, &m.member).is_some_and(|f| f.array_size.is_some()) {
                                    return call("__objaget", [o, text(&m.member)].into_iter().chain(args).collect());
                                }
                            }
                            return call("__objcall", [o, text(&m.member)].into_iter().chain(args).collect());
                        }
                    }
                    Expression::Identifier(id) if self.ctx.current_type.is_some() && !self.is_local(&id.name) => {
                        let t = self.ctx.current_type.clone().unwrap_or_default();
                        if let Some((def, name, _)) = self.types.find_method(&t, &id.name) {
                            return call(&mangle(&def, &name), std::iter::once(ident("This")).chain(args).collect());
                        }
                        if self.types.field(&t, &id.name).is_some_and(|f| f.array_size.is_some()) {
                            return call("__objaget", [ident("This"), text(&id.name)].into_iter().chain(args).collect());
                        }
                    }
                    _ => {}
                }
                let callee = match fc.callee.as_ref() {
                    // A routine name stays a name (not a field read).
                    Expression::Identifier(_) => fc.callee.as_ref().clone(),
                    other => self.expr(other),
                };
                Expression::FunctionCall(FunctionCallExpression { span: fc.span, callee: Box::new(callee), args })
            }
            Expression::ArrayAccess(a) => Expression::ArrayAccess(ArrayAccessExpression {
                span: a.span,
                array: a.array.clone(),
                indices: a.indices.iter().map(|i| self.expr(i)).collect(),
            }),
            Expression::Binary(b) => Expression::Binary(BinaryExpression {
                span: b.span,
                left: Box::new(self.expr(&b.left)),
                operator: b.operator,
                right: Box::new(self.expr(&b.right)),
            }),
            Expression::Unary(u) => Expression::Unary(UnaryExpression { span: u.span, operator: u.operator, operand: Box::new(self.expr(&u.operand)) }),
            Expression::MethodCall(mc) => Expression::MethodCall(MethodCallExpression {
                span: mc.span,
                object: Box::new(self.expr(&mc.object)),
                method: mc.method.clone(),
                args: mc.args.iter().map(|a| self.expr(a)).collect(),
            }),
            Expression::Literal(_) => e.clone(),
        }
    }

    fn body(&mut self, stmts: &[Statement]) -> Vec<Statement> {
        stmts.iter().flat_map(|s| self.stmt(s)).collect()
    }

    /// `obj.field = v` / `field = v`: the setter, or a registry store.
    fn store_field(&self, obj: Expression, type_name: &str, field: &str, value: Expression) -> Statement {
        if let Some((def, setter)) = self.types.setter(type_name, field) {
            let in_setter = self.ctx.current_method.as_ref().is_some_and(|(m, _, _)| m.eq_ignore_ascii_case(&setter));
            if !in_setter {
                return call_stmt(&mangle(&def, &setter), vec![obj, value]);
            }
        }
        call_stmt("__objset", vec![obj, text(field), value])
    }

    fn assignment(&mut self, a: &AssignmentStatement) -> Vec<Statement> {
        let value = self.expr(&a.value);
        match &a.target {
            Expression::MemberAccess(m) => {
                if let Some(t) = self.object_type(&m.object) {
                    let o = self.expr(&m.object);
                    if self.is_user_type(&t) {
                        return vec![self.store_field(o, &t, &m.member, value)];
                    }
                    return vec![call_stmt("__objset", vec![o, text(&m.member), value])];
                }
            }
            Expression::FunctionCall(fc) => {
                let idx: Vec<Expression> = fc.args.iter().map(|x| self.expr(x)).collect();
                match fc.callee.as_ref() {
                    Expression::MemberAccess(m) => {
                        if let Some(t) = self.object_type(&m.object) {
                            if self.is_user_type(&t) && self.types.field(&t, &m.member).is_some_and(|f| f.array_size.is_some()) {
                                let o = self.expr(&m.object);
                                let args = [o, text(&m.member)].into_iter().chain(idx).chain([value]).collect();
                                return vec![call_stmt("__objaset", args)];
                            }
                        }
                    }
                    Expression::Identifier(id) if self.ctx.current_type.is_some() && !self.is_local(&id.name) => {
                        let t = self.ctx.current_type.clone().unwrap_or_default();
                        if self.types.field(&t, &id.name).is_some_and(|f| f.array_size.is_some()) {
                            let args = [ident("This"), text(&id.name)].into_iter().chain(idx).chain([value]).collect();
                            return vec![call_stmt("__objaset", args)];
                        }
                    }
                    _ => {}
                }
                let target = Expression::FunctionCall(FunctionCallExpression { span: fc.span, callee: fc.callee.clone(), args: idx });
                return vec![Statement::Assignment(AssignmentStatement { span: a.span, target, value })];
            }
            Expression::Identifier(id) => {
                if let Some((name, routine, _)) = &self.ctx.current_method {
                    if id.name.eq_ignore_ascii_case(name) {
                        return vec![Statement::Assignment(AssignmentStatement { span: a.span, target: ident(routine), value })];
                    }
                }
                if !self.is_this(&id.name) && self.implicit_member(&id.name) {
                    let t = self.ctx.current_type.clone().unwrap_or_default();
                    return vec![self.store_field(ident("This"), &t, &id.name, value)];
                }
            }
            _ => {}
        }
        let target = match &a.target {
            Expression::ArrayAccess(x) => self.expr(&Expression::ArrayAccess(x.clone())),
            other => other.clone(),
        };
        vec![Statement::Assignment(AssignmentStatement { span: a.span, target, value })]
    }

    fn call_statement(&mut self, c: &CallStatement) -> Vec<Statement> {
        let routines = self.routines.clone();
        if let Some(a) = inc_dec_assignment(c, |n| routines.contains(&key(n))) {
            return self.assignment(&a);
        }
        let args: Vec<Expression> = c.args.iter().map(|a| self.expr(a)).collect();
        match &c.callee {
            Expression::MemberAccess(m) => {
                if let Some(t) = self.object_type(&m.object) {
                    let o = self.expr(&m.object);
                    if self.is_user_type(&t) {
                        if let Some((def, name, _)) = self.types.find_method(&t, &m.member) {
                            return vec![call_stmt(&mangle(&def, &name), std::iter::once(o).chain(args).collect())];
                        }
                    }
                    return vec![call_stmt("__objcall", [o, text(&m.member)].into_iter().chain(args).collect())];
                }
            }
            Expression::Identifier(id) if self.ctx.current_type.is_some() && !self.is_local(&id.name) => {
                let t = self.ctx.current_type.clone().unwrap_or_default();
                if let Some((def, name, _)) = self.types.find_method(&t, &id.name) {
                    return vec![call_stmt(&mangle(&def, &name), std::iter::once(ident("This")).chain(args).collect())];
                }
                if self.types.base_component(&t).is_some() && self.implicit_member(&id.name) {
                    // `Center` / `ShowModal` inside a TYPE EXTENDS QFORM.
                    return vec![call_stmt("__objcall", [ident("This"), text(&id.name)].into_iter().chain(args).collect())];
                }
            }
            _ => {}
        }
        vec![Statement::Call(CallStatement { span: c.span, callee: c.callee.clone(), args })]
    }

    /// `DIM x AS TType`: the variable holds its id, then `TType__init(id)`.
    fn dim(&mut self, d: &DimStatement) -> Vec<Statement> {
        let Some(t) = self.types.get(&d.type_name).map(|t| t.name.clone()) else {
            for v in &d.declarators {
                self.ctx.locals.insert(key(&v.name));
            }
            let mut d = d.clone();
            for v in &mut d.declarators {
                for dim in &mut v.dimensions {
                    match dim {
                        ArrayDimension::Single(x) => *x = self.expr(x),
                        ArrayDimension::Range { start, end } => {
                            *start = self.expr(start);
                            *end = self.expr(end);
                        }
                    }
                }
            }
            return vec![Statement::Dim(d)];
        };
        let mut out = Vec::new();
        for v in &d.declarators {
            let k = key(&v.name);
            self.ctx.locals.insert(k.clone());
            if !v.dimensions.is_empty() {
                // `DIM a(n) AS TType`: a plain array, as in the interpreter.
                out.push(Statement::Dim(DimStatement { declarators: vec![v.clone()], type_name: "VARIANT".into(), ..d.clone() }));
                continue;
            }
            if self.ctx.in_main {
                self.global_types.insert(k, t.clone());
            } else {
                self.ctx.local_types.insert(k, t.clone());
            }
            out.push(Statement::Dim(DimStatement {
                span: d.span,
                declarators: vec![VariableDeclarator { span: v.span, name: v.name.clone(), dimensions: Vec::new() }],
                type_name: "STRING".into(),
                is_static: false,
                is_redim: false,
            }));
            out.extend(self.instance(&v.name, &t));
        }
        out
    }

    /// `CREATE x AS TType … END CREATE`: set the instance up, then its
    /// settings (`Caption = …` is `x.Caption = …`, `Center` is `x.Center`);
    /// a nested CREATE gets `x` as its Parent.
    fn create_instance(&mut self, c: &CreateStatement, t: &str) -> Vec<Statement> {
        let k = key(&c.name);
        if self.ctx.in_main {
            self.global_types.insert(k, t.to_string());
        } else {
            self.ctx.local_types.insert(k.clone(), t.to_string());
            self.ctx.locals.insert(k);
        }
        let mut out = vec![Statement::Dim(DimStatement {
            span: c.span,
            declarators: vec![VariableDeclarator { span: c.span, name: c.name.clone(), dimensions: Vec::new() }],
            type_name: "STRING".into(),
            is_static: false,
            is_redim: false,
        })];
        out.extend(self.instance(&c.name, t));
        let member = |m: &str| Expression::MemberAccess(MemberAccessExpression { span: c.span, object: Box::new(ident(&c.name)), member: m.to_string() });
        for s in &c.body {
            match s {
                Statement::Assignment(a) => {
                    let target = match &a.target {
                        Expression::Identifier(id) => member(&id.name),
                        other => other.clone(),
                    };
                    out.extend(self.assignment(&AssignmentStatement { span: a.span, target, value: a.value.clone() }));
                }
                Statement::Call(call) if matches!(&call.callee, Expression::Identifier(id) if !self.routines.contains(&key(&id.name))) => {
                    let Expression::Identifier(id) = &call.callee else { unreachable!() };
                    out.extend(self.call_statement(&CallStatement { span: call.span, callee: member(&id.name), args: call.args.clone() }));
                }
                Statement::Create(child) => {
                    out.extend(self.stmt(s));
                    out.push(call_stmt("__objset", vec![text(&child.name), text("parent"), text(&c.name)]));
                }
                other => out.extend(self.stmt(other)),
            }
        }
        out
    }

    fn stmt(&mut self, s: &Statement) -> Vec<Statement> {
        let e = |this: &Self, x: &Expression| this.expr(x);
        match s {
            Statement::Assignment(a) => self.assignment(a),
            Statement::Call(c) => self.call_statement(c),
            Statement::Dim(d) => self.dim(d),
            Statement::With(w) => {
                if matches!(&w.object, Expression::Identifier(id) if self.is_this(&id.name)) || self.object_type(&w.object).is_some_and(|t| self.is_user_type(&t)) {
                    let body = resolve_with_body(&w.body, &w.object);
                    return self.body(&body);
                }
                vec![Statement::With(WithStatement { span: w.span, object: e(self, &w.object), body: self.body(&w.body) })]
            }
            Statement::Print(p) => vec![Statement::Print(PrintStatement { items: p.items.iter().map(|x| e(self, x)).collect(), ..p.clone() })],
            Statement::If(i) => {
                let condition = e(self, &i.condition);
                let then_body = self.body(&i.then_body);
                let elseif_branches = i
                    .elseif_branches
                    .iter()
                    .map(|b| ElseIfBranch { condition: self.expr(&b.condition), body: self.body(&b.body), ..b.clone() })
                    .collect();
                let else_body = self.body(&i.else_body);
                vec![Statement::If(IfStatement { span: i.span, condition, then_body, elseif_branches, else_body })]
            }
            Statement::For(f) => {
                let body = self.body(&f.body);
                vec![Statement::For(ForStatement {
                    start: e(self, &f.start),
                    end: e(self, &f.end),
                    step: f.step.as_ref().map(|x| e(self, x)),
                    body,
                    ..f.clone()
                })]
            }
            Statement::While(w) => {
                let body = self.body(&w.body);
                vec![Statement::While(WhileStatement { span: w.span, condition: e(self, &w.condition), body })]
            }
            Statement::DoLoop(d) => {
                let body = self.body(&d.body);
                vec![Statement::DoLoop(DoLoopStatement { condition: d.condition.as_ref().map(|x| e(self, x)), body, ..d.clone() })]
            }
            Statement::SelectCase(sc) => {
                let cases = sc
                    .cases
                    .iter()
                    .map(|c| CaseBranch {
                        values: c
                            .values
                            .iter()
                            .map(|v| match v {
                                CaseValue::Value(x) => CaseValue::Value(self.expr(x)),
                                CaseValue::Is(op, x) => CaseValue::Is(*op, self.expr(x)),
                                CaseValue::Range(a, b) => CaseValue::Range(self.expr(a), self.expr(b)),
                            })
                            .collect(),
                        body: self.body(&c.body),
                        ..c.clone()
                    })
                    .collect();
                let case_else = self.body(&sc.case_else);
                vec![Statement::SelectCase(SelectCaseStatement { span: sc.span, expression: e(self, &sc.expression), cases, case_else })]
            }
            Statement::Return(r) => vec![Statement::Return(ReturnStatement { span: r.span, value: r.value.as_ref().map(|x| e(self, x)) })],
            Statement::Const(c) => {
                self.ctx.locals.insert(key(&c.name));
                vec![Statement::Const(ConstStatement { value: e(self, &c.value), ..c.clone() })]
            }
            Statement::Input(i) => vec![Statement::Assignment(input_assignment(i))].iter().flat_map(|a| self.stmt(a)).collect(),
            Statement::Create(c) => match self.types.get(&c.type_name).map(|t| t.name.clone()) {
                Some(t) => self.create_instance(c, &t),
                None => vec![Statement::Create(CreateStatement { body: self.body(&c.body), ..c.clone() })],
            },
            Statement::Subroutine(sub) => vec![Statement::Subroutine(self.routine(sub.name.clone(), &sub.params, &sub.body, None, None).into_sub(sub))],
            Statement::Function(f) => {
                let body = self.routine(f.name.clone(), &f.params, &f.body, None, None);
                vec![Statement::Function(FunctionStatement { body: body.0, ..f.clone() })]
            }
            _ => vec![s.clone()],
        }
    }

    /// Rewrites a routine body with its own locals; `in_type` and `method`
    /// set the TYPE context for methods and constructors.
    fn routine(&mut self, _name: String, params: &[Parameter], body: &[Statement], in_type: Option<&str>, method: Option<(String, String, bool)>) -> Body {
        let mut ctx = Ctx { in_main: false, current_type: in_type.map(str::to_string), current_method: method, ..Default::default() };
        for p in params {
            ctx.locals.insert(key(strip_suffix(&p.name)));
            // `obj AS TType`, or `Sender AS QBUTTON`: an object parameter.
            if let Some(kind) = object_kind(self.types, &p.type_name) {
                ctx.local_types.insert(key(strip_suffix(&p.name)), kind);
            }
        }
        // Names the body DIMs are its own (not fields or globals).
        walk(
            body,
            &mut |s| {
                if let Statement::Dim(d) = s {
                    for v in &d.declarators {
                        ctx.locals.insert(key(&v.name));
                    }
                }
            },
            &mut |_| {},
        );
        let saved = std::mem::replace(&mut self.ctx, ctx);
        let out = self.body(body);
        self.ctx = saved;
        Body(out)
    }
}

struct Body(Vec<Statement>);

impl Body {
    fn into_sub(self, s: &SubroutineStatement) -> SubroutineStatement {
        SubroutineStatement { body: self.0, ..s.clone() }
    }
}

fn strip_suffix(name: &str) -> &str {
    name.trim_end_matches(['$', '%', '&', '!', '#'])
}

fn this_param() -> Parameter {
    Parameter { span: sp(), name: "This".into(), type_name: String::new(), by_ref: false, is_array: false }
}

/// Rewrites every object-oriented TYPE in `program` into plain routines (see
/// the module docs). Programs without such TYPEs are returned unchanged.
pub fn lower(program: &Program) -> Program {
    let types = collect_types(program);
    let component_params = program.statements.iter().any(|s| match s {
        Statement::Subroutine(r) => r.params.iter().any(|p| object_kind(&types, &p.type_name).is_some()),
        Statement::Function(r) => r.params.iter().any(|p| object_kind(&types, &p.type_name).is_some()),
        _ => false,
    });
    if types.map.is_empty() && !component_params {
        return program.clone();
    }
    let mut globals = HashSet::new();
    let mut routines = HashSet::new();
    for s in &program.statements {
        match s {
            Statement::Dim(d) => globals.extend(d.declarators.iter().map(|v| key(&v.name))),
            Statement::Const(c) => {
                globals.insert(key(&c.name));
            }
            Statement::Create(c) => {
                globals.insert(key(&c.name));
            }
            Statement::Subroutine(s) => {
                routines.insert(key(&s.name));
            }
            Statement::Function(f) => {
                routines.insert(key(&f.name));
            }
            _ => {}
        }
    }
    let mut l = Lowering {
        types: &types,
        global_types: HashMap::new(),
        globals,
        routines,
        ctx: Ctx { in_main: true, ..Default::default() },
        extra: Vec::new(),
        defined_trampolines: HashSet::new(),
    };

    // Main-program object variables are known before any routine uses them.
    for s in &program.statements {
        if let Statement::Dim(d) = s {
            if let Some(t) = types.get(&d.type_name) {
                for v in &d.declarators {
                    l.global_types.insert(key(&v.name), t.name.clone());
                }
            }
        }
    }

    let mut out = Vec::new();
    for s in &program.statements {
        let Statement::Type(t) = s else { continue };
        let Some(def) = types.get(&t.name) else { continue };
        let tname = def.name.clone();
        for m in &t.methods {
            match m {
                Statement::Subroutine(sub) => {
                    let routine = mangle(&tname, &sub.name);
                    let body = l.routine(routine.clone(), &sub.params, &sub.body, Some(&tname), Some((sub.name.clone(), routine.clone(), false)));
                    out.push(Statement::Subroutine(SubroutineStatement {
                        span: sub.span,
                        name: routine,
                        params: std::iter::once(this_param()).chain(sub.params.iter().cloned()).collect(),
                        body: body.0,
                    }));
                }
                Statement::Function(f) => {
                    let routine = mangle(&tname, &f.name);
                    let body = l.routine(routine.clone(), &f.params, &f.body, Some(&tname), Some((f.name.clone(), routine.clone(), true)));
                    out.push(Statement::Function(FunctionStatement {
                        span: f.span,
                        name: routine,
                        params: std::iter::once(this_param()).chain(f.params.iter().cloned()).collect(),
                        return_type: f.return_type.clone(),
                        body: body.0,
                    }));
                }
                _ => {}
            }
        }
        if def.has_ctor {
            let routine = format!("{tname}__ctor");
            let body = l.routine(routine.clone(), &[], &t.constructor, Some(&tname), None);
            out.push(Statement::Subroutine(SubroutineStatement { span: t.span, name: routine, params: vec![this_param()], body: body.0 }));
        }
        for (i, e) in t.events.iter().enumerate() {
            let routine = format!("{tname}__ev{i}");
            let body = l.routine(routine.clone(), &e.params, &e.body, Some(&tname), None);
            out.push(Statement::Subroutine(SubroutineStatement {
                span: e.span,
                name: routine,
                params: std::iter::once(this_param()).chain(e.params.iter().cloned()).collect(),
                body: body.0,
            }));
        }
        out.push(Statement::Subroutine(init_routine(&types, &tname)));
    }
    for s in &program.statements {
        match s {
            Statement::Type(t) if types.get(&t.name).is_some() => {}
            other => out.extend(l.stmt(other)),
        }
    }
    // Handler trampolines go first (routines are found anywhere, but keep
    // generated code together).
    let mut statements = std::mem::take(&mut l.extra);
    statements.extend(out);
    Program { span: program.span, statements }
}

/// `SUB Type__init (This)`: create the component a TYPE extends, set every
/// field (base TYPE first), give object fields their own objects, then run
/// the CONSTRUCTORs, base first — the bytecode compiler's `setup_instance`.
fn init_routine(types: &Types, t: &str) -> SubroutineStatement {
    let mut body = Vec::new();
    let this = || ident("This");
    if let Some(kind) = types.base_component(t) {
        body.push(call_stmt("__objcreate", vec![this(), text(&kind)]));
    }
    let chain = types.chain(t);
    for def in &chain {
        for f in &def.fields {
            if f.array_size.is_none() {
                if let Some(kind) = object_kind(types, &f.type_name) {
                    if let Some(sub) = types.get(&kind) {
                        body.push(call_stmt("__objset", vec![this(), text(&f.name), sub_id(this(), &f.name)]));
                        body.push(call_stmt(&format!("{}__init", sub.name), vec![sub_id(this(), &f.name)]));
                    } else {
                        body.push(call_stmt("__objcreate", vec![sub_id(this(), &f.name), text(&kind)]));
                        body.push(call_stmt("__objset", vec![this(), text(&f.name), sub_id(this(), &f.name)]));
                    }
                    continue;
                }
            }
            if let (Some(upper), Some(_)) = (&f.array_size, object_kind(types, &f.type_name)) {
                // `image(10) AS QBITMAP`: the element ids `<id>.image(i)`.
                let lower = f.array_lower.clone().unwrap_or(Expression::Literal(Literal { span: sp(), value: LiteralValue::Integer(0) }));
                let prefix = Expression::Binary(BinaryExpression {
                    span: sp(),
                    left: Box::new(this()),
                    operator: BinaryOperator::Add,
                    right: Box::new(text(&format!(".{}(", f.name))),
                });
                body.push(call_stmt("__objset", vec![this(), text(&f.name), call("__objids", vec![prefix, lower, upper.clone()])]));
                continue;
            }
            let fill = field_fill(&f.type_name);
            let value = match &f.array_size {
                Some(upper) => {
                    let lower = f.array_lower.clone().unwrap_or(Expression::Literal(Literal { span: sp(), value: LiteralValue::Integer(0) }));
                    call("__objarray", vec![lower, upper.clone(), fill])
                }
                None => fill,
            };
            body.push(call_stmt("__objset", vec![this(), text(&f.name), value]));
        }
    }
    for def in &chain {
        if def.has_ctor {
            body.push(call_stmt(&format!("{}__ctor", def.name), vec![this()]));
        }
    }
    SubroutineStatement { span: sp(), name: format!("{t}__init"), params: vec![this_param()], body }
}

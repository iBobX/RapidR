//! Objects (RapidQ manual ch. 10) lowered to plain BASIC, shared by both
//! backends — the bytecode compiler and the Rust code generator run this
//! pass first, so objects behave identically in the interpreter and in
//! native builds.
//!
//! * **Instances** are values (`rapidr_value::Value::Object`): shared by
//!   reference, each field in a slot fixed at compile time — the TYPE's
//!   ancestors' fields first, so a base TYPE's methods work on derived
//!   instances. `DIM c AS TCounter` → `c = __newobject("c", "TCounter",
//!   "count,increment,…")` then `TCounter___init c`. Field access is
//!   `__getfield(obj, slot)` / `__setfield(obj, slot, v)`: an index, never a
//!   lookup by name (native builds compile it to a direct vector access, the
//!   interpreter to one opcode).
//! * **Methods** become routines `Type__Method(This, …)`; calls resolve by the
//!   declared type, the nearest ancestor defining the method winning.
//!   Inside a TYPE's code `This`, `Me`, the type's own (or an ancestor's)
//!   name and a leading `.` stand for the instance, bare field names are its
//!   fields, and assigning a field with a PROPERTY SET calls the setter
//!   (except inside the setter itself).
//! * The routines this pass generates (`Type___init`, `Type___ctor`,
//!   `Type___ev<i>`) have three underscores, so a method named `Init` (the
//!   routine `Type__Init`) never replaces them.
//! * **`Type___init(This)`** creates the component a TYPE EXTENDS (whose
//!   properties stay in the runtime's component registry, under the
//!   instance's id), sets every field, gives fields of TYPE or component
//!   type their own object (`<id>.<field>`), binds the EVENT handlers to the
//!   instance, then runs the CONSTRUCTORs — base TYPE first.
//! * **Arrays of objects**: `DIM a(1 TO 3) AS TType` makes one instance per
//!   element (ids `a(1)` …); `DIM lbl(3) AS QLABEL` one component each. Array
//!   fields of objects work the same way per instance.
//! * **Components** reached through objects, parameters (`Sender AS
//!   QBUTTON`), arrays or indexed sub-objects (`SB.Panel(0).Width`) use the
//!   registry: `__objget` / `__objset` / `__objcall` (by the object's id);
//!   `Stream.Read(var)` becomes `var = Stream.__read(var)`.

use std::collections::{HashMap, HashSet};

use crate::*;
use rapidr_diagnostics::TextSpan;

/// A TYPE with its slot layout.
struct TypeDef {
    name: String,
    extends: Option<String>,
    fields: Vec<TypeField>,
    /// Method name (lowercase) → (name as written, is FUNCTION).
    methods: HashMap<String, (String, bool)>,
    has_ctor: bool,
    /// EVENT blocks: (event, parameter count); handler `Type___ev<i>`.
    events: Vec<(String, usize)>,
}

struct Types {
    map: HashMap<String, TypeDef>,
}

fn key(name: &str) -> String {
    name.to_ascii_lowercase()
}

/// `Type__Method`: the routine a method compiles to.
pub fn mangle(type_name: &str, member: &str) -> String {
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

    /// Every field of the TYPE in slot order (ancestors' first).
    fn slots(&self, t: &str) -> Vec<&TypeField> {
        self.chain(t).into_iter().flat_map(|d| d.fields.iter()).collect()
    }

    /// Slot of `field` (the most derived declaration of that name).
    fn slot(&self, t: &str, field: &str) -> Option<usize> {
        self.slots(t).iter().rposition(|f| f.name.eq_ignore_ascii_case(field))
    }

    /// The field names, comma separated, for `__newobject`.
    fn field_names(&self, t: &str) -> String {
        self.slots(t).iter().map(|f| f.name.as_str()).collect::<Vec<_>>().join(",")
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

    /// Type of a (non-array) field that holds an object: a TYPE or a component.
    fn field_object_type(&self, t: &str, f: &str) -> Option<String> {
        let field = self.field(t, f)?;
        if field.array_size.is_some() {
            return None;
        }
        object_kind(self, &field.type_name)
    }

    /// The conversion a store into field `f` needs (a numeric field;
    /// `crate::numeric`).
    fn field_conversion(&self, t: &str, f: &str) -> Option<&'static str> {
        crate::numeric::conversion_for(&self.field(t, f)?.type_name)
    }

    fn setter(&self, t: &str, f: &str) -> Option<(String, String)> {
        let setter = self.field(t, f)?.setter.clone()?;
        self.find_method(t, &setter).map(|(def, name, _)| (def, name))
    }

    /// Whether an instance of TYPE `from` contains (through its fields,
    /// transitively) an instance of TYPE `to`: such a field would create
    /// instances without end, so it starts empty (`Next AS TNode`).
    fn contains_type(&self, from: &str, to: &str, seen: &mut HashSet<String>) -> bool {
        if from.eq_ignore_ascii_case(to) {
            return true;
        }
        if !seen.insert(key(from)) {
            return false;
        }
        let kinds: Vec<String> = self.slots(from).iter().filter_map(|f| self.get(&f.type_name).map(|d| d.name.clone())).collect();
        kinds.iter().any(|k| self.contains_type(k, to, seen))
    }

    /// Whether `name` is the TYPE `t` or one of its ancestors.
    fn is_self_or_ancestor(&self, t: &str, name: &str) -> bool {
        self.chain(t).iter().any(|d| d.name.eq_ignore_ascii_case(name))
    }
}

/// A TYPE's name, or a component type (canonical, uppercase) — including
/// RapidQ objects RapidR has no component for yet (property bags).
fn object_kind(types: &Types, type_name: &str) -> Option<String> {
    if let Some(d) = types.get(type_name) {
        return Some(d.name.clone());
    }
    let c = canonical_type_name(type_name);
    is_rapidq_object_type(&c).then(|| c.to_ascii_uppercase())
}

/// RapidQ's global objects: never a member of a TYPE's component.
const GLOBAL_OBJECTS: &[&str] = &["application", "screen", "clipboard", "printer", "mouse"];

/// Whether `name` is one of RapidQ's global objects (`Application`,
/// `Screen`, `Clipboard`, `Printer`, `Mouse`).
pub fn is_global_object(name: &str) -> bool {
    GLOBAL_OBJECTS.contains(&key(name).as_str())
}

fn collect_types(program: &Program) -> Types {
    let map = program
        .statements
        .iter()
        .filter_map(|s| match s {
            Statement::Type(t) => Some(t),
            _ => None,
        })
        .map(|t| {
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
                key(&t.name),
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

// ---------- building blocks ----------

fn ident_at(span: TextSpan, name: &str) -> Expression {
    Expression::Identifier(Identifier { span, name: name.into() })
}

fn text_at(span: TextSpan, s: &str) -> Expression {
    Expression::Literal(Literal { span, value: LiteralValue::String(s.into()) })
}

fn int_at(span: TextSpan, n: i64) -> Expression {
    Expression::Literal(Literal { span, value: LiteralValue::Integer(n) })
}

fn call_at(span: TextSpan, name: &str, args: Vec<Expression>) -> Expression {
    Expression::FunctionCall(FunctionCallExpression { span, callee: Box::new(ident_at(span, name)), args })
}

fn call_stmt_at(span: TextSpan, name: &str, args: Vec<Expression>) -> Statement {
    Statement::Call(CallStatement { span, callee: ident_at(span, name), args })
}

fn assign_at(span: TextSpan, target: Expression, value: Expression) -> Statement {
    Statement::Assignment(AssignmentStatement { span, target, value })
}

fn concat_at(span: TextSpan, left: Expression, right: &str) -> Expression {
    Expression::Binary(BinaryExpression { span, left: Box::new(left), operator: BinaryOperator::Add, right: Box::new(text_at(span, right)) })
}

fn field_fill(span: TextSpan, type_name: &str) -> Expression {
    match type_name.to_ascii_uppercase().as_str() {
        "STRING" => text_at(span, ""),
        "INTEGER" | "LONG" | "SHORT" | "BYTE" | "WORD" | "DWORD" | "SINGLE" | "DOUBLE" | "CURRENCY" => int_at(span, 0),
        _ => call_at(span, "__null", Vec::new()),
    }
}

fn strip_suffix(name: &str) -> &str {
    name.trim_end_matches(['$', '%', '&', '!', '#'])
}

fn this_param() -> Parameter {
    Parameter { span: TextSpan::default(), name: "This".into(), type_name: String::new(), by_ref: false, is_array: false }
}

// ---------- the rewrite ----------

/// Where the code being rewritten is.
#[derive(Clone, Default)]
struct Ctx {
    /// Object variables of the routine (params and DIMs), lowercase → type.
    local_types: HashMap<String, String>,
    /// Every parameter and DIMmed local of the routine (lowercase).
    locals: HashSet<String>,
    /// Arrays of objects of the routine → element type.
    local_arrays: HashMap<String, String>,
    in_main: bool,
    /// Inside a TYPE's method/CONSTRUCTOR/EVENT: that TYPE.
    current_type: Option<String>,
    /// The method being rewritten: (name as written, routine name).
    current_method: Option<(String, String)>,
}

struct Lowering<'a> {
    types: &'a Types,
    is_builtin: &'a dyn Fn(&str) -> bool,
    /// Object variables of the main program, lowercase → type.
    global_types: HashMap<String, String>,
    /// Arrays of objects of the main program → element type.
    global_arrays: HashMap<String, String>,
    /// CREATE/DIM components (lowercase) → kind, for `SB.Panel(0).Width`.
    component_kinds: HashMap<String, String>,
    /// Main-program variable and constant names (lowercase).
    globals: HashSet<String>,
    /// User SUB/FUNCTION names (lowercase).
    routines: HashSet<String>,
    ctx: Ctx,
    /// Numbers generated loop variables.
    counter: usize,
}

impl Lowering<'_> {
    fn is_local(&self, name: &str) -> bool {
        self.ctx.locals.contains(&key(name))
    }

    fn is_user_type(&self, t: &str) -> bool {
        self.types.get(t).is_some()
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
                && !GLOBAL_OBJECTS.contains(&key(name).as_str())
                && !self.globals.contains(&key(name))
                && !self.routines.contains(&key(name))
                && !(self.is_builtin)(&key(name)))
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

    fn array_kind(&self, name: &str) -> Option<String> {
        if !self.ctx.in_main && self.is_local(name) {
            return self.ctx.local_arrays.get(&key(name)).cloned();
        }
        self.global_arrays.get(&key(name)).cloned()
    }

    fn array_field_type(&self, t: &str, field: &str) -> Option<String> {
        let f = self.types.field(t, field)?;
        f.array_size.as_ref()?;
        object_kind(self.types, &f.type_name)
    }

    fn is_array_field(&self, t: &str, field: &str) -> bool {
        self.types.field(t, field).is_some_and(|f| f.array_size.is_some())
    }

    /// Kind of a CREATE/DIM component named `name` (not a routine's own
    /// variable of that name, nor an object variable).
    fn static_component(&self, name: &str) -> Option<String> {
        if (self.is_local(name) && !self.ctx.in_main) || self.var_type(name).is_some() {
            return None;
        }
        self.component_kinds.get(&key(name)).cloned()
    }

    /// The object type an expression holds: a TYPE (by name) or a component
    /// type (uppercase) — for variables, parameters, fields, array elements.
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
            Expression::FunctionCall(fc) => match fc.callee.as_ref() {
                // `a(i)`: an element of an array of objects.
                Expression::Identifier(id) if self.array_kind(&id.name).is_some() => self.array_kind(&id.name),
                // `obj.items(i)` / `items(i)`: an element of an array field.
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
            Expression::ArrayAccess(a) => match a.array.as_ref() {
                Expression::Identifier(id) => self.array_kind(&id.name),
                _ => None,
            },
            _ => None,
        }
    }

    /// `Font.Size` inside a TYPE extending a component, or `obj.Canvas.Font`
    /// on a component reached through an object: a property object of the
    /// component (not a field) — the component and the combined property
    /// name the runtimes use (`font.size`).
    fn component_sub_property(&self, m: &MemberAccessExpression) -> Option<(Expression, String)> {
        match m.object.as_ref() {
            Expression::Identifier(id) => {
                let t = self.ctx.current_type.as_ref()?;
                if self.types.base_component(t).is_none() || self.types.field(t, &id.name).is_some() || !self.implicit_member(&id.name) {
                    return None;
                }
                Some((ident_at(m.span, "This"), format!("{}.{}", key(&id.name), key(&m.member))))
            }
            Expression::MemberAccess(inner) => {
                let t = self.object_type(&inner.object)?;
                if (self.is_user_type(&t) && !self.is_component_member(&t, &inner.member))
                    || matches!(inner.object.as_ref(), Expression::Identifier(id) if self.static_component(&id.name).is_some())
                {
                    return None;
                }
                Some((self.expr(&inner.object), format!("{}.{}", key(&inner.member), key(&m.member))))
            }
            _ => None,
        }
    }

    /// Whether `member` of TYPE `t` belongs to the component `t` extends
    /// (not one of its fields or methods).
    fn is_component_member(&self, t: &str, member: &str) -> bool {
        self.types.base_component(t).is_some() && self.types.field(t, member).is_none() && self.types.find_method(t, member).is_none()
    }

    /// `SB.Panel(0)` (a component's indexed sub-object): the component
    /// expression, the sub-object's name and the index arguments.
    fn indexed_sub<'e>(&self, e: &'e Expression) -> Option<(&'e Expression, &'e str, &'e [Expression])> {
        let Expression::FunctionCall(fc) = e else { return None };
        let Expression::MemberAccess(m) = fc.callee.as_ref() else { return None };
        let component = match m.object.as_ref() {
            Expression::Identifier(id) if self.static_component(&id.name).is_some() => true,
            other => self.object_type(other).is_some_and(|t| !self.is_user_type(&t) || self.is_component_member(&t, &m.member)),
        };
        component.then_some((m.object.as_ref(), m.member.as_str(), fc.args.as_slice()))
    }

    /// `obj.Method` / `TType.Method` naming a method (for CODEPTR and
    /// BIND): the routine it compiles to.
    fn method_pointer(&self, e: &Expression) -> Option<Expression> {
        let Expression::MemberAccess(m) = e else { return None };
        let t = self.object_type(&m.object).or_else(|| match m.object.as_ref() {
            Expression::Identifier(id) => self.types.get(&id.name).map(|d| d.name.clone()),
            _ => None,
        })?;
        let (def, name, _) = self.types.find_method(&t, &m.member)?;
        Some(ident_at(m.span, &mangle(&def, &name)))
    }

    // ----- expressions -----

    fn expr(&self, e: &Expression) -> Expression {
        match e {
            Expression::Identifier(id) => {
                let span = id.span;
                if self.is_this(&id.name) {
                    return ident_at(span, "This");
                }
                if let Some((name, routine)) = &self.ctx.current_method {
                    if id.name.eq_ignore_ascii_case(name) {
                        return ident_at(span, routine);
                    }
                }
                if self.implicit_member(&id.name) {
                    let t = self.ctx.current_type.clone().unwrap_or_default();
                    return self.member_read(span, ident_at(span, "This"), &t, &id.name);
                }
                e.clone()
            }
            Expression::MemberAccess(m) => {
                if let Some((o, combined)) = self.component_sub_property(m) {
                    return call_at(m.span, "__objget", vec![o, text_at(m.span, &combined)]);
                }
                if let Some((obj, sub, idx)) = self.indexed_sub(&m.object) {
                    let args = [self.expr(obj), text_at(m.span, &format!("{sub}.{}", m.member))].into_iter().chain(idx.iter().map(|i| self.expr(i))).collect();
                    return call_at(m.span, "__objcall", args);
                }
                if let Some(t) = self.object_type(&m.object) {
                    let o = self.expr(&m.object);
                    return self.member_read(m.span, o, &t, &m.member);
                }
                Expression::MemberAccess(MemberAccessExpression { span: m.span, object: Box::new(self.expr(&m.object)), member: m.member.clone() })
            }
            Expression::FunctionCall(fc) => {
                let span = fc.span;
                // `CODEPTR(obj.Method)`: a pointer to the method's routine.
                if let (Expression::Identifier(f), [arg]) = (fc.callee.as_ref(), fc.args.as_slice()) {
                    if matches!(key(&f.name).as_str(), "codeptr" | "callback") {
                        if let Some(routine) = self.method_pointer(arg) {
                            return Expression::FunctionCall(FunctionCallExpression { span, callee: fc.callee.clone(), args: vec![routine] });
                        }
                    }
                }
                let args: Vec<Expression> = fc.args.iter().map(|a| self.expr(a)).collect();
                match fc.callee.as_ref() {
                    Expression::MemberAccess(m) if self.indexed_sub(fc.callee.as_ref()).is_none() => {
                        if let Some(t) = self.object_type(&m.object) {
                            let o = self.expr(&m.object);
                            return self.member_call(span, o, &t, &m.member, args);
                        }
                    }
                    Expression::Identifier(id) if self.ctx.current_type.is_some() && !self.is_local(&id.name) => {
                        let t = self.ctx.current_type.clone().unwrap_or_default();
                        if self.types.find_method(&t, &id.name).is_some() || self.is_array_field(&t, &id.name) {
                            return self.member_call(span, ident_at(span, "This"), &t, &id.name, args);
                        }
                    }
                    _ => {}
                }
                let callee = match fc.callee.as_ref() {
                    // A routine or array name stays a name (not a field read).
                    Expression::Identifier(_) => fc.callee.as_ref().clone(),
                    other => self.expr(other),
                };
                Expression::FunctionCall(FunctionCallExpression { span, callee: Box::new(callee), args })
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

    /// `obj.member` read: a field (by slot), a FUNCTION method called
    /// without parentheses, or a component property (by the object's id).
    fn member_read(&self, span: TextSpan, o: Expression, t: &str, member: &str) -> Expression {
        if self.is_user_type(t) {
            if let Some((def, name, true)) = self.types.find_method(t, member) {
                return call_at(span, &mangle(&def, &name), vec![o]);
            }
            if let Some(slot) = self.types.slot(t, member) {
                return call_at(span, "__getfield", vec![o, int_at(span, slot as i64)]);
            }
        }
        call_at(span, "__objget", vec![o, text_at(span, member)])
    }

    /// `obj.member(args)`: a method, an element of an array field, or a
    /// component method.
    fn member_call(&self, span: TextSpan, o: Expression, t: &str, member: &str, args: Vec<Expression>) -> Expression {
        if self.is_user_type(t) {
            if let Some((def, name, _)) = self.types.find_method(t, member) {
                return call_at(span, &mangle(&def, &name), std::iter::once(o).chain(args).collect());
            }
            if let (Some(slot), true) = (self.types.slot(t, member), self.is_array_field(t, member)) {
                let array = call_at(span, "__getfield", vec![o, int_at(span, slot as i64)]);
                return call_at(span, "__aget", std::iter::once(array).chain(args).collect());
            }
        }
        call_at(span, "__objcall", [o, text_at(span, member)].into_iter().chain(args).collect())
    }

    // ----- statements -----

    fn body(&mut self, stmts: &[Statement]) -> Vec<Statement> {
        stmts.iter().flat_map(|s| self.stmt(s)).collect()
    }

    /// `obj.field = v` / `field = v`: the setter, a slot store, or a
    /// component property.
    fn store_member(&self, span: TextSpan, o: Expression, t: &str, member: &str, value: Expression) -> Statement {
        if self.is_user_type(t) {
            if let Some((def, setter)) = self.types.setter(t, member) {
                let in_setter = self.ctx.current_method.as_ref().is_some_and(|(m, _)| m.eq_ignore_ascii_case(&setter));
                if !in_setter {
                    return call_stmt_at(span, &mangle(&def, &setter), vec![o, value]);
                }
            }
            if let Some(slot) = self.types.slot(t, member) {
                let value = match self.types.field_conversion(t, member) {
                    Some(conv) if !self.is_array_field(t, member) => crate::numeric::convert_at(span, conv, value),
                    _ => value,
                };
                return call_stmt_at(span, "__setfield", vec![o, int_at(span, slot as i64), value]);
            }
        }
        call_stmt_at(span, "__objset", vec![o, text_at(span, member), value])
    }

    fn assignment(&mut self, a: &AssignmentStatement) -> Vec<Statement> {
        let span = a.span;
        // `lbl(i).OnClick = Handler`: bind to the object known at run time.
        if let (Expression::MemberAccess(m), Expression::Identifier(h)) = (&a.target, &a.value) {
            if m.member.to_ascii_lowercase().starts_with("on")
                && self.routines.contains(&key(&h.name))
                && !matches!(m.object.as_ref(), Expression::Identifier(id) if self.var_type(&id.name).is_none())
                && self.object_type(&m.object).is_some()
            {
                let ptr = call_at(span, "CODEPTR", vec![a.value.clone()]);
                return vec![call_stmt_at(span, "__bind_event", vec![self.expr(&m.object), text_at(span, &m.member), ptr])];
            }
        }
        let value = self.expr(&a.value);
        match &a.target {
            Expression::MemberAccess(m) if self.component_sub_property(m).is_some() => {
                let (o, combined) = self.component_sub_property(m).unwrap();
                return vec![call_stmt_at(span, "__objset", vec![o, text_at(span, &combined), value])];
            }
            Expression::MemberAccess(m) if self.indexed_sub(&m.object).is_some() => {
                let (obj, sub, idx) = self.indexed_sub(&m.object).unwrap();
                let args = [self.expr(obj), text_at(span, &format!("{sub}.{}=", m.member))]
                    .into_iter()
                    .chain(idx.iter().map(|i| self.expr(i)))
                    .chain([value])
                    .collect();
                return vec![call_stmt_at(span, "__objcall", args)];
            }
            Expression::MemberAccess(m) => {
                if let Some(t) = self.object_type(&m.object) {
                    let o = self.expr(&m.object);
                    return vec![self.store_member(span, o, &t, &m.member, value)];
                }
            }
            Expression::FunctionCall(fc) => {
                let idx: Vec<Expression> = fc.args.iter().map(|x| self.expr(x)).collect();
                // `obj.items(i) = v` / `items(i) = v`: an element of an array field.
                let array_field = match fc.callee.as_ref() {
                    Expression::MemberAccess(m) => {
                        self.object_type(&m.object).filter(|t| self.is_user_type(t) && self.is_array_field(t, &m.member)).map(|t| (self.expr(&m.object), t, m.member.clone()))
                    }
                    Expression::Identifier(id) if self.ctx.current_type.is_some() && !self.is_local(&id.name) => {
                        let t = self.ctx.current_type.clone().unwrap_or_default();
                        self.is_array_field(&t, &id.name).then(|| (ident_at(span, "This"), t, id.name.clone()))
                    }
                    _ => None,
                };
                if let Some((o, t, field)) = array_field {
                    let slot = self.types.slot(&t, &field).unwrap_or(0);
                    let value = match self.types.field_conversion(&t, &field) {
                        Some(conv) => crate::numeric::convert_at(span, conv, value),
                        None => value,
                    };
                    let array = call_at(span, "__getfield", vec![o, int_at(span, slot as i64)]);
                    let args = std::iter::once(array).chain(idx).chain([value]).collect();
                    return vec![call_stmt_at(span, "__aset", args)];
                }
                let target = Expression::FunctionCall(FunctionCallExpression { span: fc.span, callee: fc.callee.clone(), args: idx });
                return vec![assign_at(span, target, value)];
            }
            Expression::Identifier(id) => {
                if let Some((name, routine)) = &self.ctx.current_method {
                    if id.name.eq_ignore_ascii_case(name) {
                        return vec![assign_at(span, ident_at(id.span, routine), value)];
                    }
                }
                if !self.is_this(&id.name) && self.implicit_member(&id.name) {
                    let t = self.ctx.current_type.clone().unwrap_or_default();
                    return vec![self.store_member(span, ident_at(span, "This"), &t, &id.name, value)];
                }
            }
            _ => {}
        }
        let target = match &a.target {
            Expression::ArrayAccess(x) => self.expr(&Expression::ArrayAccess(x.clone())),
            other => other.clone(),
        };
        vec![assign_at(span, target, value)]
    }

    fn call_statement(&mut self, c: &CallStatement) -> Vec<Statement> {
        let span = c.span;
        let routines = self.routines.clone();
        if let Some(a) = inc_dec_assignment(c, |n| routines.contains(&key(n))) {
            return self.assignment(&a);
        }
        // `File.Read(x)` → `x = File.__read(x)`.
        let is_stream = |e: &Expression| {
            let kind = match e {
                Expression::Identifier(id) => self.static_component(&id.name).or_else(|| self.object_type(e)),
                other => self.object_type(other),
            };
            kind.is_some_and(|t| matches!(t.as_str(), "RFILESTREAM" | "RMEMORYSTREAM"))
        };
        if let Some(a) = stream_read_assignment(c, &is_stream) {
            // A static stream keeps `File.__read(x)`; one reached through an
            // object becomes `__objcall(obj, "__read", x)` in the rewrite.
            return self.assignment(&a);
        }
        let args: Vec<Expression> = c.args.iter().map(|a| self.expr(a)).collect();
        match &c.callee {
            // `.Canvas.Font.AddStyles(0)`: a method of a component's property object.
            Expression::MemberAccess(m) if self.component_sub_property(m).is_some() => {
                let (o, combined) = self.component_sub_property(m).unwrap();
                let all = [o, text_at(span, &combined)].into_iter().chain(args).collect();
                return vec![call_stmt_at(span, "__objcall", all)];
            }
            Expression::MemberAccess(m) if self.indexed_sub(&m.object).is_some() => {
                let (obj, sub, idx) = self.indexed_sub(&m.object).unwrap();
                let all = [self.expr(obj), text_at(span, &format!("{sub}.{}", m.member))].into_iter().chain(idx.iter().map(|i| self.expr(i))).chain(args).collect();
                return vec![call_stmt_at(span, "__objcall", all)];
            }
            Expression::MemberAccess(m) => {
                if let Some(t) = self.object_type(&m.object) {
                    let o = self.expr(&m.object);
                    if self.is_user_type(&t) {
                        if let Some((def, name, _)) = self.types.find_method(&t, &m.member) {
                            return vec![call_stmt_at(span, &mangle(&def, &name), std::iter::once(o).chain(args).collect())];
                        }
                    }
                    return vec![call_stmt_at(span, "__objcall", [o, text_at(span, &m.member)].into_iter().chain(args).collect())];
                }
            }
            Expression::Identifier(id) if self.ctx.current_type.is_some() && !self.is_local(&id.name) => {
                let t = self.ctx.current_type.clone().unwrap_or_default();
                if let Some((def, name, _)) = self.types.find_method(&t, &id.name) {
                    return vec![call_stmt_at(span, &mangle(&def, &name), std::iter::once(ident_at(span, "This")).chain(args).collect())];
                }
                if self.types.base_component(&t).is_some() && self.implicit_member(&id.name) {
                    // `Center` / `ShowModal` inside a TYPE EXTENDS QFORM.
                    return vec![call_stmt_at(span, "__objcall", [ident_at(span, "This"), text_at(span, &id.name)].into_iter().chain(args).collect())];
                }
            }
            _ => {}
        }
        vec![Statement::Call(CallStatement { span, callee: c.callee.clone(), args })]
    }

    fn record_object_var(&mut self, name: &str, kind: &str, array: bool) {
        let k = key(name);
        self.ctx.locals.insert(k.clone());
        match (self.ctx.in_main, array) {
            (true, false) => {
                self.global_types.insert(k, kind.to_string());
            }
            (true, true) => {
                self.global_arrays.insert(k, kind.to_string());
            }
            (false, false) => {
                self.ctx.local_types.insert(k, kind.to_string());
            }
            (false, true) => {
                self.ctx.local_arrays.insert(k, kind.to_string());
            }
        }
    }

    /// `x` becomes a new instance of TYPE `t` with the id `id`, set up.
    fn new_instance(&self, span: TextSpan, target: Expression, id: Expression, t: &str) -> Vec<Statement> {
        let names = self.types.field_names(t);
        vec![
            assign_at(span, target.clone(), call_at(span, "__newobject", vec![id, text_at(span, t), text_at(span, &names)])),
            call_stmt_at(span, &format!("{t}___init"), vec![target]),
        ]
    }

    /// `FOR` loops over every element of the array expression `array`
    /// (1 dimension for fields; `dims` for DIMs) calling `init` on each.
    fn init_elements(&mut self, span: TextSpan, array: &str, dims: usize, init: &str) -> Vec<Statement> {
        self.counter += 1;
        let vars: Vec<String> = (0..dims).map(|d| format!("__oi{}_{d}", self.counter)).collect();
        let element = Expression::FunctionCall(FunctionCallExpression {
            span,
            callee: Box::new(ident_at(span, array)),
            args: vars.iter().map(|v| ident_at(span, v)).collect(),
        });
        let mut body = vec![call_stmt_at(span, init, vec![element])];
        for (d, v) in vars.iter().enumerate().rev() {
            let bound = |f: &str| call_at(span, f, vec![ident_at(span, array), int_at(span, d as i64 + 1)]);
            body = vec![Statement::For(ForStatement { span, variable: v.clone(), start: bound("LBOUND"), end: bound("UBOUND"), step: None, body })];
        }
        body
    }

    fn dim(&mut self, d: &DimStatement) -> Vec<Statement> {
        let span = d.span;
        let kind = self.types.get(&d.type_name).map(|t| t.name.clone());
        let mut out = Vec::new();
        let mut plain = Vec::new();
        for v in &d.declarators {
            let mut v = v.clone();
            for dim in &mut v.dimensions {
                match dim {
                    ArrayDimension::Single(x) => *x = self.expr(x),
                    ArrayDimension::Range { start, end } => {
                        *start = self.expr(start);
                        *end = self.expr(end);
                    }
                }
            }
            let component = canonical_type_name(&d.type_name);
            match &kind {
                // `DIM x AS TType` / `DIM a(1 TO 3) AS TType`.
                Some(t) if !d.is_redim => {
                    let array = !v.dimensions.is_empty();
                    self.record_object_var(&v.name, t, array);
                    out.push(Statement::Dim(DimStatement { span, declarators: vec![v.clone()], type_name: "VARIANT".into(), is_static: d.is_static, is_redim: false }));
                    if array {
                        let mut args = vec![text_at(span, &v.name), text_at(span, t), text_at(span, &self.types.field_names(t))];
                        for dim in &v.dimensions {
                            match dim {
                                ArrayDimension::Single(x) => args.extend([int_at(span, 0), x.clone()]),
                                ArrayDimension::Range { start, end } => args.extend([start.clone(), end.clone()]),
                            }
                        }
                        out.push(assign_at(span, ident_at(span, &v.name), call_at(span, "__objectarray", args)));
                        out.extend(self.init_elements(span, &v.name, v.dimensions.len(), &format!("{t}___init")));
                    } else if !d.is_static {
                        out.extend(self.new_instance(span, ident_at(span, &v.name), text_at(span, &v.name), t));
                    }
                }
                _ => {
                    self.ctx.locals.insert(key(&v.name));
                    if !v.dimensions.is_empty() && is_component_type_name(&component) {
                        self.record_object_var(&v.name, &component.to_ascii_uppercase(), true);
                    }
                    plain.push(v);
                }
            }
        }
        if !plain.is_empty() {
            out.insert(0, Statement::Dim(DimStatement { declarators: plain, ..d.clone() }));
        }
        out
    }

    /// `CREATE x AS TType … END CREATE`: set the instance up, then its
    /// settings (`Caption = …` is `x.Caption = …`, `Center` is `x.Center`);
    /// a nested CREATE gets `x` as its Parent.
    fn create_instance(&mut self, c: &CreateStatement, t: &str) -> Vec<Statement> {
        let span = c.span;
        self.record_object_var(&c.name, t, false);
        let mut out = vec![Statement::Dim(DimStatement {
            span,
            declarators: vec![VariableDeclarator { span, name: c.name.clone(), dimensions: Vec::new() }],
            type_name: "VARIANT".into(),
            is_static: false,
            is_redim: false,
        })];
        out.extend(self.new_instance(span, ident_at(span, &c.name), text_at(span, &c.name), t));
        let member = |m: &str| Expression::MemberAccess(MemberAccessExpression { span, object: Box::new(ident_at(span, &c.name)), member: m.to_string() });
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
                    out.push(call_stmt_at(child.span, "__objset", vec![ident_at(span, &child.name), text_at(span, "parent"), ident_at(span, &c.name)]));
                }
                other => out.extend(self.stmt(other)),
            }
        }
        out
    }

    fn stmt(&mut self, s: &Statement) -> Vec<Statement> {
        match s {
            Statement::Assignment(a) => self.assignment(a),
            Statement::Call(c) => self.call_statement(c),
            Statement::Dim(d) => self.dim(d),
            Statement::With(w) => {
                if matches!(&w.object, Expression::Identifier(id) if self.is_this(&id.name)) || self.object_type(&w.object).is_some_and(|t| self.is_user_type(&t)) {
                    let body = resolve_with_body(&w.body, &w.object);
                    return self.body(&body);
                }
                let object = self.expr(&w.object);
                vec![Statement::With(WithStatement { span: w.span, object, body: self.body(&w.body) })]
            }
            Statement::Print(p) => vec![Statement::Print(PrintStatement { items: p.items.iter().map(|x| self.expr(x)).collect(), ..p.clone() })],
            Statement::If(i) => {
                let condition = self.expr(&i.condition);
                let then_body = self.body(&i.then_body);
                let elseif_branches = i.elseif_branches.iter().map(|b| ElseIfBranch { condition: self.expr(&b.condition), body: self.body(&b.body), ..b.clone() }).collect();
                let else_body = self.body(&i.else_body);
                vec![Statement::If(IfStatement { span: i.span, condition, then_body, elseif_branches, else_body })]
            }
            Statement::For(f) => {
                let (start, end, step) = (self.expr(&f.start), self.expr(&f.end), f.step.as_ref().map(|x| self.expr(x)));
                let body = self.body(&f.body);
                vec![Statement::For(ForStatement { start, end, step, body, ..f.clone() })]
            }
            Statement::While(w) => {
                let condition = self.expr(&w.condition);
                let body = self.body(&w.body);
                vec![Statement::While(WhileStatement { span: w.span, condition, body })]
            }
            Statement::DoLoop(d) => {
                let condition = d.condition.as_ref().map(|x| self.expr(x));
                let body = self.body(&d.body);
                vec![Statement::DoLoop(DoLoopStatement { condition, body, ..d.clone() })]
            }
            Statement::SelectCase(sc) => {
                let expression = self.expr(&sc.expression);
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
                vec![Statement::SelectCase(SelectCaseStatement { span: sc.span, expression, cases, case_else })]
            }
            Statement::Return(r) => vec![Statement::Return(ReturnStatement { span: r.span, value: r.value.as_ref().map(|x| self.expr(x)) })],
            // `BIND p TO obj.Method`: the method's routine.
            Statement::Bind(b) => match self.method_pointer(&b.handler) {
                Some(handler) => vec![Statement::Bind(BindStatement { handler, ..b.clone() })],
                None => vec![s.clone()],
            },
            Statement::Const(c) => {
                self.ctx.locals.insert(key(&c.name));
                vec![Statement::Const(ConstStatement { value: self.expr(&c.value), ..c.clone() })]
            }
            Statement::Create(c) => match self.types.get(&c.type_name).map(|t| t.name.clone()) {
                Some(t) => self.create_instance(c, &t),
                None => {
                    // `Panel(0).Width = 100` inside the block is `c.Panel(0).Width`.
                    let known = |n: &str| {
                        self.routines.contains(&key(n)) || self.globals.contains(&key(n)) || self.is_local(n) || self.array_kind(n).is_some() || (self.is_builtin)(&key(n))
                    };
                    let body = qualify_create_body(&c.body, &c.name, &known);
                    vec![Statement::Create(CreateStatement { body: self.body(&body), ..c.clone() })]
                }
            },
            Statement::Subroutine(sub) => {
                let body = self.routine(&sub.params, &sub.body, None, None);
                vec![Statement::Subroutine(SubroutineStatement { body, ..sub.clone() })]
            }
            Statement::Function(f) => {
                let body = self.routine(&f.params, &f.body, None, None);
                vec![Statement::Function(FunctionStatement { body, ..f.clone() })]
            }
            _ => vec![s.clone()],
        }
    }

    /// Rewrites a routine body with its own locals; `in_type` and `method`
    /// set the TYPE context for methods, CONSTRUCTORs and EVENTs.
    fn routine(&mut self, params: &[Parameter], body: &[Statement], in_type: Option<&str>, method: Option<(String, String)>) -> Vec<Statement> {
        let mut ctx = Ctx { in_main: false, current_type: in_type.map(str::to_string), current_method: method, ..Default::default() };
        for p in params {
            let k = key(strip_suffix(&p.name));
            ctx.locals.insert(k.clone());
            // `obj AS TType`, or `Sender AS QBUTTON`: an object parameter.
            if let Some(kind) = object_kind(self.types, &p.type_name) {
                ctx.local_types.insert(k, kind);
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
        out
    }

    /// `SUB Type___init (This)`: see the module docs.
    fn init_routine(&mut self, t: &str, span: TextSpan) -> SubroutineStatement {
        let mut body = Vec::new();
        let this = || ident_at(span, "This");
        if let Some(kind) = self.types.base_component(t) {
            body.push(call_stmt_at(span, "__objcreate", vec![this(), text_at(span, &kind)]));
        }
        let fields: Vec<TypeField> = self.types.slots(t).into_iter().cloned().collect();
        for (slot, f) in fields.iter().enumerate() {
            // A field of a TYPE that contains this one (itself included)
            // starts empty — `Next AS TNode` — instead of recursing.
            if let Some(field_type) = self.types.get(&f.type_name).map(|d| d.name.clone()) {
                if self.types.contains_type(&field_type, t, &mut HashSet::new()) {
                    let empty = match &f.array_size {
                        Some(upper) => call_at(span, "__newarray", vec![call_at(span, "__null", Vec::new()), f.array_lower.clone().unwrap_or(int_at(span, 0)), upper.clone()]),
                        None => call_at(span, "__null", Vec::new()),
                    };
                    body.push(call_stmt_at(span, "__setfield", vec![this(), int_at(span, slot as i64), empty]));
                    continue;
                }
            }
            let set = |value: Expression| call_stmt_at(span, "__setfield", vec![this(), int_at(span, slot as i64), value]);
            let get = || call_at(span, "__getfield", vec![this(), int_at(span, slot as i64)]);
            let lower = f.array_lower.clone().unwrap_or(int_at(span, 0));
            match (&f.array_size, object_kind(self.types, &f.type_name)) {
                (None, Some(kind)) => match self.types.get(&kind).map(|d| d.name.clone()) {
                    // A TYPE field: its own instance `<id>.<field>`.
                    Some(sub) => {
                        let names = self.types.field_names(&sub);
                        body.push(set(call_at(span, "__newobject", vec![concat_at(span, this(), &format!(".{}", f.name)), text_at(span, &sub), text_at(span, &names)])));
                        body.push(call_stmt_at(span, &format!("{sub}___init"), vec![get()]));
                    }
                    // A component field: its own component `<id>.<field>`.
                    None => {
                        body.push(call_stmt_at(span, "__objcreate", vec![concat_at(span, this(), &format!(".{}", f.name)), text_at(span, &kind)]));
                        body.push(set(concat_at(span, this(), &format!(".{}", f.name))));
                    }
                },
                // `Parts(2) AS TItem` / `Buttons(3) AS QBUTTON`: one object per element.
                (Some(upper), Some(kind)) => {
                    let id = concat_at(span, this(), &format!(".{}", f.name));
                    match self.types.get(&kind).map(|d| d.name.clone()) {
                        Some(sub) => {
                            let names = self.types.field_names(&sub);
                            body.push(set(call_at(span, "__objectarray", vec![id, text_at(span, &sub), text_at(span, &names), lower, upper.clone()])));
                            self.counter += 1;
                            let local = format!("__oa{}", self.counter);
                            // Declared as an array so `local(i)` indexes it.
                            body.push(Statement::Dim(DimStatement {
                                span,
                                declarators: vec![VariableDeclarator { span, name: local.clone(), dimensions: vec![ArrayDimension::Single(int_at(span, 0))] }],
                                type_name: "VARIANT".into(),
                                is_static: false,
                                is_redim: false,
                            }));
                            body.push(assign_at(span, ident_at(span, &local), get()));
                            body.extend(self.init_elements(span, &local, 1, &format!("{sub}___init")));
                        }
                        None => body.push(set(call_at(span, "__component_array", vec![text_at(span, &kind), id, lower, upper.clone()]))),
                    }
                }
                (Some(upper), None) => body.push(set(call_at(span, "__newarray", vec![field_fill(span, &f.type_name), lower, upper.clone()]))),
                (None, None) => body.push(set(field_fill(span, &f.type_name))),
            }
        }
        let chain: Vec<(String, Vec<(String, usize)>, bool)> = self.types.chain(t).iter().map(|d| (d.name.clone(), d.events.clone(), d.has_ctor)).collect();
        for (def, events, _) in &chain {
            for (i, (event, _)) in events.iter().enumerate() {
                // `EVENT Panel.OnClick`: the field's component fires it.
                let (target, event_name) = match event.rsplit_once('.') {
                    Some((field, ev)) => (concat_at(span, this(), &format!(".{field}")), ev.to_string()),
                    None => (this(), event.clone()),
                };
                let ptr = call_at(span, "CODEPTR", vec![ident_at(span, &format!("{def}___ev{i}"))]);
                body.push(call_stmt_at(span, "__bind_event_this", vec![target, text_at(span, &event_name), ptr, this()]));
            }
        }
        for (def, _, has_ctor) in &chain {
            if *has_ctor {
                body.push(call_stmt_at(span, &format!("{def}___ctor"), vec![this()]));
            }
        }
        SubroutineStatement { span, name: format!("{t}___init"), params: vec![this_param()], body }
    }
}

/// Whether `program` has anything this pass rewrites.
fn needs_lowering(program: &Program, types: &Types) -> bool {
    if !types.map.is_empty() {
        return true;
    }
    let found_stmt = std::cell::Cell::new(false);
    let found_expr = std::cell::Cell::new(false);
    walk(
        &program.statements,
        &mut |s| match s {
            Statement::Dim(d) => {
                if is_component_type_name(&canonical_type_name(&d.type_name)) && d.declarators.iter().any(|v| !v.dimensions.is_empty()) {
                    found_stmt.set(true);
                }
            }
            Statement::Subroutine(r) if r.params.iter().any(|p| object_kind(types, &p.type_name).is_some()) => found_stmt.set(true),
            Statement::Function(r) if r.params.iter().any(|p| object_kind(types, &p.type_name).is_some()) => found_stmt.set(true),
            _ => {}
        },
        // `X.Sub(i).Member` (indexed sub-objects) and `Stream.Read`.
        &mut |e| {
            if let Expression::MemberAccess(m) = e {
                if matches!(m.object.as_ref(), Expression::FunctionCall(_)) || m.member.eq_ignore_ascii_case("read") {
                    found_expr.set(true);
                }
            }
        },
    );
    found_stmt.get() || found_expr.get()
}

/// Rewrites `program`'s objects into plain routines and builtin calls (see
/// the module docs). `is_builtin` tells the backend's builtin names apart
/// from a TYPE's implicit members. Programs without objects are returned
/// unchanged.
pub fn lower(program: &Program, is_builtin: &dyn Fn(&str) -> bool) -> Program {
    let types = collect_types(program);
    if !needs_lowering(program, &types) {
        return program.clone();
    }
    let mut globals = HashSet::new();
    let mut routines = HashSet::new();
    let mut component_kinds = HashMap::new();
    walk(
        &program.statements,
        &mut |s| match s {
            Statement::Dim(d) if is_component_type_name(&canonical_type_name(&d.type_name)) => {
                for v in d.declarators.iter().filter(|v| v.dimensions.is_empty()) {
                    component_kinds.insert(key(&v.name), canonical_type_name(&d.type_name).to_ascii_uppercase());
                }
            }
            Statement::Create(c) if is_component_type_name(&canonical_type_name(&c.type_name)) => {
                component_kinds.insert(key(&c.name), canonical_type_name(&c.type_name).to_ascii_uppercase());
            }
            _ => {}
        },
        &mut |_| {},
    );
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
        is_builtin,
        global_types: HashMap::new(),
        global_arrays: HashMap::new(),
        component_kinds,
        globals,
        routines,
        ctx: Ctx { in_main: true, ..Default::default() },
        counter: 0,
    };
    // Main-program objects are known before any routine uses them.
    for s in &program.statements {
        match s {
            Statement::Dim(d) => {
                let kind = object_kind(&types, &d.type_name);
                for v in &d.declarators {
                    match (&kind, v.dimensions.is_empty()) {
                        (Some(k), true) if types.get(k).is_some() => {
                            l.global_types.insert(key(&v.name), k.clone());
                        }
                        (Some(k), false) => {
                            l.global_arrays.insert(key(&v.name), k.clone());
                        }
                        _ => {}
                    }
                }
            }
            Statement::Create(c) => {
                if let Some(t) = types.get(&c.type_name) {
                    l.global_types.insert(key(&c.name), t.name.clone());
                }
            }
            _ => {}
        }
    }

    let mut out = Vec::new();
    for s in &program.statements {
        let Statement::Type(t) = s else { continue };
        let tname = t.name.clone();
        for m in &t.methods {
            match m {
                Statement::Subroutine(sub) => {
                    let routine = mangle(&tname, &sub.name);
                    let body = l.routine(&sub.params, &sub.body, Some(&tname), Some((sub.name.clone(), routine.clone())));
                    let params = std::iter::once(this_param()).chain(sub.params.iter().cloned()).collect();
                    out.push(Statement::Subroutine(SubroutineStatement { span: sub.span, name: routine, params, body }));
                }
                Statement::Function(f) => {
                    let routine = mangle(&tname, &f.name);
                    let body = l.routine(&f.params, &f.body, Some(&tname), Some((f.name.clone(), routine.clone())));
                    let params = std::iter::once(this_param()).chain(f.params.iter().cloned()).collect();
                    out.push(Statement::Function(FunctionStatement { span: f.span, name: routine, params, return_type: f.return_type.clone(), body }));
                }
                _ => {}
            }
        }
        if !t.constructor.is_empty() {
            let body = l.routine(&[], &t.constructor, Some(&tname), None);
            out.push(Statement::Subroutine(SubroutineStatement { span: t.span, name: format!("{tname}___ctor"), params: vec![this_param()], body }));
        }
        for (i, e) in t.events.iter().enumerate() {
            let body = l.routine(&e.params, &e.body, Some(&tname), None);
            let params = std::iter::once(this_param()).chain(e.params.iter().cloned()).collect();
            out.push(Statement::Subroutine(SubroutineStatement { span: e.span, name: format!("{tname}___ev{i}"), params, body }));
        }
        let init = l.init_routine(&tname, t.span);
        out.push(Statement::Subroutine(init));
    }
    for s in &program.statements {
        match s {
            Statement::Type(_) => {}
            other => out.extend(l.stmt(other)),
        }
    }
    Program { span: program.span, statements: out }
}

/// The builtins this pass emits (each backend implements them).
pub const OBJECT_BUILTINS: &[&str] = &[
    "__newobject", "__getfield", "__setfield", "__objectarray", "__newarray", "__aget", "__aset", "__null",
    "__objget", "__objset", "__objcall", "__objcreate", "__component_array", "__bind_event", "__bind_event_this",
];

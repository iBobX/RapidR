//! RapidR Rust code generator.
//!
//! Walks the AST produced by `rapidr-parser` and emits readable Rust
//! source code targeting the `rapidr-runtime-core` library.

use std::collections::{HashMap, HashSet};
use std::fmt::Write;

use rapidr_ast::*;

/// Generate a complete Rust `main.rs` from a parsed RapidP program.
pub fn generate(program: &Program) -> String {
    let mut gen = RustCodegen::new();
    gen.emit_program(program);
    gen.output
}

struct RustCodegen {
    output: String,
    indent: usize,
    /// Names of subs/functions defined at the top level.
    defined_functions: HashSet<String>,
    /// Names of variables declared with DIM at top level.
    top_level_vars: HashSet<String>,
    /// Names of variables declared as arrays (DIM with dimensions).
    array_vars: HashSet<String>,
    /// User-defined TYPE names (lowercase) → struct name for DIM default generation.
    user_types: HashSet<String>,
    /// Per-UDT, which fields are arrays: type_name(lowercase) → set of field names(lowercase).
    udt_array_fields: HashMap<String, HashSet<String>>,
    /// Variable name (lowercase) → UDT type name (original case) for DIM'd UDT vars.
    var_udt_type: HashMap<String, String>,
    /// Tracks the current CREATE nesting stack (object name).
    create_stack: Vec<String>,
    /// Whether we're inside a FUNCTION body (name → return-var tracking).
    current_function: Option<String>,
    /// Component variable names (lowercase) → type name (UPPERCASE), e.g. "form1" → "RFORM".
    component_vars: HashMap<String, String>,
    /// Stack of component names for WITH blocks targeting components.
    with_component_stack: Vec<String>,
    /// All variable names referenced in the program (for implicit variable detection).
    all_referenced_vars: HashSet<String>,
    /// Sub/function name (lowercase) → parameter count.
    function_param_counts: HashMap<String, usize>,
    /// DECLARE'd FFI function names (lowercase) → (alias, lib, params, return_type).
    declared_functions: HashSet<String>,
    /// Array variable name (lowercase) → (default_value_str, size_expr_str) for re-declaring in subs.
    array_init_info: HashMap<String, (String, String)>,
    /// Whether we are inside a SUB or FUNCTION body (as opposed to top-level / main).
    in_sub_or_function: bool,
}

impl RustCodegen {
    fn new() -> Self {
        Self {
            output: String::with_capacity(4096),
            indent: 0,
            defined_functions: HashSet::new(),
            top_level_vars: HashSet::new(),
            array_vars: HashSet::new(),
            user_types: HashSet::new(),
            udt_array_fields: HashMap::new(),
            var_udt_type: HashMap::new(),
            create_stack: Vec::new(),
            current_function: None,
            component_vars: HashMap::new(),
            with_component_stack: Vec::new(),
            all_referenced_vars: HashSet::new(),
            function_param_counts: HashMap::new(),
            declared_functions: HashSet::new(),
            array_init_info: HashMap::new(),
            in_sub_or_function: false,
        }
    }

    /// Check if a variable name (lowercase) is a known component variable.
    fn is_component_var(&self, name: &str) -> bool {
        self.component_vars.contains_key(&name.to_lowercase())
    }

    /// Extract the component variable name from an expression, if it's a component identifier.
    fn get_component_name(&self, expr: &Expression) -> Option<String> {
        if let Expression::Identifier(id) = expr {
            let lower = id.name.to_lowercase();
            if self.component_vars.contains_key(&lower) {
                return Some(to_snake(&strip_type_suffix(&id.name)));
            }
        }
        None
    }

    /// Check if a variable is a module-level scalar (DIM at top level, not component, not array, not UDT).
    fn is_global_scalar(&self, name: &str) -> bool {
        let lower = name.to_lowercase();
        self.top_level_vars.contains(&lower)
            && !self.component_vars.contains_key(&lower)
            && !self.array_vars.contains(&lower)
            && !self.user_types.contains(&lower)
            && !self.var_udt_type.contains_key(&lower)
    }

    /// Check if a variable is a module-level array (DIM at top level with dimensions).
    fn is_global_array(&self, name: &str) -> bool {
        let lower = name.to_lowercase();
        self.top_level_vars.contains(&lower)
            && self.array_vars.contains(&lower)
            && !self.component_vars.contains_key(&lower)
    }

    // --- output helpers ---

    fn line(&mut self, s: &str) {
        for _ in 0..self.indent {
            self.output.push_str("    ");
        }
        self.output.push_str(s);
        self.output.push('\n');
    }

    fn blank(&mut self) {
        self.output.push('\n');
    }

    fn write_indent(&mut self) {
        for _ in 0..self.indent {
            self.output.push_str("    ");
        }
    }

    /// Emit rp_bind_event / rp_bind_event_N depending on handler arity.
    fn emit_bind_event_call(&mut self, comp_name: &str, event: &str, handler: &str) {
        let handler_lower = handler.to_lowercase();
        let arity = self.function_param_counts.get(&handler_lower).copied().unwrap_or(0);
        self.write_indent();
        match arity {
            0 => { let _ = writeln!(self.output, "rp_bind_event(\"{comp_name}\", \"{event}\", {handler});"); }
            1 => { let _ = writeln!(self.output, "rp_bind_event_1(\"{comp_name}\", \"{event}\", {handler});"); }
            2 => { let _ = writeln!(self.output, "rp_bind_event_2(\"{comp_name}\", \"{event}\", {handler});"); }
            3 => { let _ = writeln!(self.output, "rp_bind_event_3(\"{comp_name}\", \"{event}\", {handler});"); }
            4 => { let _ = writeln!(self.output, "rp_bind_event_4(\"{comp_name}\", \"{event}\", {handler});"); }
            _ => { let _ = writeln!(self.output, "rp_bind_event_5(\"{comp_name}\", \"{event}\", {handler});"); }
        }
    }

    // --- program ---

    fn emit_program(&mut self, program: &Program) {
        // First pass: collect top-level function/sub names, variable names, TYPE and component defs
        for stmt in &program.statements {
            match stmt {
                Statement::Subroutine(s) => {
                    self.defined_functions.insert(s.name.to_lowercase());
                    self.function_param_counts.insert(s.name.to_lowercase(), s.params.len());
                    // Scan body for local component DIMs
                    for body_stmt in &s.body {
                        if let Statement::Dim(d) = body_stmt {
                            for decl in &d.declarators {
                                if is_component_type_name(&d.type_name) {
                                    self.component_vars.insert(decl.name.to_lowercase(), d.type_name.to_uppercase());
                                }
                            }
                        }
                    }
                }
                Statement::Function(f) => {
                    self.defined_functions.insert(f.name.to_lowercase());
                    self.function_param_counts.insert(f.name.to_lowercase(), f.params.len());
                    // Scan body for local component DIMs
                    for body_stmt in &f.body {
                        if let Statement::Dim(d) = body_stmt {
                            for decl in &d.declarators {
                                if is_component_type_name(&d.type_name) {
                                    self.component_vars.insert(decl.name.to_lowercase(), d.type_name.to_uppercase());
                                }
                            }
                        }
                    }
                }
                Statement::Dim(d) => {
                    for decl in &d.declarators {
                        let name_lower = decl.name.to_lowercase();
                        self.top_level_vars.insert(name_lower.clone());
                        if !decl.dimensions.is_empty() {
                            self.array_vars.insert(name_lower.clone());
                            // Store init info for re-declaring in subs
                            let size = match decl.dimensions.first() {
                                Some(ArrayDimension::Single(expr)) => {
                                    format!("(({}).to_i64() + 1) as usize", self.expr_to_string(expr))
                                }
                                Some(ArrayDimension::Range { start: _, end }) => {
                                    format!("(({}).to_i64() + 1) as usize", self.expr_to_string(end))
                                }
                                None => "0usize".to_string(),
                            };
                            let default = default_value_for_type(&d.type_name);
                            self.array_init_info.insert(name_lower.clone(), (default, size));
                        }
                        // Track if this variable is a UDT instance
                        if self.user_types.contains(&d.type_name.to_lowercase()) {
                            self.var_udt_type.insert(
                                name_lower.clone(),
                                d.type_name.clone(),
                            );
                        }
                        // Track if this is a component variable
                        if is_component_type_name(&d.type_name) {
                            self.component_vars.insert(name_lower, d.type_name.to_uppercase());
                        }
                    }
                }
                Statement::Type(t) => {
                    let type_lower = t.name.to_lowercase();
                    self.user_types.insert(type_lower.clone());
                    // Track which fields are arrays
                    let mut arr_fields = HashSet::new();
                    for field in &t.fields {
                        if field.array_size.is_some() {
                            arr_fields.insert(field.name.to_lowercase());
                        }
                    }
                    if !arr_fields.is_empty() {
                        self.udt_array_fields.insert(type_lower, arr_fields);
                    }
                }
                Statement::Create(c) => {
                    // Register CREATE targets as component variables
                    self.component_vars.insert(c.name.to_lowercase(), c.type_name.to_uppercase());
                    self.top_level_vars.insert(c.name.to_lowercase());
                    collect_nested_creates(&c.body, &mut self.component_vars, &mut self.top_level_vars);
                }
                Statement::Const(c) => {
                    self.top_level_vars.insert(c.name.to_lowercase());
                }
                Statement::Declare(d) => {
                    let name_lower = d.name.to_lowercase();
                    self.declared_functions.insert(name_lower.clone());
                    self.defined_functions.insert(name_lower.clone());
                    self.function_param_counts.insert(name_lower, d.params.len());
                }
                _ => {}
            }
        }

        // Second pass: collect all referenced variable names for implicit variable detection
        collect_all_refs(&program.statements, &mut self.all_referenced_vars);

        self.line("use rapidr_runtime_core::prelude::*;");
        self.line("use std::cell::RefCell;");
        self.line("use std::collections::HashMap;");
        self.blank();

        // Emit global variable helpers for module-level DIM variables
        self.line("thread_local! {");
        self.line("    static GVARS: RefCell<HashMap<String, Value>> = RefCell::new(HashMap::new());");
        self.line("    static GARRS: RefCell<HashMap<String, Vec<Value>>> = RefCell::new(HashMap::new());");
        self.line("}");
        self.line("fn gv(n: &str) -> Value { GVARS.with(|g| g.borrow().get(n).cloned().unwrap_or(v_null())) }");
        self.line("fn gs(n: &str, v: Value) { GVARS.with(|g| g.borrow_mut().insert(n.to_string(), v)); }");
        self.line("fn ga_get(n: &str, i: usize) -> Value { GARRS.with(|g| g.borrow().get(n).and_then(|a| a.get(i).cloned()).unwrap_or(v_null())) }");
        self.line("fn ga_set(n: &str, i: usize, v: Value) { GARRS.with(|g| { let mut m = g.borrow_mut(); if let Some(a) = m.get_mut(n) { if i < a.len() { a[i] = v; } } }); }");
        self.line("#[allow(dead_code)]");
        self.line("fn ga_init(n: &str, sz: usize, d: Value) { GARRS.with(|g| g.borrow_mut().insert(n.to_string(), vec![d; sz])); }");
        self.line("#[allow(dead_code)]");
        self.line("fn ga_len(n: &str) -> usize { GARRS.with(|g| g.borrow().get(n).map(|a| a.len()).unwrap_or(0)) }");
        self.blank();

        // Emit subs/functions/declares before main
        for stmt in &program.statements {
            match stmt {
                Statement::Subroutine(_) | Statement::Function(_) | Statement::Type(_) | Statement::Declare(_) => {
                    self.emit_statement(stmt);
                    self.blank();
                }
                _ => {}
            }
        }

        // Emit main
        self.line("fn main() {");
        self.indent += 1;

        // Auto-declare implicit variables (referenced but never DIM'd)
        let mut implicit: Vec<String> = self.all_referenced_vars.iter()
            .filter(|name| {
                !self.top_level_vars.contains(name.as_str())
                    && !self.defined_functions.contains(name.as_str())
                    && !self.component_vars.contains_key(name.as_str())
                    && !matches!(name.as_str(), "true" | "false" | "vttrue" | "vtfalse" | "pi" | "_with_")
                    && builtin_function_call(name, &[]).is_none()
            })
            .cloned()
            .collect();
        implicit.sort();
        for name in &implicit {
            let snake = to_snake(name);
            self.write_indent();
            let _ = writeln!(self.output, "let mut {snake} = v_null();");
        }
        if !implicit.is_empty() {
            self.blank();
        }

        for stmt in &program.statements {
            match stmt {
                Statement::Subroutine(_) | Statement::Function(_) | Statement::Type(_) | Statement::Declare(_) => {
                    // already emitted above
                }
                _ => self.emit_statement(stmt),
            }
        }

        self.indent -= 1;
        self.line("}");
    }

    // --- statements ---

    fn emit_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Directive(d) => {
                // Handle $THEME directive — emit set_theme() call
                if d.name.eq_ignore_ascii_case("$THEME") || d.name.eq_ignore_ascii_case("THEME") {
                    if let Some(ref val) = d.value {
                        let theme_lower = val.to_lowercase();
                        self.line(&format!("set_theme(\"{}\");", theme_lower));
                    }
                }
                // Other directives like $TYPECHECK, $APPTYPE are compile-time; skip
            }
            Statement::Dim(d) => self.emit_dim(d),
            Statement::Const(c) => self.emit_const(c),
            Statement::Assignment(a) => self.emit_assignment(a),
            Statement::Print(p) => self.emit_print(p),
            Statement::Call(c) => self.emit_call(c),
            Statement::If(i) => self.emit_if(i),
            Statement::For(f) => self.emit_for(f),
            Statement::While(w) => self.emit_while(w),
            Statement::DoLoop(d) => self.emit_do_loop(d),
            Statement::SelectCase(s) => self.emit_select_case(s),
            Statement::Subroutine(s) => self.emit_sub(s),
            Statement::Function(f) => self.emit_function(f),
            Statement::Type(t) => self.emit_type_def(t),
            Statement::Create(c) => self.emit_create(c),
            Statement::With(w) => self.emit_with(w),
            Statement::Exit(e) => self.emit_exit(e),
            Statement::Return(r) => self.emit_return(r),
            Statement::Import(i) => self.emit_import(i),
            Statement::Input(i) => self.emit_input(i),
            Statement::Bind(b) => self.emit_bind(b),
            Statement::Declare(d) => self.emit_declare(d),
            Statement::Open(o) => self.emit_open(o),
            Statement::Close(c) => self.emit_close(c),
            Statement::PrintHash(p) => self.emit_print_hash(p),
            Statement::WriteHash(w) => self.emit_write_hash(w),
            Statement::Seek(s) => self.emit_seek(s),
            Statement::Comment(c) => {
                self.write_indent();
                let _ = writeln!(self.output, "// {}", c.text);
            }
            Statement::Line(l) => {
                self.write_indent();
                let _ = writeln!(self.output, "// UNHANDLED: {}", l.text);
            }
            Statement::RustBlock(rb) => {
                // Emit raw Rust code verbatim
                for line in rb.code.lines() {
                    self.write_indent();
                    let _ = writeln!(self.output, "{}", line);
                }
            }
        }
    }

    fn emit_dim(&mut self, d: &DimStatement) {
        for decl in &d.declarators {
            let name = to_snake(&decl.name);
            let name_lower = decl.name.to_lowercase();
            self.array_vars.remove(&name_lower); // re-insert if has dims

            // Component variable → create via registry
            if self.component_vars.contains_key(&name_lower) {
                let type_name = self.component_vars[&name_lower].clone();
                self.write_indent();
                let _ = writeln!(self.output, "rp_create_component(\"{name}\", \"{type_name}\");");
                if type_name == "RTIMER" {
                    self.write_indent();
                    let _ = writeln!(self.output, "gui_register_timer(\"{name}\");");
                }
                continue;
            }

            if decl.dimensions.is_empty() {
                // Check if this is a UDT type
                if self.user_types.contains(&d.type_name.to_lowercase()) {
                    self.write_indent();
                    let _ = writeln!(self.output, "let mut {name} = {}::default();", d.type_name);
                    self.var_udt_type.insert(name_lower, d.type_name.clone());
                } else if !self.in_sub_or_function && self.top_level_vars.contains(&name_lower) {
                    // Module-level scalar → store in global vars
                    let default = default_value_for_type(&d.type_name);
                    self.write_indent();
                    let _ = writeln!(self.output, "gs(\"{name}\", {default});");
                } else {
                    let default = default_value_for_type(&d.type_name);
                    self.write_indent();
                    let _ = writeln!(self.output, "let mut {name} = {default};");
                }
            } else {
                self.array_vars.insert(decl.name.to_lowercase());
                // Array declaration — compute size expression
                let size = match decl.dimensions.first() {
                    Some(ArrayDimension::Single(expr)) => {
                        format!("(({}).to_i64() + 1) as usize", self.expr_to_string(expr))
                    }
                    Some(ArrayDimension::Range { start: _start, end }) => {
                        // Over-allocate to end+1 so direct indexing works
                        // (BASIC indices are used as-is, e.g., DIM B(1 TO 5) → B(5) is index 5)
                        format!(
                            "(({}).to_i64() + 1) as usize",
                            self.expr_to_string(end),
                        )
                    }
                    None => "0usize".to_string(),
                };
                let default = default_value_for_type(&d.type_name);
                if !self.in_sub_or_function && self.top_level_vars.contains(&name_lower) {
                    // Module-level array → store in global arrays
                    self.write_indent();
                    let _ = writeln!(self.output, "ga_init(\"{name}\", {size}, {default});");
                } else {
                    self.write_indent();
                    let _ = writeln!(
                        self.output,
                        "let mut {name} = vec![{default}; {size}];",
                    );
                }
            }
        }
    }

    fn emit_const(&mut self, c: &ConstStatement) {
        let name = to_snake(&c.name);
        let val = self.owned_expr(&c.value);
        if !self.in_sub_or_function && self.top_level_vars.contains(&c.name.to_lowercase()) {
            self.write_indent();
            let _ = writeln!(self.output, "gs(\"{name}\", {val});");
        } else {
            self.write_indent();
            let _ = writeln!(self.output, "let {name} = {val};");
        }
    }

    fn emit_assignment(&mut self, a: &AssignmentStatement) {
        // Check if this is a FUNCTION return pattern: FuncName = expr
        if let Some(fname) = self.current_function.clone() {
            if let Expression::Identifier(id) = &a.target {
                if id.name.eq_ignore_ascii_case(&fname) {
                    let val = self.owned_expr(&a.value);
                    let fname_lc = fname.to_lowercase();
                    self.write_indent();
                    let _ = writeln!(self.output, "_{fname_lc} = {val};");
                    return;
                }
            }
        }

        // Assignment to bare component variable: comp = expr → evaluate for side effects
        if let Expression::Identifier(id) = &a.target {
            if self.is_component_var(&id.name) {
                let val = self.owned_expr(&a.value);
                self.write_indent();
                let _ = writeln!(self.output, "let _ = {val};");
                return;
            }
        }

        // Component property assignment: comp.Property = value
        if let Expression::MemberAccess(ma) = &a.target {
            if let Some(comp_name) = self.get_component_name(&ma.object) {
                let prop = ma.member.to_lowercase();
                let value = self.owned_expr(&a.value);
                // Event binding: comp.OnClick = handler
                if prop.starts_with("on") {
                    let handler = match &a.value {
                        Expression::Identifier(id) => to_snake(&strip_type_suffix(&id.name)),
                        _ => value.clone(),
                    };
                    self.emit_bind_event_call(&comp_name, &prop, &handler);
                    return;
                }
                // Parent assignment: comp.Parent = otherComp → store name as string
                if prop == "parent" {
                    if let Some(parent_name) = self.get_component_name(&a.value) {
                        self.write_indent();
                        let _ = writeln!(self.output, "rp_comp_set(\"{comp_name}\", \"parent\", v_str(\"{parent_name}\"));");
                        return;
                    }
                }
                self.write_indent();
                let _ = writeln!(self.output, "rp_comp_set(\"{comp_name}\", \"{prop}\", {value});");
                return;
            }
            // Nested member: comp.Sub.Prop = value → rp_comp_set(comp, "sub.prop", value)
            if let Expression::MemberAccess(inner_ma) = ma.object.as_ref() {
                if let Some(comp_name) = self.get_component_name(&inner_ma.object) {
                    let sub = inner_ma.member.to_lowercase();
                    let prop = ma.member.to_lowercase();
                    let value = self.owned_expr(&a.value);
                    self.write_indent();
                    let _ = writeln!(self.output, "rp_comp_set(\"{comp_name}\", \"{sub}.{prop}\", {value});");
                    return;
                }
            }
            // WITH-dot on component: _with_.Property = value
            if let Expression::Identifier(id) = ma.object.as_ref() {
                if id.name == "_with_" {
                    if let Some(with_comp) = self.with_component_stack.last().cloned() {
                        let prop = ma.member.to_lowercase();
                        let value = self.owned_expr(&a.value);
                        if prop.starts_with("on") {
                            let handler = match &a.value {
                                Expression::Identifier(hid) => to_snake(&strip_type_suffix(&hid.name)),
                                _ => value.clone(),
                            };
                            self.emit_bind_event_call(&with_comp, &prop, &handler);
                            return;
                        }
                        self.write_indent();
                        let _ = writeln!(self.output, "rp_comp_set(\"{with_comp}\", \"{prop}\", {value});");
                        return;
                    }
                }
            }
        }

        // Inside CREATE block: bare identifier = value → component property
        if !self.create_stack.is_empty() {
            if let Expression::Identifier(id) = &a.target {
                let obj = self.create_stack.last().unwrap().clone();
                let prop = id.name.to_lowercase();
                let value = self.owned_expr(&a.value);
                if prop.starts_with("on") {
                    let handler = match &a.value {
                        Expression::Identifier(hid) => to_snake(&strip_type_suffix(&hid.name)),
                        _ => value.clone(),
                    };
                    self.emit_bind_event_call(&obj, &prop, &handler);
                    return;
                }
                if prop == "parent" {
                    if let Some(parent_name) = self.get_component_name(&a.value) {
                        self.write_indent();
                        let _ = writeln!(self.output, "rp_comp_set(\"{obj}\", \"parent\", v_str(\"{parent_name}\"));");
                        return;
                    }
                }
                self.write_indent();
                let _ = writeln!(self.output, "rp_comp_set(\"{obj}\", \"{prop}\", {value});");
                return;
            }
            // Inside CREATE: MemberAccess target like Font.Size = value → sub-property
            if let Expression::MemberAccess(ma) = &a.target {
                if let Expression::Identifier(sub_id) = ma.object.as_ref() {
                    let obj = self.create_stack.last().unwrap().clone();
                    let sub = sub_id.name.to_lowercase();
                    let prop = ma.member.to_lowercase();
                    let value = self.owned_expr(&a.value);
                    self.write_indent();
                    let _ = writeln!(self.output, "rp_comp_set(\"{obj}\", \"{sub}.{prop}\", {value});");
                    return;
                }
            }
        }

        // Global scalar assignment: gs("name", value)
        if let Expression::Identifier(id) = &a.target {
            let stripped = strip_type_suffix(&id.name);
            if self.is_global_scalar(&stripped) {
                let snake = to_snake(&stripped);
                let value = self.owned_expr(&a.value);
                self.write_indent();
                let _ = writeln!(self.output, "gs(\"{snake}\", {value});");
                return;
            }
        }
        // Global array element assignment: ga_set("name", idx, value)
        if let Expression::FunctionCall(fc) = &a.target {
            if let Expression::Identifier(id) = fc.callee.as_ref() {
                let stripped = strip_type_suffix(&id.name);
                if self.is_global_array(&stripped) {
                    let snake = to_snake(&stripped);
                    let idx = fc.args.first()
                        .map(|a| self.owned_expr(a))
                        .unwrap_or_else(|| "v_int(0)".to_string());
                    let value = self.owned_expr(&a.value);
                    self.write_indent();
                    let _ = writeln!(self.output, "ga_set(\"{snake}\", ({idx}).to_i64() as usize, {value});");
                    return;
                }
            }
        }

        let target = self.lvalue_to_string(&a.target);
        let value = self.owned_expr(&a.value);
        self.write_indent();
        let _ = writeln!(self.output, "{target} = {value};");
    }

    fn emit_print(&mut self, p: &PrintStatement) {
        let items: Vec<String> = p.items.iter().map(|e| self.owned_expr(e)).collect();
        if items.is_empty() {
            self.line("rp_print(&[], true);");
            return;
        }
        let items_str = items.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ");
        self.write_indent();
        let _ = writeln!(
            self.output,
            "rp_print(&[{items_str}], {});",
            p.append_newline
        );
    }

    fn emit_call(&mut self, c: &CallStatement) {
        // --- Component method dispatch ---

        // 1. MethodCall on component: SQLite.Query(Q)
        if let Expression::MethodCall(mc) = &c.callee {
            if let Some(comp_name) = self.get_component_name(&mc.object) {
                let method = mc.method.to_lowercase();
                let mut all_args: Vec<String> = mc.args.iter().map(|a| self.owned_expr(a)).collect();
                all_args.extend(c.args.iter().map(|a| self.owned_expr(a)));
                let args_str = all_args.join(", ");
                self.write_indent();
                if all_args.is_empty() {
                    let _ = writeln!(self.output, "rp_comp_method(\"{comp_name}\", \"{method}\", &[]);");
                } else {
                    let _ = writeln!(self.output, "rp_comp_method(\"{comp_name}\", \"{method}\", &[{args_str}]);");
                }
                return;
            }
        }

        // 2. FunctionCall(MemberAccess(comp, method), args): SQLite.Row(0) as statement
        if let Expression::FunctionCall(fc) = &c.callee {
            if let Expression::MemberAccess(ma) = fc.callee.as_ref() {
                if let Some(comp_name) = self.get_component_name(&ma.object) {
                    let method = ma.member.to_lowercase();
                    let mut all_args: Vec<String> = fc.args.iter().map(|a| self.owned_expr(a)).collect();
                    all_args.extend(c.args.iter().map(|a| self.owned_expr(a)));
                    let args_str = all_args.join(", ");
                    self.write_indent();
                    if all_args.is_empty() {
                        let _ = writeln!(self.output, "rp_comp_method(\"{comp_name}\", \"{method}\", &[]);");
                    } else {
                        let _ = writeln!(self.output, "rp_comp_method(\"{comp_name}\", \"{method}\", &[{args_str}]);");
                    }
                    return;
                }
            }
        }

        // 3. MemberAccess on component: ListBox1.Clear, sock.close, ListBox1.AddItems expr
        if let Expression::MemberAccess(ma) = &c.callee {
            if let Some(comp_name) = self.get_component_name(&ma.object) {
                let method = ma.member.to_lowercase();
                let args: Vec<String> = c.args.iter().map(|a| self.owned_expr(a)).collect();
                let args_str = args.join(", ");
                self.write_indent();
                if args.is_empty() {
                    let _ = writeln!(self.output, "rp_comp_method(\"{comp_name}\", \"{method}\", &[]);");
                } else {
                    let _ = writeln!(self.output, "rp_comp_method(\"{comp_name}\", \"{method}\", &[{args_str}]);");
                }
                return;
            }
            // WITH-dot component method: .Method inside WITH on component
            if let Expression::Identifier(id) = ma.object.as_ref() {
                if id.name == "_with_" {
                    if let Some(with_comp) = self.with_component_stack.last().cloned() {
                        let method = ma.member.to_lowercase();
                        let args: Vec<String> = c.args.iter().map(|a| self.owned_expr(a)).collect();
                        let args_str = args.join(", ");
                        self.write_indent();
                        if args.is_empty() {
                            let _ = writeln!(self.output, "rp_comp_method(\"{with_comp}\", \"{method}\", &[]);");
                        } else {
                            let _ = writeln!(self.output, "rp_comp_method(\"{with_comp}\", \"{method}\", &[{args_str}]);");
                        }
                        return;
                    }
                }
            }
        }

        // Inside CREATE: bare identifier calls → component method on CREATE target
        if !self.create_stack.is_empty() {
            if let Expression::Identifier(id) = &c.callee {
                if !self.defined_functions.contains(&id.name.to_lowercase()) {
                    let obj = self.create_stack.last().unwrap().clone();
                    let method = id.name.to_lowercase();
                    let args: Vec<String> = c.args.iter().map(|a| self.owned_expr(a)).collect();
                    let args_str = args.join(", ");
                    self.write_indent();
                    if args.is_empty() {
                        let _ = writeln!(self.output, "rp_comp_method(\"{obj}\", \"{method}\", &[]);");
                    } else {
                        let _ = writeln!(self.output, "rp_comp_method(\"{obj}\", \"{method}\", &[{args_str}]);");
                    }
                    return;
                }
            }
        }

        // --- Standard call handling ---
        let callee = self.expr_to_string(&c.callee);
        let args: Vec<String> = c.args.iter().map(|e| self.owned_expr(e)).collect();

        let callee_lower = match &c.callee {
            Expression::Identifier(id) => id.name.to_lowercase(),
            _ => String::new(),
        };

        if callee_lower == "showmessage" {
            let args_str = args.join(", ");
            self.write_indent();
            let _ = writeln!(self.output, "rp_showmessage(&{args_str});");
            return;
        }

        // Try mapping through builtin_function_call for known builtins used as statements
        if let Some(result) = builtin_function_call(&callee_lower, &args) {
            self.write_indent();
            let _ = writeln!(self.output, "{result};");
            return;
        }

        let args_str = args.join(", ");
        self.write_indent();
        let _ = writeln!(self.output, "{callee}({args_str});");
    }

    fn emit_if(&mut self, i: &IfStatement) {
        let cond = self.expr_to_string(&i.condition);
        self.write_indent();
        let _ = writeln!(self.output, "if ({cond}).to_bool() {{");
        self.indent += 1;
        for s in &i.then_body {
            self.emit_statement(s);
        }
        self.indent -= 1;

        for branch in &i.elseif_branches {
            let cond = self.expr_to_string(&branch.condition);
            self.write_indent();
            let _ = writeln!(self.output, "}} else if ({cond}).to_bool() {{");
            self.indent += 1;
            for s in &branch.body {
                self.emit_statement(s);
            }
            self.indent -= 1;
        }

        if !i.else_body.is_empty() {
            self.line("} else {");
            self.indent += 1;
            for s in &i.else_body {
                self.emit_statement(s);
            }
            self.indent -= 1;
        }
        self.line("}");
    }

    fn emit_for(&mut self, f: &ForStatement) {
        let var = to_snake(&f.variable);
        let start = self.owned_expr(&f.start);
        let end = self.owned_expr(&f.end);
        let step = f
            .step
            .as_ref()
            .map(|e| self.owned_expr(e))
            .unwrap_or_else(|| "v_int(1)".to_string());

        let is_global = self.is_global_scalar(&f.variable);
        if is_global {
            self.write_indent();
            let _ = writeln!(self.output, "gs(\"{var}\", {start});");
            self.write_indent();
            let _ = writeln!(self.output, "while (gv(\"{var}\").rp_le(&{end})).to_bool() {{");
            self.indent += 1;
            for s in &f.body {
                self.emit_statement(s);
            }
            self.write_indent();
            let _ = writeln!(self.output, "gs(\"{var}\", &gv(\"{var}\") + &{step});");
        } else {
            self.write_indent();
            let _ = writeln!(self.output, "{var} = {start};");
            self.write_indent();
            let _ = writeln!(self.output, "while ({var}.rp_le(&{end})).to_bool() {{");
            self.indent += 1;
            for s in &f.body {
                self.emit_statement(s);
            }
            self.write_indent();
            let _ = writeln!(self.output, "{var} = &{var} + &{step};");
        }
        self.indent -= 1;
        self.line("}");
    }

    fn emit_while(&mut self, w: &WhileStatement) {
        let cond = self.expr_to_string(&w.condition);
        self.write_indent();
        let _ = writeln!(self.output, "while ({cond}).to_bool() {{");
        self.indent += 1;
        for s in &w.body {
            self.emit_statement(s);
        }
        self.indent -= 1;
        self.line("}");
    }

    fn emit_do_loop(&mut self, d: &DoLoopStatement) {
        if d.pre_condition {
            let cond = d
                .condition
                .as_ref()
                .map(|e| self.expr_to_string(e))
                .unwrap_or_else(|| "v_bool(true)".to_string());
            if d.is_until {
                self.write_indent();
                let _ = writeln!(self.output, "while !({cond}).to_bool() {{");
            } else {
                self.write_indent();
                let _ = writeln!(self.output, "while ({cond}).to_bool() {{");
            }
            self.indent += 1;
            for s in &d.body {
                self.emit_statement(s);
            }
            self.indent -= 1;
            self.line("}");
        } else {
            // Post-condition or infinite loop
            self.line("loop {");
            self.indent += 1;
            for s in &d.body {
                self.emit_statement(s);
            }
            if let Some(cond_expr) = &d.condition {
                let cond = self.expr_to_string(cond_expr);
                if d.is_until {
                    self.write_indent();
                    let _ = writeln!(self.output, "if ({cond}).to_bool() {{ break; }}");
                } else {
                    self.write_indent();
                    let _ = writeln!(self.output, "if !({cond}).to_bool() {{ break; }}");
                }
            }
            self.indent -= 1;
            self.line("}");
        }
    }

    fn emit_select_case(&mut self, s: &SelectCaseStatement) {
        let expr = self.expr_to_string(&s.expression);
        self.write_indent();
        let _ = writeln!(self.output, "let _select_val = {expr};");
        let mut first = true;
        for case in &s.cases {
            let conditions: Vec<String> = case
                .values
                .iter()
                .map(|v| {
                    let v = self.expr_to_string(v);
                    format!("_select_val == {v}")
                })
                .collect();
            let keyword = if first { "if" } else { "} else if" };
            first = false;
            self.write_indent();
            let _ = writeln!(self.output, "{keyword} {} {{", conditions.join(" || "));
            self.indent += 1;
            for stmt in &case.body {
                self.emit_statement(stmt);
            }
            self.indent -= 1;
        }
        if !s.case_else.is_empty() {
            self.line("} else {");
            self.indent += 1;
            for stmt in &s.case_else {
                self.emit_statement(stmt);
            }
            self.indent -= 1;
        }
        if !first {
            self.line("}");
        }
    }

    fn emit_sub(&mut self, s: &SubroutineStatement) {
        let name = to_snake(&s.name);
        let params = self.emit_params(&s.params);
        self.write_indent();
        let _ = writeln!(self.output, "fn {name}({params}) {{");
        self.indent += 1;

        self.in_sub_or_function = true;
        // Auto-declare local variables for refs in body that aren't params
        self.emit_local_vars(&s.body, &s.params);

        for stmt in &s.body {
            self.emit_statement(stmt);
        }
        self.in_sub_or_function = false;
        self.indent -= 1;
        self.line("}");
    }

    fn emit_function(&mut self, f: &FunctionStatement) {
        let name = to_snake(&f.name);
        let params = self.emit_params(&f.params);
        self.write_indent();
        let _ = writeln!(self.output, "fn {name}({params}) -> Value {{");
        self.indent += 1;

        // BASIC FUNCTION return pattern: `FuncName = value`
        let ret_default = f
            .return_type
            .as_ref()
            .map(|t| default_value_for_type(t))
            .unwrap_or_else(|| "v_null()".to_string());
        self.write_indent();
        let _ = writeln!(self.output, "let mut _{name} = {ret_default};");

        self.in_sub_or_function = true;
        // Auto-declare local variables for refs in body that aren't params
        self.emit_local_vars(&f.body, &f.params);

        self.current_function = Some(f.name.clone());
        for stmt in &f.body {
            self.emit_statement(stmt);
        }
        self.current_function = None;
        self.in_sub_or_function = false;

        self.write_indent();
        let _ = writeln!(self.output, "_{name}");
        self.indent -= 1;
        self.line("}");
    }

    /// Emit local variable declarations for undeclared refs inside a sub/function body.
    fn emit_local_vars(&mut self, body: &[Statement], params: &[Parameter]) {
        let param_names: HashSet<String> = params
            .iter()
            .map(|p| strip_type_suffix(&p.name).to_lowercase())
            .collect();
        let mut local_refs = HashSet::new();
        collect_all_refs(body, &mut local_refs);
        let mut locals: Vec<String> = local_refs
            .iter()
            .filter(|name| {
                !param_names.contains(name.as_str())
                    && !self.defined_functions.contains(name.as_str())
                    && !self.component_vars.contains_key(name.as_str())
                    && !self.is_global_scalar(name)
                    && !self.is_global_array(name)
                    && !matches!(
                        name.as_str(),
                        "true" | "false" | "vttrue" | "vtfalse" | "pi" | "_with_"
                    )
                    && builtin_function_call(name, &[]).is_none()
            })
            .cloned()
            .collect();
        locals.sort();
        for local in &locals {
            let snake = to_snake(local);
            self.write_indent();
            if let Some((default, size)) = self.array_init_info.get(local.as_str()) {
                // Only emit local array if not a global array
                if !self.is_global_array(local) {
                    let _ = writeln!(self.output, "let mut {snake} = vec![{default}; {size}];");
                }
            } else if let Some(_type_name) = self.var_udt_type.get(local.as_str()) {
                let _ = writeln!(self.output, "let mut {snake} = {}::default();", _type_name);
            } else {
                let _ = writeln!(self.output, "let mut {snake} = v_null();");
            }
        }
    }

    fn emit_params(&self, params: &[Parameter]) -> String {
        params
            .iter()
            .map(|p| {
                let name = to_snake(&p.name);
                format!("{name}: Value")
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn emit_type_def(&mut self, t: &TypeStatement) {
        let name = &t.name;
        self.line("#[derive(Debug, Clone)]");
        self.write_indent();
        let _ = writeln!(self.output, "struct {name} {{");
        self.indent += 1;
        for field in &t.fields {
            let fname = to_snake(&field.name);
            self.write_indent();
            if field.array_size.is_some() {
                let _ = writeln!(self.output, "{fname}: Vec<Value>,");
            } else {
                let _ = writeln!(self.output, "{fname}: Value,");
            }
        }
        self.indent -= 1;
        self.line("}");
        self.blank();

        // Default impl
        self.write_indent();
        let _ = writeln!(self.output, "impl Default for {name} {{");
        self.indent += 1;
        self.line("fn default() -> Self {");
        self.indent += 1;
        self.line("Self {");
        self.indent += 1;
        for field in &t.fields {
            let fname = to_snake(&field.name);
            let default = default_value_for_type(&field.type_name);
            self.write_indent();
            if let Some(ref size_expr) = field.array_size {
                let size = self.expr_to_string(size_expr);
                let _ = writeln!(self.output, "{fname}: vec![{default}; ({size}).to_i64() as usize + 1],");
            } else {
                let _ = writeln!(self.output, "{fname}: {default},");
            }
        }
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
    }

    fn emit_create(&mut self, c: &CreateStatement) {
        let name = to_snake(&c.name);
        let type_upper = c.type_name.to_uppercase();
        self.write_indent();
        let _ = writeln!(self.output, "rp_create_component(\"{name}\", \"{type_upper}\");");

        // Set parent if inside another CREATE
        if let Some(parent) = self.create_stack.last().cloned() {
            self.write_indent();
            let _ = writeln!(self.output, "rp_comp_set(\"{name}\", \"parent\", v_str(\"{parent}\"));");
        }

        self.create_stack.push(name.clone());
        for stmt in &c.body {
            self.emit_statement(stmt);
        }
        self.create_stack.pop();

        // Register timers declared in CREATE blocks
        if type_upper == "RTIMER" {
            self.write_indent();
            let _ = writeln!(self.output, "gui_register_timer(\"{name}\");");
        }
    }

    fn emit_with(&mut self, w: &WithStatement) {
        // Check if the WITH target is a component variable
        if let Some(comp_name) = self.get_component_name(&w.object) {
            self.write_indent();
            let _ = writeln!(self.output, "{{ // WITH {comp_name}");
            self.indent += 1;
            self.with_component_stack.push(comp_name.clone());
            for stmt in &w.body {
                self.emit_statement(stmt);
            }
            self.with_component_stack.pop();
            self.indent -= 1;
            self.line("} // END WITH");
        } else {
            let obj = self.expr_to_string(&w.object);
            self.write_indent();
            let _ = writeln!(self.output, "{{ // WITH {obj}");
            self.indent += 1;
            self.write_indent();
            let _ = writeln!(self.output, "let _with_ = &mut {obj};");
            for stmt in &w.body {
                self.emit_statement(stmt);
            }
            self.indent -= 1;
            self.line("} // END WITH");
        }
    }

    fn emit_exit(&mut self, e: &ExitStatement) {
        match e.exit_type.as_str() {
            "FOR" | "WHILE" | "DO" => self.line("break;"),
            "SUB" => self.line("return;"),
            "FUNCTION" => {
                if let Some(fname) = self.current_function.clone() {
                    let fname_lc = fname.to_lowercase();
                    self.write_indent();
                    let _ = writeln!(
                        self.output,
                        "return _{fname_lc};",
                    );
                } else {
                    self.line("return v_null();");
                }
            }
            _ => {
                self.write_indent();
                let _ = writeln!(self.output, "// EXIT {}", e.exit_type);
            }
        }
    }

    fn emit_return(&mut self, r: &ReturnStatement) {
        if let Some(val) = &r.value {
            let v = self.expr_to_string(val);
            self.write_indent();
            let _ = writeln!(self.output, "return {v};");
        } else {
            self.line("return;");
        }
    }

    fn emit_import(&mut self, i: &ImportStatement) {
        // IMPORT "math" → use std::f64::consts::{PI, E, ...}
        let module = i.module_name.trim_matches('"');
        match module {
            "math" => {
                self.line("// IMPORT math — constants available via std::f64::consts");
            }
            "numpy" | "pandas" | "matplotlib" => {
                self.write_indent();
                let _ = writeln!(self.output, "// IMPORT \"{module}\" — available via R{} component",
                    match module { "numpy" => "NumPy", "pandas" => "Pandas", _ => "MatPlotLib" });
            }
            _ => {
                self.write_indent();
                let _ = writeln!(self.output, "// IMPORT \"{module}\" (not yet implemented)");
            }
        }
    }

    fn emit_input(&mut self, i: &InputStatement) {
        // Check if target is a global scalar
        if let Expression::Identifier(id) = &i.target {
            let stripped = strip_type_suffix(&id.name);
            if self.is_global_scalar(&stripped) {
                let snake = to_snake(&stripped);
                if let Some(prompt) = &i.prompt {
                    let prompt = self.expr_to_string(prompt);
                    self.write_indent();
                    let _ = writeln!(self.output, "gs(\"{snake}\", rp_input(&{prompt}));");
                } else {
                    self.write_indent();
                    let _ = writeln!(self.output, "gs(\"{snake}\", rp_input(&v_str(\"\")));");
                }
                return;
            }
        }
        let target = self.lvalue_to_string(&i.target);
        if let Some(prompt) = &i.prompt {
            let prompt = self.expr_to_string(prompt);
            self.write_indent();
            let _ = writeln!(self.output, "{target} = rp_input(&{prompt});");
        } else {
            self.write_indent();
            let _ = writeln!(self.output, "{target} = rp_input(&v_str(\"\"));");
        }
    }

    fn emit_bind(&mut self, b: &BindStatement) {
        // Check if target is a component event
        if let Expression::MemberAccess(ma) = &b.target {
            if let Some(comp_name) = self.get_component_name(&ma.object) {
                let event = ma.member.to_lowercase();
                let handler = match &b.handler {
                    Expression::Identifier(id) => to_snake(&strip_type_suffix(&id.name)),
                    _ => self.expr_to_string(&b.handler),
                };
                self.emit_bind_event_call(&comp_name, &event, &handler);
                return;
            }
        }
        let target = self.expr_to_string(&b.target);
        let handler = self.expr_to_string(&b.handler);
        self.write_indent();
        let _ = writeln!(
            self.output,
            "// BIND {target} TO {handler} (not a component event)"
        );
    }

    fn emit_declare(&mut self, d: &DeclareStatement) {
        // Generate a stub function that maps the DECLARE'd FFI func to a runtime call
        let name = to_snake(&d.name);
        let alias = d.alias.as_deref().unwrap_or(&d.name);
        let params: Vec<String> = d.params.iter().enumerate().map(|(i, _)| format!("arg{i}: Value")).collect();
        let param_names: Vec<String> = (0..d.params.len()).map(|i| format!("arg{i}")).collect();
        let params_str = params.join(", ");

        let ret_type_str = d.return_type.as_deref().unwrap_or("");

        // If a LIB is specified, emit a wrapper that calls ffi_call at runtime
        if let Some(ref lib_path) = d.lib {
            let lib_clean = lib_path.trim_matches('"');
            let alias_clean = alias.trim_matches('"');
            let args_list = param_names.iter()
                .map(|n| format!("{n}.clone()"))
                .collect::<Vec<_>>()
                .join(", ");

            if d.is_function {
                self.write_indent();
                let _ = writeln!(self.output, "fn {name}({params_str}) -> Value {{");
                self.indent += 1;
                self.write_indent();
                let _ = writeln!(self.output, "ffi_call(\"{lib_clean}\", \"{alias_clean}\", &[{args_list}], \"{ret_type_str}\")");
                self.indent -= 1;
                self.line("}");
            } else {
                self.write_indent();
                let _ = writeln!(self.output, "fn {name}({params_str}) {{");
                self.indent += 1;
                self.write_indent();
                let _ = writeln!(self.output, "ffi_call(\"{lib_clean}\", \"{alias_clean}\", &[{args_list}], \"\");");
                self.indent -= 1;
                self.line("}");
            }
            return;
        }

        // No LIB specified — try mapping the alias to a known builtin
        let alias_lower = alias.to_lowercase();
        let body = match alias_lower.as_str() {
            "sqrt" => "rp_sqr(&arg0)".to_string(),
            "sin" => "rp_sin(&arg0)".to_string(),
            "cos" => "rp_cos(&arg0)".to_string(),
            "tan" => "rp_tan(&arg0)".to_string(),
            "abs" => "rp_abs(&arg0)".to_string(),
            "log" => "rp_log(&arg0)".to_string(),
            "exp" => "rp_exp(&arg0)".to_string(),
            "ceil" => "rp_ceil(&arg0)".to_string(),
            "floor" => "rp_floor(&arg0)".to_string(),
            "round" => "rp_round(&arg0)".to_string(),
            "randint" if d.params.len() >= 2 => {
                "rp_int(&(&(&rp_rnd(&v_int(0)) * &(&arg1 - &arg0)) + &arg0))".to_string()
            }
            _ => {
                if d.is_function {
                    format!("eprintln!(\"[WARN] FFI function {name}() not available\"); v_null()")
                } else {
                    format!("eprintln!(\"[WARN] FFI sub {name}() not available\")")
                }
            }
        };

        if d.is_function {
            self.write_indent();
            let _ = writeln!(self.output, "fn {name}({params_str}) -> Value {{");
            self.indent += 1;
            self.write_indent();
            let _ = writeln!(self.output, "{body}");
            self.indent -= 1;
            self.line("}");
        } else {
            self.write_indent();
            let _ = writeln!(self.output, "fn {name}({params_str}) {{");
            self.indent += 1;
            self.write_indent();
            let _ = writeln!(self.output, "{body}");
            self.indent -= 1;
            self.line("}");
        }
    }

    // --- File I/O statement codegen ---

    fn emit_open(&mut self, o: &OpenStatement) {
        let filename = self.expr_to_string(&o.filename);
        let fnum = self.expr_to_string(&o.file_number);
        self.write_indent();
        let _ = writeln!(
            self.output,
            "rp_open(&{filename}, &v_str(\"{}\"), &{fnum});",
            o.mode
        );
    }

    fn emit_close(&mut self, c: &CloseStatement) {
        let fnum = self.expr_to_string(&c.file_number);
        self.write_indent();
        let _ = writeln!(self.output, "rp_close(&{fnum});");
    }

    fn emit_print_hash(&mut self, p: &PrintHashStatement) {
        let fnum = self.expr_to_string(&p.file_number);
        if p.items.is_empty() {
            self.write_indent();
            let _ = writeln!(self.output, "rp_print_hash(&{fnum}, &[]);");
        } else {
            let items: Vec<String> = p.items.iter().map(|e| self.expr_to_string(e)).collect();
            let args = items.join(", ");
            self.write_indent();
            let _ = writeln!(self.output, "rp_print_hash(&{fnum}, &[{args}]);");
        }
    }

    fn emit_write_hash(&mut self, w: &WriteHashStatement) {
        let fnum = self.expr_to_string(&w.file_number);
        if w.items.is_empty() {
            self.write_indent();
            let _ = writeln!(self.output, "rp_write_hash(&{fnum}, &[]);");
        } else {
            let items: Vec<String> = w.items.iter().map(|e| self.expr_to_string(e)).collect();
            let args = items.join(", ");
            self.write_indent();
            let _ = writeln!(self.output, "rp_write_hash(&{fnum}, &[{args}]);");
        }
    }

    fn emit_seek(&mut self, s: &SeekStatement) {
        let fnum = self.expr_to_string(&s.file_number);
        let pos = self.expr_to_string(&s.position);
        self.write_indent();
        let _ = writeln!(self.output, "rp_seek(&{fnum}, &{pos});");
    }

    // --- expression codegen ---

    /// Emit an lvalue expression (assignment target) — no `.clone()`.
    fn lvalue_to_string(&self, expr: &Expression) -> String {
        match expr {
            Expression::Identifier(id) => {
                let name = strip_type_suffix(&id.name);
                to_snake(&name)
            }
            Expression::ArrayAccess(aa) => {
                let arr = self.lvalue_to_string(&aa.array);
                let idx = aa
                    .indices
                    .iter()
                    .map(|e| self.expr_to_string(e))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{arr}[({idx}).to_i64() as usize]")
            }
            Expression::MemberAccess(ma) => {
                let obj = self.lvalue_to_string(&ma.object);
                let member = to_snake(&ma.member);
                format!("{obj}.{member}")
            }
            // FunctionCall as lvalue: array assignment, e.g. A(0) = 42
            // Also handles UDT array field: r.Names(1) = "First" → FunctionCall(MemberAccess(r, Names), [1])
            Expression::FunctionCall(fc) => {
                if let Expression::Identifier(id) = fc.callee.as_ref() {
                    let name = strip_type_suffix(&id.name).to_lowercase();
                    if self.array_vars.contains(&name) {
                        let arr = to_snake(&name);
                        let idx = fc.args.first()
                            .map(|a| self.expr_to_string(a))
                            .unwrap_or_else(|| "0".to_string());
                        return format!("{arr}[({idx}).to_i64() as usize]");
                    }
                }
                // UDT array field assignment: FunctionCall(MemberAccess(obj, field), [idx])
                if let Expression::MemberAccess(ma) = fc.callee.as_ref() {
                    if let Expression::Identifier(id) = ma.object.as_ref() {
                        let var_lower = id.name.to_lowercase();
                        if let Some(type_name) = self.var_udt_type.get(&var_lower) {
                            let type_lower = type_name.to_lowercase();
                            let field_lower = ma.member.to_lowercase();
                            if let Some(arr_fields) = self.udt_array_fields.get(&type_lower) {
                                if arr_fields.contains(&field_lower) {
                                    let obj = to_snake(&strip_type_suffix(&id.name));
                                    let field = to_snake(&ma.member);
                                    let idx = fc.args.first()
                                        .map(|a| self.expr_to_string(a))
                                        .unwrap_or_else(|| "0".to_string());
                                    return format!("{obj}.{field}[({idx}).to_i64() as usize]");
                                }
                            }
                        }
                    }
                }
                self.expr_to_string(expr)
            }
            // MethodCall as lvalue: UDT array field assignment, e.g. r.Names(1) = "First"
            Expression::MethodCall(mc) => {
                if let Expression::Identifier(id) = mc.object.as_ref() {
                    let var_lower = id.name.to_lowercase();
                    if let Some(type_name) = self.var_udt_type.get(&var_lower) {
                        let type_lower = type_name.to_lowercase();
                        let method_lower = mc.method.to_lowercase();
                        if let Some(arr_fields) = self.udt_array_fields.get(&type_lower) {
                            if arr_fields.contains(&method_lower) {
                                let obj = self.lvalue_to_string(&mc.object);
                                let field = to_snake(&mc.method);
                                let idx = mc.args.first()
                                    .map(|a| self.expr_to_string(a))
                                    .unwrap_or_else(|| "0".to_string());
                                return format!("{obj}.{field}[({idx}).to_i64() as usize]");
                            }
                        }
                    }
                }
                self.expr_to_string(expr)
            }
            other => self.expr_to_string(other),
        }
    }

    /// Emit an expression as an owned Value (adds .clone() for bare identifiers).
    fn owned_expr(&self, expr: &Expression) -> String {
        let s = self.expr_to_string(expr);
        // Identifiers need .clone() to avoid move; constructors/literals are already owned.
        // Global vars (gv(...)) already return owned values.
        if matches!(expr, Expression::Identifier(_)) {
            if let Expression::Identifier(id) = expr {
                let stripped = strip_type_suffix(&id.name);
                if self.is_global_scalar(&stripped) || self.is_component_var(&stripped) {
                    return s; // gv() and v_str() already return owned values
                }
            }
            format!("{s}.clone()")
        } else {
            s
        }
    }

    fn expr_to_string(&self, expr: &Expression) -> String {
        match expr {
            Expression::Literal(lit) => match &lit.value {
                LiteralValue::Integer(n) => format!("v_int({n})"),
                LiteralValue::Float(n) => format!("v_dbl({n:?})"),
                LiteralValue::String(s) => {
                    let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
                    format!("v_str(\"{escaped}\")")
                }
            },
            Expression::Identifier(id) => {
                let name_lower = id.name.to_lowercase();
                // Known implicit identifiers
                match name_lower.as_str() {
                    "true" | "vttrue" => "v_bool(true)".to_string(),
                    "false" | "vtfalse" => "v_bool(false)".to_string(),
                    "pi" => "v_dbl(std::f64::consts::PI)".to_string(),
                    "time$" => "rp_time()".to_string(),
                    "date$" => "rp_date()".to_string(),
                    "command$" => "rp_command()".to_string(),
                    _ => {
                        let name = strip_type_suffix(&id.name);
                        let snake = to_snake(&name);
                        // Component var used as bare expression → emit its name as a string value
                        if self.component_vars.contains_key(&name.to_lowercase()) {
                            return format!("v_str(\"{snake}\")");
                        }
                        // Module-level scalar → read from global storage
                        if self.is_global_scalar(&name) {
                            return format!("gv(\"{snake}\")");
                        }
                        snake
                    }
                }
            }
            Expression::Binary(b) => {
                let left = self.expr_to_string(&b.left);
                let right = self.expr_to_string(&b.right);
                match b.operator {
                    BinaryOperator::Add => format!("(&{left} + &{right})"),
                    BinaryOperator::Subtract => format!("(&{left} - &{right})"),
                    BinaryOperator::Multiply => format!("(&{left} * &{right})"),
                    BinaryOperator::Divide => format!("(&{left} / &{right})"),
                    BinaryOperator::IntegerDivide => format!("{left}.int_div(&{right})"),
                    BinaryOperator::Modulo => format!("(&{left} % &{right})"),
                    BinaryOperator::Power => format!("{left}.power(&{right})"),
                    BinaryOperator::Concat => format!("{left}.concat(&{right})"),
                    BinaryOperator::Equal => format!("{left}.rp_eq(&{right})"),
                    BinaryOperator::NotEqual => format!("{left}.rp_ne(&{right})"),
                    BinaryOperator::LessThan => format!("{left}.rp_lt(&{right})"),
                    BinaryOperator::LessThanOrEqual => format!("{left}.rp_le(&{right})"),
                    BinaryOperator::GreaterThan => format!("{left}.rp_gt(&{right})"),
                    BinaryOperator::GreaterThanOrEqual => format!("{left}.rp_ge(&{right})"),
                    BinaryOperator::And => format!("{left}.and(&{right})"),
                    BinaryOperator::Or => format!("{left}.or(&{right})"),
                    BinaryOperator::Xor => format!("{left}.xor(&{right})"),
                }
            }
            Expression::Unary(u) => {
                let operand = self.expr_to_string(&u.operand);
                match u.operator {
                    UnaryOperator::Negate => format!("(-&{operand})"),
                    UnaryOperator::Positive => operand,
                    UnaryOperator::Not => format!("{operand}.not()"),
                }
            }
            Expression::FunctionCall(fc) => {
                let args: Vec<String> =
                    fc.args.iter().map(|a| self.owned_expr(a)).collect();

                // Check for well-known builtin function names or array access
                if let Expression::Identifier(id) = fc.callee.as_ref() {
                    let name_stripped = strip_type_suffix(&id.name).to_lowercase();

                    // Check if this is an array access (DIM'd with dimensions)
                    if self.array_vars.contains(&name_stripped) {
                        let arr = to_snake(&name_stripped);
                        let idx = args.first().map(|s| s.as_str()).unwrap_or("0");
                        // Module-level array → read from global storage
                        if self.is_global_array(&name_stripped) {
                            return format!("ga_get(\"{arr}\", ({idx}).to_i64() as usize)");
                        }
                        return format!("{arr}[({idx}).to_i64() as usize].clone()");
                    }

                    if let Some(rust_call) = builtin_function_call(&name_stripped, &args) {
                        return rust_call;
                    }
                    // Check if it's a known function/sub or declared FFI function
                    if self.defined_functions.contains(&name_stripped) {
                        let fname = to_snake(&strip_type_suffix(&id.name));
                        let args_str = args.join(", ");
                        return format!("{fname}({args_str})");
                    }
                    // Not a known function — treat as variant array indexing
                    let varname = to_snake(&strip_type_suffix(&id.name));
                    if args.len() == 1 {
                        return format!("{varname}.rp_index(&{})", args[0]);
                    }
                    // Multiple args or no args — fall through to regular call
                    let fname = to_snake(&strip_type_suffix(&id.name));
                    let args_str = args.join(", ");
                    return format!("{fname}({args_str})");
                }

                // Check for UDT array field access: r.Names(1) → FunctionCall(MemberAccess(r, Names), [1])
                if let Expression::MemberAccess(ma) = fc.callee.as_ref() {
                    // Component method: SQLite.Row(0) → FunctionCall(MemberAccess(SQLite, Row), [0])
                    if let Some(comp_name) = self.get_component_name(&ma.object) {
                        let method = ma.member.to_lowercase();
                        let args_str = args.join(", ");
                        if args.is_empty() {
                            return format!("rp_comp_method(\"{comp_name}\", \"{method}\", &[])");
                        }
                        return format!("rp_comp_method(\"{comp_name}\", \"{method}\", &[{args_str}])");
                    }

                    if let Expression::Identifier(id) = ma.object.as_ref() {
                        let var_lower = id.name.to_lowercase();
                        // UDT array field access
                        if let Some(type_name) = self.var_udt_type.get(&var_lower) {
                            let type_lower = type_name.to_lowercase();
                            let field_lower = ma.member.to_lowercase();
                            if let Some(arr_fields) = self.udt_array_fields.get(&type_lower) {
                                if arr_fields.contains(&field_lower) {
                                    let obj = to_snake(&strip_type_suffix(&id.name));
                                    let field = to_snake(&ma.member);
                                    let idx = args.first().map(|s| s.as_str()).unwrap_or("0");
                                    return format!("{obj}.{field}[({idx}).to_i64() as usize].clone()");
                                }
                            }
                        }

                        // Static component type method: RNum.sin(x) → builtin route
                        if is_component_type_name(&id.name) || var_lower == "math" {
                            let method_lower = ma.member.to_lowercase();
                            // Try mapping to a builtin
                            if let Some(rust_call) = builtin_function_call(&method_lower, &args) {
                                return rust_call;
                            }
                            // Otherwise, warn and return null
                            return format!("{{ eprintln!(\"[WARN] {}.{}() not implemented\"); v_null() }}", id.name, ma.member);
                        }
                    }
                }

                // Method call or complex callee
                let callee = self.expr_to_string(fc.callee.as_ref());
                let args_str = args.join(", ");
                format!("{callee}({args_str})")
            }
            Expression::MemberAccess(ma) => {
                // Component property/method access
                if let Some(comp_name) = self.get_component_name(&ma.object) {
                    let member_lower = ma.member.to_lowercase();
                    if is_component_method_name(&member_lower) {
                        return format!("rp_comp_method(\"{comp_name}\", \"{member_lower}\", &[])");
                    }
                    return format!("rp_comp_get(\"{comp_name}\", \"{member_lower}\")");
                }

                // Nested component member: comp.Sub.Prop → rp_comp_get("comp", "sub.prop")
                if let Expression::MemberAccess(inner_ma) = ma.object.as_ref() {
                    if let Some(comp_name) = self.get_component_name(&inner_ma.object) {
                        let sub = inner_ma.member.to_lowercase();
                        let prop = ma.member.to_lowercase();
                        return format!("rp_comp_get(\"{comp_name}\", \"{sub}.{prop}\")");
                    }
                }

                // WITH-dot on component: _with_.Property
                if let Expression::Identifier(id) = ma.object.as_ref() {
                    if id.name == "_with_" {
                        if let Some(with_comp) = self.with_component_stack.last() {
                            let member_lower = ma.member.to_lowercase();
                            if is_component_method_name(&member_lower) {
                                return format!("rp_comp_method(\"{with_comp}\", \"{member_lower}\", &[])");
                            }
                            return format!("rp_comp_get(\"{with_comp}\", \"{member_lower}\")");
                        }
                    }
                }

                let obj_str = self.expr_to_string(&ma.object);
                let member = to_snake(&ma.member);
                let member_lower = ma.member.to_lowercase();

                // WITH-dot expansion fallback
                if obj_str == "_with_" {
                    return format!("_with_.{member}");
                }

                // Handle math module access (e.g. math.pi)
                if obj_str == "math" {
                    match member.as_str() {
                        "pi" => return "v_dbl(std::f64::consts::PI)".to_string(),
                        "e" => return "v_dbl(std::f64::consts::E)".to_string(),
                        _ => {}
                    }
                }

                // Check if object is a UDT variable → use field access
                if let Expression::Identifier(id) = ma.object.as_ref() {
                    let var_lower = id.name.to_lowercase();
                    if self.var_udt_type.contains_key(&var_lower) {
                        return format!("{obj_str}.{member}");
                    }
                }

                // Generic fallback: use rp_comp_get via string name
                // This handles event handler params typed as components (e.g. `client.host`)
                format!("rp_comp_get(&{obj_str}.to_string_val(), \"{member_lower}\")")
            }
            Expression::MethodCall(mc) => {
                // Component method call: comp.Method(args)
                if let Some(comp_name) = self.get_component_name(&mc.object) {
                    let method = mc.method.to_lowercase();
                    let args: Vec<String> = mc.args.iter().map(|a| self.owned_expr(a)).collect();
                    let args_str = args.join(", ");
                    if args.is_empty() {
                        return format!("rp_comp_method(\"{comp_name}\", \"{method}\", &[])");
                    }
                    return format!("rp_comp_method(\"{comp_name}\", \"{method}\", &[{args_str}])");
                }
                // Check if this is a UDT array field access, e.g. r.Names(1)
                if let Expression::Identifier(id) = mc.object.as_ref() {
                    let var_lower = id.name.to_lowercase();
                    if let Some(type_name) = self.var_udt_type.get(&var_lower) {
                        let type_lower = type_name.to_lowercase();
                        let method_lower = mc.method.to_lowercase();
                        if let Some(arr_fields) = self.udt_array_fields.get(&type_lower) {
                            if arr_fields.contains(&method_lower) {
                                let obj = self.expr_to_string(&mc.object);
                                let field = to_snake(&mc.method);
                                let idx = mc.args.first()
                                    .map(|a| self.expr_to_string(a))
                                    .unwrap_or_else(|| "0".to_string());
                                return format!("{obj}.{field}[({idx}).to_i64() as usize].clone()");
                            }
                        }
                    }
                }
                // Fallback: assume object holds a component instance name (Value)
                let obj = self.owned_expr(&mc.object);
                let method_lower = mc.method.to_lowercase();
                let args: Vec<String> =
                    mc.args.iter().map(|a| self.owned_expr(a)).collect();
                if args.is_empty() {
                    format!("rp_comp_method(&{obj}.to_string_val(), \"{method_lower}\", &[])")
                } else {
                    let args_str = args.join(", ");
                    format!("rp_comp_method(&{obj}.to_string_val(), \"{method_lower}\", &[{args_str}])")
                }
            }
            Expression::ArrayAccess(aa) => {
                let arr = self.expr_to_string(&aa.array);
                let idx = aa
                    .indices
                    .iter()
                    .map(|e| self.expr_to_string(e))
                    .collect::<Vec<_>>()
                    .join(", ");
                // Check if the array is a known Vec<Value> array variable
                let is_array = if let Expression::Identifier(id) = aa.array.as_ref() {
                    self.array_vars.contains(&id.name.to_lowercase())
                } else {
                    false
                };
                if is_array {
                    format!("{arr}[({idx}).to_i64() as usize]")
                } else {
                    // Value-based indexing
                    format!("{arr}.rp_index(&{idx})")
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn to_snake(name: &str) -> String {
    // Strip type suffixes first
    let name = strip_type_suffix(name);
    // Just lowercase for now since BASIC names are case-insensitive
    let lower = name.to_lowercase();
    // Escape Rust reserved keywords by prefixing with r#
    // (raw identifier syntax) or appending underscore
    match lower.as_str() {
        "fn" | "let" | "mut" | "ref" | "type" | "use" | "mod" | "pub" | "as" | "in"
        | "if" | "else" | "for" | "while" | "loop" | "match" | "return" | "break"
        | "continue" | "struct" | "enum" | "impl" | "trait" | "where" | "self"
        | "super" | "crate" | "const" | "static" | "extern" | "unsafe" | "async"
        | "await" | "dyn" | "abstract" | "become" | "box" | "do" | "final" | "macro"
        | "override" | "priv" | "try" | "typeof" | "unsized" | "virtual" | "yield"
        | "move" | "main" => format!("{lower}_"),
        _ => lower,
    }
}

fn strip_type_suffix(name: &str) -> String {
    let mut s = name.to_string();
    if s.ends_with('$') || s.ends_with('%') || s.ends_with('#') || s.ends_with('&') || s.ends_with('!') {
        s.pop();
    }
    s
}

fn default_value_for_type(type_name: &str) -> String {
    match type_name.to_uppercase().as_str() {
        "INTEGER" | "BYTE" | "WORD" | "DWORD" | "LONG" | "INT64" => "v_int(0)".to_string(),
        "DOUBLE" | "SINGLE" | "CURRENCY" => "v_dbl(0.0)".to_string(),
        "STRING" => "v_str(\"\")".to_string(),
        _ => "v_null()".to_string(),
    }
}

/// Map a known BASIC builtin function name to a Rust runtime call.
fn builtin_function_call(name: &str, args: &[String]) -> Option<String> {
    // Most builtins take a single argument as &Value
    let a0 = args.first().map(|s| s.as_str()).unwrap_or("&v_null()");
    let a1 = args.get(1).map(|s| s.as_str()).unwrap_or("&v_null()");
    let a2 = args.get(2).map(|s| s.as_str()).unwrap_or("&v_null()");

    match name {
        "len" => Some(format!("rp_len(&{a0})")),
        "mid" => Some(format!("rp_mid(&{a0}, &{a1}, &{a2})")),
        "left" => Some(format!("rp_left(&{a0}, &{a1})")),
        "right" => Some(format!("rp_right(&{a0}, &{a1})")),
        "ucase" => Some(format!("rp_ucase(&{a0})")),
        "lcase" => Some(format!("rp_lcase(&{a0})")),
        "ltrim" => Some(format!("rp_ltrim(&{a0})")),
        "rtrim" => Some(format!("rp_rtrim(&{a0})")),
        "trim" => Some(format!("rp_trim(&{a0})")),
        "instr" => Some(format!("rp_instr(&{a0}, &{a1}, &{a2})")),
        "space" => Some(format!("rp_space(&{a0})")),
        "string" => Some(format!("rp_string_func(&{a0}, &{a1})")),
        "chr" => Some(format!("rp_chr(&{a0})")),
        "asc" => Some(format!("rp_asc(&{a0})")),
        "replace" => Some(format!("rp_replace(&{a0}, &{a1}, &{a2})")),
        "str" => Some(format!("rp_str(&{a0})")),
        "val" => Some(format!("rp_val(&{a0})")),
        "int" => Some(format!("rp_int(&{a0})")),
        "abs" => Some(format!("rp_abs(&{a0})")),
        "sgn" => Some(format!("rp_sgn(&{a0})")),
        "sqr" => Some(format!("rp_sqr(&{a0})")),
        "sin" => Some(format!("rp_sin(&{a0})")),
        "cos" => Some(format!("rp_cos(&{a0})")),
        "tan" => Some(format!("rp_tan(&{a0})")),
        "atn" => Some(format!("rp_atn(&{a0})")),
        "acos" => Some(format!("rp_acos(&{a0})")),
        "asin" => Some(format!("rp_asin(&{a0})")),
        "log" => Some(format!("rp_log(&{a0})")),
        "exp" => Some(format!("rp_exp(&{a0})")),
        "ceil" => Some(format!("rp_ceil(&{a0})")),
        "floor" => Some(format!("rp_floor(&{a0})")),
        "round" => Some(format!("rp_round(&{a0})")),
        "hex" => Some(format!("rp_hex(&{a0})")),
        "oct" => Some(format!("rp_oct(&{a0})")),
        "bin" => Some(format!("rp_bin(&{a0})")),
        "rnd" => Some(format!("rp_rnd(&{a0})")),
        "timer" => Some("rp_timer()".to_string()),
        "isnumeric" => Some(format!("rp_isnumeric(&{a0})")),
        "sleep" => Some(format!("rp_sleep(&{a0})")),
        "command" => Some("rp_command()".to_string()),
        "environ" => Some(format!("rp_environ(&{a0})")),
        "doevents" => Some("rp_doevents()".to_string()),
        "end" => Some("rp_end()".to_string()),
        "showmessage" => Some(format!("rp_showmessage(&{a0})")),
        "msgbox" => Some(format!("rp_msgbox(&{a0})")),
        "direxists" => Some(format!("rp_direxists(&{a0})")),
        "fileexists" => Some(format!("rp_fileexists(&{a0})")),
        "dir" => Some(format!("rp_dir(&{a0}, &{a1})")),
        "input" | "input_func" => Some(format!("rp_input(&{a0})")),

        // --- Phase 3: New builtins ---

        // Math / conversion
        "fix" => Some(format!("rp_fix(&{a0})")),
        "frac" => Some(format!("rp_frac(&{a0})")),
        "cint" => Some(format!("rp_cint(&{a0})")),
        "clng" => Some(format!("rp_clng(&{a0})")),
        "cdbl" => Some(format!("rp_cdbl(&{a0})")),
        "csng" => Some(format!("rp_csng(&{a0})")),
        "iif" => Some(format!("rp_iif(&{a0}, &{a1}, &{a2})")),
        "hextodec" => Some(format!("rp_hextodec(&{a0})")),
        "convbase" => Some(format!("rp_convbase(&{a0}, &{a1}, &{a2})")),
        "rgb" => Some(format!("rp_rgb(&{a0}, &{a1}, &{a2})")),
        "date" => Some("rp_date()".to_string()),
        "time" => Some("rp_time()".to_string()),
        "randomize" => Some(format!("rp_randomize(&{a0})")),
        "vartype" => Some(format!("rp_vartype(&{a0})")),
        "sizeof" => Some(format!("rp_sizeof(&{a0})")),

        // String functions
        "insert" => Some(format!("rp_insert(&{a0}, &{a1}, &{a2})")),
        "delete" => Some(format!("rp_delete(&{a0}, &{a1}, &{a2})")),
        "reverse" => Some(format!("rp_reverse(&{a0})")),
        "field" => Some(format!("rp_field(&{a0}, &{a1}, &{a2})")),
        "tally" => Some(format!("rp_tally(&{a0}, &{a1})")),
        "rinstr" => Some(format!("rp_rinstr(&{a0}, &{a1})")),
        "format" => Some(format!("rp_format(&{a0}, &{a1})")),
        "strf" => Some(format!("rp_strf(&{a0})")),

        // File I/O (function forms)
        "freefile" => Some("rp_freefile()".to_string()),
        "eof" => Some(format!("rp_eof(&{a0})")),
        "lof" => Some(format!("rp_lof(&{a0})")),
        "filelen" => Some(format!("rp_filelen(&{a0})")),
        "line_input" => Some(format!("rp_line_input(&{a0})")),

        // File/directory management
        "mkdir" => Some(format!("rp_mkdir(&{a0})")),
        "rmdir" => Some(format!("rp_rmdir(&{a0})")),
        "kill" => Some(format!("rp_kill(&{a0})")),
        "rename" => Some(format!("rp_rename(&{a0}, &{a1})")),
        "curdir" => Some("rp_curdir()".to_string()),
        "chdir" => Some(format!("rp_chdir(&{a0})")),

        // System
        "shell" => Some(format!("rp_shell(&{a0})")),
        "shellwait" => Some(format!("rp_shellwait(&{a0})")),
        "beep" => Some("rp_beep()".to_string()),
        "date_func" | "date$" => Some("rp_date()".to_string()),
        "time_func" | "time$" => Some("rp_time()".to_string()),

        // Array functions
        "lbound" => Some(format!("rp_lbound(&{a0})")),
        "ubound" => Some(format!("rp_ubound(&{a0})")),

        // Misc
        "sound" => Some(format!("rp_sound(&{a0}, &{a1})")),
        "sndplayasync" | "playsound" => Some(format!("rp_sound(&{a0}, &{a1})")),

        // Pointer helpers
        "varptr" => Some(format!("rp_varptr(&{a0})")),
        "varptr$" => Some(format!("rp_varptr_str(&{a0})")),

        _ => None,
    }
}

/// Generate a Cargo.toml for the output project that depends on the runtime.
pub fn generate_cargo_toml(project_name: &str, runtime_path: &str) -> String {
    format!(
        r#"[package]
name = "{project_name}"
version = "0.1.0"
edition = "2021"

[workspace]

[dependencies]
rapidr-runtime-core = {{ path = "{runtime_path}" }}
"#
    )
}

// ---------------------------------------------------------------------------
// Component system helpers (compile-time, no runtime dependency)
// ---------------------------------------------------------------------------

/// Check if a type name is a known RapidP component type.
fn is_component_type_name(type_name: &str) -> bool {
    matches!(
        type_name.to_uppercase().as_str(),
        "RFORM" | "RFORMMDI" | "RBUTTON" | "RLABEL" | "REDIT" | "RPANEL"
        | "RCHECKBOX" | "RRADIOBUTTON" | "RCOMBOBOX" | "RLISTBOX"
        | "RTIMER" | "RIMAGE" | "RCANVAS" | "RSTRINGGRID" | "RTABCONTROL"
        | "RTREEVIEW" | "RMAINMENU" | "RMENUITEM" | "RPOPUPMENU"
        | "ROPENDIALOG" | "RSAVEDIALOG" | "RCOLORDIALOG" | "RFONTDIALOG"
        | "RTOOLBAR" | "RSTATUSBAR" | "RPROGRESS" | "RRICHEDIT" | "RMEMO"
        | "RSCROLLBAR" | "RUPDOWN" | "RDATETIMEPICKER"
        | "RFILESTREAM" | "RSTRINGLIST" | "RTRACKBAR" | "RPRINTER"
        | "RSPLITTER" | "RSCROLLBOX"
        | "RSQLITE" | "RMYSQL"
        | "RSOCKET" | "RSERVERSOCKET" | "RHTTP"
        | "RLISTVIEW" | "RPROGRESSBAR"
        | "RNUM" | "RDATAFRAME" | "RPLOT"
        | "RDESIGNSURFACE" | "RCODEEDITOR" | "RGROUPBOX"
    )
}

/// Check if a member name is a known component method (not a property).
fn is_component_method_name(member: &str) -> bool {
    matches!(
        member,
        // Form/Widget methods
        "showmodal" | "close" | "show" | "hide" | "refresh" | "center"
        // Collection methods
        | "clear" | "additems" | "additem" | "deleteitems" | "deleteitem" | "removeitem"
        | "addrow" | "sort" | "find"
        // Focus/input methods
        | "setfocus" | "focus" | "click" | "selectall" | "copy" | "paste" | "cut"
        // Dialog methods
        | "execute"
        // Database methods
        | "connect" | "disconnect" | "query" | "fetchrow" | "fetchfield"
        | "fieldseek" | "rowseek" | "row" | "rowblob" | "escapestring"
        | "selectdb" | "createdb" | "dropdb"
        // Network methods
        | "write" | "writeline" | "read" | "readline"
        | "bind" | "listen" | "accept"
        | "start" | "stop" | "broadcast"
        | "get" | "post"
        // FileStream methods
        | "open" | "readall" | "eof"
        // StringList methods
        | "loadfromfile" | "savetofile" | "add" | "delete"
        // Canvas methods
        | "line" | "rect" | "fillrect" | "circle" | "ellipse"
        | "setpixel" | "getpixel" | "drawtext" | "loadimage" | "saveimage"
        // TreeView methods
        | "addroot" | "addchild" | "expand" | "collapse"
        // FormMDI methods
        | "closechild" | "closeallchild" | "cascadechild"
        | "sethorzchild" | "setvertchild" | "iconarrangechild"
        // NumPy/Pandas/Matplotlib methods
        | "array" | "zeros" | "ones" | "arange" | "linspace" | "reshape"
        | "readcsv" | "head" | "describe" | "columns" | "plot"
        | "scatter" | "title" | "xlabel" | "ylabel" | "legend" | "savefig"
        // Design surface methods
        | "addcomponent" | "getname" | "gettype"
        | "getcompx" | "getcompy" | "getcompw" | "getcomph"
        | "setprop" | "getprop" | "setcompbounds" | "setname"
        | "selectcomp" | "removecomponent" | "clearall"
        // StringGrid methods
        | "cell" | "cells" | "setcell" | "setsuggestions"
        // CodeEditor methods
        | "getsublist" | "gotosub" | "gotoline"
        // TabControl methods
        | "addtabs" | "tab"
    )
}

/// Recursively collect CREATE targets from nested CREATE body statements.
fn collect_nested_creates(
    stmts: &[Statement],
    component_vars: &mut HashMap<String, String>,
    top_level_vars: &mut HashSet<String>,
) {
    for stmt in stmts {
        if let Statement::Create(c) = stmt {
            component_vars.insert(c.name.to_lowercase(), c.type_name.to_uppercase());
            top_level_vars.insert(c.name.to_lowercase());
            collect_nested_creates(&c.body, component_vars, top_level_vars);
        }
    }
}

/// Collect all variable identifier references across the program for implicit variable detection.
fn collect_all_refs(stmts: &[Statement], refs: &mut HashSet<String>) {
    for stmt in stmts {
        match stmt {
            Statement::Assignment(a) => {
                collect_expr_refs(&a.target, refs);
                collect_expr_refs(&a.value, refs);
            }
            Statement::Print(p) => {
                for e in &p.items {
                    collect_expr_refs(e, refs);
                }
            }
            Statement::Call(c) => {
                collect_expr_refs(&c.callee, refs);
                for e in &c.args {
                    collect_expr_refs(e, refs);
                }
            }
            Statement::If(i) => {
                collect_expr_refs(&i.condition, refs);
                collect_all_refs(&i.then_body, refs);
                for branch in &i.elseif_branches {
                    collect_expr_refs(&branch.condition, refs);
                    collect_all_refs(&branch.body, refs);
                }
                collect_all_refs(&i.else_body, refs);
            }
            Statement::For(f) => {
                refs.insert(f.variable.to_lowercase());
                collect_expr_refs(&f.start, refs);
                collect_expr_refs(&f.end, refs);
                if let Some(step) = &f.step {
                    collect_expr_refs(step, refs);
                }
                collect_all_refs(&f.body, refs);
            }
            Statement::While(w) => {
                collect_expr_refs(&w.condition, refs);
                collect_all_refs(&w.body, refs);
            }
            Statement::DoLoop(d) => {
                if let Some(c) = &d.condition {
                    collect_expr_refs(c, refs);
                }
                collect_all_refs(&d.body, refs);
            }
            Statement::SelectCase(s) => {
                collect_expr_refs(&s.expression, refs);
                for case in &s.cases {
                    for v in &case.values {
                        collect_expr_refs(v, refs);
                    }
                    collect_all_refs(&case.body, refs);
                }
                collect_all_refs(&s.case_else, refs);
            }
            Statement::Subroutine(s) => {
                collect_all_refs(&s.body, refs);
            }
            Statement::Function(f) => {
                collect_all_refs(&f.body, refs);
            }
            Statement::Create(c) => {
                // Inside CREATE, assignment targets are property names, not variable refs
                for stmt in &c.body {
                    if let Statement::Assignment(a) = stmt {
                        collect_expr_refs(&a.value, refs);
                    } else {
                        collect_all_refs(std::slice::from_ref(stmt), refs);
                    }
                }
            }
            Statement::With(w) => {
                collect_expr_refs(&w.object, refs);
                collect_all_refs(&w.body, refs);
            }
            Statement::Input(i) => {
                collect_expr_refs(&i.target, refs);
                if let Some(p) = &i.prompt {
                    collect_expr_refs(p, refs);
                }
            }
            Statement::Bind(b) => {
                collect_expr_refs(&b.target, refs);
                collect_expr_refs(&b.handler, refs);
            }
            _ => {}
        }
    }
}

fn collect_expr_refs(expr: &Expression, refs: &mut HashSet<String>) {
    match expr {
        Expression::Identifier(id) => {
            let name = strip_type_suffix(&id.name).to_lowercase();
            refs.insert(name);
        }
        Expression::Binary(b) => {
            collect_expr_refs(&b.left, refs);
            collect_expr_refs(&b.right, refs);
        }
        Expression::Unary(u) => {
            collect_expr_refs(&u.operand, refs);
        }
        Expression::FunctionCall(fc) => {
            collect_expr_refs(&fc.callee, refs);
            for a in &fc.args {
                collect_expr_refs(a, refs);
            }
        }
        Expression::MethodCall(mc) => {
            collect_expr_refs(&mc.object, refs);
            for a in &mc.args {
                collect_expr_refs(a, refs);
            }
        }
        Expression::MemberAccess(ma) => {
            collect_expr_refs(&ma.object, refs);
        }
        Expression::ArrayAccess(aa) => {
            collect_expr_refs(&aa.array, refs);
            for idx in &aa.indices {
                collect_expr_refs(idx, refs);
            }
        }
        Expression::Literal(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rapidr_lexer::Lexer;
    use rapidr_parser::parse_tokens;

    fn gen(code: &str) -> String {
        let tokens = Lexer::new(code, None).tokenize().unwrap();
        let program = parse_tokens(&tokens);
        generate(&program)
    }

    #[test]
    fn hello_world_generates_rust() {
        let code = "PRINT \"Hello World\"\n";
        let rust = gen(code);
        assert!(rust.contains("rp_print"));
        assert!(rust.contains("v_str(\"Hello World\")"));
        assert!(rust.contains("fn main()"));
    }

    #[test]
    fn dim_generates_let_mut() {
        let code = "DIM x AS INTEGER\n";
        let rust = gen(code);
        // Top-level DIM now uses global storage via gs()
        assert!(rust.contains("gs(\"x\", v_int(0));"));
    }

    #[test]
    fn for_loop_generates_while() {
        let code = "DIM i AS INTEGER\nFOR i = 1 TO 5\n  PRINT i\nNEXT i\n";
        let rust = gen(code);
        // i is a top-level DIM, so it uses global storage
        assert!(rust.contains("while (gv(\"i\").rp_le("));
        assert!(rust.contains("gs(\"i\", &gv(\"i\") + &v_int(1))"));
    }

    #[test]
    fn sub_generates_fn() {
        let code = "SUB MySub(msg AS STRING)\n  PRINT msg\nEND SUB\n";
        let rust = gen(code);
        assert!(rust.contains("fn mysub(msg: Value)"));
    }

    #[test]
    fn builtin_functions_mapped() {
        let code = "PRINT LEN(\"hello\")\n";
        let rust = gen(code);
        assert!(rust.contains("rp_len("));
    }

    #[test]
    fn if_generates_correct_structure() {
        let code = "IF x > 5 THEN\n  PRINT \"big\"\nELSE\n  PRINT \"small\"\nEND IF\n";
        let rust = gen(code);
        assert!(rust.contains("if ("));
        assert!(rust.contains(".to_bool()"));
        assert!(rust.contains("} else {"));
    }
}

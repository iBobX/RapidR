//! RapidR Rust code generator.
//!
//! Walks the AST produced by `rapidr-parser` and emits readable Rust
//! source code targeting the `rapidr-runtime-core` library.

use std::collections::{HashMap, HashSet};
use std::fmt::Write;

use rapidr_ast::*;

mod jumps;
mod typed;

/// Target platform for code generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppTarget {
    /// Desktop (native) — uses `rapidr-runtime-core`.
    Desktop,
    /// Web (WASM) — uses `rapidr-runtime-web`.
    Web,
}

impl Default for AppTarget {
    fn default() -> Self {
        Self::Desktop
    }
}

/// What the Rust backend can't compile yet in `program`, if anything (see
/// `objects::unsupported`). `rapidr build` reports it as an error.
pub fn native_gap(_program: &Program) -> Option<String> {
    None
}

/// Generate a complete Rust `main.rs` from a parsed RapidP program.
pub fn generate(program: &Program) -> String {
    generate_for_target(program, AppTarget::Desktop)
}

/// Generate code for a specific target platform.
pub fn generate_for_target(program: &Program, target: AppTarget) -> String {
    generate_with_resources(program, target, &[])
}

/// Like [`generate_for_target`], for a program with `$RESOURCE`s: each is
/// (name, file) in order, built into the program with `include_bytes!` and
/// registered at startup (`rapidr_value::resources`).
pub fn generate_with_resources(program: &Program, target: AppTarget, resources: &[(String, String)]) -> String {
    let mut gen = RustCodegen::new(target);
    gen.resources = resources.to_vec();
    // Objects → plain routines and builtins, the same pass the bytecode
    // compiler runs (rapidr_ast::objects); fields become direct slot access.
    let program = rapidr_ast::stream_arrays::lower(&rapidr_ast::array_refs::lower(&rapidr_ast::routine_objects(&rapidr_ast::hoist_routines(program))));
    let program = rapidr_ast::objects::lower(&program, &|n| builtin_function_call(n, &[]).is_some() || is_object_builtin(n));
    // Stores into declared numeric types convert (rapidr_ast::numeric).
    let program = rapidr_ast::numeric::lower(program);
    let (program, promoted) = promote_ref_params(&program);
    gen.promoted_byref = promoted;
    gen.emit_program(&program);
    index_globals(&gen.output)
}

/// Turns the named module-level variable accesses the generator writes —
/// `gv("total")`, `gs("total", …)`, `ginit("total", …)` — into slot indexes
/// (`gv(0)`), and sizes the slot table. Inside a Rust string literal a quote
/// is always escaped, so user text can't match.
fn index_globals(code: &str) -> String {
    let mut slots: HashMap<String, usize> = HashMap::new();
    let mut out = String::with_capacity(code.len());
    let mut rest = code;
    while let Some(pos) = ["gv(\"", "gs(\"", "ginit(\""].iter().filter_map(|p| rest.find(p).map(|i| (i, p.len()))).min() {
        let (at, len) = pos;
        let name_start = at + len;
        let Some(end) = rest[name_start..].find('"') else { break };
        let name = &rest[name_start..name_start + end];
        let valid = !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
        out.push_str(&rest[..name_start - 1]);
        if valid {
            let next = slots.len();
            let slot = *slots.entry(name.to_string()).or_insert(next);
            out.push_str(&slot.to_string());
            rest = &rest[name_start + end + 1..];
        } else {
            out.push('"');
            rest = &rest[name_start..];
        }
    }
    out.push_str(rest);
    out.replace("__GLOBAL_SLOTS__", &slots.len().max(1).to_string())
}

/// `MySub @x` passes x by reference (RapidQ manual 3.5). A Rust function
/// can't choose per call, so every parameter some call site passes with `@`
/// becomes BYREF (`&mut Value`); calls without `@` then pass a temporary.
/// Returns the program with those parameters marked, and the promoted
/// positions per routine (lowercase name).
fn promote_ref_params(program: &Program) -> (Program, HashMap<String, Vec<usize>>) {
    let refs = rapidr_ast::ref_argument_positions(&program.statements);
    if refs.is_empty() {
        return (program.clone(), HashMap::new());
    }
    let mut program = program.clone();
    let mut promoted: HashMap<String, Vec<usize>> = HashMap::new();
    for stmt in &mut program.statements {
        let (name, params) = match stmt {
            Statement::Subroutine(s) => (s.name.to_lowercase(), &mut s.params),
            Statement::Function(f) => (strip_type_suffix(&f.name).to_lowercase(), &mut f.params),
            _ => continue,
        };
        for &i in refs.get(&name).map(Vec::as_slice).unwrap_or(&[]) {
            if let Some(p) = params.get_mut(i).filter(|p| !p.by_ref) {
                p.by_ref = true;
                promoted.entry(name.clone()).or_default().push(i);
            }
        }
    }
    (program, promoted)
}

struct RustCodegen {
    output: String,
    /// `$RESOURCE`s: (name, file) in order.
    resources: Vec<(String, String)>,
    indent: usize,
    /// Target platform.
    target: AppTarget,
    /// Names of subs/functions defined at the top level.
    defined_functions: HashSet<String>,
    /// BYREF flag of each parameter, by lowercase SUB/FUNCTION name.
    fn_byref: HashMap<String, Vec<bool>>,
    /// Parameters made BYREF only because some call passes `@x` to them:
    /// calls without `@` must not write back (see `promote_ref_params`).
    promoted_byref: HashMap<String, Vec<usize>>,
    /// Enclosing BASIC loops (kind, Rust label), innermost last, so
    /// `EXIT FOR` inside a WHILE breaks out of the FOR (`break 'l3;`).
    loop_labels: Vec<(&'static str, String)>,
    loop_label_counter: usize,
    /// Only emit the GOTO/GOSUB compile_error! once per program.
    reported_goto: bool,
    /// Names of variables declared with DIM at top level.
    top_level_vars: HashSet<String>,
    /// Names of variables declared as arrays (DIM with dimensions).
    array_vars: HashSet<String>,
    /// User-defined TYPE names (lowercase) → struct name for DIM default generation.
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
    /// FUNCTIONs (not SUBs), lowercase: a bare `Name` in an expression calls one.
    returning_functions: HashSet<String>,
    /// Labels some GOTO/GOSUB jumps to (lowercase).
    jump_targets: HashSet<String>,
    /// DECLARE'd FFI function names (lowercase) → (alias, lib, params, return_type).
    declared_functions: HashSet<String>,
    /// Array variable name (lowercase) → (default_value_str, size_expr_str) for re-declaring in subs.
    array_init_info: HashMap<String, (String, String)>,
    /// Whether we are inside a SUB or FUNCTION body (as opposed to top-level / main).
    in_sub_or_function: bool,
    /// Names the current SUB/FUNCTION declares itself (parameters, DIM):
    /// they shadow module-level variables of the same name.
    shadowed: HashSet<String>,
    /// Names the main program assigns (`x = …`, `FOR x`, `INPUT x`), and
    /// the current SUB/FUNCTION: variables, never bare builtins.
    assigned_main: HashSet<String>,
    assigned_routine: HashSet<String>,
    /// The current routine's locals kept as Rust numbers (`typed`).
    typed_locals: HashMap<String, typed::Kind>,
    /// Main-program variables kept as Rust numbers in atomic statics
    /// (`typed::analyze_globals`).
    typed_globals: HashMap<String, typed::Kind>,
    /// Names (lowercase) that appear in CREATE blocks — DIM for these should not emit rp_create_component.
    create_declared_names: HashSet<String>,
    /// Set while a routine with GOTO/GOSUB is emitted as a state machine.
    state_machine: Option<jumps::StateMachine>,
    /// The SUB/FUNCTION being emitted (None in the main program).
    current_routine_name: Option<String>,
    /// Function pointers: each SUB/FUNCTION's id (1, 2, …) with its
    /// BYREF flags and whether it returns a value, in definition order.
    routine_pointers: Vec<(String, Vec<bool>, bool)>,
}

impl RustCodegen {
    fn new(target: AppTarget) -> Self {
        Self {
            resources: Vec::new(),
            output: String::with_capacity(4096),
            indent: 0,
            target,
            defined_functions: HashSet::new(),
            fn_byref: HashMap::new(),
            promoted_byref: HashMap::new(),
            loop_labels: Vec::new(),
            loop_label_counter: 0,
            reported_goto: false,
            top_level_vars: HashSet::new(),
            array_vars: HashSet::new(),
            create_stack: Vec::new(),
            current_function: None,
            component_vars: HashMap::new(),
            with_component_stack: Vec::new(),
            all_referenced_vars: HashSet::new(),
            function_param_counts: HashMap::new(),
            returning_functions: HashSet::new(),
            jump_targets: HashSet::new(),
            declared_functions: HashSet::new(),
            array_init_info: HashMap::new(),
            in_sub_or_function: false,
            shadowed: HashSet::new(),
            assigned_main: HashSet::new(),
            assigned_routine: HashSet::new(),
            typed_locals: HashMap::new(),
            typed_globals: HashMap::new(),
            create_declared_names: HashSet::new(),
            state_machine: None,
            current_routine_name: None,
            routine_pointers: Vec::new(),
        }
    }

    /// Check if a variable name (lowercase) is a known component variable.
    fn is_component_var(&self, name: &str) -> bool {
        self.component_vars.contains_key(&name.to_lowercase())
    }

    /// Extract the component variable name from an expression, if it's a component identifier.
    fn get_component_name(&self, expr: &Expression) -> Option<String> {
        if let Expression::Identifier(id) = expr {
            let lower = strip_type_suffix(&id.name).to_lowercase();
            if self.component_vars.contains_key(&lower) {
                return Some(to_snake(&strip_type_suffix(&id.name)));
            }
        }
        None
    }

    /// `object.method(args)` on an object that isn't a component variable of
    /// the program: RapidQ's global objects by name (`Application`,
    /// `Screen`, …), anything else through the component id it holds.
    fn object_method_call(&self, object: &Expression, method: &str, args: &[&Expression]) -> String {
        let receiver = self.receiver(object);
        let args: Vec<String> = args.iter().map(|a| self.owned_expr(a)).collect();
        format!("rp_comp_method({receiver}, \"{}\", &[{}])", method.to_lowercase(), args.join(", "))
    }

    /// The component id argument for `object.member` when `object` isn't a
    /// component variable of the program, resolved as the VM does: a
    /// SUB/FUNCTION's own parameter or local holds the id; any other name
    /// (`Application`, `Screen`, an object RapidR has no component for) is
    /// the id itself.
    fn receiver(&self, object: &Expression) -> String {
        match object {
            Expression::Identifier(id) if id.name != "_with_" => {
                if let Some(comp) = self.get_component_name(object) {
                    return format!("\"{comp}\"");
                }
                let lower = strip_type_suffix(&id.name).to_lowercase();
                if self.in_sub_or_function && self.shadowed.contains(&lower) {
                    format!("&{}.to_string_val()", self.expr_to_string(object))
                } else {
                    format!("\"{}\"", id.name.to_lowercase())
                }
            }
            _ => format!("&{}.to_string_val()", self.expr_to_string(object)),
        }
    }

    /// Check if a variable is a module-level scalar (DIM at top level, not component, not array, not UDT).
    /// The names the program uses without declaring them (no DIM, parameter,
    /// component, constant or routine of that name): the main program's and
    /// every routine's, lowercase. A routine's own DIMs and parameters
    /// shadow; names of builtins stay builtins unless assigned.
    fn implicit_global_names(&self, program: &Program) -> HashSet<String> {
        let main: Vec<Statement> = program.statements.iter().filter(|s| !matches!(s, Statement::Subroutine(_) | Statement::Function(_) | Statement::Type(_))).cloned().collect();
        let mut used: HashSet<String> = HashSet::new();
        let mut assigned = assigned_names(&main);
        collect_all_refs(&main, &mut used);
        for stmt in &program.statements {
            let (params, body): (Vec<String>, &[Statement]) = match stmt {
                Statement::Subroutine(r) => (r.params.iter().map(|p| strip_type_suffix(&p.name).to_lowercase()).collect(), &r.body),
                Statement::Function(f) => (f.params.iter().map(|p| strip_type_suffix(&p.name).to_lowercase()).collect(), &f.body),
                _ => continue,
            };
            let mut local: HashSet<String> = params.into_iter().collect();
            rapidr_ast::walk(
                body,
                &mut |s| {
                    if let Statement::Dim(d) = s {
                        local.extend(d.declarators.iter().map(|v| strip_type_suffix(&v.name).to_lowercase()));
                    }
                    if let Statement::Create(c) = s {
                        local.insert(c.name.to_lowercase());
                    }
                },
                &mut |_| {},
            );
            let mut refs = HashSet::new();
            collect_all_refs(body, &mut refs);
            used.extend(refs.into_iter().filter(|n| !local.contains(n)));
            assigned.extend(assigned_names(body).into_iter().filter(|n| !local.contains(n)));
        }
        used.into_iter()
            .filter(|name| {
                let n = name.as_str();
                !self.top_level_vars.contains(n)
                    && !self.defined_functions.contains(n)
                    && !self.component_vars.contains_key(n)
                    // (`RESULT` is a FUNCTION's own return value)
                    && !matches!(n, "true" | "false" | "vttrue" | "vtfalse" | "_with_" | "result")
                    && (n != "pi" || assigned.contains("pi"))
                    && (builtin_function_call(n, &[]).is_none() || assigned.contains(n))
                    && n.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                    && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            })
            .collect()
    }

    fn is_global_scalar(&self, name: &str) -> bool {
        let lower = strip_type_suffix(name).to_lowercase();
        !self.shadowed.contains(strip_type_suffix(&lower).as_str())
            && self.top_level_vars.contains(&lower)
            && !self.component_vars.contains_key(&lower)
            && !self.array_vars.contains(&lower)
    }

    /// The Value holding array `name`: its global, or the local variable.
    fn array_base(&self, name: &str) -> String {
        let snake = to_snake(name);
        if self.is_global_array(name) {
            format!("gv(\"{snake}\")")
        } else {
            snake
        }
    }

    /// `i, j` → `(i).to_i64(), (j).to_i64()` for rp_get / rp_set.
    fn index_list(&self, indices: &[Expression]) -> String {
        indices
            .iter()
            .map(|e| format!("({}).to_i64()", self.expr_to_string(e)))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Check if a variable is a module-level array (DIM at top level with dimensions).
    fn is_global_array(&self, name: &str) -> bool {
        let lower = strip_type_suffix(name).to_lowercase();
        !self.shadowed.contains(strip_type_suffix(&lower).as_str())
            && self.top_level_vars.contains(&lower)
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
        // `handler` is the Rust name of a SUB/FUNCTION (`n_01click` for
        // `01Click`); a name that isn't one (a handler the program never
        // defines) sets the event to nothing, as in the VM.
        let routine = self.function_param_counts.iter().find(|(name, _)| to_snake(name) == handler);
        let Some((_, &arity)) = routine else {
            self.write_indent();
            let _ = writeln!(self.output, "rp_comp_set(\"{comp_name}\", \"{event}\", v_null());");
            return;
        };
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
        let main: Vec<Statement> =
            program.statements.iter().filter(|s| !matches!(s, Statement::Subroutine(_) | Statement::Function(_))).cloned().collect();
        self.assigned_main = assigned_names(&main);
        let mut targets = HashSet::new();
        rapidr_ast::walk(
            &program.statements,
            &mut |s| {
                if let Statement::Goto(j) | Statement::Gosub(j) = s {
                    targets.insert(j.label.to_lowercase());
                }
            },
            &mut |_| {},
        );
        self.jump_targets = targets;
        // First pass: collect top-level function/sub names, variable names, TYPE and component defs
        for stmt in &program.statements {
            match stmt {
                Statement::Subroutine(s) => {
                    self.routine_pointers.push((s.name.clone(), s.params.iter().map(|p| p.by_ref).collect(), false));
                    self.defined_functions.insert(s.name.to_lowercase());
                    self.defined_functions.insert(strip_type_suffix(&s.name).to_lowercase());
                    self.function_param_counts.insert(s.name.to_lowercase(), s.params.len());
                    self.fn_byref.insert(s.name.to_lowercase(), s.params.iter().map(|p| p.by_ref).collect());
                    // Scan body for local component DIMs and CREATEs
                    for body_stmt in &s.body {
                        if let Statement::Dim(d) = body_stmt {
                            for decl in &d.declarators {
                                if is_component_type_name(&d.type_name) && decl.dimensions.is_empty() {
                                    self.component_vars.insert(strip_type_suffix(&decl.name).to_lowercase(), d.type_name.to_uppercase());
                                }
                            }
                        }
                        if let Statement::Create(c) = body_stmt {
                            self.create_declared_names.insert(c.name.to_lowercase());
                            collect_nested_create_names(&c.body, &mut self.create_declared_names);
                        }
                    }
                }
                Statement::Function(f) => {
                    self.routine_pointers.push((f.name.clone(), f.params.iter().map(|p| p.by_ref).collect(), true));
                    self.defined_functions.insert(f.name.to_lowercase());
                    self.defined_functions.insert(strip_type_suffix(&f.name).to_lowercase());
                    self.returning_functions.insert(strip_type_suffix(&f.name).to_lowercase());
                    self.function_param_counts.insert(f.name.to_lowercase(), f.params.len());
                    self.fn_byref.insert(f.name.to_lowercase(), f.params.iter().map(|p| p.by_ref).collect());
                    // Scan body for local component DIMs and CREATEs
                    for body_stmt in &f.body {
                        if let Statement::Dim(d) = body_stmt {
                            for decl in &d.declarators {
                                if is_component_type_name(&d.type_name) && decl.dimensions.is_empty() {
                                    self.component_vars.insert(strip_type_suffix(&decl.name).to_lowercase(), d.type_name.to_uppercase());
                                }
                            }
                        }
                        if let Statement::Create(c) = body_stmt {
                            self.create_declared_names.insert(c.name.to_lowercase());
                            collect_nested_create_names(&c.body, &mut self.create_declared_names);
                        }
                    }
                }
                Statement::Dim(d) => {
                    for decl in &d.declarators {
                        let name_lower = strip_type_suffix(&decl.name).to_lowercase();
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
                        // Track if this is a component variable (not an array of them)
                        if is_component_type_name(&d.type_name) && decl.dimensions.is_empty() {
                            self.component_vars.insert(name_lower, d.type_name.to_uppercase());
                        }
                    }
                }
                Statement::Create(c) => {
                    // Register CREATE targets as component variables
                    self.component_vars.insert(c.name.to_lowercase(), c.type_name.to_uppercase());
                    self.top_level_vars.insert(c.name.to_lowercase());
                    self.create_declared_names.insert(c.name.to_lowercase());
                    collect_nested_creates(&c.body, &mut self.component_vars, &mut self.top_level_vars);
                    collect_nested_create_names(&c.body, &mut self.create_declared_names);
                }
                Statement::Const(c) => {
                    self.top_level_vars.insert(c.name.to_lowercase());
                }
                Statement::Declare(d) => {
                    let name_lower = d.name.to_lowercase();
                    self.declared_functions.insert(name_lower.clone());
                    self.defined_functions.insert(name_lower.clone());
                    self.defined_functions.insert(strip_type_suffix(&name_lower));
                    self.function_param_counts.insert(name_lower, d.params.len());
                }
                _ => {}
            }
        }

        // Components DIMmed in a nested block (`IF … : DIM Dlg AS
        // QOPENDIALOG`), in the main program or a routine: created by their
        // DIM like any other (as in the VM).
        rapidr_ast::walk(
            &program.statements,
            &mut |s| {
                if let Statement::Dim(d) = s {
                    if is_component_type_name(&d.type_name) {
                        for decl in d.declarators.iter().filter(|v| v.dimensions.is_empty()) {
                            self.component_vars.entry(strip_type_suffix(&decl.name).to_lowercase()).or_insert_with(|| d.type_name.to_uppercase());
                        }
                    }
                }
            },
            &mut |_| {},
        );

        // Second pass: collect all referenced variable names for implicit variable detection
        collect_all_refs(&program.statements, &mut self.all_referenced_vars);

        // Variables that are never DIMmed are global — one variable shared by
        // the main program and every routine, kept between calls — as in the
        // VM: they join the module-level variables.
        for name in self.implicit_global_names(&program) {
            self.top_level_vars.insert(name);
        }

        // Generated code declares every BYVAL parameter `mut` (BASIC may
        // assign to it); don't warn when a routine doesn't.
        self.line("#![allow(unused_mut, unused_labels, unreachable_code, unused_assignments)]");
        self.line(match self.target {
            AppTarget::Desktop => "use rapidr_runtime_core::prelude::*;",
            AppTarget::Web => "use rapidr_runtime_web::prelude::*;",
        });
        self.line("use std::cell::RefCell;");
        self.line("use std::collections::HashMap;");
        self.blank();

        // Emit global variable helpers for module-level DIM variables
        // Module-level variables live in slots (see `index_globals`): `gv(3)`
        // is a vector index, not a lookup by name.
        self.line("thread_local! {");
        self.line("    static GVARS: RefCell<Vec<Value>> = RefCell::new(vec![Value::Null; __GLOBAL_SLOTS__]);");
        self.line("    static GINIT: RefCell<Vec<bool>> = RefCell::new(vec![false; __GLOBAL_SLOTS__]);");
        self.line("}");
        self.line("#[inline] fn gv(i: usize) -> Value { GVARS.with(|g| g.borrow()[i].clone()) }");
        self.line("#[inline] fn gs(i: usize, v: Value) { GVARS.with(|g| g.borrow_mut()[i] = v); }");
        self.line("/// A STATIC variable's slot, set the first time only.");
        self.line("#[allow(dead_code)] fn ginit(i: usize, f: impl FnOnce() -> Value) { if !GINIT.with(|d| std::mem::replace(&mut d.borrow_mut()[i], true)) { gs(i, f()); } }");
        // Typed main-program variables (`typed::analyze_globals`).
        self.typed_globals = typed::analyze_globals(&program.statements, &|n| self.defined_functions.contains(n));
        // (Relaxed atomics: a plain memory access, no `unsafe`; a DOUBLE is
        // kept as its bits.)
        let mut typed: Vec<(String, typed::Kind)> = self.typed_globals.iter().map(|(n, k)| (n.clone(), *k)).collect();
        typed.sort();
        for (name, kind) in typed {
            let atomic = if kind == typed::Kind::Double { "AtomicU64" } else { "AtomicI64" };
            self.line(&format!("static {}: std::sync::atomic::{atomic} = std::sync::atomic::{atomic}::new(0);", typed::global_static(&name)));
        }
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

        self.emit_callfunc_table();

        // Emit main
        if self.target == AppTarget::Web {
            self.line("use wasm_bindgen::prelude::*;");
            self.blank();
            self.line("#[wasm_bindgen(start)]");
            self.line("pub fn main() {");
        } else {
            self.line("fn main() {");
        }
        self.indent += 1;
        let runtime = if self.target == AppTarget::Web { "rapidr_runtime_web" } else { "rapidr_runtime_core" };
        for (name, file) in self.resources.clone() {
            self.line(&format!("{runtime}::value::resources::register({name:?}, include_bytes!({file:?}).as_slice());"));
        }

        // Auto-declare implicit variables (referenced but never DIM'd)
        let assigned = self.assigned_main.clone();
        let mut implicit: Vec<String> = self.all_referenced_vars.iter()
            .filter(|name| {
                !self.top_level_vars.contains(name.as_str())
                    && !self.defined_functions.contains(name.as_str())
                    && !self.component_vars.contains_key(name.as_str())
                    && !matches!(name.as_str(), "true" | "false" | "vttrue" | "vtfalse" | "_with_")
                    // (`pi` is the constant unless the program assigns it)
                    && (name.as_str() != "pi" || assigned.contains("pi"))
                    && (builtin_function_call(name, &[]).is_none() || assigned.contains(name.as_str()))
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

        if self.routine_needs_states(&program.statements) {
            // Labels / GOTO / GOSUB: a state machine (jumps.rs).
            self.emit_state_machine(&program.statements);
        } else {
            for stmt in &program.statements {
                match stmt {
                    Statement::Subroutine(_) | Statement::Function(_) | Statement::Type(_) | Statement::Declare(_) => {
                        // already emitted above
                    }
                    _ => self.emit_statement(stmt),
                }
            }
        }

        // For web targets, finalize: auto-parent orphan widgets and show forms
        if self.target == AppTarget::Web {
            self.line("gui_web_finalize();");
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
            Statement::Create(c) => self.emit_create(c),
            Statement::With(w) => self.emit_with(w),
            Statement::Exit(e) => self.emit_exit(e),
            Statement::Return(r) => self.emit_return(r),
            Statement::Import(i) => self.emit_import(i),
            Statement::Input(i) => self.emit_input(i),
            Statement::Bind(b) => self.emit_bind(b),
            Statement::Declare(d) => self.emit_declare(d),
            // TYPEs are lowered to routines and object builtins by
            // `rapidr_ast::objects` before code generation.
            Statement::Type(_) => {}
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
            // `Name:` that names a SUB or builtin is a call followed by `:`.
            Statement::Label(l)
                if self.defined_functions.contains(&strip_type_suffix(&l.name).to_lowercase())
                    || builtin_function_call(&l.name.to_lowercase(), &[]).is_some() =>
            {
                let call = CallStatement {
                    span: l.span,
                    callee: Expression::Identifier(Identifier { span: l.span, name: l.name.clone() }),
                    args: Vec::new(),
                };
                self.emit_call(&call);
            }
            // A label nothing jumps to (e.g. only a RESTORE target) is just a marker.
            Statement::Label(l) if !self.jump_targets.contains(&l.name.to_lowercase()) => {}
            Statement::Goto(j) if self.in_state_machine() => self.emit_goto(j),
            // Rust has no goto; until codegen gets a state-machine lowering,
            // refuse clearly instead of generating code that runs wrongly.
            Statement::Label(_) | Statement::Goto(_) | Statement::Gosub(_) => {
                if !self.reported_goto {
                    self.reported_goto = true;
                    self.line(
                        "compile_error!(\"Line labels, GOTO and GOSUB are not supported in native builds yet. Run the program with the bytecode interpreter instead (rapidr build-bc / run-bc, --interp, or the web IDE).\");",
                    );
                }
            }
        }
    }

    fn emit_dim(&mut self, d: &DimStatement) {
        if d.is_static && self.in_sub_or_function {
            // Renamed to a program-wide slot by `prepare_statics`: create it
            // the first time only, so every call (and recursion) shares it.
            for decl in &d.declarators {
                let name = to_snake(&decl.name);
                let default = default_value_for_type(&d.type_name);
                let init = if decl.dimensions.is_empty() {
                    default
                } else {
                    let bounds: Vec<String> = decl
                        .dimensions
                        .iter()
                        .map(|dim| match dim {
                            ArrayDimension::Single(upper) => format!("(0, ({}).to_i64())", self.expr_to_string(upper)),
                            ArrayDimension::Range { start, end } => {
                                format!("(({}).to_i64(), ({}).to_i64())", self.expr_to_string(start), self.expr_to_string(end))
                            }
                        })
                        .collect();
                    format!("rp_new_array(&[{}], {default})", bounds.join(", "))
                };
                self.write_indent();
                let _ = writeln!(self.output, "ginit(\"{name}\", || {init});");
            }
            return;
        }
        for decl in &d.declarators {
            let name = to_snake(&decl.name);
            let name_lower = strip_type_suffix(&decl.name).to_lowercase();
            let was_array = self.array_vars.contains(&name_lower);
            self.array_vars.remove(&name_lower); // re-insert if has dims

            // Component variable → create via registry (skip if a CREATE block handles it)
            if self.component_vars.contains_key(&name_lower) {
                if !self.create_declared_names.contains(&name_lower) {
                    let type_name = self.component_vars[&name_lower].clone();
                    self.write_indent();
                    let _ = writeln!(self.output, "rp_create_component(\"{name}\", \"{type_name}\");");
                    if type_name == "RTIMER" {
                        self.write_indent();
                        let _ = writeln!(self.output, "gui_register_timer(\"{name}\");");
                    }
                }
                continue;
            }

            // `DIM lbl(1 TO 3) AS QLABEL`: one component per element (ids
            // `lbl(1)`, …), the variable holding the array of ids.
            if !decl.dimensions.is_empty() && is_component_type_name(&d.type_name) {
                self.array_vars.insert(name_lower.clone());
                let bounds: Vec<String> = decl
                    .dimensions
                    .iter()
                    .map(|dim| match dim {
                        ArrayDimension::Single(upper) => format!("(0, ({}).to_i64())", self.expr_to_string(upper)),
                        ArrayDimension::Range { start, end } => {
                            format!("(({}).to_i64(), ({}).to_i64())", self.expr_to_string(start), self.expr_to_string(end))
                        }
                    })
                    .collect();
                let kind = rapidr_ast::canonical_type_name(&d.type_name).to_ascii_uppercase();
                let array = format!("rp_component_array(\"{kind}\", \"{}\", &[{}])", decl.name, bounds.join(", "));
                if !self.in_sub_or_function && self.top_level_vars.contains(&name_lower) {
                    self.write_indent();
                    let _ = writeln!(self.output, "gs(\"{name}\", {array});");
                } else {
                    self.declare_local(&name, &array);
                }
                continue;
            }

            if decl.dimensions.is_empty() {
                if !self.in_sub_or_function && self.top_level_vars.contains(&name_lower) {
                    self.write_indent();
                    if let Some((kind, _)) = self.typed_var(&decl.name) {
                        // A typed main-program variable: DIM resets it.
                        let _ = writeln!(self.output, "{}", self.typed_write(&decl.name, kind.zero()));
                    } else {
                        // Module-level scalar → store in global vars
                        let default = default_value_for_type(&d.type_name);
                        let _ = writeln!(self.output, "gs(\"{name}\", {default});");
                    }
                } else if let Some(kind) = self.typed_local(&decl.name) {
                    // Declared at the routine's start; DIM resets it.
                    self.write_indent();
                    let _ = writeln!(self.output, "{name} = {};", kind.zero());
                } else {
                    let default = default_value_for_type(&d.type_name);
                    self.declare_local(&name, &default);
                }
            } else {
                self.array_vars.insert(strip_type_suffix(&decl.name).to_lowercase());
                // Array declaration: a shared Value::Array with real bounds
                // (`DIM a(10)` → 0..=10, `DIM b(1 TO 5, 3)`), the same model
                // as the bytecode VM.
                let bounds: Vec<String> = decl
                    .dimensions
                    .iter()
                    .map(|dim| match dim {
                        ArrayDimension::Single(upper) => {
                            format!("(0, ({}).to_i64())", self.expr_to_string(upper))
                        }
                        ArrayDimension::Range { start, end } => format!(
                            "(({}).to_i64(), ({}).to_i64())",
                            self.expr_to_string(start),
                            self.expr_to_string(end)
                        ),
                    })
                    .collect();
                let default = default_value_for_type(&d.type_name);
                // REDIM in a SUB resizes the module-level array unless the
                // SUB declares its own (as in the VM).
                let global = if self.in_sub_or_function {
                    d.is_redim && self.top_level_vars.contains(&name_lower) && !self.shadowed.contains(&name_lower)
                } else {
                    self.top_level_vars.contains(&name_lower)
                };
                let array = if d.is_redim {
                    // REDIM: resize keeping the data (a new array if there's none yet).
                    let current = if global {
                        format!("gv(\"{name}\")")
                    } else if was_array {
                        name.clone()
                    } else {
                        "v_null()".to_string()
                    };
                    format!("rp_redim(&{current}, &[{}], {default})", bounds.join(", "))
                } else {
                    format!("rp_new_array(&[{}], {default})", bounds.join(", "))
                };
                if global {
                    // Module-level array → global variable
                    self.write_indent();
                    let _ = writeln!(self.output, "gs(\"{name}\", {array});");
                } else {
                    self.declare_local(&name, &array);
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
            self.declare_local(&name, &val);
        }
    }

    /// `let mut name = value;` — or, inside a state machine (jumps.rs), a
    /// declaration before its loop and an assignment here.
    fn declare_local(&mut self, name: &str, value: &str) {
        self.write_indent();
        if self.in_state_machine() {
            self.hoist(format!("let mut {name} = v_null();"));
            let _ = writeln!(self.output, "{name} = {value};");
        } else {
            let _ = writeln!(self.output, "let mut {name} = {value};");
        }
    }

    fn emit_assignment(&mut self, a: &AssignmentStatement) {
        // Check if this is a FUNCTION return pattern: FuncName = expr
        if let Some(fname) = self.current_function.clone() {
            if let Expression::Identifier(id) = &a.target {
                // `FuncName = v` or RapidQ's `RESULT = v`
                if strip_type_suffix(&id.name).eq_ignore_ascii_case(&strip_type_suffix(&fname)) || id.name.eq_ignore_ascii_case("result") {
                    let val = self.owned_expr(&a.value);
                    let fname_lc = to_snake(&fname);
                    self.write_indent();
                    let _ = writeln!(self.output, "_{fname_lc} = {val};");
                    return;
                }
            }
        }

        // A typed local (`typed`): converted, as native arithmetic when it can be.
        if let Expression::Identifier(id) = &a.target {
            if let Some((kind, _)) = self.typed_var(&id.name).filter(|_| self.create_stack.is_empty()) {
                let value = self.typed_store(kind, &a.value);
                self.write_indent();
                let _ = writeln!(self.output, "{}", self.typed_write(&id.name, &value));
                return;
            }
        }

        // Assignment to bare component variable: comp = expr → evaluate for
        // side effects (not in a CREATE block, where `Font = Font` sets the
        // created component's property from the variable).
        if let Expression::Identifier(id) = &a.target {
            if self.is_component_var(&id.name) && self.create_stack.is_empty() {
                let val = self.owned_expr(&a.value);
                self.write_indent();
                let _ = writeln!(self.output, "let _ = {val};");
                return;
            }
        }

        // `Bitmap.Pixel(x, y) = c` on a component: its `pixel` method with the
        // value as an extra last argument.
        let indexed = match &a.target {
            Expression::FunctionCall(fc) if !fc.args.is_empty() => Some((&fc.callee, &fc.args)),
            Expression::ArrayAccess(aa) => Some((&aa.array, &aa.indices)),
            _ => None,
        };
        if let Some((Expression::MemberAccess(ma), indices)) = indexed.map(|(c, i)| (c.as_ref(), i)) {
            if let Some(comp_name) = self.get_component_name(&ma.object) {
                let mut args: Vec<String> = indices.iter().map(|e| self.owned_expr(e)).collect();
                args.push(self.owned_expr(&a.value));
                let method = ma.member.to_lowercase();
                self.write_indent();
                let _ = writeln!(self.output, "rp_comp_method(\"{comp_name}\", \"{method}\", &[{}]);", args.join(", "));
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

        // `Screen.Cursors(i) = v`, `Sender.Pixel(x, y) = c` on any other
        // object: its method with the value as an extra last argument.
        if self.create_stack.is_empty() {
            if let Some((Expression::MemberAccess(ma), indices)) = indexed.map(|(c, i)| (c.as_ref(), i)) {
                if !matches!(ma.object.as_ref(), Expression::Identifier(id) if id.name == "_with_") {
                    let mut args: Vec<&Expression> = indices.iter().collect();
                    args.push(&a.value);
                    let call = self.object_method_call(&ma.object, &ma.member, &args);
                    self.write_indent();
                    let _ = writeln!(self.output, "{call};");
                    return;
                }
            }
            // `DXTimer.Interval = 10`, `Sender.Caption = s` on any other
            // object; `Obj.OnEvent = Handler` binds a SUB.
            if let Expression::MemberAccess(ma) = &a.target {
                if let Expression::Identifier(id) = ma.object.as_ref() {
                    if id.name != "_with_" {
                        let prop = ma.member.to_lowercase();
                        if prop.starts_with("on") {
                            if let Expression::Identifier(h) = &a.value {
                                let handler = strip_type_suffix(&h.name).to_lowercase();
                                if self.defined_functions.contains(&handler) {
                                    self.emit_bind_event_call(&id.name.to_lowercase(), &prop, &to_snake(&handler));
                                    return;
                                }
                            }
                        }
                        let receiver = self.receiver(&ma.object);
                        let value = self.owned_expr(&a.value);
                        self.write_indent();
                        let _ = writeln!(self.output, "rp_comp_set({receiver}, \"{prop}\", {value});");
                        return;
                    }
                }
                // `RichEdit.SelAttributes.Color = c`: the object's `sub.prop`.
                if let Expression::MemberAccess(inner) = ma.object.as_ref() {
                    if matches!(inner.object.as_ref(), Expression::Identifier(id) if id.name != "_with_") {
                        let receiver = self.receiver(&inner.object);
                        let value = self.owned_expr(&a.value);
                        self.write_indent();
                        let _ = writeln!(
                            self.output,
                            "rp_comp_set({receiver}, \"{}.{}\", {value});",
                            inner.member.to_lowercase(),
                            ma.member.to_lowercase()
                        );
                        return;
                    }
                }
            }
        }

        // Inside CREATE block: `Cell(1, 0) = s` → the object's indexed
        // property (its method with the value as an extra last argument).
        if let Some(obj) = self.create_stack.last().cloned() {
            if let Some((Expression::Identifier(id), indices)) = indexed.map(|(c, i)| (c.as_ref(), i)) {
                let lower = strip_type_suffix(&id.name).to_lowercase();
                if !self.array_vars.contains(&lower) && !self.top_level_vars.contains(&lower) {
                    let mut args: Vec<String> = indices.iter().map(|e| self.owned_expr(e)).collect();
                    args.push(self.owned_expr(&a.value));
                    self.write_indent();
                    let _ = writeln!(self.output, "rp_comp_method(\"{obj}\", \"{lower}\", &[{}]);", args.join(", "));
                    return;
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
            // A module-level scalar, or a whole array (`a = __objectarray(…)`).
            if self.is_global_scalar(&stripped) || self.is_global_array(&stripped) {
                let snake = to_snake(&stripped);
                let value = self.owned_expr(&a.value);
                self.write_indent();
                let _ = writeln!(self.output, "gs(\"{snake}\", {value});");
                return;
            }
        }
        // Array element assignment: `a(i, j) = v` → a.rp_set(&[i, j], v)
        let element = match &a.target {
            Expression::FunctionCall(fc) => match fc.callee.as_ref() {
                Expression::Identifier(id) => Some((id, &fc.args)),
                _ => None,
            },
            Expression::ArrayAccess(aa) => match aa.array.as_ref() {
                Expression::Identifier(id) => Some((id, &aa.indices)),
                _ => None,
            },
            _ => None,
        };
        if let Some((id, indices)) = element {
            let stripped = strip_type_suffix(&id.name).to_lowercase();
            if self.array_vars.contains(&stripped) {
                let base = self.array_base(&stripped);
                let idx = self.index_list(indices);
                let value = self.owned_expr(&a.value);
                self.write_indent();
                let _ = writeln!(self.output, "{base}.rp_set(&[{idx}], {value});");
                return;
            }
        }

        let target = self.lvalue_to_string(&a.target);
        let value = self.owned_expr(&a.value);
        self.write_indent();
        let _ = writeln!(self.output, "{target} = {value};");
    }

    /// One rp_print per item (so `;` joins items directly), rp_print_zone
    /// after each `,`, and the newline only when the statement doesn't end
    /// with a separator — the same output as the bytecode VM.
    fn emit_print(&mut self, p: &PrintStatement) {
        if p.items.is_empty() {
            if p.append_newline {
                self.line("rp_print(&[], true);");
            }
            return;
        }
        let n = p.items.len();
        for (i, item) in p.items.iter().enumerate() {
            let value = self.owned_expr(item);
            let newline = i + 1 == n && p.append_newline;
            self.write_indent();
            let _ = writeln!(self.output, "rp_print(&[{value}], {newline});");
            if p.zones.get(i).copied().unwrap_or(false) {
                self.line("rp_print_zone();");
            }
        }
    }

    fn emit_call(&mut self, c: &CallStatement) {
        // RapidQ `INC x [, n]` / `DEC x [, n]` → `x = x ± n` (shared with the VM).
        let inc_dec = inc_dec_assignment(c, |name| {
            self.defined_functions.contains(&strip_type_suffix(name).to_lowercase())
        });
        if let Some(assignment) = inc_dec {
            self.emit_assignment(&assignment);
            return;
        }

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

        // Inside CREATE: bare identifier calls → component method on CREATE
        // target (not the compiler's own `__…` helpers).
        if !self.create_stack.is_empty() {
            if let Expression::Identifier(id) = &c.callee {
                if !self.defined_functions.contains(&id.name.to_lowercase()) && !id.name.starts_with("__") {
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

        // --- User SUB/FUNCTION with BYREF parameters ---
        if let Expression::Identifier(id) = &c.callee {
            let lower = strip_type_suffix(&id.name).to_lowercase();
            if self.fn_byref.get(&lower).is_some_and(|f| f.contains(&true)) {
                let (args, pre, post) = self.user_call_args(&lower, &c.args);
                let fname = to_snake(&strip_type_suffix(&id.name));
                self.write_indent();
                let _ = writeln!(self.output, "{{ {} {fname}({}); {} }}", pre.join(" "), args.join(", "), post.join(" "));
                return;
            }
        }

        // A method of any other object (`Application.Terminate`,
        // `Printer.Printers(i)`, a component held in a variable), as the VM
        // calls it.
        let member_call = match &c.callee {
            Expression::MemberAccess(ma) => Some((ma.object.as_ref(), &ma.member, &[][..])),
            Expression::MethodCall(mc) => Some((mc.object.as_ref(), &mc.method, &mc.args[..])),
            Expression::FunctionCall(fc) => match fc.callee.as_ref() {
                Expression::MemberAccess(ma) => Some((ma.object.as_ref(), &ma.member, &fc.args[..])),
                _ => None,
            },
            _ => None,
        };
        if let Some((object, method, first)) = member_call {
            let args: Vec<&Expression> = first.iter().chain(&c.args).collect();
            let call = self.object_method_call(object, method, &args);
            self.write_indent();
            let _ = writeln!(self.output, "{call};");
            return;
        }

        // --- Standard call handling ---
        let callee = match &c.callee {
            Expression::Identifier(id) if self.defined_functions.contains(&id.name.to_lowercase()) => to_snake(&strip_type_suffix(&id.name)),
            other => self.expr_to_string(other),
        };
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

        // A routine nobody defines: only unreached library code gets here
        // (the shared checks reject it in the program's own code), and it
        // fails when run, as in the VM.
        if let Expression::Identifier(id) = &c.callee {
            if !self.defined_functions.contains(&id.name.to_lowercase()) {
                let call = unknown_routine(&id.name);
                self.write_indent();
                let _ = writeln!(self.output, "{call};");
                return;
            }
        }
        let call = match &c.callee {
            Expression::Identifier(id) => self.fitted_call(&strip_type_suffix(&id.name).to_lowercase(), &callee, args),
            _ => format!("{callee}({})", args.join(", ")),
        };
        self.write_indent();
        let _ = writeln!(self.output, "{call};");
    }

    /// `f(args)` for the program's routine `name`, with the arguments fitted
    /// to its parameters as the VM does: every argument is evaluated, in
    /// order; extra ones are dropped, missing ones are empty.
    fn fitted_call(&self, name: &str, fname: &str, args: Vec<String>) -> String {
        let params = self
            .function_param_counts
            .iter()
            .find(|(n, _)| strip_type_suffix(n).eq_ignore_ascii_case(name))
            .map(|(_, &p)| p);
        match params {
            Some(p) if p != args.len() => {
                let lets: String = args.iter().enumerate().map(|(i, a)| format!("let __a{i} = {a}; ")).collect();
                let passed: Vec<String> = (0..p).map(|i| if i < args.len() { format!("__a{i}") } else { "v_null()".to_string() }).collect();
                format!("{{ {lets}{fname}({}) }}", passed.join(", "))
            }
            _ => format!("{fname}({})", args.join(", ")),
        }
    }

    fn emit_if(&mut self, i: &IfStatement) {
        let cond = self.cond_to_string(&i.condition);
        self.write_indent();
        let _ = writeln!(self.output, "if {cond} {{");
        self.indent += 1;
        for s in &i.then_body {
            self.emit_statement(s);
        }
        self.indent -= 1;

        for branch in &i.elseif_branches {
            let cond = self.cond_to_string(&branch.condition);
            self.write_indent();
            let _ = writeln!(self.output, "}} else if {cond} {{");
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
        if let Some((kind, _)) = self.typed_var(&f.variable) {
            return self.emit_typed_for(f, kind);
        }
        let var = to_snake(&f.variable);
        let start = self.owned_expr(&f.start);
        let end = self.owned_expr(&f.end);
        let step = f
            .step
            .as_ref()
            .map(|e| self.owned_expr(e))
            .unwrap_or_else(|| "v_int(1)".to_string());

        let is_global = self.is_global_scalar(&f.variable);
        let lbl = self.open_loop("FOR");
        // Capture end and step in temporaries so direction respects step sign.
        // Loop continues while: (step >= 0 && var <= end) || (step < 0 && var >= end).
        let end_tmp = format!("__for_end_{var}");
        let step_tmp = format!("__for_step_{var}");
        if is_global {
            self.write_indent();
            let _ = writeln!(self.output, "let {end_tmp} = {end};");
            self.write_indent();
            let _ = writeln!(self.output, "let {step_tmp} = {step};");
            self.write_indent();
            let _ = writeln!(self.output, "gs(\"{var}\", {start});");
            self.write_indent();
            let _ = writeln!(self.output, "{lbl}: while (if {step_tmp}.rp_ge(&v_int(0)).to_bool() {{ gv(\"{var}\").rp_le(&{end_tmp}) }} else {{ gv(\"{var}\").rp_ge(&{end_tmp}) }}).to_bool() {{");
            self.indent += 1;
            for s in &f.body {
                self.emit_statement(s);
            }
            self.write_indent();
            let _ = writeln!(self.output, "gs(\"{var}\", &gv(\"{var}\") + &{step_tmp});");
        } else {
            self.write_indent();
            let _ = writeln!(self.output, "let {end_tmp} = {end};");
            self.write_indent();
            let _ = writeln!(self.output, "let {step_tmp} = {step};");
            self.write_indent();
            let _ = writeln!(self.output, "{var} = {start};");
            self.write_indent();
            let _ = writeln!(self.output, "{lbl}: while (if {step_tmp}.rp_ge(&v_int(0)).to_bool() {{ {var}.rp_le(&{end_tmp}) }} else {{ {var}.rp_ge(&{end_tmp}) }}).to_bool() {{");
            self.indent += 1;
            for s in &f.body {
                self.emit_statement(s);
            }
            self.write_indent();
            let _ = writeln!(self.output, "{var} = &{var} + &{step_tmp};");
        }
        self.indent -= 1;
        self.line("}");
        self.loop_labels.pop();
    }

    /// Names the Rust loop for a BASIC loop of `kind`; pair with a pop.
    fn open_loop(&mut self, kind: &'static str) -> String {
        self.loop_label_counter += 1;
        let label = format!("'l{}", self.loop_label_counter);
        self.loop_labels.push((kind, label.clone()));
        label
    }

    fn emit_while(&mut self, w: &WhileStatement) {
        let cond = self.cond_to_string(&w.condition);
        let lbl = self.open_loop("WHILE");
        self.write_indent();
        let _ = writeln!(self.output, "{lbl}: while {cond} {{");
        self.indent += 1;
        for s in &w.body {
            self.emit_statement(s);
        }
        self.indent -= 1;
        self.line("}");
        self.loop_labels.pop();
    }

    fn emit_do_loop(&mut self, d: &DoLoopStatement) {
        let lbl = self.open_loop("DO");
        if d.pre_condition {
            let cond = d.condition.as_ref().map(|e| self.cond_to_string(e)).unwrap_or_else(|| "true".to_string());
            if d.is_until {
                self.write_indent();
                let _ = writeln!(self.output, "{lbl}: while !{cond} {{");
            } else {
                self.write_indent();
                let _ = writeln!(self.output, "{lbl}: while {cond} {{");
            }
            self.indent += 1;
            for s in &d.body {
                self.emit_statement(s);
            }
            self.indent -= 1;
            self.line("}");
        } else {
            // Post-condition or infinite loop
            self.write_indent();
            let _ = writeln!(self.output, "{lbl}: loop {{");
            self.indent += 1;
            for s in &d.body {
                self.emit_statement(s);
            }
            if let Some(cond_expr) = &d.condition {
                let cond = self.cond_to_string(cond_expr);
                if d.is_until {
                    self.write_indent();
                    let _ = writeln!(self.output, "if {cond} {{ break; }}");
                } else {
                    self.write_indent();
                    let _ = writeln!(self.output, "if !{cond} {{ break; }}");
                }
            }
            self.indent -= 1;
            self.line("}");
        }
        self.loop_labels.pop();
    }

    fn emit_select_case(&mut self, s: &SelectCaseStatement) {
        // Owned: `SELECT CASE k` inside a loop mustn't move `k`.
        let expr = self.owned_expr(&s.expression);
        self.write_indent();
        let _ = writeln!(self.output, "let _select_val = {expr};");
        let mut first = true;
        for case in &s.cases {
            // Same BASIC comparison helpers as ordinary expressions (so
            // 2 matches 2.0), not Rust's strict `==`.
            let conditions: Vec<String> = case
                .values
                .iter()
                .map(|v| match v {
                    CaseValue::Value(e) => {
                        format!("_select_val.rp_eq(&{}).to_bool()", self.expr_to_string(e))
                    }
                    CaseValue::Range(low, high) => format!(
                        "(_select_val.rp_ge(&{}).to_bool() && _select_val.rp_le(&{}).to_bool())",
                        self.expr_to_string(low),
                        self.expr_to_string(high)
                    ),
                    CaseValue::Is(op, e) => {
                        let method = match op {
                            BinaryOperator::Equal => "rp_eq",
                            BinaryOperator::NotEqual => "rp_ne",
                            BinaryOperator::LessThan => "rp_lt",
                            BinaryOperator::LessThanOrEqual => "rp_le",
                            BinaryOperator::GreaterThan => "rp_gt",
                            _ => "rp_ge",
                        };
                        format!("_select_val.{method}(&{}).to_bool()", self.expr_to_string(e))
                    }
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
        self.shadowed = shadowing_names(&s.params, &s.body);
        self.assigned_routine = assigned_names(&s.body);
        let byref = self.byref_prologue(&s.params, false);
        let body = self.prepare_statics(&s.name, &s.body);
        self.typed_locals = typed::analyze(&s.params, &body, None, &|n| self.defined_functions.contains(n));
        // Auto-declare local variables for refs in body that aren't params
        self.emit_local_vars(&body, &s.params);

        let array_params = self.enter_array_params(&s.params);
        self.current_routine_name = Some(s.name.clone());
        self.emit_routine_body(&body);
        self.current_routine_name = None;
        self.leave_array_params(array_params);
        self.byref_epilogue(&byref, false);
        self.in_sub_or_function = false;
        self.shadowed.clear();
        self.assigned_routine.clear();
        self.typed_locals.clear();
        self.indent -= 1;
        self.line("}");
    }

    /// The id `CODEPTR(Name)` / `BIND p TO Name` give routine `name`, if it's
    /// a SUB or FUNCTION of the program.
    fn routine_pointer(&self, name: &str) -> Option<usize> {
        let key = strip_type_suffix(name).to_lowercase();
        self.routine_pointers
            .iter()
            .position(|(n, _, _)| strip_type_suffix(n).to_lowercase() == key)
            .map(|i| i + 1)
    }

    /// `__callfunc(ptr, args)`: CALLFUNC's dispatch over the program's
    /// SUBs and FUNCTIONs by pointer (see [`Self::routine_pointer`]).
    fn emit_callfunc_table(&mut self) {
        if self.routine_pointers.is_empty() {
            return;
        }
        self.line("#[allow(dead_code)]");
        self.line("fn __callfunc(ptr: &Value, args: &[Value]) -> Value {");
        self.indent += 1;
        self.line("let arg = |i: usize| args.get(i).cloned().unwrap_or(v_null());");
        self.line("match ptr.to_i64() {");
        self.indent += 1;
        for (i, (name, byref, returns)) in self.routine_pointers.clone().iter().enumerate() {
            let args: Vec<String> = byref
                .iter()
                .enumerate()
                .map(|(k, r)| if *r { format!("&mut arg({k})") } else { format!("arg({k})") })
                .collect();
            let call = format!("{}({})", to_snake(name), args.join(", "));
            self.write_indent();
            if *returns {
                let _ = writeln!(self.output, "{} => {call},", i + 1);
            } else {
                let _ = writeln!(self.output, "{} => {{ {call}; v_null() }}", i + 1);
            }
        }
        self.line("p => { eprintln!(\"run-time error: CALLFUNC: {p} is not a function pointer (use BIND or CODEPTR)\"); std::process::exit(1) }");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();
    }

    /// `STATIC x` in routine `routine`: `x` becomes the program-wide variable
    /// `__static_<routine>_<x>` inside the body (a global, like a
    /// module-level DIM), so its value survives between calls. Returns the
    /// body with the names changed.
    fn prepare_statics(&mut self, routine: &str, body: &[Statement]) -> Vec<Statement> {
        let mut renames: HashMap<String, String> = HashMap::new();
        rapidr_ast::walk(
            body,
            &mut |s| {
                if let Statement::Dim(d) = s {
                    if d.is_static {
                        for decl in &d.declarators {
                            let key = strip_type_suffix(&decl.name).to_lowercase();
                            let new = format!("__static_{}_{}", strip_type_suffix(routine).to_lowercase(), key);
                            renames.insert(key, new);
                        }
                    }
                }
            },
            &mut |_| {},
        );
        let mut body = body.to_vec();
        if renames.is_empty() {
            return body;
        }
        let rename = |name: &mut String| {
            if let Some(new) = renames.get(&strip_type_suffix(name).to_lowercase()) {
                *name = new.clone();
            }
        };
        rapidr_ast::walk_expressions_mut(&mut body, true, &mut |e| {
            if let Expression::Identifier(id) = e {
                rename(&mut id.name);
            }
        });
        rapidr_ast::walk_statements_mut(&mut body, &mut |s| match s {
            Statement::For(f) => rename(&mut f.variable),
            Statement::Dim(d) => d.declarators.iter_mut().for_each(|decl| rename(&mut decl.name)),
            _ => {}
        });
        rapidr_ast::walk(
            &body,
            &mut |s| {
                if let Statement::Dim(d) = s {
                    if d.is_static {
                        for decl in &d.declarators {
                            let name = strip_type_suffix(&decl.name).to_lowercase();
                            self.top_level_vars.insert(name.clone());
                            if !decl.dimensions.is_empty() {
                                self.array_vars.insert(name);
                            }
                        }
                    }
                }
            },
            &mut |_| {},
        );
        body
    }

    /// A SUB/FUNCTION body: plain statements, or a state machine when it
    /// uses labels, GOTO or GOSUB (jumps.rs).
    fn emit_routine_body(&mut self, body: &[Statement]) {
        if self.routine_needs_states(body) {
            self.emit_state_machine(body);
        } else {
            for stmt in body {
                self.emit_statement(stmt);
            }
        }
    }

    /// Array parameters (`list() AS STRING`) are arrays inside the body, so
    /// `list(i) = v` stores into the caller's (shared) array. Returns the
    /// names added, for [`Self::leave_array_params`].
    fn enter_array_params(&mut self, params: &[Parameter]) -> Vec<String> {
        params
            .iter()
            .filter(|p| p.is_array)
            .map(|p| strip_type_suffix(&p.name).to_lowercase())
            .filter(|name| self.array_vars.insert(name.clone()))
            .collect()
    }

    fn leave_array_params(&mut self, added: Vec<String>) {
        for name in added {
            self.array_vars.remove(&name);
        }
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
        self.shadowed = shadowing_names(&f.params, &f.body);
        self.assigned_routine = assigned_names(&f.body);
        let byref = self.byref_prologue(&f.params, true);
        let body = self.prepare_statics(&f.name, &f.body);
        self.typed_locals = typed::analyze(&f.params, &body, Some(&f.name), &|n| self.defined_functions.contains(n));
        // Auto-declare local variables for refs in body that aren't params
        self.emit_local_vars(&body, &f.params);

        self.current_function = Some(f.name.clone());
        let array_params = self.enter_array_params(&f.params);
        self.current_routine_name = Some(f.name.clone());
        self.emit_routine_body(&body);
        self.current_routine_name = None;
        self.leave_array_params(array_params);
        self.current_function = None;
        self.in_sub_or_function = false;
        self.shadowed.clear();
        self.assigned_routine.clear();
        self.typed_locals.clear();

        self.write_indent();
        let _ = writeln!(self.output, "_{name}");
        self.byref_epilogue(&byref, true);
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
        let assigned = assigned_names(body);
        let mut locals: Vec<String> = local_refs
            .iter()
            .filter(|name| {
                !param_names.contains(name.as_str())
                    && !self.defined_functions.contains(name.as_str())
                    && !self.component_vars.contains_key(name.as_str())
                    && !self.is_global_scalar(name)
                    && !self.is_global_array(name)
                    && !matches!(name.as_str(), "true" | "false" | "vttrue" | "vtfalse" | "_with_")
                    && (name.as_str() != "pi" || assigned.contains("pi"))
                    && (builtin_function_call(name, &[]).is_none() || assigned.contains(name.as_str()))
            })
            .cloned()
            .collect();
        locals.sort();
        // Typed locals (`typed`) are DIMmed in this routine: always declared,
        // as Rust numbers, even when their name is also a builtin's (`pi`).
        let mut typed: Vec<(String, typed::Kind)> = self.typed_locals.iter().map(|(n, k)| (n.clone(), *k)).collect();
        typed.sort();
        for (name, kind) in typed {
            self.write_indent();
            let snake = to_snake(&name);
            // A typed BYVAL parameter: the number the caller passed.
            let init = if !param_names.contains(&name) {
                kind.zero().to_string()
            } else if kind == typed::Kind::Double {
                format!("numeric::double_of(&{snake})")
            } else {
                format!("numeric::int_of(&{snake}, {})", kind.runtime())
            };
            let _ = writeln!(self.output, "let mut {snake}: {} = {init};", kind.rust_type());
        }
        locals.retain(|l| self.typed_local(l).is_none());
        for local in &locals {
            let snake = to_snake(local);
            self.write_indent();
            if let Some((default, size)) = self.array_init_info.get(local.as_str()) {
                // Only emit local array if not a global array
                // The DIM statement allocates it; declare it for earlier uses.
                let _ = (default, size);
                if !self.is_global_array(local) {
                    let _ = writeln!(self.output, "let mut {snake} = v_null();");
                }
            } else {
                let _ = writeln!(self.output, "let mut {snake} = v_null();");
            }
        }
    }

    /// BYVAL parameters are `mut` (BASIC may assign to them); BYREF ones
    /// arrive as `name__ref: &mut Value` and are copied in/out around the
    /// body (see [`Self::byref_prologue`]), matching the bytecode VM.
    fn emit_params(&self, params: &[Parameter]) -> String {
        params
            .iter()
            .map(|p| {
                let name = to_snake(&p.name);
                if p.by_ref {
                    format!("{name}__ref: &mut Value")
                } else {
                    format!("mut {name}: Value")
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Copies BYREF arguments into locals of the parameter's name and opens
    /// a closure around the body, so `RETURN` / `EXIT SUB` still reach the
    /// copy-out in [`Self::byref_epilogue`]. Returns the BYREF names.
    fn byref_prologue(&mut self, params: &[Parameter], returns_value: bool) -> Vec<String> {
        let names: Vec<String> = params.iter().filter(|p| p.by_ref).map(|p| to_snake(&p.name)).collect();
        if names.is_empty() {
            return names;
        }
        for n in &names {
            self.write_indent();
            let _ = writeln!(self.output, "let mut {n} = {n}__ref.clone();");
        }
        self.line(if returns_value { "let __result: Value = (|| -> Value {" } else { "(|| {" });
        self.indent += 1;
        names
    }

    fn byref_epilogue(&mut self, names: &[String], returns_value: bool) {
        if names.is_empty() {
            return;
        }
        self.indent -= 1;
        self.line(if returns_value { "})();" } else { "})();" });
        for n in names {
            self.write_indent();
            let _ = writeln!(self.output, "*{n}__ref = {n};");
        }
        if returns_value {
            self.line("__result");
        }
    }

    /// Arguments for a call to a user SUB/FUNCTION. BYREF positions become
    /// `&mut place`; a global variable goes through a temp that is stored
    /// back afterwards. Returns (args, statements before, statements after).
    fn user_call_args(&self, callee_lower: &str, args: &[Expression]) -> (Vec<String>, Vec<String>, Vec<String>) {
        let flags = self.fn_byref.get(callee_lower).cloned().unwrap_or_default();
        let (mut out, mut pre, mut post) = (Vec::new(), Vec::new(), Vec::new());
        let promoted = self.promoted_byref.get(callee_lower).cloned().unwrap_or_default();
        for (i, arg) in args.iter().enumerate() {
            if !flags.get(i).copied().unwrap_or(false) {
                out.push(self.owned_expr(arg));
                continue;
            }
            let arg = match arg {
                Expression::Unary(u) if u.operator == UnaryOperator::Ref => u.operand.as_ref(),
                // BYREF only because another call uses `@`: this one passes a copy.
                other if promoted.contains(&i) => {
                    out.push(format!("&mut {}", self.owned_expr(other)));
                    continue;
                }
                other => other,
            };
            match arg {
                Expression::Identifier(id) if self.is_global_scalar(&strip_type_suffix(&id.name)) => {
                    let snake = to_snake(&strip_type_suffix(&id.name));
                    let tmp = format!("__byref{i}");
                    pre.push(format!("let mut {tmp} = gv(\"{snake}\");"));
                    post.push(format!("gs(\"{snake}\", {tmp});"));
                    out.push(format!("&mut {tmp}"));
                }
                Expression::Identifier(id) => out.push(format!("&mut {}", to_snake(&strip_type_suffix(&id.name)))),
                // Not a variable: pass a temporary (nothing to write back).
                other => out.push(format!("&mut {}", self.owned_expr(other))),
            }
        }
        (out, pre, post)
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
        self.with_component_stack.push(name.clone());
        for stmt in &c.body {
            self.emit_statement(stmt);
        }
        self.with_component_stack.pop();
        self.create_stack.pop();

        // Register timers declared in CREATE blocks
        if type_upper == "RTIMER" {
            self.write_indent();
            let _ = writeln!(self.output, "gui_register_timer(\"{name}\");");
        }
    }

    /// `WITH obj … .x … END WITH`: the body with `.x` meaning `obj.x`
    /// (`rapidr_ast::resolve_with_body`, as in the VM).
    fn emit_with(&mut self, w: &WithStatement) {
        self.line("{ // WITH");
        self.indent += 1;
        for stmt in &rapidr_ast::resolve_with_body(&w.body, &w.object) {
            self.emit_statement(stmt);
        }
        self.indent -= 1;
        self.line("} // END WITH");
    }

    fn emit_exit(&mut self, e: &ExitStatement) {
        match e.exit_type.as_str() {
            kind @ ("FOR" | "WHILE" | "DO") => {
                match self.loop_labels.iter().rev().find(|(k, _)| *k == kind) {
                    // A loop flattened into states (jumps.rs): jump to its end.
                    Some((_, label)) if label.starts_with('@') => {
                        let state = label[1..].to_string();
                        self.write_indent();
                        let _ = writeln!(self.output, "{{ __pc = {state}; continue 'sm; }}");
                    }
                    Some((_, label)) => {
                        let label = label.clone();
                        self.write_indent();
                        let _ = writeln!(self.output, "break {label};");
                    }
                    None => self.line("break;"),
                }
            }
            "SUB" if self.in_state_machine() => self.line("break 'sm;"),
            "SUB" => self.line("return;"),
            "FUNCTION" => {
                if let Some(fname) = self.current_function.clone() {
                    let fname_lc = to_snake(&fname);
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
        if r.value.is_none() && self.in_state_machine() {
            // Back from a GOSUB (or out of the routine if none is pending).
            self.emit_gosub_return();
            return;
        }
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
        // `var = __input_value(INPUT(prompt), var, suffix)`, shared with the VM.
        self.emit_assignment(&rapidr_ast::input_assignment(i));
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
        // `lbl(i).OnClick = Handler` (objects.rs): an object known at run time.
        if let Expression::MemberAccess(ma) = &b.target {
            if ma.member.to_ascii_lowercase().starts_with("on") {
                if let Expression::Identifier(h) = &b.handler {
                    if self.defined_functions.contains(&strip_type_suffix(&h.name).to_lowercase()) {
                        let obj = self.expr_to_string(&ma.object);
                        let handler = to_snake(&strip_type_suffix(&h.name));
                        let arity = self.function_param_counts.get(&h.name.to_lowercase()).copied().unwrap_or(0);
                        let bind = match arity {
                            0 => "rp_bind_event".to_string(),
                            n => format!("rp_bind_event_{}", n.min(5)),
                        };
                        self.write_indent();
                        let _ = writeln!(self.output, "{bind}(&({obj}).to_string_val(), \"{}\", {handler});", ma.member.to_lowercase());
                        return;
                    }
                }
            }
        }
        // `BIND ptr TO Proc`: the pointer is the routine's id (`__callfunc`).
        let id = match &b.handler {
            Expression::Identifier(h) => self.routine_pointer(&h.name),
            _ => None,
        };
        match id {
            Some(id) => self.emit_assignment(&AssignmentStatement {
                span: b.span,
                target: b.target.clone(),
                value: Expression::Literal(Literal { span: b.span, value: LiteralValue::Integer(id as i64) }),
            }),
            // `BIND ptr TO Prototype` with no such routine only gives the
            // pointer a signature (RAPIDQ2.INC then assigns it), as in the VM.
            None if matches!(b.handler, Expression::Identifier(_)) => {}
            None => self.line("compile_error!(\"BIND … TO needs the name of a SUB or FUNCTION of the program\");"),
        }
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

        // No LIB and the SUB/FUNCTION is defined in the program: a plain
        // forward declaration (RapidQ `DECLARE SUB Foo (...)`), nothing to emit.
        if self.defined_functions.contains(&strip_type_suffix(&d.name).to_lowercase()) {
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
                if self.is_global_scalar(&stripped) || self.is_component_var(&stripped) || self.typed_local(&stripped).is_some() {
                    return s; // gv() and v_str() already return owned values
                }
            }
            format!("{s}.clone()")
        } else {
            s
        }
    }

    fn expr_to_string(&self, expr: &Expression) -> String {
        // `CODEPTR(Name)` / `CALLBACK(Name)`: the routine's pointer id.
        if let Expression::FunctionCall(fc) = expr {
            if let (Expression::Identifier(f), [Expression::Identifier(target)]) = (fc.callee.as_ref(), fc.args.as_slice()) {
                if matches!(f.name.to_lowercase().as_str(), "codeptr" | "callback") && !self.defined_functions.contains(&f.name.to_lowercase()) {
                    if let Some(id) = self.routine_pointer(&target.name) {
                        return format!("v_int({id})");
                    }
                }
            }
        }
        // Typed locals and native arithmetic on them (`typed`).
        if let Some(boxed) = self.boxed_typed(expr) {
            return boxed;
        }
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
                // Builtins written without parentheses (the bytecode's
                // `BARE_BUILTINS`), unless a variable has that name, as in
                // the VM.
                let bare = strip_type_suffix(&name_lower);
                let variable = self.is_global_scalar(&bare)
                    || self.is_global_array(&bare)
                    || self.shadowed.contains(&bare)
                    || self.assigned_main.contains(&bare)
                    || self.assigned_routine.contains(&bare);
                let bare_builtin = match name_lower.as_str() {
                    _ if variable => None,
                    "pi" => Some("v_dbl(std::f64::consts::PI)"),
                    "time" | "time$" => Some("rp_time()"),
                    "date" | "date$" => Some("rp_date()"),
                    "command$" => Some("rp_command()"),
                    "timer" => Some("rp_timer()"),
                    "csrlin" => Some("console::csrlin()"),
                    "curdir" | "curdir$" => Some("rp_curdir()"),
                    "rnd" | "rnd!" | "rnd#" => Some("rp_rnd(&v_null())"),
                    "dir" | "dir$" => Some("rp_dir(&v_null(), &v_null())"),
                    "resourcecount" => Some("rp_resourcecount()"),
                    "mousex" => Some("rp_mousex()"),
                    "mousey" => Some("rp_mousey()"),
                    _ => None,
                };
                if let Some(call) = bare_builtin {
                    return call.to_string();
                }
                match name_lower.as_str() {
                    "true" | "vttrue" => "v_bool(true)".to_string(),
                    "false" | "vtfalse" => "v_bool(false)".to_string(),
                    // An omitted argument (`COLOR , 1`, `INSTR(, a, b)`)
                    "__omitted" => "v_null()".to_string(),
                    _ => {
                        let name = strip_type_suffix(&id.name);
                        let snake = to_snake(&name);
                        // Component var used as bare expression → emit its name as a string value
                        if self.component_vars.contains_key(&name.to_lowercase()) {
                            return format!("v_str(\"{snake}\")");
                        }
                        // Module-level scalar or array → read from global storage
                        if self.is_global_scalar(&name) || self.is_global_array(&name) {
                            return format!("gv(\"{snake}\")");
                        }
                        // RapidQ's RESULT inside a FUNCTION is its return value.
                        if name.eq_ignore_ascii_case("result") {
                            if let Some(fname) = &self.current_function {
                                return format!("_{}.clone()", to_snake(fname));
                            }
                        }
                        // A FUNCTION named without parentheses is called
                        // (`y = Five + 1`), except inside itself, where the
                        // name is its result variable.
                        let lower = name.to_lowercase();
                        // A SUB's name as a value (`WndProc = MyProc`) is
                        // nothing, as in the VM.
                        if !self.returning_functions.contains(&lower)
                            && self.function_param_counts.keys().any(|f| strip_type_suffix(f).eq_ignore_ascii_case(&name))
                            && !self.shadowed.contains(&lower)
                        {
                            return "v_null()".to_string();
                        }
                        if self.returning_functions.contains(&lower)
                            && self.function_param_counts.get(&lower).copied().unwrap_or(0) == 0
                            && !self.current_function.as_deref().is_some_and(|f| strip_type_suffix(f).eq_ignore_ascii_case(&name))
                        {
                            return format!("{snake}()");
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
                    // `@x` outside a user SUB/FUNCTION call (a DLL argument):
                    // the variable's address, like VARPTR(x).
                    UnaryOperator::Ref => format!("rp_varptr(&{operand})"),
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
                        let base = self.array_base(&name_stripped);
                        return format!("{base}.rp_get(&[{}])", self.index_list(&fc.args));
                    }

                    if let Some(rust_call) = builtin_function_call(&name_stripped, &args) {
                        return rust_call;
                    }
                    // Check if it's a known function/sub or declared FFI function
                    if self.defined_functions.contains(&name_stripped) {
                        let fname = to_snake(&strip_type_suffix(&id.name));
                        if self.fn_byref.get(&name_stripped).is_some_and(|f| f.contains(&true)) {
                            let (args, pre, post) = self.user_call_args(&name_stripped, &fc.args);
                            return format!(
                                "{{ {} let __r = {fname}({}); {} __r }}",
                                pre.join(" "), args.join(", "), post.join(" ")
                            );
                        }
                        return self.fitted_call(&name_stripped, &fname, args);
                    }
                    // Not a known function: a variable's element (variant
                    // indexing), or — only in unreached library code — a
                    // routine nobody defines, which fails when run (as in
                    // the VM).
                    let varname = to_snake(&strip_type_suffix(&id.name));
                    let variable = self.shadowed.contains(&name_stripped)
                        || self.assigned_main.contains(&name_stripped)
                        || self.assigned_routine.contains(&name_stripped)
                        || self.is_global_scalar(&name_stripped);
                    if args.len() == 1 && variable {
                        return format!("{varname}.rp_index(&{})", args[0]);
                    }
                    return unknown_routine(&id.name);
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

                    // Nested static call: Type.namespace.method(args) e.g. RNum.random.randint()
                    if let Expression::MemberAccess(inner_ma) = ma.object.as_ref() {
                        if let Expression::Identifier(id) = inner_ma.object.as_ref() {
                            let var_lower = id.name.to_lowercase();
                            if is_component_type_name(&id.name) || var_lower == "math" {
                                let namespace = inner_ma.member.to_lowercase();
                                let method = ma.member.to_lowercase();
                                let combined = format!("{namespace}_{method}");
                                if let Some(rust_call) = builtin_function_call(&combined, &args) {
                                    return rust_call;
                                }
                                return format!("{{ eprintln!(\"[WARN] {}.{}.{}() not implemented\"); v_null() }}", id.name, inner_ma.member, ma.member);
                            }
                        }
                    }

                    if let Expression::Identifier(id) = ma.object.as_ref() {
                        let var_lower = id.name.to_lowercase();
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

                if let Expression::MemberAccess(ma) = fc.callee.as_ref() {
                    let args: Vec<&Expression> = fc.args.iter().collect();
                    return self.object_method_call(&ma.object, &ma.member, &args);
                }

                // Complex callee
                let callee = self.expr_to_string(fc.callee.as_ref());
                let args_str = args.join(", ");
                format!("{callee}({args_str})")
            }
            Expression::MemberAccess(ma) => {
                // Component property/method access
                if let Some(comp_name) = self.get_component_name(&ma.object) {
                    let member_lower = ma.member.to_lowercase();
                    if is_component_method_name(&member_lower) && !is_also_property(&member_lower) {
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
                            if is_component_method_name(&member_lower) && !is_also_property(&member_lower) {
                                return format!("rp_comp_method(\"{with_comp}\", \"{member_lower}\", &[])");
                            }
                            return format!("rp_comp_get(\"{with_comp}\", \"{member_lower}\")");
                        }
                    }
                }

                // `Obj.Font.Color` on any other object: its `font.color`.
                if let Expression::MemberAccess(inner) = ma.object.as_ref() {
                    if matches!(inner.object.as_ref(), Expression::Identifier(id) if id.name != "_with_") {
                        return format!(
                            "rp_comp_get({}, \"{}.{}\")",
                            self.receiver(&inner.object),
                            inner.member.to_lowercase(),
                            ma.member.to_lowercase()
                        );
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

                // Any other object (`Screen.Width`, an event handler's
                // `Sender.Caption`); `Dlg.Execute` is a call
                // (rapidr_ast::VALUE_METHODS, as in the VM).
                if rapidr_ast::VALUE_METHODS.contains(&member_lower.as_str()) {
                    return format!("rp_comp_method({}, \"{member_lower}\", &[])", self.receiver(&ma.object));
                }
                format!("rp_comp_get({}, \"{member_lower}\")", self.receiver(&ma.object))
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
                let is_array = if let Expression::Identifier(id) = aa.array.as_ref() {
                    self.array_vars.contains(&strip_type_suffix(&id.name).to_lowercase())
                } else {
                    false
                };
                let _ = arr;
                if let (true, Expression::Identifier(id)) = (is_array, aa.array.as_ref()) {
                    let base = self.array_base(&strip_type_suffix(&id.name).to_lowercase());
                    format!("{base}.rp_get(&[{}])", self.index_list(&aa.indices))
                } else {
                    // Value-based indexing
                    let target = self.expr_to_string(&aa.array);
                    format!("{target}.rp_get(&[{}])", self.index_list(&aa.indices))
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub(crate) fn to_snake(name: &str) -> String {
    // Strip type suffixes first
    let name = strip_type_suffix(name);
    // Just lowercase for now since BASIC names are case-insensitive; a dotted
    // DECLARE name (`SLEEP.ms`) becomes `sleep_ms`.
    // Type suffixes inside a combined name (`QDebug.Err$` → `_qdebug__err$`)
    // aren't part of a Rust identifier either.
    let lower: String = name.to_lowercase().replace('.', "_").chars().filter(|c| !matches!(c, '$' | '%' | '&' | '!' | '#')).collect();
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
        // RapidQ names may start with a digit (`SUB 01click`); Rust's can't.
        _ if lower.starts_with(|c: char| c.is_ascii_digit()) => format!("n_{lower}"),
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
        "INTEGER" | "BYTE" | "WORD" | "DWORD" | "LONG" | "SHORT" | "INT64" => "v_int(0)".to_string(),
        "DOUBLE" | "SINGLE" | "CURRENCY" => "v_dbl(0.0)".to_string(),
        "STRING" => "v_str(\"\")".to_string(),
        _ => "v_null()".to_string(),
    }
}

/// Map a known BASIC builtin function name to a Rust runtime call.
/// `(lo1, hi1), (lo2, hi2)…` from bound pairs (compiled Rust expressions).
fn bounds_list(args: &[String]) -> String {
    args.chunks(2)
        .map(|b| format!("(({}).to_i64(), ({}).to_i64())", b[0], b.get(1).map(|s| s.as_str()).unwrap_or("v_int(0)")))
        .collect::<Vec<_>>()
        .join(", ")
}

/// `i1, i2…` as i64 array indices.
fn index_list(args: &[String]) -> String {
    args.iter().map(|i| format!("({i}).to_i64()")).collect::<Vec<_>>().join(", ")
}

/// The object pass's own builtins (rapidr_ast::objects::OBJECT_BUILTINS).
/// Names a SUB/FUNCTION declares itself — parameters and (non-STATIC)
/// DIMs — which shadow module-level variables inside it, as in the VM.
pub(crate) fn shadowing_names(params: &[Parameter], body: &[Statement]) -> HashSet<String> {
    let mut names: HashSet<String> = params.iter().map(|p| strip_type_suffix(&p.name).to_lowercase()).collect();
    rapidr_ast::walk(
        body,
        &mut |s| {
            if let Statement::Dim(d) = s {
                if !d.is_static && !d.is_redim {
                    names.extend(d.declarators.iter().map(|v| strip_type_suffix(&v.name).to_lowercase()));
                }
            }
        },
        &mut |_| {},
    );
    names
}

/// A call of a routine the program doesn't define (in library code the
/// program never reaches): a run-time error, the VM's message.
fn unknown_routine(name: &str) -> String {
    let name = name.replace('\\', "").replace('"', "");
    format!("{{ eprintln!(\"run-time error: Unknown builtin function '{name}'\"); std::process::exit(1) }}")
}

/// Variables `stmts` assign to by name (`RGB$ = …`, `FOR i`, `INPUT s$`),
/// lowercase without type suffix: a variable even when a builtin has that
/// name.
fn assigned_names(stmts: &[Statement]) -> HashSet<String> {
    let mut names = HashSet::new();
    rapidr_ast::walk(
        stmts,
        &mut |s| {
            let name = match s {
                Statement::Assignment(a) => match &a.target {
                    Expression::Identifier(id) => Some(&id.name),
                    _ => None,
                },
                Statement::Input(i) => match &i.target {
                    Expression::Identifier(id) => Some(&id.name),
                    _ => None,
                },
                Statement::For(f) => Some(&f.variable),
                _ => None,
            };
            if let Some(name) = name {
                names.insert(strip_type_suffix(name).to_lowercase());
            }
        },
        &mut |_| {},
    );
    names
}

fn is_object_builtin(name: &str) -> bool {
    rapidr_ast::objects::OBJECT_BUILTINS.contains(&name)
}

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
        "instr" => {
            // BASIC INSTR has two arities:
            //   2-arg: INSTR(haystack, needle)            → start defaults to 1
            //   3-arg: INSTR(start, haystack, needle)
            if args.len() >= 3 {
                Some(format!("rp_instr(&{a0}, &{a1}, &{a2})"))
            } else {
                Some(format!("rp_instr(&v_int(1), &{a0}, &{a1})"))
            }
        }
        "space" => Some(format!("rp_space(&{a0})")),
        "string" => Some(format!("rp_string_func(&{a0}, &{a1})")),
        "chr" => Some(format!("rp_chr(&{a0})")),
        "asc" => Some(format!("rp_asc(&{a0})")),
        "replace" => Some(format!("rp_replace(&{a0}, &{a1}, &{a2})")),
        "replacesubstr" => Some(format!("rp_replacesubstr(&{a0}, &{a1}, &{a2})")),
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
        "messagebox" => Some(format!("rp_messagebox(&{a0}, &{a1}, &{a2})")),
        "messagedlg" => Some(format!("rp_messagedlg(&{a0}, &{a1}, &{a2}, &v_null())")),
        "direxists" => Some(format!("rp_direxists(&{a0})")),
        "fileexists" => Some(format!("rp_fileexists(&{a0})")),
        "resource" => Some(format!("rp_resource(&{a0})")),
        "resourcecount" => Some("rp_resourcecount()".to_string()),
        "playwav" => Some(format!("{{ rp_playwav(&{a0}, &{a1}); v_null() }}")),
        "mousex" => Some("rp_mousex()".to_string()),
        "mousey" => Some("rp_mousey()".to_string()),
        "extractresource" => Some(format!("{{ rp_extractresource(&{a0}, &{a1}); v_null() }}")),
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
        // Console (RapidQ appendix C), as ANSI sequences
        "cls" => Some("{ rp_print(&[v_str(&console::cls())], false); v_null() }".to_string()),
        "color" => Some(format!("{{ rp_print(&[v_str(&console::color(&{a0}, &{a1}))], false); v_null() }}")),
        "locate" => Some(format!("{{ rp_print(&[v_str(&console::locate(&{a0}, &{a1}))], false); v_null() }}")),
        "csrlin" => Some("console::csrlin()".to_string()),
        "pos" => Some("console::pos()".to_string()),
        "shl" => Some(format!("rp_shl(&{a0}, &{a1})")),
        "inv" => Some(format!("rp_inv(&{a0}, &{a1})")),
        "shr" => Some(format!("rp_shr(&{a0}, &{a1})")),
        // SUBI / FUNCTIONI arguments (inserted by the parser)
        "__pack" => Some(format!("variadic::pack(&[{}])", args.join(", "))),
        "__paramstr" => Some(format!("variadic::param_str(&{a0}, &{a1})")),
        "__paramval" => Some(format!("variadic::param_val(&{a0}, &{a1})")),
        "__paramstrcount" => Some(format!("variadic::param_str_count(&{a0})")),
        "__paramvalcount" => Some(format!("variadic::param_val_count(&{a0})")),
        // DATA / READ / RESTORE (inserted by the parser)
        "__data_reset" => Some("{ data::reset(); v_null() }".to_string()),
        "__data_add" => Some(format!("{{ data::add(&[{}]); v_null() }}", args.join(", "))),
        "__data_label" => Some(format!("{{ data::label(&{a0}, &{a1}); v_null() }}")),
        "__read" => Some("data::read_compiled()".to_string()),
        // Objects (rapidr_ast::objects): instances with field slots —
        // direct vector access, no lookup by name.
        "__null" => Some("v_null()".to_string()),
        // Stores into declared numeric types (rapidr_ast::numeric).
        "__to_byte" => Some(format!("numeric::to_byte(&{a0})")),
        "__to_word" => Some(format!("numeric::to_word(&{a0})")),
        "__to_short" => Some(format!("numeric::to_short(&{a0})")),
        "__to_long" => Some(format!("numeric::to_long(&{a0})")),
        "__to_dword" => Some(format!("numeric::to_dword(&{a0})")),
        "__to_double" => Some(format!("numeric::to_double(&{a0})")),
        "__newobject" => Some(format!("rp_new_object(&{a0}, &{a1}, &{a2})")),
        "__getfield" => Some(format!("obj_field(&{a0}, ({a1}).to_i64() as usize)")),
        "__setfield" => Some(format!("{{ set_obj_field(&{a0}, ({a1}).to_i64() as usize, ({a2}).clone()); v_null() }}")),
        "__objectarray" => Some(format!(
            "rp_new_object_array(&{a0}, &{a1}, &{a2}, &[{}])",
            bounds_list(args.get(3..).unwrap_or(&[]))
        )),
        "__newarray" => Some(format!("rp_new_array(&[{}], ({a0}).clone())", bounds_list(args.get(1..).unwrap_or(&[])))),
        "__aget" => Some(format!("({a0}).rp_get(&[{}])", index_list(args.get(1..).unwrap_or(&[])))),
        "__aset" => {
            let (value, idx) = args.get(1..)?.split_last()?;
            Some(format!("{{ ({a0}).rp_set(&[{}], ({value}).clone()); v_null() }}", index_list(idx)))
        }
        "__objget" => Some(format!("rp_comp_get(&({a0}).to_string_val(), &({a1}).to_string_val())")),
        "__objset" => Some(format!("{{ rp_comp_set(&({a0}).to_string_val(), &({a1}).to_string_val(), ({a2}).clone()); v_null() }}")),
        "__objcreate" => Some(format!("{{ rp_create_component(&({a0}).to_string_val(), &({a1}).to_string_val()); v_null() }}")),
        "__objcall" => Some(format!(
            "rp_comp_method(&({a0}).to_string_val(), &({a1}).to_string_val(), &[{}])",
            args.iter().skip(2).map(|a| format!("({a}).clone()")).collect::<Vec<_>>().join(", ")
        )),
        "__component_array" => Some(format!(
            "rp_component_array(&({a0}).to_string_val(), &({a1}).to_string_val(), &[{}])",
            bounds_list(args.get(2..).unwrap_or(&[]))
        )),
        // Handlers bound to an object known at run time: through the
        // function-pointer table (`__callfunc`), `This` first for EVENTs.
        "__bind_event" => Some(format!(
            "{{ let ptr = ({a2}).clone(); rp_bind_event_closure(&({a0}).to_string_val(), &({a1}).to_string_val(), std::rc::Rc::new(move |args: &[Value]| {{ __callfunc(&ptr, args); }})); v_null() }}"
        )),
        "__bind_event_this" => {
            let a3 = args.get(3).map(|s| s.as_str()).unwrap_or("v_null()");
            Some(format!(
                "{{ let ptr = ({a2}).clone(); let this = ({a3}).clone(); rp_bind_event_closure(&({a0}).to_string_val(), &({a1}).to_string_val(), std::rc::Rc::new(move |args: &[Value]| {{ let mut all = vec![this.clone()]; all.extend_from_slice(args); __callfunc(&ptr, &all); }})); v_null() }}"
            ))
        }
        "__input_value" => Some(format!("input_value(&{a0}, &{a1}, &({a2}).to_string_val())")),
        "__restore" => Some(if args.is_empty() {
            "data::restore_compiled(None)".to_string()
        } else {
            format!("data::restore_compiled(Some(&{a0}))")
        }),
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

        // RNum static helpers
        "array" => Some(format!("{a0}.clone()")),
        "random_randint" => Some(format!("rp_randint(&{a0}, &{a1}, &{a2})")),

        // Array functions
        "lbound" => Some(format!("({a0}).rp_bound(&{a1}, false)")),
        "ubound" => Some(format!("({a0}).rp_bound(&{a1}, true)")),

        // Misc
        "sound" => Some(format!("rp_sound(&{a0}, &{a1})")),
        "sndplayasync" | "playsound" => Some(format!("rp_sound(&{a0}, &{a1})")),

        // Pointer helpers
        "callfunc" => Some(format!(
            "__callfunc(&{a0}, &[{}])",
            args.iter().skip(1).map(|a| format!("({a}).clone()")).collect::<Vec<_>>().join(", ")
        )),
        "codeptr" | "callback" => Some(
            "compile_error!(\"CODEPTR(Name) takes the name of a SUB or FUNCTION of the program\")".to_string(),
        ),
        "varptr" => Some(format!("rp_varptr(&{a0})")),
        "varptr$" => Some(format!("rp_varptr_str(&{a0})")),

        _ => None,
    }
}

/// The Cargo package (and binary) name for a program file named `stem`:
/// Cargo accepts letters, digits, `-` and `_`, not starting with a digit
/// (`Cancel Form Close` → `Cancel_Form_Close`, `3dview` → `rq_3dview`).
pub fn crate_name(stem: &str) -> String {
    let name: String = stem.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
    if name.is_empty() || name.starts_with(|c: char| c.is_ascii_digit() || c == '-') {
        format!("rq_{name}")
    } else {
        name
    }
}

/// Generate a Cargo.toml for the output project that depends on the runtime.
pub fn generate_cargo_toml(project_name: &str, runtime_path: &str) -> String {
    let project_name = crate_name(project_name);
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

/// Generate a Cargo.toml for a web (WASM) project.
pub fn generate_cargo_toml_web(project_name: &str, runtime_web_path: &str) -> String {
    let project_name = crate_name(project_name);
    format!(
        r#"[package]
name = "{project_name}"
version = "0.1.0"
edition = "2021"

[workspace]

[lib]
crate-type = ["cdylib"]

[dependencies]
rapidr-runtime-web = {{ path = "{runtime_web_path}" }}
wasm-bindgen = "=0.2.118"
"#
    )
}

// ---------------------------------------------------------------------------
// Component system helpers (compile-time, no runtime dependency)
// ---------------------------------------------------------------------------

/// Check if a type name is a known RapidP component type.
fn is_component_type_name(type_name: &str) -> bool {
    rapidr_ast::is_component_type_name(type_name)
}

/// A method that is also a property, read as the property (`Img.Center`:
/// QIMAGE's Center setting; `Form.Center` as a statement centers the form).
fn is_also_property(member: &str) -> bool {
    member == "center"
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
        // JSON methods
        | "parse" | "stringify" | "prettify" | "has" | "remove" | "keys"
        | "loadfile" | "savefile"
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
        // NumPy/RNum methods
        | "array" | "zeros" | "ones" | "full" | "arange" | "linspace" | "reshape"
        | "fromlist" | "from_list"
        | "sum" | "mean" | "min" | "max" | "std" | "var" | "variance" | "median"
        | "argmin" | "argmax" | "count" | "ptp" | "dot" | "norm" | "normalize"
        | "sin" | "cos" | "tan" | "asin" | "arcsin" | "acos" | "arccos" | "atan" | "arctan"
        | "sqrt" | "abs" | "exp" | "log" | "ln" | "log2" | "log10"
        | "floor" | "ceil" | "round" | "sign" | "reciprocal" | "square" | "negative" | "neg"
        | "add" | "subtract" | "sub" | "multiply" | "mul" | "divide" | "div"
        | "power" | "pow" | "mod" | "fmod" | "clip" | "clamp"
        | "reverse" | "flip" | "unique" | "shuffle" | "append" | "concatenate" | "slice"
        | "cumsum" | "cumprod" | "diff" | "any" | "all" | "nonzero" | "searchsorted"
        | "rand" | "random" | "randn" | "random_normal" | "normal"
        | "uniform" | "random_uniform" | "randint" | "choice"
        | "tolist" | "tostring" | "print"
        // RDataFrame methods
        | "readcsv" | "read_csv" | "loadfromcsv"
        | "savetocsv" | "to_csv" | "writecsv"
        | "loadfromjson" | "read_json" | "savetojson" | "to_json"
        | "head" | "tail" | "describe" | "columns" | "info" | "dtypes" | "shape"
        | "cellbyname" | "at" | "setcell" | "iloc" | "select"
        | "sort_values" | "filter" | "query"
        | "groupby" | "group_by" | "value_counts" | "nunique" | "corr" | "correlation"
        | "drop" | "drop_column" | "rename" | "rename_column"
        | "addcolumn" | "add_column" | "set_column"
        | "fillna" | "fill_null" | "dropna" | "drop_nulls"
        | "sample" | "nlargest" | "nsmallest"
        | "merge" | "join" | "concat"
        | "transpose" | "apply" | "replace"
        | "togrid" | "to_grid" | "display"
        // RPlot methods
        | "plot" | "bar" | "barh" | "scatter" | "step" | "area" | "fill_between"
        | "hist" | "histogram" | "pie"
        | "hline" | "axhline" | "vline" | "axvline" | "annotate"
        | "legend" | "savefig" | "save" | "figsize" | "xlim" | "ylim" | "xscale" | "yscale"
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
        // Web-exclusive methods
        | "sethtml" | "navigate" | "appendto" | "setattribute" | "getattribute"
        | "addclass" | "removeclass" | "toggleclass" | "queryselector" | "queryselectorall"
        | "eval" | "call" | "set" | "haskey" | "keys"
        | "play" | "pause" | "stop" | "seek" | "fullscreen"
        | "requestpermission" | "getposition" | "watchposition" | "clearwatch"
        | "addroute" | "back" | "forward"
        // Web file-bridge methods (RFILESTREAM)
        | "pickfile" | "download" | "loadfromurl"
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

/// Collect names declared in nested CREATE blocks for DIM deduplication.
fn collect_nested_create_names(stmts: &[Statement], names: &mut HashSet<String>) {
    for stmt in stmts {
        if let Statement::Create(c) = stmt {
            names.insert(c.name.to_lowercase());
            collect_nested_create_names(&c.body, names);
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
                        match v {
                            CaseValue::Value(e) | CaseValue::Is(_, e) => collect_expr_refs(e, refs),
                            CaseValue::Range(low, high) => {
                                collect_expr_refs(low, refs);
                                collect_expr_refs(high, refs);
                            }
                        }
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
        let program = parse_tokens(&tokens).expect("test source should parse");
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
    fn dim_generates_global_storage() {
        // A top-level STRING lives in a global slot…
        let rust = gen("DIM s AS STRING\n");
        assert!(rust.contains("gs(0, v_str(\"\"));"));
        // …a numeric one in its typed static (typed::analyze_globals).
        let rust = gen("DIM x AS INTEGER\n");
        assert!(rust.contains("static TG_X: std::sync::atomic::AtomicI64"));
        assert!(rust.contains("TG_X.store(0_i64"));
    }

    #[test]
    fn for_loop_generates_while() {
        let code = "DIM i AS INTEGER\nFOR i = 1 TO 5\n  PRINT i\nNEXT i\n";
        let rust = gen(code);
        // i is a typed top-level variable: a native loop over its static.
        assert!(rust.contains("while (if __for_step_i >= 0"));
        assert!(rust.contains("TG_I.store(TG_I.load(std::sync::atomic::Ordering::Relaxed).wrapping_add(__for_step_i)"));
    }

    #[test]
    fn sub_generates_fn() {
        let code = "SUB MySub(msg AS STRING)\n  PRINT msg\nEND SUB\n";
        let rust = gen(code);
        assert!(rust.contains("fn mysub(mut msg: Value)"));
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

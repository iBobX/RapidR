//! AST → bytecode lowering for RapidR.
//!
//! Entry point: [`compile_program`] takes a parsed [`rapidr_ast::Program`]
//! and produces a [`rapidr_bytecode::Module`] ready for the VM.
//!
//! ## Design
//!
//! * Top-level statements (everything outside SUB/FUNCTION) are lowered
//!   into the implicit `__main` function.
//! * Each `SUB`/`FUNCTION` becomes its own [`Function`] entry.
//! * Identifier resolution is case-insensitive (see [`name_key`]): locals
//!   first (per-function scope), then globals by name. SUB / FUNCTION names
//!   are resolved to function indices for `CallSub` / `CallFunc`. Any other
//!   callee must be a builtin from [`rapidr_bytecode::builtins`] and becomes
//!   a `CallBuiltin`; unknown names are compile errors.
//! * `CREATE Foo AS Kind ... END CREATE` lowers to `CreateComp(Kind, Foo)`
//!   followed by per-property `SetProp` and per-method `CallMethod` (a bare
//!   call inside the block is a method of the object being created).
//!   When a property assignment's RHS is a bare identifier matching a known
//!   SUB, it is lowered to `RegisterEvent` (e.g. `OnClick = MyHandler`).
//!
//! Nothing is silently skipped: constructs the interpreter can't run yet
//! (DLL calls, RUSTSTART blocks, TYPE methods, …) are compile errors with a
//! line and column, and every error in the program is reported at once.

use std::collections::{HashMap, HashSet};

use rapidr_ast::{
    ArrayAccessExpression, ArrayDimension, AssignmentStatement, BinaryOperator, BindStatement,
    CallStatement, CaseValue, VariableDeclarator,
    CreateStatement, DoLoopStatement, Expression, ForStatement, FunctionStatement, IfStatement,
    Literal, LiteralValue, Parameter, PrintStatement, Program, ReturnStatement, Statement,
    SubroutineStatement, UnaryOperator, WhileStatement,
};
use rapidr_bytecode::{builtins, Const, Function, Module, Op, Param};
use rapidr_diagnostics::TextSpan;

/// Result of compilation: the produced module plus any non-fatal warnings.
pub struct Compiled {
    pub module: Module,
    pub warnings: Vec<String>,
}

/// Compile a full program to a bytecode module.
pub fn compile_program(program: &Program) -> Result<Compiled, String> {
    compile_program_with_source(program, None)
}

/// Compile a full program to a bytecode module, mapping text spans back to source lines.
pub fn compile_program_with_source(program: &Program, source: Option<&str>) -> Result<Compiled, String> {
    compile_program_with_libraries(program, source, &[])
}

/// Like [`compile_program_with_source`]; `library_lines[n]` is true when
/// line n+1 of the source came from an `$INCLUDE`d library. In library code
/// that the program never reaches, names and features the interpreter
/// doesn't know aren't errors (RapidQ's own libraries use Win32 routines
/// and built-ins RapidR lacks); the program's own code is always checked.
pub fn compile_program_with_libraries(program: &Program, source: Option<&str>, library_lines: &[bool]) -> Result<Compiled, String> {
    // Objects → plain routines and builtins, the same pass native builds
    // run (rapidr_ast::objects), so both backends treat objects alike.
    let hoisted = rapidr_ast::stream_arrays::lower(&rapidr_ast::routine_objects(&rapidr_ast::hoist_routines(program)));
    let lowered = rapidr_ast::objects::lower(&hoisted, &|n| builtins::is_builtin(n));
    // Stores into declared numeric types convert (rapidr_ast::numeric).
    let lowered = rapidr_ast::numeric::lower(lowered);
    let program = &lowered;
    let mut bcgen = Bcgen::new();
    bcgen.library_lines = library_lines.to_vec();
    if let Some(src) = source {
        let mut starts = vec![0];
        for (offset, c) in src.char_indices() {
            if c == '\n' {
                starts.push(offset + 1);
            }
        }
        bcgen.line_starts = Some(starts);
    }
    bcgen.compile_program(program)?;
    Ok(Compiled { module: bcgen.module, warnings: bcgen.warnings })
}

/// BASIC identifiers are case-insensitive and may carry a type suffix
/// (`Name$`, `Count%`): all name lookups go through this key.
fn name_key(name: &str) -> String {
    let mut key = name.to_ascii_lowercase();
    if matches!(key.chars().last(), Some('$' | '%' | '#' | '&' | '!')) && key.len() > 1 {
        key.pop();
    }
    key
}

/// Map keyed case-insensitively by BASIC identifier (see [`name_key`]).
#[derive(Default, Clone)]
struct NameMap<V>(HashMap<String, V>);

impl<V> NameMap<V> {
    fn get(&self, name: &str) -> Option<&V> {
        self.0.get(&name_key(name))
    }
    fn contains_key(&self, name: &str) -> bool {
        self.0.contains_key(&name_key(name))
    }
    fn insert(&mut self, name: String, value: V) {
        self.0.insert(name_key(&name), value);
    }
}

/// Per-function scope: maps local variable names to slot indices.
#[derive(Default, Clone)]
struct Scope {
    locals: NameMap<u16>,
    /// Declared TYPE of locals/params that hold objects (for method calls).
    types: NameMap<String>,
    /// Spelling of each slot as first written, for the debugger.
    display: Vec<String>,
    next_slot: u16,
    /// `STATIC` variables of this routine → the hidden global holding each
    /// (`Routine::name`), shared by every call.
    statics: NameMap<String>,
    /// Label of the routine this scope belongs to.
    owner: String,
    /// Local arrays of objects (`DIM lbl(3) AS QLABEL`) → element type.
    object_arrays: NameMap<String>,
}

impl Scope {
    fn declare(&mut self, name: &str) -> u16 {
        if let Some(&s) = self.locals.get(name) {
            return s;
        }
        let s = self.next_slot;
        self.next_slot += 1;
        self.locals.insert(name.to_string(), s);
        self.display.push(name.to_string());
        s
    }
    fn get(&self, name: &str) -> Option<u16> {
        self.locals.get(name).copied()
    }
}

struct Bcgen {
    module: Module,
    /// Map SUB/FUNCTION names → function index.
    fn_indices: NameMap<u32>,
    /// Whether each name is a FUNCTION (true) or SUB (false). Used to choose
    /// CallFunc vs CallSub when invoked from an expression.
    fn_is_func: NameMap<bool>,
    /// Reachability key of the routine being compiled (`name_key` of a
    /// SUB/FUNCTION, `type:<name>` for TYPE code); `None` in the main program.
    current_routine: Option<String>,
    /// Which source lines came from `$INCLUDE`d libraries (see
    /// [`compile_program_with_libraries`]).
    library_lines: Vec<bool>,
    /// Errors about things only native builds can do (DLL calls, VARPTR, …)
    /// raised inside a routine: reported only if the program can reach it,
    /// so unused parts of big include libraries don't block a program.
    deferred_errors: HashMap<String, Vec<String>>,
    warnings: Vec<String>,
    /// Compile errors ("line:col: error: message"). Collected rather than
    /// returned immediately so one compile reports every problem.
    errors: Vec<String>,
    /// DECLARE ... LIB (external DLL) function names, by [`name_key`].
    lib_functions: HashSet<String>,
    /// Library of each `DECLARE … LIB` routine (`name_key` → lowercase DLL name).
    lib_of: HashMap<String, String>,
    /// BYREF flag of each parameter, per SUB/FUNCTION.
    fn_byref: NameMap<Vec<bool>>,
    /// The SUB/FUNCTION being lowered, if any.
    fn_ctx: Option<FnCtx>,
    /// Labels, GOTO/GOSUB jumps and GOSUB use of the routine being lowered.
    routine: RoutineLabels,
    /// Global arrays of objects (`DIM lbl(3) AS QLABEL`) → element type.
    global_object_arrays: NameMap<String>,
    /// Scope stack for the function currently being lowered.
    scope: Scope,
    /// Set of declared global variables (keys from [`name_key`])
    globals: HashSet<String>,
    /// Global variable spelling as first seen, by [`name_key`]. The VM keys
    /// globals by string, so every access must use the same spelling.
    global_spelling: HashMap<String, String>,
    /// Active "WITH object" name (or None). Bare member-access on the
    /// implicit object is not yet a separate AST node, so this is reserved.
    _with_object: Option<String>,
    /// Stack of (continue_target, breaks_to_patch) for loops, used by EXIT.
    loop_stack: Vec<LoopCtx>,
    /// CREATE-block instance name stack (for nested CREATE).
    create_stack: Vec<String>,
    /// Names (lowercase) declared via CREATE anywhere in the program.
    /// Used so a redundant `DIM x AS RForm` after `CREATE x AS RForm` does
    /// not emit a duplicate `CreateComp`.
    create_declared_names: HashSet<String>,
    /// All component instance names (lowercase) declared anywhere in the
    /// program — via CREATE or via `DIM x AS <ComponentType>`. Maps the
    /// original-case name (as written) to lowercase id. Used to detect
    /// when an RHS identifier (e.g. `Label1.Parent = Form1`) refers to
    /// a component instance, so we emit `LoadConst(v_str("form1"))` +
    /// `SetProp` instead of trying to load a non-existent global.
    component_instance_names: HashMap<String, String>,
    /// Component type (canonical, uppercase) of each CREATE/DIM component
    /// instance (lowercase name), for `SB.Panel(0).Width`.
    component_kinds: HashMap<String, String>,
    /// Whether we are currently lowering the top-level main program.
    in_main: bool,
    /// Starts of each line (byte offsets) to resolve line numbers for statements.
    line_starts: Option<Vec<usize>>,
}

struct LoopCtx {
    /// "FOR", "WHILE" or "DO" — `EXIT FOR` leaves the innermost FOR even
    /// from inside a nested WHILE.
    kind: &'static str,
    /// Patch sites (offsets that hold a u32 target) waiting for the loop end.
    breaks: Vec<usize>,
}

/// Line labels of one routine (main program, SUB or FUNCTION): GOTO/GOSUB
/// can only jump within the routine they appear in.
#[derive(Default)]
struct RoutineLabels {
    /// Label (by [`name_key`]) → code offset.
    offsets: HashMap<String, u32>,
    /// Jumps waiting for their label: (label as written, patch offset, span).
    pending: Vec<(String, usize, TextSpan)>,
    /// Whether the routine contains GOSUB, so RETURN must check for one.
    uses_gosub: bool,
}

/// The SUB/FUNCTION whose body is being lowered (None in the main program).
#[derive(Clone, Copy)]
struct FnCtx {
    /// FUNCTIONs return the value assigned to their own name, kept in this
    /// local (`Fact = n * Fact(n - 1)`); SUBs have none.
    result_slot: Option<u16>,
}

impl Bcgen {
    fn new() -> Self {
        Self {
            module: Module::new(),
            fn_indices: NameMap::default(),
            fn_is_func: NameMap::default(),
            current_routine: None,
            library_lines: Vec::new(),
            deferred_errors: HashMap::new(),
            warnings: Vec::new(),
            errors: Vec::new(),
            lib_functions: HashSet::new(),
            lib_of: HashMap::new(),
            fn_byref: NameMap::default(),
            fn_ctx: None,
            routine: RoutineLabels::default(),
            global_object_arrays: NameMap::default(),
            scope: Scope::default(),
            globals: HashSet::new(),
            global_spelling: HashMap::new(),
            _with_object: None,
            loop_stack: Vec::new(),
            create_stack: Vec::new(),
            create_declared_names: HashSet::new(),
            component_instance_names: HashMap::new(),
            component_kinds: HashMap::new(),
            in_main: false,
            line_starts: None,
        }
    }

    // ------------------- top-level driver -------------------

    fn compile_program(&mut self, program: &Program) -> Result<(), String> {
        // Pass 0: collect every name declared via CREATE (recursively, into
        // nested CREATE bodies and into SUB / FUNCTION bodies). Used to
        // avoid double-creating components in a later DIM lowering.
        collect_create_names(&program.statements, &mut self.create_declared_names);
        // Pass 0b: collect every component instance name (both CREATE and
        // DIM) — used to detect RHS identifiers that refer to a component.
        rapidr_ast::walk(
            &program.statements,
            &mut |s| match s {
                Statement::Create(c) if is_component_type_name(&c.type_name) => {
                    self.component_kinds.insert(c.name.to_lowercase(), rapidr_ast::canonical_type_name(&c.type_name).to_ascii_uppercase());
                }
                Statement::Dim(d) if is_component_type_name(&d.type_name) => {
                    for v in d.declarators.iter().filter(|v| v.dimensions.is_empty()) {
                        self.component_kinds.insert(v.name.to_lowercase(), rapidr_ast::canonical_type_name(&d.type_name).to_ascii_uppercase());
                    }
                }
                _ => {}
            },
            &mut |_| {},
        );
        collect_component_instance_names(
            &program.statements,
            &mut self.component_instance_names,
        );

        // Pass 1: collect SUB / FUNCTION declarations so forward references work.
        let mut subs: Vec<&SubroutineStatement> = Vec::new();
        let mut funcs: Vec<&FunctionStatement> = Vec::new();
        for stmt in &program.statements {
            match stmt {
                Statement::Subroutine(s) => {
                    let idx = self.reserve_function(&s.name, &s.params, false);
                    self.fn_indices.insert(s.name.clone(), idx);
                    self.fn_is_func.insert(s.name.clone(), false);
                    self.fn_byref.insert(s.name.clone(), s.params.iter().map(|p| p.by_ref).collect());
                    subs.push(s);
                }
                Statement::Function(f) => {
                    let idx = self.reserve_function(&f.name, &f.params, true);
                    self.fn_indices.insert(f.name.clone(), idx);
                    self.fn_is_func.insert(f.name.clone(), true);
                    self.fn_byref.insert(f.name.clone(), f.params.iter().map(|p| p.by_ref).collect());
                    funcs.push(f);
                }
                Statement::Declare(d) if d.lib.is_some() => {
                    self.lib_functions.insert(name_key(&d.name));
                    let lib = d.lib.clone().unwrap_or_default().trim_matches('"').to_ascii_lowercase();
                    self.lib_of.insert(name_key(&d.name), lib);
                }
                _ => {}
            }
        }
        // Pass 2: emit the implicit __main from top-level non-fn statements.
        let main_idx = self.module.add_function(Function {
            name: "__main".into(),
            ..Default::default()
        });
        self.module.entry = main_idx;
                let mut main_code = Vec::new();
        let mut main_lines = Vec::new();
        let saved_scope = std::mem::take(&mut self.scope);
        self.in_main = true;
        self.routine = RoutineLabels { uses_gosub: contains_gosub(&program.statements), ..Default::default() };
        for stmt in &program.statements {
            if matches!(stmt, Statement::Subroutine(_) | Statement::Function(_)) {
                continue;
            }
            self.lower_stmt(stmt, &mut main_code, &mut main_lines)?;
        }
        self.in_main = false;
        emit(&mut main_code, Op::Halt);
        self.resolve_labels(&mut main_code, "the main program");
        let main_locals = self.scope.next_slot as u32;
        let main_local_names = self.scope.display.clone();
        self.scope = saved_scope;
        let f = &mut self.module.functions[main_idx as usize];
        f.code = main_code;
        f.line_info = main_lines;
        f.n_locals = main_locals;
        f.local_names = main_local_names;

        // Pass 3: emit each SUB and FUNCTION body.
        for s in subs {
            let idx = *self.fn_indices.get(&s.name).unwrap();
            self.current_routine = Some(name_key(&s.name));
            self.compile_function_body(idx, &s.name, &format!("SUB {}", s.name), &s.params, &s.body, false)?;
        }
        for f in funcs {
            let idx = *self.fn_indices.get(&f.name).unwrap();
            self.current_routine = Some(name_key(&f.name));
            self.compile_function_body(idx, &f.name, &format!("FUNCTION {}", f.name), &f.params, &f.body, true)?;
        }
        self.current_routine = None;

        // Native-only features in routines the program can't reach (unused
        // parts of RAPIDQ2.INC, windows.inc, …) don't stop it compiling.
        if !self.deferred_errors.is_empty() {
            let reachable = reachable_routines(program);
            let mut deferred: Vec<(String, Vec<String>)> = std::mem::take(&mut self.deferred_errors).into_iter().collect();
            deferred.sort();
            for (routine, errors) in deferred {
                if reachable.contains(&routine) {
                    self.errors.extend(errors);
                }
            }
        }

        if !self.errors.is_empty() {
            return Err(self.errors.join("\n"));
        }
        Ok(())
    }

    /// 1-based (line, column) of a span, when source text was provided.
    fn span_location(&self, span: TextSpan) -> Option<(usize, usize)> {
        let starts = self.line_starts.as_ref()?;
        let line = match starts.binary_search(&span.start) {
            Ok(idx) => idx + 1,
            Err(idx) => idx,
        };
        let col = span.start - starts.get(line.checked_sub(1)?)? + 1;
        Some((line, col))
    }

    fn error_at(&mut self, span: TextSpan, message: String) {
        let formatted = match self.span_location(span) {
            Some((line, col)) => format!("{line}:{col}: error: {message}"),
            None => format!("error: {message}"),
        };
        // Features the interpreter lacks (native-only or not supported yet),
        // and names it doesn't know, only count in code the program can
        // reach: include libraries are full of routines a program never
        // calls (RAPIDQ2.INC, windows.inc).
        let in_library = self
            .span_location(span)
            .is_some_and(|(line, _)| self.library_lines.get(line - 1).copied().unwrap_or(false));
        if message.contains(NATIVE_ONLY_MARKER)
            || message.contains(WINDOWS_ONLY_MARKER)
            || (in_library && (message.contains(UNSUPPORTED_MARKER) || message.starts_with("Unknown SUB or FUNCTION")))
        {
            if let Some(routine) = self.current_routine.clone() {
                self.deferred_errors.entry(routine).or_default().push(formatted);
                return;
            }
        }
        self.errors.push(formatted);
    }

    /// A call to `name` that is not a user SUB/FUNCTION compiles to
    /// `CallBuiltin`; make sure the builtin exists instead of letting the
    /// call silently do nothing at run time.
    fn check_builtin_call(&mut self, name: &str, argc: usize, span: TextSpan) {
        if builtins::is_builtin(name) {
            return;
        }
        let key = name_key(name);
        let windows_lib = self.lib_of.get(&key).filter(|lib| is_windows_system_library(lib)).cloned();
        let message = if let Some(lib) = windows_lib {
            let api = key.trim_end_matches(['a', 'w']).to_string();
            let hint = windows_api_hint(&key).or_else(|| windows_api_hint(&api));
            format!(
                "'{name}' is a Windows API function (LIB \"{lib}\"){WINDOWS_ONLY_MARKER}{}",
                hint.map(|h| format!(". Instead, {h}")).unwrap_or_else(|| ", and this call has no portable equivalent yet".to_string())
            )
        } else if self.lib_functions.contains(&key) {
            format!(
                "'{name}' is an external DLL function (DECLARE ... LIB); the bytecode interpreter can't call DLLs yet, build natively with `rapidr build`"
            )
        } else if key == "varptr" {
            format!("{} (memory addresses) only works in native builds (`rapidr build`), not in the bytecode interpreter", name.to_uppercase())
        } else if matches!(key.as_str(), "memcpy" | "memset" | "memcmp" | "peek" | "poke") {
            format!("{} (raw memory access){UNSUPPORTED_MARKER}", name.to_uppercase())
        } else if key == "inc" || key == "dec" {
            format!("{} needs a variable: `{} x` or `{} x, amount`", name.to_uppercase(), name.to_uppercase(), name.to_uppercase())
        } else if RAPIDQ_BUILTINS.contains(&key.as_str()) {
            format!("{} (a RapidQ built-in){UNSUPPORTED_MARKER}", name.to_uppercase())
        } else if argc == 0 {
            format!("Unknown SUB or FUNCTION '{name}'")
        } else {
            format!("Unknown SUB or FUNCTION '{name}'")
        };
        self.error_at(span, message);
    }

    /// String-pool index for a global variable's name, always using the
    /// first spelling seen so `Total`, `total` and `TOTAL` are one variable.
    fn global_str(&mut self, name: &str) -> u32 {
        if let Some(mangled) = self.scope.statics.get(name) {
            let mangled = mangled.clone();
            return self.module.add_string(&mangled);
        }
        let spelling = self
            .global_spelling
            .entry(name_key(name))
            .or_insert_with(|| name.to_string())
            .clone();
        self.module.add_string(&spelling)
    }

    /// Reserve a function entry up-front so its index is known before its
    /// body is lowered (forward references / recursion).
    fn reserve_function(&mut self, name: &str, params: &[Parameter], _is_func: bool) -> u32 {
        let mut f = Function::default();
        f.name = name.to_string();
        f.params = params.iter().map(|p| Param { name: p.name.clone(), by_ref: p.by_ref }).collect();
        self.module.add_function(f)
    }

    /// `name` is the routine's own name (a FUNCTION's result variable),
    /// `label` how errors refer to it ("SUB Foo", "TForm.Reset").
    fn compile_function_body(
        &mut self,
        idx: u32,
        name: &str,
        label: &str,
        params: &[Parameter],
        body: &[Statement],
        is_func: bool,
    ) -> Result<(), String> {
        let saved_scope = std::mem::take(&mut self.scope);
        self.scope.owner = label.to_string();
        // Pre-declare parameter slots (slots 0..N).
        for p in params {
            self.scope.declare(&p.name);
            if is_component_type_name(&p.type_name) {
                self.scope.types.insert(p.name.clone(), p.type_name.clone());
            }
        }
        // A FUNCTION's own name is a local holding its result; RapidQ's
        // `RESULT = value` sets the same slot.
        let result_slot = is_func.then(|| self.scope.declare(name));
        if let Some(slot) = result_slot {
            if !self.scope.locals.contains_key("Result") {
                self.scope.locals.insert("Result".to_string(), slot);
            }
        }
        let saved_ctx = self.fn_ctx.replace(FnCtx { result_slot });
        let saved_loops = std::mem::take(&mut self.loop_stack);
        self.routine = RoutineLabels { uses_gosub: contains_gosub(body), ..Default::default() };
        let mut code = Vec::new();
        let mut lines = Vec::new();
        for stmt in body {
            self.lower_stmt(stmt, &mut code, &mut lines)?;
        }
        // Implicit return at END SUB / END FUNCTION (needs this fn's ctx).
        self.emit_return_from_routine(&mut code);
        self.resolve_labels(&mut code, label);
        self.loop_stack = saved_loops;
        self.fn_ctx = saved_ctx;
        let n_locals = self.scope.next_slot as u32;
        let local_names = self.scope.display.clone();
        self.scope = saved_scope;
        let f = &mut self.module.functions[idx as usize];
        f.code = code;
        f.line_info = lines;
        f.n_locals = n_locals;
        f.local_names = local_names;
        Ok(())
    }

    // ------------------- statements -------------------

    /// Lowers one statement. A failure is recorded as an error at that
    /// statement's position (the innermost one, since nested bodies go
    /// through here too) and compilation continues, so every problem in the
    /// program is reported with its line.
    fn lower_stmt(
        &mut self,
        stmt: &Statement,
        code: &mut Vec<u8>,
        lines: &mut Vec<(u32, u32)>,
    ) -> Result<(), String> {
        if let Err(message) = self.lower_stmt_inner(stmt, code, lines) {
            self.error_at(stmt_span(stmt), message);
        }
        Ok(())
    }

    fn lower_stmt_inner(
        &mut self,
        stmt: &Statement,
        code: &mut Vec<u8>,
        lines: &mut Vec<(u32, u32)>,
    ) -> Result<(), String> {
        let line = self.stmt_line(stmt);
        let off = code.len() as u32;
        if line > 0 {
            lines.push((off, line));
        }
        match stmt {
            Statement::Print(p) => self.lower_print(p, code)?,
            Statement::Assignment(a) => self.lower_assignment(a, code)?,
            Statement::Call(c) => self.lower_call_stmt(c, code)?,
            Statement::If(i) => self.lower_if(i, code, lines)?,
            Statement::For(f) => self.lower_for(f, code, lines)?,
            Statement::While(w) => self.lower_while(w, code, lines)?,
            Statement::DoLoop(d) => self.lower_do(d, code, lines)?,
            Statement::Return(r) => self.lower_return(r, code)?,
            Statement::Const(c) => {
                // CONST x = expr  → eval + StoreGlobal x  (treat all as globals).
                self.globals.insert(name_key(&c.name));
                self.lower_expr(&c.value, code)?;
                let s = self.global_str(&c.name);
                emit(code, Op::StoreGlobal);
                push_u32(code, s);
            }
            Statement::Dim(d) if d.is_static && !self.in_main => self.lower_static(d, code)?,
            Statement::Dim(d) => {
                // Declare locals; initial value Null is already the default.
                for decl in &d.declarators {
                    // REDIM in a SUB resizes the module-level array unless
                    // the SUB has its own (QBasic / RapidQ).
                    let resizes_global = d.is_redim && self.scope.get(&decl.name).is_none() && self.is_known_global(&decl.name);
                    if !self.in_main {
                        if !resizes_global {
                            self.scope.declare(&decl.name);
                        }
                    } else {
                        self.globals.insert(name_key(&decl.name));
                    }
                    if d.is_redim && !decl.dimensions.is_empty() {
                        self.lower_redim(decl, &d.type_name, code)?;
                    } else if !decl.dimensions.is_empty() && is_component_type_name(&d.type_name) {
                        self.lower_component_array(decl, &d.type_name, code)?;
                    } else if !decl.dimensions.is_empty() && !is_component_type_name(&d.type_name) {
                        self.lower_array_dim(decl, &d.type_name, code)?;
                    } else if !is_component_type_name(&d.type_name) {
                        // `DIM n AS INTEGER` starts at 0, a STRING at "".
                        let default = self.module.add_const(type_default(&d.type_name));
                        emit(code, Op::LoadConst); push_u32(code, default);
                        if let Some(slot) = self.scope.get(&decl.name) {
                            emit(code, Op::StoreLocal); push_u16(code, slot);
                        } else {
                            let s = self.global_str(&decl.name);
                            emit(code, Op::StoreGlobal); push_u32(code, s);
                        }
                    }
                    // Component DIM → eagerly CreateComp (mirrors the
                    // compiled-mode `emit_dim` path), unless a CREATE block
                    // already declares the same name.
                    if is_component_type_name(&d.type_name) && decl.dimensions.is_empty() {
                        let lower = decl.name.to_lowercase();
                        if !self.create_declared_names.contains(&lower) {
                            let kind_s = self.module.add_string(&d.type_name.to_uppercase());
                            let id_s = self.module.add_string(&decl.name);
                            emit(code, Op::CreateComp);
                            push_u32(code, kind_s); push_u32(code, id_s);
                            emit(code, Op::Pop);
                            if d.type_name.eq_ignore_ascii_case("RTIMER") {
                                self.emit_register_timer(&decl.name, code);
                            }
                        }
                    }
                }
            }
            Statement::Create(c) => self.lower_create(c, code, lines)?,
            Statement::Bind(b) => self.lower_bind(b, code)?,
            Statement::With(w) => {
                // `.Member` inside the block is a member of the WITH object.
                for s in &rapidr_ast::resolve_with_body(&w.body, &w.object) {
                    self.lower_stmt(s, code, lines)?;
                }
            }
            Statement::Subroutine(_) | Statement::Function(_) => {
                // Already collected in pass 1.
            }
            Statement::Comment(_) | Statement::Directive(_) | Statement::Line(_)
            | Statement::Import(_) => {
                // No runtime effect.
            }
            Statement::Input(i) => {
                // RapidQ: print the prompt as written, read a whole line, and
                // store it as text or a number for the variable
                // (rapidr_value::input_value).
                if let Some(prompt) = &i.prompt {
                    self.lower_expr(prompt, code)?;
                    emit(code, Op::Print);
                }
                emit(code, Op::Input);
                self.lower_expr(&i.target, code)?;
                let suffix = rapidr_ast::input_suffix(&i.target);
                let c = self.module.add_const(Const::Str(suffix.into()));
                emit(code, Op::LoadConst); push_u32(code, c);
                let name = self.module.add_string("__input_value");
                emit(code, Op::CallBuiltin); push_u32(code, name); code.push(3);
                self.store_target(&i.target, code)?;
            }
            Statement::Exit(e) => {
                let kind = e.exit_type.to_uppercase();
                match kind.as_str() {
                    "FOR" | "WHILE" | "DO" => {
                        let target = self.loop_stack.iter().rposition(|l| l.kind == kind);
                        match target {
                            Some(i) => {
                                emit(code, Op::Jump);
                                self.loop_stack[i].breaks.push(code.len());
                                push_u32(code, 0); // patched at that loop's end
                            }
                            None => self.error_at(e.span, format!("EXIT {kind} is not inside a {kind} loop")),
                        }
                    }
                    "SUB" | "FUNCTION" => {
                        let in_function = self.fn_ctx.map(|c| c.result_slot.is_some());
                        match (in_function, kind.as_str()) {
                            (Some(false), "SUB") | (Some(true), "FUNCTION") => self.emit_return_from_routine(code),
                            _ => self.error_at(e.span, format!("EXIT {kind} is not inside a {kind}")),
                        }
                    }
                    _ => self.error_at(e.span, format!("EXIT {kind} is not supported (use EXIT FOR, WHILE, DO, SUB or FUNCTION)")),
                }
            }
            Statement::SelectCase(s) => {
                // The SELECT expression is evaluated once into a temp. Each
                // CASE is a chain of tests: any match jumps to its body,
                // otherwise control falls to the next CASE, then CASE ELSE.
                let tmp = self.scope.declare(&format!("__sel_{}", code.len()));
                self.lower_expr(&s.expression, code)?;
                emit(code, Op::StoreLocal); push_u16(code, tmp);
                let mut end_jumps: Vec<usize> = Vec::new();
                for case in &s.cases {
                    let mut to_body: Vec<usize> = Vec::new();
                    for value in &case.values {
                        self.lower_case_test(tmp, value, code)?;
                        emit(code, Op::JumpIf);
                        to_body.push(code.len());
                        push_u32(code, 0);
                    }
                    emit(code, Op::Jump);
                    let to_next_case = code.len();
                    push_u32(code, 0);
                    let body_start = code.len() as u32;
                    for j in to_body {
                        patch_u32(code, j, body_start);
                    }
                    for stmt in &case.body {
                        self.lower_stmt(stmt, code, lines)?;
                    }
                    emit(code, Op::Jump);
                    end_jumps.push(code.len());
                    push_u32(code, 0);
                    let next_case = code.len() as u32;
                    patch_u32(code, to_next_case, next_case);
                }
                for stmt in &s.case_else {
                    self.lower_stmt(stmt, code, lines)?;
                }
                let end = code.len() as u32;
                for j in end_jumps {
                    patch_u32(code, j, end);
                }
            }
            // File I/O by file number → the hosts' file builtins (same
            // argument order as the Rust codegen's rp_open/rp_print_hash/...).
            Statement::Open(o) => {
                self.lower_expr(&o.filename, code)?;
                let mode = self.module.add_const(Const::Str(o.mode.clone()));
                emit(code, Op::LoadConst); push_u32(code, mode);
                self.lower_expr(&o.file_number, code)?;
                self.emit_builtin_stmt("open", 3, code);
            }
            Statement::Close(c) => {
                self.lower_expr(&c.file_number, code)?;
                self.emit_builtin_stmt("close", 1, code);
            }
            Statement::PrintHash(p) => {
                self.lower_expr(&p.file_number, code)?;
                for item in &p.items { self.lower_expr(item, code)?; }
                self.emit_builtin_stmt("print_hash", 1 + p.items.len() as u8, code);
            }
            Statement::WriteHash(w) => {
                self.lower_expr(&w.file_number, code)?;
                for item in &w.items { self.lower_expr(item, code)?; }
                self.emit_builtin_stmt("write_hash", 1 + w.items.len() as u8, code);
            }
            Statement::Seek(sk) => {
                self.lower_expr(&sk.file_number, code)?;
                self.lower_expr(&sk.position, code)?;
                self.emit_builtin_stmt("seek", 2, code);
            }
            // `Name:` is a label unless Name is a SUB or builtin, in which
            // case it's a call followed by `:` (e.g. `DoEvents: x = 1`).
            Statement::Label(l) => {
                if self.fn_indices.contains_key(&l.name) || builtins::is_builtin(&l.name) {
                    let call = CallStatement {
                        span: l.span,
                        callee: Expression::Identifier(rapidr_ast::Identifier { span: l.span, name: l.name.clone() }),
                        args: Vec::new(),
                    };
                    self.lower_call_stmt(&call, code)?;
                } else if self.routine.offsets.insert(name_key(&l.name), code.len() as u32).is_some() {
                    self.error_at(l.span, format!("Label '{}' is defined more than once", l.name));
                }
            }
            Statement::Goto(j) | Statement::Gosub(j) => {
                emit(code, if matches!(stmt, Statement::Gosub(_)) { Op::Gosub } else { Op::Jump });
                self.routine.pending.push((j.label.clone(), code.len(), j.span));
                push_u32(code, 0);
            }
            // `DECLARE SUB Foo(...)` is only a forward declaration: nothing
            // to emit. DLL imports are reported where they are called.
            Statement::Declare(_) => {}
            // TYPE fields need no code (members are dynamic); methods do.
            // Collected in pass 1; methods/events are compiled in pass 3 and
            // instances are set up where they are DIMmed.
            Statement::Type(_) => {}
            Statement::RustBlock(r) => {
                self.error_at(
                    r.span,
                    "RUSTSTART ... RUSTEND blocks only work in native builds (`rapidr build`), not in the bytecode interpreter or the web IDE".into(),
                );
            }
            // No catch-all arm: every statement kind is handled explicitly
            // above, so adding a new one to the AST is a compile error here
            // rather than a statement the interpreter silently skips.
        }
        Ok(())
    }

    /// `STATIC n AS LONG` in a SUB/FUNCTION: the name refers to a hidden
    /// global (`Routine::n`) from here on, initialised the first time the
    /// statement runs, so the value survives between calls and is shared by
    /// recursive calls (RapidQ manual, STATIC).
    fn lower_static(&mut self, d: &rapidr_ast::DimStatement, code: &mut Vec<u8>) -> Result<(), String> {
        for decl in &d.declarators {
            let mangled = format!("{}::{}", self.scope.owner, decl.name);
            self.scope.statics.insert(decl.name.clone(), mangled.clone());
            self.globals.insert(name_key(&mangled));
            let flag = self.module.add_string(&format!("{mangled}#init"));
            emit(code, Op::LoadGlobal); push_u32(code, flag);
            emit(code, Op::JumpIf);
            let skip = code.len();
            push_u32(code, 0);
            if decl.dimensions.is_empty() {
                let default = self.module.add_const(type_default(&d.type_name));
                emit(code, Op::LoadConst); push_u32(code, default);
                let s = self.global_str(&decl.name);
                emit(code, Op::StoreGlobal); push_u32(code, s);
            } else {
                self.lower_array_dim(decl, &d.type_name, code)?;
            }
            let yes = self.module.add_const(Const::Bool(true));
            emit(code, Op::LoadConst); push_u32(code, yes);
            emit(code, Op::StoreGlobal); push_u32(code, flag);
            let here = code.len() as u32;
            patch_u32(code, skip, here);
        }
        Ok(())
    }

    /// Whether `name` is a module-level variable (or a STATIC of this routine).
    fn is_known_global(&self, name: &str) -> bool {
        self.globals.contains(&name_key(name)) || self.scope.statics.contains_key(name)
    }

    /// `REDIM a(20) AS T`: `a = __redim(a, fill, lo1, hi1, …)`, which resizes
    /// an existing array in place keeping its data (rapidr_value::redim).
    fn lower_redim(&mut self, decl: &VariableDeclarator, type_name: &str, code: &mut Vec<u8>) -> Result<(), String> {
        let var = Expression::Identifier(rapidr_ast::Identifier { span: decl.span, name: decl.name.clone() });
        self.lower_expr(&var, code)?;
        let fill = self.module.add_const(type_default(type_name));
        emit(code, Op::LoadConst); push_u32(code, fill);
        let zero = self.module.add_const(Const::Int(0));
        for dim in &decl.dimensions {
            match dim {
                ArrayDimension::Single(upper) => {
                    emit(code, Op::LoadConst); push_u32(code, zero);
                    self.lower_expr(upper, code)?;
                }
                ArrayDimension::Range { start, end } => {
                    self.lower_expr(start, code)?;
                    self.lower_expr(end, code)?;
                }
            }
        }
        let s = self.module.add_string("__redim");
        emit(code, Op::CallBuiltin); push_u32(code, s); code.push((2 + 2 * decl.dimensions.len()) as u8);
        self.store_target(&var, code)
    }

    /// `DIM a(10)`, `DIM b(1 TO 5, 3) AS STRING`: allocate the array (each
    /// element set to the type's default) and store it in the variable.
    /// `DIM lbl(1 TO 3) AS QLABEL`: one component per element (ids `lbl(1)`,
    /// …, created by the host), the variable holding the array of ids.
    fn lower_component_array(&mut self, decl: &VariableDeclarator, type_name: &str, code: &mut Vec<u8>) -> Result<(), String> {
        let kind = rapidr_ast::canonical_type_name(type_name).to_ascii_uppercase();
        let k = self.module.add_const(Const::Str(kind.clone()));
        emit(code, Op::LoadConst); push_u32(code, k);
        let n = self.module.add_const(Const::Str(decl.name.clone()));
        emit(code, Op::LoadConst); push_u32(code, n);
        let zero = self.module.add_const(Const::Int(0));
        for dim in &decl.dimensions {
            match dim {
                ArrayDimension::Single(upper) => {
                    emit(code, Op::LoadConst); push_u32(code, zero);
                    self.lower_expr(upper, code)?;
                }
                ArrayDimension::Range { start, end } => {
                    self.lower_expr(start, code)?;
                    self.lower_expr(end, code)?;
                }
            }
        }
        let b = self.module.add_string("__component_array");
        emit(code, Op::CallBuiltin); push_u32(code, b); code.push(2 + 2 * decl.dimensions.len() as u8);
        if let Some(slot) = self.scope.get(&decl.name).filter(|_| !self.in_main) {
            emit(code, Op::StoreLocal); push_u16(code, slot);
            self.scope.object_arrays.insert(decl.name.clone(), kind);
        } else {
            let s = self.global_str(&decl.name);
            emit(code, Op::StoreGlobal); push_u32(code, s);
            self.global_object_arrays.insert(decl.name.clone(), kind);
        }
        Ok(())
    }

    /// Element type of an array of objects named `name`, if it is one.
    fn object_array_type(&self, name: &str) -> Option<String> {
        if !self.in_main && self.scope.get(name).is_some() {
            return self.scope.object_arrays.get(name).cloned();
        }
        self.global_object_arrays.get(name).cloned()
    }

    fn lower_array_dim(&mut self, decl: &VariableDeclarator, type_name: &str, code: &mut Vec<u8>) -> Result<(), String> {
        let fill = self.module.add_const(type_default(type_name));
        emit(code, Op::LoadConst); push_u32(code, fill);
        let zero = self.module.add_const(Const::Int(0));
        for dim in &decl.dimensions {
            match dim {
                ArrayDimension::Single(upper) => {
                    emit(code, Op::LoadConst); push_u32(code, zero);
                    self.lower_expr(upper, code)?;
                }
                ArrayDimension::Range { start, end } => {
                    self.lower_expr(start, code)?;
                    self.lower_expr(end, code)?;
                }
            }
        }
        emit(code, Op::NewArray); code.push(decl.dimensions.len() as u8);
        if let Some(slot) = self.scope.get(&decl.name) {
            emit(code, Op::StoreLocal); push_u16(code, slot);
        } else {
            let s = self.global_str(&decl.name);
            emit(code, Op::StoreGlobal); push_u32(code, s);
        }
        Ok(())
    }

    /// Pushes whether the SELECT value in local `tmp` matches one CASE item.
    fn lower_case_test(&mut self, tmp: u16, value: &CaseValue, code: &mut Vec<u8>) -> Result<(), String> {
        match value {
            CaseValue::Value(e) => {
                emit(code, Op::LoadLocal); push_u16(code, tmp);
                self.lower_expr(e, code)?;
                emit(code, Op::Eq);
            }
            CaseValue::Range(low, high) => {
                emit(code, Op::LoadLocal); push_u16(code, tmp);
                self.lower_expr(low, code)?;
                emit(code, Op::Ge);
                emit(code, Op::LoadLocal); push_u16(code, tmp);
                self.lower_expr(high, code)?;
                emit(code, Op::Le);
                emit(code, Op::And);
            }
            CaseValue::Is(op, e) => {
                emit(code, Op::LoadLocal); push_u16(code, tmp);
                self.lower_expr(e, code)?;
                emit(code, match op {
                    BinaryOperator::Equal => Op::Eq,
                    BinaryOperator::NotEqual => Op::Ne,
                    BinaryOperator::LessThan => Op::Lt,
                    BinaryOperator::LessThanOrEqual => Op::Le,
                    BinaryOperator::GreaterThan => Op::Gt,
                    _ => Op::Ge,
                });
            }
        }
        Ok(())
    }

    // ------------------- objects (components, TYPE instances) -------------------

    /// `Obj.Name` naming a SUB/FUNCTION defined as `SUB Obj.Name` (when
    /// `Obj` isn't a variable), as its full name.
    fn dotted_routine(&self, callee: &Expression) -> Option<String> {
        let Expression::MemberAccess(m) = callee else { return None };
        let Expression::Identifier(o) = m.object.as_ref() else { return None };
        if self.scope.get(&o.name).is_some() || self.var_type(&o.name).is_some() {
            return None;
        }
        let full = format!("{}.{}", o.name, m.member);
        self.fn_indices.contains_key(&full).then_some(full)
    }

    /// The component type an expression holds, when it holds one: a
    /// component parameter (`Sender AS QBUTTON`), a CREATE/DIM component, or
    /// an element of an array of components. `None` otherwise. (User TYPEs
    /// never reach this compiler: `rapidr_ast::objects::lower` turns them
    /// into plain routines and builtins first.)
    fn object_type_of(&self, e: &Expression) -> Option<String> {
        match e {
            Expression::Identifier(id) => self.var_type(&id.name).or_else(|| {
                // A CREATE/DIM component (`SB` in `SB.Panel(0).Width`).
                if self.scope.get(&id.name).is_some() && !self.in_main {
                    return None;
                }
                self.component_kinds.get(&id.name.to_lowercase()).cloned()
            }),
            // `lbl(i)`: an element of an array of components.
            Expression::FunctionCall(fc) => match fc.callee.as_ref() {
                Expression::Identifier(id) => self.object_array_type(&id.name),
                _ => None,
            },
            Expression::ArrayAccess(a) => match a.array.as_ref() {
                Expression::Identifier(id) => self.object_array_type(&id.name),
                _ => None,
            },
            _ => None,
        }
    }

    /// `obj.Canvas.Font.Size`: when `e` is `<component>.<sub>`, `<sub>` is
    /// one of its property objects (Font, …), addressed like native builds'
    /// combined names (`font.size`). Returns the object expression and `sub`.
    fn sub_property<'e>(&self, e: &'e Expression) -> Option<(&'e Expression, &'e str)> {
        let Expression::MemberAccess(m) = e else { return None };
        self.object_type_of(&m.object)?;
        Some((&m.object, m.member.as_str()))
    }

    /// `obj.item(i)` where `obj` is a component: one of its indexed
    /// sub-objects (a ListView's items/columns, …). Returns the object,
    /// `item` and the index arguments. Its members become methods with
    /// combined names: `obj.item(i).caption` → `item.caption(i)`; assigning
    /// calls `item.caption=(i, value)`.
    fn indexed_sub_object<'e>(&self, e: &'e Expression) -> Option<(&'e Expression, &'e str, &'e [Expression])> {
        let Expression::FunctionCall(fc) = e else { return None };
        let Expression::MemberAccess(m) = fc.callee.as_ref() else { return None };
        self.object_type_of(&m.object)?;
        Some((&m.object, m.member.as_str(), fc.args.as_slice()))
    }

    /// Declared component type of a parameter (`Sender AS QBUTTON`).
    fn var_type(&self, name: &str) -> Option<String> {
        self.scope.types.get(name).cloned()
    }

    /// `x.Prop` addresses the object whose id is *stored in* `x` when `x` is
    /// a parameter or local of the current SUB/FUNCTION (`Sender`);
    /// globals and component names are addressed by name.
    fn is_dynamic_object(&self, name: &str) -> bool {
        !self.in_main && self.scope.get(name).is_some() && !self.component_instance_names.contains_key(&name.to_lowercase())
    }

    /// `obj.Method(args)` where obj is a TYPE instance (user method), or a
    /// parameter/local holding a component (dynamic method call); also a bare
    /// `Method args` inside the TYPE's own code. Returns false if `callee`
    /// is none of these. `want_value`: leave exactly one value on the stack.
    fn try_lower_object_call(&mut self, callee: &Expression, args: &[Expression], want_value: bool, code: &mut Vec<u8>) -> Result<bool, String> {
        // `Screen.MousePresent`: a routine defined with a dotted name
        // (`FUNCTION Screen.MousePresent`), not a method of an object.
        if let Some(full) = self.dotted_routine(callee) {
            let fi = *self.fn_indices.get(&full).unwrap();
            let is_func = self.fn_is_func.get(&full).copied().unwrap_or(false);
            for a in args {
                self.lower_expr(a, code)?;
            }
            emit(code, if is_func { Op::CallFunc } else { Op::CallSub });
            push_u32(code, fi); code.push(args.len() as u8);
            match (want_value, is_func) {
                (true, false) => emit(code, Op::LoadNull),
                (false, true) => emit(code, Op::Pop),
                _ => {}
            }
            return Ok(true);
        }
        // `obj.item(i).Delete(…)` → CallMethodDyn(obj, "item.delete", i, …)
        if let Expression::MemberAccess(m) = callee {
            if let Some((object, sub, index)) = self.indexed_sub_object(&m.object) {
                let (object, index) = (object.clone(), index.to_vec());
                let combo = format!("{}.{}", sub.to_lowercase(), m.member.to_lowercase());
                self.lower_expr(&object, code)?;
                for a in index.iter().chain(args) {
                    self.lower_expr(a, code)?;
                }
                let m_s = self.module.add_string(&combo);
                emit(code, Op::CallMethodDyn); push_u32(code, m_s); code.push((index.len() + args.len()) as u8);
                if !want_value {
                    emit(code, Op::Pop);
                }
                return Ok(true);
            }
        }
        // `obj.Canvas.Font.AddStyles(…)`: a method of a component's Font.
        if let Expression::MemberAccess(m) = callee {
            if let Some((object, sub)) = self.sub_property(&m.object) {
                let object = object.clone();
                let combo = format!("{}.{}", sub.to_lowercase(), m.member.to_lowercase());
                self.lower_expr(&object, code)?;
                for a in args {
                    self.lower_expr(a, code)?;
                }
                let m_s = self.module.add_string(&combo);
                emit(code, Op::CallMethodDyn); push_u32(code, m_s); code.push(args.len() as u8);
                if !want_value {
                    emit(code, Op::Pop);
                }
                return Ok(true);
            }
        }
        // `lbl(i).Show`: a method of an element of an array of components.
        if let Expression::MemberAccess(m) = callee {
            if !matches!(m.object.as_ref(), Expression::Identifier(_)) && self.object_type_of(&m.object).is_some() {
                self.lower_expr(&m.object, code)?;
                for a in args {
                    self.lower_expr(a, code)?;
                }
                let m_s = self.module.add_string(&m.member);
                emit(code, Op::CallMethodDyn); push_u32(code, m_s); code.push(args.len() as u8);
                if !want_value {
                    emit(code, Op::Pop);
                }
                return Ok(true);
            }
        }
        let Expression::MemberAccess(m) = callee else { return Ok(false) };
        let Expression::Identifier(o) = m.object.as_ref() else { return Ok(false) };
        let (object, method) = (Some(o.name.clone()), m.member.clone());
        if let Some(o) = object.filter(|o| self.is_dynamic_object(o)) {
            self.lower_expr(&Expression::Identifier(rapidr_ast::Identifier { span: TextSpan::default(), name: o }), code)?;
            for a in args {
                self.lower_expr(a, code)?;
            }
            let m_s = self.module.add_string(&method);
            emit(code, Op::CallMethodDyn); push_u32(code, m_s); code.push(args.len() as u8);
            if !want_value {
                emit(code, Op::Pop);
            }
            return Ok(true);
        }
        Ok(false)
    }

    /// `CallBuiltin(name, argc)` as a statement (result discarded).
    fn emit_builtin_stmt(&mut self, name: &str, argc: u8, code: &mut Vec<u8>) {
        let s = self.module.add_string(name);
        emit(code, Op::CallBuiltin);
        push_u32(code, s); code.push(argc);
        emit(code, Op::Pop);
    }

    fn lower_print(&mut self, p: &PrintStatement, code: &mut Vec<u8>) -> Result<(), String> {
        if p.items.is_empty() {
            if p.append_newline {
                // Push "" then PrintLn.
                let c = self.module.add_const(Const::Str(String::new()));
                emit(code, Op::LoadConst); push_u32(code, c);
                emit(code, Op::PrintLn);
            }
            return Ok(());
        }
        let n = p.items.len();
        for (i, item) in p.items.iter().enumerate() {
            self.lower_expr(item, code)?;
            let last = i + 1 == n;
            if last && p.append_newline {
                emit(code, Op::PrintLn);
            } else {
                emit(code, Op::Print);
            }
            if p.zones.get(i).copied().unwrap_or(false) {
                emit(code, Op::PrintZone);
            }
        }
        Ok(())
    }

    fn lower_assignment(&mut self, a: &AssignmentStatement, code: &mut Vec<u8>) -> Result<(), String> {
        // Inside a CREATE block `Caption = …` sets a property, not a global.
        if self.in_main && self.create_stack.is_empty() {
            if let Expression::Identifier(id) = &a.target {
                self.globals.insert(name_key(&id.name));
            }
        }
        // Inside CREATE, `Cell(1, 0) = s` / `ColWidths(0) = w` set an indexed
        // property of the object being created: its method with the value
        // as an extra last argument (as `Grid.Cell(1, 0) = s` does).
        if let Some(inst) = self.create_stack.last().cloned() {
            let indexed = match &a.target {
                Expression::FunctionCall(fc) if !fc.args.is_empty() => Some((fc.callee.as_ref(), &fc.args)),
                Expression::ArrayAccess(aa) => Some((aa.array.as_ref(), &aa.indices)),
                _ => None,
            };
            if let Some((Expression::Identifier(id), indices)) = indexed {
                if self.scope.get(&id.name).is_none() && !self.is_known_global(&id.name) {
                    for i in indices {
                        self.lower_expr(i, code)?;
                    }
                    self.lower_expr(&a.value, code)?;
                    let id_s = self.module.add_string(&inst);
                    let mn_s = self.module.add_string(&id.name.to_lowercase());
                    emit(code, Op::CallMethod);
                    push_u32(code, id_s); push_u32(code, mn_s); code.push(indices.len() as u8 + 1);
                    emit(code, Op::Pop);
                    return Ok(());
                }
            }
        }
        // Special case for CREATE-block property assignment with a SUB-name RHS:
        // → emit RegisterEvent instead of SetProp.
        if let (Some(inst), Expression::Identifier(rhs_id)) =
            (self.create_stack.last().cloned(), &a.value)
        {
            if let Expression::Identifier(lhs_id) = &a.target {
                if let Some(&fi) = self.fn_indices.get(&rhs_id.name) {
                    let id_s = self.module.add_string(&inst);
                    let ev_s = self.module.add_string(&lhs_id.name);
                    emit(code, Op::RegisterEvent);
                    push_u32(code, id_s); push_u32(code, ev_s); push_u32(code, fi);
                    return Ok(());
                }
            }
        }
        // `lbl(i).OnClick = Handler`: bind the handler to an object known
        // only at run time (an element of an array of components, …).
        if let (Expression::MemberAccess(m), Expression::Identifier(rhs_id)) = (&a.target, &a.value) {
            if m.member.to_ascii_lowercase().starts_with("on")
                && !matches!(m.object.as_ref(), Expression::Identifier(_))
                && self.object_type_of(&m.object).is_some()
            {
                if let Some(&fi) = self.fn_indices.get(&rhs_id.name) {
                    self.lower_expr(&m.object, code)?;
                    let ev = self.module.add_const(Const::Str(m.member.clone()));
                    emit(code, Op::LoadConst); push_u32(code, ev);
                    let ptr = self.module.add_const(Const::Int(fi as i64 + 1));
                    emit(code, Op::LoadConst); push_u32(code, ptr);
                    let b = self.module.add_string("__bind_event");
                    emit(code, Op::CallBuiltin); push_u32(code, b); code.push(3);
                    emit(code, Op::Pop);
                    return Ok(());
                }
            }
        }
        // Top-level (outside CREATE) `Obj.OnEvent = Handler` — emit
        // RegisterEvent so DOM/FLTK callbacks reach the bytecode SUB.
        if let (Expression::MemberAccess(m), Expression::Identifier(rhs_id)) =
            (&a.target, &a.value)
        {
            if let Expression::Identifier(obj) = &*m.object {
                if m.member.to_lowercase().starts_with("on") {
                    if let Some(&fi) = self.fn_indices.get(&rhs_id.name) {
                        let id_s = self.module.add_string(&obj.name);
                        let ev_s = self.module.add_string(&m.member);
                        emit(code, Op::RegisterEvent);
                        push_u32(code, id_s); push_u32(code, ev_s); push_u32(code, fi);
                        return Ok(());
                    }
                }
            }
        }
        // Top-level `Comp.Prop = OtherComp` (e.g. `Label1.Parent = Form1`):
        // RHS is an identifier referring to a component instance. The
        // component name was never stored as a global (the CreateComp result
        // was popped), so a normal LoadGlobal would push v_null. Mirror the
        // codegen-rust behaviour by lowering it to a string literal of the
        // component id, then SetProp.
        if let (Expression::MemberAccess(m), Expression::Identifier(rhs_id)) =
            (&a.target, &a.value)
        {
            if let Expression::Identifier(obj) = &*m.object {
                if self.component_instance_names.contains_key(&rhs_id.name.to_lowercase()) {
                    let cs = self.module.add_const(Const::Str(rhs_id.name.clone()));
                    emit(code, Op::LoadConst);
                    push_u32(code, cs);
                    let id_s = self.module.add_string(&obj.name);
                    let nm_s = self.module.add_string(&m.member);
                    emit(code, Op::SetProp);
                    push_u32(code, id_s); push_u32(code, nm_s);
                    return Ok(());
                }
            }
        }
        // Top-level nested member-access assignment:
        // `Form1.Font.Size = 12` → SetProp(form1, "font.size", 12).
        // Mirrors codegen-rust's `comp.Sub.Prop = value` path.
        if let Expression::MemberAccess(m) = &a.target {
            if let (Expression::MemberAccess(inner), None) = (&*m.object, self.object_type_of(&m.object)) {
                if let Expression::Identifier(obj) = &*inner.object {
                    self.lower_expr(&a.value, code)?;
                    let id_s = self.module.add_string(&obj.name);
                    let combo = format!(
                        "{}.{}",
                        inner.member.to_lowercase(),
                        m.member.to_lowercase()
                    );
                    let nm_s = self.module.add_string(&combo);
                    emit(code, Op::SetProp);
                    push_u32(code, id_s); push_u32(code, nm_s);
                    return Ok(());
                }
            }
        }
        // Inside a CREATE block, a bare-identifier LHS is a property of the
        // current instance — emit SetProp instead of StoreLocal/StoreGlobal.
        if let Some(inst) = self.create_stack.last().cloned() {
            // Inside CREATE: `Font.Size = 12` (MemberAccess LHS where the
            // object is a bare identifier, e.g. `Font`) — lower as
            // SetProp(inst, "font.size", value). Mirrors codegen-rust.
            if let Expression::MemberAccess(m) = &a.target {
                if let Expression::Identifier(sub_id) = &*m.object {
                    self.lower_expr(&a.value, code)?;
                    let id_s = self.module.add_string(&inst);
                    let combo = format!(
                        "{}.{}",
                        sub_id.name.to_lowercase(),
                        m.member.to_lowercase()
                    );
                    let nm_s = self.module.add_string(&combo);
                    emit(code, Op::SetProp);
                    push_u32(code, id_s); push_u32(code, nm_s);
                    return Ok(());
                }
            }
            if let Expression::Identifier(lhs_id) = &a.target {
                // If RHS is a component-instance identifier, lower it as a
                // string literal (component id) rather than a variable load
                // — same reason as the top-level case above.
                if let Expression::Identifier(rhs_id) = &a.value {
                    if self.component_instance_names.contains_key(&rhs_id.name.to_lowercase()) {
                        let cs = self.module.add_const(Const::Str(rhs_id.name.clone()));
                        emit(code, Op::LoadConst);
                        push_u32(code, cs);
                        let id_s = self.module.add_string(&inst);
                        let nm_s = self.module.add_string(&lhs_id.name);
                        emit(code, Op::SetProp);
                        push_u32(code, id_s); push_u32(code, nm_s);
                        return Ok(());
                    }
                }
                self.lower_expr(&a.value, code)?;
                let id_s = self.module.add_string(&inst);
                let nm_s = self.module.add_string(&lhs_id.name);
                emit(code, Op::SetProp);
                push_u32(code, id_s); push_u32(code, nm_s);
                return Ok(());
            }
        }
        // Normal assignment.
        self.lower_expr(&a.value, code)?;
        self.store_target(&a.target, code)
    }

    fn store_target(&mut self, target: &Expression, code: &mut Vec<u8>) -> Result<(), String> {
        match target {
            Expression::Identifier(id) => {
                if let Some(slot) = self.scope.get(&id.name) {
                    emit(code, Op::StoreLocal);
                    push_u16(code, slot);
                } else {
                    let s = self.global_str(&id.name);
                    emit(code, Op::StoreGlobal);
                    push_u32(code, s);
                }
                Ok(())
            }
            Expression::MemberAccess(m) => {
                if let Expression::Identifier(obj) = &*m.object {
                    if self.is_dynamic_object(&obj.name) {
                        // Stack: [value] → [value, id] → SetPropDyn.
                        self.lower_expr(&m.object, code)?;
                        let nm_s = self.module.add_string(&m.member);
                        emit(code, Op::SetPropDyn); push_u32(code, nm_s);
                        return Ok(());
                    }
                    let id_s = self.module.add_string(&obj.name);
                    let nm_s = self.module.add_string(&m.member);
                    emit(code, Op::SetProp);
                    push_u32(code, id_s); push_u32(code, nm_s);
                    Ok(())
                } else if let Some((object, sub, index)) = self.indexed_sub_object(&m.object) {
                    // `obj.item(i).caption = v` → CallMethodDyn(obj, "item.caption=", i, v)
                    let (object, index) = (object.clone(), index.to_vec());
                    let combo = format!("{}.{}=", sub.to_lowercase(), m.member.to_lowercase());
                    let tmp = self.scope.declare(&format!("__tmpv_{}", code.len()));
                    emit(code, Op::StoreLocal); push_u16(code, tmp);
                    self.lower_expr(&object, code)?;
                    for a in &index {
                        self.lower_expr(a, code)?;
                    }
                    emit(code, Op::LoadLocal); push_u16(code, tmp);
                    let m_s = self.module.add_string(&combo);
                    emit(code, Op::CallMethodDyn); push_u32(code, m_s); code.push(index.len() as u8 + 1);
                    emit(code, Op::Pop);
                    Ok(())
                } else if let Some((object, sub)) = self.sub_property(&m.object) {
                    // `obj.Canvas.Font.Size = v` → SetPropDyn(canvas, "font.size")
                    let object = object.clone();
                    let combo = format!("{}.{}", sub.to_lowercase(), m.member.to_lowercase());
                    self.lower_expr(&object, code)?;
                    let nm_s = self.module.add_string(&combo);
                    emit(code, Op::SetPropDyn); push_u32(code, nm_s);
                    Ok(())
                } else if self.object_type_of(&m.object).is_some() {
                    // `GF.Panel.Left = v`: stack [value] → [value, id].
                    self.lower_expr(&m.object, code)?;
                    let nm_s = self.module.add_string(&m.member);
                    emit(code, Op::SetPropDyn); push_u32(code, nm_s);
                    Ok(())
                } else {
                    Err("nested member-access store not yet supported".into())
                }
            }
            Expression::ArrayAccess(a) => {
                // `Bitmap.Pixel(x, y) = c`, `Screen.Cursors(i) = h`: the
                // object's `pixel` method with the value as an extra last
                // argument (by name, or through the id a local holds).
                if let Expression::MemberAccess(m) = &*a.array {
                    if let Expression::Identifier(obj) = &*m.object {
                        let by_name = !self.is_dynamic_object(&obj.name);
                        let tmp = self.scope.declare(&format!("__tmpv_{}", code.len()));
                        emit(code, Op::StoreLocal); push_u16(code, tmp);
                        if !by_name {
                            self.lower_expr(&m.object, code)?;
                        }
                        for i in &a.indices {
                            self.lower_expr(i, code)?;
                        }
                        emit(code, Op::LoadLocal); push_u16(code, tmp);
                        let mn_s = self.module.add_string(&m.member.to_lowercase());
                        if by_name {
                            let id_s = self.module.add_string(&obj.name);
                            emit(code, Op::CallMethod); push_u32(code, id_s); push_u32(code, mn_s);
                        } else {
                            emit(code, Op::CallMethodDyn); push_u32(code, mn_s);
                        }
                        code.push(a.indices.len() as u8 + 1);
                        emit(code, Op::Pop);
                        return Ok(());
                    } else {
                        // `This.Grid.Cell(x, y) = v`: the method of the object
                        // the expression holds.
                        let tmp = self.scope.declare(&format!("__tmpv_{}", code.len()));
                        emit(code, Op::StoreLocal); push_u16(code, tmp);
                        self.lower_expr(&m.object, code)?;
                        for i in &a.indices {
                            self.lower_expr(i, code)?;
                        }
                        emit(code, Op::LoadLocal); push_u16(code, tmp);
                        let mn_s = self.module.add_string(&m.member.to_lowercase());
                        emit(code, Op::CallMethodDyn); push_u32(code, mn_s);
                        code.push(a.indices.len() as u8 + 1);
                        emit(code, Op::Pop);
                        return Ok(());
                    }
                }
                // Stack so far: [..., value]. ASet wants [array, i1..iN, value]
                // and updates the array in place, so park the value first.
                let tmp = self.scope.declare(&format!("__tmpv_{}", code.len()));
                emit(code, Op::StoreLocal); push_u16(code, tmp);
                self.lower_expr(&a.array, code)?;
                for i in &a.indices {
                    self.lower_expr(i, code)?;
                }
                emit(code, Op::LoadLocal); push_u16(code, tmp);
                emit(code, Op::ASet); code.push(a.indices.len() as u8);
                Ok(())
            }
            _ => {
                // BASIC parses `A(0) = 42` and `r.Names(1) = "x"` as a
                // FunctionCall on the LHS. Re-route to the array-set path
                // by synthesizing an ArrayAccess view.
                if let Expression::FunctionCall(fc) = target {
                    if !fc.args.is_empty() {
                        let synth = ArrayAccessExpression {
                            span: fc.span.clone(),
                            array: fc.callee.clone(),
                            indices: fc.args.clone(),
                        };
                        return self.store_target(&Expression::ArrayAccess(synth), code);
                    }
                }
                Err("invalid assignment target".into())
            }
        }
    }

    fn lower_call_stmt(&mut self, c: &CallStatement, code: &mut Vec<u8>) -> Result<(), String> {
        if let Some(assignment) = rapidr_ast::inc_dec_assignment(c, |name| self.fn_indices.contains_key(name)) {
            return self.lower_assignment(&assignment, code);
        }
        // `__setfield obj, slot, value` (rapidr_ast::objects) → one opcode.
        let call = rapidr_ast::FunctionCallExpression { span: c.span, callee: Box::new(c.callee.clone()), args: c.args.clone() };
        if let Some(slot) = field_slot(&call, "__setfield", 3) {
            self.lower_expr(&c.args[0], code)?;
            self.lower_expr(&c.args[2], code)?;
            emit(code, Op::SetField); push_u16(code, slot);
            return Ok(());
        }
        let is_stream = |e: &Expression| self.object_type_of(e).is_some_and(|t| matches!(t.to_ascii_uppercase().as_str(), "RFILESTREAM" | "RMEMORYSTREAM"));
        if let Some(assignment) = rapidr_ast::stream_read_assignment(c, &is_stream) {
            return self.lower_assignment(&assignment, code);
        }
        if self.try_lower_object_call(&c.callee, &c.args, false, code)? {
            return Ok(());
        }
        if self.try_lower_pointer_call(&c.callee, &c.args, false, code)? {
            return Ok(());
        }
        // Push args.
        let user_routine = matches!(&c.callee, Expression::Identifier(id) if self.fn_indices.contains_key(&id.name));
        for a in &c.args {
            self.lower_arg(a, user_routine, code)?;
        }
        let argc = c.args.len() as u8;
        // Inside CREATE, a bare name that isn't a user SUB is a method of the
        // object being created (RapidQ: `CREATE F AS QFORM ... Center ...`).
        // Mirrors codegen-rust's `rp_comp_method(<create target>, ...)`.
        if let (Some(obj), Expression::Identifier(id)) = (self.create_stack.last().cloned(), &c.callee) {
            // (Not the compilers' own `__…` helpers.)
            if !self.fn_indices.contains_key(&id.name) && !id.name.starts_with("__") {
                let id_s = self.module.add_string(&obj);
                let mn_s = self.module.add_string(&id.name.to_lowercase());
                emit(code, Op::CallMethod);
                push_u32(code, id_s); push_u32(code, mn_s); code.push(argc);
                emit(code, Op::Pop);
                return Ok(());
            }
        }
        // Module-style call: `math.sqrt(x)` or `RNum.zeros(n)` — route to
        // builtin (mirrors `builtin_function_call` in codegen-rust).
        if let Expression::MemberAccess(m) = &c.callee {
            if let Expression::Identifier(obj) = &*m.object {
                if obj.name.eq_ignore_ascii_case("math") || is_component_type_name(&obj.name) {
                    let s = self.module.add_string(&m.member.to_lowercase());
                    emit(code, Op::CallBuiltin);
                    push_u32(code, s); code.push(argc);
                    emit(code, Op::Pop);
                    return Ok(());
                }
            }
            // Nested static call: Type.namespace.method(args)
            // e.g. RNum.random.randint() → builtin "random_randint".
            if let Expression::MemberAccess(inner) = &*m.object {
                if let Expression::Identifier(id) = &*inner.object {
                    if is_component_type_name(&id.name)
                        || id.name.eq_ignore_ascii_case("math")
                    {
                        let combined = format!(
                            "{}_{}",
                            inner.member.to_lowercase(),
                            m.member.to_lowercase()
                        );
                        let s = self.module.add_string(&combined);
                        emit(code, Op::CallBuiltin);
                        push_u32(code, s); code.push(argc);
                        emit(code, Op::Pop);
                        return Ok(());
                    }
                }
            }
        }
        if let Expression::Identifier(id) = &c.callee {
            if let Some(&fi) = self.fn_indices.get(&id.name) {
                let is_func = *self.fn_is_func.get(&id.name).unwrap_or(&false);
                if is_func {
                    emit(code, Op::CallFunc);
                    push_u32(code, fi); code.push(argc);
                    emit(code, Op::Pop); // discard return value when used as statement
                } else {
                    emit(code, Op::CallSub);
                    push_u32(code, fi); code.push(argc);
                }
                self.emit_byref_writeback(&id.name, &c.args, code)?;
                return Ok(());
            }
            // Builtin.
            self.check_builtin_call(&id.name, c.args.len(), c.span);
            let s = self.module.add_string(&id.name);
            emit(code, Op::CallBuiltin);
            push_u32(code, s); code.push(argc);
            emit(code, Op::Pop);
            // END: after the host's cleanup, stop executing here on every
            // host (the browser's END only logs), rather than falling
            // through into the code after it (typically GOSUB subroutines).
            if name_key(&id.name) == "end" {
                emit(code, Op::Halt);
            }
            return Ok(());
        }
        // CALL obj.method(args) — treat as method call.
        if let Expression::MemberAccess(m) = &c.callee {
            if let Expression::Identifier(obj) = &*m.object {
                let id_s = self.module.add_string(&obj.name);
                let mn_s = self.module.add_string(&m.member);
                emit(code, Op::CallMethod);
                push_u32(code, id_s); push_u32(code, mn_s); code.push(argc);
                emit(code, Op::Pop);
                return Ok(());
            }
        }
        Err("unsupported CALL target".into())
    }

    fn lower_if(
        &mut self,
        i: &IfStatement,
        code: &mut Vec<u8>,
        lines: &mut Vec<(u32, u32)>,
    ) -> Result<(), String> {
        // condition
        self.lower_expr(&i.condition, code)?;
        emit(code, Op::JumpIfNot);
        let mut next_branch_patch = code.len();
        push_u32(code, 0);

        // then body
        for s in &i.then_body {
            self.lower_stmt(s, code, lines)?;
        }
        let mut end_patches: Vec<usize> = Vec::new();
        if !i.elseif_branches.is_empty() || !i.else_body.is_empty() {
            emit(code, Op::Jump);
            end_patches.push(code.len());
            push_u32(code, 0);
        }

        for branch in &i.elseif_branches {
            let here = code.len() as u32;
            patch_u32(code, next_branch_patch, here);
            self.lower_expr(&branch.condition, code)?;
            emit(code, Op::JumpIfNot);
            next_branch_patch = code.len();
            push_u32(code, 0);
            for s in &branch.body {
                self.lower_stmt(s, code, lines)?;
            }
            emit(code, Op::Jump);
            end_patches.push(code.len());
            push_u32(code, 0);
        }

        // else
        let else_off = code.len() as u32;
        patch_u32(code, next_branch_patch, else_off);
        for s in &i.else_body {
            self.lower_stmt(s, code, lines)?;
        }

        let end_off = code.len() as u32;
        for p in end_patches {
            patch_u32(code, p, end_off);
        }
        Ok(())
    }

    fn emit_load_for_var(&mut self, f: &ForStatement, is_global: bool, code: &mut Vec<u8>) {
        if is_global {
            let s = self.global_str(&f.variable);
            emit(code, Op::LoadGlobal); push_u32(code, s);
        } else {
            let slot = self.scope.get(&f.variable).unwrap();
            emit(code, Op::LoadLocal); push_u16(code, slot);
        }
    }

    fn lower_for(
        &mut self,
        f: &ForStatement,
        code: &mut Vec<u8>,
        lines: &mut Vec<(u32, u32)>,
    ) -> Result<(), String> {
        let is_global = self.in_main && self.scope.get(&f.variable).is_none();
        
        // var = start
        if is_global {
            self.globals.insert(name_key(&f.variable));
            self.lower_expr(&f.start, code)?;
            let s = self.global_str(&f.variable);
            emit(code, Op::StoreGlobal); push_u32(code, s);
        } else {
            let var_slot = self.scope.declare(&f.variable);
            self.lower_expr(&f.start, code)?;
            emit(code, Op::StoreLocal); push_u16(code, var_slot);
        }

        // end and step into temp slots so they evaluate once.
        let temp_slot_id = self.scope.next_slot;
        let end_slot = self.scope.declare(&format!("__for_end_{}", temp_slot_id));
        self.lower_expr(&f.end, code)?;
        emit(code, Op::StoreLocal); push_u16(code, end_slot);
        let step_slot = self.scope.declare(&format!("__for_step_{}", temp_slot_id));
        if let Some(step) = &f.step {
            self.lower_expr(step, code)?;
        } else {
            let one = self.module.add_const(Const::Int(1));
            emit(code, Op::LoadConst); push_u32(code, one);
        }
        emit(code, Op::StoreLocal); push_u16(code, step_slot);

        // loop start. Continue while var <= end for a step >= 0, or
        // var >= end for a negative step (`FOR i = 10 TO 1 STEP -1`).
        let loop_top = code.len() as u32;
        let zero = self.module.add_const(Const::Int(0));
        emit(code, Op::LoadLocal); push_u16(code, step_slot);
        emit(code, Op::LoadConst); push_u32(code, zero);
        emit(code, Op::Lt);
        emit(code, Op::JumpIfNot);
        let to_ascending = code.len();
        push_u32(code, 0);
        self.emit_load_for_var(f, is_global, code);
        emit(code, Op::LoadLocal); push_u16(code, end_slot);
        emit(code, Op::Ge);
        emit(code, Op::Jump);
        let to_test = code.len();
        push_u32(code, 0);
        let ascending = code.len() as u32;
        patch_u32(code, to_ascending, ascending);
        self.emit_load_for_var(f, is_global, code);
        emit(code, Op::LoadLocal); push_u16(code, end_slot);
        emit(code, Op::Le);
        let test = code.len() as u32;
        patch_u32(code, to_test, test);
        emit(code, Op::JumpIfNot);
        let exit_patch = code.len();
        push_u32(code, 0);

        self.loop_stack.push(LoopCtx { kind: "FOR", breaks: Vec::new() });
        for s in &f.body {
            self.lower_stmt(s, code, lines)?;
        }
        // var = var + step
        if is_global {
            let s = self.global_str(&f.variable);
            emit(code, Op::LoadGlobal); push_u32(code, s);
            emit(code, Op::LoadLocal); push_u16(code, step_slot);
            emit(code, Op::Add);
            emit(code, Op::StoreGlobal); push_u32(code, s);
        } else {
            let var_slot = self.scope.get(&f.variable).unwrap();
            emit(code, Op::LoadLocal); push_u16(code, var_slot);
            emit(code, Op::LoadLocal); push_u16(code, step_slot);
            emit(code, Op::Add);
            emit(code, Op::StoreLocal); push_u16(code, var_slot);
        }
        emit(code, Op::Jump);
        push_u32(code, loop_top);

        let after = code.len() as u32;
        patch_u32(code, exit_patch, after);
        let ctx = self.loop_stack.pop().unwrap();
        for b in ctx.breaks { patch_u32(code, b, after); }
        Ok(())
    }

    fn lower_while(
        &mut self,
        w: &WhileStatement,
        code: &mut Vec<u8>,
        lines: &mut Vec<(u32, u32)>,
    ) -> Result<(), String> {
        let top = code.len() as u32;
        self.lower_expr(&w.condition, code)?;
        emit(code, Op::JumpIfNot);
        let exit_patch = code.len();
        push_u32(code, 0);
        self.loop_stack.push(LoopCtx { kind: "WHILE", breaks: Vec::new() });
        for s in &w.body {
            self.lower_stmt(s, code, lines)?;
        }
        emit(code, Op::Jump);
        push_u32(code, top);
        let after = code.len() as u32;
        patch_u32(code, exit_patch, after);
        let ctx = self.loop_stack.pop().unwrap();
        for b in ctx.breaks { patch_u32(code, b, after); }
        Ok(())
    }

    fn lower_do(
        &mut self,
        d: &DoLoopStatement,
        code: &mut Vec<u8>,
        lines: &mut Vec<(u32, u32)>,
    ) -> Result<(), String> {
        let top = code.len() as u32;
        self.loop_stack.push(LoopCtx { kind: "DO", breaks: Vec::new() });
        // pre-condition test (DO WHILE / DO UNTIL ... LOOP)
        let mut exit_patch: Option<usize> = None;
        if d.pre_condition {
            if let Some(cond) = &d.condition {
                self.lower_expr(cond, code)?;
                emit(code, if d.is_until { Op::JumpIf } else { Op::JumpIfNot });
                exit_patch = Some(code.len());
                push_u32(code, 0);
            }
        }
        for s in &d.body {
            self.lower_stmt(s, code, lines)?;
        }
        // post-condition test (DO ... LOOP WHILE / UNTIL)
        if !d.pre_condition {
            if let Some(cond) = &d.condition {
                self.lower_expr(cond, code)?;
                emit(code, if d.is_until { Op::JumpIfNot } else { Op::JumpIf });
                push_u32(code, top);
            } else {
                emit(code, Op::Jump);
                push_u32(code, top);
            }
        } else {
            emit(code, Op::Jump);
            push_u32(code, top);
        }
        let after = code.len() as u32;
        if let Some(p) = exit_patch { patch_u32(code, p, after); }
        let ctx = self.loop_stack.pop().unwrap();
        for b in ctx.breaks { patch_u32(code, b, after); }
        Ok(())
    }

    fn lower_return(&mut self, r: &ReturnStatement, code: &mut Vec<u8>) -> Result<(), String> {
        if let Some(v) = &r.value {
            self.lower_expr(v, code)?;
            emit(code, Op::RetVal);
        } else {
            // In a routine that uses GOSUB, RETURN first goes back to the
            // most recent GOSUB, if any.
            if self.routine.uses_gosub {
                emit(code, Op::GosubRet);
            }
            self.emit_return_from_routine(code);
        }
        Ok(())
    }

    /// Patch this routine's GOTO/GOSUB jumps; unknown labels are errors.
    fn resolve_labels(&mut self, code: &mut [u8], routine: &str) {
        let labels = std::mem::take(&mut self.routine);
        for (label, at, span) in labels.pending {
            match labels.offsets.get(&name_key(&label)) {
                Some(&target) => patch_u32(code, at, target),
                None => self.error_at(span, format!("Label '{label}' not found in {routine}")),
            }
        }
    }

    /// Leave the current SUB (Ret) or FUNCTION (return its result local).
    fn emit_return_from_routine(&mut self, code: &mut Vec<u8>) {
        match self.fn_ctx.and_then(|c| c.result_slot) {
            Some(slot) => {
                emit(code, Op::LoadLocal); push_u16(code, slot);
                emit(code, Op::RetVal);
            }
            None => emit(code, Op::Ret),
        }
    }

    /// After a call to `callee`, copy each BYREF parameter's final value back
    /// into the caller's variable (copy-in/copy-out, as VB does for plain
    /// variables). Only variables and array elements can be written back.
    fn emit_byref_writeback(&mut self, callee: &str, args: &[Expression], code: &mut Vec<u8>) -> Result<(), String> {
        let flags = self.fn_byref.get(callee).cloned().unwrap_or_default();
        for (i, arg) in args.iter().enumerate() {
            // `@x` at the call site makes that argument BYREF (RapidQ manual 3.5).
            let (by_ref, target) = match arg {
                Expression::Unary(u) if u.operator == UnaryOperator::Ref => (true, u.operand.as_ref()),
                _ => (flags.get(i).copied().unwrap_or(false), arg),
            };
            if !by_ref || !self.is_assignable(target) {
                continue;
            }
            emit(code, Op::LoadArgOut); code.push(i as u8);
            self.store_target(target, code)?;
        }
        Ok(())
    }

    /// One argument of a call. `@x` is only meaningful when calling a user
    /// SUB/FUNCTION (the value is passed now and written back after the call).
    fn lower_arg(&mut self, arg: &Expression, user_routine: bool, code: &mut Vec<u8>) -> Result<(), String> {
        match arg {
            Expression::Unary(u) if u.operator == UnaryOperator::Ref && user_routine => self.lower_expr(&u.operand, code),
            _ => self.lower_expr(arg, code),
        }
    }

    fn is_assignable(&self, e: &Expression) -> bool {
        match e {
            Expression::Identifier(id) => {
                !self.fn_indices.contains_key(&id.name)
                    && !self.component_instance_names.contains_key(&id.name.to_lowercase())
            }
            Expression::ArrayAccess(_) => true,
            Expression::FunctionCall(fc) => match fc.callee.as_ref() {
                // `a(i)` parses as a call; it's an element if `a` is a variable.
                Expression::Identifier(id) => {
                    fc.args.len() == 1
                        && !self.fn_indices.contains_key(&id.name)
                        && (self.scope.get(&id.name).is_some() || self.is_known_global(&id.name))
                }
                _ => false,
            },
            _ => false,
        }
    }

    fn lower_create(
        &mut self,
        c: &CreateStatement,
        code: &mut Vec<u8>,
        lines: &mut Vec<(u32, u32)>,
    ) -> Result<(), String> {
        let kind_s = self.module.add_string(&c.type_name.to_uppercase());
        let id_s = self.module.add_string(&c.name);
        emit(code, Op::CreateComp);
        push_u32(code, kind_s); push_u32(code, id_s);
        emit(code, Op::Pop); // discard returned reference for now
        // Nested CREATE: link this child to its parent so the runtime
        // can place / reparent the widget. Mirrors codegen-rust
        // `emit_create` → `rp_comp_set(name, "parent", v_str(parent))`.
        if let Some(parent) = self.create_stack.last().cloned() {
            let pv = self.module.add_const(Const::Str(parent));
            emit(code, Op::LoadConst); push_u32(code, pv);
            let id2 = self.module.add_string(&c.name);
            let pn = self.module.add_string("parent");
            emit(code, Op::SetProp);
            push_u32(code, id2); push_u32(code, pn);
        }
        self.create_stack.push(c.name.clone());
        // `Panel(0).Width = 100` inside the block: `c.Panel(0).Width = 100`.
        let body = {
            let known = |n: &str| {
                self.fn_indices.contains_key(n)
                    || builtins::is_builtin(n)
                    || self.scope.get(n).is_some()
                    || self.is_known_global(n)
                    || self.object_array_type(n).is_some()
            };
            rapidr_ast::qualify_create_body(&c.body, &c.name, &known)
        };
        for s in &body {
            self.lower_stmt(s, code, lines)?;
        }
        self.create_stack.pop();
        // RTIMER must be registered with the GUI tick loop, same as the
        // compiled mode.
        if c.type_name.eq_ignore_ascii_case("RTIMER") {
            self.emit_register_timer(&c.name, code);
        }
        Ok(())
    }

    /// Emit a `CallBuiltin("__gui_register_timer", [name])` op. The host
    /// dispatches this to `gui_register_timer` in its runtime crate.
    fn emit_register_timer(&mut self, name: &str, code: &mut Vec<u8>) {
        let nv = self.module.add_const(Const::Str(name.to_string()));
        emit(code, Op::LoadConst); push_u32(code, nv);
        let bi = self.module.add_string("__gui_register_timer");
        emit(code, Op::CallBuiltin);
        push_u32(code, bi); code.push(1u8);
        emit(code, Op::Pop);
    }

    fn lower_bind(&mut self, b: &BindStatement, code: &mut Vec<u8>) -> Result<(), String> {
        // BIND obj.event TO handler
        if let (Expression::MemberAccess(m), Expression::Identifier(h)) = (&b.target, &b.handler) {
            if let Expression::Identifier(obj) = &*m.object {
                if let Some(&fi) = self.fn_indices.get(&h.name) {
                    let id_s = self.module.add_string(&obj.name);
                    let ev_s = self.module.add_string(&m.member);
                    emit(code, Op::RegisterEvent);
                    push_u32(code, id_s); push_u32(code, ev_s); push_u32(code, fi);
                    return Ok(());
                }
            }
        }
        // `BIND ptr TO Proc` (or a TYPE method): a function pointer for CALLFUNC.
        if let Some(fi) = self.routine_pointer(&b.handler) {
            let c = self.module.add_const(Const::Int(fi as i64 + 1));
            emit(code, Op::LoadConst); push_u32(code, c);
            return self.store_target(&b.target, code);
        }
        // `BIND ptr TO Prototype` with no such routine only gives the pointer
        // a signature (RAPIDQ2.INC then assigns it: `hBind = hFunction`).
        if matches!(b.handler, Expression::Identifier(_)) {
            return Ok(());
        }
        Err("BIND needs `BIND variable TO SubName` or `BIND obj.Event TO SubName`".into())
    }

    /// The routine an expression names for a function pointer: `Proc`, or
    /// a dotted `SUB Obj.Name`. (TYPE method pointers are lowered to their
    /// `Type__Method` routine by `rapidr_ast::objects`.)
    fn routine_pointer(&self, e: &Expression) -> Option<u32> {
        match e {
            Expression::Identifier(id) => self.fn_indices.get(&id.name).copied(),
            Expression::MemberAccess(_) => self.dotted_routine(e).and_then(|full| self.fn_indices.get(&full).copied()),
            _ => None,
        }
    }

    /// `CODEPTR(Proc)` / `CALLBACK(Proc)` / `CALLFUNC(ptr, args…)`: function
    /// pointers are the routine's index + 1 (0 stays "no function").
    /// Returns false for any other call.
    fn try_lower_pointer_call(&mut self, callee: &Expression, args: &[Expression], want_value: bool, code: &mut Vec<u8>) -> Result<bool, String> {
        let Expression::Identifier(id) = callee else { return Ok(false) };
        if self.fn_indices.contains_key(&id.name) {
            return Ok(false);
        }
        match name_key(&id.name).as_str() {
            "codeptr" | "callback" => {
                let [target] = args else {
                    return Err(format!("{}(SubName) takes the name of a SUB or FUNCTION", id.name.to_uppercase()));
                };
                let fi = self
                    .routine_pointer(target)
                    .ok_or_else(|| format!("{}(…) takes the name of a SUB, FUNCTION or TYPE method", id.name.to_uppercase()))?;
                let c = self.module.add_const(Const::Int(fi as i64 + 1));
                emit(code, Op::LoadConst); push_u32(code, c);
            }
            "callfunc" => {
                let Some((ptr, rest)) = args.split_first() else {
                    return Err("CALLFUNC needs a function pointer: CALLFUNC(ptr, args…)".into());
                };
                self.lower_expr(ptr, code)?;
                for a in rest {
                    self.lower_expr(a, code)?;
                }
                emit(code, Op::CallIndirect); code.push(rest.len() as u8);
            }
            _ => return Ok(false),
        }
        if !want_value {
            emit(code, Op::Pop);
        }
        Ok(true)
    }

    // ------------------- expressions -------------------

    fn lower_expr(&mut self, e: &Expression, code: &mut Vec<u8>) -> Result<(), String> {
        match e {
            Expression::Literal(l) => self.lower_literal(l, code),
            Expression::Identifier(id) => {
                let name_lower = id.name.to_lowercase();
                // A left-out argument (`COLOR , 1`): the callee's default.
                if id.name == rapidr_ast::OMITTED_ARGUMENT {
                    emit(code, Op::LoadNull);
                    return Ok(());
                }
                if self.component_instance_names.contains_key(&name_lower) {
                    let cs = self.module.add_const(Const::Str(id.name.clone()));
                    emit(code, Op::LoadConst);
                    push_u32(code, cs);
                } else if let Some(slot) = self.scope.get(&id.name) {
                    emit(code, Op::LoadLocal); push_u16(code, slot);
                } else if !self.is_known_global(&id.name)
                    && builtins::BARE_BUILTINS.contains(&builtins::builtin_key(&id.name).as_str())
                {
                    // `x = TIMER`: a builtin written without parentheses.
                    let s = self.module.add_string(&id.name);
                    emit(code, Op::CallBuiltin);
                    push_u32(code, s); code.push(0);
                } else if let (Some(&fi), Some(true), false) = (
                    self.fn_indices.get(&id.name),
                    self.fn_is_func.get(&id.name).copied(),
                    self.is_known_global(&id.name),
                ) {
                    // A FUNCTION named without parentheses is called: `y = Five + 1`.
                    emit(code, Op::CallFunc);
                    push_u32(code, fi); code.push(0);
                } else {
                    let s = self.global_str(&id.name);
                    emit(code, Op::LoadGlobal); push_u32(code, s);
                }
                Ok(())
            }
            Expression::Binary(b) => {
                self.lower_expr(&b.left, code)?;
                self.lower_expr(&b.right, code)?;
                let op = match b.operator {
                    BinaryOperator::Add => Op::Add,
                    BinaryOperator::Subtract => Op::Sub,
                    BinaryOperator::Multiply => Op::Mul,
                    BinaryOperator::Divide => Op::Div,
                    BinaryOperator::IntegerDivide => Op::IDiv,
                    BinaryOperator::Modulo => Op::Mod,
                    BinaryOperator::Power => Op::Pow,
                    BinaryOperator::Concat => Op::Concat,
                    BinaryOperator::Equal => Op::Eq,
                    BinaryOperator::NotEqual => Op::Ne,
                    BinaryOperator::LessThan => Op::Lt,
                    BinaryOperator::LessThanOrEqual => Op::Le,
                    BinaryOperator::GreaterThan => Op::Gt,
                    BinaryOperator::GreaterThanOrEqual => Op::Ge,
                    BinaryOperator::And => Op::And,
                    BinaryOperator::Or => Op::Or,
                    BinaryOperator::Xor => Op::Xor,
                };
                emit(code, op);
                Ok(())
            }
            Expression::Unary(u) if u.operator == UnaryOperator::Ref => Err(
                "`@` passes a variable by reference to a SUB or FUNCTION (`MySub @x`); memory addresses for DLL calls only work in native builds (`rapidr build`)".into(),
            ),
            Expression::Unary(u) => {
                self.lower_expr(&u.operand, code)?;
                match u.operator {
                    UnaryOperator::Negate => emit(code, Op::Neg),
                    UnaryOperator::Not => emit(code, Op::Not),
                    UnaryOperator::Positive | UnaryOperator::Ref => {} // no-op
                }
                Ok(())
            }
            // `__to_long(v)` & co. (rapidr_ast::numeric) → one opcode.
            Expression::FunctionCall(fc) if fc.args.len() == 1 && numeric_kind(fc).is_some() => {
                let kind = numeric_kind(fc).unwrap_or(rapidr_value::numeric::NumKind::Double);
                self.lower_expr(&fc.args[0], code)?;
                emit(code, Op::ToNum); code.push(kind.code());
                Ok(())
            }
            // `__getfield(obj, slot)` (rapidr_ast::objects) → one opcode.
            Expression::FunctionCall(fc) if field_slot(fc, "__getfield", 2).is_some() => {
                let slot = field_slot(fc, "__getfield", 2).unwrap_or(0);
                self.lower_expr(&fc.args[0], code)?;
                emit(code, Op::GetField); push_u16(code, slot);
                Ok(())
            }
            Expression::FunctionCall(fc) => {
                if self.try_lower_object_call(&fc.callee, &fc.args, true, code)? {
                    return Ok(());
                }
                if self.try_lower_pointer_call(&fc.callee, &fc.args, true, code)? {
                    return Ok(());
                }
                // Check if this is a variant array/list subscript indexing:
                // callee is an identifier, not a defined function, and is a variable.
                if let Expression::Identifier(id) = fc.callee.as_ref() {
                    let is_local = self.scope.get(&id.name).is_some();
                    let is_global = self.is_known_global(&id.name);
                    if (is_local || is_global) && !fc.args.is_empty() && !self.fn_indices.contains_key(&id.name) {
                        let synth = ArrayAccessExpression {
                            span: fc.span.clone(),
                            array: fc.callee.clone(),
                            indices: fc.args.clone(),
                        };
                        return self.lower_expr(&Expression::ArrayAccess(synth), code);
                    }
                }

                let user_routine = matches!(&*fc.callee, Expression::Identifier(id) if self.fn_indices.contains_key(&id.name));
                for a in &fc.args { self.lower_arg(a, user_routine, code)?; }
                let argc = fc.args.len() as u8;
                // Module-style call: `math.sqrt(x)`, `RNum.zeros(n)` →
                // builtin (mirrors codegen-rust's `builtin_function_call`).
                if let Expression::MemberAccess(m) = &*fc.callee {
                    if let Expression::Identifier(obj) = &*m.object {
                        if obj.name.eq_ignore_ascii_case("math")
                            || is_component_type_name(&obj.name)
                        {
                            let s = self.module.add_string(&m.member.to_lowercase());
                            emit(code, Op::CallBuiltin);
                            push_u32(code, s); code.push(argc);
                            return Ok(());
                        }
                    }
                    // Nested static call: Type.namespace.method(args)
                    // e.g. RNum.random.randint() → builtin "random_randint".
                    if let Expression::MemberAccess(inner) = &*m.object {
                        if let Expression::Identifier(id) = &*inner.object {
                            if is_component_type_name(&id.name)
                                || id.name.eq_ignore_ascii_case("math")
                            {
                                let combined = format!(
                                    "{}_{}",
                                    inner.member.to_lowercase(),
                                    m.member.to_lowercase()
                                );
                                let s = self.module.add_string(&combined);
                                emit(code, Op::CallBuiltin);
                                push_u32(code, s); code.push(argc);
                                return Ok(());
                            }
                        }
                    }
                }
                if let Expression::Identifier(id) = &*fc.callee {
                    if let Some(&fi) = self.fn_indices.get(&id.name) {
                        let is_func = *self.fn_is_func.get(&id.name).unwrap_or(&false);
                        if is_func {
                            emit(code, Op::CallFunc);
                            push_u32(code, fi); code.push(argc);
                        } else {
                            // SUB used as expression — call then push Null.
                            emit(code, Op::CallSub);
                            push_u32(code, fi); code.push(argc);
                            emit(code, Op::LoadNull);
                        }
                        // The call's value stays on the stack underneath.
                        self.emit_byref_writeback(&id.name, &fc.args, code)?;
                        return Ok(());
                    }
                    self.check_builtin_call(&id.name, fc.args.len(), fc.span);
                    let s = self.module.add_string(&id.name);
                    emit(code, Op::CallBuiltin);
                    push_u32(code, s); code.push(argc);
                    return Ok(());
                }
                // Member-access callee: e.g. df.cell(i, 0), http.get(url).
                // Lower as a method call on the receiver.
                if let Expression::MemberAccess(m) = &*fc.callee {
                    if let Expression::Identifier(obj) = &*m.object {
                        let id_s = self.module.add_string(&obj.name);
                        let mn_s = self.module.add_string(&m.member);
                        emit(code, Op::CallMethod);
                        push_u32(code, id_s); push_u32(code, mn_s); code.push(argc);
                        return Ok(());
                    }
                }
                Err("unsupported function call target".into())
            }
            Expression::MethodCall(mc) => {
                let callee = Expression::MemberAccess(rapidr_ast::MemberAccessExpression {
                    span: mc.span,
                    object: mc.object.clone(),
                    member: mc.method.clone(),
                });
                if self.try_lower_object_call(&callee, &mc.args, true, code)? {
                    return Ok(());
                }
                for a in &mc.args { self.lower_expr(a, code)?; }
                let argc = mc.args.len() as u8;
                if let Expression::Identifier(obj) = &*mc.object {
                    let id_s = self.module.add_string(&obj.name);
                    let mn_s = self.module.add_string(&mc.method);
                    emit(code, Op::CallMethod);
                    push_u32(code, id_s); push_u32(code, mn_s); code.push(argc);
                    return Ok(());
                }
                Err("unsupported method call object".into())
            }
            Expression::MemberAccess(m) => {
                if self.dotted_routine(e).is_some() && self.try_lower_object_call(e, &[], true, code)? {
                    return Ok(());
                }
                // `IF Form.ShowModal THEN`: a method call (rapidr_ast::VALUE_METHODS).
                if let Expression::Identifier(obj) = &*m.object {
                    if rapidr_ast::VALUE_METHODS.contains(&m.member.to_ascii_lowercase().as_str()) {
                        let mn_s = self.module.add_string(&m.member);
                        if self.is_dynamic_object(&obj.name) {
                            self.lower_expr(&m.object, code)?;
                            emit(code, Op::CallMethodDyn); push_u32(code, mn_s); code.push(0);
                        } else {
                            let id_s = self.module.add_string(&obj.name);
                            emit(code, Op::CallMethod); push_u32(code, id_s); push_u32(code, mn_s); code.push(0);
                        }
                        return Ok(());
                    }
                }
                if let Expression::Identifier(obj) = &*m.object {
                    if self.is_dynamic_object(&obj.name) {
                        self.lower_expr(&m.object, code)?;
                        let nm_s = self.module.add_string(&m.member);
                        emit(code, Op::GetPropDyn); push_u32(code, nm_s);
                        return Ok(());
                    }
                    let id_s = self.module.add_string(&obj.name);
                    let nm_s = self.module.add_string(&m.member);
                    emit(code, Op::GetProp);
                    push_u32(code, id_s); push_u32(code, nm_s);
                    return Ok(());
                }
                // `obj.item(i).caption` → CallMethodDyn(obj, "item.caption", i)
                if let Some((object, sub, index)) = self.indexed_sub_object(&m.object) {
                    let (object, index) = (object.clone(), index.to_vec());
                    let combo = format!("{}.{}", sub.to_lowercase(), m.member.to_lowercase());
                    self.lower_expr(&object, code)?;
                    for a in &index {
                        self.lower_expr(a, code)?;
                    }
                    let m_s = self.module.add_string(&combo);
                    emit(code, Op::CallMethodDyn); push_u32(code, m_s); code.push(index.len() as u8);
                    return Ok(());
                }
                // `obj.Canvas.Font.Size`: a property of a component's Font.
                if let Some((object, sub)) = self.sub_property(&m.object) {
                    let object = object.clone();
                    let combo = format!("{}.{}", sub.to_lowercase(), m.member.to_lowercase());
                    self.lower_expr(&object, code)?;
                    let nm_s = self.module.add_string(&combo);
                    emit(code, Op::GetPropDyn); push_u32(code, nm_s);
                    return Ok(());
                }
                // `lbl(i).Caption`: a property of an element of an array of
                // components.
                if self.object_type_of(&m.object).is_some() {
                    self.lower_expr(&m.object, code)?;
                    let nm_s = self.module.add_string(&m.member);
                    emit(code, Op::GetPropDyn); push_u32(code, nm_s);
                    return Ok(());
                }
                // Nested member access: a.b.c → GetProp(a, "b.c").
                // Mirrors codegen-rust's flattening for sub-properties
                // like Form1.Font.Size, df.Names.Length, etc.
                if let Expression::MemberAccess(inner) = &*m.object {
                    if let Expression::Identifier(obj) = &*inner.object {
                        let id_s = self.module.add_string(&obj.name);
                        let combo = format!("{}.{}", inner.member.to_lowercase(), m.member.to_lowercase());
                        let nm_s = self.module.add_string(&combo);
                        emit(code, Op::GetProp);
                        push_u32(code, id_s); push_u32(code, nm_s);
                        return Ok(());
                    }
                }
                Err("nested member access not yet supported".into())
            }
            Expression::ArrayAccess(a) => {
                self.lower_expr(&a.array, code)?;
                for i in &a.indices {
                    self.lower_expr(i, code)?;
                }
                emit(code, Op::AGet); code.push(a.indices.len() as u8);
                Ok(())
            }
        }
    }

    fn lower_literal(&mut self, l: &Literal, code: &mut Vec<u8>) -> Result<(), String> {
        let c = match &l.value {
            LiteralValue::Integer(n) => Const::Int(*n),
            LiteralValue::Float(n) => Const::Double(*n),
            LiteralValue::String(s) => Const::Str(s.clone()),
        };
        let i = self.module.add_const(c);
        emit(code, Op::LoadConst);
        push_u32(code, i);
        Ok(())
    }
}

// ------------------- helpers -------------------

fn emit(code: &mut Vec<u8>, op: Op) { code.push(op as u8); }
fn push_u16(code: &mut Vec<u8>, v: u16) { code.extend_from_slice(&v.to_le_bytes()); }
fn push_u32(code: &mut Vec<u8>, v: u32) { code.extend_from_slice(&v.to_le_bytes()); }
fn patch_u32(code: &mut [u8], at: usize, v: u32) {
    code[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

/// Mirror of `rapidr_codegen_rust::is_component_type_name` (kept as a
/// local copy so bcgen has no runtime-crate dependency).
fn is_component_type_name(type_name: &str) -> bool {
    rapidr_ast::is_component_type_name(type_name)
}

/// Recursively walk every statement, collecting names declared via
/// `CREATE` (lowercase) into `out`. Recurses into CREATE bodies, SUB /
/// FUNCTION bodies, IF / FOR / WHILE / DO / WITH / SELECT bodies.
fn collect_create_names(stmts: &[Statement], out: &mut HashSet<String>) {
    for stmt in stmts {
        match stmt {
            Statement::Create(c) => {
                out.insert(c.name.to_lowercase());
                collect_create_names(&c.body, out);
            }
            Statement::Subroutine(s) => collect_create_names(&s.body, out),
            Statement::Function(f) => collect_create_names(&f.body, out),
            Statement::If(i) => {
                collect_create_names(&i.then_body, out);
                for b in &i.elseif_branches { collect_create_names(&b.body, out); }
                collect_create_names(&i.else_body, out);
            }
            Statement::For(f) => collect_create_names(&f.body, out),
            Statement::While(w) => collect_create_names(&w.body, out),
            Statement::DoLoop(d) => collect_create_names(&d.body, out),
            Statement::With(w) => collect_create_names(&w.body, out),
            Statement::SelectCase(s) => {
                for c in &s.cases { collect_create_names(&c.body, out); }
                collect_create_names(&s.case_else, out);
            }
            _ => {}
        }
    }
}

/// Every procedure and function of RapidQ (its KEYWORD.LST and manual),
/// as `name_key`s: an unknown call to one of these is "a RapidQ built-in
/// RapidR doesn't support yet", not a typo.
const RAPIDQ_BUILTINS: &[&str] = &[
    "abs", "acos", "asc", "asin", "atan", "atn", "bin", "callback", "callfunc", "ceil",
    "chdir", "chr", "cint", "clng", "cls", "codeptr", "color", "command", "commandcount",
    "convbase", "convbasex", "cos", "csrlin", "curdir", "date", "delete", "dir",
    "direxists", "doevents", "environ", "execute", "exp", "extractresource", "field",
    "fileexists", "fix", "floor", "format", "frac", "get", "getcapture", "getfocus", "hex",
    "hextodec", "iif", "initarray", "inkey", "inp", "input", "inpw", "insert", "instr",
    "int", "isconsole", "kill", "killmessage", "lbound", "lcase", "left", "len", "lflush",
    "libraryinst", "locate", "log", "lprint", "ltrim", "memcmp", "memcpy", "memset",
    "messagebox", "messagedlg", "microtimer", "mid", "mkdir", "mousex", "mousey",
    "nviewlibpresent", "out", "outw", "paramstr", "paramstrcount", "paramval",
    "paramvalcount", "pcopy", "peek", "playwav", "poke", "pos", "postmessage",
    "releasecapture", "rename", "replace", "replacesubstr", "resource", "resourcecount",
    "reverse", "rgb", "right", "rinstr", "rmdir", "rnd", "round", "rtlmovememory", "rtrim",
    "run", "sendmessage", "setcapture", "setconsoletitle", "setfocus", "sgn", "shell",
    "showmessage", "sin", "sizeof", "sleep", "sound", "space", "sqr", "str", "strf",
    "string", "tab", "tally", "tan", "time", "timer", "ubound", "ucase", "udtptr",
    "unloadlibrary", "val", "varptr", "vartype", "wstring", "wstringtoascii",
];

/// Whether a compile error also stops a native build (`rapidr build`, the
/// Rust backend): errors about the program itself do; ones about what only
/// the interpreter lacks (DLL calls, raw memory, Windows APIs) don't.
pub fn error_applies_to_native_builds(message: &str) -> bool {
    !message.contains(NATIVE_ONLY_MARKER) && !message.contains(WINDOWS_ONLY_MARKER)
}

/// Present in every error about a feature only native builds support.
const NATIVE_ONLY_MARKER: &str = "`rapidr build`";
/// In every error about calling Windows itself (RapidR doesn't emulate it).
const WINDOWS_ONLY_MARKER: &str = "; RapidR runs on every platform and doesn't emulate Windows";

/// Windows system DLLs (`DECLARE … LIB "user32"` …). Calls into other DLLs
/// are the program's own libraries, which native builds can load.
fn is_windows_system_library(lib: &str) -> bool {
    let base = lib.rsplit(['\\', '/']).next().unwrap_or(lib).trim_end_matches(".dll");
    matches!(
        base,
        "user32" | "kernel32" | "gdi32" | "gdiplus" | "shell32" | "winmm" | "advapi32" | "comctl32"
            | "comdlg32" | "wsock32" | "ws2_32" | "ole32" | "oleaut32" | "odbc32" | "odbccp32"
            | "wininet" | "winspool.drv" | "winspool" | "version" | "shlwapi" | "psapi" | "ntdll"
            | "msvcrt" | "mpr" | "netapi32" | "iphlpapi" | "urlmon" | "imm32" | "msimg32" | "avifil32"
            | "msvfw32" | "vfw32" | "opengl32" | "glu32" | "ddraw" | "dsound" | "dinput" | "rasapi32"
            | "setupapi" | "powrprof" | "secur32" | "crypt32" | "dwmapi" | "uxtheme" | "comctl32.dll"
    )
}

/// What to use instead of a Windows API call, by API name (`name_key`,
/// with the A/W suffix removed by the caller when needed).
fn windows_api_hint(api: &str) -> Option<&'static str> {
    let a = api;
    Some(if a.starts_with("sql") {
        "use the RSQLITE or RMYSQL component for databases"
    } else if a.starts_with("gdip") {
        "load, draw and save images with RIMAGE / RCANVAS"
    } else if a.starts_with("mcisend") || matches!(a, "playsound" | "sndplaysound" | "waveoutopen") {
        "play sounds with PLAYSOUND (or RWEBAUDIO on the web)"
    } else if matches!(a, "shellexecute" | "shellexecuteex" | "winexec" | "createprocess") {
        "run programs and open files with SHELL / SHELLWAIT"
    } else if matches!(a, "sleep") {
        "use SLEEP"
    } else if matches!(a, "messagebox" | "messageboxex") {
        "use SHOWMESSAGE"
    } else if matches!(a, "messagebeep" | "beep") {
        "use BEEP"
    } else if matches!(a, "gettickcount" | "timegettime" | "queryperformancecounter") {
        "use TIMER"
    } else if matches!(a, "getsystemmetrics") {
        "use Screen.Width / Screen.Height"
    } else if matches!(a, "getenvironmentvariable") {
        "use ENVIRON$"
    } else if matches!(a, "getcurrentdirectory" | "setcurrentdirectory") {
        "use CURDIR$ / CHDIR"
    } else if matches!(a, "createdirectory" | "removedirectory" | "deletefile" | "movefile") {
        "use MKDIR / RMDIR / KILL / RENAME"
    } else if matches!(
        a,
        "selectobject" | "createpen" | "createsolidbrush" | "deleteobject" | "getstockobject" | "getobject"
            | "getcurrentobject" | "ellipse" | "rectangle" | "lineto" | "moveto" | "movetoex" | "bitblt"
            | "stretchblt" | "getdc" | "releasedc" | "setpixel" | "getpixel" | "textout" | "setgraphicsmode"
            | "setworldtransform" | "createfont" | "createfontindirect" | "polygon"
    ) {
        "draw with an RCANVAS (Line, Circle, Rectangle, TextOut, …)"
    } else if matches!(
        a,
        "setwindowlong" | "getwindowlong" | "callwindowproc" | "setwindowpos" | "showwindow" | "movewindow"
            | "setfocus" | "getfocus" | "findwindow" | "getwindowrect" | "getclientrect" | "createwindowex"
            | "windowfrompoint" | "sendmessage" | "sendmessageapi" | "postmessage" | "setwindowtext"
            | "getwindowtext" | "getactivewindow" | "setforegroundwindow" | "enablewindow" | "destroywindow"
            | "setparent" | "getsyscolor" | "registerhotkey" | "setcapture" | "releasecapture"
    ) {
        "use the component's own properties, methods and events (Left, Top, Visible, Caption, SetFocus, OnKeyDown, …)"
    } else if matches!(
        a,
        "socket" | "recv" | "send" | "connect" | "bind" | "listen" | "accept" | "closesocket" | "htons" | "htonl"
            | "ntohs" | "inet_addr" | "gethostbyname" | "wsastartup" | "wsacleanup" | "wsaasyncselect" | "ioctlsocket"
    ) {
        "use the RSOCKET / RSERVERSOCKET components"
    } else if a.starts_with("internet") || a.starts_with("qftp_internet") || a.starts_with("http") || a == "urldownloadtofile" {
        "use the RHTTP component"
    } else if matches!(a, "multibytetowidechar" | "widechartomultibyte" | "lstrlen" | "lstrcpy") {
        "RapidR strings are Unicode already; use the string functions (LEN, MID$, …)"
    } else if matches!(a, "copymemory" | "rtlmovememory" | "movememory" | "zeromemory" | "fillmemory") {
        "raw memory access has no portable equivalent; copy values or arrays instead"
    } else if a.starts_with("reg") {
        "store settings in a file (e.g. with RSQLITE or a text file) instead of the registry"
    } else {
        return None;
    })
}

/// Ends every error about a feature no backend supports yet.
const UNSUPPORTED_MARKER: &str = " isn't supported yet";

/// Routines the program can reach (`name_key` of SUBs/FUNCTIONs, TYPE
/// methods included once `rapidr_ast::objects` has lowered them), found by
/// following every name mentioned from the main program onwards. Any
/// mention counts (a call, `OnClick = Handler`, BIND, CODEPTR), so this
/// over-approximates what can run.
fn reachable_routines(program: &Program) -> HashSet<String> {
    let mut routines: HashMap<String, &[Statement]> = HashMap::new();
    let mut main: Vec<&[Statement]> = Vec::new();
    for (i, stmt) in program.statements.iter().enumerate() {
        match stmt {
            Statement::Subroutine(s) => { routines.insert(name_key(&s.name), &s.body); }
            Statement::Function(f) => { routines.insert(name_key(&f.name), &f.body); }
            _ => main.push(std::slice::from_ref(&program.statements[i])),
        }
    }
    let mut reached: HashSet<String> = HashSet::new();
    let mut pending = main;
    while let Some(body) = pending.pop() {
        let mut names: Vec<String> = Vec::new();
        {
            let mut stmt_names: Vec<String> = Vec::new();
            rapidr_ast::walk(
                body,
                &mut |s| match s {
                    Statement::Dim(d) => stmt_names.push(d.type_name.clone()),
                    Statement::Create(c) => stmt_names.push(c.type_name.clone()),
                    _ => {}
                },
                &mut |e| {
                    if let Expression::Identifier(id) = e {
                        names.push(id.name.clone());
                    }
                },
            );
            names.extend(stmt_names);
        }
        for name in names {
            let key = name_key(&name);
            if let Some(body) = routines.get(&key) {
                if reached.insert(key.clone()) {
                    pending.push(body);
                }
            }
        }
    }
    reached
}

/// The slot of `name(obj, slot, …)` with `argc` arguments and a literal
/// slot (the object pass's field access), for the GetField/SetField opcodes.
fn field_slot(fc: &rapidr_ast::FunctionCallExpression, name: &str, argc: usize) -> Option<u16> {
    let Expression::Identifier(id) = fc.callee.as_ref() else { return None };
    if !id.name.eq_ignore_ascii_case(name) || fc.args.len() != argc {
        return None;
    }
    match &fc.args[1] {
        Expression::Literal(lit) => match &lit.value {
            LiteralValue::Integer(n) => u16::try_from(*n).ok(),
            _ => None,
        },
        _ => None,
    }
}

/// The conversion `fc` is, when it's `__to_long(v)` & co.
fn numeric_kind(fc: &rapidr_ast::FunctionCallExpression) -> Option<rapidr_value::numeric::NumKind> {
    let Expression::Identifier(id) = fc.callee.as_ref() else { return None };
    rapidr_value::numeric::NumKind::from_builtin(&id.name)
}

/// The value a variable of a BASIC type starts with (as in native builds'
/// `default_value_for_type`); VARIANTs and objects start as Null.
fn type_default(type_name: &str) -> Const {
    match type_name.to_ascii_uppercase().as_str() {
        "STRING" => Const::Str(String::new()),
        "INTEGER" | "LONG" | "SHORT" | "BYTE" | "WORD" | "DWORD" | "INT64" => Const::Int(0),
        "SINGLE" | "DOUBLE" | "CURRENCY" => Const::Double(0.0),
        _ => Const::Null,
    }
}

/// Collect every component instance name (declared via either CREATE or
/// `DIM x AS <ComponentType>`) anywhere in the program. Maps lowercase
/// id → original-case name as written.
fn collect_component_instance_names(stmts: &[Statement], out: &mut HashMap<String, String>) {
    for stmt in stmts {
        match stmt {
            Statement::Create(c) => {
                out.insert(c.name.to_lowercase(), c.name.clone());
                collect_component_instance_names(&c.body, out);
            }
            Statement::Dim(d) => {
                if is_component_type_name(&d.type_name) {
                    // `DIM lbl(3) AS QLABEL` is an array of components, not one.
                    for decl in d.declarators.iter().filter(|d| d.dimensions.is_empty()) {
                        out.insert(decl.name.to_lowercase(), decl.name.clone());
                    }
                }
            }
            Statement::Subroutine(s) => collect_component_instance_names(&s.body, out),
            Statement::Function(f) => collect_component_instance_names(&f.body, out),
            Statement::If(i) => {
                collect_component_instance_names(&i.then_body, out);
                for b in &i.elseif_branches { collect_component_instance_names(&b.body, out); }
                collect_component_instance_names(&i.else_body, out);
            }
            Statement::For(f) => collect_component_instance_names(&f.body, out),
            Statement::While(w) => collect_component_instance_names(&w.body, out),
            Statement::DoLoop(d) => collect_component_instance_names(&d.body, out),
            Statement::With(w) => collect_component_instance_names(&w.body, out),
            Statement::SelectCase(s) => {
                for c in &s.cases { collect_component_instance_names(&c.body, out); }
                collect_component_instance_names(&s.case_else, out);
            }
            _ => {}
        }
    }
}

impl Bcgen {
    fn stmt_line(&self, stmt: &Statement) -> u32 {
        let span = stmt_span(stmt);

        if let Some(ref starts) = self.line_starts {
            match starts.binary_search(&span.start) {
                Ok(idx) => (idx + 1) as u32,
                Err(idx) => idx as u32,
            }
        } else {
            0
        }
    }
}

/// Whether `stmts` (including nested blocks, but not nested SUB/FUNCTION
/// definitions) contain a GOSUB.
fn contains_gosub(stmts: &[Statement]) -> bool {
    stmts.iter().any(|s| match s {
        Statement::Gosub(_) => true,
        Statement::If(i) => {
            contains_gosub(&i.then_body)
                || i.elseif_branches.iter().any(|b| contains_gosub(&b.body))
                || contains_gosub(&i.else_body)
        }
        Statement::For(f) => contains_gosub(&f.body),
        Statement::While(w) => contains_gosub(&w.body),
        Statement::DoLoop(d) => contains_gosub(&d.body),
        Statement::SelectCase(c) => {
            c.cases.iter().any(|b| contains_gosub(&b.body)) || contains_gosub(&c.case_else)
        }
        Statement::With(w) => contains_gosub(&w.body),
        Statement::Create(c) => contains_gosub(&c.body),
        _ => false,
    })
}

fn stmt_span(stmt: &Statement) -> TextSpan {
    match stmt {
        Statement::Assignment(a) => a.span,
        Statement::Bind(b) => b.span,
        Statement::Call(c) => c.span,
        Statement::Close(c) => c.span,
        Statement::Comment(c) => c.span,
        Statement::Const(c) => c.span,
        Statement::Create(c) => c.span,
        Statement::Declare(d) => d.span,
        Statement::Dim(d) => d.span,
        Statement::Directive(d) => d.span,
        Statement::DoLoop(d) => d.span,
        Statement::Exit(e) => e.span,
        Statement::For(f) => f.span,
        Statement::Function(f) => f.span,
        Statement::If(i) => i.span,
        Statement::Import(i) => i.span,
        Statement::Input(i) => i.span,
        Statement::Label(l) => l.span,
        Statement::Goto(j) | Statement::Gosub(j) => j.span,
        Statement::Line(l) => l.span,
        Statement::Open(o) => o.span,
        Statement::Print(p) => p.span,
        Statement::PrintHash(p) => p.span,
        Statement::Return(r) => r.span,
        Statement::Seek(s) => s.span,
        Statement::SelectCase(s) => s.span,
        Statement::Subroutine(s) => s.span,
        Statement::Type(t) => t.span,
        Statement::While(w) => w.span,
        Statement::With(w) => w.span,
        Statement::WriteHash(w) => w.span,
        Statement::RustBlock(r) => r.span,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rapidr_vm::{StubHost, Vm};

    fn parse(src: &str) -> Program {
        let toks = rapidr_lexer::Lexer::new(src, Some("test".into()))
            .tokenize()
            .expect("lex");
        rapidr_parser::parse_tokens(&toks).expect("test source should parse")
    }

    fn run(src: &str) -> StubHost {
        let prog = parse(src);
        let compiled = compile_program(&prog).unwrap();
        let mut h = StubHost::default();
        let mut vm = Vm::new(&mut h);
        vm.run(&compiled.module).unwrap();
        h
    }

    #[test]
    fn native_only_code_blocks_only_when_reachable() {
        let lib = "DECLARE FUNCTION MessageBeep LIB \"user32\" (t AS LONG) AS LONG\n\
                   SUB Unused\n  x = MessageBeep(0)\n  y = VARPTR(x)\nEND SUB\n\
                   SUB Used\n  z = MessageBeep(1)\nEND SUB\n\
                   TYPE TIdle\n  SUB Go\n    q = MessageBeep(2)\n  END SUB\nEND TYPE\n";
        // Nothing reaches the DLL calls: the program compiles and runs.
        let h = run(&format!("{lib}PRINT \"ok\""));
        assert_eq!(h.output, "ok\n");
        // Calling `Used` reaches one; only that call is reported.
        let src = format!("{lib}Used");
        let Err(err) = compile_program_with_source(&parse(&src), Some(&src)) else { panic!("should not compile") };
        assert_eq!(err.lines().count(), 1, "{err}");
        assert!(err.contains("MessageBeep") && err.starts_with("7:"), "{err}");
        // A method called on an instance brings its code along.
        let src = format!("{lib}DIM t AS TIdle\nt.Go");
        let Err(err) = compile_program_with_source(&parse(&src), Some(&src)) else { panic!("should not compile") };
        assert!(err.starts_with("11:"), "{err}");
    }

    #[test]
    fn unknown_names_in_unused_code_only_pass_in_libraries() {
        let src = "SUB Helper\n  Frobble 1\nEND SUB\nPRINT \"ok\"\n";
        // The program's own code: a typo is an error even if nothing calls it.
        let Err(err) = compile_program_with_source(&parse(src), Some(src)) else { panic!("should not compile") };
        assert!(err.starts_with("2:3:") && err.contains("Frobble"), "{err}");
        // The same routine from an $INCLUDE'd library (lines 1-3): unused, so fine.
        let library = [true, true, true, false];
        assert!(compile_program_with_libraries(&parse(src), Some(src), &library).is_ok());
        // …unless the program calls it.
        let used = format!("{src}Helper\n");
        let Err(err) = compile_program_with_libraries(&parse(&used), Some(&used), &library) else { panic!("should not compile") };
        assert!(err.contains("Frobble"), "{err}");
    }

    #[test]
    fn windows_api_calls_say_what_to_use_instead() {
        let src = "DECLARE FUNCTION ShellExecute LIB \"shell32.dll\" ALIAS \"ShellExecuteA\" (h AS LONG, f AS STRING) AS LONG\n\
                   DECLARE FUNCTION MySum LIB \"mylib\" (a AS LONG) AS LONG\n\
                   x = ShellExecute(0, \"a.txt\")\ny = MySum(1)\n";
        let Err(err) = compile_program_with_source(&parse(src), Some(src)) else { panic!("should not compile") };
        let lines: Vec<&str> = err.lines().collect();
        assert!(lines[0].contains("Windows API function") && lines[0].contains("SHELL"), "{err}");
        assert!(lines[1].contains("external DLL function") && lines[1].contains("rapidr build"), "{err}");
    }

    #[test]
    fn print_string() {
        let h = run(r#"PRINT "hello""#);
        assert_eq!(h.output, "hello\n");
    }

    #[test]
    fn arithmetic() {
        let h = run(r#"PRINT 3 + 4 * 2"#);
        assert_eq!(h.output, "11\n");
    }

    #[test]
    fn for_loop() {
        let h = run("DIM i AS INTEGER\nFOR i = 1 TO 3\nPRINT i\nNEXT i");
        assert_eq!(h.output, "1\n2\n3\n");
    }

    #[test]
    fn if_else() {
        let h = run("DIM x AS INTEGER\nx = 5\nIF x > 3 THEN\nPRINT \"big\"\nELSE\nPRINT \"small\"\nEND IF");
        assert_eq!(h.output, "big\n");
    }

    #[test]
    fn sub_and_call() {
        let src = "SUB greet(name AS STRING)\nPRINT \"Hi \"; name\nEND SUB\nCALL greet(\"world\")";
        let h = run(src);
        assert_eq!(h.output, "Hi world\n");
    }

    #[test]
    fn function_returning_value() {
        let src = "FUNCTION sq(n AS INTEGER) AS INTEGER\nRETURN n * n\nEND FUNCTION\nPRINT sq(7)";
        let h = run(src);
        assert_eq!(h.output, "49\n");
    }
}

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
//! * Identifier resolution: locals first (per-function scope), then globals
//!   by name. SUB / FUNCTION names are resolved to function indices for
//!   `CallSub` / `CallFunc`. Anything else becomes a `CallBuiltin`
//!   (the host's builtin dispatch decides how to handle it).
//! * `CREATE Foo AS Kind ... END CREATE` lowers to `CreateComp(Kind, Foo)`
//!   followed by per-property `SetProp` and per-method `CallMethod`.
//!   When a property assignment's RHS is a bare identifier matching a known
//!   SUB, it is lowered to `RegisterEvent` (e.g. `OnClick = MyHandler`).
//!
//! Unsupported constructs (DECLARE foreign DLL, file I/O, RUST blocks,
//! TYPE/UDT, IMPORT, EXIT) currently lower to a no-op and emit a warning
//! into [`Compiled::warnings`]; they will be filled in incrementally.

use std::collections::{HashMap, HashSet};

use rapidr_ast::{
    ArrayAccessExpression, AssignmentStatement, BinaryOperator, BindStatement, CallStatement,
    CreateStatement, DoLoopStatement, Expression, ForStatement, FunctionStatement, IfStatement,
    Literal, LiteralValue, Parameter, PrintStatement, Program, ReturnStatement, Statement,
    SubroutineStatement, UnaryOperator, WhileStatement,
};
use rapidr_bytecode::{Const, Function, Module, Op, Param};

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
    let mut bcgen = Bcgen::new();
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

/// Per-function scope: maps local variable names to slot indices.
#[derive(Default, Clone)]
struct Scope {
    locals: HashMap<String, u16>,
    next_slot: u16,
}

impl Scope {
    fn declare(&mut self, name: &str) -> u16 {
        if let Some(&s) = self.locals.get(name) {
            return s;
        }
        let s = self.next_slot;
        self.next_slot += 1;
        self.locals.insert(name.to_string(), s);
        s
    }
    fn get(&self, name: &str) -> Option<u16> {
        self.locals.get(name).copied()
    }
}

struct Bcgen {
    module: Module,
    /// Map SUB/FUNCTION names → function index.
    fn_indices: HashMap<String, u32>,
    /// Whether each name is a FUNCTION (true) or SUB (false). Used to choose
    /// CallFunc vs CallSub when invoked from an expression.
    fn_is_func: HashMap<String, bool>,
    warnings: Vec<String>,
    /// Scope stack for the function currently being lowered.
    scope: Scope,
    /// Set of declared global variables (lowercase names)
    globals: HashSet<String>,
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
    /// Whether we are currently lowering the top-level main program.
    in_main: bool,
    /// Starts of each line (byte offsets) to resolve line numbers for statements.
    line_starts: Option<Vec<usize>>,
}

struct LoopCtx {
    /// Patch sites (offsets that hold a u32 target) waiting for the loop end.
    breaks: Vec<usize>,
}

impl Bcgen {
    fn new() -> Self {
        Self {
            module: Module::new(),
            fn_indices: HashMap::new(),
            fn_is_func: HashMap::new(),
            warnings: Vec::new(),
            scope: Scope::default(),
            globals: HashSet::new(),
            _with_object: None,
            loop_stack: Vec::new(),
            create_stack: Vec::new(),
            create_declared_names: HashSet::new(),
            component_instance_names: HashMap::new(),
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
                    subs.push(s);
                }
                Statement::Function(f) => {
                    let idx = self.reserve_function(&f.name, &f.params, true);
                    self.fn_indices.insert(f.name.clone(), idx);
                    self.fn_is_func.insert(f.name.clone(), true);
                    funcs.push(f);
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
        for stmt in &program.statements {
            if matches!(stmt, Statement::Subroutine(_) | Statement::Function(_)) {
                continue;
            }
            self.lower_stmt(stmt, &mut main_code, &mut main_lines)?;
        }
        self.in_main = false;
        emit(&mut main_code, Op::Halt);
        let main_locals = self.scope.next_slot as u32;
        let mut main_local_names = vec![String::new(); main_locals as usize];
        for (name, &slot) in &self.scope.locals {
            if (slot as usize) < main_local_names.len() {
                main_local_names[slot as usize] = name.clone();
            }
        }
        self.scope = saved_scope;
        let f = &mut self.module.functions[main_idx as usize];
        f.code = main_code;
        f.line_info = main_lines;
        f.n_locals = main_locals;
        f.local_names = main_local_names;

        // Pass 3: emit each SUB and FUNCTION body.
        for s in subs {
            self.compile_function_body(&s.name, &s.params, &s.body, false)?;
        }
        for f in funcs {
            self.compile_function_body(&f.name, &f.params, &f.body, true)?;
        }

        Ok(())
    }

    /// Reserve a function entry up-front so its index is known before its
    /// body is lowered (forward references / recursion).
    fn reserve_function(&mut self, name: &str, params: &[Parameter], _is_func: bool) -> u32 {
        let mut f = Function::default();
        f.name = name.to_string();
        f.params = params.iter().map(|p| Param { name: p.name.clone(), by_ref: p.by_ref }).collect();
        self.module.add_function(f)
    }

    fn compile_function_body(
        &mut self,
        name: &str,
        params: &[Parameter],
        body: &[Statement],
        is_func: bool,
    ) -> Result<(), String> {
        let idx = *self.fn_indices.get(name).unwrap();
        let saved_scope = std::mem::take(&mut self.scope);
        // Pre-declare parameter slots (slots 0..N).
        for p in params {
            self.scope.declare(&p.name);
        }
        let mut code = Vec::new();
        let mut lines = Vec::new();
        for stmt in body {
            self.lower_stmt(stmt, &mut code, &mut lines)?;
        }
        // Implicit return.
        if is_func {
            // FUNCTION without explicit RETURN: push Null then RetVal.
            emit(&mut code, Op::LoadNull);
            emit(&mut code, Op::RetVal);
        } else {
            emit(&mut code, Op::Ret);
        }
        let n_locals = self.scope.next_slot as u32;
        let mut local_names = vec![String::new(); n_locals as usize];
        for (name, &slot) in &self.scope.locals {
            if (slot as usize) < local_names.len() {
                local_names[slot as usize] = name.clone();
            }
        }
        self.scope = saved_scope;
        let f = &mut self.module.functions[idx as usize];
        f.code = code;
        f.line_info = lines;
        f.n_locals = n_locals;
        f.local_names = local_names;
        Ok(())
    }

    // ------------------- statements -------------------

    fn lower_stmt(
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
                self.globals.insert(c.name.to_lowercase());
                self.lower_expr(&c.value, code)?;
                let s = self.module.add_string(&c.name);
                emit(code, Op::StoreGlobal);
                push_u32(code, s);
            }
            Statement::Dim(d) => {
                // Declare locals; initial value Null is already the default.
                for decl in &d.declarators {
                    if !self.in_main {
                        self.scope.declare(&decl.name);
                    } else {
                        self.globals.insert(decl.name.to_lowercase());
                    }
                    // Component DIM → eagerly CreateComp (mirrors the
                    // compiled-mode `emit_dim` path), unless a CREATE block
                    // already declares the same name.
                    if is_component_type_name(&d.type_name) {
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
                // For now, just lower the body normally; member-access uses
                // explicit object references in the AST.
                for s in &w.body {
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
                emit(code, Op::Input);
                self.store_target(&i.target, code)?;
            }
            Statement::Exit(_) => {
                if let Some(ctx) = self.loop_stack.last_mut() {
                    emit(code, Op::Jump);
                    ctx.breaks.push(code.len());
                    push_u32(code, 0); // patched at loop end
                } else {
                    self.warnings.push("EXIT outside loop ignored".into());
                }
            }
            Statement::SelectCase(s) => {
                // Lower as cascading IFs. Only equality checks supported here.
                // Stash the discriminant into a temp local.
                let tmp = self.scope.declare(&format!("__sel_{}", code.len()));
                self.lower_expr(&s.expression, code)?;
                emit(code, Op::StoreLocal); push_u16(code, tmp);
                let mut end_jumps: Vec<usize> = Vec::new();
                for case in &s.cases {
                    // Build a disjunction of (tmp == val) for each value.
                    if case.values.is_empty() { continue; }
                    let mut next_jumps: Vec<usize> = Vec::new();
                    for (i, val) in case.values.iter().enumerate() {
                        emit(code, Op::LoadLocal); push_u16(code, tmp);
                        self.lower_expr(val, code)?;
                        emit(code, Op::Eq);
                        if i + 1 == case.values.len() {
                            // Last value: if false, jump to next case.
                            emit(code, Op::JumpIfNot);
                            next_jumps.push(code.len());
                            push_u32(code, 0);
                        } else {
                            // If true, jump to body (collect later); else fallthrough.
                            emit(code, Op::JumpIf);
                            // Patch directly to body start, recorded after we know it.
                            // Easier: use a "match" flag local.
                            // Simplification: just chain JumpIfNot for each, with a single
                            // Or-bridge. For Phase 2 the simple form below is sufficient
                            // when each case has 1 value.
                            // Mark unreachable for the multi-value case:
                            self.warnings.push("SELECT CASE with multiple values per branch only partially supported".into());
                            push_u32(code, 0);
                        }
                    }
                    // Body
                    for stmt in &case.body {
                        self.lower_stmt(stmt, code, lines)?;
                    }
                    emit(code, Op::Jump);
                    end_jumps.push(code.len());
                    push_u32(code, 0);
                    let after = code.len() as u32;
                    for j in next_jumps {
                        patch_u32(code, j, after);
                    }
                }
                // CASE ELSE
                for stmt in &s.case_else {
                    self.lower_stmt(stmt, code, lines)?;
                }
                let end = code.len() as u32;
                for j in end_jumps {
                    patch_u32(code, j, end);
                }
            }
            // Unhandled — record a warning and emit nothing.
            other => {
                self.warnings.push(format!("statement not yet lowered: {}", short_name(other)));
            }
        }
        Ok(())
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
        }
        Ok(())
    }

    fn lower_assignment(&mut self, a: &AssignmentStatement, code: &mut Vec<u8>) -> Result<(), String> {
        if self.in_main {
            if let Expression::Identifier(id) = &a.target {
                self.globals.insert(id.name.to_lowercase());
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
            if let Expression::MemberAccess(inner) = &*m.object {
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
                    let s = self.module.add_string(&id.name);
                    emit(code, Op::StoreGlobal);
                    push_u32(code, s);
                }
                Ok(())
            }
            Expression::MemberAccess(m) => {
                if let Expression::Identifier(obj) = &*m.object {
                    let id_s = self.module.add_string(&obj.name);
                    let nm_s = self.module.add_string(&m.member);
                    emit(code, Op::SetProp);
                    push_u32(code, id_s); push_u32(code, nm_s);
                    Ok(())
                } else {
                    Err("nested member-access store not yet supported".into())
                }
            }
            Expression::ArrayAccess(a) => {
                // Stack so far: [..., value]. We need [arr, idx, value].
                // Re-emit array base + first index, then move value on top.
                // For simplicity assume single index.
                if a.indices.len() != 1 {
                    return Err("multi-dim ASet not yet supported".into());
                }
                // Save value into a temp local.
                let tmp = self.scope.declare(&format!("__tmpv_{}", code.len()));
                emit(code, Op::StoreLocal); push_u16(code, tmp);
                self.lower_expr(&a.array, code)?;
                self.lower_expr(&a.indices[0], code)?;
                emit(code, Op::LoadLocal); push_u16(code, tmp);
                emit(code, Op::ASet);
                // ASet pushes the new array; if the base was a simple identifier,
                // store it back.
                if let Expression::Identifier(id) = &*a.array {
                    if let Some(slot) = self.scope.get(&id.name) {
                        emit(code, Op::StoreLocal); push_u16(code, slot);
                    } else {
                        let s = self.module.add_string(&id.name);
                        emit(code, Op::StoreGlobal); push_u32(code, s);
                    }
                } else {
                    emit(code, Op::Pop);
                }
                Ok(())
            }
            _ => {
                // BASIC parses `A(0) = 42` and `r.Names(1) = "x"` as a
                // FunctionCall on the LHS. Re-route to the array-set path
                // by synthesizing an ArrayAccess view.
                if let Expression::FunctionCall(fc) = target {
                    if fc.args.len() == 1 {
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
        // Push args.
        for a in &c.args {
            self.lower_expr(a, code)?;
        }
        let argc = c.args.len() as u8;
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
                return Ok(());
            }
            // Builtin.
            let s = self.module.add_string(&id.name);
            emit(code, Op::CallBuiltin);
            push_u32(code, s); code.push(argc);
            emit(code, Op::Pop);
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

    fn lower_for(
        &mut self,
        f: &ForStatement,
        code: &mut Vec<u8>,
        lines: &mut Vec<(u32, u32)>,
    ) -> Result<(), String> {
        let is_global = self.in_main && self.scope.get(&f.variable).is_none();
        
        // var = start
        if is_global {
            self.globals.insert(f.variable.to_lowercase());
            self.lower_expr(&f.start, code)?;
            let s = self.module.add_string(&f.variable);
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

        // loop start
        let loop_top = code.len() as u32;
        // condition: var <= end  (assumes positive step)
        if is_global {
            let s = self.module.add_string(&f.variable);
            emit(code, Op::LoadGlobal); push_u32(code, s);
        } else {
            let var_slot = self.scope.get(&f.variable).unwrap();
            emit(code, Op::LoadLocal); push_u16(code, var_slot);
        }
        emit(code, Op::LoadLocal); push_u16(code, end_slot);
        emit(code, Op::Le);
        emit(code, Op::JumpIfNot);
        let exit_patch = code.len();
        push_u32(code, 0);

        self.loop_stack.push(LoopCtx { breaks: Vec::new() });
        for s in &f.body {
            self.lower_stmt(s, code, lines)?;
        }
        // var = var + step
        if is_global {
            let s = self.module.add_string(&f.variable);
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
        self.loop_stack.push(LoopCtx { breaks: Vec::new() });
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
        self.loop_stack.push(LoopCtx { breaks: Vec::new() });
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
            emit(code, Op::Ret);
        }
        Ok(())
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
        for s in &c.body {
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
        self.warnings.push("BIND form not yet supported".into());
        Ok(())
    }

    // ------------------- expressions -------------------

    fn lower_expr(&mut self, e: &Expression, code: &mut Vec<u8>) -> Result<(), String> {
        match e {
            Expression::Literal(l) => self.lower_literal(l, code),
            Expression::Identifier(id) => {
                let name_lower = id.name.to_lowercase();
                if self.component_instance_names.contains_key(&name_lower) {
                    let cs = self.module.add_const(Const::Str(id.name.clone()));
                    emit(code, Op::LoadConst);
                    push_u32(code, cs);
                } else if let Some(slot) = self.scope.get(&id.name) {
                    emit(code, Op::LoadLocal); push_u16(code, slot);
                } else {
                    let s = self.module.add_string(&id.name);
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
            Expression::Unary(u) => {
                self.lower_expr(&u.operand, code)?;
                match u.operator {
                    UnaryOperator::Negate => emit(code, Op::Neg),
                    UnaryOperator::Not => emit(code, Op::Not),
                    UnaryOperator::Positive => {} // no-op
                }
                Ok(())
            }
            Expression::FunctionCall(fc) => {
                // Check if this is a variant array/list subscript indexing:
                // callee is an identifier, not a defined function, and is a variable.
                if let Expression::Identifier(id) = fc.callee.as_ref() {
                    let name_lower = id.name.to_lowercase();
                    let is_local = self.scope.get(&id.name).is_some();
                    let is_global = self.globals.contains(&name_lower);
                    if (is_local || is_global) && fc.args.len() == 1 && !self.fn_indices.contains_key(&id.name) {
                        let synth = ArrayAccessExpression {
                            span: fc.span.clone(),
                            array: fc.callee.clone(),
                            indices: fc.args.clone(),
                        };
                        return self.lower_expr(&Expression::ArrayAccess(synth), code);
                    }
                }

                for a in &fc.args { self.lower_expr(a, code)?; }
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
                        return Ok(());
                    }
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
                if let Expression::Identifier(obj) = &*m.object {
                    let id_s = self.module.add_string(&obj.name);
                    let nm_s = self.module.add_string(&m.member);
                    emit(code, Op::GetProp);
                    push_u32(code, id_s); push_u32(code, nm_s);
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
                if a.indices.len() != 1 {
                    return Err("multi-dim AGet not yet supported".into());
                }
                self.lower_expr(&a.array, code)?;
                self.lower_expr(&a.indices[0], code)?;
                emit(code, Op::AGet);
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
        | "RCOOLBTN" | "ROVALBTN"
        | "RJSON"
        // Web-exclusive components
        | "RWEBVIEW" | "RDOM" | "RJAVASCRIPT" | "RWEBSTORAGE"
        | "RWEBAUDIO" | "RWEBVIDEO" | "RWEBNOTIFICATION" | "RWEBGEOLOCATION"
        | "RROUTER"
    )
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
                    for decl in &d.declarators {
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
        let span = match stmt {
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
        };

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

fn short_name(s: &Statement) -> &'static str {
    match s {
        Statement::Assignment(_) => "Assignment",
        Statement::Bind(_) => "Bind",
        Statement::Call(_) => "Call",
        Statement::Close(_) => "Close",
        Statement::Comment(_) => "Comment",
        Statement::Const(_) => "Const",
        Statement::Create(_) => "Create",
        Statement::Declare(_) => "Declare",
        Statement::Dim(_) => "Dim",
        Statement::Directive(_) => "Directive",
        Statement::DoLoop(_) => "DoLoop",
        Statement::Exit(_) => "Exit",
        Statement::For(_) => "For",
        Statement::Function(_) => "Function",
        Statement::If(_) => "If",
        Statement::Import(_) => "Import",
        Statement::Input(_) => "Input",
        Statement::Line(_) => "Line",
        Statement::Open(_) => "Open",
        Statement::Print(_) => "Print",
        Statement::PrintHash(_) => "PrintHash",
        Statement::Return(_) => "Return",
        Statement::Seek(_) => "Seek",
        Statement::SelectCase(_) => "SelectCase",
        Statement::Subroutine(_) => "Subroutine",
        Statement::Type(_) => "Type",
        Statement::While(_) => "While",
        Statement::With(_) => "With",
        Statement::WriteHash(_) => "WriteHash",
        Statement::RustBlock(_) => "RustBlock",
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
        rapidr_parser::parse_tokens(&toks)
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
        let src = "SUB greet(name AS STRING)\nPRINT \"Hi \", name\nEND SUB\nCALL greet(\"world\")";
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

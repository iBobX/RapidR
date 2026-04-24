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

use std::collections::HashMap;

use rapidr_ast::{
    AssignmentStatement, BinaryOperator, BindStatement, CallStatement, CreateStatement,
    DoLoopStatement, Expression, ForStatement, FunctionStatement, IfStatement, Literal,
    LiteralValue, Parameter, PrintStatement, Program, ReturnStatement, Statement,
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
    let mut bcgen = Bcgen::new();
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
    /// Active "WITH object" name (or None). Bare member-access on the
    /// implicit object is not yet a separate AST node, so this is reserved.
    _with_object: Option<String>,
    /// Stack of (continue_target, breaks_to_patch) for loops, used by EXIT.
    loop_stack: Vec<LoopCtx>,
    /// CREATE-block instance name stack (for nested CREATE).
    create_stack: Vec<String>,
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
            _with_object: None,
            loop_stack: Vec::new(),
            create_stack: Vec::new(),
        }
    }

    // ------------------- top-level driver -------------------

    fn compile_program(&mut self, program: &Program) -> Result<(), String> {
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
        for stmt in &program.statements {
            if matches!(stmt, Statement::Subroutine(_) | Statement::Function(_)) {
                continue;
            }
            self.lower_stmt(stmt, &mut main_code, &mut main_lines)?;
        }
        emit(&mut main_code, Op::Halt);
        let main_locals = self.scope.next_slot as u32;
        self.scope = saved_scope;
        let f = &mut self.module.functions[main_idx as usize];
        f.code = main_code;
        f.line_info = main_lines;
        f.n_locals = main_locals;

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
        self.scope = saved_scope;
        let f = &mut self.module.functions[idx as usize];
        f.code = code;
        f.line_info = lines;
        f.n_locals = n_locals;
        Ok(())
    }

    // ------------------- statements -------------------

    fn lower_stmt(
        &mut self,
        stmt: &Statement,
        code: &mut Vec<u8>,
        lines: &mut Vec<(u32, u32)>,
    ) -> Result<(), String> {
        let line = stmt_line(stmt);
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
                self.lower_expr(&c.value, code)?;
                let s = self.module.add_string(&c.name);
                emit(code, Op::StoreGlobal);
                push_u32(code, s);
            }
            Statement::Dim(d) => {
                // Declare locals; initial value Null is already the default.
                for decl in &d.declarators {
                    self.scope.declare(&decl.name);
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
        // Inside a CREATE block, a bare-identifier LHS is a property of the
        // current instance — emit SetProp instead of StoreLocal/StoreGlobal.
        if let Some(inst) = self.create_stack.last().cloned() {
            if let Expression::Identifier(lhs_id) = &a.target {
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
            _ => Err("invalid assignment target".into()),
        }
    }

    fn lower_call_stmt(&mut self, c: &CallStatement, code: &mut Vec<u8>) -> Result<(), String> {
        // Push args.
        for a in &c.args {
            self.lower_expr(a, code)?;
        }
        let argc = c.args.len() as u8;
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
        // var = start
        let var_slot = self.scope.declare(&f.variable);
        self.lower_expr(&f.start, code)?;
        emit(code, Op::StoreLocal); push_u16(code, var_slot);

        // end and step into temp slots so they evaluate once.
        let end_slot = self.scope.declare(&format!("__for_end_{}", var_slot));
        self.lower_expr(&f.end, code)?;
        emit(code, Op::StoreLocal); push_u16(code, end_slot);
        let step_slot = self.scope.declare(&format!("__for_step_{}", var_slot));
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
        emit(code, Op::LoadLocal); push_u16(code, var_slot);
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
        emit(code, Op::LoadLocal); push_u16(code, var_slot);
        emit(code, Op::LoadLocal); push_u16(code, step_slot);
        emit(code, Op::Add);
        emit(code, Op::StoreLocal); push_u16(code, var_slot);
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
        let kind_s = self.module.add_string(&c.type_name);
        let id_s = self.module.add_string(&c.name);
        emit(code, Op::CreateComp);
        push_u32(code, kind_s); push_u32(code, id_s);
        emit(code, Op::Pop); // discard returned reference for now
        self.create_stack.push(c.name.clone());
        for s in &c.body {
            self.lower_stmt(s, code, lines)?;
        }
        self.create_stack.pop();
        Ok(())
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
                if let Some(slot) = self.scope.get(&id.name) {
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
                for a in &fc.args { self.lower_expr(a, code)?; }
                let argc = fc.args.len() as u8;
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

fn stmt_line(_s: &Statement) -> u32 {
    // TextSpan currently only carries byte offsets; line numbers will be
    // added once the diagnostics layer exposes them.
    0
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

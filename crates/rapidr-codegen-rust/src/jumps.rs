//! Line labels, GOTO, GOSUB and RETURN in native builds.
//!
//! Rust has no goto, so a routine (the main program, a SUB or a FUNCTION)
//! that has jump targets or GOSUBs becomes a state machine:
//!
//! ```text
//! let mut __pc: usize = 0;
//! let mut __gosub: Vec<usize> = Vec::new();
//! 'sm: loop {
//!     match __pc {
//!         0 => { …statements up to the first label…; __pc = 1; continue 'sm; }
//!         1 => { … }          // `Label:`
//!         _ => break 'sm,
//!     }
//! }
//! ```
//!
//! `GOTO L` is `{ __pc = L; continue 'sm; }`, which Rust allows from any
//! depth inside the loop, so ordinary statements stay ordinary structured
//! Rust. Only statements that *contain* a jump target or a GOSUB (whose
//! return point must be a state too) are flattened into states: IF, FOR,
//! WHILE and DO. `EXIT FOR/WHILE/DO` out of a flattened loop jumps to its
//! exit state; a bare `RETURN` pops the GOSUB stack, or leaves the routine
//! when it's empty (as the bytecode VM does).
//!
//! Variables a routine DIMs are declared before the loop (a `let` inside
//! one state wouldn't be visible in the next), and DIM becomes an
//! assignment.

use std::collections::HashMap;
use std::fmt::Write;

use rapidr_ast::*;

use super::{to_snake, RustCodegen};

/// The state machine being emitted for the current routine.
#[derive(Debug, Default)]
pub(crate) struct StateMachine {
    /// Jump targets in this routine (lowercase) → state.
    labels: HashMap<String, usize>,
    next_state: usize,
    /// Declarations to put before the loop (`let mut x = v_null();`).
    hoisted: Vec<String>,
    /// Loop temporaries used by flattened FOR loops.
    for_counter: usize,
}

/// Does this statement contain (at any depth) a label some GOTO/GOSUB
/// jumps to, or a GOSUB? Those can't live inside structured Rust.
fn needs_states(stmt: &Statement, targets: &std::collections::HashSet<String>) -> bool {
    let mut found = false;
    rapidr_ast::walk(
        std::slice::from_ref(stmt),
        &mut |s| match s {
            Statement::Label(l) if targets.contains(&l.name.to_lowercase()) => found = true,
            Statement::Gosub(_) => found = true,
            _ => {}
        },
        &mut |_| {},
    );
    found
}

impl RustCodegen {
    /// Whether a routine body needs the state machine (it has GOTO, GOSUB
    /// or a jump target outside nested SUB/FUNCTION definitions).
    pub(crate) fn routine_needs_states(&self, body: &[Statement]) -> bool {
        body.iter().any(|s| match s {
            Statement::Subroutine(_) | Statement::Function(_) | Statement::Type(_) | Statement::Declare(_) => false,
            Statement::Goto(_) => true,
            s => needs_states(s, &self.jump_targets) || contains_goto(s),
        })
    }

    /// True while a routine is emitted as a state machine.
    pub(crate) fn in_state_machine(&self) -> bool {
        self.state_machine.is_some()
    }

    /// Emits a routine body as a state machine; `after` runs when it ends
    /// (falling off the end, `END SUB`, or a RETURN with no GOSUB pending).
    pub(crate) fn emit_state_machine(&mut self, body: &[Statement]) {
        let mut sm = StateMachine { next_state: 1, ..Default::default() };
        // Every label in this routine (not in nested SUB definitions) gets
        // a state; which ones are jumped to decides flattening.
        let mut labels = Vec::new();
        for s in body {
            if !matches!(s, Statement::Subroutine(_) | Statement::Function(_)) {
                collect_labels(std::slice::from_ref(s), &mut labels);
            }
        }
        for name in labels {
            if !sm.labels.contains_key(&name) {
                let id = sm.next_state;
                sm.next_state += 1;
                sm.labels.insert(name, id);
            }
        }
        let outer = std::mem::replace(&mut self.state_machine, Some(sm));

        // Emit the states into a buffer: the hoisted declarations are only
        // known afterwards and must come first.
        let saved_output = std::mem::take(&mut self.output);
        let saved_indent = self.indent;
        self.indent += 3;
        for s in body {
            if matches!(s, Statement::Subroutine(_) | Statement::Function(_) | Statement::Type(_) | Statement::Declare(_)) {
                continue;
            }
            self.emit_flat(s);
        }
        let states = std::mem::replace(&mut self.output, saved_output);
        self.indent = saved_indent;
        let sm = std::mem::replace(&mut self.state_machine, outer).expect("state machine");

        for decl in &sm.hoisted {
            self.line(decl);
        }
        self.line("let mut __pc: usize = 0;");
        self.line("let mut __gosub: Vec<usize> = Vec::new();");
        self.line("'sm: loop {");
        self.indent += 1;
        self.line("match __pc {");
        self.indent += 1;
        self.line("0 => {");
        self.output.push_str(&states);
        self.indent += 1;
        self.line("break 'sm;");
        self.indent -= 1;
        self.line("}");
        self.line("_ => break 'sm,");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
    }

    fn new_state(&mut self) -> usize {
        let sm = self.state_machine.as_mut().expect("state machine");
        let id = sm.next_state;
        sm.next_state += 1;
        id
    }

    /// Ends the current state (falling through to `id`) and starts `id`.
    fn start_state(&mut self, id: usize) {
        self.jump(id);
        self.indent -= 1;
        self.line("}");
        let _ = writeln!(self.output, "{}{id} => {{", "    ".repeat(self.indent));
        self.indent += 1;
    }

    fn jump(&mut self, id: usize) {
        self.write_indent();
        let _ = writeln!(self.output, "__pc = {id}; continue 'sm;");
    }

    /// Adds a declaration before the state machine's loop.
    pub(crate) fn hoist(&mut self, decl: String) {
        if let Some(sm) = self.state_machine.as_mut() {
            if !sm.hoisted.contains(&decl) {
                sm.hoisted.push(decl);
            }
        }
    }

    /// The state of jump target `label`, if this routine has it.
    fn label_state(&self, label: &str) -> Option<usize> {
        self.state_machine.as_ref()?.labels.get(&label.to_lowercase()).copied()
    }

    fn missing_label(&mut self, label: &str) {
        let place = match &self.current_routine_name {
            Some(name) => format!("SUB/FUNCTION {name}"),
            None => "the main program".to_string(),
        };
        self.write_indent();
        let _ = writeln!(self.output, "compile_error!(\"Label '{label}' not found in {place}\");");
    }

    /// `GOTO label` (anywhere inside the state machine).
    pub(crate) fn emit_goto(&mut self, j: &JumpStatement) {
        match self.label_state(&j.label) {
            Some(id) => {
                self.write_indent();
                let _ = writeln!(self.output, "{{ __pc = {id}; continue 'sm; }}");
            }
            None => self.missing_label(&j.label),
        }
    }

    /// A bare `RETURN`: back from a GOSUB, or out of the routine.
    pub(crate) fn emit_gosub_return(&mut self) {
        self.line("if let Some(__back) = __gosub.pop() { __pc = __back; continue 'sm; }");
        self.line("break 'sm;");
    }

    /// Emits one statement of a state-machine routine, flattening it into
    /// states when it contains a jump target or a GOSUB.
    fn emit_flat(&mut self, stmt: &Statement) {
        let flatten = needs_states(stmt, &self.jump_targets);
        match stmt {
            Statement::Label(l) => match self.label_state(&l.name) {
                Some(id) if self.jump_targets.contains(&l.name.to_lowercase()) => self.start_state(id),
                _ => self.emit_statement(stmt),
            },
            Statement::Gosub(j) => match self.label_state(&j.label) {
                Some(id) => {
                    let back = self.new_state();
                    self.write_indent();
                    let _ = writeln!(self.output, "__gosub.push({back}); __pc = {id}; continue 'sm;");
                    self.start_state(back);
                }
                None => self.missing_label(&j.label),
            },
            Statement::If(i) if flatten => self.flat_if(i),
            Statement::For(f) if flatten => self.flat_for(f),
            Statement::While(w) if flatten => {
                let (check, exit) = (self.new_state(), self.new_state());
                self.start_state(check);
                let cond = self.expr_to_string(&w.condition);
                self.write_indent();
                let _ = writeln!(self.output, "if !({cond}).to_bool() {{ __pc = {exit}; continue 'sm; }}");
                self.flat_loop_body("WHILE", exit, &w.body);
                self.jump(check);
                self.start_state(exit);
            }
            Statement::DoLoop(d) if flatten => self.flat_do(d),
            Statement::SelectCase(c) if flatten => self.flat_select(c),
            // (WITH and CREATE bodies are plain statements with a
            // compile-time context: their states follow one another)
            Statement::With(w) if flatten => self.flat_body(&rapidr_ast::resolve_with_body(&w.body, &w.object)),
            Statement::Create(c) if flatten => self.flat_create(c),
            _ => self.emit_statement(stmt),
        }
    }

    /// A CREATE holding a label or GOSUB: as `emit_create`, its body in
    /// states.
    fn flat_create(&mut self, c: &CreateStatement) {
        let name = to_snake(&c.name);
        let type_upper = c.type_name.to_uppercase();
        self.write_indent();
        let _ = writeln!(self.output, "rp_create_component(\"{name}\", \"{type_upper}\");");
        if let Some(parent) = self.create_stack.last().cloned() {
            self.write_indent();
            let _ = writeln!(self.output, "rp_comp_set(\"{name}\", \"parent\", v_str(\"{parent}\"));");
        }
        self.create_stack.push(name.clone());
        self.with_component_stack.push(name.clone());
        self.flat_body(&c.body);
        self.with_component_stack.pop();
        self.create_stack.pop();
        if type_upper == "RTIMER" {
            self.write_indent();
            let _ = writeln!(self.output, "gui_register_timer(\"{name}\");");
        }
    }

    fn flat_body(&mut self, body: &[Statement]) {
        for s in body {
            self.emit_flat(s);
        }
    }

    fn flat_if(&mut self, i: &IfStatement) {
        let end = self.new_state();
        let branches = std::iter::once((&i.condition, &i.then_body))
            .chain(i.elseif_branches.iter().map(|b| (&b.condition, &b.body)));
        for (cond, body) in branches {
            let next = self.new_state();
            let cond = self.expr_to_string(cond);
            self.write_indent();
            let _ = writeln!(self.output, "if !({cond}).to_bool() {{ __pc = {next}; continue 'sm; }}");
            self.flat_body(body);
            self.jump(end);
            self.start_state(next);
        }
        self.flat_body(&i.else_body);
        self.start_state(end);
    }

    /// `SELECT CASE` holding a label or GOSUB: the selector is kept in a
    /// hoisted variable and each branch is a state, like `flat_if`.
    fn flat_select(&mut self, c: &SelectCaseStatement) {
        let n = {
            let sm = self.state_machine.as_mut().expect("state machine");
            sm.for_counter += 1;
            sm.for_counter
        };
        let sel = format!("__sm_select_{n}");
        self.hoist(format!("let mut {sel} = v_null();"));
        let value = self.owned_expr(&c.expression);
        self.write_indent();
        let _ = writeln!(self.output, "{sel} = {value};");
        let end = self.new_state();
        for case in &c.cases {
            let next = self.new_state();
            let test = self.case_conditions(&sel, case).join(" || ");
            self.write_indent();
            let _ = writeln!(self.output, "if !({test}) {{ __pc = {next}; continue 'sm; }}");
            self.flat_body(&case.body);
            self.jump(end);
            self.start_state(next);
        }
        self.flat_body(&c.case_else);
        self.start_state(end);
    }

    fn flat_for(&mut self, f: &ForStatement) {
        let n = {
            let sm = self.state_machine.as_mut().expect("state machine");
            sm.for_counter += 1;
            sm.for_counter
        };
        let (end_tmp, step_tmp) = (format!("__sm_for_end_{n}"), format!("__sm_for_step_{n}"));
        self.hoist(format!("let mut {end_tmp} = v_null();"));
        self.hoist(format!("let mut {step_tmp} = v_null();"));
        let start = self.owned_expr(&f.start);
        let end = self.owned_expr(&f.end);
        let step = f.step.as_ref().map(|e| self.owned_expr(e)).unwrap_or_else(|| "v_int(1)".into());
        let var = to_snake(&f.variable);
        let global = self.is_global_scalar(&f.variable);
        // A typed main-program counter (typed::analyze_globals) lives in its
        // static; the start is already converted and the steps aren't (as in
        // the VM), so it's stored as the plain number.
        let typed = self.typed_var(&f.variable);
        let get = match &typed {
            Some((k, read)) if *k == crate::typed::Kind::Double => format!("v_dbl({read})"),
            Some((_, read)) => format!("v_int({read})"),
            None if global => format!("gv(\"{var}\")"),
            None => var.clone(),
        };
        let store = typed.as_ref().map(|(k, _)| {
            let write = self.typed_write(&f.variable, "\u{0}");
            let (before, after) = write.split_once('\u{0}').unwrap_or_default();
            let to = if *k == crate::typed::Kind::Double { "to_f64" } else { "to_i64" };
            (before.to_string(), after.to_string(), to)
        });
        let assign = |value: &str| match &store {
            Some((before, after, to)) => format!("{before}({value}).{to}(){after}"),
            None if global => format!("gs(\"{var}\", {value});"),
            None => format!("{var} = {value};"),
        };
        self.write_indent();
        let _ = writeln!(self.output, "{end_tmp} = {end};");
        self.write_indent();
        let _ = writeln!(self.output, "{step_tmp} = {step};");
        self.write_indent();
        let _ = writeln!(self.output, "{}", assign(&start));
        let (check, exit) = (self.new_state(), self.new_state());
        self.start_state(check);
        self.write_indent();
        let _ = writeln!(
            self.output,
            "if !(if {step_tmp}.rp_ge(&v_int(0)).to_bool() {{ {get}.rp_le(&{end_tmp}) }} else {{ {get}.rp_ge(&{end_tmp}) }}).to_bool() {{ __pc = {exit}; continue 'sm; }}"
        );
        self.flat_loop_body("FOR", exit, &f.body);
        self.write_indent();
        let _ = writeln!(self.output, "{}", assign(&format!("&{get} + &{step_tmp}")));
        self.jump(check);
        self.start_state(exit);
    }

    fn flat_do(&mut self, d: &DoLoopStatement) {
        let (top, exit) = (self.new_state(), self.new_state());
        self.start_state(top);
        let test = |gen: &mut Self, cond: &Option<Expression>| {
            cond.as_ref().map(|c| {
                let c = gen.expr_to_string(c);
                if d.is_until { format!("({c}).to_bool()") } else { format!("!({c}).to_bool()") }
            })
        };
        if d.pre_condition {
            if let Some(stop) = test(self, &d.condition) {
                self.write_indent();
                let _ = writeln!(self.output, "if {stop} {{ __pc = {exit}; continue 'sm; }}");
            }
        }
        self.flat_loop_body("DO", exit, &d.body);
        if !d.pre_condition {
            if let Some(stop) = test(self, &d.condition) {
                self.write_indent();
                let _ = writeln!(self.output, "if {stop} {{ __pc = {exit}; continue 'sm; }}");
            }
        }
        self.jump(top);
        self.start_state(exit);
    }

    /// A flattened loop's body: `EXIT kind` inside it jumps to `exit`.
    fn flat_loop_body(&mut self, kind: &'static str, exit: usize, body: &[Statement]) {
        self.loop_labels.push((kind, format!("@{exit}")));
        self.flat_body(body);
        self.loop_labels.pop();
    }
}

fn contains_goto(stmt: &Statement) -> bool {
    let mut found = false;
    rapidr_ast::walk(std::slice::from_ref(stmt), &mut |s| found |= matches!(s, Statement::Goto(_)), &mut |_| {});
    found
}

fn collect_labels(stmts: &[Statement], out: &mut Vec<String>) {
    rapidr_ast::walk(
        stmts,
        &mut |s| {
            if let Statement::Label(l) = s {
                out.push(l.name.to_lowercase());
            }
        },
        &mut |_| {},
    );
}

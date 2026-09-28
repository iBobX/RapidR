//! `Fill M(), n` / `Sort(Names(), count)`: an array's name with empty
//! parentheses passes the whole array (arrays are shared with the callee,
//! `SUB Fill (A() AS INTEGER, n AS INTEGER)`). The parser reads `M()` as a
//! call of a function `M`; here, where `M` is a declared array (a DIM with
//! dimensions, or an array parameter) and no SUB / FUNCTION has that name,
//! it becomes the array itself, so both backends see the same thing.

use std::collections::HashSet;

use crate::*;

type Names = HashSet<String>;

fn arrays_declared(stmts: &[Statement], into: &mut Names) {
    walk(
        stmts,
        &mut |s| {
            if let Statement::Dim(d) = s {
                for v in d.declarators.iter().filter(|v| !v.dimensions.is_empty()) {
                    into.insert(v.name.to_ascii_lowercase());
                }
            }
        },
        &mut |_| {},
    );
}

fn rewrite(stmts: &mut [Statement], arrays: &Names, routines: &Names) {
    walk_expressions_mut(stmts, true, &mut |e| {
        let Expression::FunctionCall(fc) = e else { return };
        if !fc.args.is_empty() {
            return;
        }
        let Expression::Identifier(id) = fc.callee.as_ref() else { return };
        let name = id.name.to_ascii_lowercase();
        if arrays.contains(&name) && !routines.contains(&name) {
            *e = Expression::Identifier(id.clone());
        }
    });
}

/// Rewrites every `Array()` of `program` into `Array` (see the module docs).
pub fn lower(program: &Program) -> Program {
    let mut routines = Names::new();
    let mut main: Vec<Statement> = Vec::new();
    for s in &program.statements {
        match s {
            Statement::Subroutine(r) => {
                routines.insert(r.name.to_ascii_lowercase());
            }
            Statement::Function(f) => {
                routines.insert(f.name.to_ascii_lowercase());
            }
            other => main.push(other.clone()),
        }
    }
    let mut globals = Names::new();
    arrays_declared(&main, &mut globals);
    let mut program = program.clone();
    for stmt in program.statements.iter_mut() {
        match stmt {
            Statement::Subroutine(r) => {
                let mut scope = globals.clone();
                scope.extend(r.params.iter().filter(|p| p.is_array).map(|p| p.name.to_ascii_lowercase()));
                arrays_declared(&r.body, &mut scope);
                rewrite(&mut r.body, &scope, &routines);
            }
            Statement::Function(f) => {
                let mut scope = globals.clone();
                scope.extend(f.params.iter().filter(|p| p.is_array).map(|p| p.name.to_ascii_lowercase()));
                arrays_declared(&f.body, &mut scope);
                rewrite(&mut f.body, &scope, &routines);
            }
            Statement::Type(_) => {}
            other => rewrite(std::slice::from_mut(other), &globals, &routines),
        }
    }
    program
}

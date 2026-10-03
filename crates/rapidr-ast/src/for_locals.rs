//! A FOR loop's variable inside a SUB or FUNCTION that the routine doesn't
//! declare is the routine's own (a local), not a global: two SUBs looping
//! `FOR i = …` don't move each other's counters (RapidQ's games do this —
//! `blocks.bas`' `initstr` loops over columns calling `displaycol`, which
//! loops over rows, both on `i`). The routine gets a DIM of it at its top —
//! typed by its suffix, else DOUBLE, an undeclared variable's type — before
//! either backend sees the program, so both keep it local alike.

use std::collections::HashSet;

use crate::*;

fn key(name: &str) -> String {
    strip_type_suffix(name).to_ascii_lowercase()
}

/// The routine-level DIMs FOR loop variables need (see the module docs).
pub fn lower(program: &Program) -> Program {
    let mut program = program.clone();
    for stmt in program.statements.iter_mut() {
        let (params, body) = match stmt {
            Statement::Subroutine(r) => (&r.params, &mut r.body),
            Statement::Function(f) => (&f.params, &mut f.body),
            _ => continue,
        };
        // What the routine declares itself.
        let mut declared: HashSet<String> = params.iter().map(|p| key(&p.name)).collect();
        let mut loops: Vec<(String, TextSpan)> = Vec::new();
        walk(
            body,
            &mut |s| match s {
                Statement::Dim(d) => d.declarators.iter().for_each(|v| {
                    declared.insert(key(&v.name));
                }),
                Statement::Const(c) => {
                    declared.insert(key(&c.name));
                }
                Statement::For(f) => loops.push((f.variable.clone(), f.span)),
                _ => {}
            },
            &mut |_| {},
        );
        let mut added: HashSet<String> = HashSet::new();
        let mut dims = Vec::new();
        for (var, span) in loops {
            let k = key(&var);
            // (a field — `FOR obj.x` — or one already declared: as it is)
            if var.contains('.') || declared.contains(&k) || !added.insert(k) {
                continue;
            }
            dims.push(Statement::Dim(DimStatement {
                span,
                declarators: vec![VariableDeclarator { span, name: var.clone(), dimensions: Vec::new() }],
                type_name: suffix_type(&var).unwrap_or("DOUBLE").to_string(),
                fixed_len: None,
                is_static: false,
                is_redim: false,
            }));
        }
        if !dims.is_empty() {
            body.splice(0..0, dims);
        }
    }
    program
}

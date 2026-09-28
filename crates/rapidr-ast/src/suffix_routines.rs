//! `FUNCTION Day$ (...)` next to `FUNCTION Day (...)`: RapidQ tells routines
//! that differ only by type suffix apart, while both backends key names
//! without the suffix. Where a program defines such a pair, the suffixed
//! ones get their own names (`Day$` → `Day__str`) at the definition, at
//! every call and at the function's result assignments, before either
//! backend sees the program. A program without such a pair is untouched.

use std::collections::{HashMap, HashSet};

use crate::*;

/// The unique name for a suffixed routine name, e.g. `Day$` → `Day__str`.
fn renamed(name: &str) -> Option<String> {
    let tag = match name.chars().last()? {
        '$' => "str",
        '%' => "int",
        '&' => "lng",
        '!' => "sng",
        '#' => "dbl",
        _ => return None,
    };
    let base = &name[..name.len() - 1];
    (!base.is_empty()).then(|| format!("{base}__{tag}"))
}

fn stripped(name: &str) -> String {
    let n = name.to_ascii_lowercase();
    match renamed(&n) {
        Some(_) => n[..n.len() - 1].to_string(),
        None => n,
    }
}

/// Renames the suffixed routines that clash with another routine of the
/// same name (see the module docs).
pub fn lower(program: &Program) -> Program {
    let mut by_key: HashMap<String, HashSet<String>> = HashMap::new();
    for s in &program.statements {
        let name = match s {
            Statement::Subroutine(r) => &r.name,
            Statement::Function(f) => &f.name,
            _ => continue,
        };
        by_key.entry(stripped(name)).or_default().insert(name.to_ascii_lowercase());
    }
    let mut map: HashMap<String, String> = HashMap::new();
    for spellings in by_key.values().filter(|s| s.len() > 1) {
        for raw in spellings {
            if let Some(new) = renamed(raw) {
                map.insert(raw.clone(), new);
            }
        }
    }
    if map.is_empty() {
        return program.clone();
    }
    let mut program = program.clone();
    let rename = |name: &mut String| {
        if let Some(new) = map.get(&name.to_ascii_lowercase()) {
            *name = new.clone();
        }
    };
    for stmt in program.statements.iter_mut() {
        match stmt {
            Statement::Subroutine(r) => {
                rename(&mut r.name);
                rewrite(&mut r.body, &map);
            }
            Statement::Function(f) => {
                rename(&mut f.name);
                rewrite(&mut f.body, &map);
            }
            Statement::Type(_) => {}
            other => rewrite(std::slice::from_mut(other), &map),
        }
    }
    program
}

fn rewrite(stmts: &mut [Statement], map: &HashMap<String, String>) {
    walk_expressions_mut(stmts, true, &mut |e| {
        if let Expression::Identifier(id) = e {
            if let Some(new) = map.get(&id.name.to_ascii_lowercase()) {
                id.name = new.clone();
            }
        }
    });
}

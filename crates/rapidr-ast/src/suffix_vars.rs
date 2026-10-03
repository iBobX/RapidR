//! `i%` next to `i$`: in RapidQ, as in QBasic, variables whose names differ
//! only by type suffix are different variables (a loop counter `i%` and a
//! string `i$` side by side is common), while both backends key variables
//! without the suffix. Where a program uses one name with two or more
//! suffixes, each suffixed spelling gets its own name — keeping its suffix,
//! which still gives an undeclared variable its type (`i$` →
//! `i__str$`) — at its declarations, parameters, FOR loops, constants and
//! every use, before either backend sees the program. A program that uses a
//! name with one suffix at most is untouched (`DIM i AS INTEGER` and `i%`
//! stay one variable). A variable named like a routine but spelled with
//! another suffix (`day&` inside `FUNCTION Day`) is its own too — not the
//! function's result, which keeps the routine's own spelling.

use std::collections::{HashMap, HashSet};

use crate::*;

/// A spelling's suffix and the tag its new name carries.
fn suffix_of(name: &str) -> Option<(&str, &'static str)> {
    let base = strip_type_suffix(name);
    let suffix = &name[base.len()..];
    let tag = match suffix {
        "$" => "str",
        "%" => "int",
        "&" => "lng",
        "!" => "sng",
        "#" => "dbl",
        "?" => "byt",
        "??" => "wrd",
        "???" => "dwd",
        _ => return None,
    };
    (!base.is_empty()).then_some((suffix, tag))
}

fn renamed(name: &str) -> Option<String> {
    let (suffix, tag) = suffix_of(name)?;
    let base = &name[..name.len() - suffix.len()];
    Some(format!("{base}__{tag}{suffix}"))
}

/// Renames the suffixed variables that share a name with another suffix
/// (see the module docs).
pub fn lower(program: &Program) -> Program {
    // The routines' spellings (`day$`, `day`) and their names (`day`).
    let routine_spellings: HashSet<String> = program
        .statements
        .iter()
        .filter_map(|s| match s {
            Statement::Subroutine(r) => Some(r.name.to_ascii_lowercase()),
            Statement::Function(f) => Some(f.name.to_ascii_lowercase()),
            _ => None,
        })
        .collect();
    let routines: HashSet<String> = routine_spellings.iter().map(|n| strip_type_suffix(n).to_string()).collect();
    // Every spelling a variable is declared or stored with.
    let mut forms: HashSet<String> = HashSet::new();
    let mut add = |name: &str| {
        forms.insert(name.to_ascii_lowercase());
    };
    for s in &program.statements {
        match s {
            Statement::Subroutine(r) => r.params.iter().for_each(|p| add(&p.name)),
            Statement::Function(f) => f.params.iter().for_each(|p| add(&p.name)),
            _ => {}
        }
    }
    let outside_types: Vec<Statement> = program.statements.iter().filter(|s| !matches!(s, Statement::Type(_))).cloned().collect();
    walk(
        &outside_types,
        &mut |s| match s {
            Statement::Dim(d) => d.declarators.iter().for_each(|v| add(&v.name)),
            Statement::For(f) => add(&f.variable),
            Statement::Const(c) => add(&c.name),
            Statement::Assignment(a) => match &a.target {
                Expression::Identifier(i) => add(&i.name),
                Expression::FunctionCall(c) => {
                    if let Expression::Identifier(i) = c.callee.as_ref() {
                        add(&i.name);
                    }
                }
                _ => {}
            },
            Statement::Subroutine(r) => r.params.iter().for_each(|p| add(&p.name)),
            Statement::Function(f) => f.params.iter().for_each(|p| add(&p.name)),
            _ => {}
        },
        &mut |_| {},
    );
    // Names spelled with two or more suffixes; a routine's name spelled
    // with a suffix not its own.
    let mut by_base: HashMap<String, HashSet<String>> = HashMap::new();
    let mut map: HashMap<String, String> = HashMap::new();
    for f in &forms {
        if suffix_of(f).is_none() {
            continue;
        }
        let base = strip_type_suffix(f).to_string();
        if routines.contains(&base) {
            if !routine_spellings.contains(f) {
                if let Some(n) = renamed(f) {
                    map.insert(f.clone(), n);
                }
            }
        } else {
            by_base.entry(base).or_default().insert(f.clone());
        }
    }
    map.extend(by_base.values().filter(|s| s.len() > 1).flatten().filter_map(|f| renamed(f).map(|n| (f.clone(), n))));
    if map.is_empty() {
        return program.clone();
    }
    let mut program = program.clone();
    for stmt in program.statements.iter_mut() {
        if !matches!(stmt, Statement::Type(_)) {
            rewrite(std::slice::from_mut(stmt), &map);
        }
    }
    program
}

fn rewrite(stmts: &mut [Statement], map: &HashMap<String, String>) {
    let rename = |name: &mut String| {
        if let Some(new) = map.get(&name.to_ascii_lowercase()) {
            *name = new.clone();
        }
    };
    walk_statements_mut(stmts, &mut |s| match s {
        Statement::Dim(d) => d.declarators.iter_mut().for_each(|v| rename(&mut v.name)),
        Statement::For(f) => rename(&mut f.variable),
        Statement::Const(c) => rename(&mut c.name),
        Statement::Subroutine(r) => r.params.iter_mut().for_each(|p| rename(&mut p.name)),
        Statement::Function(f) => f.params.iter_mut().for_each(|p| rename(&mut p.name)),
        _ => {}
    });
    walk_expressions_mut(stmts, true, &mut |e| {
        if let Expression::Identifier(id) = e {
            if let Some(new) = map.get(&id.name.to_ascii_lowercase()) {
                id.name = new.clone();
            }
        }
    });
}

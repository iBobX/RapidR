//! `Result` inside a FUNCTION is the FUNCTION's own value (RapidQ's
//! RESULT), whatever else has that name: a parameter named `Result`, a
//! `DIM Result` of the FUNCTION's or a global `Result` can't be read or set
//! there — RC.EXE compiles them, and in the body `Result` reads and sets
//! the value the FUNCTION returns (`FUNCTION F (Id AS STRING, Result AS
//! STRING)`: `F = Id + Result` adds the result so far, "", not the
//! argument). RC.EXE probes, 2026-10-22 (`function_result_name`).
//!
//! The pass gives such a parameter or local an unreachable name before
//! either backend sees the program, so `Result` in the body is the
//! FUNCTION's value in the interpreter, native builds and the web alike
//! (the argument is still passed: the parameter keeps its place). A SUB's
//! parameter named `Result` is an ordinary parameter (RC.EXE).
//!
//! [`hidden`] says where it happened, for the editor's warning.

use crate::*;

/// The name a FUNCTION's own `Result` parameter or local gets: no program
/// can spell it, so nothing reaches it.
pub const HIDDEN: &str = "__hidden_result";

fn is_result(name: &str) -> bool {
    strip_type_suffix(name).eq_ignore_ascii_case("result")
}

/// The FUNCTIONs of `stmts` (a TYPE's methods too), each passed to `f`.
fn functions(stmts: &mut [Statement], f: &mut dyn FnMut(&mut FunctionStatement)) {
    for s in stmts {
        match s {
            Statement::Function(func) => f(func),
            Statement::Type(t) => functions(&mut t.methods, f),
            _ => {}
        }
    }
}

/// The FUNCTION's parameters and DIMs named `Result`, renamed [`HIDDEN`];
/// the spans of those renamed.
fn hide(func: &mut FunctionStatement) -> Vec<TextSpan> {
    let mut spans = Vec::new();
    for p in func.params.iter_mut().filter(|p| is_result(&p.name)) {
        p.name = HIDDEN.to_string();
        spans.push(p.span);
    }
    walk_statements_mut(&mut func.body, &mut |s| {
        if let Statement::Dim(d) = s {
            for v in d.declarators.iter_mut().filter(|v| is_result(&v.name)) {
                v.name = HIDDEN.to_string();
                spans.push(v.span);
            }
        }
    });
    spans
}

/// The program with every FUNCTION's own `Result` parameter or local made
/// unreachable (see the module docs).
pub fn lower(program: &Program) -> Program {
    let mut program = program.clone();
    functions(&mut program.statements, &mut |f| {
        hide(f);
    });
    program
}

/// Where a FUNCTION has a parameter or a DIM named `Result`: its span and
/// the FUNCTION's name, for a warning (`Result` there is the FUNCTION's
/// value, never that variable).
pub fn hidden(program: &Program) -> Vec<(TextSpan, String)> {
    let mut program = program.clone();
    let mut out = Vec::new();
    functions(&mut program.statements, &mut |f| {
        for span in hide(f) {
            out.push((span, f.name.clone()));
        }
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // (a FUNCTION built by hand: this crate has no parser)
    fn program() -> Program {
        let span = TextSpan::default();
        let param = |name: &str| Parameter { span, name: name.into(), type_name: "STRING".into(), by_ref: false, is_array: false };
        let dim = Statement::Dim(DimStatement {
            span,
            declarators: vec![VariableDeclarator { span, name: "result$".into(), dimensions: Vec::new() }],
            type_name: "STRING".into(),
            fixed_len: None,
            is_static: false,
            is_redim: false,
        });
        Program {
            span,
            statements: vec![Statement::Function(FunctionStatement {
                span,
                name: "F".into(),
                params: vec![param("Id"), param("Result")],
                return_type: Some("STRING".into()),
                body: vec![dim],
            })],
        }
    }

    #[test]
    fn a_functions_own_result_parameter_and_local_are_hidden() {
        let p = program();
        let lowered = lower(&p);
        let Statement::Function(f) = &lowered.statements[0] else { panic!() };
        assert_eq!(f.params.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), ["Id", HIDDEN], "the argument keeps its place");
        let Statement::Dim(d) = &f.body[0] else { panic!() };
        assert_eq!(d.declarators[0].name, HIDDEN);
        assert_eq!(hidden(&p).len(), 2);
    }
}

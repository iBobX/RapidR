//! The reference tells the truth: the check `rapidr lang truth` runs (the CLI
//! also runs the examples that have an `expect`), here its compile half, so a
//! plain `cargo test -p rapidr-lang` keeps every syntax line and every
//! example compiling. The planner itself is unit-tested in src/truth.rs.

use rapidr_lang::truth::{plan, What};

fn compile_error(src: &str) -> Option<String> {
    let tokens = match rapidr_lexer::Lexer::new(src, None).tokenize() {
        Ok(t) => t,
        Err(e) => return Some(e.to_string()),
    };
    let program = match rapidr_parser::parse_tokens(&tokens) {
        Ok(p) => p,
        Err(e) => return Some(e.to_string()),
    };
    rapidr_bcgen::compile_program_with_source(&program, Some(src)).err().map(|e| e.to_string())
}

#[test]
fn every_syntax_line_and_example_compiles() {
    let plan = plan();
    assert!(plan.cases.len() > 300, "{} cases", plan.cases.len());
    let mut failures = Vec::new();
    for c in &plan.cases {
        let mut errors = Vec::new();
        for f in &c.forms {
            match compile_error(f) {
                None => {
                    errors.clear();
                    break;
                }
                Some(e) => errors.push(e),
            }
        }
        if !errors.is_empty() {
            let what = if c.what == What::Syntax { "syntax" } else { "example" };
            failures.push(format!("{} ({what}): `{}`\n    {}", c.entry, c.text.replace('\n', " / "), errors[0].lines().next().unwrap_or("")));
        }
    }
    assert!(failures.is_empty(), "{} failed:\n{}", failures.len(), failures.join("\n"));
}

#[test]
fn nothing_the_planner_found_is_wrong() {
    let plan = plan();
    let bad: Vec<String> = plan.problems.iter().map(|(e, w)| format!("{e}: {w}")).collect();
    assert!(bad.is_empty(), "{} problems:\n{}", bad.len(), bad.join("\n"));
}

//! Two reference entries that once taught a wrong signature (found by
//! Database Studio), kept right until the generated check of every syntax and
//! example (ROADMAP K7) covers them.

use rapidr_lang::{component, statement};

/// A frame's cells are (row, col): the code and its tests take the row first.
#[test]
fn a_frames_cell_and_setcell_take_the_row_first() {
    let frame = component("RDATAFRAME").unwrap();
    for (name, first, second) in [("Cell", "row", "col"), ("SetCell", "row", "col")] {
        let m = frame.method(name).unwrap_or_else(|| panic!("RDataFrame.{name}"));
        let names: Vec<&str> = m.params.iter().map(|p| p.name).collect();
        assert_eq!(&names[..2], [first, second], "RDataFrame.{name} is ({first}, {second}, …)");
    }
}

/// RapidQ has one constant to a CONST, with an optional type: the parser
/// takes no list (`CONST a = 1, b = 2`), so the syntax must not offer one.
#[test]
fn const_syntax_is_one_constant_with_an_optional_type() {
    let c = statement("CONST").unwrap();
    assert_eq!(c.syntax, "CONST name [AS type] = expression");
    assert!(!c.syntax.contains(", …"), "no list form");
    let compiles = |src: &str| {
        let tokens = rapidr_lexer::Lexer::new(src, None).tokenize().unwrap();
        rapidr_parser::parse_tokens(&tokens).is_ok()
    };
    assert!(compiles("CONST a = 1\nCONST b AS LONG = 2\n"));
    assert!(!compiles("CONST a = 1, b = 2\n"), "the parser has no list form");
}

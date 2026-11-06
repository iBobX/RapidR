//! QFormEx (Rapidq2.inc's form) is RapidR's RFormEx (rapidr_ast::formex):
//! the include's TYPE recognised by its public shape and left out, the
//! form written QFormEx marked to behave as the include's, a TYPE of
//! another shape refused with what differs. The include here is RapidR's
//! own stand-in with the same public members (tests/fixtures/QFormEx.inc).

use std::path::Path;

use rapidr_ast::{Expression, LiteralValue, Statement};

fn fixture(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures").join(name)
}

fn parse_text(src: &str) -> rapidr_ast::Program {
    let dir = std::env::temp_dir().join(format!("rapidr-formex-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join(format!("p{}.bas", src.len()));
    std::fs::write(&file, src).unwrap();
    let p = rapidr_parser::parse_file(&file).unwrap_or_else(|e| panic!("{e}"));
    let _ = std::fs::remove_file(&file);
    p
}

#[test]
fn the_includes_qformex_is_rformex_and_classic() {
    let program = rapidr_parser::parse_file(fixture("formex_classic.bas")).unwrap_or_else(|e| panic!("{e}"));
    // (the parser keeps the name as written, for the pass)
    assert!(program.statements.iter().any(|s| matches!(s, Statement::Create(c) if c.type_name == "QFORMEX")));
    let qtype = program.statements.iter().find_map(|s| if let Statement::Type(t) = s { t.name.eq_ignore_ascii_case("QFormEx").then_some(t) } else { None }).expect("the include's TYPE");
    // (its PRIVATE: members aren't its shape)
    assert!(qtype.private.contains(&"busy".to_string()) && qtype.private.contains(&"note".to_string()), "{:?}", qtype.private);
    assert!(rapidr_ast::formex::check(&program).is_empty(), "{:?}", rapidr_ast::formex::check(&program));
    let lowered = rapidr_ast::formex::lower(&program);
    assert!(!lowered.statements.iter().any(|s| matches!(s, Statement::Type(t) if t.name.eq_ignore_ascii_case("QFormEx"))), "the TYPE is left out");
    let create = lowered.statements.iter().find_map(|s| if let Statement::Create(c) = s { Some(c) } else { None }).expect("CREATE Main");
    assert_eq!(create.type_name, "RFORMEX");
    // (its first statement marks it classic)
    let Some(Statement::Assignment(a)) = create.body.first() else { panic!("{:?}", create.body.first()) };
    assert!(matches!(&a.target, Expression::Identifier(i) if i.name == "__classic"));
    assert!(matches!(&a.value, Expression::Literal(l) if l.value == LiteralValue::Integer(1)));
    // (the include's other TYPE stays)
    assert!(lowered.statements.iter().any(|s| matches!(s, Statement::Type(t) if t.name.eq_ignore_ascii_case("FxStandInPoint"))));
}

#[test]
fn rapidr_s_name_is_not_classic_and_dims_are_marked() {
    let p = parse_text("CREATE A AS RFormEx\nEND CREATE\nSUB S\n  DIM B AS QFormEx\n  DIM C AS RFormEx\nEND SUB\n");
    let lowered = rapidr_ast::formex::lower(&p);
    let c = lowered.statements.iter().find_map(|s| if let Statement::Create(c) = s { Some(c) } else { None }).expect("CREATE A");
    assert_eq!(c.type_name, "RFORMEX");
    assert!(c.body.is_empty(), "written RFormEx: no mark");
    let body = lowered.statements.iter().find_map(|s| if let Statement::Subroutine(r) = s { Some(&r.body) } else { None }).unwrap();
    assert!(matches!(&body[0], Statement::Dim(d) if d.type_name == "RFORMEX"));
    assert!(matches!(&body[1], Statement::Assignment(a) if matches!(&a.target, Expression::MemberAccess(m) if m.member == "__classic")), "{:?}", body[1]);
    assert!(matches!(&body[2], Statement::Dim(d) if d.type_name == "RFORMEX"));
    assert_eq!(body.len(), 3);
}

#[test]
fn another_shape_is_refused_not_guessed() {
    let p = parse_text(
        "DECLARE SUB Two (A AS INTEGER, B AS INTEGER)\nTYPE QFormEx EXTENDS QFORM\n  DragEnable AS INTEGER\n  Glow AS INTEGER\n  OnDrag AS EVENT(Two)\n  SUB Blink\n  END SUB\nEND TYPE\nCREATE F AS QFormEx\nEND CREATE\n",
    );
    let errors = rapidr_ast::formex::check(&p);
    assert_eq!(errors.len(), 1, "{errors:?}");
    let message = &errors[0].1;
    for part in ["TYPE QFormEx isn't the QFormEx RapidR knows", "Glow", "OnDrag with 2 parameter(s) (1 there)", "Blink", "MyFormEx"] {
        assert!(message.contains(part), "{part}: {message}");
    }
    // (the program is left as it is: its own TYPE, never guessed at)
    let lowered = rapidr_ast::formex::lower(&p);
    assert!(lowered.statements.iter().any(|s| matches!(s, Statement::Type(t) if t.name == "QFormEx")));
    // (a TYPE QFormEx that extends something else isn't it either)
    let other = parse_text("TYPE QFormEx EXTENDS QPANEL\n  DragEnable AS INTEGER\nEND TYPE\n");
    assert!(rapidr_ast::formex::check(&other)[0].1.contains("EXTENDS RPANEL"), "{:?}", rapidr_ast::formex::check(&other));
}

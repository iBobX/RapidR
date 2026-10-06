//! The semantic model (docs/ide-plan.md, I0): the compiler's own name
//! resolution, exported.

use rapidr_bcgen::semantic::{analyze, Access, SemanticModel, SymbolKind};

fn model(src: &str) -> SemanticModel {
    let tokens = rapidr_lexer::Lexer::new(src, None).tokenize().unwrap();
    let program = rapidr_parser::parse_tokens(&tokens).unwrap();
    analyze(&program, Some(src))
}

fn find(m: &SemanticModel, name: &str, kind: SymbolKind) -> usize {
    m.symbols.iter().position(|s| s.name.eq_ignore_ascii_case(name) && s.kind == kind).unwrap_or_else(|| panic!("no {kind:?} {name}: {:#?}", m.symbols))
}

/// The offsets where a symbol is used, as `(line text, access)`.
fn uses<'a>(m: &SemanticModel, src: &'a str, symbol: usize) -> Vec<(&'a str, Access)> {
    m.references_to(symbol)
        .map(|r| {
            let line_start = src[..r.span.start].rfind('\n').map_or(0, |i| i + 1);
            let line_end = src[r.span.start..].find('\n').map_or(src.len(), |i| r.span.start + i);
            (src[line_start..line_end].trim(), r.access)
        })
        .collect()
}

const PROGRAM: &str = "DIM total AS INTEGER
CONST LIMIT = 10
CREATE Form AS QFORM
    Caption = \"x\"
    CREATE Btn AS QBUTTON
    END CREATE
END CREATE
TYPE TPoint
    X AS INTEGER
    Y AS INTEGER
END TYPE
SUB AddUp (n AS INTEGER)
    DIM i AS INTEGER
    STATIC calls AS LONG
    FOR i = 1 TO n
        total = total + i
    NEXT
    calls = calls + 1
END SUB
FUNCTION Twice (v AS INTEGER) AS INTEGER
    Twice = v * 2
END FUNCTION
AddUp LIMIT
count$ = \"n\"
PRINT Twice(total), count$, TIMER
Form.Caption = STR$(Total)
GOTO done
done:
";

#[test]
fn declarations_types_and_references() {
    let m = model(PROGRAM);
    use Access::*;
    use SymbolKind::*;

    // Every span holds its name.
    for r in &m.references {
        let text = &PROGRAM[r.span.start..r.span.end];
        let s = &m.symbols[r.symbol];
        assert!(text.eq_ignore_ascii_case(&s.name) || rapidr_bcgen::semantic::name_key(text) == rapidr_bcgen::semantic::name_key(&s.name), "{text:?} for {s:?}");
    }

    let total = find(&m, "total", Global);
    assert_eq!(m.symbols[total].ty.as_deref(), Some("INTEGER"));
    assert!(!m.symbols[total].implicit);
    assert_eq!(
        uses(&m, PROGRAM, total),
        [
            ("DIM total AS INTEGER", Declare),
            ("total = total + i", Write),
            ("total = total + i", Read),
            ("PRINT Twice(total), count$, TIMER", Read),
            ("Form.Caption = STR$(Total)", Read),
        ]
    );

    let i = find(&m, "i", Local);
    assert_eq!(m.symbols[i].ty.as_deref(), Some("INTEGER"));
    assert!(matches!(&m.scopes[m.symbols[i].scope].kind, rapidr_bcgen::semantic::ScopeKind::Routine(r) if r == "AddUp"));
    assert_eq!(uses(&m, PROGRAM, i), [("DIM i AS INTEGER", Declare), ("FOR i = 1 TO n", Write), ("total = total + i", Read)]);

    let n = find(&m, "n", Param);
    assert_eq!(uses(&m, PROGRAM, n), [("SUB AddUp (n AS INTEGER)", Declare), ("FOR i = 1 TO n", Read)]);

    let calls = find(&m, "calls", Static);
    assert_eq!(m.symbols[calls].ty.as_deref(), Some("LONG"));
    assert_eq!(uses(&m, PROGRAM, calls).len(), 3);

    let limit = find(&m, "LIMIT", Constant);
    assert_eq!(uses(&m, PROGRAM, limit), [("CONST LIMIT = 10", Declare), ("AddUp LIMIT", Read)]);

    let twice = find(&m, "Twice", Function);
    assert_eq!(m.symbols[twice].ty.as_deref(), Some("INTEGER"));
    assert_eq!(uses(&m, PROGRAM, twice), [("FUNCTION Twice (v AS INTEGER) AS INTEGER", Declare), ("Twice = v * 2", Write), ("PRINT Twice(total), count$, TIMER", Call)]);

    let add_up = find(&m, "AddUp", Sub);
    assert_eq!(uses(&m, PROGRAM, add_up), [("SUB AddUp (n AS INTEGER)", Declare), ("AddUp LIMIT", Call)]);

    let form = find(&m, "Form", Component);
    assert_eq!(m.symbols[form].ty.as_deref(), Some("RFORM"), "one model for Q and R names: the R one");
    assert_eq!(uses(&m, PROGRAM, form), [("CREATE Form AS QFORM", Declare), ("Form.Caption = STR$(Total)", Read)]);
    let btn = find(&m, "Btn", Component);
    assert_eq!(m.symbols[btn].ty.as_deref(), Some("RBUTTON"));

    // An implicit variable: declared by its first use, typed by its suffix.
    let count = find(&m, "count$", Global);
    assert!(m.symbols[count].implicit);
    assert_eq!(m.symbols[count].ty.as_deref(), Some("STRING"));
    assert_eq!(uses(&m, PROGRAM, count), [("count$ = \"n\"", Write), ("PRINT Twice(total), count$, TIMER", Read)]);

    // TIMER is a builtin: no symbol. `Caption` in CREATE is a property: none either.
    assert!(m.symbols.iter().all(|s| !s.name.eq_ignore_ascii_case("TIMER") && !s.name.eq_ignore_ascii_case("Caption")));

    let point = find(&m, "TPoint", Type);
    let x = find(&m, "X", Field);
    assert_eq!(m.scopes[m.symbols[x].scope].parent, Some(0));
    assert_eq!(m.symbols[x].ty.as_deref(), Some("INTEGER"));
    let _ = point;

    let done = find(&m, "done", Label);
    assert_eq!(uses(&m, PROGRAM, done), [("GOTO done", Read), ("done:", Declare)]);

    // Navigation.
    let at = PROGRAM.find("Twice(total)").unwrap() + "Twice(".len();
    assert_eq!(m.symbol_at(at), Some(total));
    let in_sub = m.scope_at(PROGRAM.find("total = total + i").unwrap());
    assert_eq!(m.lookup("I", in_sub), Some(i));
    assert_eq!(m.lookup("TOTAL", in_sub), Some(total));
    assert_eq!(m.lookup("i", 0), None);
    assert!(m.visible(in_sub).contains(&total) && m.visible(in_sub).contains(&n));
}

#[test]
fn a_local_hides_a_global_and_redim_resizes_the_global() {
    let src = "DIM a(5) AS INTEGER\nDIM x AS STRING\nSUB S\n    DIM x AS INTEGER\n    x = 1\n    REDIM a(10) AS INTEGER\n    a(1) = x\nEND SUB\nx = \"g\"\n";
    let m = model(src);
    let global_x = m.symbols.iter().position(|s| s.name == "x" && s.kind == SymbolKind::Global).unwrap();
    let local_x = m.symbols.iter().position(|s| s.name == "x" && s.kind == SymbolKind::Local).unwrap();
    assert_eq!(uses(&m, src, local_x).len(), 3);
    assert_eq!(uses(&m, src, global_x), [("DIM x AS STRING", Access::Declare), ("x = \"g\"", Access::Write)]);
    let a = m.symbols.iter().position(|s| s.name == "a").unwrap();
    assert_eq!(m.symbols[a].kind, SymbolKind::Global);
    assert!(uses(&m, src, a).contains(&("REDIM a(10) AS INTEGER", Access::Write)));
}

#[test]
fn the_rules_are_the_compilers() {
    use rapidr_bcgen::semantic::*;
    let none = NameFacts::default();
    assert_eq!(resolve_name(&none), NameUse::Global);
    assert_eq!(resolve_name(&NameFacts { is_component: true, is_local: true, ..none }), NameUse::Component);
    assert_eq!(resolve_name(&NameFacts { is_local: true, is_bare_builtin: true, ..none }), NameUse::Local);
    assert_eq!(resolve_name(&NameFacts { is_bare_builtin: true, ..none }), NameUse::BareBuiltin);
    // A program's own global wins over a builtin, a constant or a FUNCTION.
    assert_eq!(resolve_name(&NameFacts { is_bare_builtin: true, is_known_global: true, ..none }), NameUse::Global);
    assert_eq!(resolve_name(&NameFacts { is_function: true, is_known_global: true, ..none }), NameUse::Global);
    assert_eq!(resolve_name(&NameFacts { is_function: true, ..none }), NameUse::FunctionCall);
    assert_eq!(dim_target(true, true, false, true), DimTarget::Global);
    assert_eq!(dim_target(false, true, false, true), DimTarget::ResizeGlobal);
    assert_eq!(dim_target(false, false, false, true), DimTarget::Local);
    assert_eq!(for_target(true, false), NameUse::Global);
    assert_eq!(for_target(false, false), NameUse::Local);
}

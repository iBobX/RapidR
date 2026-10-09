//! Find / replace, folding and bracket matching.

use rapidr_editor::{Document, FoldKind, FoldRange, Languages, SearchQuery, Selection, Selections};

fn doc(lang: &str, text: &str) -> Document {
    Document::new(text, Languages::builtin().get(lang).unwrap())
}

#[test]
fn find() {
    let d = doc("plaintext", "Foo foo food\nfoo_bar FOO");
    let q = SearchQuery::literal("foo");
    assert_eq!(d.find_all(&q, false).unwrap(), [0..3, 4..7, 8..11, 13..16, 21..24]);
    let q = SearchQuery::with_options("foo", "case, word");
    assert_eq!(d.find_all(&q, false).unwrap(), vec![4..7]);
    let q = SearchQuery::with_options("FOO", "word");
    assert_eq!(d.find_all(&q, false).unwrap(), [0..3, 4..7, 21..24]);
    let q = SearchQuery::regex(r"^f\w+");
    assert_eq!(d.find_all(&q, false).unwrap(), [0..3, 13..20]);
    // across lines
    let q = SearchQuery::regex(r"food\nfoo");
    assert_eq!(d.find_all(&q, false).unwrap(), vec![8..16]);
    // next / previous, wrapping
    assert_eq!(d.find_next(&SearchQuery::literal("foo"), 22, true).unwrap(), Some(0..3));
    assert_eq!(d.find_next(&SearchQuery::literal("foo"), 4, false).unwrap(), Some(0..3));
    assert_eq!(d.find_next(&SearchQuery::literal("foo"), 0, false).unwrap(), Some(21..24));
    assert!(d.find_all(&SearchQuery::regex("("), false).is_err());
    assert!(d.find_all(&SearchQuery::literal(""), false).is_err());
    // CR LF: `$` is before the CR
    let d = doc("plaintext", "ab\r\ncd");
    assert_eq!(d.find_all(&SearchQuery::regex("b$"), false).unwrap(), vec![1..2]);
}

#[test]
fn find_in_selection_and_incremental() {
    let mut d = doc("plaintext", "a1 a2 a3 a4");
    d.set_selections(Selections::single(Selection::new(3, 8)));
    assert_eq!(d.find_all(&SearchQuery::literal("a"), true).unwrap(), [3..4, 6..7]);
    // incremental: from where the search started, as the pattern grows
    let origin = 4;
    assert_eq!(d.incremental_find(&SearchQuery::literal("a"), origin).unwrap(), Some(6..7));
    assert_eq!(d.incremental_find(&SearchQuery::literal("a1"), origin).unwrap(), Some(0..2));
    assert_eq!(d.selections().primary(), Selection::new(0, 2));
    assert!(d.select_next_match(&SearchQuery::literal("a"), true).unwrap());
    assert_eq!(d.selections().primary(), Selection::new(3, 4));
}

#[test]
fn replace() {
    let mut d = doc("plaintext", "x=1, y=22, z=333");
    let q = SearchQuery::regex(r"(\w)=(\d+)");
    assert_eq!(d.replace_all(&q, "$2:${1}", false, 0).unwrap(), 3);
    assert_eq!(&*d.text(), "1:x, 22:y, 333:z");
    assert!(d.undo());
    assert_eq!(&*d.text(), "x=1, y=22, z=333");
    // replace next: finds first, then replaces the selected match
    let q = SearchQuery::literal("=");
    d.set_selections(Selections::caret(0));
    assert!(!d.replace_next(&q, " = ", 1).unwrap());
    assert_eq!(d.selections().primary(), Selection::new(1, 2));
    assert!(d.replace_next(&q, " = ", 2).unwrap());
    assert_eq!(&*d.text(), "x = 1, y=22, z=333");
    assert_eq!(d.selections().primary(), Selection::new(8, 9));
    // literal replacements are literal
    let mut d = doc("plaintext", "a.b");
    d.replace_all(&SearchQuery::literal("."), "$1", false, 0).unwrap();
    assert_eq!(&*d.text(), "a$1b");
    // in the selection only
    let mut d = doc("plaintext", "aaaa");
    d.set_selections(Selections::single(Selection::new(1, 3)));
    assert_eq!(d.replace_all(&SearchQuery::literal("a"), "b", true, 0).unwrap(), 2);
    assert_eq!(&*d.text(), "abba");
}

#[test]
fn basic_folds() {
    let src = "SUB A\n  IF x THEN\n    y\n  END IF\n  IF z THEN w\n  ' SUB in a comment\n  s = \"FOR i\"\nEND SUB\nRUSTSTART\nfn f() {\n}\nRUSTEND\nCREATE F AS QFORM\nEND CREATE";
    let mut d = doc("rapidr-basic", src);
    let folds = d.fold_ranges();
    let f = |s, e, kind| FoldRange { start_line: s, end_line: e, kind };
    assert_eq!(folds, [f(0, 7, FoldKind::Marker), f(1, 3, FoldKind::Marker), f(8, 11, FoldKind::Embedded), f(12, 13, FoldKind::Marker)]);
}

#[test]
fn bracket_and_indent_folds() {
    let mut d = doc("rust", "fn a() {\n    let x = [\n        1,\n    ];\n    // {\n    \"}\"\n}\nfn b() {}\n");
    let folds = d.fold_ranges();
    assert_eq!(folds, [FoldRange { start_line: 0, end_line: 6, kind: FoldKind::Bracket }, FoldRange { start_line: 1, end_line: 3, kind: FoldKind::Bracket }]);
    let mut d = doc("plaintext", "a\n  b\n\n  c\n    d\ne\n");
    let folds = d.fold_ranges();
    assert_eq!(folds, [FoldRange { start_line: 0, end_line: 4, kind: FoldKind::Indent }, FoldRange { start_line: 3, end_line: 4, kind: FoldKind::Indent }]);
}

#[test]
fn brackets() {
    let mut d = doc("rapidr-basic", "x = f(a(1), \")\" , b) ' (\ny = (\n2)");
    // from the open bracket and just after it
    assert_eq!(d.matching_bracket(5), Some((5, 19)));
    assert_eq!(d.matching_bracket(6), Some((5, 19)));
    assert_eq!(d.matching_bracket(19), Some((19, 5)));
    assert_eq!(d.matching_bracket(7), Some((7, 9)));
    // brackets in strings and comments don't count; across lines
    assert_eq!(d.matching_bracket(29), Some((29, 32)));
    assert_eq!(d.matching_bracket(32), Some((32, 29)));
    assert_eq!(d.matching_bracket(1), None);
    // the comment's open bracket has no partner
    assert_eq!(d.matching_bracket(23), None);
}

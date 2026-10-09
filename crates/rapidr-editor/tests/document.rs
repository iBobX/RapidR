//! Editing: typing and its helpers, multiple carets, undo grouping, line
//! endings, moves.

use rapidr_editor::{Change, Direction, Document, EditError, LineEnding, Languages, Selection, Selections};

fn basic(text: &str) -> Document {
    Document::new(text, Languages::builtin().get("rapidr-basic").unwrap())
}

fn rust(text: &str) -> Document {
    Document::new(text, Languages::builtin().get("rust").unwrap())
}

fn carets(doc: &Document) -> Vec<usize> {
    doc.selections().iter().map(|s| s.head).collect()
}

fn at(doc: &mut Document, pos: usize) {
    doc.set_selections(Selections::caret(pos));
}

#[test]
fn typing_and_undo_groups() {
    let mut d = basic("");
    let mut t = 0;
    for c in "hello world".chars() {
        d.type_text(&c.to_string(), t).unwrap();
        t += 50;
    }
    assert_eq!(&*d.text(), "hello world");
    assert!(d.is_modified());
    // a word per undo step
    assert!(d.undo());
    assert_eq!(&*d.text(), "hello ");
    assert!(d.undo());
    assert_eq!(&*d.text(), "");
    assert!(!d.undo());
    assert!(!d.is_modified());
    assert!(d.redo());
    assert!(d.redo());
    assert_eq!(&*d.text(), "hello world");
    assert_eq!(carets(&d), [11]);

    // a pause of 400 ms starts a new step
    let mut d = basic("");
    d.type_text("a", 0).unwrap();
    d.type_text("b", 100).unwrap();
    d.type_text("c", 600).unwrap();
    d.undo();
    assert_eq!(&*d.text(), "ab");

    // moving the caret starts a new step
    let mut d = basic("");
    d.type_text("a", 0).unwrap();
    d.type_text("b", 10).unwrap();
    at(&mut d, 0);
    d.type_text("c", 20).unwrap();
    d.undo();
    assert_eq!(&*d.text(), "ab");

    // backspaces group with each other, not with typing
    let mut d = basic("abc");
    at(&mut d, 3);
    d.backspace(0).unwrap();
    d.backspace(10).unwrap();
    d.type_text("x", 20).unwrap();
    d.undo();
    assert_eq!(&*d.text(), "a");
    d.undo();
    assert_eq!(&*d.text(), "abc");
    assert_eq!(carets(&d), [3]);
}

#[test]
fn undo_tree_keeps_branches() {
    let mut d = basic("");
    d.type_text("a", 0).unwrap();
    d.type_text(" ", 1000).unwrap();
    d.type_text("b", 2000).unwrap();
    let with_b = d.history().current();
    d.undo();
    d.type_text("c", 3000).unwrap();
    assert_eq!(&*d.text(), "a c");
    assert!(!d.can_redo());
    // the other branch is still there
    assert!(d.goto_revision(with_b));
    assert_eq!(&*d.text(), "a b");
    assert!(d.goto_revision(0));
    assert_eq!(&*d.text(), "");
    assert!(d.redo());
    assert_eq!(&*d.text(), "a");
}

#[test]
fn auto_closing_pairs() {
    let mut d = basic("");
    d.type_text("(", 0).unwrap();
    assert_eq!(&*d.text(), "()");
    assert_eq!(carets(&d), [1]);
    d.type_text("x", 1).unwrap();
    d.type_text(")", 2).unwrap(); // overtyped
    assert_eq!(&*d.text(), "(x)");
    assert_eq!(carets(&d), [3]);
    // quotes: a pair, overtyped at the end; not right after a word
    let mut d = basic("");
    d.type_text("\"", 0).unwrap();
    d.type_text("a", 1).unwrap();
    d.type_text("\"", 2).unwrap();
    assert_eq!(&*d.text(), "\"a\"");
    let mut d = basic("x");
    at(&mut d, 1);
    d.type_text("\"", 0).unwrap();
    assert_eq!(&*d.text(), "x\"");
    // not inside a comment, not before a word
    let mut d = basic("' a");
    at(&mut d, 3);
    d.type_text("(", 0).unwrap();
    assert_eq!(&*d.text(), "' a(");
    let mut d = basic("abc");
    at(&mut d, 0);
    d.type_text("(", 0).unwrap();
    assert_eq!(&*d.text(), "(abc");
    // a selection is surrounded
    let mut d = basic("abc");
    d.set_selections(Selections::single(Selection::new(0, 3)));
    d.type_text("(", 0).unwrap();
    assert_eq!(&*d.text(), "(abc)");
    assert_eq!(d.selections().primary(), Selection::new(1, 4));
    // Backspace inside an empty pair takes both
    let mut d = basic("");
    d.type_text("(", 0).unwrap();
    d.backspace(1).unwrap();
    assert_eq!(&*d.text(), "");
    // switched off
    let mut d = basic("");
    d.auto_close = false;
    d.type_text("(", 0).unwrap();
    assert_eq!(&*d.text(), "(");
}

#[test]
fn auto_indent() {
    let mut d = basic("SUB Foo");
    at(&mut d, 7);
    d.newline(0).unwrap();
    assert_eq!(&*d.text(), "SUB Foo\n    ");
    for c in "PRINT 1".chars() {
        d.type_text(&c.to_string(), 1).unwrap();
    }
    d.newline(2).unwrap();
    for c in "END SUB".chars() {
        d.type_text(&c.to_string(), 3).unwrap();
    }
    // END SUB went back a level while it was typed
    assert_eq!(&*d.text(), "SUB Foo\n    PRINT 1\nEND SUB");
    // (undo takes back the last word, "SUB", and the outdent with it)
    d.undo();
    assert_eq!(&*d.text(), "SUB Foo\n    PRINT 1\n    END ");

    // between brackets the pair goes on its own lines
    let mut d = rust("fn x() {}");
    at(&mut d, 8);
    d.newline(0).unwrap();
    assert_eq!(&*d.text(), "fn x() {\n    \n}");
    assert_eq!(carets(&d), [13]);
    // tabs when asked
    let mut d = rust("    if x {");
    d.insert_spaces = false;
    at(&mut d, 10);
    d.newline(0).unwrap();
    assert_eq!(&*d.text(), "    if x {\n    \t");
}

#[test]
fn crlf_files_stay_crlf() {
    let mut d = basic("a\r\nb\r\n");
    assert_eq!(d.line_ending(), LineEnding::CrLf);
    assert_eq!(d.line_count(), 3);
    at(&mut d, 1);
    d.newline(0).unwrap();
    assert_eq!(&*d.text(), "a\r\n\r\nb\r\n");
    // one step over CR LF
    at(&mut d, 3);
    d.move_char(Direction::Backward, false);
    assert_eq!(carets(&d), [1]);
    d.move_char(Direction::Forward, false);
    assert_eq!(carets(&d), [3]);
    d.backspace(10).unwrap();
    assert_eq!(&*d.text(), "a\r\nb\r\n");
    // a caret never lands between CR and LF
    d.set_selections(Selections::caret(2));
    assert_eq!(carets(&d), [1]);
    d.convert_line_endings(LineEnding::Lf, 20).unwrap();
    assert_eq!(&*d.text(), "a\nb\n");
    d.undo();
    assert_eq!(&*d.text(), "a\r\nb\r\n");
}

#[test]
fn multiple_carets() {
    let mut d = basic("a\nb\nc");
    d.set_selections(Selections::new(vec![Selection::caret(1), Selection::caret(3), Selection::caret(5)], 0));
    d.type_text("x", 0).unwrap();
    assert_eq!(&*d.text(), "ax\nbx\ncx");
    assert_eq!(carets(&d), [2, 5, 8]);
    d.backspace(1).unwrap();
    assert_eq!(&*d.text(), "a\nb\nc");
    d.undo();
    d.undo();
    assert_eq!(&*d.text(), "a\nb\nc");
    assert_eq!(carets(&d), [1, 3, 5]);

    // add carets below, type on all
    let mut d = basic("one\ntwo\nthree");
    at(&mut d, 1);
    d.add_cursor_vertical(Direction::Forward);
    d.add_cursor_vertical(Direction::Forward);
    assert_eq!(carets(&d), [1, 5, 9]);
    d.type_text("-", 0).unwrap();
    assert_eq!(&*d.text(), "o-ne\nt-wo\nt-hree");

    // carets that meet merge
    let mut d = basic("ab");
    d.set_selections(Selections::new(vec![Selection::caret(1), Selection::caret(2)], 1));
    d.backspace(0).unwrap();
    d.backspace(1).unwrap();
    assert_eq!(&*d.text(), "");
    assert_eq!(d.selections().len(), 1);

    // paste one line per caret
    let mut d = basic("a\nb");
    d.set_selections(Selections::new(vec![Selection::caret(1), Selection::caret(3)], 0));
    d.paste("1\n2", 0).unwrap();
    assert_eq!(&*d.text(), "a1\nb2");
}

#[test]
fn next_occurrence_and_all() {
    let mut d = basic("foo bar foo baz foo");
    at(&mut d, 1);
    assert!(d.select_next_occurrence());
    assert_eq!(d.selections().primary(), Selection::new(0, 3));
    assert!(d.select_next_occurrence());
    assert!(d.select_next_occurrence());
    assert_eq!(d.selections().len(), 3);
    assert!(!d.select_next_occurrence());
    d.type_text("q", 0).unwrap();
    assert_eq!(&*d.text(), "q bar q baz q");
    let mut d = basic("x = x + xx");
    at(&mut d, 0);
    // (a word: whole words only)
    assert_eq!(d.select_all_occurrences(), 2);
}

#[test]
fn indentation_and_comments() {
    let mut d = basic("a\n\nb");
    d.select_all();
    d.tab(0).unwrap();
    assert_eq!(&*d.text(), "    a\n\n    b");
    d.outdent_lines(1).unwrap();
    assert_eq!(&*d.text(), "a\n\nb");
    d.toggle_line_comment(2).unwrap();
    assert_eq!(&*d.text(), "' a\n\n' b");
    d.toggle_line_comment(3).unwrap();
    assert_eq!(&*d.text(), "a\n\nb");
    // Tab at a caret: spaces to the next stop
    let mut d = basic("ab");
    at(&mut d, 1);
    d.tab(0).unwrap();
    assert_eq!(&*d.text(), "a   b");
    // Backspace in the indentation: back to the previous stop
    let mut d = basic("      x");
    at(&mut d, 6);
    d.backspace(0).unwrap();
    assert_eq!(&*d.text(), "    x");
    let mut d = rust("    x\n    y");
    d.select_all();
    d.toggle_line_comment(0).unwrap();
    assert_eq!(&*d.text(), "    // x\n    // y");
}

#[test]
fn apply_edits_is_one_step() {
    let mut d = basic("hello world");
    at(&mut d, 11);
    d.apply_edits(vec![Change::new(0..5, "goodbye"), Change::insert(11, "!")], 0).unwrap();
    assert_eq!(&*d.text(), "goodbye world!");
    assert_eq!(carets(&d), [14]);
    assert_eq!(d.apply_edits(vec![Change::new(0..5, "x"), Change::new(3..6, "y")], 1), Err(EditError::Overlap(0, 1)));
    d.undo();
    assert_eq!(&*d.text(), "hello world");
    d.read_only = true;
    assert_eq!(d.type_text("x", 2), Err(EditError::ReadOnly));
    // not on a character boundary
    let mut d = basic("é");
    assert!(matches!(d.apply_edits(vec![Change::delete(0..1)], 0), Err(EditError::BadRange(_))));
}

#[test]
fn moves() {
    let mut d = basic("short\n\tlonger line\nx");
    at(&mut d, 4);
    d.move_line(Direction::Forward, false);
    // column 4 is inside the tab (columns 0..4): the nearer side, after it
    assert_eq!(carets(&d), [7]);
    d.move_line(Direction::Forward, false);
    assert_eq!(carets(&d), [20]);
    d.move_line(Direction::Backward, false);
    // the goal column (4) was kept across the short line
    assert_eq!(carets(&d), [7]);
    d.move_line_start(false);
    assert_eq!(carets(&d), [6]);
    d.move_line_start(false);
    assert_eq!(carets(&d), [7]);
    d.move_line_end(true);
    assert_eq!(d.selections().primary(), Selection::new(7, 18));
    d.move_word(Direction::Backward, false);
    assert_eq!(carets(&d), [14]);
    d.move_word(Direction::Backward, false);
    assert_eq!(carets(&d), [7]);
    d.move_doc(Direction::Forward, false);
    assert_eq!(carets(&d), [20]);
    let mut d = basic("foo(bar, baz)");
    at(&mut d, 0);
    d.delete_word(Direction::Forward, 0).unwrap();
    assert_eq!(&*d.text(), "(bar, baz)");
}

#[test]
fn copy_and_lines() {
    let mut d = basic("one\ntwo\nthree");
    at(&mut d, 5);
    assert_eq!(d.copy_text(), "two\n");
    d.select_lines();
    assert_eq!(d.selections().primary(), Selection::new(4, 8));
    d.delete_lines(0).unwrap();
    assert_eq!(&*d.text(), "one\nthree");
    at(&mut d, 6);
    d.delete_lines(1).unwrap();
    assert_eq!(&*d.text(), "one");
}

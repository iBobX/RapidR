//! The editor's text wins over the disk's: an `$INCLUDE`d file changed in
//! the editor and not saved is what the program is analysed and compiled
//! with (the preprocessor's virtual files), and positions through
//! `$DEFINE`s map exactly (the origin map).

use std::fs;

use rapidr_langsvc::{Analysis, LineIndex, Options};

fn scratch(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("rapidr-langsvc-open-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn an_unsaved_include_is_what_the_program_sees() {
    let dir = scratch("unsaved");
    let main = dir.join("main.bas");
    let util = dir.join("util.inc");
    let main_text = "$INCLUDE \"util.inc\"\nPRINT Twice(2)\n";
    fs::write(&main, main_text).unwrap();
    // On disk: no Twice.
    fs::write(&util, "FUNCTION Once(n AS INTEGER) AS INTEGER\n    Once = n\nEND FUNCTION\n").unwrap();
    let mut a = Analysis::new(Options::default());
    a.update(main.clone(), main_text);
    let diags = a.diagnostics(&main);
    assert_eq!(diags.len(), 1, "{diags:?}");
    assert!(diags[0].message.contains("Twice"), "{diags:?}");

    // In the editor, not saved: Twice exists.
    let edited = "' (edited)\nFUNCTION Twice(n AS INTEGER) AS INTEGER\n    Twice = n * 2\nEND FUNCTION\n";
    a.update(util.clone(), edited);
    assert!(a.diagnostics(&main).is_empty(), "{:?}", a.diagnostics(&main));
    let at = main_text.find("Twice").unwrap() + 2;
    let defs = a.definition(&main, at);
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].file, util);
    let index = LineIndex::new(edited);
    assert_eq!(index.line_col(defs[0].start), (1, 9), "the editor's line, not the disk's");
    let hover = a.hover(&main, at).unwrap().markdown;
    assert!(hover.contains("FUNCTION Twice(n AS INTEGER) AS INTEGER"), "{hover}");
    // From the include file itself (it has no program of its own): the
    // program that includes it answers.
    let refs = a.references(&util, edited.find("Twice").unwrap(), true);
    assert_eq!(refs.len(), 3, "{refs:?}");
    // An error in the unsaved include is reported in it.
    a.update(util.clone(), "FUNCTION Twice(n AS INTEGER) AS INTEGER\n    Twice = n * 2\n    Nope n\nEND FUNCTION\n");
    let diags = a.diagnostics(&main);
    assert_eq!(diags.len(), 1, "{diags:?}");
    assert_eq!(diags[0].file, util);
    assert_eq!(diags[0].message, "Unknown SUB or FUNCTION 'Nope'");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn names_after_a_define_are_where_they_are_written() {
    let dir = scratch("define");
    let main = dir.join("main.bas");
    // `$DEFINE` rewrites the line: the names after it keep their places.
    let text = "$DEFINE TEN 10\nDIM total AS INTEGER\ntotal = TEN + total\nPRINT total\n";
    fs::write(&main, text).unwrap();
    let mut a = Analysis::new(Options::default());
    a.update(main.clone(), text);
    let line3 = text.find("total = TEN").unwrap();
    let second = line3 + "total = TEN + ".len();
    let refs = a.references(&main, second + 1, true);
    let starts: Vec<usize> = refs.iter().map(|l| l.start).collect();
    assert!(starts.contains(&second), "the use after TEN is at its own place: {starts:?} (want {second})");
    assert_eq!(refs.len(), 4, "{refs:?}");
    let _ = fs::remove_dir_all(&dir);
}

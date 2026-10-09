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

/// A form of a program of several files (RapidR Studio's Project > Add
/// Form: Form2.rr, which main.rr includes) is analysed as part of the
/// program: Form1 and the main file's SUBs are known there, its problems
/// are the program's in it, and an error in it is the program's too.
#[test]
fn an_included_form_is_analysed_in_its_program() {
    let dir = scratch("forms");
    let main = dir.join("main.rr");
    let form2 = dir.join("Form2.rr");
    let main_text = "$INCLUDE \"Form2.rr\"\n\nSUB Hello\n    Form2.Show\nEND SUB\n\nCREATE Form1 AS RForm\nEND CREATE\n\nForm1.ShowModal\n";
    let form2_text = "SUB Back\n    Form1.Caption = \"back\"\n    Hello\nEND SUB\n\nCREATE Form2 AS RForm\nEND CREATE\n";
    fs::write(&main, main_text).unwrap();
    fs::write(&form2, form2_text).unwrap();
    let mut a = Analysis::new(Options::default());
    a.update(main.clone(), main_text);
    a.update(form2.clone(), form2_text);
    assert!(a.diagnostics(&form2).is_empty(), "Form1 and Hello are the program's: {:?}", a.diagnostics(&form2));
    assert!(a.diagnostics(&main).is_empty(), "{:?}", a.diagnostics(&main));
    // a typo in the form: in the form's problems, and the program's
    a.update(form2.clone(), form2_text.replace("Form1.Caption", "Form1.Captoin"));
    let d = a.diagnostics(&form2);
    assert_eq!(d.len(), 1, "{d:?}");
    assert!(d[0].message.to_ascii_lowercase().contains("captoin") && d[0].file == form2, "{d:?}");
    // a compiler error in the form stops the program: main's problems have it
    // (an unclosed parenthesis: `= = 1` compiles since development's
    // RC.EXE-checked `Obj.Member = …` forms)
    a.update(form2.clone(), form2_text.replace("Form1.Caption = \"back\"", "Form1.Caption = (1"));
    assert!(a.diagnostics(&main).iter().any(|x| x.file == form2), "{:?}", a.diagnostics(&main));
    assert!(!a.diagnostics(&form2).is_empty());
    a.update(form2.clone(), form2_text);
    // completion in the form knows the main file's form
    let typed = form2_text.replace("Form1.Caption = \"back\"", "Form1.");
    a.update(form2.clone(), typed.clone());
    let at = typed.find("Form1.").unwrap() + 6;
    assert!(a.completions(&form2, at).items.iter().any(|i| i.label.eq_ignore_ascii_case("Caption")), "Form1's members in Form2.rr");
    let _ = fs::remove_dir_all(&dir);
}

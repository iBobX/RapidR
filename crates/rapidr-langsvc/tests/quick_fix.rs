//! A misspelt member (docs/studio-wow.md ED-5): RapidQ's compiler's words
//! as a squiggle, and Ctrl+.'s fix — the component's members a typo away.

use std::path::Path;

use rapidr_langsvc::{Analysis, Options};

#[test]
fn a_misspelt_member_is_flagged_and_fixed() {
    let f = Path::new("/project/main.bas");
    let text = "CREATE Form AS QFORM\n    CREATE NameEdit AS QEDIT\n    END CREATE\nEND CREATE\nx$ = NameEdit.Txet\nForm.Captoin = \"Hi\"\n";
    let mut a = Analysis::new(Options::default());
    a.update(f, text);
    let d = a.diagnostics(f);
    let msgs: Vec<&str> = d.iter().map(|d| d.message.as_str()).collect();
    assert!(msgs.contains(&"Member TXET not part of class QEDIT"), "{msgs:?}");
    assert!(msgs.contains(&"Member CAPTOIN not part of class QFORM"), "{msgs:?}");
    let txet = d.iter().find(|d| d.message.contains("TXET")).unwrap();
    assert_eq!(&text[txet.start..txet.end], "Txet");
    // the fixes there: the nearest first, preferred
    let fixes = a.code_actions(f, txet.start, txet.start);
    assert_eq!(fixes[0].title, "Change to Text");
    assert!(fixes[0].preferred);
    let (file, edits) = &fixes[0].edit[0];
    assert_eq!(file, f);
    let mut fixed = text.to_string();
    fixed.replace_range(edits[0].start..edits[0].end, &edits[0].text);
    assert!(fixed.contains("x$ = NameEdit.Text\n"));
    a.update(f, fixed);
    assert!(!a.diagnostics(f).iter().any(|d| d.message.contains("TXET")));
    // a name some component has is the compiler's business, not a typo
    a.update(f, "DIM b AS QBUTTON\nb.FontName = \"x\"\nb.QueryScalar 1\n");
    assert!(!a.diagnostics(f).iter().any(|d| d.message.starts_with("Member")), "{:?}", a.diagnostics(f));
}

/// K4b (docs/preview4-plan.md, docs/rai-mistake-ledger.md): FILEEXISTS
/// gives 1 (RapidQ-exact), and NOT 1 is -2 — true as well —, so `IF NOT
/// FILEEXISTS(f) THEN` is always true. A warning, and Ctrl+.'s fix `= 0`.
#[test]
fn not_fileexists_is_always_true_and_fixed() {
    let f = Path::new("/project/main.rr");
    let text = "DIM f AS STRING\nf = \"notes.txt\"\nIF NOT FILEEXISTS(f) THEN PRINT \"new\"\nDIM ok AS INTEGER\nok = NOT DIREXISTS(\"data\") AND 1\nIF FILEEXISTS(f) = 0 THEN PRINT \"fine\"\n";
    let mut a = Analysis::new(Options::default());
    a.update(f, text);
    let d = a.diagnostics(f);
    let hints: Vec<_> = d.iter().filter(|d| d.code.as_deref() == Some("not-one-is-yes")).collect();
    assert_eq!(hints.len(), 2, "{d:?}");
    assert_eq!(hints[0].message, "FILEEXISTS gives 1, and NOT 1 is -2 (true): write FILEEXISTS(f) = 0");
    assert_eq!(hints[0].severity, rapidr_diagnostics::Severity::Warning);
    assert_eq!(&text[hints[0].start..hints[0].end], "NOT FILEEXISTS(f)");
    assert!(hints[1].message.starts_with("DIREXISTS gives 1"), "{}", hints[1].message);
    // the fix: the whole condition plain, inside an expression in brackets
    let fix = a.code_actions(f, hints[0].start, hints[0].start);
    let fix = fix.iter().find(|x| x.title == "Write FILEEXISTS(f) = 0").expect("the fix");
    assert!(fix.preferred);
    let mut fixed = text.to_string();
    let e = &fix.edit[0].1[0];
    fixed.replace_range(e.start..e.end, &e.text);
    assert!(fixed.contains("IF FILEEXISTS(f) = 0 THEN PRINT \"new\"\n"), "{fixed}");
    let fix2 = a.code_actions(f, hints[1].start, hints[1].start);
    let e = &fix2.iter().find(|x| x.title.starts_with("Write (")).expect("the bracketed fix").edit[0].1[0];
    fixed.replace_range(e.start..e.end, &e.text);
    assert!(fixed.contains("ok = (DIREXISTS(\"data\") = 0) AND 1\n"), "{fixed}");
    a.update(f, fixed);
    assert!(!a.diagnostics(f).iter().any(|d| d.code.as_deref() == Some("not-one-is-yes")));
}

/// K4 messages that say the fix: a global declared again in another file
/// says where the first one is; an unknown SUB the nearest names.
#[test]
fn messages_say_the_fix_across_files() {
    let main = Path::new("/project/main.rr");
    let globals = Path::new("/project/globals.rr");
    let mut a = Analysis::new(Options::default());
    a.update(globals, "' the names every file shares\nDIM siteCount AS INTEGER\n");
    a.update(main, "$INCLUDE \"globals.rr\"\nDIM siteCount AS INTEGER\nSUB ShowNumber(n AS INTEGER)\n    PRINT n\nEND SUB\nShowNumbr 5\n");
    let d = a.diagnostics(main);
    let dup = d.iter().find(|d| d.message.starts_with("Identifier siteCount already used, try another name")).unwrap_or_else(|| panic!("{d:?}"));
    assert!(dup.message.contains("it is declared in globals.rr:2 already"), "{}", dup.message);
    assert_eq!(dup.file, main);
    let unknown = d.iter().find(|d| d.message.starts_with("Unknown SUB or FUNCTION 'ShowNumbr'")).unwrap_or_else(|| panic!("{d:?}"));
    assert!(unknown.message.ends_with("did you mean ShowNumber?"), "{}", unknown.message);
}

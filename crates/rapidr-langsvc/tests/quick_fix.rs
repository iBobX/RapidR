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

//! Typing's questions answered from the last analysis, seen across the
//! edits since (docs/studio-wow.md ED-3: "it never dies"): no new analysis
//! per key, the right answers at today's offsets.

use std::path::Path;

use rapidr_langsvc::{Analysis, Options};

#[test]
fn typing_is_answered_from_the_last_analysis() {
    let f = Path::new("/project/main.bas");
    let base = "DIM b AS QBUTTON\nSUB Greet(Who AS STRING)\n    PRINT Who\nEND SUB\n";
    let mut a = Analysis::new(Options::default());
    a.update(f, base);
    assert!(a.diagnostics(f).is_empty(), "{:?}", a.diagnostics(f));
    // a line typed at the top (everything below moves), then at the end
    a.update(f, format!("' a comment\n{base}"));
    let typed = format!("' a comment\n{base}b.Cap");
    a.update(f, typed.clone());
    let c = a.completions(f, typed.len());
    assert!(c.items.iter().any(|i| i.label == "Caption"), "b's members from the last analysis");
    assert_eq!((c.start, c.end), (typed.len() - 3, typed.len()), "today's offsets");
    // a parameter's hover below the edit
    let at = typed.find("PRINT Who").unwrap() + 7;
    let h = a.hover(f, at).expect("a hover on Who");
    assert!(h.markdown.contains("Who"), "{}", h.markdown);
    // signature help on a call typed now
    let call = format!("{typed}\nGreet(");
    a.update(f, call.clone());
    let sig = a.signature(f, call.len()).expect("Greet's signature");
    assert!(sig.signatures[0].label.contains("Who AS STRING"), "{:?}", sig.signatures[0].label);
    // what must be exact is analysed again
    let d = a.diagnostics(f);
    assert!(!d.is_empty(), "the unfinished lines are reported");
    let defs = a.definition(f, at);
    assert_eq!(defs.len(), 1);
    assert_eq!(&call[defs[0].start..defs[0].end], "Who");
    assert!(defs[0].start < at && call[..defs[0].start].ends_with("SUB Greet("));
}

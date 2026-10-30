//! The language service writes its compatibility notes in English and Studio
//! shows them translated (rapidr_diagnostics::messages::localize): a note
//! whose English no template matches would stay English for good, and one
//! that names the older product would break the rule that RapidR speaks of
//! itself. The golden case `registry_compat` holds each note as the service
//! writes it; every one must be a template with a Spanish catalog entry that
//! says the same and names nothing else.

use std::path::Path;

use rapidr_diagnostics::messages::{identify, localize};

fn notes() -> Vec<String> {
    let case = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/langsvc/registry_compat/main.bas");
    let text = std::fs::read_to_string(&case).expect("the golden case is there");
    text.lines()
        .filter_map(|l| l.strip_prefix("'! diagnostic "))
        .filter_map(|l| {
            let q = l.find('"')?;
            Some(l[q + 1..l.rfind('"')?].to_string())
        })
        .collect()
}

#[test]
fn every_compatibility_note_is_a_template_with_a_spanish_twin() {
    let notes = notes();
    assert!(notes.len() >= 6, "{notes:?}");
    for en in &notes {
        assert!(identify(en).is_some(), "no message template matches the note the language service writes: {en}");
        assert!(!en.to_ascii_lowercase().contains("rapidq"), "the note names the older product: {en}");
        let es = localize(en, "es");
        assert_ne!(&es, en, "not translated: {en}");
        assert!(!es.to_ascii_lowercase().contains("rapidq"), "the Spanish names the older product: {es}");
    }
    assert_eq!(
        localize("RNum is RapidR's own component: a classic compiler doesn't have it (this project is kept classic-compatible)", "es"),
        "RNum es un componente propio de RapidR: un compilador clásico no lo tiene (este proyecto se mantiene compatible con el lenguaje clásico)"
    );
    assert_eq!(localize("QPLOT is not a classic name: RapidR reads it as RPlot", "es"), "QPLOT no es un nombre clásico: RapidR lo lee como RPlot");
}

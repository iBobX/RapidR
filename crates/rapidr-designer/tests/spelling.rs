//! How the inspector's values are written into a RapidQ program
//! (docs/studio-wow.md INS-2): Booleans as 1 / 0 (a line in words keeps
//! them); a RapidQ property's constant the program doesn't define — no
//! RAPIDQ.INC: RC.EXE would read `clRed` as an empty variable, 0 — as its
//! number; RapidR programs (.rr) keep the names RapidR knows.

use std::path::Path;

use rapidr_designer::Document;
use rapidr_preprocessor::PreprocessOptions;

const FORM: &str = "CREATE Form AS QFORM\n    CREATE B AS QBUTTON\n        Default = False\n    END CREATE\n    CREATE E AS QEDIT\n    END CREATE\nEND CREATE\n";

fn set(path: &str, text: &str, comp: &str, prop: &str, value: &str) -> String {
    let mut doc = Document::open(text, Some(Path::new(path)), PreprocessOptions::default());
    let d = doc.designer(0).unwrap();
    let id = d.design.find(comp).unwrap();
    d.selection.set(id);
    d.set_property(prop, Some(value)).unwrap();
    doc.sync();
    doc.text().to_string()
}

#[test]
fn booleans_as_rapidq_writes_them() {
    assert!(set("f.bas", FORM, "E", "ReadOnly", "True").contains("ReadOnly = 1"));
    assert!(set("f.bas", FORM, "E", "ReadOnly", "False").contains("ReadOnly = 0"));
    // (the line in words keeps them)
    assert!(set("f.bas", FORM, "B", "Default", "True").contains("Default = True"));
}

#[test]
fn constants_a_rapidq_program_lacks_are_numbers() {
    let t = set("f.bas", FORM, "E", "Color", "clRed");
    assert!(t.contains("Color = &H0000FF"), "{t}");
    let t = set("f.bas", FORM, "E", "Align", "alClient");
    assert!(t.contains("Align = 5"), "{t}");
    // (the program defines them: their names)
    let with = format!("CONST clRed = &HFF\nCONST alClient = 5\n{FORM}");
    assert!(set("f.bas", &with, "E", "Color", "clRed").contains("Color = clRed"));
    assert!(set("f.bas", &with, "E", "Align", "alClient").contains("Align = alClient"));
    // (a RapidR program: RapidR knows them)
    assert!(set("f.rr", FORM, "E", "Color", "clRed").contains("Color = clRed"));
    // (RapidR's own properties keep RapidR's constants)
    assert!(set("f.bas", FORM, "E", "Anchors", "akLeft + akRight").contains("Anchors = akLeft + akRight"));
}

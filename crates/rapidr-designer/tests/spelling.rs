//! How the inspector's values are written into a RapidQ program
//! (docs/studio-wow.md INS-2): Booleans as 1 / 0 (a line in words keeps
//! them); a RapidQ property's constant the program doesn't define — no
//! RAPIDQ.INC: RC.EXE would read `clRed` as an empty variable, 0 — as its
//! number — in a RapidR program (.rr) too: they aren't built in there.

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
    // (a RapidR program without RAPIDQ.INC doesn't know them either — they
    // read as nothing: `PRINT clRed` prints an empty line — so numbers too;
    // with the include, their names)
    assert!(set("f.rr", FORM, "E", "Color", "clRed").contains("Color = &H0000FF"));
    let included = format!("$INCLUDE \"RAPIDQ.INC\"\n{FORM}");
    assert!(set("f.rr", &included, "E", "Color", "clRed").contains("Color = clRed"));
    // (RapidR's own properties keep RapidR's constants)
    assert!(set("f.bas", FORM, "E", "Anchors", "akLeft + akRight").contains("Anchors = akLeft + akRight"));
}

/// What a component added from the toolbox is written as (R-NAMES): the
/// file's own style — RapidR's names (`RButton`), RapidQ's (`QBUTTON`) in a
/// file written with RapidQ's names — so a file never mixes them; the names
/// already there are never changed.
#[test]
fn new_components_follow_the_files_names() {
    let add = |path: &str, text: &str, ty: &str| -> String {
        let mut doc = Document::open(text, Some(Path::new(path)), PreprocessOptions::default());
        let d = doc.designer(0).unwrap();
        d.add_component(ty, rapidr_value::layout::Rect::new(8, 8, 75, 25), None).unwrap();
        doc.sync();
        doc.text().to_string()
    };
    let t = add("f.bas", FORM, "RBUTTON");
    assert!(t.contains("AS QBUTTON\n") && !t.contains("RButton"), "a RapidQ program keeps RapidQ's names:\n{t}");
    let r = "CREATE Form AS RForm\n    CREATE B AS RButton\n    END CREATE\nEND CREATE\n";
    let t = add("f.rr", r, "QLABEL");
    assert!(t.contains("AS RLabel\n") && !t.contains("QLABEL"), "{t}");
    // (the extension doesn't decide: the names written do)
    let t = add("f.bas", r, "QEDIT");
    assert!(t.contains("AS REdit\n"), "{t}");
    let t = add("f.rr", FORM, "REDIT");
    assert!(t.contains("AS QEDIT\n"), "{t}");
    // (DIMs count too: a file with more of RapidR's names writes RapidR's)
    let mixed = format!("DIM F1 AS RFont\nDIM F2 AS RFont\nDIM F3 AS RFont\n{FORM}");
    assert!(add("f.bas", &mixed, "QLABEL").contains("AS RLabel\n"));
}

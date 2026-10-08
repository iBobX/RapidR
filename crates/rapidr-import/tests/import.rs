//! The RapidQ importer on small programs: what it changes, what it leaves,
//! and the proof that the copy compiles to the same bytecode.

use std::fs;
use std::path::{Path, PathBuf};

use rapidr_import::{import, plan_program, report, upgrade_file, NameStyle, Options, Verification};

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("rapidr-import-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

fn write(d: &Path, name: &str, text: &str) -> PathBuf {
    let p = d.join(name);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&p, text).unwrap();
    p
}

const PROGRAM: &str = "' A QFORM with a QBUTTON (a comment: not changed)
$INCLUDE \"RAPIDQ.INC\"
$INCLUDE \"lib.rqb\"
DIM QButtonCount AS INTEGER
DIM Pb AS QGAUGE
DIM Grid AS qstringgrid, Old AS RBUTTON
TYPE TMyForm EXTENDS QFORM
    Extra AS QLABEL
END TYPE
SUB Clicked (Sender AS QBUTTON)
    QButtonCount = QButtonCount + 1
    PRINT \"QBUTTON clicked\", clRed
END SUB
FUNCTION MakeFont AS QFONT
    DIM f AS QFONT
    MakeFont = f
END FUNCTION
CREATE Form AS QFORM
    Caption = \"QFORM\"
    CREATE Button AS QBUTTON
        OnClick = Clicked
    END CREATE
END CREATE
$IFDEF NOT_DEFINED
DIM Never AS QEDIT
$ENDIF
";

#[test]
fn converts_type_names_only() {
    let d = dir("names");
    let main = write(&d, "main.bas", PROGRAM);
    write(&d, "lib.rqb", "SUB Helper (L AS QLISTBOX)\nEND SUB\n");
    let plan = plan_program(&main, &Options::default()).unwrap();
    let f = &plan.files[0];
    let out = f.converted();
    let expected = PROGRAM
        .replace("DIM Pb AS QGAUGE", "DIM Pb AS RProgressBar")
        .replace("AS qstringgrid, Old AS RBUTTON", "AS RStringGrid, Old AS RButton")
        .replace("EXTENDS QFORM", "EXTENDS RForm")
        .replace("Extra AS QLABEL", "Extra AS RLabel")
        .replace("(Sender AS QBUTTON)", "(Sender AS RButton)")
        .replace("MakeFont AS QFONT", "MakeFont AS RFont")
        .replace("f AS QFONT", "f AS RFont")
        .replace("Form AS QFORM", "Form AS RForm")
        .replace("Button AS QBUTTON", "Button AS RButton");
    assert_eq!(out, expected);
    // (the comment, the strings, QButtonCount and the $IFDEF branch stay)
    assert!(out.contains("' A QFORM with a QBUTTON") && out.contains("\"QBUTTON clicked\"") && out.contains("Never AS QEDIT"));
    assert!(plan.notes.iter().any(|n| n.message.contains("`QEDIT` is in an $IFDEF branch")), "{:?}", plan.notes);
    let lib = plan.files.iter().find(|f| f.path.ends_with("lib.rqb")).unwrap();
    assert_eq!(lib.converted(), "SUB Helper (L AS RListBox)\nEND SUB\n");
    assert_eq!(f.style(), NameStyle::Mixed, "QBUTTON … and RBUTTON");
    assert_eq!(lib.style(), NameStyle::RapidQ);
    let c = &f.changes[0];
    assert_eq!((c.line, c.column, c.from.as_str(), c.to.as_str()), (5, 11, "QGAUGE", "RProgressBar"));
}

#[test]
fn program_types_and_defines_stay() {
    let d = dir("own");
    // (QBevel.inc's own TYPE QBEVEL: the program's, not RapidR's built-in)
    let main = write(&d, "main.bas", "TYPE QBEVEL EXTENDS QPANEL\n    Depth AS INTEGER\nEND TYPE\nDIM B AS QBEVEL\n$DEFINE MyForm QFORM\nDIM F AS MyForm\nTYPE T EXTENDS QOBJECT\nEND TYPE\n");
    let plan = plan_program(&main, &Options::default()).unwrap();
    assert_eq!(plan.files[0].converted(), "TYPE QBEVEL EXTENDS RPanel\n    Depth AS INTEGER\nEND TYPE\nDIM B AS QBEVEL\n$DEFINE MyForm QFORM\nDIM F AS MyForm\nTYPE T EXTENDS RObject\nEND TYPE\n");
    assert!(plan.notes.iter().any(|n| n.message.contains("$DEFINE MyForm QFORM")), "{:?}", plan.notes);
}

#[test]
fn import_copies_proves_and_reports() {
    let d = dir("import");
    let src = d.join("prog");
    let main = write(&src, "main.rqw", &PROGRAM.replace('\n', "\r\n"));
    write(&src, "lib.rqb", "SUB Helper (L AS QLISTBOX)\r\nEND SUB\r\n");
    write(&src, "data/notes.txt", "QFORM in a data file\n");
    write(&src, "Window.tpl", "a RapidQ IDE template\n");
    let before = fs::read(&main).unwrap();
    let out = d.join("copy");
    let r = import(&src, &out, &Options::default(), true).unwrap();
    assert_eq!(fs::read(&main).unwrap(), before, "the original is untouched");
    let copy = fs::read_to_string(out.join("main.rqw")).unwrap();
    assert!(copy.contains("CREATE Form AS RForm\r\n"), "CRLF and the extension kept: {copy}");
    assert_eq!(fs::read_to_string(out.join("lib.rqb")).unwrap(), "SUB Helper (L AS RListBox)\r\nEND SUB\r\n");
    assert_eq!(fs::read_to_string(out.join("data/notes.txt")).unwrap(), "QFORM in a data file\n");
    assert!(!out.join("Window.tpl").exists(), "RapidQ's templates aren't copied");
    // (the $INCLUDE "RAPIDQ.INC" line is kept: its constants come with it)
    assert!(copy.contains("$INCLUDE \"RAPIDQ.INC\""));
    let main_report = r.programs.iter().find(|p| p.entry.ends_with("main.rqw")).unwrap();
    assert_eq!(main_report.verification, Some(Verification::Identical));
    let md = report::markdown(&r);
    assert!(md.contains("line 5, column 11: `QGAUGE` → `RProgressBar`"), "{md}");
    assert!(md.contains("`.rqw`") && md.contains("RAPIDQ.INC"), "{md}");
    assert!(md.contains("RapidQ IDE template"), "{md}");
    // Importing the copy again changes nothing: it already has RapidR's names.
    let again = import(&out, &d.join("copy2"), &Options::default(), true).unwrap();
    assert_eq!(again.changes(), 0, "{}", report::markdown(&again));
    // Never over the original.
    assert!(import(&src, &src, &Options::default(), false).is_err());
}

#[test]
fn rapidq_inc_file_left_out_unless_used() {
    let d = dir("rqinc");
    let src = d.join("prog");
    // (a RAPIDQ.INC of the program's own folder: constants RapidR has, and
    // RapidQ's QBColor array, which RapidR's built-in constants don't have)
    write(&src, "RAPIDQ.INC", "$IFNDEF __RQINC\n$DEFINE __RQINC\nCONST clRed = &HFF\nCONST MB_OK = 0\nDIM QBColor(0 TO 15) AS INTEGER\n$ENDIF\n");
    write(&src, "a.bas", "$INCLUDE \"RAPIDQ.INC\"\nPRINT clRed\nDIM F AS QFORM\n");
    let out = d.join("copy");
    let r = import(&src, &out, &Options::default(), true).unwrap();
    assert!(!out.join("RAPIDQ.INC").exists(), "RapidR supplies RAPIDQ.INC");
    assert_eq!(r.rapidq_inc_left_out.len(), 1);
    assert!(r.programs.iter().all(|p| p.verification == Some(Verification::Identical)), "{:?}", r.programs);

    write(&src, "a.bas", "$INCLUDE \"RAPIDQ.INC\"\nPRINT QBColor(4)\n");
    let out = d.join("copy2");
    let r = import(&src, &out, &Options::default(), true).unwrap();
    assert!(out.join("RAPIDQ.INC").exists(), "QBColor is the file's own: copied");
    assert!(r.notes.iter().any(|n| n.message.contains("QBCOLOR")), "{:?}", r.notes);
}

#[test]
fn upgrade_in_place_plan() {
    let d = dir("upgrade");
    let p = write(&d, "app.rr", "CREATE F AS QFORM\n    CREATE B AS rbutton\n    END CREATE\nEND CREATE\nF.ShowModal\n");
    let u = upgrade_file(&p, &Options::default()).unwrap();
    assert_eq!(u.converted, "CREATE F AS RForm\n    CREATE B AS RButton\n    END CREATE\nEND CREATE\nF.ShowModal\n");
    assert_eq!(u.verified, Some(true));
    assert_eq!(u.changes.len(), 2);
    let upgraded = write(&d, "new.rr", &u.converted);
    let plan = plan_program(&upgraded, &Options::default()).unwrap();
    assert_eq!(plan.files[0].style(), NameStyle::RapidR);
    let diff = rapidr_import::diff::unified("app.rr", &u.original, &u.converted, 1);
    assert!(diff.contains("-CREATE F AS QFORM\n+CREATE F AS RForm\n"), "{diff}");
}

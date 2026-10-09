//! IntelliSense as Robert asked for it in RapidR Studio (S-EDITOR): members
//! after `.` for RapidR's and RapidQ's names, TYPEs, WITH and CREATE; R
//! names first after `AS`; RAPIDQ.INC's constants offered with their
//! include; a RAPIDQ.INC constant used without the include flagged, with
//! its quick fix; signature help, hover and F12.

use std::path::PathBuf;

use rapidr_editor::service::LanguageService;
use rapidr_langsvc::editor::EditorService;
use rapidr_langsvc::{Analysis, FileDiagnostic, Severity};

const FILE: &str = "untitled-1.bas";

/// The completions at `|` in `text`: (label, detail, edits' texts).
fn complete(text: &str) -> Vec<(String, String, Vec<String>)> {
    let at = text.find('|').expect("a caret");
    let text = text.replacen('|', "", 1);
    let mut s = EditorService::new();
    s.update(FILE, &text);
    let mut items = s.completions(FILE, at).items;
    items.sort_by(|a, b| a.sort.cmp(&b.sort));
    items.into_iter().map(|c| (c.label, c.detail, c.edits.into_iter().map(|e| e.text).collect())).collect()
}

fn labels(text: &str) -> Vec<String> {
    complete(text).into_iter().map(|(l, _, _)| l).collect()
}

fn position(labels: &[String], label: &str) -> usize {
    labels.iter().position(|l| l == label).unwrap_or_else(|| panic!("no {label} in {labels:?}"))
}

#[test]
fn members_after_a_dot_for_every_kind_of_object() {
    for text in [
        "DIM b AS RBUTTON\nb.|",
        "DIM b AS QBUTTON\nb.|",
        "DIM bs(3) AS RBUTTON\nbs(1).|",
        "DIM b AS RBUTTON\nWITH b\n  .|\nEND WITH",
    ] {
        let l = labels(text);
        for want in ["Caption", "OnClick", "Left"] {
            assert!(l.iter().any(|x| x == want), "{text:?}: no {want} in {l:?}");
        }
    }
    let l = labels("CREATE Form AS RFORM\n  Caption = \"x\"\nEND CREATE\nForm.|");
    assert!(l.contains(&"ShowModal".to_string()) && l.contains(&"Caption".to_string()), "{l:?}");
    // a TYPE's fields; a TYPE extending a component: its own, then the form's
    assert_eq!(labels("TYPE Point\n  X AS INTEGER\n  Y AS INTEGER\nEND TYPE\nDIM p AS Point\np.|"), ["X", "Y"]);
    let l = labels("TYPE TMy EXTENDS QFORM\n  Count AS INTEGER\nEND TYPE\nDIM m AS TMy\nm.|");
    assert_eq!(l[0], "Count");
    assert!(l.contains(&"Caption".to_string()), "{l:?}");
}

#[test]
fn a_create_body_offers_what_can_be_set() {
    let l = labels("CREATE Form AS RFORM\n  CREATE Btn AS RBUTTON\n    |\n  END CREATE\nEND CREATE");
    assert!(l.contains(&"Caption".to_string()) && l.contains(&"OnClick".to_string()), "{l:?}");
    // (read-only properties and methods aren't set in a CREATE)
    assert!(!l.contains(&"Handle".to_string()) && !l.contains(&"SetFocus".to_string()), "{l:?}");
}

#[test]
fn rapidr_names_come_first_after_as_and_rapidq_names_still_complete() {
    // (the file's own style only, never mixed: R-NAMES)
    let l = labels("DIM x AS |");
    position(&l, "RButton");
    position(&l, "RForm");
    assert!(!l.iter().any(|x| x == "QButton" || x == "QForm"), "R style lists RapidR's names: {l:?}");
    // RapidQ's names still complete once one is being typed, after RapidR's
    let l = labels("DIM x AS Q|");
    assert!(position(&l, "RButton") < position(&l, "QButton"), "R first: {l:?}");
    assert!(position(&l, "RForm") < position(&l, "QForm"), "{l:?}");
    let detail = complete("DIM x AS Q|").into_iter().find(|(l, _, _)| l == "QButton").unwrap().1;
    assert_eq!(detail, "RapidQ's name for RButton");
    // a file written with RapidQ's names keeps to them
    let l = labels("DIM f AS QFORM\nDIM b AS QBUTTON\nDIM x AS |");
    position(&l, "QButton");
    assert!(!l.iter().any(|x| x == "RButton"), "Q style: {l:?}");
    let l = labels("DIM f AS QFORM\nDIM b AS QBUTTON\nDIM x AS R|");
    assert!(position(&l, "QButton") < position(&l, "RButton"), "Q style: {l:?}");
}

#[test]
fn rapidq_inc_constants_come_with_their_include() {
    let items = complete("PRINT 1\nr = MessageDlg(\"Save?\", 0, mbY|, 0)\n");
    let (_, detail, edits) = items.iter().find(|(l, _, _)| l == "mbYes").expect("mbYes offered");
    assert!(detail.starts_with("RAPIDQ.INC constant = "), "{detail}");
    assert_eq!(edits, &["$INCLUDE \"RAPIDQ.INC\"\n"]);
    // with the include, they are the program's constants: no import
    let items = complete("$INCLUDE \"RAPIDQ.INC\"\nr = MessageDlg(\"Save?\", 0, mbY|, 0)\n");
    let (_, _, edits) = items.iter().find(|(l, _, _)| l.eq_ignore_ascii_case("mbYes")).expect("mbYes offered");
    assert!(edits.is_empty());
}

fn hints(text: &str) -> (Analysis, PathBuf, Vec<FileDiagnostic>) {
    let mut a = Analysis::default();
    let p = PathBuf::from("/nowhere/prog.bas");
    a.update(&p, text);
    let d = a.diagnostics(&p);
    (a, p, d)
}

#[test]
fn a_rapidq_inc_constant_without_the_include_is_a_warning_with_its_fix() {
    let text = "' Save prompt\n$APPTYPE GUI\nr = MessageDlg(\"Save?\", mtConfirmation, mbYes OR mbNo, 0)\nIF r = mrYes THEN PRINT r\n";
    let (mut a, p, d) = hints(text);
    let w: Vec<&FileDiagnostic> = d.iter().filter(|d| d.code.as_deref() == Some("needs-include")).collect();
    assert_eq!(w.len(), 4, "{d:?}");
    assert!(w.iter().all(|d| d.severity == Severity::Warning));
    let first = w.iter().find(|d| &text[d.start..d.end] == "mbYes").expect("mbYes squiggled");
    assert_eq!(first.message, "mbYes is a RAPIDQ.INC constant — add $INCLUDE \"RAPIDQ.INC\"");
    // the fix: the include after the header, before the code
    let fixes = a.code_actions(&p, first.start, first.end);
    let fix = fixes.iter().find(|f| f.title == "Add $INCLUDE \"RAPIDQ.INC\"").expect("the fix");
    let (_, edits) = &fix.edit[0];
    let mut fixed = text.to_string();
    fixed.insert_str(edits[0].start, &edits[0].text);
    assert!(fixed.starts_with("' Save prompt\n$APPTYPE GUI\n$INCLUDE \"RAPIDQ.INC\"\nr = MessageDlg"), "{fixed}");
    let (_, _, after) = hints(&fixed);
    assert!(after.iter().all(|d| d.code.as_deref() != Some("needs-include")), "{after:?}");
}

#[test]
fn only_an_include_constant_never_stored_is_flagged() {
    // a name the program stores in is its own variable (INPUT, SWAP, READ,
    // FOR and = store); any other undeclared name is RapidQ's implicit
    // variable, as it always was, and not flagged
    let text = "mbOK = 5\nPRINT mbOK\nINPUT clRed\nSWAP clBlue, mrNo\nPRINT foo; Totl\nPRINT mbYes\n";
    let (_, _, d) = hints(text);
    let flagged: Vec<&str> = d.iter().filter(|d| d.code.as_deref() == Some("needs-include")).map(|d| &text[d.start..d.end]).collect();
    assert_eq!(flagged, ["mbYes"], "{d:?}");
    assert!(d.iter().all(|d| d.code.as_deref() == Some("needs-include")), "nothing else: {d:?}");
}

/// The service at `|` in `text`.
fn at(text: &str) -> (EditorService, usize) {
    let at = text.find('|').expect("a caret");
    let mut s = EditorService::new();
    s.update(FILE, &text.replacen('|', "", 1));
    (s, at)
}

#[test]
fn signature_help_for_the_programs_routines_and_components_methods() {
    let (mut s, offset) = at("SUB Hello(Name AS STRING, Times AS INTEGER)\nEND SUB\nHello(\"you\", |");
    let h = s.signature(FILE, offset).expect("a signature");
    assert!(h.signatures[0].label.contains("Hello(Name AS STRING, Times AS INTEGER)"), "{:?}", h.signatures[0].label);
    assert_eq!(h.active_param, 1);
    let (mut s, offset) = at("DIM l AS RLISTBOX\nl.AddItems(|");
    let h = s.signature(FILE, offset).expect("a method's signature");
    assert!(h.signatures[0].label.to_ascii_lowercase().contains("additems("), "{:?}", h.signatures[0].label);
}

#[test]
fn hover_and_go_to_definition_on_a_routine() {
    let text = "SUB Hello(Name AS STRING)\n  PRINT Name\nEND SUB\nHel|lo \"you\"\n";
    let (mut s, offset) = at(text);
    let hover = s.hover(FILE, offset).expect("a hover");
    assert!(hover.text.contains("Hello(Name AS STRING)"), "{}", hover.text);
    let defs = s.definition(FILE, offset);
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].start, text.find("Hello").unwrap());
}

#[test]
fn go_to_definition_of_an_implicit_variable_is_where_it_is_first_stored() {
    let text = "PRINT total\ntotal = 5\nPRINT to|tal\n";
    let (mut s, offset) = at(text);
    let defs = s.definition(FILE, offset);
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].start, text.find("total = 5").unwrap());
}

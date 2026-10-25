//! Completion as Robert asked for it in RapidR Studio (S-EDITOR): members
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
    assert_eq!(detail, "classic name for RButton");
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
    assert!(detail.starts_with("constant = "), "{detail}");
    assert_eq!(edits, &["$INCLUDE \"RAPIDQ.INC\"\n"]);
    // with the include, they are the program's constants: no import
    let items = complete("$INCLUDE \"RAPIDQ.INC\"\nr = MessageDlg(\"Save?\", 0, mbY|, 0)\n");
    let (_, _, edits) = items.iter().find(|(l, _, _)| l.eq_ignore_ascii_case("mbYes")).expect("mbYes offered");
    assert!(edits.is_empty());
}

/// A RapidR program (`.rr`): RAPIDQ.INC's constants are built in — offered
/// as they are (no include added), never flagged, their hover says so; a
/// name the program has itself is its own (offered once, as its own).
#[test]
fn a_rapidr_program_has_the_constants_built_in() {
    let complete_rr = |text: &str| {
        let at = text.find('|').expect("a caret");
        let text = text.replacen('|', "", 1);
        let mut s = EditorService::new();
        s.update("untitled-1.rr", &text);
        s.completions("untitled-1.rr", at).items
    };
    let items = complete_rr("r = MessageDlg(\"Save?\", mtConfirmation, mbY|, 0)\n");
    let yes = items.iter().find(|c| c.label == "mbYes").expect("mbYes offered");
    assert_eq!(yes.detail, "built-in constant = 1");
    assert!(yes.edits.is_empty(), "no include added: {:?}", yes.edits);
    // (True / False stay RapidR's: not offered as RAPIDQ.INC's 1)
    let items = complete_rr("x = Tr|\n");
    assert!(!items.iter().any(|c| c.label == "True" && c.detail.contains("constant = 1")), "{:?}", items.iter().map(|c| (&c.label, &c.detail)).collect::<Vec<_>>());
    // (the program's own mbNo: its own, not the built-in one too)
    let items = complete_rr("DIM mbNo AS INTEGER = 5\nx = mbN|\n");
    assert_eq!(items.iter().filter(|c| c.label.eq_ignore_ascii_case("mbNo")).count(), 1, "{:?}", items.iter().map(|c| (&c.label, &c.detail)).collect::<Vec<_>>());
    // no hint, and the program compiles clean
    let mut a = Analysis::default();
    let p = PathBuf::from("/nowhere/prog.rr");
    let text = "' Save prompt\n$APPTYPE GUI\nr = MessageDlg(\"Save?\", mtConfirmation, mbYes OR mbNo, 0)\nIF r = mrYes THEN PRINT r; QBColor(12)\n";
    a.update(&p, text);
    let d = a.diagnostics(&p);
    assert!(d.is_empty(), "{d:?}");
    // the hover: built in
    let (mut s, offset) = {
        let text = "r = MessageDlg(\"Save?\", mtConfirmation, mbY|es, 0)\n";
        let at = text.find('|').unwrap();
        let mut s = EditorService::new();
        s.update("untitled-1.rr", &text.replacen('|', "", 1));
        (s, at)
    };
    let hover = s.hover("untitled-1.rr", offset).expect("a hover");
    assert!(hover.text.contains("CONST mbYes = 1") && hover.text.contains("built in"), "{}", hover.text);
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

/// `True` in a RapidQ program without RAPIDQ.INC is the include's name read
/// as RapidQ's implicit variable, 0 (RC.EXE): the same warning as mbYes.
/// With the include, or in a RapidR program (built in, -1), nothing.
#[test]
fn true_without_the_include_is_a_warning_in_a_rapidq_program() {
    let text = "Done = 0\nIF Done = True THEN PRINT \"done\"\n";
    let (_, _, d) = hints(text);
    let w: Vec<&FileDiagnostic> = d.iter().filter(|d| d.code.as_deref() == Some("needs-include")).collect();
    assert_eq!(w.len(), 1, "{d:?}");
    assert_eq!(&text[w[0].start..w[0].end], "True");
    assert_eq!(w[0].message, "True is a RAPIDQ.INC constant — add $INCLUDE \"RAPIDQ.INC\"");
    let (_, _, with) = hints(&format!("$INCLUDE \"RAPIDQ.INC\"\n{text}"));
    assert!(with.iter().all(|d| d.code.as_deref() != Some("needs-include")), "{with:?}");
    let mut a = Analysis::default();
    let p = PathBuf::from("/nowhere/prog.rr");
    a.update(&p, text);
    assert!(a.diagnostics(&p).iter().all(|d| d.code.as_deref() != Some("needs-include")));
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

#[test]
fn a_function_parameter_named_result_is_a_warning() {
    // (inside a FUNCTION, Result is its own value — RapidQ's RESULT — so the
    // parameter can't be read: rapidr_ast::function_result)
    let text = "FUNCTION F (Id AS STRING, Result AS STRING) AS STRING\n    F = Id + Result\nEND FUNCTION\nSUB S (Result AS STRING)\n    PRINT Result\nEND SUB\nPRINT F(\"a\", \"b\")\nS \"c\"\n";
    let (_, _, d) = hints(text);
    let w: Vec<&FileDiagnostic> = d.iter().filter(|d| d.code.as_deref() == Some("hidden-result")).collect();
    assert_eq!(w.len(), 1, "the FUNCTION's only, not the SUB's: {d:?}");
    assert_eq!(w[0].severity, Severity::Warning);
    assert!(text[w[0].start..w[0].end].starts_with("Result"), "{:?}", &text[w[0].start..w[0].end]);
    assert!(w[0].message.contains("FUNCTION F"), "{}", w[0].message);
}

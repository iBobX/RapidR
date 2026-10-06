//! The language service as RapidR Studio's code editor asks it
//! (`rapidr_langsvc::editor`, the `rapidr_editor::service` contract): the
//! same answers as `rapidr lsp`, in the editor's data.

use rapidr_editor::service::{self, Edit, LanguageService, Severity};
use rapidr_langsvc::editor::EditorService;

const FILE: &str = "untitled-1.bas";

const PROGRAM: &str = "DIM Total AS INTEGER\n\
SUB Greet(Name AS STRING)\n  PRINT Name\nEND SUB\n\
CREATE Form AS QFORM\n  Caption = \"x\"\nEND CREATE\n\
Total = 1\nTotal = Total + 2\nGreet(\"you\")\nForm.";

fn service(text: &str) -> EditorService {
    let mut s = EditorService::new();
    s.update(FILE, text);
    s
}

/// The byte offset of the `nth` (0-based) `needle` in `text`, plus `delta`.
fn at(text: &str, needle: &str, nth: usize, delta: usize) -> usize {
    text.match_indices(needle).nth(nth).unwrap_or_else(|| panic!("no {needle:?} #{nth}")).0 + delta
}

fn apply(text: &str, edits: &[Edit]) -> String {
    let mut sorted = edits.to_vec();
    sorted.sort_by_key(|e| e.start);
    let mut out = text.to_string();
    for e in sorted.iter().rev() {
        out.replace_range(e.start..e.end, &e.text);
    }
    out
}

#[test]
fn serves_rapidq_basic() {
    let s = EditorService::new();
    assert!(s.serves("rapidq-basic"));
    assert!(!s.serves("rust"));
    assert!(s.case_triggers().contains(&' ') && s.case_triggers().contains(&'\n'));
}

#[test]
fn completion_after_a_component() {
    let mut s = service(PROGRAM);
    let got = s.completions(FILE, PROGRAM.len());
    let labels: Vec<&str> = got.items.iter().map(|c| c.label.as_str()).collect();
    assert!(labels.contains(&"Caption"), "{labels:?}");
    assert!(labels.contains(&"ShowModal"), "{labels:?}");
    assert_eq!((got.start, got.end), (PROGRAM.len(), PROGRAM.len()));
    let caption = got.items.iter().find(|c| c.label == "Caption").unwrap();
    assert_eq!(caption.kind, service::CompletionKind::Property);
    assert!(!caption.doc.is_empty() || !caption.detail.is_empty(), "{caption:?}");
}

#[test]
fn hover_over_a_variable() {
    let mut s = service(PROGRAM);
    let offset = at(PROGRAM, "Total", 1, 2);
    let h = s.hover(FILE, offset).expect("a hover");
    assert!(h.text.to_ascii_uppercase().contains("INTEGER"), "{}", h.text);
    assert!(h.start <= offset && offset <= h.end);
}

#[test]
fn signature_of_a_sub() {
    let mut s = service(PROGRAM);
    let offset = at(PROGRAM, "Greet(\"", 0, "Greet(".len());
    let help = s.signature(FILE, offset).expect("signature help");
    let sig = &help.signatures[help.active_signature];
    assert!(sig.label.contains("Name"), "{}", sig.label);
    assert_eq!(help.active_param, 0);
    let (a, b) = sig.params[0];
    assert!(sig.label[a..b].contains("Name"), "{}", &sig.label[a..b]);
}

#[test]
fn diagnostics_in_the_compilers_words() {
    let text = "PRINT (\n";
    let mut s = service(text);
    let diags = s.diagnostics(FILE);
    let e = diags.iter().find(|d| d.severity == Severity::Error).unwrap_or_else(|| panic!("an error: {diags:?}"));
    assert_eq!(e.file, FILE);
    assert_eq!(e.message, "Syntax error in PRINT statement");
    assert!(e.start <= e.end && e.end <= text.len(), "{e:?}");
    // (a program without errors: none)
    let mut s = service(PROGRAM.trim_end_matches("Form."));
    assert!(s.diagnostics(FILE).iter().all(|d| d.severity != Severity::Error), "{:?}", s.diagnostics(FILE));
}

#[test]
fn definition_of_a_sub_at_its_call() {
    let mut s = service(PROGRAM);
    let defs = s.definition(FILE, at(PROGRAM, "Greet(\"", 0, 2));
    assert_eq!(defs.len(), 1, "{defs:?}");
    assert_eq!(defs[0].file, FILE);
    let decl = at(PROGRAM, "Greet", 0, 0);
    assert!(defs[0].start <= decl && decl < defs[0].end.max(defs[0].start + 1), "{defs:?}");
    // references: the declaration and the call
    let refs = s.references(FILE, decl + 1);
    assert_eq!(refs.len(), 2, "{refs:?}");
}

#[test]
fn rename_changes_every_use() {
    let mut s = service(PROGRAM);
    let edits = s.rename(FILE, at(PROGRAM, "Total", 0, 1), "Sum").expect("a rename");
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].0, FILE);
    assert_eq!(edits[0].1.len(), 4);
    let renamed = apply(PROGRAM, &edits[0].1);
    assert!(!renamed.contains("Total"), "{renamed}");
    assert_eq!(renamed.matches("Sum").count(), 4);
    // (not a name: why not)
    assert!(s.rename(FILE, at(PROGRAM, "Total", 0, 1), "1x").is_err());
}

#[test]
fn outline_and_semantic_tokens() {
    let mut s = service(PROGRAM);
    let outline = s.outline(FILE);
    fn names(items: &[service::OutlineItem], out: &mut Vec<String>) {
        for i in items {
            out.push(i.name.clone());
            names(&i.children, out);
        }
    }
    let mut all = Vec::new();
    names(&outline, &mut all);
    assert!(all.iter().any(|n| n == "Greet"), "{all:?}");
    assert!(all.iter().any(|n| n == "Form"), "{all:?}");
    let greet = outline.iter().find(|i| i.name == "Greet").unwrap();
    assert_eq!(greet.kind, service::OutlineKind::Sub);
    assert_eq!(&PROGRAM[greet.name_start..greet.name_end], "Greet");

    let tokens = s.semantic_tokens(FILE);
    assert!(tokens.iter().any(|t| t.kind == "function" && &PROGRAM[t.start..t.end] == "Greet"), "{tokens:?}");
    assert!(tokens.iter().any(|t| t.kind == "variable.parameter" && &PROGRAM[t.start..t.end] == "Name"), "{tokens:?}");
}

#[test]
fn keyword_case_as_typed() {
    let text = "dim x as integer ";
    let mut s = service(text);
    let edits = s.case_edits(FILE, text.len(), ' ', "upper");
    // (the word just finished: `integer`)
    assert_eq!(apply(text, &edits), "dim x as INTEGER ");
    // a line left: the whole line
    let text = "dim x as integer\n";
    s.update(FILE, text);
    assert_eq!(apply(text, &s.case_edits(FILE, text.len(), '\n', "proper")), "Dim x As Integer\n");
    // off, or a case it doesn't know: nothing
    assert!(s.case_edits(FILE, text.len(), '\n', "preserve").is_empty());
    assert!(s.case_edits(FILE, text.len(), '\n', "shouting").is_empty());
}

#[test]
fn format_reindents() {
    let text = "SUB A\nPRINT 1\nEND SUB\n";
    let mut s = service(text);
    // (the case the editor last asked: preserve, so only indentation)
    s.case_edits(FILE, 0, ' ', "preserve");
    assert_eq!(apply(text, &s.format(FILE, "    ")), "SUB A\n    PRINT 1\nEND SUB\n");
}

#[test]
fn installed_for_the_thread() {
    rapidr_langsvc::editor::install();
    assert!(service::available("rapidq-basic"));
    let n = service::with(|s| {
        s.update("a.bas", "SUB Greet\nEND SUB\n");
        s.outline("a.bas").len()
    });
    assert_eq!(n, Some(1));
}

#[test]
fn closed_files_are_forgotten() {
    let mut s = service(PROGRAM);
    s.close(FILE);
    assert!(s.outline(FILE).is_empty());
}

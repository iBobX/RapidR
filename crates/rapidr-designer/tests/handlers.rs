//! Event handlers made from the designer / the inspector
//! (`Document::create_handler`, docs/studio-wow.md DES-6 / INS-3): the
//! binding and the SUB in one undo step, where RapidQ wants them, with the
//! registry's parameters.
//!
//! The programs written are kept in tests/fixtures/handlers/out/ (written
//! again with RAPIDR_HANDLERS_BLESS=1); RC.EXE compiles every one of them
//! (tools/rc_probe.sh, compile-only: docs/rapidq-ground-truth.md, "Event
//! handlers"), and so do both of RapidR's backends here.

use std::path::{Path, PathBuf};

use rapidr_designer::{Document, TextPatch};
use rapidr_preprocessor::PreprocessOptions;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn open(name: &str) -> (Document, String, PathBuf) {
    let path = root().join("tests/fixtures/handlers").join(name);
    let text = std::fs::read_to_string(&path).unwrap();
    (Document::open(&text, Some(&path), PreprocessOptions::default()), text, path)
}

/// The patches applied in order to `text` give `want`.
fn replay(text: &str, patches: &[TextPatch]) -> String {
    let mut t = text.to_string();
    for p in patches {
        t.replace_range(p.start..p.end, &p.insert);
    }
    t
}

/// Compiles `text` as `path` to bytecode (the VM) and to Rust (native):
/// both must accept it.
fn compiles_on_both_backends(text: &str, path: &Path) {
    let p = path.to_string_lossy().to_string();
    let tokens = rapidr_lexer::Lexer::new(text, Some(p.clone())).tokenize().unwrap_or_else(|e| panic!("{p}: lexer: {e}"));
    let program = rapidr_parser::parse_tokens(&tokens).unwrap_or_else(|e| panic!("{p}: parser: {e}"));
    rapidr_bcgen::compile_program_with_source(&program, Some(text)).unwrap_or_else(|e| panic!("{p}: bytecode: {e}"));
    assert_eq!(rapidr_codegen_rust::native_gap(&program), None, "{p}: native");
    let rust = rapidr_codegen_rust::generate(&program);
    assert!(rust.contains("fn main"), "{p}: native code generated");
}

/// What a case did, kept as a fixture RC.EXE compiles.
fn keep(name: &str, text: &str) {
    let out = root().join("tests/fixtures/handlers/out").join(name);
    if std::env::var_os("RAPIDR_HANDLERS_BLESS").is_some() {
        std::fs::create_dir_all(out.parent().unwrap()).unwrap();
        std::fs::write(&out, text).unwrap();
    } else {
        let want = std::fs::read_to_string(&out).unwrap_or_else(|_| panic!("{} (RAPIDR_HANDLERS_BLESS=1 writes it)", out.display()));
        assert_eq!(text, want, "{name} differs from tests/fixtures/handlers/out (RAPIDR_HANDLERS_BLESS=1 writes it)");
    }
    compiles_on_both_backends(text, &root().join("tests/fixtures/handlers/out").join(name));
}

#[test]
fn a_bound_event_jumps_to_its_sub() {
    let (mut doc, text, _) = open("declares.bas");
    let h = doc.create_handler(0, "OkButton", "OnClick").unwrap();
    assert_eq!(h.sub, "ShowIt");
    assert!(!h.created);
    assert!(h.patches.is_empty());
    assert_eq!(h.line, Some(25), "inside SUB ShowIt");
    assert_eq!(doc.text(), text);
    // (the default event of a button: OnClick)
    assert_eq!(doc.create_handler(0, "OkButton", "").unwrap().sub, "ShowIt");
}

/// With DECLAREs before the form: a DECLARE after the last one, the SUB at
/// the end; one undo step puts the bytes back.
#[test]
fn with_declares() {
    let (mut doc, text, _) = open("declares.bas");
    let h = doc.create_handler(0, "NameEdit", "OnKeyDown").unwrap();
    assert!(h.created);
    assert_eq!(h.sub, "NameEditKeyDown");
    assert_eq!(h.event, "OnKeyDown");
    let after = doc.text().to_string();
    assert_eq!(replay(&text, &h.patches), after, "the patches are the edit");
    assert!(after.contains("DECLARE SUB ShowIt\nDECLARE SUB NameEditKeyDown (Key AS WORD, Shift AS INTEGER)\n"), "{after}");
    assert!(after.contains("        OnKeyDown = NameEditKeyDown\n"), "{after}");
    assert!(after.ends_with("END SUB\n\nSUB NameEditKeyDown (Key AS WORD, Shift AS INTEGER)\n    \nEND SUB\n"), "{after}");
    let lines: Vec<&str> = after.lines().collect();
    assert_eq!(lines[h.line.unwrap() - 1], "SUB NameEditKeyDown (Key AS WORD, Shift AS INTEGER)");
    // (asked again: the same SUB, nothing written)
    let again = doc.create_handler(0, "NameEdit", "OnKeyDown").unwrap();
    assert!(!again.created && again.patches.is_empty() && again.sub == "NameEditKeyDown");
    assert!(doc.undo());
    assert_eq!(doc.text(), text, "one undo step");
    assert!(doc.redo());
    assert_eq!(doc.text(), after);

    // the form's and the grid's events: their parameters as the registry has them
    doc.create_handler(0, "Form", "OnClose").unwrap();
    doc.create_handler(0, "Form", "OnMouseDown").unwrap();
    doc.create_handler(0, "Grid", "OnDrawCell").unwrap();
    doc.create_handler(0, "OkButton", "OnKeyPress").unwrap();
    keep("declares.bas", doc.text());
}

/// No DECLAREs: the SUB just before the form's CREATE.
#[test]
fn without_declares() {
    let (mut doc, text, _) = open("nodeclares.bas");
    let h = doc.create_handler(0, "NameEdit", "").unwrap();
    assert_eq!(h.sub, "NameEditChange");
    let after = doc.text().to_string();
    assert_eq!(replay(&text, &h.patches), after);
    assert!(after.contains("END SUB\n\nSUB NameEditChange\n    \nEND SUB\n\nCREATE Form AS QFORM\n"), "{after}");
    assert!(after.contains("        OnChange = NameEditChange\n"), "{after}");
    let lines: Vec<&str> = after.lines().collect();
    assert_eq!(lines[h.line.unwrap() - 1], "SUB NameEditChange");
    assert!(doc.undo());
    assert_eq!(doc.text(), text);
    assert!(doc.redo());
    doc.create_handler(0, "Form", "OnClose").unwrap();
    doc.create_handler(0, "Form", "OnMouseDown").unwrap();
    doc.create_handler(0, "Grid", "OnDrawCell").unwrap();
    doc.create_handler(0, "OkButton", "OnKeyPress").unwrap();
    keep("nodeclares.bas", doc.text());
}

/// A name a FUNCTION has is taken: the next free one; a SUB of the name
/// already written is bound as it is.
#[test]
fn names() {
    let (_, text, path) = open("nodeclares.bas");
    let mut t = text.replace("SUB ShowIt\n", "FUNCTION OkButtonKeyPress (K AS INTEGER) AS INTEGER\nEND FUNCTION\n\nSUB NameEditChange\nEND SUB\n\nSUB ShowIt\n");
    let mut doc = Document::open(&t, Some(&path), PreprocessOptions::default());
    let h = doc.create_handler(0, "OkButton", "OnKeyPress").unwrap();
    assert_eq!(h.sub, "OkButtonKeyPress2");
    let h = doc.create_handler(0, "NameEdit", "OnChange").unwrap();
    assert_eq!(h.sub, "NameEditChange");
    assert!(!h.created);
    t = doc.text().to_string();
    assert_eq!(t.matches("SUB NameEditChange").count(), 1, "{t}");
    assert!(t.contains("OnChange = NameEditChange"));
    assert!(doc.create_handler(0, "Nobody", "OnClick").is_err());
    assert!(doc.create_handler(0, "OkButton", "OnNothing").is_err());
}

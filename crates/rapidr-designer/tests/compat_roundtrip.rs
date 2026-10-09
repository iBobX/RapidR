//! Robert: "careful not to break compatibility". Real forms round-tripped
//! through the designer stay programs both of RapidR's backends compile:
//! opened and saved untouched they are byte for byte the same (all of
//! `examples/` and RapidQ's corpus: tests/corpus.rs); a component moved
//! changes its one line; a component added and its OnClick handler made are
//! written as RapidQ writes them (CREATE … END CREATE in the file's own
//! names, `OnClick = Handler`, a plain SUB) — and the program compiles to
//! bytecode and to Rust after each step.

use std::path::{Path, PathBuf};

use rapidr_designer::{Command, Document};
use rapidr_preprocessor::{preprocess_source, PreprocessOptions};
use rapidr_value::layout::Rect;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The program (its includes read as the compiler reads them) compiles to
/// bytecode and to Rust.
fn compiles(text: &str, path: &Path, what: &str) {
    let base = path.parent().unwrap();
    let pre = preprocess_source(text, base, Some(path.to_path_buf()), PreprocessOptions::default()).unwrap_or_else(|e| panic!("{what}: preprocessor: {e}"));
    let tokens = rapidr_lexer::Lexer::new(&pre.source, Some(path.display().to_string())).tokenize().unwrap_or_else(|e| panic!("{what}: lexer: {e}"));
    let program = rapidr_parser::parse_tokens(&tokens).unwrap_or_else(|e| panic!("{what}: parser: {e}"));
    rapidr_bcgen::compile_program_with_source(&program, Some(&pre.source)).unwrap_or_else(|e| panic!("{what}: bytecode: {e}"));
    assert_eq!(rapidr_codegen_rust::native_gap(&program), None, "{what}: native");
}

#[test]
fn real_forms_round_trip_and_compile() {
    for rel in ["examples/rapidq/notepad.bas", "examples/gui/hello_form.rr", "examples/gui/pantry.rr", "examples/gui/menus.rr", "examples/gui/dialogs.rr", "examples/gui/stopwatch.rr"] {
        let path = root().join(rel);
        let bytes = std::fs::read(&path).unwrap();
        let mut doc = Document::open_bytes(&bytes, Some(&path), PreprocessOptions::default());
        assert_eq!(doc.bytes(), bytes, "{rel}: opened, nothing changed");
        assert!(!doc.forms().is_empty(), "{rel}: a form");
        compiles(doc.text(), &path, rel);

        // a component moved 8 px: its one line (notepad's are aligned: none)
        let des = &doc.forms()[0].designer;
        let moved = des.design.ids().into_iter().skip(1).find(|&id| des.design.node(id).is_some_and(|n| n.prop("Left").is_some_and(|v| v.trim().parse::<i64>().is_ok())));
        if let Some(id) = moved {
            let left = des.design.node(id).unwrap().int("Left").unwrap();
            let before = doc.text().to_string();
            doc.designer(0).unwrap().execute(Command::SetProp { node: id, name: "Left".into(), value: Some((left + 8).to_string()) }).unwrap();
            doc.sync();
            let after = doc.text().to_string();
            let changed = before.lines().zip(after.lines()).filter(|(a, b)| a != b).count();
            assert_eq!((changed, before.lines().count()), (1, after.lines().count()), "{rel}: one line changed");
            compiles(&after, &path, &format!("{rel} (moved)"));
        }

        // a button added and its OnClick made: RapidQ's way, the file's names
        let des = doc.designer(0).unwrap();
        let button = des.add_component("QBUTTON", Rect::new(8, 8, 75, 25), None).unwrap();
        let name = des.design.node(button).unwrap().name.clone();
        let written = des.design.node(button).unwrap().type_written.clone();
        doc.sync();
        let q = rel.ends_with(".bas") || doc.text().contains("AS QFORM");
        assert_eq!(written, if q { "QBUTTON" } else { "RButton" }, "{rel}: the file's own names");
        let h = doc.create_handler(0, &name, "OnClick").unwrap();
        let text = doc.text().to_string();
        assert!(text.contains(&format!("OnClick = {}", h.sub)) && text.contains(&format!("SUB {}", h.sub)), "{rel}: bound and written");
        compiles(&text, &path, &format!("{rel} (a button and its handler)"));

        // undone to the exact bytes
        while doc.undo() {}
        assert_eq!(doc.bytes(), bytes, "{rel}: undone");
    }
}

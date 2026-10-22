//! `OnMessage` is RapidR's other name for a form's `WndProc`: one event with
//! one signature in the registry, and one thing to the compilers.

use rapidr_lang::{component, Origin};

#[test]
fn on_message_and_wndproc_share_one_signature() {
    for form in ["RFORM", "QFORM"] {
        let form = component(form).unwrap();
        let wndproc = form.event("WndProc").expect("a form's WndProc");
        let on_message = form.event("OnMessage").expect("a form's OnMessage");
        assert!(!wndproc.params.is_empty(), "WndProc has its four parameters");
        assert_eq!(on_message.params, wndproc.params, "same parameters, same types");
        assert_eq!(wndproc.origin, Origin::RapidQ);
        assert_eq!(on_message.origin, Origin::RapidR, "an extension, marked as such");
        assert!(on_message.doc.contains("WndProc"), "its doc says whose other name it is");
        assert_eq!(on_message.runtimes, wndproc.runtimes, "the same runtimes");
    }
    // (it is the form's: a control's WndProc is accepted and never called)
    assert!(component("RBUTTON").unwrap().event("OnMessage").is_none());
}

/// The bytecode a program compiles to.
fn bytecode(src: &str) -> Vec<u8> {
    let tokens = rapidr_lexer::Lexer::new(src, None).tokenize().unwrap();
    let program = rapidr_parser::parse_tokens(&tokens).unwrap_or_else(|e| panic!("{e}"));
    rapidr_bcgen::compile_program_with_source(&program, Some(src)).unwrap().module.to_bytes()
}

#[test]
fn on_message_compiles_to_what_wndproc_does() {
    let program = |name: &str| {
        format!(
            "DECLARE SUB FormProc (h AS LONG, m AS LONG, w AS LONG, l AS LONG)\nCREATE Form AS QFORM\n  Caption = \"x\"\n  {name} = FormProc\nEND CREATE\n\
             CREATE Other AS RFORM\n  Caption = \"y\"\nEND CREATE\nOther.{name} = FormProc\n\
             SUB FormProc (h AS LONG, m AS LONG, w AS LONG, l AS LONG)\n  PRINT m\nEND SUB\n"
        )
    };
    let wndproc = bytecode(&program("WndProc"));
    assert_eq!(bytecode(&program("OnMessage")), wndproc, "the same module");
    assert_eq!(bytecode(&program("onmessage")), wndproc, "in any case");
    // (and it is not nothing: the event the runtimes fire is bound)
    let text = String::from_utf8_lossy(&wndproc);
    assert!(text.contains("OnWndProc"), "the form's OnWndProc is bound");
    assert!(!text.contains("OnMessage"));
}

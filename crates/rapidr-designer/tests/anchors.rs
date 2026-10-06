//! Anchoring in the designer is the running program's (docs/ide-plan.md I4,
//! a first-release must): tests/fixtures/designer_anchors.bas read into the
//! designer and its resize preview made 600 × 450 gives every component the
//! rectangle that tests/gui_parity_cases.mjs's "designer_anchors" case
//! expects of the program itself resized by its user to 600 × 450 — a case
//! tests/native_gui_events.mjs runs native and interpreted, and
//! tests/web_gui_parity.mjs in the browser. One expectation, three runtimes
//! and the designer: the same layout code (`rapidr_value::layout::engine`).

use std::path::Path;

use rapidr_designer::Document;
use rapidr_preprocessor::PreprocessOptions;

const COMPONENTS: [&str; 10] = ["bar", "status", "namelbl", "nameed", "notes", "side", "pick", "ok", "cancel", "mid"];

/// The case's `expect` lines in tests/gui_parity_cases.mjs.
fn expected(root: &Path) -> Vec<String> {
    let cases = std::fs::read_to_string(root.join("tests/gui_parity_cases.mjs")).unwrap();
    let at = cases.find("name: \"designer_anchors\"").expect("the designer_anchors case");
    let rest = &cases[at..];
    let start = rest.find("expect: [").unwrap() + "expect: [".len();
    let end = start + rest[start..].find(']').unwrap();
    rest[start..end].split(',').map(|s| s.trim().trim_matches('"').to_string()).filter(|s| !s.is_empty()).collect()
}

#[test]
fn the_resize_preview_is_the_running_program() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let path = root.join("tests/fixtures/designer_anchors.bas");
    let doc = Document::open_bytes(&std::fs::read(&path).unwrap(), Some(&path), PreprocessOptions::default());
    let des = &doc.forms()[0].designer;
    let preview = des.preview(600, 450);
    assert_eq!(preview.form_size(), (600, 450));
    let mut got = Vec::new();
    for c in COMPONENTS {
        let id = des.design.find(c).unwrap_or_else(|| panic!("{c} in the form"));
        let r = preview.rect(id).unwrap();
        for (p, v) in [("left", r.left), ("top", r.top), ("width", r.width), ("height", r.height)] {
            got.push(format!("{c}.{p}={v}"));
        }
    }
    if std::env::var_os("RAPIDR_DESIGNER_REPORT").is_some() {
        eprintln!("{}", got.iter().map(|l| format!("\"{l}\"")).collect::<Vec<_>>().join(", "));
    }
    assert_eq!(got, expected(&root));
    // Smaller than its Constraints allow: 260 × 200.
    let small = des.preview(100, 100);
    assert_eq!(small.form_size(), (260, 200));
    // The design itself didn't move.
    let ok = des.design.find("Ok").unwrap();
    assert_eq!(des.layout().rect(ok).map(|r| (r.left, r.top)), Some((248, 236)));
}

//! RDIFFVIEW's three views drawn headlessly, as the desktop draws them
//! (the UI kernel and the CPU renderer), in RapidR light, dark and high
//! contrast (Versions lane G2, docs/versions-plan.md §2):
//! - **text**: two versions of a program, the changed words marked;
//! - **form**: examples/gui/hello_form.rr before and after a change, the
//!   form drawn as the designer draws it on both sides, the changed
//!   components outlined, the changes listed in words;
//! - **table**: two versions of a CSV file: a changed cell, a removed row,
//!   an added row, a new column.
//!
//! The PNGs (2×) go to `$RAPIDR_CAPTURE_DIR`, else the workspace's
//! `target/captures/diff/`; what they show is checked here too (texts in
//! the display list, the outline colours in the pixels).

use std::path::PathBuf;

use rapidr_designer::{Designer, Document};
use rapidr_preprocessor::PreprocessOptions;
use rapidr_ui_kernel::display::{DisplayList, Item};
use rapidr_ui_kernel::{FormUi, MemStore, TextSystem};
use rapidr_value::objects::codec::{encode_png, Pixels};
use rapidr_value::objects::design::{set_source_reader, SourceDoc, SourceEdit};
use rapidr_value::objects::ops::Op;
use rapidr_value::{v_int, v_str};

/// The designer's reader of a program's source, as RapidR Studio's
/// runtimes install it (rapidr-studio's design.rs), reading only.
struct Doc(Document);

impl SourceDoc for Doc {
    fn text(&self) -> String {
        self.0.text().to_string()
    }
    fn set_text(&mut self, text: &str) -> bool {
        self.0 = Document::open(text, None, PreprocessOptions::default());
        true
    }
    fn forms(&self) -> Vec<(String, String)> {
        self.0.forms().iter().map(|f| (f.name().to_string(), f.designer.design.node(f.designer.design.root()).map(|n| n.type_written.clone()).unwrap_or_default())).collect()
    }
    fn designer(&self, form: usize) -> Option<Designer> {
        self.0.forms().get(form).map(|f| f.designer.clone())
    }
    fn commit(&mut self, _: usize, _: Designer) -> (Vec<SourceEdit>, Option<Designer>) {
        (Vec::new(), None)
    }
    fn undo(&mut self) -> Option<Vec<SourceEdit>> {
        None
    }
    fn redo(&mut self) -> Option<Vec<SourceEdit>> {
        None
    }
    fn can_undo(&self) -> bool {
        false
    }
    fn can_redo(&self) -> bool {
        false
    }
    fn name_taken(&self, _: &str) -> bool {
        false
    }
    fn diagnostics(&self) -> Vec<String> {
        self.0.diagnostics().to_vec()
    }
    fn error_line(&self) -> Option<usize> {
        self.0.error_line()
    }
}

fn read(text: &str, _path: &str) -> Box<dyn SourceDoc> {
    Box::new(Doc(Document::open(text, None, PreprocessOptions::default())))
}

fn out_dir() -> PathBuf {
    let dir = std::env::var_os("RAPIDR_CAPTURE_DIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/captures/diff"));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A form `w` × `h` with an RDIFFVIEW filling it (8 pixels around), its
/// kind and texts set.
fn diff_form(kind: &str, left: &str, right: &str, w: i64, h: i64) -> MemStore {
    let mut s = MemStore::new();
    s.add("cap", "RFORM", None).set("cap", "clientwidth", v_int(w)).set("cap", "clientheight", v_int(h));
    s.add("dv", "RDIFFVIEW", Some("cap"));
    for (p, v) in [("left", 8), ("top", 8), ("width", w - 16), ("height", h - 16)] {
        s.set("dv", p, v_int(v));
    }
    s.set("dv", "kind", v_str(kind));
    s.set("dv", "lefttext", v_str(left));
    s.set("dv", "righttext", v_str(right));
    s
}

/// Drawn at `scale` in each look; the PNGs written; the light look's
/// display list and pixels returned.
fn capture(name: &str, s: &MemStore, scale: f64) -> (DisplayList, Pixels) {
    let mut first = None;
    for (look, theme) in [("light", "rapidr light"), ("dark", "rapidr dark"), ("contrast", "rapidr high contrast")] {
        let rapidr_value::theme::Choice::Theme(t) = rapidr_value::theme::choose(theme) else { panic!("{theme}") };
        rapidr_value::theme::set(t);
        let mut ts = TextSystem::new();
        rapidr_value::objects::bitmap::set_display_scale(scale);
        let mut f = FormUi::build(s, "cap", false);
        // (twice: the first lays the view out at its size)
        drop(f.paint(s, &mut ts, scale));
        let list = f.paint(s, &mut ts, scale);
        let px = rapidr_ui_render::cpu::capture(&list, &mut ts, &f);
        let path = out_dir().join(format!("{name}-{look}@{}x.png", scale as i64));
        std::fs::write(&path, encode_png(&px).unwrap()).unwrap();
        eprintln!("{}", path.display());
        if first.is_none() {
            first = Some((list, px));
        }
    }
    let rapidr_value::theme::Choice::Theme(t) = rapidr_value::theme::choose("rapidr light") else { unreachable!() };
    rapidr_value::theme::set(t);
    first.unwrap()
}

fn texts(list: &DisplayList) -> Vec<String> {
    list.items
        .iter()
        .filter_map(|i| match i {
            Item::Op { op: Op::Text { text, .. }, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// How many pixels (RapidQ colours, &HBBGGRR) are within a few levels of
/// `rgb` (0xRRGGBB, a scheme's).
fn count_near(px: &Pixels, rgb: u32) -> usize {
    let ch = |v: u32, s: u32| ((v >> s) & 0xFF) as i32;
    px.pixels.iter().filter(|&&p| [(0, 16), (8, 8), (16, 0)].iter().all(|&(sp, sr)| (ch(p, sp) - ch(rgb, sr)).abs() <= 6)).count()
}

const HELLO: &str = include_str!("../../../examples/gui/hello_form.rr");

#[test]
fn form_diff_of_hello_form() {
    set_source_reader(read);
    let after = HELLO
        .replace("Caption = \"&Greet\"\n        Left = 112: Top = 52: Width = 90", "Caption = \"&Say hello\"\n        Left = 112: Top = 52: Width = 110")
        .replace("Text = \"World\"\n        Left = 112: Top = 16: Width = 200", "Text = \"RapidR\"\n        Left = 112: Top = 16: Width = 160")
        .replace("    CREATE Answer AS RLabel", "    CREATE Hint AS RLabel\n        Caption = \"Press Enter to greet\"\n        Left = 16: Top = 124: Width = 300\n    END CREATE\n    CREATE Answer AS RLabel")
        .replace("Height = 170", "Height = 200");
    assert_ne!(after, HELLO);
    let s = diff_form("form", HELLO, &after, 760, 560);
    let summary = rapidr_ui_kernel::Store::get(&s, "dv", "summary").to_string_val();
    assert_eq!(
        summary,
        "Form: Height 170 \u{2192} 200\n\
         NameEdit: Text \"World\" \u{2192} \"RapidR\"; Width 200 \u{2192} 160\n\
         GreetButton: Caption \"&Greet\" \u{2192} \"&Say hello\"; Width 90 \u{2192} 110\n\
         Added Hint (RLabel)"
    );
    assert_eq!(rapidr_ui_kernel::Store::get(&s, "dv", "changecount").to_i64(), 4);
    let (list, px) = capture("form-diff-hello_form", &s, 2.0);
    let shown = texts(&list);
    for want in ["Before", "After", "GreetButton (RButton)", "Hint (RLabel)", "\"&Say hello\"", "Hello, RapidR"] {
        assert!(shown.iter().any(|t| t == want), "{want}: {shown:?}");
    }
    // the changed components outlined in the theme's colours
    let scheme = rapidr_value::code_scheme::for_theme(rapidr_value::theme::current());
    assert!(count_near(&px, scheme.mark_changed) > 400, "the changed components' outlines");
    assert!(count_near(&px, scheme.mark_added) > 200, "the added label's outline");
}

#[test]
fn text_diff_with_word_marks() {
    let old = "' Greets the person whose name is typed\nSUB Greet\n    clicks = clicks + 1\n    Answer.Caption = \"Hello, \" + NameEdit.Text + \"!\"\nEND SUB\n\nSUB NameChanged\n    GreetButton.Enabled = LEN(NameEdit.Text) > 0\nEND SUB\n";
    let new = "' Greets the person whose name is typed\nSUB Greet\n    clicks = clicks + 1\n    Answer.Caption = \"Hello again, \" + NameEdit.Text + \"! (\" + STR$(clicks) + \")\"\nEND SUB\n\nSUB NameChanged\n    GreetButton.Enabled = LEN(TRIM$(NameEdit.Text)) > 0\n    Hint.Visible = LEN(NameEdit.Text) = 0\nEND SUB\n";
    let s = diff_form("text", old, new, 1180, 330);
    assert_eq!(rapidr_ui_kernel::Store::get(&s, "dv", "hunkcount").to_i64(), 2);
    let (list, _) = capture("text-diff-words", &s, 2.0);
    // (word marks: rounded boxes)
    let marks = list.items.iter().filter(|i| matches!(i, Item::Op { op: Op::Round { radius, .. }, .. } if *radius == 2.0)).count();
    assert!(marks >= 4, "{marks} marks");
}

#[test]
fn table_diff_of_a_csv() {
    let old = "Id,Name,Score,City\n1,Ana,10,Lima\n2,Ben,7,Oslo\n3,Cy,5,Rome\n4,Dee,9,Kyiv\n";
    let new = "Id,Name,Score,City,Email\n1,Ana,12,Lima,ana@example.com\n3,Cy,5,Rome,cy@example.com\n4,Dee,9,Kyiv,dee@example.com\n5,Eve,11,Quito,eve@example.com\n";
    let s = diff_form("table", old, new, 760, 300);
    assert_eq!(rapidr_ui_kernel::Store::get(&s, "dv", "summary").to_string_val(), "Added column: Email\nRow 1 (Id 1): Score 10 \u{2192} 12\nRemoved row 2 (Id 2)\nAdded row 4 (Id 5)");
    let (list, _) = capture("table-diff-csv", &s, 2.0);
    let shown = texts(&list);
    for want in ["+ Email", "Id", "12", "10", "Eve", "ben@example.com"].iter().filter(|w| **w != "ben@example.com") {
        assert!(shown.iter().any(|t| t == want), "{want}: {shown:?}");
    }
}

use proptest::prelude::*;
use rapidr_value::designer::arrange::{AlignHow, Axis};
use rapidr_value::designer::Side;
use rapidr_value::layout::Rect;

use super::*;

const FORM: &str = "' A form with comments, blank lines and code in its CREATE.\r
DECLARE SUB Go\r
\r
CREATE Form AS QFORM\r
    Caption = \"Demo\": Width = 400: Height = 300   ' the size\r
    Center\r
\r
    ' The OK button, bottom right.\r
    CREATE Ok AS QBUTTON\r
        Caption = \"OK\"\r
        Left = 300 : Top = 230\r
        OnClick = Go\r
    END CREATE\r
    CREATE Pn AS QPANEL\r
        Left = 10: Top = 50: Width = 300: Height = 100\r
        CREATE Inner AS QLABEL\r
            Caption = \"in\"   ' a label\r
        END CREATE\r
    END CREATE\r
END CREATE\r
\r
SUB Go\r
  ShowMessage \"hi\"\r
END SUB\r
\r
Form.ShowModal\r
";

fn open() -> Document {
    Document::open(FORM, None, PreprocessOptions::default())
}

#[test]
fn reads_the_create_blocks() {
    let d = open();
    assert_eq!(d.forms().len(), 1);
    let f = &d.forms()[0];
    let m = &f.designer.design;
    assert_eq!(f.name(), "Form");
    let ok = m.find("Ok").unwrap();
    let n = m.node(ok).unwrap();
    assert_eq!((n.type_written.as_str(), n.canonical.as_str()), ("QBUTTON", "RBUTTON"));
    assert_eq!(n.props().map(|p| (p.name.as_str(), p.value.as_str())).collect::<Vec<_>>(), [("Caption", "\"OK\""), ("Left", "300"), ("Top", "230"), ("OnClick", "Go")]);
    let form = m.node(m.root()).unwrap();
    assert!(form.body.iter().any(|i| *i == Item::Code("Center".into())), "{:?}", form.body);
    assert_eq!(m.find("Inner").and_then(|i| m.parent(i)), m.find("Pn"));
    assert_eq!(d.bytes(), FORM.as_bytes(), "no change: the same bytes");
}

fn set(d: &mut Document, comp: &str, prop: &str, value: &str) -> Vec<TextPatch> {
    let id = d.forms()[0].designer.design.find(comp).unwrap();
    d.apply(0, Command::SetProp { node: id, name: prop.into(), value: Some(value.into()) }).unwrap()
}

#[test]
fn a_property_change_edits_its_value_only() {
    let mut d = open();
    let p = set(&mut d, "Ok", "left", "308");
    assert_eq!(p.len(), 1);
    assert_eq!(d.text(), FORM.replace("Left = 300 : Top", "Left = 308 : Top"));
    // the same value again: nothing
    assert!(set(&mut d, "Ok", "Left", "308").is_empty());
    assert!(d.undo());
    assert_eq!(d.bytes(), FORM.as_bytes());
    assert!(d.redo());
    assert!(d.text().contains("Left = 308 : Top"));
}

#[test]
fn new_lines_in_the_blocks_style_and_removals() {
    let mut d = open();
    set(&mut d, "Ok", "Anchors", "akRight + akBottom");
    assert!(d.text().contains("        OnClick = Go\r\n        Anchors = akRight + akBottom\r\n    END CREATE"), "{}", d.text());
    set(&mut d, "Inner", "Left", "4");
    assert!(d.text().contains("            Caption = \"in\"   ' a label\r\n            Left = 4\r\n"), "{}", d.text());
    // removing a property alone on its line drops the line; one on a
    // shared line, the statement and its ':'
    let ok = d.forms()[0].designer.design.find("Ok").unwrap();
    d.apply(0, Command::SetProp { node: ok, name: "OnClick".into(), value: None }).unwrap();
    assert!(!d.text().contains("OnClick"));
    d.apply(0, Command::SetProp { node: ok, name: "Top".into(), value: None }).unwrap();
    assert!(d.text().contains("        Left = 300\r\n"), "{}", d.text());
    let form = d.forms()[0].designer.design.root();
    d.apply(0, Command::SetProp { node: form, name: "Width".into(), value: None }).unwrap();
    assert!(d.text().contains("    Caption = \"Demo\": Height = 300   ' the size\r\n"), "{}", d.text());
    while d.undo() {}
    assert_eq!(d.bytes(), FORM.as_bytes());
}

#[test]
fn components_added_removed_and_reordered() {
    let mut d = open();
    let des = d.designer(0).unwrap();
    let pn = des.design.find("Pn").unwrap();
    des.add_component("QEDIT", Rect::new(8, 8, 120, 25), Some(pn)).unwrap();
    des.add_component("QBUTTON", Rect::new(16, 260, 75, 25), None).unwrap();
    let expected = d.forms()[0].designer.design.clone();
    d.sync();
    assert_eq!(d.forms()[0].designer.design, expected, "the text read back is the model");
    assert!(d.text().contains("        CREATE Edit1 AS QEDIT\r\n            Left = 8\r\n"), "{}", d.text());
    assert!(d.text().contains("    CREATE Button1 AS QBUTTON\r\n        Caption = \"Button1\"\r\n"), "{}", d.text());
    // z-order: Ok to the front (after Pn and Button1), then back
    let des = d.designer(0).unwrap();
    des.selection.set(des.design.find("Ok").unwrap());
    des.bring_to_front().unwrap();
    let expected = d.forms()[0].designer.design.clone();
    d.sync();
    assert_eq!(d.forms()[0].designer.design, expected);
    let t = d.text().to_string();
    assert!(t.find("CREATE Ok").unwrap() > t.find("CREATE Button1").unwrap());
    assert!(t.contains("    ' The OK button, bottom right.\r\n    CREATE Pn"), "the comment stays where it was: {t}");
    // removed
    let des = d.designer(0).unwrap();
    des.selection.set(des.design.find("Pn").unwrap());
    des.delete().unwrap();
    d.sync();
    assert!(!d.text().contains("Inner"));
    while d.undo() {}
    assert_eq!(d.bytes(), FORM.as_bytes());
    // reparenting re-indents the block
    let des = d.designer(0).unwrap();
    let (ok, pn) = (des.design.find("Ok").unwrap(), des.design.find("Pn").unwrap());
    let expected = {
        des.execute(Command::Move { node: ok, parent: pn, index: usize::MAX }).unwrap();
        des.design.clone()
    };
    d.sync();
    assert_eq!(d.forms()[0].designer.design, expected);
    assert!(d.text().contains("        CREATE Ok AS QBUTTON\r\n            Caption = \"OK\"\r\n"), "{}", d.text());
    let des = d.designer(0).unwrap();
    des.rename(ok, "Accept").unwrap();
    d.sync();
    assert!(d.text().contains("CREATE Accept AS QBUTTON"));
    assert_eq!(d.forms()[0].designer.design.find("Accept"), Some(ok), "ids kept by name");
    while d.undo() {}
    assert_eq!(d.bytes(), FORM.as_bytes());
}

/// A random designer operation (as the property tests of the model, on a
/// real text).
#[derive(Clone, Debug)]
enum Op {
    Add(u8, i64, i64, bool),
    Select(usize),
    Delete,
    Nudge(i64, i64),
    Align(u8),
    Size(u8),
    Front,
    Back,
    Anchor(u8),
    Prop(u8, i64),
    Paste,
    Reparent(usize),
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        (0u8..3, 0i64..300, 0i64..200, any::<bool>()).prop_map(|(t, x, y, inside)| Op::Add(t, x, y, inside)),
        (0usize..10).prop_map(Op::Select),
        Just(Op::Delete),
        (-9i64..9, -9i64..9).prop_map(|(x, y)| Op::Nudge(x, y)),
        (0u8..6).prop_map(Op::Align),
        (0u8..3).prop_map(Op::Size),
        Just(Op::Front),
        Just(Op::Back),
        (0u8..4).prop_map(Op::Anchor),
        (0u8..3, -20i64..400).prop_map(|(p, v)| Op::Prop(p, v)),
        Just(Op::Paste),
        (0usize..10).prop_map(Op::Reparent),
    ]
}

fn run(d: &mut Designer, o: &Op) {
    let ids = d.design.ids();
    let pick = |i: usize| ids[i % ids.len()];
    let _ = match o {
        Op::Add(t, x, y, inside) => {
            let parent = if *inside { d.design.find("Pn") } else { None };
            d.add_component(["QBUTTON", "QEDIT", "QPANEL"][*t as usize], Rect::new(*x, *y, 60, 24), parent).map(|_| ())
        }
        Op::Select(i) => {
            d.selection.toggle(pick(*i));
            Ok(())
        }
        Op::Delete => d.delete(),
        Op::Nudge(x, y) => d.nudge(*x, *y, false),
        Op::Align(h) => d.align([AlignHow::Left, AlignHow::Center, AlignHow::Right, AlignHow::Top, AlignHow::Middle, AlignHow::Bottom][*h as usize]),
        Op::Size(a) => d.same_size([Axis::Horizontal, Axis::Vertical, Axis::Both][*a as usize]),
        Op::Front => d.bring_to_front(),
        Op::Back => d.send_to_back(),
        Op::Anchor(s) => d.toggle_anchor([Side::Left, Side::Top, Side::Right, Side::Bottom][*s as usize]),
        Op::Prop(p, v) => d.set_property(["Caption", "Width", "TabOrder"][*p as usize], Some(&v.to_string())),
        Op::Paste => {
            let c = d.copy();
            d.paste(&c).map(|_| ())
        }
        Op::Reparent(i) => match d.selection.primary() {
            Some(n) if d.design.node(pick(*i)).is_some_and(|x| x.is_container() || x.is_form()) => d.execute(Command::Move { node: n, parent: pick(*i), index: usize::MAX }),
            _ => Ok(()),
        },
    };
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 60, ..ProptestConfig::default() })]

    /// Random designer commands, each synced: the text read back equals the
    /// designer's model; untouched lines stay; undo gives the exact bytes.
    #[test]
    fn the_text_and_the_model_agree(ops in proptest::collection::vec(op(), 1..15)) {
        let mut d = open();
        for o in &ops {
            let des = d.designer(0).unwrap();
            run(des, o);
            let expected = des.design.clone();
            d.sync();
            prop_assert_eq!(&d.forms()[0].designer.design, &expected, "after {:?}:\n{}", o, d.text());
            // what's outside the form never changes
            prop_assert!(d.text().starts_with("' A form with comments, blank lines and code in its CREATE.\r\nDECLARE SUB Go\r\n\r\nCREATE Form AS QFORM\r\n"));
            prop_assert!(d.text().ends_with("END CREATE\r\n\r\nSUB Go\r\n  ShowMessage \"hi\"\r\nEND SUB\r\n\r\nForm.ShowModal\r\n"));
        }
        while d.undo() {}
        prop_assert_eq!(d.text(), FORM);
    }
}

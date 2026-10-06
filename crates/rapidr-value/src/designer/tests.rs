//! The Designer as a whole: random command sequences with undo / redo
//! (proptest), the selection operations, anchors.

use proptest::prelude::*;

use super::arrange::{AlignHow, Axis};
use super::*;

fn sample() -> Designer {
    let mut d = Designer::new(FormDesign::new("Form1", "QFORM"));
    d.execute(Command::Batch(vec![
        Command::SetProp { node: 1, name: "Width".into(), value: Some("400".into()) },
        Command::SetProp { node: 1, name: "Height".into(), value: Some("300".into()) },
    ]))
    .unwrap();
    d.add_component("QPANEL", Rect::new(8, 8, 200, 120), None).unwrap();
    let panel = d.design.find("Panel1").unwrap();
    d.add_component("QBUTTON", Rect::new(16, 16, 75, 25), Some(panel)).unwrap();
    d.add_component("QEDIT", Rect::new(220, 8, 120, 25), None).unwrap();
    d.add_component("QLABEL", Rect::new(220, 48, 80, 20), None).unwrap();
    d.history.clear();
    d.take_applied();
    d
}

/// One of the designer's operations, chosen at random.
#[derive(Clone, Debug)]
enum Op {
    Add(u8, i64, i64),
    Select(usize, bool),
    Delete,
    Nudge(i64, i64, bool),
    Align(u8),
    Distribute(bool),
    SameSize(u8),
    Front,
    Back,
    Anchor(u8),
    Prop(u8, i64),
    CopyPaste,
    Duplicate,
    Reparent(usize),
    Rename(usize),
    TabOrder,
    Undo,
    Redo,
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        (0u8..4, 0i64..300, 0i64..200).prop_map(|(t, x, y)| Op::Add(t, x, y)),
        (0usize..12, any::<bool>()).prop_map(|(i, add)| Op::Select(i, add)),
        Just(Op::Delete),
        (-9i64..9, -9i64..9, any::<bool>()).prop_map(|(x, y, r)| Op::Nudge(x, y, r)),
        (0u8..6).prop_map(Op::Align),
        any::<bool>().prop_map(Op::Distribute),
        (0u8..3).prop_map(Op::SameSize),
        Just(Op::Front),
        Just(Op::Back),
        (0u8..4).prop_map(Op::Anchor),
        (0u8..4, -50i64..500).prop_map(|(p, v)| Op::Prop(p, v)),
        Just(Op::CopyPaste),
        Just(Op::Duplicate),
        (0usize..12).prop_map(Op::Reparent),
        (0usize..12).prop_map(Op::Rename),
        Just(Op::TabOrder),
        Just(Op::Undo),
        Just(Op::Redo),
    ]
}

fn run(d: &mut Designer, op: &Op) {
    let ids = d.design.ids();
    let pick = |i: usize| ids[i % ids.len()];
    let _ = match op {
        Op::Add(t, x, y) => d.add_component(["QBUTTON", "QPANEL", "QEDIT", "RPLOT"][*t as usize], Rect::new(*x, *y, 60, 24), None).map(|_| ()),
        Op::Select(i, add) => {
            if *add {
                d.selection.toggle(pick(*i));
            } else {
                d.selection.set(pick(*i));
            }
            Ok(())
        }
        Op::Delete => d.delete(),
        Op::Nudge(x, y, r) => d.nudge(*x, *y, *r),
        Op::Align(h) => d.align([AlignHow::Left, AlignHow::Center, AlignHow::Right, AlignHow::Top, AlignHow::Middle, AlignHow::Bottom][*h as usize]),
        Op::Distribute(h) => d.distribute(if *h { Axis::Horizontal } else { Axis::Vertical }),
        Op::SameSize(a) => d.same_size([Axis::Horizontal, Axis::Vertical, Axis::Both][*a as usize]),
        Op::Front => d.bring_to_front(),
        Op::Back => d.send_to_back(),
        Op::Anchor(s) => d.toggle_anchor([Side::Left, Side::Top, Side::Right, Side::Bottom][*s as usize]),
        Op::Prop(p, v) => d.set_property(["Caption", "Width", "Visible", "Align"][*p as usize], Some(&v.to_string())),
        Op::CopyPaste => {
            let c = d.copy();
            d.paste(&c).map(|_| ())
        }
        Op::Duplicate => d.duplicate().map(|_| ()),
        Op::Reparent(i) => match d.selection.primary() {
            Some(n) => d.execute(Command::Move { node: n, parent: pick(*i), index: usize::MAX }),
            None => Ok(()),
        },
        Op::Rename(i) => {
            let n = pick(*i);
            let name = d.design.unique_name("Renamed");
            d.rename(n, &name)
        }
        Op::TabOrder => {
            let root = d.design.root();
            let mut order = arrange::tab_order(&d.design, root);
            order.reverse();
            d.set_tab_order(root, &order)
        }
        Op::Undo => {
            d.undo();
            Ok(())
        }
        Op::Redo => {
            d.redo();
            Ok(())
        }
    };
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 200, ..ProptestConfig::default() })]

    /// Any sequence: undoing everything gives the form back exactly, redoing
    /// everything gives the last state exactly, and layout never panics.
    #[test]
    fn undo_and_redo_restore_exactly(ops in proptest::collection::vec(op(), 1..40)) {
        let mut d = sample();
        let start = d.design.clone();
        for o in &ops {
            run(&mut d, o);
            // (the names stay unique; every child knows its parent)
            let ids = d.design.ids();
            let mut names: Vec<String> = ids.iter().map(|&i| d.design.node(i).unwrap().name.to_lowercase()).collect();
            names.sort();
            names.dedup();
            prop_assert_eq!(names.len(), ids.len());
            for &i in &ids {
                for c in d.design.children(i) {
                    prop_assert_eq!(d.design.parent(c), Some(i));
                }
            }
            let _ = d.layout();
        }
        let end = d.design.clone();
        let mut undone = 0;
        while d.undo() {
            undone += 1;
        }
        prop_assert_eq!(&d.design, &start);
        for _ in 0..undone {
            prop_assert!(d.redo());
        }
        prop_assert_eq!(&d.design, &end);
    }
}

#[test]
fn anchors_written_as_constants_and_previewed() {
    let mut d = sample();
    let ed = d.design.find("Edit1").unwrap();
    d.selection.set(ed);
    d.toggle_anchor(Side::Right).unwrap();
    assert_eq!(d.design.node(ed).unwrap().prop("Anchors"), Some("akLeft + akTop + akRight"));
    let l = d.preview(500, 300);
    assert_eq!(l.rect(ed).unwrap().width, 220, "stretches with the form: 120 + 100");
    assert_eq!(d.layout().rect(ed).unwrap().width, 120, "the design itself unchanged");
    d.toggle_anchor(Side::Right).unwrap();
    assert_eq!(d.design.node(ed).unwrap().prop("Anchors"), Some("akLeft + akTop"));
    assert!(d.undo() && d.undo());
    assert_eq!(d.design.node(ed).unwrap().prop("Anchors"), None);
    // what the text side hears: two settings, two undos
    assert_eq!(d.take_applied().len(), 4);
}

#[test]
fn copy_paste_cut_and_duplicate() {
    let mut d = sample();
    let panel = d.design.find("Panel1").unwrap();
    d.selection.set(panel);
    let clip = d.copy();
    assert!(clip.text.contains("CREATE Panel1 AS QPANEL") && clip.text.contains("CREATE Button1 AS QBUTTON"), "{}", clip.text);
    d.selection.clear();
    let pasted = d.paste(&clip).unwrap();
    assert_eq!(d.design.node(pasted[0]).unwrap().name, "Panel2");
    assert_eq!(d.design.node(pasted[0]).unwrap().int("Left"), Some(16), "offset by the grid");
    assert!(d.design.find("Button2").is_some(), "its child renamed too");
    d.selection.set(d.design.find("Label1").unwrap());
    let dup = d.duplicate().unwrap();
    assert_eq!(d.design.node(dup[0]).unwrap().name, "Label2");
    let cut = d.cut().unwrap();
    assert_eq!(cut.trees[0].name, "Label2");
    assert!(d.design.find("Label2").is_none());
}

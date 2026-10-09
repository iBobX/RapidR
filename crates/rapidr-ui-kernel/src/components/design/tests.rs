use rapidr_value::designer::{Designer, FormDesign, Prop, SubItem, Subtree};
use rapidr_value::objects::design::DesignSurface;
use rapidr_value::objects::ops::Op;
use rapidr_value::{v_int, v_str, Value};

use crate::components::list::ListAction;
use crate::display::{DisplayList, Item};
use crate::{FormUi, KernelEvent, MemStore, Mods, TextSystem};

fn p(n: &str, v: &str) -> SubItem {
    SubItem::Prop(Prop { name: n.into(), value: v.into() })
}

fn comp(name: &str, ty: &str, body: Vec<SubItem>) -> SubItem {
    SubItem::Child(Subtree { id: 0, name: name.into(), type_written: ty.into(), body })
}

/// A program's form on a surface, in a window of its own: the store, the
/// window's tree, its drawing.
fn shown(form: Subtree, surface: (i64, i64)) -> (MemStore, FormUi, TextSystem, DisplayList) {
    let mut s = MemStore::new();
    s.add("df", "RFORM", None).set("df", "clientwidth", v_int(surface.0 + 20)).set("df", "clientheight", v_int(surface.1 + 20));
    s.add("ds", "RDESIGNSURFACE", Some("df")).set("ds", "left", v_int(10)).set("ds", "top", v_int(10)).set("ds", "width", v_int(surface.0)).set("ds", "height", v_int(surface.1));
    let design = Designer::new(FormDesign::from_subtree(form));
    rapidr_value::objects::with_design_mut("ds", |d| *d = DesignSurface::with_designer(design));
    let mut ts = TextSystem::new();
    let mut f = FormUi::build(&s, "df", false);
    let list = f.paint(&s, &mut ts, 1.0);
    (s, f, ts, list)
}

fn texts(list: &DisplayList) -> Vec<String> {
    list.items.iter().filter_map(|i| if let Item::Op { op: Op::Text { text, .. }, .. } = i { Some(text.clone()) } else { None }).collect()
}

#[test]
fn the_designed_form_is_drawn_by_the_real_components() {
    let form = Subtree {
        id: 0,
        name: "Main".into(),
        type_written: "QFORM".into(),
        body: vec![
            p("Caption", "\"Hello\""),
            p("Width", "300"),
            p("Height", "200"),
            comp("Ok", "QBUTTON", vec![p("Caption", "\"OK\""), p("Left", "10"), p("Top", "10")]),
            comp("Lbl", "QLABEL", vec![p("Caption", "\"Name:\""), p("Left", "10"), p("Top", "50")]),
            comp("Lst", "QLISTBOX", vec![p("Left", "100"), p("Top", "10"), SubItem::Code("AddItems \"one\", \"two\"".into())]),
            comp("Tick", "QTIMER", vec![p("Interval", "500")]),
        ],
    };
    let (_s, f, _ts, list) = shown(form, (400, 320));
    let t = texts(&list);
    // the title bar's caption, the button's, the label's, the list's items,
    // the timer's name in the tray
    for want in ["Hello", "OK", "Name:", "one", "two", "Tick"] {
        assert!(t.iter().any(|x| x == want), "{want} not drawn: {t:?}");
    }
    // the label is the label's code: its caption at its place in the
    // client area (the surface at 10, 10; the form's frame inside the margin)
    let (ox, oy) = rapidr_value::objects::with_design("ds", |d| d.client_origin()).unwrap();
    assert!(list.items.iter().any(|i| matches!(i, Item::Op { origin, op: Op::Text { text, .. } } if text == "Name:" && *origin == (10 + ox + 10, 10 + oy + 50))), "{}", list.dump());
    // the designed components aren't the window's: its tree has the surface only
    assert_eq!(f.nodes.len(), 1);
}

#[test]
fn a_memo_aligned_to_the_client_is_drawn() {
    let form = Subtree {
        id: 0,
        name: "Form".into(),
        type_written: "QFORM".into(),
        body: vec![p("Width", "300"), p("Height", "200"), comp("Text", "QMEMO", vec![p("Align", "5"), p("Text", "\"Right-click me\""), p("WordWrap", "1")])],
    };
    let (_s, f, _ts, list) = shown(form, (400, 320));
    let editors = list.items.iter().filter(|i| matches!(i, Item::Text(_))).count();
    assert!(editors > 0, "{}", list.dump());
    // (the renderer finds the designed memo's text through the window's form)
    let Some(Item::Text(t)) = list.items.iter().find(|i| matches!(i, Item::Text(_))) else { unreachable!() };
    assert!(f.editor_layout_at(&t.node, t.para).is_some());
}

#[test]
fn the_mouse_selects_moves_and_clears_with_the_designers_events() {
    let mut s = MemStore::new();
    s.add("df", "RFORM", None).set("df", "clientwidth", v_int(400)).set("df", "clientheight", v_int(300));
    s.add("ds", "RDESIGNSURFACE", Some("df")).set("ds", "left", v_int(10)).set("ds", "top", v_int(20)).set("ds", "width", v_int(200)).set("ds", "height", v_int(160));
    s.add("ed", "REDIT", Some("df")).set("ed", "left", v_int(220)).set("ed", "top", v_int(20));
    // (a frameless form fills the surface: its client is the surface's)
    rapidr_value::objects::with_design_mut("ds", |d| {
        let root = d.designer.design.root();
        let _ = rapidr_value::designer::Command::SetProp { node: root, name: "BorderStyle".into(), value: Some("0".into()) }.apply(&mut d.designer.design);
    });
    s.call("ds", "addcomponent", &[v_str("RBUTTON"), v_str("Button1"), v_int(16), v_int(16), v_int(80), v_int(24)]);
    // (what the call left to hear, heard: the runtimes fire it after the call)
    rapidr_value::objects::take_design_events("ds");
    let mut f = FormUi::build(&s, "df", false);
    let mut ts = TextSystem::new();
    drop(f.paint(&s, &mut ts, 1.0));
    f.focus_id(&s, "ed");
    let fired = |events: Vec<KernelEvent>| -> Vec<(String, Vec<i64>)> {
        events
            .into_iter()
            .filter_map(|e| match e {
                KernelEvent::List(id, ListAction::Fire(ev, args)) if id == "ds" => Some((ev, args.iter().map(Value::to_i64).collect())),
                _ => None,
            })
            .collect()
    };
    // a press on Button1 (at 30, 30 of the surface), a drag, a release
    f.mouse_down(&s, &mut ts, 40.5, 50.5, rapidr_value::input::Button::Left, Mods::NONE);
    f.mouse_move(&s, &mut ts, 53.5, 56.5, Mods::NONE);
    f.mouse_up(&s, &mut ts, 53.5, 56.5, rapidr_value::input::Button::Left, Mods::NONE);
    let events = f.take_events();
    assert_eq!(fired(events.clone()), [("onselect".to_string(), vec![0]), ("onmove".to_string(), vec![0, 32, 24, 80, 24]), ("onchange".to_string(), vec![])]);
    assert!(!events.iter().any(|e| matches!(e, KernelEvent::Click(_))), "no OnClick");
    assert_eq!(f.focus.map(|i| f.nodes[i].id.clone()).as_deref(), Some("ds"), "the designer takes the focus (its keys)");
    // the background, then a double click on the button
    f.mouse_down(&s, &mut ts, 160.5, 140.5, rapidr_value::input::Button::Left, Mods::NONE);
    f.mouse_up(&s, &mut ts, 160.5, 140.5, rapidr_value::input::Button::Left, Mods::NONE);
    f.mouse_down(&s, &mut ts, 60.5, 60.5, rapidr_value::input::Button::Left, Mods::NONE);
    f.mouse_up(&s, &mut ts, 60.5, 60.5, rapidr_value::input::Button::Left, Mods::NONE);
    f.mouse_down(&s, &mut ts, 60.5, 60.5, rapidr_value::input::Button::Left, Mods::NONE);
    assert_eq!(fired(f.take_events()), [("onbgclick".to_string(), vec![150, 120]), ("onselect".to_string(), vec![0]), ("ondblclick".to_string(), vec![0])]);
    let tree = f.access_tree(&s, &mut ts);
    let json = tree.to_json();
    assert!(json.contains("Button1 (RButton)") && json.contains("\"listbox\""), "RapidR's name: {json}");
}

#[test]
fn a_component_dragged_in_from_elsewhere_is_dropped_where_the_mouse_lets_go() {
    // (a frameless 300 × 200 form read from a program: its client 24 px
    // into the surface, the surface 10 px into its window)
    let form = Subtree { id: 0, name: "Main".into(), type_written: "QFORM".into(), body: vec![p("Width", "300"), p("Height", "200"), p("BorderStyle", "0")] };
    let (s, mut f, mut ts, _) = shown(form, (360, 260));
    let at = (10.0 + 24.0 + 41.5, 10.0 + 24.0 + 33.5);
    // the press elsewhere (a toolbox), the drag over the surface, the release
    rapidr_value::objects::design::begin_drop("QBUTTON");
    f.mouse_move(&s, &mut ts, at.0, at.1, Mods::NONE);
    let ghost = rapidr_value::objects::with_design("ds", |d| d.ghost.clone()).flatten();
    assert_eq!(ghost.map(|(r, t)| (r.left, r.top, t)), Some((40, 32, "QBUTTON".to_string())), "the ghost on the grid");
    f.mouse_up(&s, &mut ts, at.0, at.1, rapidr_value::input::Button::Left, Mods::NONE);
    assert!(rapidr_value::objects::design::drop_pending().is_none(), "the drag ended");
    assert_eq!(rapidr_value::objects::with_design("ds", |d| (d.ids().len(), d.root_name())), Some((1, "Main".to_string())));
    assert_eq!(rapidr_value::objects::with_design_mut("ds", |d| d.call("getcompx", &[v_int(0)])).flatten(), Some(v_int(40)));
    let events: Vec<String> = f
        .take_events()
        .into_iter()
        .filter_map(|e| match e {
            KernelEvent::List(id, ListAction::Fire(ev, _)) if id == "ds" => Some(ev),
            _ => None,
        })
        .collect();
    assert_eq!(events, ["onchange", "onselect"]);
    // Escape drops a drag without adding
    rapidr_value::objects::design::begin_drop("QEDIT");
    let mut clip = crate::MemClipboard::default();
    f.key_down(&s, &mut ts, 27, "", Mods::NONE, &mut clip);
    assert!(rapidr_value::objects::design::drop_pending().is_none());
    assert_eq!(rapidr_value::objects::with_design("ds", |d| d.ids().len()), Some(1));
}

#[test]
fn a_drop_settles_in_and_is_gone_after_100_ms() {
    let form = Subtree { id: 0, name: "Main".into(), type_written: "QFORM".into(), body: vec![p("Width", "300"), p("Height", "200"), p("BorderStyle", "0")] };
    let (s, mut f, mut ts, _) = shown(form, (360, 260));
    let t0 = std::time::Instant::now();
    crate::tick::set_test_now(Some(t0));
    let at = (10.0 + 24.0 + 41.5, 10.0 + 24.0 + 33.5);
    rapidr_value::objects::design::begin_drop("QBUTTON");
    f.mouse_move(&s, &mut ts, at.0, at.1, Mods::NONE);
    f.mouse_up(&s, &mut ts, at.0, at.1, rapidr_value::input::Button::Left, Mods::NONE);
    let fades = |l: &DisplayList| l.items.iter().filter(|i| matches!(i, Item::Op { op: Op::Fade { alpha }, .. } if *alpha <= 110 && *alpha != 40)).count();
    let first = f.paint(&s, &mut ts, 1.0);
    assert!(fades(&first) >= 1, "the wash drawn as it lands");
    assert!(f.next_wake().is_some(), "a frame asked for");
    // 50 ms on: still settling; 120 ms on: gone
    crate::tick::set_test_now(Some(t0 + std::time::Duration::from_millis(50)));
    f.tick(&s, &mut ts, crate::tick::now());
    assert!(fades(&f.paint(&s, &mut ts, 1.0)) >= 1);
    crate::tick::set_test_now(Some(t0 + std::time::Duration::from_millis(120)));
    f.tick(&s, &mut ts, crate::tick::now());
    let last = f.paint(&s, &mut ts, 1.0);
    crate::tick::set_test_now(None);
    assert_eq!(fades(&last), 0, "settled");
}

/// The settling eases out: the wash fades and the ring closes in, frame by
/// frame, from the drop; with the system's reduced motion there is none.
#[test]
fn a_drop_settles_with_easing_and_not_with_reduced_motion() {
    let form = Subtree { id: 0, name: "Main".into(), type_written: "QFORM".into(), body: vec![p("Width", "300"), p("Height", "200"), p("BorderStyle", "0")] };
    // (the wash's alpha and the ring's size in a frame)
    let wash = |l: &DisplayList| l.items.iter().filter_map(|i| match i { Item::Op { op: Op::Fade { alpha }, .. } if *alpha <= 110 && *alpha != 40 => Some(*alpha), _ => None }).max();
    let drop_at = |f: &mut FormUi, s: &MemStore, ts: &mut TextSystem| {
        let at = (10.0 + 24.0 + 41.5, 10.0 + 24.0 + 33.5);
        rapidr_value::objects::design::begin_drop("QBUTTON");
        f.mouse_move(s, ts, at.0, at.1, Mods::NONE);
        f.mouse_up(s, ts, at.0, at.1, rapidr_value::input::Button::Left, Mods::NONE);
    };
    let (s, mut f, mut ts, _) = shown(form.clone(), (360, 260));
    let t0 = std::time::Instant::now();
    crate::tick::set_test_now(Some(t0));
    drop_at(&mut f, &s, &mut ts);
    let mut alphas = Vec::new();
    for ms in [0u64, 25, 50, 75, 95] {
        crate::tick::set_test_now(Some(t0 + std::time::Duration::from_millis(ms)));
        f.tick(&s, &mut ts, crate::tick::now());
        alphas.push(wash(&f.paint(&s, &mut ts, 1.0)).unwrap_or(0));
    }
    assert!(alphas.windows(2).all(|w| w[1] < w[0]), "fading frame by frame: {alphas:?}");
    assert!(alphas[0] >= 100 && *alphas.last().unwrap() < 20, "{alphas:?}");
    // reduced motion: placed at once
    rapidr_value::theme::set_reduced_motion(true);
    let (s, mut f, mut ts, _) = shown(form, (360, 260));
    crate::tick::set_test_now(Some(t0));
    drop_at(&mut f, &s, &mut ts);
    let quiet = wash(&f.paint(&s, &mut ts, 1.0));
    rapidr_value::theme::set_reduced_motion(false);
    crate::tick::set_test_now(None);
    assert_eq!(quiet, None, "no settling with reduced motion");
}

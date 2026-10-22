//! The mouse pointer every component shows by default (`ComponentKind::
//! pointer`, `FormUi::pointer_at`): a text box's I-beam, a divider's or a
//! section edge's resize arrows, a window edge's diagonals, a tab being
//! carried's closed hand — and a Cursor the program set winning (RapidQ).

use rapidr_value::input::{Button, Cursor};
use rapidr_value::{mdi, v_int, v_str};

use crate::{FormUi, MemStore, Mods, TextSystem};

const NONE: Mods = Mods::NONE;

fn place(s: &mut MemStore, id: &str, (l, t, w, h): (i64, i64, i64, i64)) {
    s.set(id, "left", v_int(l)).set(id, "top", v_int(t)).set(id, "width", v_int(w)).set(id, "height", v_int(h));
}

fn built(s: &MemStore, form: &str) -> (FormUi, TextSystem) {
    let mut ts = TextSystem::new();
    let mut f = FormUi::build(s, form, false);
    drop(f.paint(s, &mut ts, 1.0));
    (f, ts)
}

/// The pointer with the mouse at (x, y) of the form's inside.
fn at(f: &mut FormUi, s: &MemStore, ts: &mut TextSystem, x: i64, y: i64) -> Cursor {
    f.mouse_move(s, ts, x as f64 + 0.5, y as f64 + 0.5, NONE);
    f.pointer_at(s, ts, x as f64 + 0.5, y as f64 + 0.5)
}

#[test]
fn text_fields_show_the_ibeam_and_the_programs_cursor_wins() {
    rapidr_value::theme::set(&rapidr_value::theme::CLASSIC);
    let mut s = MemStore::new();
    s.add("pf", "RFORM", None);
    s.add("ed", "REDIT", Some("pf"));
    place(&mut s, "ed", (10, 10, 100, 22));
    s.add("memo", "RMEMO", Some("pf"));
    place(&mut s, "memo", (10, 40, 150, 60));
    s.add("code", "RCODEEDITOR", Some("pf"));
    place(&mut s, "code", (10, 110, 150, 60));
    s.add("btn", "RBUTTON", Some("pf"));
    place(&mut s, "btn", (200, 10, 60, 22));
    let (mut f, mut ts) = built(&s, "pf");
    assert_eq!(at(&mut f, &s, &mut ts, 40, 20), Cursor::IBeam, "an edit");
    assert_eq!(at(&mut f, &s, &mut ts, 60, 60), Cursor::IBeam, "a memo's text");
    // (the code editor's text right of its gutter — line numbers, markers,
    // folds: the arrow there)
    assert_eq!(at(&mut f, &s, &mut ts, 130, 130), Cursor::IBeam, "a code editor's text");
    assert_eq!(at(&mut f, &s, &mut ts, 14, 130), Cursor::Default, "a code editor's gutter");
    assert_eq!(at(&mut f, &s, &mut ts, 230, 20), Cursor::Default, "a button");
    assert_eq!(at(&mut f, &s, &mut ts, 300, 200), Cursor::Default, "the form");
    // (a disabled edit: the arrow; its own Cursor, a hand: the hand —
    // RapidQ's crHandPoint, whatever the component would show)
    s.set("ed", "enabled", v_int(0));
    f.sync(&s);
    assert_eq!(at(&mut f, &s, &mut ts, 40, 20), Cursor::Default);
    s.set("ed", "enabled", v_int(-1)).set("ed", "cursor", v_int(-21));
    f.sync(&s);
    assert_eq!(at(&mut f, &s, &mut ts, 40, 20), Cursor::Hand);
    s.set("ed", "cursor", v_int(0));
    f.sync(&s);
    assert_eq!(at(&mut f, &s, &mut ts, 40, 20), Cursor::IBeam);
}

#[test]
fn splitters_show_the_resize_pointer_for_their_direction_and_keep_it_while_held() {
    rapidr_value::theme::set(&rapidr_value::theme::CLASSIC);
    let mut s = MemStore::new();
    s.add("sf", "RFORM", None).set("sf", "clientwidth", v_int(300)).set("sf", "clientheight", v_int(300));
    s.add("vs", "RSPLITTER", Some("sf"));
    place(&mut s, "vs", (100, 0, 5, 150));
    s.add("hs", "RSPLITTER", Some("sf"));
    place(&mut s, "hs", (0, 200, 300, 5));
    s.set("hs", "align", v_int(1));
    let (mut f, mut ts) = built(&s, "sf");
    assert_eq!(at(&mut f, &s, &mut ts, 102, 50), Cursor::ColResize, "between left and right");
    assert_eq!(at(&mut f, &s, &mut ts, 150, 202), Cursor::RowResize, "between top and bottom");
    assert_eq!(at(&mut f, &s, &mut ts, 50, 50), Cursor::Default);
    // (the program's crHSplit / crVSplit on it: the direction decides, as
    // RapidQ's own Splitter; crSizeWE is taken as asked)
    s.set("vs", "cursor", v_int(-15));
    f.sync(&s);
    assert_eq!(at(&mut f, &s, &mut ts, 102, 50), Cursor::ColResize);
    s.set("vs", "cursor", v_int(-9));
    f.sync(&s);
    assert_eq!(at(&mut f, &s, &mut ts, 102, 50), Cursor::SizeWE);
    s.set("vs", "cursor", v_int(0));
    f.sync(&s);
    // (held: the arrows follow the mouse wherever it goes)
    f.mouse_down(&s, &mut ts, 102.5, 50.5, Button::Left, NONE);
    assert_eq!(at(&mut f, &s, &mut ts, 220, 80), Cursor::ColResize);
    f.mouse_up(&s, &mut ts, 220.5, 80.5, Button::Left, NONE);
    assert_eq!(at(&mut f, &s, &mut ts, 220, 80), Cursor::Default);
}

#[test]
fn the_splitter_is_hot_under_the_mouse_in_rapidrs_look() {
    rapidr_value::theme::set(&rapidr_value::theme::RAPIDR);
    let mut s = MemStore::new();
    s.add("sf", "RFORM", None).set("sf", "clientwidth", v_int(300)).set("sf", "clientheight", v_int(300));
    s.add("vs", "RSPLITTER", Some("sf"));
    place(&mut s, "vs", (100, 0, 6, 150));
    let mut ts = TextSystem::new();
    let mut f = FormUi::build(&s, "sf", false);
    let rest = f.paint(&s, &mut ts, 1.0).dump();
    // (lit once the mouse has rested on it a moment: passing over it
    // doesn't flicker — the tick paints it then)
    let t0 = crate::tick::now();
    crate::tick::set_test_now(Some(t0));
    f.mouse_move(&s, &mut ts, 103.5, 50.5, NONE);
    assert_eq!(f.paint(&s, &mut ts, 1.0).dump(), rest, "not at once");
    let later = t0 + std::time::Duration::from_millis(rapidr_value::dock::look::HOT_DELAY_MS + 10);
    crate::tick::set_test_now(Some(later));
    assert!(f.next_wake().is_some_and(|w| w <= later), "a tick is due");
    f.tick(&s, &mut ts, later);
    let hot = f.paint(&s, &mut ts, 1.0).dump();
    crate::tick::set_test_now(None);
    assert_ne!(rest, hot, "the band in the accent");
    assert!(hot.contains(&format!("#{:06x}", rapidr_value::theme::RAPIDR.accent)), "{hot}");
    f.mouse_move(&s, &mut ts, 20.5, 50.5, NONE);
    assert_eq!(f.paint(&s, &mut ts, 1.0).dump(), rest, "and gone when the mouse leaves");
    // (at rest it shows: a line along it and a grip, the theme's tokens —
    // the dock's splitters' look, one implementation)
    let t = &rapidr_value::theme::RAPIDR;
    assert!(rest.contains(&format!("#{:06x}", t.splitter)), "the line at rest:\n{rest}");
    assert!(rest.contains(&format!("#{:06x}", t.splitter_grip)), "the grip at rest:\n{rest}");
    // (its place and size are the program's: unchanged)
    assert_eq!((f.node("vs").unwrap().abs.0, f.node("vs").unwrap().abs.2), (100, 6));
    rapidr_value::theme::set(&rapidr_value::theme::CLASSIC);
}

/// A dock manager with its groups and panes as the runtime places them:
/// Left | (documents / Output), 800 × 500, in RapidR's look.
fn docked(tag: &str) -> (MemStore, rapidr_value::dock::geometry::Geometry) {
    use rapidr_value::dock::{geometry::Slot, manager};
    rapidr_value::theme::set(&rapidr_value::theme::RAPIDR);
    let (form, dock) = (format!("{tag}f"), format!("{tag}k"));
    let mut s = MemStore::new();
    s.add(&form, "RFORM", None);
    s.add(&dock, "RDOCKMANAGER", Some(&form));
    place(&mut s, &dock, (0, 0, 800, 500));
    manager::with_mut(&dock, |m| {
        m.resize((800, 500), &rapidr_value::objects::font::Font::default());
        m.add_pane(&format!("{tag}left"), "Left", "left", "");
        m.add_pane(&format!("{tag}out"), "Output", "bottom:documents", "");
        m.layout.mode = rapidr_value::dock::DocumentMode::Tabs;
        m.touch();
    });
    let g = manager::with_mut(&dock, |m| m.geometry().clone());
    for (k, gr) in g.groups.iter().enumerate() {
        let name = format!("{dock}__{}", Slot::Docked(k).suffix());
        s.add(&name, "RDOCKGROUP", Some(&dock)).set(&name, "__dock", v_str(&dock)).set(&name, "__slot", v_str(&Slot::Docked(k).suffix()));
        place(&mut s, &name, gr.rect);
        // (each pane a field filling its group: it takes the keyboard)
        let pane = gr.panes[gr.active].clone();
        s.add(&pane, "REDIT", Some(&name));
        place(&mut s, &pane, gr.content);
    }
    (s, g)
}

#[test]
fn a_dock_splitter_is_grabbed_over_the_edges_of_the_panes_beside_it() {
    use rapidr_value::dock::manager::User;
    use rapidr_value::dock::Axis;
    let (s, g) = docked("ga");
    let (mut f, mut ts) = built(&s, "gaf");
    let sp = g.splitters.iter().find(|sp| sp.axis == Axis::Row).unwrap().clone();
    let y = sp.rect.1 + sp.rect.3 / 2;
    // (8 pixels: 2 over the left pane, the gutter's 4, 2 over the right one)
    for x in sp.rect.0 - 2..sp.rect.0 + sp.rect.2 + 2 {
        assert_eq!(at(&mut f, &s, &mut ts, x, y), Cursor::ColResize, "x {x}");
    }
    assert_eq!(at(&mut f, &s, &mut ts, sp.rect.0 - 3, y), Cursor::IBeam, "the pane's own beyond");
    // (a drag from over the pane's edge moves the splitter 40 pixels)
    let (x0, y0) = ((sp.rect.0 - 2) as f64 + 0.5, y as f64 + 0.5);
    f.mouse_down(&s, &mut ts, x0, y0, Button::Left, NONE);
    f.mouse_move(&s, &mut ts, x0 + 40.0, y0, NONE);
    f.mouse_up(&s, &mut ts, x0 + 40.0, y0, Button::Left, NONE);
    let moved = f.take_events().into_iter().any(|e| match e {
        crate::KernelEvent::Container(crate::components::form::Container::Dock { action: User::Split { extents, done: true, .. }, .. }) => extents[0] == g.extents.iter().find(|(p, _)| *p == sp.path).unwrap().1[0] + 40,
        _ => false,
    });
    assert!(moved, "the split's new extents, let go");
    rapidr_value::theme::set(&rapidr_value::theme::CLASSIC);
}

#[test]
fn where_two_dock_splitters_meet_the_corner_drags_both() {
    use rapidr_value::dock::manager::{self, User};
    let (s, g) = docked("gc");
    let (mut f, mut ts) = built(&s, "gcf");
    let c = g.corners.first().expect("Left | (documents / Output): a corner").clone();
    let (cx, cy) = (c.rect.0 + c.rect.2 / 2, c.rect.1 + c.rect.3 / 2);
    // (the four-way arrows; lit — both bands — once the mouse rests)
    let t0 = crate::tick::now();
    crate::tick::set_test_now(Some(t0));
    assert_eq!(at(&mut f, &s, &mut ts, cx, cy), Cursor::Move);
    assert!(!manager::with("gck", |m| m.ui.hot_ready).unwrap(), "not at once");
    let later = t0 + std::time::Duration::from_millis(rapidr_value::dock::look::HOT_DELAY_MS + 10);
    f.tick(&s, &mut ts, later);
    crate::tick::set_test_now(None);
    let mut hot = manager::with_mut("gck", |m| {
        let g = m.geometry().clone();
        rapidr_value::dock::look::hot_splitters(m, &g)
    });
    hot.sort();
    let mut both = vec![c.stands, c.lies];
    both.sort();
    assert_eq!(hot, both, "both bands lit");
    // (a drag from the corner: both splitters follow, 30 right and 20 up)
    let (x0, y0) = (cx as f64 + 0.5, cy as f64 + 0.5);
    f.mouse_down(&s, &mut ts, x0, y0, Button::Left, NONE);
    assert_eq!(at(&mut f, &s, &mut ts, cx + 30, cy - 20), Cursor::Move, "held: the four-way arrows");
    f.mouse_up(&s, &mut ts, x0 + 30.0, y0 - 20.0, Button::Left, NONE);
    let moved: Vec<(usize, Vec<i64>)> = f
        .take_events()
        .into_iter()
        .filter_map(|e| match e {
            crate::KernelEvent::Container(crate::components::form::Container::Dock { action: User::Split { splitter, extents, done: true }, .. }) => Some((splitter, extents)),
            _ => None,
        })
        .collect();
    let start = |i: usize| g.extents.iter().find(|(p, _)| *p == g.splitters[i].path).unwrap().1.clone();
    let (sv, sh) = (&g.splitters[c.stands], &g.splitters[c.lies]);
    assert!(moved.iter().any(|(i, e)| *i == c.stands && e[sv.index] == start(c.stands)[sv.index] + 30), "{moved:?}");
    assert!(moved.iter().any(|(i, e)| *i == c.lies && e[sh.index] == start(c.lies)[sh.index] - 20), "{moved:?}");
    assert!(manager::with("gck", |m| m.ui.split.is_none() && m.ui.split2.is_none()).unwrap(), "let go");
    rapidr_value::theme::set(&rapidr_value::theme::CLASSIC);
}

#[test]
fn a_dock_splitter_takes_the_keyboard_and_gives_it_back() {
    use rapidr_value::dock::manager::{self, User, SPLIT_JUMP, SPLIT_STEP};
    use rapidr_value::dock::Axis;
    let (s, g) = docked("gb");
    let (mut f, mut ts) = built(&s, "gbf");
    // (the keyboard in Output's field, then FocusSplitter beside it: the
    // splitter above Output, the dock manager holding the keyboard)
    assert!(f.focus_id(&s, "gbout"));
    assert!(manager::with_mut("gbk", |m| m.focus_splitter(Some("gbout"))));
    assert!(f.focus_id(&s, "gbk"), "the dock manager takes the keyboard while a splitter has it");
    let k = manager::with("gbk", |m| m.ui.split_key.clone()).flatten().unwrap();
    assert_eq!(k.back.as_deref(), Some("gbout"), "where it goes back to");
    assert_eq!(g.splitters[k.splitter].axis, Axis::Column);
    let mods = |shift| crate::Mods { shift, ..NONE };
    let step = |f: &mut crate::FormUi| {
        f.take_events().into_iter().find_map(|e| match e {
            crate::KernelEvent::Container(crate::components::form::Container::Dock { action: User::Split { extents, .. }, .. }) => Some(extents),
            _ => None,
        })
    };
    let start = g.extents.iter().find(|(p, _)| *p == g.splitters[k.splitter].path).unwrap().1.clone();
    let mut clip = crate::input::MemClipboard(None);
    // (Down: 10 pixels; Shift+Up: 50 back)
    f.key_down(&s, &mut ts, 40, "", mods(false), &mut clip);
    assert_eq!(step(&mut f).unwrap()[0], start[0] + SPLIT_STEP);
    f.key_down(&s, &mut ts, 38, "", mods(true), &mut clip);
    assert_eq!(step(&mut f).unwrap()[0], start[0] - SPLIT_JUMP, "(from where the model has it)");
    // (drawn as given the keyboard: the focus ring over it)
    let shown = f.paint(&s, &mut ts, 1.0).dump();
    assert!(shown.contains(&format!("#{:06x}", rapidr_value::theme::RAPIDR.focus)), "the focus ring");
    // (the screen reader's focus on the splitter, its value a percentage)
    fn focused(n: &rapidr_value::objects::a11y::AccessNode) -> Option<rapidr_value::objects::a11y::AccessNode> {
        if n.states.focused {
            return Some(n.clone());
        }
        n.children.iter().find_map(focused)
    }
    let tree = f.access_tree(&s, &mut ts);
    let sp = focused(&tree).expect("a focused node");
    assert_eq!(sp.role, rapidr_value::objects::a11y::Role::Splitter, "{sp:?}");
    assert!(sp.value.as_deref().is_some_and(|v| v.ends_with('%')), "{:?}", sp.value);
    // (Escape: the keyboard back in the field it came from)
    f.key_down(&s, &mut ts, 27, "", mods(false), &mut clip);
    assert_eq!(f.focus.map(|i| f.nodes[i].id.clone()).as_deref(), Some("gbout"));
    assert!(manager::with("gbk", |m| m.ui.split_key.is_none()).unwrap());
    rapidr_value::theme::set(&rapidr_value::theme::CLASSIC);
}

#[test]
fn a_status_bars_size_grip_is_the_windows_sizing_corner() {
    rapidr_value::theme::set(&rapidr_value::theme::CLASSIC);
    let mut s = MemStore::new();
    s.add("gf", "RFORM", None);
    s.add("bar", "RSTATUSBAR", Some("gf")).set("bar", "align", v_int(2));
    place(&mut s, "bar", (0, 76, 200, 24));
    let (mut f, mut ts) = built(&s, "gf");
    assert_eq!(at(&mut f, &s, &mut ts, 195, 96), Cursor::SizeNWSE);
    assert_eq!(at(&mut f, &s, &mut ts, 100, 90), Cursor::Default);
    // (held: the corner's arrow wherever the mouse goes)
    f.mouse_down(&s, &mut ts, 195.5, 96.5, Button::Left, NONE);
    assert_eq!(at(&mut f, &s, &mut ts, 100, 40), Cursor::SizeNWSE);
    f.mouse_up(&s, &mut ts, 100.5, 40.5, Button::Left, NONE);
    assert_eq!(at(&mut f, &s, &mut ts, 100, 40), Cursor::Default);
}

#[test]
fn an_mdi_childs_edges_and_corners_show_their_sizing_arrows() {
    rapidr_value::theme::set(&rapidr_value::theme::CLASSIC);
    let names = |h: i64| Some(format!("pmd({h})"));
    mdi::register("pform");
    let mut s = MemStore::new();
    s.add("pform", "RFORM", None);
    mdi::call("pform", "AddChild", &[v_int(0), v_str("One"), v_int(0), v_int(0), v_int(0), v_int(0), v_int(0), v_int(-1)], (600, 400), &names).unwrap();
    let fr = mdi::frames("pform")[0].clone();
    let name = mdi::frame_name("pform", &fr.component);
    s.add(&name, "RMDICHILD", Some("pform")).set(&name, "caption", v_str("One")).set(&name, "__form", v_str("pform")).set(&name, "__component", v_str(&fr.component));
    place(&mut s, &name, (fr.rect.left, fr.rect.top, fr.rect.width, fr.rect.height));
    let (mut f, mut ts) = built(&s, "pform");
    let (l, t, w, h) = f.node(&name).unwrap().abs;
    assert_eq!(at(&mut f, &s, &mut ts, l, t + h / 2), Cursor::SizeWE, "left edge");
    assert_eq!(at(&mut f, &s, &mut ts, l + w - 1, t + h / 2), Cursor::SizeWE, "right edge");
    assert_eq!(at(&mut f, &s, &mut ts, l + w / 2, t), Cursor::SizeNS, "top edge");
    assert_eq!(at(&mut f, &s, &mut ts, l + w / 2, t + h - 1), Cursor::SizeNS, "bottom edge");
    assert_eq!(at(&mut f, &s, &mut ts, l, t), Cursor::SizeNWSE, "top left");
    assert_eq!(at(&mut f, &s, &mut ts, l + w - 1, t + h - 1), Cursor::SizeNWSE, "bottom right");
    assert_eq!(at(&mut f, &s, &mut ts, l + w - 1, t), Cursor::SizeNESW, "top right");
    assert_eq!(at(&mut f, &s, &mut ts, l, t + h - 1), Cursor::SizeNESW, "bottom left");
    assert_eq!(at(&mut f, &s, &mut ts, l + w / 2, t + h / 2), Cursor::Default, "inside");
    // (held on an edge: its arrows wherever the mouse goes; a title bar
    // being moved keeps the arrow)
    f.mouse_down(&s, &mut ts, l as f64 + 0.5, (t + h / 2) as f64 + 0.5, Button::Left, NONE);
    assert_eq!(at(&mut f, &s, &mut ts, l - 30, t + h / 2 + 40), Cursor::SizeWE);
    f.mouse_up(&s, &mut ts, l as f64, (t + h / 2) as f64, Button::Left, NONE);
    assert_eq!(at(&mut f, &s, &mut ts, l + w / 2, t + h / 2), Cursor::Default);
}

#[test]
fn dock_splitters_show_the_resize_pointer_and_a_carried_tab_the_closed_hand() {
    use rapidr_value::dock::manager;
    rapidr_value::theme::set(&rapidr_value::theme::CLASSIC);
    let mut s = MemStore::new();
    s.add("df", "RFORM", None);
    s.add("dk", "RDOCKMANAGER", Some("df"));
    place(&mut s, "dk", (0, 0, 800, 500));
    manager::with_mut("dk", |m| {
        m.resize((800, 500), &rapidr_value::objects::font::Font::default());
        m.add_pane("explorer", "Explorer", "left", "explorer");
        m.add_pane("output", "Output", "bottom:documents", "output");
        m.add_pane("one", "One", "documents", "code");
        m.add_pane("two", "Two", "documents", "code");
        m.layout.mode = rapidr_value::dock::DocumentMode::Tabs;
        m.layout.select("one");
        m.touch();
    });
    let g = manager::with_mut("dk", |m| m.geometry().clone());
    let d = g.documents.clone().unwrap();
    s.add("dk__docs", "RDOCKDOCS", Some("dk")).set("dk__docs", "__dock", v_str("dk"));
    place(&mut s, "dk__docs", d.rect);
    let (mut f, mut ts) = built(&s, "df");
    // (the splitters of the manager: left | documents is ↔, documents above
    // output is ↕)
    let mid = |r: (i64, i64, i64, i64)| (r.0 + r.2 / 2, r.1 + r.3 / 2);
    let across = g.splitters.iter().find(|sp| sp.axis == rapidr_value::dock::Axis::Row).expect("a splitter between left and right");
    let (x, y) = mid(across.rect);
    assert_eq!(at(&mut f, &s, &mut ts, x, y), Cursor::ColResize);
    let along = g.splitters.iter().find(|sp| sp.axis == rapidr_value::dock::Axis::Column).expect("a splitter between top and bottom");
    let (x, y) = mid(along.rect);
    assert_eq!(at(&mut f, &s, &mut ts, x, y), Cursor::RowResize);
    // (a document tab: the arrow at rest, the closed hand carried, the arrow
    // again let go)
    let gr = d.groups[0].clone();
    let (tx, ty) = (d.rect.0 + gr.tabs[1].rect.0 + 20, d.rect.1 + gr.tabs[1].rect.1 + 10);
    assert_eq!(at(&mut f, &s, &mut ts, tx, ty), Cursor::Default);
    f.mouse_down(&s, &mut ts, tx as f64 + 0.5, ty as f64 + 0.5, Button::Left, NONE);
    assert_eq!(at(&mut f, &s, &mut ts, tx + 4, ty), Cursor::Default, "a press is not yet a drag");
    assert_eq!(at(&mut f, &s, &mut ts, tx + 40, ty + 60), Cursor::Grabbing);
    f.mouse_up(&s, &mut ts, tx as f64 + 40.5, ty as f64 + 60.5, Button::Left, NONE);
    assert_eq!(at(&mut f, &s, &mut ts, tx, ty), Cursor::Default);
}

#[test]
fn a_header_sections_edge_and_a_grids_sizable_column_edge_show_the_column_resize_pointer() {
    rapidr_value::theme::set(&rapidr_value::theme::CLASSIC);
    let mut s = MemStore::new();
    s.add("hf", "RFORM", None);
    s.add("hd", "RHEADER", Some("hf"));
    place(&mut s, "hd", (10, 10, 300, 20));
    s.call("hd", "addsections", &[v_str("Name"), v_str("Size")]);
    s.add("gr", "RSTRINGGRID", Some("hf"));
    place(&mut s, "gr", (10, 50, 340, 140));
    let (mut f, mut ts) = built(&s, "hf");
    // (a header: the edge of "Name" is its width, plus the grip's reach)
    let width = rapidr_value::objects::with_header("hd", |h| h.sections[0].width).unwrap();
    assert_eq!(at(&mut f, &s, &mut ts, 10 + width, 20), Cursor::ColResize, "a section's edge");
    assert_eq!(at(&mut f, &s, &mut ts, 10 + width / 2, 20), Cursor::Default, "a section's middle");
    // (a grid: only a fixed row's cell border, and only with goColSizing)
    fn edge(f: &mut FormUi, s: &MemStore, ts: &mut TextSystem) -> Option<i64> {
        (12..350).find(|&x| at(f, s, ts, x, 50 + 8) == Cursor::ColResize)
    }
    assert_eq!(edge(&mut f, &s, &mut ts), None, "without goColSizing the grid doesn't size columns");
    s.call("gr", "addoptions", &[v_int(rapidr_value::objects::grid::GO_COL_SIZING as i64)]);
    f.sync(&s);
    let x = edge(&mut f, &s, &mut ts).expect("a fixed row's cell border");
    assert_eq!(at(&mut f, &s, &mut ts, x, 50 + 60), Cursor::Default, "below the fixed row");
    // (held: the arrows stay wherever the mouse goes)
    f.mouse_down(&s, &mut ts, x as f64 + 0.5, 58.5, Button::Left, NONE);
    assert_eq!(at(&mut f, &s, &mut ts, x + 40, 58 + 70), Cursor::ColResize);
    f.mouse_up(&s, &mut ts, x as f64 + 40.5, 128.5, Button::Left, NONE);
    assert_eq!(at(&mut f, &s, &mut ts, 20, 50 + 70), Cursor::Default);
    // (goRowSizing: a fixed column's cell border sizes the row, ↕)
    fn row_edge(f: &mut FormUi, s: &MemStore, ts: &mut TextSystem) -> Option<i64> {
        (52..190).find(|&y| at(f, s, ts, 20, y) == Cursor::RowResize)
    }
    assert_eq!(row_edge(&mut f, &s, &mut ts), None, "without goRowSizing the grid doesn't size rows");
    s.call("gr", "addoptions", &[v_int(rapidr_value::objects::grid::GO_ROW_SIZING as i64)]);
    f.sync(&s);
    let y = row_edge(&mut f, &s, &mut ts).expect("a fixed column's cell border");
    assert_eq!(at(&mut f, &s, &mut ts, 120, y), Cursor::Default, "not in the fixed column");
    let before = rapidr_value::objects::with_grid("gr", |g| g.row_heights.clone()).unwrap();
    f.mouse_down(&s, &mut ts, 20.5, y as f64 + 0.5, Button::Left, NONE);
    assert_eq!(at(&mut f, &s, &mut ts, 200, y + 30), Cursor::RowResize, "held: wherever the mouse goes");
    f.mouse_up(&s, &mut ts, 200.5, y as f64 + 30.5, Button::Left, NONE);
    let after = rapidr_value::objects::with_grid("gr", |g| g.row_heights.clone()).unwrap();
    assert_eq!(after.iter().zip(&before).filter(|(a, b)| a != b).count(), 1, "one row sized");
    assert!(after.iter().sum::<i64>() > before.iter().sum::<i64>());
}

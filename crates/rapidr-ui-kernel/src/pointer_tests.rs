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
    assert_eq!(at(&mut f, &s, &mut ts, 60, 130), Cursor::IBeam, "a code editor's text");
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
    // Delphi's TSplitter; crSizeWE is taken as asked)
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
    f.mouse_move(&s, &mut ts, 103.5, 50.5, NONE);
    let hot = f.paint(&s, &mut ts, 1.0).dump();
    assert_ne!(rest, hot, "a line along the splitter's middle");
    f.mouse_move(&s, &mut ts, 20.5, 50.5, NONE);
    assert_eq!(f.paint(&s, &mut ts, 1.0).dump(), rest, "and gone when the mouse leaves");
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

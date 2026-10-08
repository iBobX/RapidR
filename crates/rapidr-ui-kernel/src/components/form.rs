//! What a form's containers do that only runtime-core can carry out, as
//! [`KernelEvent::Container`](crate::KernelEvent::Container) events: the
//! kernel routes the mouse into the shared models (the scroll bars,
//! `rapidr_value::scrollbars`; a QFORMMDI's children, `rapidr_value::mdi`;
//! a QSPLITTER's drag) and runtime-core moves the components after the
//! pump — scrolling moves them (their Left / Top, as Delphi's ScrollBy), a
//! splitter resizes its neighbour (`layout::splitter_*`), an MDI child's
//! frame is moved, raised or closed (`mdi::Runtime`), with the program's
//! events (OnMoved, OnChildActive …) where the web runtime fires them.

use rapidr_value::mdi::Action;

/// A container's action for runtime-core.
#[derive(Clone, Debug, PartialEq)]
pub enum Container {
    /// The user scrolled form / QSCROLLBOX `id`: its components move by
    /// (-dx, -dy) (`scroll::user_scrolled`).
    Scrolled { id: String, dx: i64, dy: i64 },
    /// A QSPLITTER pressed: its drag starts (`layout::splitter_begin`).
    SplitBegin(String),
    /// The pressed splitter dragged `delta` pixels along its axis from where
    /// it was pressed (`layout::splitter_move`).
    SplitMove(i64),
    /// The splitter let go: OnMoved (`layout::splitter_end`).
    SplitEnd,
    /// A QFORMMDI child's frame used: `form`'s child showing `component`
    /// raised, moved, resized, minimized, maximized or closed
    /// (`mdi::rt_user`).
    Mdi { form: String, component: String, action: Action },
    /// (the input lane's) A QSTATUSBAR's size grip dragged: form `form`'s
    /// window's inside asked to be `w` × `h` (logical; its in-window menu
    /// included). The host's, not runtime-core's: `Desktop` makes it a
    /// `HostCmd::Resize`, and the window's resize comes back as the user's.
    Resize { form: String, w: i64, h: i64 },
    /// (I1) What the user did to RDOCKMANAGER `id`: a tab or header
    /// pressed, a button, a pane dropped on the compass or away from it, a
    /// splitter dragged, a flyout slid out (`rapidr_value::dock::runtime::
    /// rt_user`).
    Dock { id: String, action: rapidr_value::dock::manager::User },
    /// (I1 / L-PANELS) What the user did to panel `id` (an inspector, a
    /// toolbox, a project tree, a console, a toolbar, a palette:
    /// `rapidr_value::panels::runtime::rt_user`).
    Panel { id: String, action: rapidr_value::panels::User },
}

#[cfg(test)]
mod tests {
    use rapidr_value::input::{Button, Mouse};
    use rapidr_value::layout::Align;
    use rapidr_value::objects::a11y::Role;
    use rapidr_value::scrollbars::{self, Child};
    use rapidr_value::{mdi, v_int, v_str};

    use super::Container;
    use crate::{FormUi, Item, KernelEvent, MemStore, Mods, Op, TextSystem};

    const NONE: Mods = Mods::NONE;

    fn container(events: &[KernelEvent]) -> Vec<Container> {
        events.iter().filter_map(|e| if let KernelEvent::Container(c) = e { Some(c.clone()) } else { None }).collect()
    }

    fn mouse_downs(events: &[KernelEvent]) -> usize {
        events.iter().filter(|e| matches!(e, KernelEvent::Mouse { kind: Mouse::Down, .. })).count()
    }

    fn place(s: &mut MemStore, id: &str, (l, t, w, h): (i64, i64, i64, i64)) {
        s.set(id, "left", v_int(l)).set(id, "top", v_int(t)).set(id, "width", v_int(w)).set(id, "height", v_int(h));
    }

    /// autoscroll.bas's box: a 150 × 100 QSCROLLBOX (its edge 2 pixels)
    /// with a label reaching past its right side.
    #[test]
    fn scroll_box_bars_take_the_mouse_first() {
        let mut s = MemStore::new();
        s.add("sform", "RFORM", None);
        s.add("sbox", "RSCROLLBOX", Some("sform"));
        place(&mut s, "sbox", (10, 10, 150, 100));
        s.add("slbl", "RLABEL", Some("sbox")).set("slbl", "caption", v_str("right"));
        place(&mut s, "slbl", (200, 5, 100, 25));
        let kid = Child { left: 200, top: 5, width: 100, height: 25, align: Align::None, visible: true };
        scrollbars::with_mut("sbox", |b| b.update(146, 96, &[kid]));
        let mut ts = TextSystem::new();
        let mut f = FormUi::build(&s, "sform", false);
        let list = f.paint(&s, &mut ts, 1.0);
        assert_eq!(f.node("slbl").unwrap().abs, (212, 17, 100, 25), "a scroll box's components sit inside its edge");
        // (the bars after the label: drawn over the components)
        let label_at = list.items.iter().position(|i| matches!(i, Item::Op { op: Op::Text { text, .. }, .. } if text == "right")).unwrap();
        let arrow_at = list.items.iter().rposition(|i| matches!(i, Item::Op { op: Op::Arrow { .. }, .. })).unwrap();
        assert!(arrow_at > label_at);
        // The right arrow (at 140, 90 of the box): the components move by
        // the Increment, no OnMouseDown.
        f.mouse_down(&s, &mut ts, 150.5, 100.5, Button::Left, NONE);
        f.mouse_up(&s, &mut ts, 150.5, 100.5, Button::Left, NONE);
        let ev = f.take_events();
        assert_eq!(container(&ev), vec![Container::Scrolled { id: "sbox".into(), dx: 8, dy: 0 }]);
        assert_eq!(mouse_downs(&ev), 0);
        // Inside the box, off its bars: OnMouseDown.
        f.mouse_down(&s, &mut ts, 60.5, 40.5, Button::Left, NONE);
        let ev = f.take_events();
        assert!(container(&ev).is_empty());
        assert_eq!(mouse_downs(&ev), 1);
        scrollbars::remove("sbox");
    }

    #[test]
    fn panel_group_box_and_status_bar_draw_and_describe() {
        let mut s = MemStore::new();
        s.add("pform", "RFORM", None);
        s.add("ppanel", "RPANEL", Some("pform")).set("ppanel", "caption", v_str("Hi"));
        place(&mut s, "ppanel", (0, 0, 100, 60));
        s.set("ppanel", "bevelouter", v_int(1)).set("ppanel", "bevelinner", v_int(2)).set("ppanel", "borderwidth", v_int(3));
        s.add("pgroup", "RGROUPBOX", Some("pform")).set("pgroup", "caption", v_str("&Group"));
        place(&mut s, "pgroup", (0, 70, 100, 60));
        s.add("pbar", "RSTATUSBAR", Some("pform")).set("pbar", "panelcount", v_int(2)).set("pbar", "panel(0).caption", v_str("Ready")).set("pbar", "panel(1).caption", v_str("INS"));
        place(&mut s, "pbar", (0, 140, 300, 24));
        let mut ts = TextSystem::new();
        let mut f = FormUi::build(&s, "pform", false);
        let dump = f.paint(&s, &mut ts, 1.0).dump();
        // the panel's bevels: lowered outside, 3 pixels, raised inside
        assert!(dump.contains("edge 0,0 100x60 #a0a0a0 #ffffff @0,0"), "{dump}");
        assert!(dump.contains("edge 4,4 92x52 #ffffff #a0a0a0 @0,0"), "{dump}");
        assert!(dump.contains("\"Hi\""), "{dump}");
        // the group box: etched, its caption without the &
        assert!(dump.contains("#a0a0a0/#ffffff #ffffff/#a0a0a0 @0,70"), "{dump}");
        assert!(dump.contains("\"Group\""), "{dump}");
        // the status bar: the first panel 100 wide by default, the last the
        // rest — up to the size grip (the input lane's: a sizeable form, the
        // bar docked at the bottom)
        assert!(dump.contains("edge 1,2 98x21 #a0a0a0 #ffffff @0,140"), "{dump}");
        assert!(dump.contains("edge 101,2 182x21 #a0a0a0 #ffffff @0,140"), "{dump}");
        let tree = f.access_tree(&s, &mut ts);
        let roles: Vec<(Role, String)> = tree.children.iter().map(|n| (n.role, n.name.clone())).collect();
        // (the status bar: a polite live region, as the web's role=status)
        assert_eq!(roles, vec![(Role::Pane, "Hi".into()), (Role::Group, "Group".into()), (Role::Status, String::new())]);
        assert_eq!(tree.children[2].children.iter().map(|n| n.name.as_str()).collect::<Vec<_>>(), ["Ready", "INS"]);
        assert!(f.focused().is_none(), "containers don't take the focus");
    }

    #[test]
    fn splitter_drag_is_measured_in_the_window() {
        let mut s = MemStore::new();
        s.add("tform", "RFORM", None);
        s.add("tsplit", "RSPLITTER", Some("tform"));
        place(&mut s, "tsplit", (100, 0, 5, 200));
        let mut ts = TextSystem::new();
        let mut f = FormUi::build(&s, "tform", false);
        drop(f.paint(&s, &mut ts, 1.0));
        f.mouse_down(&s, &mut ts, 102.0, 50.0, Button::Left, NONE);
        // (the splitter follows the drag before the next move)
        s.set("tsplit", "left", v_int(110));
        f.sync(&s);
        f.mouse_move(&s, &mut ts, 122.0, 50.0, NONE);
        f.mouse_up(&s, &mut ts, 122.0, 50.0, Button::Left, NONE);
        let ev = f.take_events();
        assert_eq!(container(&ev), vec![Container::SplitBegin("tsplit".into()), Container::SplitMove(20), Container::SplitEnd]);
    }

    #[test]
    fn mdi_children_stack_and_take_the_mouse() {
        let names = |h: i64| Some(format!("med({h})"));
        mdi::register("mform");
        let mut s = MemStore::new();
        s.add("mform", "RFORM", None);
        s.add("mbutton", "RBUTTON", Some("mform"));
        let args = |h: i64, t: &str| [v_int(h), v_str(t), v_int(h), v_int(0), v_int(0), v_int(0), v_int(0), v_int(-1)];
        for (h, t) in [(0, "One"), (1, "Two")] {
            mdi::call("mform", "AddChild", &args(h, t), (600, 400), &names).unwrap();
        }
        // (what mdi::render makes: a frame per child)
        for fr in mdi::frames("mform") {
            let name = mdi::frame_name("mform", &fr.component);
            s.add(&name, "RMDICHILD", Some("mform")).set(&name, "caption", v_str(&fr.title)).set(&name, "__form", v_str("mform")).set(&name, "__component", v_str(&fr.component));
            place(&mut s, &name, (fr.rect.left, fr.rect.top, fr.rect.width, fr.rect.height));
        }
        // the one the model raises is on top, whatever the creation order
        mdi::call("mform", "ActiveNextChild", &[], (600, 400), &names).unwrap();
        let mut ts = TextSystem::new();
        let mut f = FormUi::build(&s, "mform", false);
        let order: Vec<&str> = f.nodes.iter().map(|n| n.id.as_str()).collect();
        assert_eq!(order, ["mbutton", "mform__mdi__med(1)", "mform__mdi__med(0)"]);
        drop(f.paint(&s, &mut ts, 1.0));
        // the top one's close button; then its title bar dragged
        let (l, t, w, _) = f.node("mform__mdi__med(0)").unwrap().abs;
        let close = ((l + w - 8) as f64, (t + 10) as f64);
        f.mouse_down(&s, &mut ts, close.0, close.1, Button::Left, NONE);
        f.mouse_up(&s, &mut ts, close.0, close.1, Button::Left, NONE);
        f.mouse_down(&s, &mut ts, (l + 30) as f64, (t + 10) as f64, Button::Left, NONE);
        f.mouse_move(&s, &mut ts, (l + 40) as f64, (t + 15) as f64, NONE);
        f.mouse_up(&s, &mut ts, (l + 40) as f64, (t + 15) as f64, Button::Left, NONE);
        let mdi_of = |c: Container| if let Container::Mdi { action, .. } = c { Some(action) } else { None };
        let actions: Vec<mdi::Action> = container(&f.take_events()).into_iter().filter_map(mdi_of).collect();
        assert_eq!(actions, vec![mdi::Action::Close, mdi::Action::Activate, mdi::Action::Move(l + 10, t + 5)]);
    }

    /// Tabbed documents: a tab dragged to a group's right quarter splits it,
    /// along the strip reorders, a view switch's segment shows that view,
    /// the splitter between groups drags.
    #[test]
    fn document_tabs_drag_to_split_and_switch_views() {
        use rapidr_value::dock::manager::{self, DocView, User};
        use rapidr_value::dock::{DocTarget, DocumentMode, Side};
        let mut s = MemStore::new();
        s.add("kf", "RFORM", None);
        s.add("kd", "RDOCKMANAGER", Some("kf"));
        place(&mut s, "kd", (0, 0, 800, 500));
        manager::with_mut("kd", |m| {
            m.resize((800, 500), &rapidr_value::objects::font::Font::default());
            m.layout.mode = DocumentMode::Tabs;
            for d in ["one", "two", "three"] {
                m.add_pane(d, d, "documents", "code");
            }
            m.add_view("one", "onedesign", "Design");
            m.add_view("one", "one", "Code");
            m.layout.select("one");
            m.touch();
        });
        let d = manager::with_mut("kd", |m| m.geometry().documents.clone()).unwrap();
        s.add("kd__docs", "RDOCKDOCS", Some("kd")).set("kd__docs", "__dock", v_str("kd"));
        place(&mut s, "kd__docs", d.rect);
        let mut ts = TextSystem::new();
        let mut f = FormUi::build(&s, "kf", false);
        drop(f.paint(&s, &mut ts, 1.0));
        let dock_of = |c: Container| if let Container::Dock { action, .. } = c { Some(action) } else { None };
        let gr = d.groups[0].clone();
        let at = |r: (i64, i64, i64, i64), dx: i64, dy: i64| ((d.rect.0 + r.0 + dx) as f64 + 0.5, (d.rect.1 + r.1 + dy) as f64 + 0.5);
        // (tab "three" dragged to the group's right quarter)
        let (x, y) = at(gr.tabs[2].rect, 20, 10);
        let (tx, ty) = at(gr.content, gr.content.2 - 30, gr.content.3 / 2);
        f.mouse_down(&s, &mut ts, x, y, Button::Left, NONE);
        f.mouse_move(&s, &mut ts, x + 20.0, y + 40.0, NONE);
        f.mouse_move(&s, &mut ts, tx, ty, NONE);
        let preview = manager::with("kd", |m| m.ui.doc_drag.as_ref().and_then(|dd| dd.preview)).flatten();
        assert_eq!(preview.map(|p| p.2), Some(gr.content.2 / 2), "the half it would take, outlined");
        f.mouse_up(&s, &mut ts, tx, ty, Button::Left, NONE);
        let acts: Vec<User> = container(&f.take_events()).into_iter().filter_map(dock_of).collect();
        assert_eq!(acts, vec![User::Select("three".into()), User::MoveDocument("three".into(), DocTarget::Split { anchor: "one".into(), side: Side::Right })]);
        // (along the strip: before the first tab)
        let (x, y) = at(gr.tabs[1].rect, 20, 10);
        let (bx, by) = at(gr.tabs[0].rect, 4, 10);
        f.mouse_down(&s, &mut ts, x, y, Button::Left, NONE);
        f.mouse_move(&s, &mut ts, bx, by, NONE);
        f.mouse_up(&s, &mut ts, bx, by, Button::Left, NONE);
        let acts: Vec<User> = container(&f.take_events()).into_iter().filter_map(dock_of).collect();
        assert_eq!(acts.last(), Some(&User::MoveDocument("two".into(), DocTarget::Into { anchor: "one".into(), index: 0 })));
        // (the switch's "Code" segment)
        let (cx, cy) = at(gr.switch[1].2, 5, 5);
        f.mouse_down(&s, &mut ts, cx, cy, Button::Left, NONE);
        f.mouse_up(&s, &mut ts, cx, cy, Button::Left, NONE);
        let acts: Vec<User> = container(&f.take_events()).into_iter().filter_map(dock_of).collect();
        assert_eq!(acts, vec![User::View("one".into(), DocView::One(1))]);
    }

    /// A QFORMMDI child sizes by any edge or corner (Windows' sizing
    /// border), never under the least size; a maximized one doesn't.
    #[test]
    fn mdi_children_resize_by_every_edge_and_corner() {
        let names = |h: i64| Some(format!("red({h})"));
        mdi::register("rform");
        let mut s = MemStore::new();
        s.add("rform", "RFORM", None);
        mdi::call("rform", "AddChild", &[v_int(0), v_str("One"), v_int(0), v_int(0), v_int(0), v_int(0), v_int(0), v_int(-1)], (600, 400), &names).unwrap();
        let fr = mdi::frames("rform")[0].clone();
        let name = mdi::frame_name("rform", &fr.component);
        s.add(&name, "RMDICHILD", Some("rform")).set(&name, "caption", v_str("One")).set(&name, "__form", v_str("rform")).set(&name, "__component", v_str(&fr.component));
        place(&mut s, &name, (fr.rect.left, fr.rect.top, fr.rect.width, fr.rect.height));
        let mut ts = TextSystem::new();
        let mut f = FormUi::build(&s, "rform", false);
        drop(f.paint(&s, &mut ts, 1.0));
        let (l, t, w, h) = f.node(&name).unwrap().abs;
        fn drag(s: &MemStore, ts: &mut TextSystem, f: &mut FormUi, from: (i64, i64), by: (i64, i64)) -> Vec<mdi::Action> {
            let mdi_of = |c: Container| if let Container::Mdi { action, .. } = c { Some(action) } else { None };
            let (x, y) = (from.0 as f64 + 0.5, from.1 as f64 + 0.5);
            f.mouse_down(s, ts, x, y, Button::Left, NONE);
            f.mouse_move(s, ts, x + by.0 as f64, y + by.1 as f64, NONE);
            f.mouse_up(s, ts, x + by.0 as f64, y + by.1 as f64, Button::Left, NONE);
            container(&f.take_events()).into_iter().filter_map(mdi_of).filter(|a| *a != mdi::Action::Activate).collect()
        }
        // each edge from its middle, each corner
        assert_eq!(drag(&s, &mut ts, &mut f, (l, t + h / 2), (-20, 7)), [mdi::Action::Resize(l - 20, t, w + 20, h)]);
        assert_eq!(drag(&s, &mut ts, &mut f, (l + w - 1, t + h / 2), (30, 0)), [mdi::Action::Resize(l, t, w + 30, h)]);
        assert_eq!(drag(&s, &mut ts, &mut f, (l + w / 2, t), (0, -10)), [mdi::Action::Resize(l, t - 10, w, h + 10)]);
        assert_eq!(drag(&s, &mut ts, &mut f, (l + w / 2, t + h - 1), (5, 25)), [mdi::Action::Resize(l, t, w, h + 25)]);
        assert_eq!(drag(&s, &mut ts, &mut f, (l, t), (10, 10)), [mdi::Action::Resize(l + 10, t + 10, w - 10, h - 10)]);
        assert_eq!(drag(&s, &mut ts, &mut f, (l + w - 1, t), (10, -10)), [mdi::Action::Resize(l, t - 10, w + 10, h + 10)]);
        assert_eq!(drag(&s, &mut ts, &mut f, (l, t + h - 1), (-10, 10)), [mdi::Action::Resize(l - 10, t, w + 10, h + 10)]);
        assert_eq!(drag(&s, &mut ts, &mut f, (l + w - 1, t + h - 1), (15, 15)), [mdi::Action::Resize(l, t, w + 15, h + 15)]);
        // (never smaller than Windows' least)
        assert_eq!(drag(&s, &mut ts, &mut f, (l + w - 1, t + h - 1), (-2000, -2000)), [mdi::Action::Resize(l, t, mdi::MIN_TRACK.0, mdi::MIN_TRACK.1)]);
        // (inside the border: the title bar moves it, the client does nothing)
        assert_eq!(drag(&s, &mut ts, &mut f, (l + 30, t + 10), (4, 4)), [mdi::Action::Move(l + 4, t + 4)]);
        // (maximized: no sizing border)
        s.set(&name, "childstate", v_int(2));
        f.sync(&s);
        assert!(!drag(&s, &mut ts, &mut f, (l, t + h / 2), (-20, 0)).iter().any(|a| matches!(a, mdi::Action::Resize(..))));
    }
}

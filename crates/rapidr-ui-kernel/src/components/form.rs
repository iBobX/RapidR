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
}

//! RDESIGNSURFACE (Stage 10): RapidR's form designer — the shared model
//! (`rapidr_value::objects::design`: the designed components, the
//! selection, the grid-snapped moves and resizes) drawn and driven by the
//! kernel:
//!
//! - **The look** is the model's ops: white with grey dots on the 8-pixel
//!   grid, each designed component as a placeholder of its type (a raised
//!   button, a sunken edit, a check box's box …, with its Caption, Color,
//!   FontColor and Font), the selected one framed in blue with its eight
//!   handles.
//! - **The mouse** (the left button): a press grabs the selected one's
//!   right, bottom or corner handle, or selects the component under it
//!   (OnSelect; OnDblClick for a double click's second press), or clears
//!   the selection (OnBgClick with the point); a drag moves or resizes on
//!   the grid (OnMove at each step); the release ends it.
//! - It takes **no focus and no keys**, fires no OnClick, and the program
//!   hears no OnMouseDown / Move / Up of it: runtime-core drops those (the
//!   designer takes the mouse whole).
//!
//! A screen reader sees a list box whose options are the designed
//! components ("Button1 (RBUTTON)"), the selected one selected; clicking
//! one selects it (OnSelect).

use rapidr_value::objects::a11y::{node_id, part_id, AccessNode, Action, Role, PART_ITEM};
use rapidr_value::objects::design::DesignEvent;
use rapidr_value::objects::{with_design, with_design_mut};
use rapidr_value::{input::Button, v_int};

use super::list::{act, ListAction};
use super::{ComponentKind, Cx, MouseIn, MouseKind, MouseOut};
use crate::a11y::AccessValue;
use crate::paint::Painter;
use crate::store::Store;

pub struct Design;

/// What the program hears, queued (after the pump, as its handlers run).
fn heard(cx: &mut Cx, e: DesignEvent) {
    let args = e.args().into_iter().map(v_int).collect();
    act(cx, ListAction::Fire(e.event().to_string(), args));
}

impl ComponentKind for Design {
    fn name(&self) -> &'static str {
        "RDESIGNSURFACE"
    }

    fn focusable(&self, _store: &dyn Store, _id: &str) -> bool {
        false
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let (w, h) = (cx.width(), cx.height());
        // (the surface is the designed form's inside: its size is the form's)
        with_design_mut(cx.id, |d| d.set_size(w, h));
        p.ops(with_design(cx.id, |d| d.ops(w, h)).unwrap_or_default());
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        let out = MouseOut { press: false, focus: Some(false) };
        if m.button != Button::Left {
            return out;
        }
        let (x, y) = (m.x.floor() as i64, m.y.floor() as i64);
        let (w, h) = (cx.width(), cx.height());
        with_design_mut(cx.id, |d| d.set_size(w, h));
        // (Shift / Ctrl / Cmd+click: in or out of the selection; Alt /
        // Option: no snapping)
        let add = m.mods.shift || m.mods.ctrl || m.mods.command;
        let e = match m.kind {
            MouseKind::Down => with_design_mut(cx.id, |d| d.mouse_down_with(x, y, m.clicks >= 2, add)),
            MouseKind::Move if m.captured => with_design_mut(cx.id, |d| d.mouse_drag_with(x, y, m.mods.alt)),
            MouseKind::Up => with_design_mut(cx.id, |d| {
                d.mouse_up();
                None
            }),
            _ => None,
        };
        if let Some(e) = e.flatten() {
            heard(cx, e);
        }
        out
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        let mut n = AccessNode::new(node_id(cx.id), Role::ListBox);
        n.name = with_design(cx.id, |d| d.form_caption.clone()).unwrap_or_default();
        n.bounds = cx.rect;
        let (x0, y0) = (cx.rect.0, cx.rect.1);
        let comps = with_design(cx.id, |d| {
            let sel = d.selected();
            d.components().iter().enumerate().map(|(i, c)| (format!("{} ({})", c.name, c.type_name), c.bounds(), sel.contains(&i))).collect::<Vec<_>>()
        })
        .unwrap_or_default();
        for (i, (name, (x, y, w, h), selected)) in comps.into_iter().enumerate() {
            let mut o = AccessNode::new(part_id(cx.id, PART_ITEM, i), Role::ListBoxOption);
            o.name = name;
            o.states.selected = Some(selected);
            o.actions = vec![Action::Click];
            o.bounds = (x0 + x, y0 + y, w, h);
            n.children.push(o);
        }
        n
    }

    fn access(&self, cx: &mut Cx, action: Action, part: Option<usize>, _value: Option<&AccessValue>) -> bool {
        let (Action::Click, Some(i)) = (action, part) else { return false };
        let picked = with_design_mut(cx.id, |d| d.select(i));
        if picked == Some(true) {
            heard(cx, DesignEvent::Select(i));
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use rapidr_value::objects::ops::Op;
    use rapidr_value::{v_int, v_str, Value};

    use crate::components::list::ListAction;
    use crate::display::Item;
    use crate::{FormUi, KernelEvent, MemStore, Mods, TextSystem};

    fn designer() -> (MemStore, FormUi, TextSystem) {
        let mut s = MemStore::new();
        s.add("df", "RFORM", None).set("df", "clientwidth", v_int(400)).set("df", "clientheight", v_int(300));
        s.add("ds", "RDESIGNSURFACE", Some("df")).set("ds", "left", v_int(10)).set("ds", "top", v_int(20)).set("ds", "width", v_int(200)).set("ds", "height", v_int(160));
        s.add("ed", "REDIT", Some("df")).set("ed", "left", v_int(220)).set("ed", "top", v_int(20));
        s.call("ds", "addcomponent", &[v_str("RBUTTON"), v_str("Button1"), v_int(16), v_int(16), v_int(80), v_int(24)]);
        let f = FormUi::build(&s, "df", false);
        (s, f, TextSystem::new())
    }

    fn fired(events: Vec<KernelEvent>) -> Vec<(String, Vec<i64>)> {
        events
            .into_iter()
            .filter_map(|e| match e {
                KernelEvent::List(id, ListAction::Fire(ev, args)) if id == "ds" => Some((ev, args.iter().map(Value::to_i64).collect())),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_mouse_selects_moves_and_clears_with_the_designers_events() {
        let (s, mut f, mut ts) = designer();
        drop(f.paint(&s, &mut ts, 1.0));
        f.focus_id(&s, "ed");
        // a press on Button1 (at 30, 30 of the surface), a drag, a release
        f.mouse_down(&s, &mut ts, 40.5, 50.5, rapidr_value::input::Button::Left, Mods::NONE);
        f.mouse_move(&s, &mut ts, 53.5, 56.5, Mods::NONE);
        f.mouse_up(&s, &mut ts, 53.5, 56.5, rapidr_value::input::Button::Left, Mods::NONE);
        let events = f.take_events();
        assert_eq!(fired(events.clone()), [("onselect".to_string(), vec![0]), ("onmove".to_string(), vec![0, 32, 24, 80, 24])]);
        assert!(!events.iter().any(|e| matches!(e, KernelEvent::Click(_))), "no OnClick");
        assert_eq!(f.focus.map(|i| f.nodes[i].id.clone()).as_deref(), Some("ed"), "the focus stays");
        // the background, then a double click on the button
        f.mouse_down(&s, &mut ts, 160.5, 140.5, rapidr_value::input::Button::Left, Mods::NONE);
        f.mouse_up(&s, &mut ts, 160.5, 140.5, rapidr_value::input::Button::Left, Mods::NONE);
        f.mouse_down(&s, &mut ts, 60.5, 60.5, rapidr_value::input::Button::Left, Mods::NONE);
        f.mouse_up(&s, &mut ts, 60.5, 60.5, rapidr_value::input::Button::Left, Mods::NONE);
        f.mouse_down(&s, &mut ts, 60.5, 60.5, rapidr_value::input::Button::Left, Mods::NONE);
        assert_eq!(fired(f.take_events()), [("onbgclick".to_string(), vec![150, 120]), ("onselect".to_string(), vec![0]), ("ondblclick".to_string(), vec![0])]);
    }

    #[test]
    fn it_paints_the_models_ops_and_describes_its_components() {
        let (s, mut f, mut ts) = designer();
        let list = f.paint(&s, &mut ts, 1.0);
        let at = |o: &Op| list.items.iter().any(|i| matches!(i, Item::Op { origin: (10, 20), op } if op == o));
        assert!(at(&Op::Fill { rect: (0, 0, 200, 160), color: rapidr_value::theme::current().face }), "{}", list.dump());
        assert!(list.items.iter().any(|i| matches!(i, Item::Op { origin: (10, 20), op: Op::Text { text, .. } } if text == "Button1")));
        let tree = f.access_tree(&s, &mut ts);
        let json = tree.to_json();
        assert!(json.contains("Button1 (RBUTTON)") && json.contains("\"listbox\""), "{json}");
    }
}

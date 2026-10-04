//! QLISTVIEW: the shared model (`rapidr_value::objects::listview`) does
//! everything — its four views, the header, check boxes, image lists,
//! selection, the mouse, the keys and the wheel — and paints the control
//! as a bitmap (`listview_paint`), which the kernel shows as a picture (as
//! FLTK's frame and the web's canvas do). What the user did comes back as
//! the model's events: OnClick, OnDblClick, OnColumnClick (Column),
//! OnChange (Index, Change), and F2 / a click on the selected item edit
//! its caption in place (Enter keeps it: OnChange (Index, ctText)).

use rapidr_value::objects::a11y::{node_id, part_id, AccessNode, Action, Role};
use rapidr_value::objects::listview::Event;
use rapidr_value::objects::{listview_paint, listview_setup, with_listview, with_listview_mut};
use rapidr_value::{v_int, Value};

use super::list::{begin_edit, double_click, edit_key, editing, end_edit, fire, paint_edit, picture_of, set_edit_text, InPlace};
use super::{ComponentKind, Cx, KeyIn, MouseIn, MouseKind, MouseOut};
use crate::a11y::AccessValue;
use crate::input::Clipboard;
use crate::paint::Painter;

pub struct ListViewBox;

/// A list view's Color as &HBBGGRR (white unless the program says).
fn background(cx: &Cx) -> u32 {
    match cx.store.get(cx.id, "color") {
        Value::Null => 0xFFFFFF,
        Value::String(s) if s.is_empty() => 0xFFFFFF,
        v => rapidr_value::objects::form_color(&v) as u32,
    }
}

/// The model told its control's size, font and focus (before it paints or
/// takes input).
fn setup(cx: &Cx) {
    listview_setup(cx.id, cx.width(), cx.height(), &cx.font, cx.state.focused);
}

/// The model's events, for the program (an edit starts here).
fn events(cx: &mut Cx, events: Vec<Event>) {
    for e in events {
        match e {
            Event::Click => cx.click(),
            Event::DblClick => fire(cx, "ondblclick", Vec::new()),
            Event::ColumnClick(i) => fire(cx, "oncolumnclick", vec![v_int(i as i64)]),
            Event::Change(i, ct) => fire(cx, "onchange", vec![v_int(i as i64), v_int(ct)]),
            // (a click on the selected item: Windows edits it after a pause
            // unless a double click comes; the kernel's timers are the
            // runtime's, so it waits for F2 — the text lane's follow-up)
            Event::EditSoon(_) => {}
            Event::Edit(i) => {
                let rect = with_listview_mut(cx.id, |lv| lv.editor_rect(i)).flatten();
                let text = with_listview(cx.id, |lv| lv.items.get(i).map(|it| it.caption.clone())).flatten();
                if let (Some((l, t, r, b)), Some(text)) = (rect, text) {
                    begin_edit(cx.id, InPlace { target: (i, 0), text, rect: Some((l, t, r - l, b - t)) });
                }
            }
        }
    }
}

/// The caption's edit ends: kept, the item gets it (OnChange (Index,
/// ctText)).
fn finish_edit(cx: &mut Cx, keep: bool) {
    let Some(ed) = end_edit(cx.id) else { return };
    if keep {
        let ev = with_listview_mut(cx.id, |lv| lv.edited(ed.target.0, ed.text)).unwrap_or_default();
        events(cx, ev);
    }
}

impl ComponentKind for ListViewBox {
    fn name(&self) -> &'static str {
        "RLISTVIEW"
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        setup(cx);
        let Some(mut b) = listview_paint(cx.id, background(cx)) else { return };
        let (w, h) = (cx.width(), cx.height());
        p.picture(&format!("{}#view", cx.id), 0, picture_of(b.display_rgba()), (0, 0, w, h));
        if let Some(ed) = editing(cx.id) {
            if let Some(r) = ed.rect {
                paint_edit(p, r, &ed.text, &cx.font, cx.state.caret_on);
            }
        }
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        setup(cx);
        let (x, y) = (m.x.floor() as i64, m.y.floor() as i64);
        let (shift, ctrl) = (m.mods.shift, m.mods.ctrl || m.mods.command);
        let ev = match m.kind {
            MouseKind::Down => {
                if editing(cx.id).is_some() {
                    finish_edit(cx, true);
                }
                // (a second press near the first, soon after: a double click)
                let dbl = double_click(cx.id, ((x / 4) * 100_000 + y / 4) as usize);
                with_listview_mut(cx.id, |lv| lv.mouse_down(x, y, shift, ctrl, dbl)).unwrap_or_default()
            }
            MouseKind::Up => with_listview_mut(cx.id, |lv| lv.mouse_up(x, y)).unwrap_or_default(),
            MouseKind::Move => {
                with_listview_mut(cx.id, |lv| lv.mouse_move(x, y));
                Vec::new()
            }
            MouseKind::Leave => {
                with_listview_mut(cx.id, |lv| lv.mouse_leave());
                Vec::new()
            }
        };
        events(cx, ev);
        MouseOut::default()
    }

    fn key(&self, cx: &mut Cx, k: &KeyIn, _clip: &mut dyn Clipboard) -> bool {
        if let Some(end) = edit_key(cx.id, k) {
            if let Some(keep) = end {
                finish_edit(cx, keep);
            }
            return true;
        }
        if k.mods.alt || k.mods.command {
            return false;
        }
        setup(cx);
        let (ev, _changed) = with_listview_mut(cx.id, |lv| lv.key_down(k.vk, k.mods.shift, k.mods.ctrl)).unwrap_or_default();
        let took = !ev.is_empty() || matches!(k.vk, 32..=40 | 113);
        events(cx, ev);
        took
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        let mut n = AccessNode::new(node_id(cx.id), Role::ListBox);
        n.actions = vec![Action::Focus];
        n.bounds = cx.rect;
        let items = with_listview(cx.id, |lv| lv.items.iter().map(|it| it.caption.clone()).collect::<Vec<_>>()).unwrap_or_default();
        let index = cx.store.get(cx.id, "itemindex").to_i64();
        for (i, caption) in items.into_iter().enumerate().take(1_000) {
            let mut o = AccessNode::new(part_id(cx.id, 1, i), Role::ListBoxOption);
            o.name = caption;
            o.states.selected = Some(index == i as i64);
            n.children.push(o);
        }
        n
    }

    fn access(&self, _cx: &mut Cx, _action: Action, _part: Option<usize>, _value: Option<&AccessValue>) -> bool {
        false
    }

    /// `__edit` (F2), `__enter` ("Renamed" typed, then Enter), `__escape`.
    fn test_action(&self, cx: &mut Cx, action: &str) -> bool {
        let key = |vk: i64| KeyIn { vk, text: "", mods: crate::input::Mods::NONE };
        let mut clip = crate::input::MemClipboard::default();
        match action {
            "__edit" => {
                self.key(cx, &key(113), &mut clip);
            }
            "__enter" | "__escape" => {
                if editing(cx.id).is_some() {
                    if action == "__enter" {
                        set_edit_text(cx.id, "Renamed");
                    }
                    self.key(cx, &key(if action == "__enter" { 13 } else { 27 }), &mut clip);
                }
            }
            _ => return false,
        }
        true
    }
}

//! QLISTVIEW: the shared model (`rapidr_value::objects::listview`) does
//! everything — its four views, the header, check boxes, image lists,
//! selection, the mouse, the keys and the wheel — and paints the control
//! as a bitmap (`listview_paint`), which the kernel shows as a picture (as
//! the web's canvas does). What the user did comes back as
//! the model's events: OnClick, OnDblClick, OnColumnClick (Column),
//! OnChange (Index, Change), and F2 / a click on the selected item edit
//! its caption in place (Enter keeps it: OnChange (Index, ctText)) — the
//! click's edit after Windows' double-click time, unless another press
//! comes first (a double click).

use std::cell::RefCell;
use std::collections::HashMap;

use rapidr_value::objects::a11y::{AccessNode, Action};
use rapidr_value::objects::listview::Event;
use rapidr_value::objects::{listview_paint, listview_setup, with_listview, with_listview_mut};
use rapidr_value::{v_int, Value};

use rapidr_value::objects::ops::Rect;

use super::list::{begin_edit, drop_editor, edit_key, editing, editor_ime, editor_ime_area, editor_menu, editor_mouse, end_edit, paint_editor, picture_of, set_edit_text, InPlace, ListAction};
use super::{ComponentKind, Cx, Ime, KeyIn, MouseIn, MouseKind, MouseOut};
use crate::a11y::AccessValue;
use crate::input::{Clipboard, KernelEvent};
use crate::paint::Painter;
use crate::store::Store;

pub struct ListViewBox;

thread_local! {
    /// (the input lane's) A click on the selected item: the item whose
    /// caption is edited when the double-click time is up (`tick`), and the
    /// model's press count then — a press since (a double click) cancels
    /// it, as the web's timer does.
    static EDIT_SOON: RefCell<HashMap<String, (usize, u64)>> = RefCell::new(HashMap::new());
}

/// A list view's Color as &HBBGGRR (the theme's window, white, unless the
/// program says).
fn background(cx: &Cx) -> u32 {
    let window = rapidr_value::theme::bgr(rapidr_value::theme::current().window);
    match cx.store.get(cx.id, "color") {
        Value::Null => window,
        Value::String(s) if s.is_empty() => window,
        v => rapidr_value::objects::form_color(&v) as u32,
    }
}

/// The model told its control's size, font (its colour the one its text
/// is drawn in: `paint::inked`) and focus (before it paints or takes
/// input).
fn setup(cx: &Cx) {
    let font = crate::paint::inked(cx, rapidr_value::theme::bgr(background(cx)));
    listview_setup(cx.id, cx.width(), cx.height(), &font, cx.state.focused);
}

/// The model's events, for the program (an edit starts here).
fn events(cx: &mut Cx, events: Vec<Event>) {
    for e in events {
        match e {
            // (a click on the selected item: Windows edits it after the
            // double-click time unless a double click comes — `tick`)
            Event::EditSoon(i) => {
                let clicks = with_listview(cx.id, |lv| lv.clicks).unwrap_or(0);
                EDIT_SOON.with(|e| e.borrow_mut().insert(cx.id.to_string(), (i, clicks)));
                cx.ui.wake = Some(crate::tick::now() + crate::tick::DOUBLE_CLICK);
            }
            Event::Edit(i) => start_edit(cx.id, i),
            e => cx.events.extend(kernel_event(cx.id, e)),
        }
    }
}

/// One of the model's events as the program hears it (`None`: the
/// kernel's own, an edit).
fn kernel_event(id: &str, e: Event) -> Option<KernelEvent> {
    let (id, fire) = (id.to_string(), |event: &str, args| Some(ListAction::Fire(event.to_string(), args)));
    let action = match e {
        Event::Click => return Some(KernelEvent::Click(id)),
        Event::DblClick => fire("ondblclick", Vec::new()),
        Event::ColumnClick(i) => fire("oncolumnclick", vec![v_int(i as i64)]),
        Event::Change(i, ct) => fire("onchange", vec![v_int(i as i64), v_int(ct)]),
        Event::Edit(_) | Event::EditSoon(_) => None,
    };
    action.map(|a| KernelEvent::List(id, a))
}

/// Item `i`'s caption edited in place.
fn start_edit(id: &str, i: usize) {
    let rect = with_listview_mut(id, |lv| lv.editor_rect(i)).flatten();
    let text = with_listview(id, |lv| lv.items.get(i).map(|it| it.caption.clone())).flatten();
    if let (Some((l, t, r, b)), Some(text)) = (rect, text) {
        begin_edit(id, InPlace { target: (i, 0), text, rect: Some((l, t, r - l, b - t)) });
    }
}

/// (the input lane's) The edit `ed` of list view `id` kept: the item gets
/// it — the program's events (OnChange (Index, ctText)).
pub(crate) fn edited(id: &str, ed: InPlace) -> Vec<KernelEvent> {
    let ev = with_listview_mut(id, |lv| lv.edited(ed.target.0, ed.text)).unwrap_or_default();
    ev.into_iter().filter_map(|e| kernel_event(id, e)).collect()
}

/// The caption's edit ends: kept, the item gets it (OnChange (Index,
/// ctText)).
fn finish_edit(cx: &mut Cx, keep: bool) {
    let Some(ed) = end_edit(cx.id) else { return };
    cx.ui.edit = None;
    if keep {
        let ev = edited(cx.id, ed);
        cx.events.extend(ev);
    }
}

/// The edit's box (in the component), while an edit goes on.
fn edit_rect(id: &str) -> Option<Rect> {
    editing(id)?.rect
}

impl ComponentKind for ListViewBox {
    fn name(&self) -> &'static str {
        "RLISTVIEW"
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        setup(cx);
        drop_editor(cx);
        let Some(mut b) = listview_paint(cx.id, background(cx)) else { return };
        let (w, h) = (cx.width(), cx.height());
        p.picture(&format!("{}#view", cx.id), 0, picture_of(b.display_rgba()), (0, 0, w, h));
        if let Some(r) = edit_rect(cx.id) {
            paint_editor(cx, p, r);
        }
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        setup(cx);
        // (the edit's box: its editor's)
        if let Some(r) = edit_rect(cx.id) {
            if editor_mouse(cx, m, r) {
                return MouseOut::default();
            }
        }
        let (x, y) = (m.x.floor() as i64, m.y.floor() as i64);
        let (shift, ctrl) = (m.mods.shift, m.mods.ctrl || m.mods.command);
        let ev = match m.kind {
            MouseKind::Down => {
                if editing(cx.id).is_some() {
                    finish_edit(cx, true);
                }
                // (a double click's second press: Windows' click count)
                let dbl = m.double();
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

    fn key(&self, cx: &mut Cx, k: &KeyIn, clip: &mut dyn Clipboard) -> bool {
        // (the input lane's: a key forgets a pending edit)
        if EDIT_SOON.with(|e| e.borrow_mut().remove(cx.id)).is_some() {
            cx.ui.wake = None;
        }
        if let Some(r) = edit_rect(cx.id) {
            if let Some(end) = edit_key(cx, k, clip, r) {
                if let Some(keep) = end {
                    finish_edit(cx, keep);
                }
                return true;
            }
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
        super::shared_describe(cx, "RLISTVIEW")
    }

    fn access(&self, _cx: &mut Cx, _action: Action, _part: Option<usize>, _value: Option<&AccessValue>) -> bool {
        false
    }

    /// (the input lane's) The wheel scrolls the items (the model's: three
    /// rows a notch, a column across in vsList), as the web's.
    fn wheel(&self, cx: &mut Cx, _dx: f64, dy: f64, _mods: crate::input::Mods) -> bool {
        setup(cx);
        let notches = super::list::whole_notches(cx.id, dy);
        if notches != 0 {
            with_listview_mut(cx.id, |lv| lv.wheel(notches));
        }
        true
    }

    /// (the input lane's) The double-click time is up after a click on the
    /// selected item: its caption edited, unless the model was pressed
    /// since.
    fn tick(&self, cx: &mut Cx) {
        let Some((i, clicks)) = EDIT_SOON.with(|e| e.borrow_mut().remove(cx.id)) else { return };
        if with_listview(cx.id, |lv| lv.clicks) == Some(clicks) && editing(cx.id).is_none() {
            start_edit(cx.id, i);
        }
    }

    // (the input lane's: the edit's input methods and context menu)
    fn ime(&self, cx: &mut Cx, ime: &Ime) -> bool {
        edit_rect(cx.id).is_some_and(|r| editor_ime(cx, ime, r))
    }

    fn ime_area(&self, cx: &mut Cx) -> Option<Rect> {
        editor_ime_area(cx, edit_rect(cx.id)?)
    }

    fn wants_ime(&self, _store: &dyn Store, id: &str) -> bool {
        editing(id).is_some()
    }

    fn context_menu(&self, cx: &mut Cx) -> Option<super::edit::MenuState> {
        editor_menu(cx, edit_rect(cx.id)?)
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

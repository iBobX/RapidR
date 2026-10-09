//! A kernel event as the program's event (moved from runtime-core's
//! `ui/kernel.rs`): OnClick, OnKeyDown (KeyPreview first), OnMouseDown …
//! with RapidQ's arguments, the close box's OnClose, a user's resize and
//! move, a screen's scale, a menu pick, a property the user changed (Set),
//! a list's request (List) and a container's action (Container). The host
//! queues the events inside its callbacks; the runtime dispatches them
//! after, one at a time, each to completion (a handler may wait again).

use rapidr_ui_kernel::components::form::Container;
use rapidr_ui_kernel::KernelEvent;
use rapidr_value::input::{Button, Mouse};
use rapidr_value::v_int;

use crate::windows::invalidate;
use crate::{forms, lists, menus, Program, Windows};

/// A kernel event of one of the program's forms, fired as the program's.
pub fn dispatch<R: Program + Windows>(rt: R, ev: KernelEvent) {
    match ev {
        KernelEvent::Click(id) => rt.fire(&id, "onclick"),
        // (the input lane's)
        KernelEvent::DblClick(id) => rt.fire(&id, "ondblclick"),
        KernelEvent::Change(id) => rt.fire(&id, "onchange"),
        KernelEvent::KeyDown { chain, vk, shift, text } => {
            // (a key pressed in the program's windows is INKEY$'s too)
            if let Some(k) = rapidr_value::console::inkey_of(vk, &text) {
                rapidr_value::console::push_key(k);
            }
            for name in rapidr_value::input::key_targets(&chain, |f| rt.get(f, "keypreview").to_bool()) {
                rt.fire_args(name, "onkeydown", &[v_int(vk), v_int(shift)]);
            }
        }
        KernelEvent::KeyPress { chain, key } => {
            for name in rapidr_value::input::key_targets(&chain, |f| rt.get(f, "keypreview").to_bool()) {
                rt.fire_args(name, "onkeypress", &[v_int(key)]);
            }
        }
        KernelEvent::KeyUp { chain, vk, shift } => {
            for name in rapidr_value::input::key_targets(&chain, |f| rt.get(f, "keypreview").to_bool()) {
                rt.fire_args(name, "onkeyup", &[v_int(vk), v_int(shift)]);
            }
        }
        KernelEvent::Mouse { id, kind, button, x, y, shift } => mouse_event(rt, &id, kind, button, x, y, shift),
        KernelEvent::Close(f) => forms::close(rt, &f),
        KernelEvent::Resized(f, w, h) => {
            let menu = i64::from(forms::menu_offset(rt, &f));
            forms::form_resized(rt, &f, w, h + menu);
        }
        KernelEvent::Moved(f, x, y) => forms::form_moved(rt, &f, x, y),
        KernelEvent::ScaleChanged(f, scale) => forms::scale_changed(rt, &f, scale),
        KernelEvent::DropFiles(f, files) => forms::files_dropped(rt, &f, &files),
        KernelEvent::MenuPick(item) => menus::picked(rt, &item),
        KernelEvent::Set { id, prop, value } => {
            rt.set(&id, &prop, v_int(value));
            invalidate();
        }
        KernelEvent::List(id, action) => lists::dispatch(rt, &id, action),
        KernelEvent::Container(c) => container_event(rt, c),
        KernelEvent::Fire { id, event, args } => rt.fire_args(&id, &event, &args),
        KernelEvent::Hint(long) => hint(rt, &long),
    }
}

/// The application's hint changed to `long` (the mouse moved onto another
/// component): OnHint of the form it was bound on last, when it changed
/// (`rapidr_value::hints`: RapidQ's Application.OnHint).
fn hint<P: Program>(p: P, long: &str) {
    if !rapidr_value::hints::change(long) {
        return;
    }
    if let Some(form) = rapidr_value::hints::receiver() {
        p.fire_args(&form, "onhint", &[rapidr_value::v_str(long)]);
    }
}

/// A container's action (containers lane): a scroll bar, a splitter or an
/// MDI frame the user moved, into the program's layout models.
fn container_event<P: Program>(p: P, c: Container) {
    match c {
        // (the input lane's: the host's — `Desktop` makes it a window command)
        Container::Resize { .. } => {}
        c => p.container(c),
    }
}

/// `name`'s mouse event (a QIMAGE's too: the kernel draws it and routes
/// its mouse like any component's).
fn mouse_event<R: Program + Windows>(rt: R, name: &str, kind: Mouse, button: Button, x: i64, y: i64, shift: i64) {
    // (Stage 10: a design surface's mouse is its own events — OnSelect,
    // OnMove …, components/design.rs — it takes the mouse whole)
    if rapidr_value::objects::is_design(name) {
        return;
    }
    if kind == Mouse::Down && button == Button::Right && menus::auto_popup(rt, name, x, y) {
        return;
    }
    rt.fire_args(name, kind.event(), &kind.args(button, x, y, shift));
}

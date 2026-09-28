//! In-page modal dialogs for MESSAGEBOX, MESSAGEDLG, SHOWMESSAGE and INPUT.
//!
//! Browsers' own `alert`/`confirm`/`prompt` block the page but offer at most
//! two buttons and no title, and some embedding pages disable them. When the
//! bytecode VM runs the program, a dialog here instead asks the VM to
//! *suspend* (`rapidr_vm::VmError::Suspended`): the VM keeps its state, the
//! browser keeps running, and choosing a button resumes the program with the
//! answer (the handler installed by `rapidr-vm-host-web`).
//!
//! A dialog can only suspend when the VM is the only thing on the JavaScript
//! call stack — the main program, or an event handler called by the browser.
//! Elsewhere (a handler fired synchronously from inside another VM call, the
//! Rust-compiled web build) `can_wait` is false and callers fall back to the
//! browser's dialogs.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::gui_web::{create_el, document};
use crate::value::Value;

/// Resumes the VM with the dialog's answer; `echo` is the line an INPUT
/// read, to show in the program's output.
pub type ResumeHandler = Rc<dyn Fn(Value, Option<String>)>;

thread_local! {
    static VM_DEPTH: Cell<u32> = const { Cell::new(0) };
    static SUSPEND: Cell<bool> = const { Cell::new(false) };
    static WAITING: Cell<bool> = const { Cell::new(false) };
    static RESUME: RefCell<Option<ResumeHandler>> = const { RefCell::new(None) };
    /// The forms the program waits on (`ShowModal`), outermost first, and
    /// whether each has closed yet.
    static MODALS: RefCell<Vec<(String, bool)>> = const { RefCell::new(Vec::new()) };
    /// The program's main body hasn't finished (it may wait for a dialog).
    static MAIN_RUNNING: Cell<bool> = const { Cell::new(false) };
}

/// The VM host says whether the program's main body is still running. A
/// ShowModal in it doesn't wait: the IDE puts the startup form's ShowModal
/// before the program's own statements, which run at once, as they always
/// have in the browser. One in an event handler (a second form opened from
/// a button) waits, as on the desktop.
pub fn set_main_running(running: bool) {
    MAIN_RUNNING.with(|m| m.set(running));
}

/// Installed by the VM host: how to continue the program after a dialog.
pub fn set_resume_handler(handler: ResumeHandler) {
    RESUME.with(|r| *r.borrow_mut() = Some(handler));
}

/// The VM host brackets every entry into the VM with these.
pub fn enter_vm() {
    VM_DEPTH.with(|d| d.set(d.get() + 1));
}

pub fn leave_vm() {
    VM_DEPTH.with(|d| d.set(d.get().saturating_sub(1)));
}

/// Whether a dialog can suspend the program now (see the module docs).
pub fn can_wait() -> bool {
    VM_DEPTH.with(Cell::get) == 1 && !is_waiting() && RESUME.with(|r| r.borrow().is_some())
}

/// A dialog is open and the program is waiting for it.
pub fn is_waiting() -> bool {
    WAITING.with(Cell::get)
}

/// `Form.ShowModal`: asks the VM to suspend the program until the form
/// closes ([`modal_closed`]), as the desktop's ShowModal blocks. `false`
/// where it can't wait (see the module docs): the caller then shows the
/// form without waiting.
pub fn begin_modal(form_id: &str) -> bool {
    if MAIN_RUNNING.with(Cell::get) || VM_DEPTH.with(Cell::get) != 1 || is_waiting() || RESUME.with(|r| r.borrow().is_none()) {
        return false;
    }
    MODALS.with(|m| m.borrow_mut().push((form_id.to_string(), false)));
    SUSPEND.with(|s| s.set(true));
    true
}

/// The form `form_id` closed (or hid): if the program waits on it, it
/// continues — now, or, inside the VM, once the VM is idle again
/// ([`resume_after_modal`]).
pub fn modal_closed(form_id: &str) {
    let waiting = MODALS.with(|m| {
        m.borrow_mut().iter_mut().rev().find(|(id, closed)| id == form_id && !*closed).map(|e| e.1 = true).is_some()
    });
    if waiting && VM_DEPTH.with(Cell::get) == 0 {
        while resume_after_modal() {}
    }
}

/// Continues the program after the form it waits on closed: the innermost
/// wait resumes when its form has closed and no message dialog is open (the
/// dialog's answer goes to the code above it first). `true` if it did.
pub fn resume_after_modal() -> bool {
    if VM_DEPTH.with(Cell::get) != 0 || is_waiting() {
        return false;
    }
    let due = MODALS.with(|m| {
        let mut m = m.borrow_mut();
        if m.last().is_some_and(|(_, closed)| *closed) {
            m.pop();
            true
        } else {
            false
        }
    });
    if !due {
        return false;
    }
    let handler = RESUME.with(|r| r.borrow().clone());
    match handler {
        Some(h) => {
            h(Value::Null, None);
            true
        }
        None => false,
    }
}

/// The program waits in a ShowModal.
pub fn modal_waiting() -> bool {
    MODALS.with(|m| !m.borrow().is_empty())
}

/// Forgets the waits of a program that was replaced.
pub fn clear_modals() {
    MODALS.with(|m| m.borrow_mut().clear());
}

/// Asked by the VM host after each builtin: did it open a dialog?
pub fn take_suspend() -> bool {
    SUSPEND.with(|s| s.replace(false))
}

/// What a dialog asks for.
pub struct Dialog<'a> {
    pub title: &'a str,
    pub text: &'a str,
    /// Button labels and the value each returns; the first is the default.
    pub buttons: Vec<(String, i64)>,
    /// Returned when the dialog is closed with Escape.
    pub dismissed: i64,
    /// `Some(initial text)`: a text field whose content is the answer
    /// (INPUT); `echo` then shows the line in the program's output.
    pub input: Option<&'a str>,
    pub echo: bool,
}

const BACKDROP: &str = "position:fixed;inset:0;z-index:100000;display:flex;align-items:center;justify-content:center;\
background:rgba(0,0,0,0.25);font:13px system-ui,-apple-system,'Segoe UI',sans-serif;";
const BOX: &str = "min-width:280px;max-width:min(560px,90vw);background:#f0f0f0;color:#000;border:1px solid #888;\
border-radius:6px;box-shadow:0 8px 28px rgba(0,0,0,0.3);overflow:hidden;";
const TITLE: &str = "background:linear-gradient(135deg,#4a90d9,#357abd);color:#fff;padding:7px 12px;font-weight:600;";
const MESSAGE: &str = "padding:16px 16px 8px;white-space:pre-wrap;overflow-wrap:anywhere;max-height:60vh;overflow:auto;";
const FIELD: &str = "display:block;box-sizing:border-box;width:calc(100% - 32px);margin:4px 16px 8px;padding:4px 6px;\
font:inherit;border:1px solid #999;border-radius:3px;background:#fff;color:#000;";
const BUTTONS: &str = "display:flex;justify-content:flex-end;gap:8px;padding:8px 16px 14px;";
const BUTTON: &str = "min-width:78px;padding:4px 12px;font:inherit;border:1px solid #999;border-radius:4px;\
background:#e1e1e1;color:#000;cursor:pointer;";
const DEFAULT_BUTTON: &str = "min-width:78px;padding:4px 12px;font:inherit;border:1px solid #2f6db3;border-radius:4px;\
background:#4a90d9;color:#fff;cursor:pointer;";

/// Opens the dialog and asks the VM to suspend; the answer resumes it.
/// Only call when [`can_wait`] is true.
pub fn open(d: Dialog<'_>) {
    let doc = document();
    let Some(body) = doc.body() else { return };
    let backdrop = create_el("div");
    backdrop.set_class_name("rr-dialog-backdrop");
    let _ = backdrop.set_attribute("style", BACKDROP);
    let panel = create_el("div");
    panel.set_class_name("rr-dialog");
    let _ = panel.set_attribute("style", BOX);
    let _ = panel.set_attribute("role", "dialog");
    let _ = panel.set_attribute("aria-modal", "true");
    if !d.title.is_empty() {
        let title = create_el("div");
        title.set_class_name("rr-dialog-title");
        let _ = title.set_attribute("style", TITLE);
        title.set_text_content(Some(d.title));
        let _ = panel.append_child(&title);
        let _ = panel.set_attribute("aria-label", d.title);
    }
    let message = create_el("div");
    message.set_class_name("rr-dialog-text");
    let _ = message.set_attribute("style", MESSAGE);
    message.set_text_content(Some(d.text));
    let _ = panel.append_child(&message);

    let field = d.input.map(|initial| {
        let input = doc.create_element("input").unwrap().dyn_into::<web_sys::HtmlInputElement>().unwrap();
        input.set_class_name("rr-dialog-input");
        let _ = input.set_attribute("style", FIELD);
        let _ = input.set_attribute("aria-label", d.text.trim());
        input.set_value(initial);
        let _ = panel.append_child(&input);
        input
    });

    // The answer, once: a button, Enter (the first button) or Escape.
    let done = Rc::new(Cell::new(false));
    let finish: Rc<dyn Fn(Option<i64>)> = {
        let (done, backdrop, field) = (done.clone(), backdrop.clone(), field.clone());
        let dismissed = d.dismissed;
        let echo = d.echo;
        Rc::new(move |choice: Option<i64>| {
            if done.replace(true) {
                return;
            }
            backdrop.remove();
            WAITING.with(|w| w.set(false));
            let (value, echoed) = match &field {
                Some(f) => {
                    let text = f.value();
                    (Value::String(text.clone()), echo.then_some(text))
                }
                None => (Value::Integer(choice.unwrap_or(dismissed)), None),
            };
            if let Some(resume) = RESUME.with(|r| r.borrow().clone()) {
                resume(value, echoed);
            }
        })
    };

    let row = create_el("div");
    row.set_class_name("rr-dialog-buttons");
    let _ = row.set_attribute("style", BUTTONS);
    let mut first_button = None;
    for (i, (label, result)) in d.buttons.iter().enumerate() {
        let button = create_el("button");
        button.set_class_name("rr-dialog-button");
        let _ = button.set_attribute("type", "button");
        let _ = button.set_attribute("style", if i == 0 { DEFAULT_BUTTON } else { BUTTON });
        button.set_text_content(Some(label));
        let (finish, result) = (finish.clone(), *result);
        let click = Closure::<dyn FnMut()>::new(move || finish(Some(result)));
        let _ = button.add_event_listener_with_callback("click", click.as_ref().unchecked_ref());
        click.forget();
        let _ = row.append_child(&button);
        first_button.get_or_insert(button);
    }
    let _ = panel.append_child(&row);
    let _ = backdrop.append_child(&panel);

    let default_result = d.buttons.first().map(|b| b.1);
    let keys = {
        let finish = finish.clone();
        Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| match e.key().as_str() {
            "Escape" => {
                e.prevent_default();
                finish(None);
            }
            // Enter on the text field or anywhere but another button.
            "Enter" if !e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()).is_some_and(|t| t.tag_name() == "BUTTON") => {
                e.prevent_default();
                finish(default_result);
            }
            _ => {}
        })
    };
    let _ = backdrop.add_event_listener_with_callback("keydown", keys.as_ref().unchecked_ref());
    keys.forget();

    let _ = body.append_child(&backdrop);
    match (&field, &first_button) {
        (Some(f), _) => {
            let _ = f.focus();
        }
        (None, Some(b)) => {
            let _ = b.focus();
        }
        _ => {}
    }
    WAITING.with(|w| w.set(true));
    SUSPEND.with(|s| s.set(true));
}

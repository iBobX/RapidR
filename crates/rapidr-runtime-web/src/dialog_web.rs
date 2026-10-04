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
//!
//! The page has one thread, so a program that never waits would freeze it:
//! the VM runs in time slices ([`should_yield`]) and, between two, *yields*
//! (`rapidr_vm::VmError::Yielded`) — the browser repaints and takes input,
//! then the host continues the program where it was. Meanwhile nothing else
//! of the program runs: its events wait until it waits, and an answer that
//! comes then (a dialog's, a SLEEP's end) is given once the yield is over
//! ([`resume_pending`]).

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
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
    /// INPUT$ waits for a key: the page's next keydown continues it.
    static KEY_WAIT: Cell<bool> = const { Cell::new(false) };
    /// The VM yielded and goes on in a moment (see the module docs).
    static YIELDED: Cell<bool> = const { Cell::new(false) };
    /// Answers that came during a yield, oldest first.
    static PENDING: RefCell<VecDeque<(Value, Option<String>)>> = const { RefCell::new(VecDeque::new()) };
    /// When the current time slice ends (ms, `performance.now()`).
    static SLICE_END: Cell<f64> = const { Cell::new(0.0) };
    static PERFORMANCE: Option<web_sys::Performance> = web_sys::window().and_then(|w| w.performance());
}

/// How long the program runs before the page gets a turn: short enough to
/// keep it responsive (a frame is ~16 ms), long enough that a busy program
/// loses little time.
const SLICE_MS: f64 = 10.0;

fn now() -> f64 {
    PERFORMANCE.with(|p| p.as_ref().map_or_else(js_sys::Date::now, web_sys::Performance::now))
}

/// Installed by the VM host: how to continue the program after a dialog.
pub fn set_resume_handler(handler: ResumeHandler) {
    RESUME.with(|r| *r.borrow_mut() = Some(handler));
}

/// The VM host brackets every entry into the VM with these. An entry
/// starts a time slice.
pub fn enter_vm() {
    VM_DEPTH.with(|d| {
        if d.get() == 0 {
            SLICE_END.with(|e| e.set(now() + SLICE_MS));
        }
        d.set(d.get() + 1);
    });
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

/// The program has had the page for its time slice.
pub fn slice_over() -> bool {
    now() >= SLICE_END.with(Cell::get)
}

/// Asked by the VM every so often: should it yield now? Only when the
/// slice is over and the VM can be continued later on its own (it's the
/// only thing on the JavaScript call stack, as for [`can_wait`]).
pub fn should_yield() -> bool {
    VM_DEPTH.with(Cell::get) == 1 && RESUME.with(|r| r.borrow().is_some()) && slice_over()
}

/// The VM host marks a yield (`true`) and its end, right before it
/// continues the program.
pub fn set_yielded(yielded: bool) {
    YIELDED.with(|y| y.set(yielded));
}

/// The program yielded and hasn't continued yet.
pub fn is_yielded() -> bool {
    YIELDED.with(Cell::get)
}

/// Continues the waiting program with `value` — or, during a yield, once
/// it's over ([`resume_pending`]; the program keeps waiting till then).
fn resume(value: Value, echo: Option<String>) {
    if is_yielded() {
        PENDING.with(|p| p.borrow_mut().push_back((value, echo)));
        return;
    }
    WAITING.with(|w| w.set(false));
    if let Some(h) = RESUME.with(|r| r.borrow().clone()) {
        h(value, echo);
    }
}

/// Gives the oldest answer that came during a yield, now that the VM is
/// idle. `true` if there was one.
pub fn resume_pending() -> bool {
    if VM_DEPTH.with(Cell::get) != 0 || is_yielded() {
        return false;
    }
    let Some((value, echo)) = PENDING.with(|p| p.borrow_mut().pop_front()) else { return false };
    resume(value, echo);
    true
}

/// `Form.ShowModal`: asks the VM to suspend the program until the form
/// closes ([`modal_closed`]), as the desktop's ShowModal blocks. `false`
/// where it can't wait (see the module docs): the caller then shows the
/// form without waiting.
pub fn begin_modal(form_id: &str) -> bool {
    if VM_DEPTH.with(Cell::get) != 1 || is_waiting() || RESUME.with(|r| r.borrow().is_none()) {
        return false;
    }
    MODALS.with(|m| m.borrow_mut().push((form_id.to_string(), false)));
    SUSPEND.with(|s| s.set(true));
    true
}

/// Whether the program waits on form `form_id` now (ShowModal).
pub fn is_modal(form_id: &str) -> bool {
    MODALS.with(|m| m.borrow().iter().any(|(id, closed)| id == form_id && !*closed))
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
    if VM_DEPTH.with(Cell::get) != 0 || is_waiting() || is_yielded() {
        return false;
    }
    let due = MODALS.with(|m| {
        let mut m = m.borrow_mut();
        if m.last().is_some_and(|(_, closed)| *closed) {
            m.pop().map(|(id, _)| id)
        } else {
            None
        }
    });
    let Some(form_id) = due else { return false };
    // (what ShowModal returns: the form's ModalResult, as the desktop's)
    let form = form_id.strip_prefix("rr-").unwrap_or(&form_id).to_uppercase();
    let result = rapidr_value::events::modal_result(crate::object_web::rp_comp_get_stored(&form, "modalresult").to_i64());
    let handler = RESUME.with(|r| r.borrow().clone());
    match handler {
        Some(h) => {
            h(Value::Integer(result), None);
            true
        }
        None => false,
    }
}

/// `SLEEP` / `DOEVENTS`: the program pauses `ms` milliseconds while the
/// browser goes on (painting, events — a DOEVENTS loop no longer freezes
/// the page), then continues. `false` where it can't wait (see the module
/// docs).
pub fn pause(ms: f64) -> bool {
    if !can_wait() {
        return false;
    }
    WAITING.with(|w| w.set(true));
    SUSPEND.with(|s| s.set(true));
    let wake = Closure::once_into_js(move || resume(Value::Null, None));
    if let Some(window) = web_sys::window() {
        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(wake.unchecked_ref(), ms.clamp(0.0, 86_400_000.0) as i32);
    }
    true
}

/// INPUT$'s wait: the program sleeps until a key is pressed in the page
/// ([`key_pressed`] continues it there and then). `false` where it can't
/// wait.
pub fn wait_key() -> bool {
    if !can_wait() {
        return false;
    }
    WAITING.with(|w| w.set(true));
    SUSPEND.with(|s| s.set(true));
    KEY_WAIT.with(|k| k.set(true));
    true
}

/// A key reached INKEY$'s queue: a program waiting in INPUT$ continues.
pub fn key_pressed() {
    if KEY_WAIT.with(|k| k.replace(false)) {
        // (what RAPIDR__WAITKEY returns: a key came)
        resume(Value::Integer(1), None);
    }
}

/// The program waits in a ShowModal.
pub fn modal_waiting() -> bool {
    MODALS.with(|m| !m.borrow().is_empty())
}

/// Forgets the waits (and a yield) of a program that was replaced.
pub fn clear_modals() {
    MODALS.with(|m| m.borrow_mut().clear());
    YIELDED.with(|y| y.set(false));
    PENDING.with(|p| p.borrow_mut().clear());
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
            let (value, echoed) = match &field {
                Some(f) => {
                    let text = f.value();
                    (Value::String(text.clone()), echo.then_some(text))
                }
                None => (Value::Integer(choice.unwrap_or(dismissed)), None),
            };
            resume(value, echoed);
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

/// What a file dialog asks for (QOPENDIALOG, QSAVEDIALOG, QFILEDIALOG).
pub struct FileRequest {
    pub title: String,
    pub save: bool,
    pub multi: bool,
    /// The program's files it lists (those matching the filter).
    pub files: Vec<String>,
    /// The name it starts with (FileName).
    pub initial: String,
    /// The file input's `accept` for Upload (".txt,.csv"; "" any).
    pub accept: String,
    /// Keeps an uploaded file among the program's files.
    pub store: Rc<dyn Fn(&str, Vec<u8>)>,
    /// Gets the picked names (none: Cancel) before the program goes on.
    pub done: Rc<dyn Fn(Vec<String>)>,
}

/// An Open / Save dialog in the page: the program's files (a click picks
/// one, Ctrl / Cmd-click adds with MultiSelect), a name field (several
/// names split by `;`), Upload… (a file from the computer, kept among the
/// program's files), OK and Cancel. The program waits for it (as for a
/// message dialog) and gets True / False from Execute. Only call when
/// [`can_wait`] is true.
pub fn open_files(req: FileRequest) {
    let doc = document();
    let Some(body) = doc.body() else { return };
    let backdrop = create_el("div");
    backdrop.set_class_name("rr-dialog-backdrop");
    let _ = backdrop.set_attribute("style", BACKDROP);
    let panel = create_el("div");
    panel.set_class_name("rr-dialog rr-file-dialog");
    let _ = panel.set_attribute("style", BOX);
    let _ = panel.set_attribute("role", "dialog");
    let _ = panel.set_attribute("aria-modal", "true");
    let title_text = if req.title.is_empty() { if req.save { "Save As" } else { "Open" } } else { req.title.as_str() };
    let title = create_el("div");
    let _ = title.set_attribute("style", TITLE);
    title.set_text_content(Some(title_text));
    let _ = panel.set_attribute("aria-label", title_text);
    let _ = panel.append_child(&title);

    let list = create_el("div");
    list.set_class_name("rr-file-list");
    let _ = list.set_attribute("role", "listbox");
    let _ = list.set_attribute(
        "style",
        "margin:10px 16px 4px;height:160px;overflow:auto;background:#fff;border:1px solid #999;border-radius:3px;",
    );
    let Ok(field) = doc.create_element("input").map(|e| e.unchecked_into::<web_sys::HtmlInputElement>()) else { return };
    field.set_class_name("rr-file-name");
    let _ = field.set_attribute("style", FIELD);
    let _ = field.set_attribute("aria-label", "File name");
    field.set_value(&req.initial);
    let add_row = {
        let (list, field, multi) = (list.clone(), field.clone(), req.multi);
        Rc::new(move |name: &str| {
            let row = create_el("div");
            row.set_class_name("rr-file-item");
            let _ = row.set_attribute("data-file", name);
            let _ = row.set_attribute("role", "option");
            let _ = row.set_attribute("style", "padding:2px 6px;cursor:default;white-space:nowrap;");
            row.set_text_content(Some(name));
            let (field, name) = (field.clone(), name.to_string());
            let click = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
                let adding = multi && (e.ctrl_key() || e.meta_key()) && !field.value().trim().is_empty();
                field.set_value(&if adding { format!("{};{name}", field.value()) } else { name.clone() });
            });
            let _ = row.add_event_listener_with_callback("click", click.as_ref().unchecked_ref());
            click.forget();
            let _ = list.append_child(&row);
        })
    };
    for f in &req.files {
        add_row(f);
    }
    let _ = panel.append_child(&list);
    let _ = panel.append_child(&field);

    // The answer, once: OK / Enter (the names), Cancel / Escape (none).
    let done = Rc::new(Cell::new(false));
    let finish: Rc<dyn Fn(bool)> = {
        let (done, backdrop, field, answer) = (done.clone(), backdrop.clone(), field.clone(), req.done.clone());
        let multi = req.multi;
        Rc::new(move |ok: bool| {
            if done.replace(true) {
                return;
            }
            let mut names: Vec<String> = if ok { field.value().split(';').map(|n| n.trim().to_string()).filter(|n| !n.is_empty()).collect() } else { Vec::new() };
            if !multi {
                names.truncate(1);
            }
            backdrop.remove();
            let picked = !names.is_empty();
            answer(names);
            resume(Value::Integer(if picked { -1 } else { 0 }), None);
        })
    };

    let row = create_el("div");
    let _ = row.set_attribute("style", BUTTONS);
    // Upload…: a file from the computer joins the program's files.
    if let Ok(upload) = doc.create_element("input").map(|e| e.unchecked_into::<web_sys::HtmlInputElement>()) {
        upload.set_type("file");
        upload.set_multiple(req.multi);
        if !req.accept.is_empty() {
            upload.set_accept(&req.accept);
        }
        let _ = upload.style().set_property("display", "none");
        let (store, add_row, field, source) = (req.store.clone(), add_row.clone(), field.clone(), upload.clone());
        let changed = Closure::<dyn FnMut()>::new(move || {
            let Some(files) = source.files() else { return };
            let mut names = Vec::new();
            for i in 0..files.length() {
                let Some(file) = files.item(i) else { continue };
                let name = file.name();
                names.push(name.clone());
                let (store, add_row) = (store.clone(), add_row.clone());
                let read = Closure::once(move |buffer: JsValue| {
                    store(&name, js_sys::Uint8Array::new(&buffer).to_vec());
                    add_row(&name);
                });
                let _ = file.array_buffer().then(&read);
                read.forget();
            }
            field.set_value(&names.join(";"));
        });
        let _ = upload.add_event_listener_with_callback("change", changed.as_ref().unchecked_ref());
        changed.forget();
        let button = create_el("button");
        button.set_class_name("rr-file-upload");
        let _ = button.set_attribute("type", "button");
        let _ = button.set_attribute("style", &format!("{BUTTON}margin-right:auto;"));
        button.set_text_content(Some("Upload…"));
        let click = Closure::<dyn FnMut()>::new(move || upload.click());
        let _ = button.add_event_listener_with_callback("click", click.as_ref().unchecked_ref());
        click.forget();
        let _ = row.append_child(&button);
    }
    for (label, ok, class) in [(if req.save { "Save" } else { "Open" }, true, "rr-file-ok"), ("Cancel", false, "rr-file-cancel")] {
        let button = create_el("button");
        button.set_class_name(class);
        let _ = button.set_attribute("type", "button");
        let _ = button.set_attribute("style", if ok { DEFAULT_BUTTON } else { BUTTON });
        button.set_text_content(Some(label));
        let finish = finish.clone();
        let click = Closure::<dyn FnMut()>::new(move || finish(ok));
        let _ = button.add_event_listener_with_callback("click", click.as_ref().unchecked_ref());
        click.forget();
        let _ = row.append_child(&button);
    }
    let _ = panel.append_child(&row);
    let _ = backdrop.append_child(&panel);
    let keys = {
        let finish = finish.clone();
        Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| match e.key().as_str() {
            "Escape" => {
                e.prevent_default();
                finish(false);
            }
            "Enter" if !e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()).is_some_and(|t| t.tag_name() == "BUTTON") => {
                e.prevent_default();
                finish(true);
            }
            _ => {}
        })
    };
    let _ = backdrop.add_event_listener_with_callback("keydown", keys.as_ref().unchecked_ref());
    keys.forget();
    let _ = body.append_child(&backdrop);
    let _ = field.focus();
    WAITING.with(|w| w.set(true));
    SUSPEND.with(|s| s.set(true));
}

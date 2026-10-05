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

/// A method that waits for the page's own asynchronous work (the I/O
/// lane's: QDOWNLOAD's fetch, QCOMPORT's Web Serial port opening): the
/// program sleeps — the page paints, its timers tick — until
/// [`task_done`] gives the method's result. `false` where it can't wait
/// (see the module docs).
pub fn wait_task() -> bool {
    if !can_wait() {
        return false;
    }
    WAITING.with(|w| w.set(true));
    SUSPEND.with(|s| s.set(true));
    true
}

/// The work [`wait_task`] waits for is done: the program continues with
/// `value` as the method's result.
pub fn task_done(value: Value) {
    resume(value, None);
}

/// [`pause`] whose end gives `value` as the method's result (QCOMPORT's
/// ReadString with a Wait: the string read).
pub fn pause_with(ms: f64, value: Value) -> bool {
    if !can_wait() {
        return false;
    }
    WAITING.with(|w| w.set(true));
    SUSPEND.with(|s| s.set(true));
    let wake = Closure::once_into_js(move || resume(value, None));
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
    /// The message box's icon, left of the text (MB_ICONxxx, mtWarning …).
    pub icon: Option<rapidr_value::dialogs::MsgIcon>,
}

// (the dialogs lane's: laid out as the desktop's message boxes are —
// `rapidr_value::dialogs::message_layout`'s margins, the icon's 32 + 15,
// Windows' 75 × 23 buttons centred six apart — in the desktop's default
// font, Arial 10 points)
const BACKDROP: &str = "position:fixed;inset:0;z-index:100000;display:flex;align-items:center;justify-content:center;\
background:rgba(0,0,0,0.25);font:13px Arial,'Liberation Sans',Helvetica,sans-serif;";
const BOX: &str = "min-width:120px;max-width:min(560px,90vw);background:#f0f0f0;color:#000;border:1px solid #888;\
border-radius:6px;box-shadow:0 8px 28px rgba(0,0,0,0.3);overflow:hidden;";
const TITLE: &str = "background:linear-gradient(to bottom,#4a90d9,#357abd);color:#fff;padding:0 10px;height:29px;line-height:29px;\
font-weight:bold;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;";
const BODY: &str = "display:flex;align-items:flex-start;gap:15px;padding:13px 12px 0;";
const ICON: &str = "flex:none;width:32px;height:32px;";
const MESSAGE: &str = "white-space:pre-wrap;overflow-wrap:anywhere;max-width:420px;max-height:60vh;overflow:auto;line-height:15px;";
const FIELD: &str = "display:block;box-sizing:border-box;width:calc(100% - 24px);margin:8px 12px 0;padding:2px 4px;\
font:inherit;border:2px inset #f0f0f0;background:#fff;color:#000;";
const BUTTONS: &str = "display:flex;justify-content:center;gap:6px;padding:16px 12px 13px;";
const BUTTON: &str = "box-sizing:border-box;width:75px;height:23px;padding:0 4px;font:inherit;color:#000;cursor:pointer;\
background:#f0f0f0;border:2px outset #f0f0f0;";
const DEFAULT_BUTTON: &str = "box-sizing:border-box;width:75px;height:23px;padding:0 4px;font:inherit;color:#000;cursor:pointer;\
background:#f0f0f0;border:2px outset #f0f0f0;outline:1px solid #000;";

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
    // (the icon at the top left, the text right of it)
    let content = create_el("div");
    content.set_class_name("rr-dialog-body");
    let _ = content.set_attribute("style", BODY);
    if let Some(icon) = d.icon {
        let pic = create_el("div");
        pic.set_class_name("rr-dialog-icon");
        let _ = pic.set_attribute("style", ICON);
        let _ = pic.set_attribute("role", "img");
        let _ = pic.set_attribute("aria-label", icon.name());
        let _ = pic.set_attribute("data-icon", icon.name());
        pic.set_inner_html(&rapidr_value::dialogs::icon_svg(icon, rapidr_value::dialogs::ICON_SIZE));
        let _ = content.append_child(&pic);
    }
    let message = create_el("div");
    message.set_class_name("rr-dialog-text");
    let _ = message.set_attribute("style", MESSAGE);
    message.set_text_content(Some(d.text));
    let _ = content.append_child(&message);
    let _ = panel.append_child(&content);

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
        // (Windows' `&Yes`: the letter underlined, Alt + it a click)
        let caption = rapidr_value::dialogs::button_caption(label);
        let (shown, mark) = rapidr_value::objects::a11y::mnemonic(&caption);
        match mark {
            Some((at, key)) => {
                let before: String = shown.chars().take(at).collect();
                let letter: String = shown.chars().skip(at).take(1).collect();
                let after: String = shown.chars().skip(at + 1).collect();
                let u = create_el("u");
                u.set_text_content(Some(&letter));
                let _ = button.append_with_str_1(&before);
                let _ = button.append_with_node_1(&u);
                let _ = button.append_with_str_1(&after);
                let _ = button.set_attribute("accesskey", &key.to_string());
            }
            None => button.set_text_content(Some(label)),
        }
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

// ------------------------------------------------- colour and font dialogs --
//
// (the dialogs lane's) QCOLORDIALOG and QFONTDIALOG in the page, laid out
// and behaving as the desktop kernel's (the shared models:
// `rapidr_value::color_dialog`, `rapidr_value::font_dialog`): their parts
// placed at the same logical pixels, Windows' classic look.

/// A rectangle (x, y, width, height), logical pixels.
type Rect = (i64, i64, i64, i64);

/// An element of `tag` with class `class`, placed at `rect` in its parent.
fn placed(tag: &str, class: &str, (x, y, w, h): Rect, style: &str) -> web_sys::HtmlElement {
    let el = create_el(tag);
    el.set_class_name(class);
    let _ = el.set_attribute("style", &format!("position:absolute;left:{x}px;top:{y}px;width:{w}px;height:{h}px;box-sizing:border-box;{style}"));
    el
}

/// A caption with its `&` mnemonic as Windows shows it: the letter
/// underlined.
fn set_caption(el: &web_sys::HtmlElement, caption: &str) {
    let (shown, mark) = rapidr_value::objects::a11y::mnemonic(caption);
    el.set_text_content(None);
    match mark {
        Some((at, key)) => {
            let before: String = shown.chars().take(at).collect();
            let letter: String = shown.chars().skip(at).take(1).collect();
            let after: String = shown.chars().skip(at + 1).collect();
            let u = create_el("u");
            u.set_text_content(Some(&letter));
            let _ = el.append_with_str_1(&before);
            let _ = el.append_with_node_1(&u);
            let _ = el.append_with_str_1(&after);
            let _ = el.set_attribute("accesskey", &key.to_string());
        }
        None => el.set_text_content(Some(&shown)),
    }
}

/// A classic push button at `rect` (the default one with its dark frame).
fn push_button(class: &str, caption: &str, rect: Rect, default: bool) -> web_sys::HtmlElement {
    let frame = if default { "outline:1px solid #000;" } else { "" };
    let b = placed("button", class, rect, &format!("padding:0 4px;font:inherit;color:#000;cursor:pointer;background:#f0f0f0;border:2px outset #f0f0f0;{frame}"));
    let _ = b.set_attribute("type", "button");
    set_caption(&b, caption);
    b
}

/// Draws RGBA pixels (`w` × `h`) into `canvas`.
fn paint_rgba(canvas: &web_sys::HtmlCanvasElement, w: usize, h: usize, rgba: &[u8]) {
    canvas.set_width(w as u32);
    canvas.set_height(h as u32);
    let Some(ctx) = canvas.get_context("2d").ok().flatten().and_then(|c| c.dyn_into::<web_sys::CanvasRenderingContext2d>().ok()) else { return };
    if let Ok(data) = web_sys::ImageData::new_with_u8_clamped_array_and_sh(wasm_bindgen::Clamped(rgba), w as u32, h as u32) {
        let _ = ctx.put_image_data(&data, 0.0, 0.0);
    }
}

/// A sunken frame (Windows' 3D edge around a picture or a swatch).
const SUNKEN: &str = "border:1px solid;border-color:#808080 #fff #fff #808080;";

/// An &HBBGGRR colour as CSS.
fn css_color(bgr: i64) -> String {
    format!("#{:06x}", rapidr_value::color_dialog::swap_rb(bgr))
}

/// Adds a mouse listener that lives as long as the page.
fn on_mouse(el: &web_sys::HtmlElement, event: &str, f: Box<dyn FnMut(web_sys::MouseEvent)>) {
    let c = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(f);
    let _ = el.add_event_listener_with_callback(event, c.as_ref().unchecked_ref());
    c.forget();
}

/// The panel, its title and its inside of a colour or font dialog.
fn dialog_panel(class: &str, title: &str) -> (web_sys::HtmlElement, web_sys::HtmlElement, web_sys::HtmlElement) {
    let backdrop = create_el("div");
    backdrop.set_class_name("rr-dialog-backdrop");
    let _ = backdrop.set_attribute("style", BACKDROP);
    let panel = create_el("div");
    panel.set_class_name(&format!("rr-dialog {class}"));
    let _ = panel.set_attribute("style", "background:#f0f0f0;color:#000;border:1px solid #888;border-radius:6px;box-shadow:0 8px 28px rgba(0,0,0,0.3);overflow:hidden;");
    let _ = panel.set_attribute("role", "dialog");
    let _ = panel.set_attribute("aria-modal", "true");
    let _ = panel.set_attribute("aria-label", title);
    let head = create_el("div");
    head.set_class_name("rr-dialog-title");
    let _ = head.set_attribute("style", TITLE);
    head.set_text_content(Some(title));
    let _ = panel.append_child(&head);
    let area = create_el("div");
    let _ = area.set_attribute("style", "position:relative;");
    let _ = panel.append_child(&area);
    let _ = backdrop.append_child(&panel);
    (backdrop, panel, area)
}

/// Escape cancels, Enter (but on a button) is OK: `finish(ok)`.
fn ok_cancel_keys(backdrop: &web_sys::HtmlElement, finish: Rc<dyn Fn(bool)>) {
    let keys = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| match e.key().as_str() {
        "Escape" => {
            e.prevent_default();
            finish(false);
        }
        "Enter" if !e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()).is_some_and(|t| t.tag_name() == "BUTTON") => {
            e.prevent_default();
            finish(true);
        }
        _ => {}
    });
    let _ = backdrop.add_event_listener_with_callback("keydown", keys.as_ref().unchecked_ref());
    keys.forget();
}

/// What a colour dialog asks for (QCOLORDIALOG).
pub struct ColorRequest {
    pub title: String,
    pub state: rapidr_value::color_dialog::State,
    /// Gets the colour chosen (`None`: Cancel) and the custom colours
    /// (kept either way) before the program goes on.
    pub done: Rc<dyn Fn(Option<i64>, [i64; 16])>,
}

/// The colour dialog in the page, as the desktop's: the basic and custom
/// swatches (the chosen one framed), "Define Custom Colors >>", and with
/// the editor the hue / saturation field, the luminance bar, the colour,
/// the Hue … Blue boxes and "Add to Custom Colors"; OK and Cancel. The
/// program waits for it and gets 1 / 0 from Execute. Only call when
/// [`can_wait`] is true.
pub fn open_color(req: ColorRequest) {
    use rapidr_value::color_dialog::{self as cd, layout as cl};
    let Some(body) = document().body() else { return };
    let state = Rc::new(RefCell::new(req.state));
    let (backdrop, _panel, area) = dialog_panel("rr-color-dialog", &req.title);
    let add = |el: &web_sys::HtmlElement| {
        let _ = area.append_child(el);
    };
    let label = |caption: &str, rect: Rect| {
        let l = placed("div", "rr-color-label", rect, "white-space:nowrap;");
        set_caption(&l, caption);
        l
    };
    add(&label("&Basic colors:", cl::BASIC_LABEL));
    // (the chosen swatch's frame, under the swatches)
    let sel = placed("div", "rr-color-sel", (0, 0, 0, 0), "background:#000;");
    add(&sel);
    let swatch = |class: &str, rect: Rect, attr: &str, i: usize| {
        let s = placed("div", class, rect, &format!("{SUNKEN}cursor:default;"));
        let _ = s.set_attribute(attr, &i.to_string());
        s
    };
    let mut basics = Vec::new();
    for (i, rgb) in cd::BASIC_COLORS.iter().enumerate() {
        let s = swatch("rr-color-swatch", cl::basic(i), "data-basic", i);
        let bgr = cd::swap_rb(i64::from(*rgb));
        let _ = s.style().set_property("background", &css_color(bgr));
        let _ = s.set_attribute("data-color", &bgr.to_string());
        add(&s);
        basics.push(s);
    }
    add(&label("&Custom colors:", cl::CUSTOM_LABEL));
    let customs: Vec<_> = (0..16).map(|i| swatch("rr-color-swatch rr-color-custom", cl::custom(i), "data-custom", i)).collect();
    for c in &customs {
        add(c);
    }
    let define = push_button("rr-color-define", "&Define Custom Colors >>", cl::DEFINE, false);
    add(&define);
    let ok = push_button("rr-color-ok", "OK", cl::OK, true);
    add(&ok);
    let cancel = push_button("rr-color-cancel", "Cancel", cl::CANCEL, false);
    add(&cancel);

    // (the editor: shown once the dialog is full open; its pictures lie
    // inside a sunken edge, as the kernel paints them)
    let editor = placed("div", "rr-color-editor", (0, 0, cl::FULL.0, cl::FULL.1), "pointer-events:none;");
    let inner = |r: Rect| (r.0 + 1, r.1 + 1, r.2 - 2, r.3 - 2);
    let field_box = inner(cl::SPECTRUM);
    let spectrum_wrap = placed("div", "rr-color-spectrum-box", cl::SPECTRUM, &format!("{SUNKEN}overflow:hidden;pointer-events:auto;"));
    let spectrum = create_el("canvas").unchecked_into::<web_sys::HtmlCanvasElement>();
    spectrum.set_class_name("rr-color-spectrum");
    let _ = spectrum.set_attribute("style", &format!("display:block;width:{}px;height:{}px;cursor:crosshair;", field_box.2, field_box.3));
    let _ = spectrum_wrap.append_child(&spectrum);
    let cross = placed("div", "rr-color-cross", (0, 0, 17, 17), "pointer-events:none;");
    for r in [(0, 7, 5, 3), (12, 7, 5, 3), (7, 0, 3, 5), (7, 12, 3, 5)] {
        let _ = cross.append_child(&placed("div", "", r, "background:#000;"));
    }
    let _ = spectrum_wrap.append_child(&cross);
    let _ = editor.append_child(&spectrum_wrap);
    let bar = (1, 4, cl::LUM_BAR_W - 2, cl::SPECTRUM.3 - 2);
    let lum_box = placed("div", "rr-color-lum-box", cl::LUM, "pointer-events:auto;cursor:default;");
    let lum_edge = placed("div", "", (bar.0 - 1, bar.1 - 1, bar.2 + 2, bar.3 + 2), SUNKEN);
    let lum = create_el("canvas").unchecked_into::<web_sys::HtmlCanvasElement>();
    lum.set_class_name("rr-color-lum");
    let _ = lum.set_attribute("style", &format!("display:block;width:{}px;height:{}px;", bar.2, bar.3));
    let _ = lum_edge.append_child(&lum);
    let _ = lum_box.append_child(&lum_edge);
    let arrow = placed("div", "rr-color-arrow", (cl::LUM_BAR_W + 2, 0, 0, 0), "border-top:6px solid transparent;border-bottom:6px solid transparent;border-right:7px solid #000;");
    let _ = lum_box.append_child(&arrow);
    let _ = editor.append_child(&lum_box);
    let preview = placed("div", "rr-color-preview", cl::PREVIEW, SUNKEN);
    let _ = editor.append_child(&preview);
    let lpreview = placed("div", "rr-color-label", cl::PREVIEW_LABEL, "text-align:center;white-space:nowrap;");
    set_caption(&lpreview, "Color|S&olid");
    let _ = editor.append_child(&lpreview);
    let mut fields = Vec::new();
    for (i, caption) in cd::FIELDS.iter().enumerate() {
        let (lrect, frect) = cl::field(i);
        let l = placed("div", "rr-color-label", lrect, "white-space:nowrap;");
        set_caption(&l, caption);
        let _ = editor.append_child(&l);
        let f = placed("input", "rr-color-field", frect, "padding:0 2px;font:inherit;border:2px inset #f0f0f0;background:#fff;color:#000;pointer-events:auto;").unchecked_into::<web_sys::HtmlInputElement>();
        let _ = f.set_attribute("data-field", &i.to_string());
        let _ = f.set_attribute("maxlength", "3");
        let _ = f.set_attribute("aria-label", &rapidr_value::objects::a11y::mnemonic(caption).0);
        let _ = editor.append_child(&f);
        fields.push(f);
    }
    let addb = push_button("rr-color-add", "&Add to Custom Colors", cl::ADD, false);
    let _ = addb.style().set_property("pointer-events", "auto");
    let _ = editor.append_child(&addb);
    add(&editor);

    // What it shows, from the state (box `typing` left as typed); the
    // pictures at the screen's resolution.
    let dpr = web_sys::window().map_or(1.0, |w| w.device_pixel_ratio()).max(1.0);
    let dev = move |v: i64| ((v as f64 * dpr).round() as usize).max(1);
    let refresh: Rc<dyn Fn(Option<usize>)> = {
        let (state, sel, define, editor, area, fields, preview, cross, arrow, lum, spectrum) =
            (state.clone(), sel.clone(), define.clone(), editor.clone(), area.clone(), fields.clone(), preview.clone(), cross.clone(), arrow.clone(), lum.clone(), spectrum.clone());
        let (spectrum_drawn, lum_key) = (Cell::new(false), Cell::new(None::<(i64, i64)>));
        Rc::new(move |typing: Option<usize>| {
            let s = state.borrow();
            for (i, el) in customs.iter().enumerate() {
                let _ = el.style().set_property("background", &css_color(s.custom[i]));
                let _ = el.set_attribute("data-color", &s.custom[i].to_string());
            }
            let frame = s.marked().map(|(custom, i)| cl::frame(if custom { cl::custom(i) } else { cl::basic(i) }));
            let (x, y, w, h) = frame.unwrap_or((0, 0, 0, 0));
            for (p, v) in [("left", x), ("top", y), ("width", w), ("height", h)] {
                let _ = sel.style().set_property(p, &format!("{v}px"));
            }
            let disabled = s.mode != cd::Mode::Compact;
            define.unchecked_ref::<web_sys::HtmlButtonElement>().set_disabled(disabled);
            let _ = define.style().set_property("color", if disabled { "#808080" } else { "#000" });
            let (w, h) = if s.full() { cl::FULL } else { cl::COMPACT };
            let _ = area.style().set_property("width", &format!("{w}px"));
            let _ = area.style().set_property("height", &format!("{h}px"));
            let _ = editor.style().set_property("display", if s.full() { "block" } else { "none" });
            if !s.full() {
                return;
            }
            if !spectrum_drawn.replace(true) {
                let (pw, ph) = (dev(field_box.2), dev(field_box.3));
                paint_rgba(&spectrum, pw, ph, &cd::spectrum_rgba(pw, ph));
            }
            let (hue, l, sat) = s.hls;
            if lum_key.get() != Some((hue, sat)) {
                lum_key.set(Some((hue, sat)));
                let (pw, ph) = (dev(bar.2), dev(bar.3));
                paint_rgba(&lum, pw, ph, &cd::lum_rgba(pw, ph, hue, sat));
            }
            let _ = cross.style().set_property("left", &format!("{}px", hue * (field_box.2 - 1) / 239 - 8));
            let _ = cross.style().set_property("top", &format!("{}px", (240 - sat) * (field_box.3 - 1) / 240 - 8));
            let _ = arrow.style().set_property("top", &format!("{}px", bar.1 + (240 - l) * (bar.3 - 1) / 240 - 6));
            let _ = preview.style().set_property("background", &css_color(s.color));
            for (i, v) in s.fields().iter().enumerate() {
                if typing != Some(i) {
                    fields[i].set_value(&v.to_string());
                }
            }
        })
    };

    // The answer, once: OK / Enter (the colour), Cancel / Escape (none).
    let done = Rc::new(Cell::new(false));
    let finish: Rc<dyn Fn(bool)> = {
        let (done, backdrop, state, answer) = (done.clone(), backdrop.clone(), state.clone(), req.done.clone());
        Rc::new(move |ok: bool| {
            if done.replace(true) {
                return;
            }
            backdrop.remove();
            let s = state.borrow().clone();
            answer(ok.then_some(s.color), s.custom);
            resume(Value::Integer(i64::from(ok)), None);
        })
    };
    // (a swatch picks its colour as the mouse goes down, as Windows')
    for (i, s) in basics.iter().enumerate() {
        let (state, refresh) = (state.clone(), refresh.clone());
        on_mouse(s, "mousedown", Box::new(move |_| {
            state.borrow_mut().pick_basic(i);
            refresh(None);
        }));
    }
    for i in 0..16 {
        let Some(el) = area.query_selector(&format!("[data-custom=\"{i}\"]")).ok().flatten() else { continue };
        let (state, refresh) = (state.clone(), refresh.clone());
        on_mouse(el.unchecked_ref(), "mousedown", Box::new(move |_| {
            state.borrow_mut().pick_custom(i);
            refresh(None);
        }));
    }
    // (the field and the bar follow the mouse while it's held)
    let drag = Rc::new(Cell::new(None::<&'static str>));
    type Pick = Rc<dyn Fn(&str, &web_sys::MouseEvent)>;
    let pick: Pick = {
        let (state, refresh, spectrum, lum) = (state.clone(), refresh.clone(), spectrum.clone(), lum.clone());
        Rc::new(move |which: &str, e: &web_sys::MouseEvent| {
            if which == "spectrum" {
                let r = spectrum.get_bounding_client_rect();
                let (w, h) = (field_box.2 as f64, field_box.3 as f64);
                let (x, y) = ((f64::from(e.client_x()) - r.left()).clamp(0.0, w - 1.0), (f64::from(e.client_y()) - r.top()).clamp(0.0, h - 1.0));
                state.borrow_mut().pick_spectrum(x, y, w, h);
            } else {
                let r = lum.get_bounding_client_rect();
                let h = bar.3 as f64;
                state.borrow_mut().pick_lum((f64::from(e.client_y()) - r.top()).clamp(0.0, h - 1.0), h);
            }
            refresh(None);
        })
    };
    for (el, which) in [(spectrum.clone().unchecked_into::<web_sys::HtmlElement>(), "spectrum"), (lum_box.clone(), "lum")] {
        let (drag, pick) = (drag.clone(), pick.clone());
        on_mouse(&el, "mousedown", Box::new(move |e: web_sys::MouseEvent| {
            e.prevent_default();
            drag.set(Some(which));
            pick(which, &e);
        }));
    }
    {
        let (held, pick) = (drag.clone(), pick.clone());
        on_mouse(&backdrop, "mousemove", Box::new(move |e: web_sys::MouseEvent| {
            if let Some(which) = held.get() {
                pick(which, &e);
            }
        }));
        on_mouse(&backdrop, "mouseup", Box::new(move |_| drag.set(None)));
    }
    for (i, f) in fields.iter().enumerate() {
        let (state, refresh, field) = (state.clone(), refresh.clone(), f.clone());
        let typed = Closure::<dyn FnMut()>::new(move || {
            if let Ok(v) = field.value().trim().parse::<i64>() {
                state.borrow_mut().set_field(i, v);
                refresh(Some(i));
            }
        });
        let _ = f.add_event_listener_with_callback("input", typed.as_ref().unchecked_ref());
        typed.forget();
    }
    for (el, action) in [(define.clone(), "define"), (addb.clone(), "add"), (ok.clone(), "ok"), (cancel.clone(), "cancel")] {
        let (state, refresh, finish) = (state.clone(), refresh.clone(), finish.clone());
        on_mouse(&el, "click", Box::new(move |_| match action {
            "define" => {
                if state.borrow_mut().define() {
                    refresh(None);
                }
            }
            "add" => {
                state.borrow_mut().add_custom();
                refresh(None);
            }
            a => finish(a == "ok"),
        }));
    }
    ok_cancel_keys(&backdrop, finish);
    refresh(None);
    let _ = body.append_child(&backdrop);
    let _ = ok.focus();
    WAITING.with(|w| w.set(true));
    SUSPEND.with(|s| s.set(true));
}

/// What a font dialog asks for (QFONTDIALOG).
pub struct FontRequest {
    pub title: String,
    pub req: rapidr_value::font_dialog::Request,
    /// The faces it may list (`Request::names` picks).
    pub names: Vec<String>,
    /// Gets the font chosen (`None`: Cancel) before the program goes on.
    pub done: Rc<dyn Fn(Option<rapidr_value::objects::font::Font>)>,
    /// Apply (fdApplyButton): the font so far, for the program's OnApply.
    pub apply: Rc<dyn Fn(&rapidr_value::objects::font::Font)>,
}

/// A font as CSS for the sample: its face (the generic family RapidR draws
/// it with after it), size, weight, slant, lines and colour.
fn font_css(f: &rapidr_value::objects::font::Font) -> String {
    let generic = match rapidr_value::objects::text::family_name(&f.name) {
        "Liberation Mono" => "monospace",
        "Liberation Serif" => "serif",
        _ => "sans-serif",
    };
    let lines: Vec<&str> = [(4, "underline"), (8, "line-through")].iter().filter(|(b, _)| f.styles & b != 0).map(|(_, l)| *l).collect();
    format!(
        "font-family:'{}',{generic};font-size:{}px;font-weight:{};font-style:{};text-decoration:{};color:{};",
        f.name.replace('\'', ""),
        f.pixel_size(),
        if f.styles & 1 != 0 { "bold" } else { "normal" },
        if f.styles & 2 != 0 { "italic" } else { "normal" },
        if lines.is_empty() { "none".to_string() } else { lines.join(" ") },
        css_color(f.color)
    )
}

/// The font dialog in the page, as the desktop's: the face, style and size
/// lists, with fdEffects Strikeout, Underline and the colour, the sample;
/// OK, Cancel, Apply (fdApplyButton), a disabled Help (fdShowHelp). The
/// program waits for it and gets 1 / 0 from Execute. Only call when
/// [`can_wait`] is true.
pub fn open_font(r: FontRequest) {
    use rapidr_value::font_dialog::{self as fd, layout as fl};
    let Some(body) = document().body() else { return };
    let req = r.req;
    let (names, sizes, colors, effects) = (req.names(&r.names), req.sizes(), req.colors(), req.has(fd::FD_EFFECTS));
    let font = Rc::new(RefCell::new(req.font.clone()));
    let (backdrop, _panel, area) = dialog_panel("rr-font-dialog", &r.title);
    let (w, h) = fl::size(effects);
    let _ = area.style().set_property("width", &format!("{w}px"));
    let _ = area.style().set_property("height", &format!("{h}px"));
    let add = |el: &web_sys::HtmlElement| {
        let _ = area.append_child(el);
    };
    let label = |caption: &str, rect: Rect| {
        let l = placed("div", "rr-font-label", rect, "white-space:nowrap;");
        set_caption(&l, caption);
        add(&l);
    };
    let group = |caption: &str, (x, y, w, h): Rect| {
        let g = placed("div", "rr-font-group", (x, y + 6, w, h - 6), "border:2px groove #f0f0f0;");
        let t = placed("div", "", (6, -9, w - 12, 14), "");
        let span = create_el("span");
        let _ = span.set_attribute("style", "background:#f0f0f0;padding:0 2px;");
        span.set_text_content(Some(caption));
        let _ = t.append_child(&span);
        let _ = g.append_child(&t);
        add(&g);
    };
    let list = |class: &str, rect: Rect, items: &[(String, String)], size: u32, selected: Option<usize>| {
        let s = placed("select", class, rect, "font:inherit;background:#fff;color:#000;border:2px inset #f0f0f0;").unchecked_into::<web_sys::HtmlSelectElement>();
        if size > 0 {
            s.set_size(size);
        }
        for (text, value) in items {
            if let Ok(o) = web_sys::HtmlOptionElement::new_with_text_and_value(text, value) {
                let _ = s.add_with_html_option_element(&o);
            }
        }
        s.set_selected_index(selected.map_or(-1, |i| i as i32));
        add(s.unchecked_ref());
        s
    };
    let f = req.font.clone();
    label("&Font:", fl::FONT_LABEL);
    label("Font st&yle:", fl::STYLE_LABEL);
    label("&Size:", fl::SIZE_LABEL);
    let pairs = |v: &[String]| v.iter().map(|n| (n.clone(), n.clone())).collect::<Vec<_>>();
    let face = list("rr-font-name", fl::FONT_LIST, &pairs(&names), 8, names.iter().position(|n| n.eq_ignore_ascii_case(&f.name)).filter(|_| !req.has(fd::FD_NO_FACE_SEL)));
    let styles: Vec<String> = fd::STYLES.iter().map(|s| s.to_string()).collect();
    let style_index = usize::from(f.styles & 2 != 0) + 2 * usize::from(f.styles & 1 != 0);
    let style = list("rr-font-style", fl::STYLE_LIST, &pairs(&styles), 8, (!req.has(fd::FD_NO_STYLE_SEL)).then_some(style_index));
    let size_items: Vec<String> = sizes.iter().map(i64::to_string).collect();
    let size = list("rr-font-size", fl::SIZE_LIST, &pairs(&size_items), 8, sizes.iter().position(|s| *s == f.size).filter(|_| !req.has(fd::FD_NO_SIZE_SEL)));
    let check = |class: &str, caption: &str, rect: Rect, on: bool| {
        let row = placed("label", "rr-font-check", rect, "display:flex;align-items:center;gap:4px;white-space:nowrap;");
        let c = create_el("input").unchecked_into::<web_sys::HtmlInputElement>();
        c.set_type("checkbox");
        c.set_class_name(class);
        c.set_checked(on);
        let _ = c.style().set_property("margin", "0");
        let text = create_el("span");
        set_caption(&text, caption);
        let _ = row.append_child(&c);
        let _ = row.append_child(&text);
        add(&row);
        c
    };
    let mut effect_inputs = None;
    if effects {
        group("Effects", fl::EFFECTS);
        let strike = check("rr-font-strike", "Stri&keout", fl::STRIKEOUT, f.styles & 8 != 0);
        let under = check("rr-font-under", "&Underline", fl::UNDERLINE, f.styles & 4 != 0);
        label("&Color:", fl::COLOR_LABEL);
        let items: Vec<(String, String)> = colors.iter().map(|(n, c)| (n.clone(), c.to_string())).collect();
        let color = list("rr-font-color", fl::COLOR_LIST, &items, 0, colors.iter().position(|(_, c)| *c == f.color));
        effect_inputs = Some((strike, under, color));
    }
    let (sample_group, sample_rect) = fl::sample(effects);
    group("Sample", sample_group);
    let sample = placed("div", "rr-font-sample", sample_rect, "display:flex;align-items:center;justify-content:center;overflow:hidden;white-space:nowrap;");
    sample.set_text_content(Some(fd::SAMPLE));
    add(&sample);
    let ok = push_button("rr-font-ok", "OK", fl::OK, true);
    add(&ok);
    let cancel = push_button("rr-font-cancel", "Cancel", fl::CANCEL, false);
    add(&cancel);
    let apply_shown = req.has(fd::FD_APPLY_BUTTON);
    let apply = apply_shown.then(|| push_button("rr-font-apply", "&Apply", fl::APPLY, false));
    if let Some(a) = &apply {
        add(a);
    }
    if req.has(fd::FD_SHOW_HELP) {
        let help = push_button("rr-font-help", "&Help", if apply_shown { fl::HELP } else { fl::APPLY }, false);
        help.unchecked_ref::<web_sys::HtmlButtonElement>().set_disabled(true);
        let _ = help.style().set_property("color", "#808080");
        add(&help);
    }

    // (any change: the choice read again and the sample shown in it)
    let update: Rc<dyn Fn()> = {
        let (font, face, style, size, sample, effect_inputs) = (font.clone(), face.clone(), style.clone(), size.clone(), sample.clone(), effect_inputs.clone());
        Rc::new(move || {
            let mut f = font.borrow_mut();
            if let Some(n) = usize::try_from(face.selected_index()).ok().and_then(|i| names.get(i)) {
                f.name = n.clone();
            }
            if let Ok(st) = usize::try_from(style.selected_index()) {
                f.styles = (f.styles & !3) | u8::from(st & 2 != 0) | u8::from(st & 1 != 0) << 1;
            }
            if let Some(s) = usize::try_from(size.selected_index()).ok().and_then(|i| sizes.get(i)) {
                f.size = *s;
            }
            if let Some((strike, under, color)) = &effect_inputs {
                f.styles = (f.styles & 3) | u8::from(under.checked()) << 2 | u8::from(strike.checked()) << 3;
                if let Some((_, c)) = usize::try_from(color.selected_index()).ok().and_then(|i| colors.get(i)) {
                    f.color = *c;
                }
            }
            sample.style().set_css_text(&format!("position:absolute;left:{}px;top:{}px;width:{}px;height:{}px;box-sizing:border-box;display:flex;align-items:center;justify-content:center;overflow:hidden;white-space:nowrap;{}", sample_rect.0, sample_rect.1, sample_rect.2, sample_rect.3, font_css(&f)));
        })
    };
    let changed = |el: &web_sys::HtmlElement| {
        let update = update.clone();
        let c = Closure::<dyn FnMut()>::new(move || update());
        let _ = el.add_event_listener_with_callback("change", c.as_ref().unchecked_ref());
        c.forget();
    };
    changed(face.unchecked_ref());
    changed(style.unchecked_ref());
    changed(size.unchecked_ref());
    if let Some((strike, under, color)) = &effect_inputs {
        changed(strike.unchecked_ref());
        changed(under.unchecked_ref());
        changed(color.unchecked_ref());
    }

    // The answer, once: OK / Enter (the font), Cancel / Escape (none).
    let done = Rc::new(Cell::new(false));
    let finish: Rc<dyn Fn(bool)> = {
        let (done, backdrop, font, answer, update) = (done.clone(), backdrop.clone(), font.clone(), r.done.clone(), update.clone());
        Rc::new(move |ok: bool| {
            if done.replace(true) {
                return;
            }
            update();
            backdrop.remove();
            answer(ok.then(|| font.borrow().clone()));
            resume(Value::Integer(i64::from(ok)), None);
        })
    };
    for (el, ok_button) in [(ok.clone(), true), (cancel.clone(), false)] {
        let finish = finish.clone();
        on_mouse(&el, "click", Box::new(move |_| finish(ok_button)));
    }
    if let Some(a) = &apply {
        let (font, applied, update) = (font.clone(), r.apply.clone(), update.clone());
        on_mouse(a, "click", Box::new(move |_| {
            update();
            let f = font.borrow().clone();
            applied(&f);
        }));
    }
    ok_cancel_keys(&backdrop, finish);
    update();
    let _ = body.append_child(&backdrop);
    let _ = ok.focus();
    WAITING.with(|w| w.set(true));
    SUSPEND.with(|s| s.set(true));
}

//! The bytecode VM's waits in the page, and the page's Open / Save picker.
//!
//! The page has one thread, and the VM can't block it. A wait for the user —
//! a modal form, a message box, INPUT, INPUT$, DOEVENTS (the UI kernel's
//! waits, `kernel_web`), a SLEEP, a download — *suspends* the VM
//! (`rapidr_vm::VmError::Suspended`): it keeps its state, the browser keeps
//! running, and the answer resumes the program (the handler installed by
//! `rapidr-vm-host-web`).
//!
//! A wait can only suspend when the VM is the only thing on the JavaScript
//! call stack — the main program, or an event handler called by the
//! browser. Elsewhere (a handler fired synchronously from inside another VM
//! call, the Rust-compiled web build) `can_wait` is false.
//!
//! A program that never waits would freeze the page: the VM runs in time
//! slices ([`should_yield`]) and, between two, *yields*
//! (`rapidr_vm::VmError::Yielded`) — the browser repaints and takes input,
//! then the host continues the program where it was. Meanwhile nothing else
//! of the program runs: its events wait until it waits, and an answer that
//! comes then (a SLEEP's end) is given once the yield is over
//! ([`resume_pending`]).

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::page_web::{create_el, document};
use crate::value::Value;

/// Resumes the VM with the dialog's answer; `echo` is the line an INPUT
/// read, to show in the program's output.
pub type ResumeHandler = Rc<dyn Fn(Value, Option<String>)>;

thread_local! {
    static VM_DEPTH: Cell<u32> = const { Cell::new(0) };
    static SUSPEND: Cell<bool> = const { Cell::new(false) };
    static WAITING: Cell<bool> = const { Cell::new(false) };
    static RESUME: RefCell<Option<ResumeHandler>> = const { RefCell::new(None) };
    /// INPUT$ waits for a key: the page's next keydown continues it.
    static KEY_WAIT: Cell<bool> = const { Cell::new(false) };
    /// The VM yielded and goes on in a moment (see the module docs).
    static YIELDED: Cell<bool> = const { Cell::new(false) };
    /// Answers that came during a yield, oldest first.
    static PENDING: RefCell<VecDeque<(Value, Option<String>)>> = const { RefCell::new(VecDeque::new()) };
    /// When the current time slice ends (ms, `performance.now()`).
    static SLICE_END: Cell<f64> = const { Cell::new(0.0) };
    /// (Stage W4) The kernel host's waits the VM is suspended in.
    static KERNEL_WAITS: Cell<u32> = const { Cell::new(0) };
    /// The program is in a SLEEP: as a desktop program it's held whole —
    /// its timers don't fire (they do once it waits: [`sleeping`]).
    static SLEEPING: Cell<bool> = const { Cell::new(false) };
    /// The line the next kernel wait's end shows in the program's output
    /// (an INPUT's answer, as a terminal shows what was typed).
    static ECHO_NEXT: RefCell<Option<String>> = const { RefCell::new(None) };
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

/// (Stage W4) A wait of the UI kernel host's (`rapidr_ui_app::waits`:
/// ShowModal, a kernel-drawn dialog, INPUT$, DOEVENTS) started in this
/// builtin: the VM stops after it, and the page's turn continues it with
/// the wait's result ([`resume_wait`]) once the wait is over. These nest —
/// a timer's handler may wait for a box while the program waits for
/// another, the innermost ending first, as the waits' stack has them.
/// `false` where nothing can wait (no VM: a native build; a VM busy
/// otherwise).
pub fn suspend_for_wait() -> bool {
    if VM_DEPTH.with(Cell::get) != 1 || RESUME.with(|r| r.borrow().is_none()) {
        return false;
    }
    KERNEL_WAITS.with(|k| k.set(k.get() + 1));
    SUSPEND.with(|s| s.set(true));
    true
}

/// The innermost of the kernel host's waits is over: the VM goes on with
/// `value` as the result of the builtin that started it.
pub fn resume_wait(value: Value) {
    KERNEL_WAITS.with(|k| k.set(k.get().saturating_sub(1)));
    resume(value, ECHO_NEXT.with(|e| e.borrow_mut().take()));
}

/// The next [`resume_wait`] shows `line` in the program's output first.
pub fn echo_next(line: String) {
    ECHO_NEXT.with(|e| *e.borrow_mut() = Some(line));
}

/// Whether the VM is free to be continued now (not running, not between
/// two time slices).
pub fn vm_free() -> bool {
    VM_DEPTH.with(Cell::get) == 0 && !is_yielded()
}

/// SLEEP: [`pause`], the program held whole meanwhile — its timers fire
/// once it waits again (as the desktop interpreter's, whose SLEEP serves
/// no timers): a handler fired then is a turn of its own, which can wait
/// for a dialog, not one squeezed into the program on its way out of the
/// SLEEP. `false` where it can't wait.
pub fn sleep(ms: f64) -> bool {
    if !can_wait() {
        return false;
    }
    SLEEPING.with(|s| s.set(true));
    WAITING.with(|w| w.set(true));
    SUSPEND.with(|s| s.set(true));
    let wake = Closure::once_into_js(move || {
        SLEEPING.with(|s| s.set(false));
        resume(Value::Null, None);
    });
    if let Some(window) = web_sys::window() {
        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(wake.unchecked_ref(), ms.clamp(0.0, 86_400_000.0) as i32);
    }
    true
}

/// Whether the program is in a SLEEP ([`sleep`]).
pub fn sleeping() -> bool {
    SLEEPING.with(Cell::get)
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
    KERNEL_WAITS.with(Cell::get) > 0
}

/// Forgets the waits (and a yield) of a program that was replaced.
pub fn clear_modals() {
    KERNEL_WAITS.with(|k| k.set(0));
    SLEEPING.with(|s| s.set(false));
    YIELDED.with(|y| y.set(false));
    PENDING.with(|p| p.borrow_mut().clear());
}

/// Asked by the VM host after each builtin: did it open a dialog?
pub fn take_suspend() -> bool {
    SUSPEND.with(|s| s.replace(false))
}

// (the page's Open / Save picker's look)
const BACKDROP: &str = "position:fixed;inset:0;z-index:100000;display:flex;align-items:center;justify-content:center;\
background:rgba(0,0,0,0.25);font:13px Arial,'Liberation Sans',Helvetica,sans-serif;";
const BOX: &str = "min-width:120px;max-width:min(560px,90vw);background:#f0f0f0;color:#000;border:1px solid #888;\
border-radius:6px;box-shadow:0 8px 28px rgba(0,0,0,0.3);overflow:hidden;";
const TITLE: &str = "background:linear-gradient(to bottom,#4a90d9,#357abd);color:#fff;padding:0 10px;height:29px;line-height:29px;\
font-weight:bold;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;";
const FIELD: &str = "display:block;box-sizing:border-box;width:calc(100% - 24px);margin:8px 12px 0;padding:2px 4px;\
font:inherit;border:2px inset #f0f0f0;background:#fff;color:#000;";
const BUTTONS: &str = "display:flex;justify-content:center;gap:6px;padding:16px 12px 13px;";
const BUTTON: &str = "box-sizing:border-box;width:75px;height:23px;padding:0 4px;font:inherit;color:#000;cursor:pointer;\
background:#f0f0f0;border:2px outset #f0f0f0;";
const DEFAULT_BUTTON: &str = "box-sizing:border-box;width:75px;height:23px;padding:0 4px;font:inherit;color:#000;cursor:pointer;\
background:#f0f0f0;border:2px outset #f0f0f0;outline:1px solid #000;";

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
    /// Gets the picked names (none: Cancel); the UI kernel host's wait
    /// for it ends then (kernel_web).
    pub done: Rc<dyn Fn(Vec<String>)>,
}

/// An Open / Save dialog in the page: the program's files (a click picks
/// one, Ctrl / Cmd-click adds with MultiSelect), a name field (several
/// names split by `;`), Upload… (a file from the computer, kept among the
/// program's files), OK and Cancel. The program waits for it (the kernel
/// host's wait for its `Windows::ask_files`).
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
            answer(names);
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

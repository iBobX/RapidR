//! The dialogs the program waits for, for every host: MESSAGEBOX /
//! MESSAGEDLG / SHOWMESSAGE / MSGBOX, QCOLORDIALOG and QFONTDIALOG (the
//! kernel's `Dialog`s, drawn by the host as forms of their own) and
//! QOPENDIALOG / QSAVEDIALOG / QFILEDIALOG (the host's own dialog:
//! `Windows::ask_files`). Shown, their answers kept as they come (a
//! dialog's events come here, [`event`], never to the program), and each
//! answer mapped to its builtin's result when the wait for it ends — the
//! button's IDYES / mrNo, Execute's 1 / 0 / -1 with Color, FileName or the
//! font stored.
//!
//! **Nothing here waits.** [`message`] and [`execute`] show the dialog and
//! return [`Pending`]: answered already (a test's hook, nothing shown), or
//! open. Then the runtime waits: a native build steps until [`finished`]
//! gives the builtin's result; an interpreter is left `Wait::Dialog`, a wait
//! it serves itself between instructions as ShowModal's — never inside the
//! builtin, where its handlers could only queue — whose end is that result
//! (`waits::answered`). Either way the program's timers tick and their
//! handlers run while the dialog is open, as RapidQ's do (Windows' dialogs
//! run a modal loop that dispatches WM_TIMER); a handler may open a dialog
//! of its own over it.
//!
//! **Under a test** `RAPIDR_TEST_MESSAGE_DIALOG` / `_FILE_` / `_COLOR_` /
//! `_FONT_DIALOG` answer (`testhooks`): at once with nothing shown, or —
//! with `RAPIDR_TEST_DIALOG_HOLD=ms` — once the dialog has been shown and
//! waited for that long ([`give_hooked`]).
//!
//! What only the desktop does — a menu held open (its tracking tick can't
//! wait), the step loop — stays in runtime-core's `ui/kernel/dialogs.rs`.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use rapidr_ui_kernel::dialogs::{Answer, Dialog};
use rapidr_ui_kernel::tick::Instant;
use rapidr_ui_kernel::{KernelEvent, Store};
use rapidr_value::dialogs::MsgIcon;
use rapidr_value::{v_int, v_null, Value};

use crate::windows::{invalidate, restructure};
use crate::{choose_dialogs, file_dialog, forms, testhooks, timers, Program, Windows};

/// A dialog's wait: the builtin's result once the dialog answered.
type Poll = Box<dyn FnMut() -> Option<Value>>;

/// What showing a dialog leaves the runtime.
#[derive(Debug, PartialEq)]
pub enum Pending {
    /// Answered already (a test's hook, nothing shown): the builtin's
    /// result.
    Done(Value),
    /// Open: the runtime waits for it, until [`finished`] gives the
    /// builtin's result.
    Open(u64),
}

thread_local! {
    /// The kernel-drawn dialogs open now, innermost last.
    static OPEN: RefCell<Vec<Dialog>> = const { RefCell::new(Vec::new()) };
    /// Their answers, by form id, until their wait takes them.
    static ANSWERS: RefCell<HashMap<String, Answer>> = RefCell::new(HashMap::new());
    static NEXT: Cell<u64> = const { Cell::new(1) };
    /// The QFONTDIALOG each open font dialog is the program's (its Apply's
    /// OnApply), by form id.
    static APPLY_TO: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
    /// The waits for the dialogs open, by id ([`Pending::Open`]).
    static WAITING: RefCell<HashMap<u64, Poll>> = RefCell::new(HashMap::new());
    /// Test hooks' answers waiting for their time (`RAPIDR_TEST_DIALOG_HOLD`).
    static HOOKED: RefCell<Vec<Hooked>> = const { RefCell::new(Vec::new()) };
    /// The paths a test hook picked in an Open / Save dialog, by request id.
    static FILES: RefCell<HashMap<u64, Vec<String>>> = RefCell::new(HashMap::new());
}

fn next_id() -> u64 {
    NEXT.with(|n| {
        let id = n.get();
        n.set(id + 1);
        id
    })
}

/// The store of the open dialog `id` belongs to (`None`: no such dialog) —
/// the host's store answers for ids starting `rapidr:` from it.
pub fn with_store<R>(id: &str, f: impl FnOnce(&dyn Store) -> R) -> Option<R> {
    let id = id.to_lowercase();
    OPEN.with(|o| {
        let o = o.try_borrow().ok()?;
        let d = o.iter().find(|d| id == d.id || id.strip_prefix(d.id.as_str()).is_some_and(|rest| rest.starts_with(':')))?;
        Some(f(&d.store))
    })
}

/// An event of dialog form `form`'s (the host hands a dialog's kernel
/// events here, never to the program): its answer kept once it closes;
/// what it changed drawn again.
pub fn event<R: Program + Windows>(rt: R, form: &str, ev: KernelEvent) {
    let (answer, size, applied) = OPEN.with(|o| {
        let mut o = o.borrow_mut();
        let Some(d) = o.iter_mut().find(|d| d.id == form) else { return (None, None, None) };
        let before = d.size;
        let answer = d.event(&ev);
        (answer, (d.size != before).then_some(d.size), d.take_applied())
    });
    if let Some(a) = answer {
        ANSWERS.with(|m| {
            m.borrow_mut().entry(form.to_string()).or_insert(a);
        });
    }
    // (a font dialog's Apply: the program's OnApply, the dialog still open)
    if let Some(font) = applied {
        if let Some(owner) = APPLY_TO.with(|a| a.borrow().get(form).cloned()) {
            choose_dialogs::font_applied(rt, &owner, &font);
        }
    }
    // (it grew: a colour dialog's editor opened — new parts, a wider window)
    if let Some(size) = size {
        rt.dialog_resized(form, size);
        restructure();
    }
    invalidate();
}

// ------------------------------------------------------------ the waits --

/// Shows dialog `d` modally (the host makes its window); its form id. The
/// program's timers tick from now on, as while it waits for a modal form.
fn open<R: Program + Windows>(rt: R, d: Dialog) -> String {
    rt.start();
    let (id, title, size) = (d.id.clone(), d.title.clone(), d.size);
    OPEN.with(|o| o.borrow_mut().push(d));
    rt.open_dialog(&id, &title, size);
    forms::push_modal(&id);
    timers::start_all(rt);
    rt.flush();
    id
}

/// Dialog `id`'s window goes (its wait is over).
fn close<R: Windows>(rt: R, id: &str) {
    forms::remove_modal(id);
    rt.close_dialog(id);
    APPLY_TO.with(|a| a.borrow_mut().remove(id));
    if let Some(mut d) = OPEN.with(|o| {
        let mut o = o.borrow_mut();
        let i = o.iter().position(|d| d.id == id)?;
        Some(o.remove(i))
    }) {
        d.close();
    }
    rt.flush();
}

/// Dialog `id`'s answer once it has one: then it's closed.
fn answer_of<R: Program + Windows>(rt: R, id: &str) -> Option<Answer> {
    give_hooked(rt);
    let a = ANSWERS.with(|m| m.borrow_mut().remove(id))?;
    close(rt, id);
    Some(a)
}

/// A dialog open, its wait's result `poll`'s.
fn open_wait(poll: impl FnMut() -> Option<Value> + 'static) -> Pending {
    let id = next_id();
    WAITING.with(|w| w.borrow_mut().insert(id, Box::new(poll)));
    Pending::Open(id)
}

/// A wait for work the runtime does in the background — QDOWNLOAD's
/// transfer (the I/O lane's) — that the program waits for as for a dialog:
/// `poll` gives the method's result once it's done. The runtime waits for
/// it as for a dialog ([`finished`]), stepping at least every
/// [`TASK_STEP`] while one is open ([`tasks_open`]).
pub fn task_wait(poll: impl FnMut() -> Option<Value> + 'static) -> Pending {
    TASKS.with(|t| t.set(t.get() + 1));
    let mut poll = poll;
    open_wait(move || {
        let r = poll();
        if r.is_some() {
            TASKS.with(|t| t.set(t.get().saturating_sub(1)));
        }
        r
    })
}

/// How often a step comes while a background task is waited for.
pub const TASK_STEP: std::time::Duration = std::time::Duration::from_millis(20);

/// Whether a background task's wait ([`task_wait`]) is open.
pub fn tasks_open() -> bool {
    TASKS.with(|t| t.get() > 0)
}

thread_local! {
    static TASKS: Cell<u32> = const { Cell::new(0) };
}

/// The builtin's result once the dialog of wait `id` ([`Pending::Open`])
/// answered — it's closed then, and the wait forgotten; `None` while it's
/// open.
pub fn finished(id: u64) -> Option<Value> {
    let Some(mut poll) = WAITING.with(|w| w.borrow_mut().remove(&id)) else { return Some(v_null()) };
    let result = poll();
    if result.is_none() {
        WAITING.with(|w| w.borrow_mut().insert(id, poll));
    }
    result
}

/// Shows the dialog `make` makes and leaves its wait, its answer mapped by
/// `then` to the builtin's result. `hooked`: a test hook's answer — given
/// at once with nothing shown, or once the dialog has been open
/// `RAPIDR_TEST_DIALOG_HOLD`.
fn show<R: Program + Windows>(rt: R, make: impl FnOnce() -> Dialog, hooked: Option<Answer>, then: impl FnOnce(Answer) -> Value + 'static) -> Pending {
    let hooked = match (hooked, testhooks::dialog_hold()) {
        (Some(a), None) => return Pending::Done(then(a)),
        (hooked, _) => hooked,
    };
    let id = open(rt, make());
    if let Some(a) = hooked {
        hold_answer(rt, Hook::Dialog(id.clone(), a));
    }
    let mut then = Some(then);
    open_wait(move || {
        let a = answer_of(rt, &id)?;
        then.take().map(|f| f(a))
    })
}

// ------------------------------------------- answers held by a test hook --

/// A test hook's answer for an open dialog, given once it has been open
/// `RAPIDR_TEST_DIALOG_HOLD` and nothing opened over it is still open (as
/// a user answers the innermost dialog first).
struct Hooked {
    at: Instant,
    /// How many modal windows were open with it (itself included).
    depth: usize,
    answer: Hook,
}

enum Hook {
    /// A kernel-drawn dialog (its form id) and its answer.
    Dialog(String, Answer),
    /// An Open / Save request (its id) and the paths picked.
    Files(u64, Vec<String>),
}

fn hold_answer<P: Program>(p: P, answer: Hook) {
    let at = p.now() + testhooks::dialog_hold().unwrap_or_default();
    let depth = forms::modal_forms().len();
    HOOKED.with(|h| h.borrow_mut().push(Hooked { at, depth, answer }));
}

/// The test hooks' answers that are due, given to their dialogs (whose
/// waits take them as a user's) — each turn of the runtime's loop, and as a
/// wait looks.
pub fn give_hooked<P: Program>(p: P) {
    let now = p.now();
    let depth = forms::modal_forms().len();
    loop {
        let due = HOOKED.with(|h| {
            let mut h = h.borrow_mut();
            let i = h.iter().position(|x| x.at <= now && depth <= x.depth)?;
            Some(h.remove(i))
        });
        match due.map(|x| x.answer) {
            None => break,
            Some(Hook::Dialog(id, a)) => ANSWERS.with(|m| {
                m.borrow_mut().entry(id).or_insert(a);
            }),
            Some(Hook::Files(id, paths)) => {
                FILES.with(|f| f.borrow_mut().insert(id, paths));
            }
        }
    }
}

/// When the next test hook's answer may be given (the runtime's loop waits
/// no longer).
pub fn hook_wake() -> Option<Instant> {
    let depth = forms::modal_forms().len();
    HOOKED.with(|h| h.borrow().iter().filter(|x| depth <= x.depth).map(|x| x.at).min())
}

// -------------------------------------------------------------- dialogs --

/// MESSAGEBOX / MESSAGEDLG / SHOWMESSAGE / MSGBOX: `text` and `labels`'
/// buttons (the first the default), `icon` left of the text, titled
/// `title`, the icon's sound as it shows when `beep` (Windows'
/// MessageBox); `then` maps the button chosen (`None`: Escape, the close
/// box) to the builtin's result.
pub fn message<R: Program + Windows>(
    rt: R,
    title: &str,
    text: &str,
    labels: &[&str],
    icon: Option<MsgIcon>,
    beep: bool,
    then: impl FnOnce(Option<usize>) -> Value + 'static,
) -> Pending {
    let hooked = testhooks::message_dialog_answer(labels).map(Answer::Button);
    let make = || {
        let d = Dialog::message(next_id(), title, text, labels, icon);
        rt.start();
        if beep && !testhooks::under_test() {
            rt.beep(icon);
        }
        d
    };
    show(rt, make, hooked, move |a| {
        then(match a {
            Answer::Button(b) => b,
            _ => None,
        })
    })
}

/// An input box (the web's INPUT with windows shown): `text` over a field
/// holding `initial`, OK; `then` maps the text typed (`None`: Escape, the
/// close box) to the builtin's result.
pub fn input<R: Program + Windows>(rt: R, title: &str, text: &str, initial: &str, then: impl FnOnce(Option<String>) -> Value + 'static) -> Pending {
    prompt(rt, title, text, initial, "OK", None, then)
}

/// [`input`] whose default button is captioned `ok`, with a Cancel button
/// captioned `cancel` when there's one (a host's own prompt: the web's
/// "Save As" name where the browser has no save picker).
pub fn prompt<R: Program + Windows>(rt: R, title: &str, text: &str, initial: &str, ok: &str, cancel: Option<&str>, then: impl FnOnce(Option<String>) -> Value + 'static) -> Pending {
    let make = || Dialog::prompt(next_id(), title, text, initial, ok, cancel);
    show(rt, make, None, move |a| {
        then(match a {
            Answer::Text(t) => t,
            _ => None,
        })
    })
}

/// `Dialog.Execute` of the file, colour and font dialogs.
pub fn execute<R: Program + Windows>(rt: R, name: &str, comp_type: &str) -> Pending {
    if let Some((save, multi)) = file_dialog::kind(rt, name, comp_type) {
        return files(rt, name, save, multi);
    }
    match comp_type {
        "RCOLORDIALOG" => color(rt, name),
        "RFONTDIALOG" => font(rt, name),
        _ => Pending::Done(v_int(0)),
    }
}

/// QCOLORDIALOG: the colour picked into Color (&HBBGGRR, RapidQ's LONG),
/// the custom colours into Colors(i) either way.
fn color<R: Program + Windows>(rt: R, name: &str) -> Pending {
    let state = choose_dialogs::color_state(rt, name);
    let custom = state.custom;
    let hooked = testhooks::color_dialog_answer().map(|c| Answer::Color(c, custom));
    let title = choose_dialogs::title(rt, name, "Color");
    let name = name.to_string();
    show(rt, || Dialog::color(next_id(), &title, state), hooked, move |a| match a {
        Answer::Color(c, custom) => choose_dialogs::color_answered(rt, &name, c, custom),
        _ => choose_dialogs::color_answered(rt, &name, None, custom),
    })
}

/// QFONTDIALOG: the faces are the shared ones and the system's; Apply
/// stores the font so far and fires OnApply.
fn font<R: Program + Windows>(rt: R, name: &str) -> Pending {
    let req = choose_dialogs::font_request(rt, name);
    let hooked = testhooks::font_dialog_answer().map(Answer::Font);
    let title = choose_dialogs::title(rt, name, "Font");
    let (owner, name) = (name.to_string(), name.to_string());
    let make = move || {
        rt.start();
        let names = choose_dialogs::font_names(|| rt.font_families());
        let d = Dialog::font(next_id(), &title, req, &names);
        APPLY_TO.with(|a| a.borrow_mut().insert(d.id.clone(), owner));
        d
    };
    show(rt, make, hooked, move |a| {
        choose_dialogs::font_answered(
            rt,
            &name,
            match a {
                Answer::Font(f) => f,
                _ => None,
            },
        )
    })
}

/// QOPENDIALOG / QSAVEDIALOG / QFILEDIALOG `name`: the host's Open / Save
/// dialog over the innermost modal form, else the frontmost (that form on
/// the modal list meanwhile: RapidQ's dialogs are app-modal); the paths
/// picked (none: cancelled) stored.
fn files<R: Program + Windows>(rt: R, name: &str, save: bool, multi: bool) -> Pending {
    let n = name.to_string();
    let then = move |paths: Vec<String>| file_dialog::answered(rt, &n, save, multi, paths);
    let hooked = match (testhooks::file_dialog_answer(multi), testhooks::dialog_hold()) {
        (Some(paths), None) => return Pending::Done(then(paths)),
        (hooked, _) => hooked,
    };
    rt.start();
    let id = next_id();
    let parent = forms::innermost_modal().or_else(|| rt.stacking().last().cloned());
    // (on the modal list before the host is asked: a window the host shows
    // for it — the web's own prompts — goes over it, the innermost, and
    // gets the focus and the keys)
    if let Some(p) = &parent {
        forms::push_modal(p);
    }
    // (a test's answer: the host isn't asked — a headless one has no
    // dialog — but the program waits for it as for the user's)
    let asked = hooked.is_none();
    if asked {
        rt.ask_files(id, parent.as_deref(), &file_dialog::request(rt, name, save, multi));
    }
    if let Some(paths) = hooked {
        hold_answer(rt, Hook::Files(id, paths));
    }
    timers::start_all(rt);
    // (the host's dialog made now)
    rt.flush();
    let mut then = Some(then);
    open_wait(move || {
        give_hooked(rt);
        let paths = if asked { rt.files_answer(id) } else { FILES.with(|f| f.borrow_mut().remove(&id)) }?;
        if let Some(p) = &parent {
            forms::remove_last_modal(p);
        }
        then.take().map(|f| f(paths))
    })
}

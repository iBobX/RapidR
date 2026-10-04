//! Dialogs on the kernel host (the dialogs lane, plan Stage 8). **Never
//! block on a native dialog**: each one is waited for by stepping (the
//! windows paint, timers tick — native handlers run, the VM's wait until
//! the builtin returns).
//!
//! - MESSAGEBOX / MESSAGEDLG / SHOWMESSAGE / MSGBOX ([`choice`]),
//!   QCOLORDIALOG and QFONTDIALOG: kernel-drawn modal forms
//!   (`rapidr_ui_kernel::dialogs`) in a `MemStore` the runtime's store
//!   answers for (`kernel_store.rs` asks [`with_store`] for ids starting
//!   `rapidr:`); their events come here ([`event`]), never to the program.
//! - QOPENDIALOG / QSAVEDIALOG / QFILEDIALOG: the system's dialog through
//!   rfd's async API ([`HostCmd::FileDialog`], made inside a pump: a sheet
//!   on macOS), the form under it pushed onto the modal list (RapidQ's
//!   dialogs are app-modal; a sheet only blocks its own window). The
//!   request and the answer are host-neutral (`ui::file_dialog`, which also
//!   answers `RAPIDR_TEST_FILE_DIALOG` without any UI).

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::time::Duration;

use rapidr_ui_host_winit::{FileRequest, Frame, HostCmd, Icon, WindowSpec};
use rapidr_ui_kernel::dialogs::{Answer, Dialog};
use rapidr_ui_kernel::{KernelEvent, Store};

use super::{ensure_host, invalidate, pump, screen, st, step, with_kern, RtStore};
use crate::value::{v_int, Value};

thread_local! {
    /// The kernel-drawn dialogs open now, innermost last.
    static OPEN: RefCell<Vec<Dialog>> = const { RefCell::new(Vec::new()) };
    /// Their answers, by form id, until their wait takes them.
    static ANSWERS: RefCell<HashMap<String, Answer>> = RefCell::new(HashMap::new());
    static NEXT: Cell<u64> = const { Cell::new(1) };
    /// The QFONTDIALOG each open font dialog is the program's (its Apply's
    /// OnApply), by form id.
    static APPLY_TO: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
}

fn next_id() -> u64 {
    NEXT.with(|n| {
        let id = n.get();
        n.set(id + 1);
        id
    })
}

/// The store of the open dialog `id` belongs to (`None`: no such dialog).
pub(in crate::ui) fn with_store<R>(id: &str, f: impl FnOnce(&dyn Store) -> R) -> Option<R> {
    let id = id.to_lowercase();
    OPEN.with(|o| {
        let o = o.try_borrow().ok()?;
        let d = o.iter().find(|d| id == d.id || id.strip_prefix(d.id.as_str()).is_some_and(|rest| rest.starts_with(':')))?;
        Some(f(&d.store))
    })
}

/// An event of dialog form `form`'s (from `dispatch_pending`): its answer
/// kept once it closes; what it changed drawn again.
pub(super) fn event(form: &str, ev: KernelEvent) {
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
            crate::ui::choose_dialogs::font_applied(&owner, &font);
        }
    }
    // (it grew: a colour dialog's editor opened — new parts, a wider window)
    if let Some((w, h)) = size {
        with_kern(|k| {
            k.desk.resized(form, w, h);
            k.desk.cmds.push(HostCmd::Size(form.to_string()));
        });
        super::restructure();
    }
    invalidate();
}

/// The form a dialog belongs over: the innermost modal one, else the
/// frontmost shown.
fn owner() -> Option<String> {
    st(|s| s.modal.last().cloned()).or_else(|| with_kern(|k| k.desk.stacking().last().cloned()).flatten())
}

/// The application's icon (Application.Icon) for a dialog's window.
fn app_icon() -> Option<Icon> {
    let (w, h, rgba, _) = rapidr_value::globals::application_icon().and_then(|v| rapidr_value::objects::icon_pixels(&v))?;
    Some(Icon { width: w as u32, height: h as u32, rgba })
}

/// Shows dialog `d` modally and steps until it answers.
fn run(d: Dialog) -> Answer {
    ensure_host();
    let id = d.id.clone();
    let (w, h) = d.size;
    // (centred on the screen, a dialog's frame: bsDialog)
    let (sw, sh) = screen();
    let (fw, fh) = rapidr_value::layout::form_frame(3);
    let spec = WindowSpec {
        title: d.title.clone(),
        size: (w, h),
        position: Some(((sw - w - fw) / 2, (sh - h - fh) / 2)),
        border: true,
        icon: app_icon(),
        frame: Frame { resizable: false, close: true, minimize: false, maximize: false },
        state: 0,
    };
    OPEN.with(|o| o.borrow_mut().push(d));
    with_kern(|k| {
        k.desk.ensure_form(&RtStore, &id, false, spec);
        k.desk.show(&id);
    });
    st(|s| s.modal.push(id.clone()));
    pump(Some(Duration::ZERO));
    let answer = loop {
        if let Some(a) = ANSWERS.with(|m| m.borrow_mut().remove(&id)) {
            break a;
        }
        step(None);
    };
    st(|s| s.modal.retain(|m| *m != id));
    with_kern(|k| k.desk.forget(&id));
    if let Some(mut d) = OPEN.with(|o| {
        let mut o = o.borrow_mut();
        let i = o.iter().position(|d| d.id == id)?;
        Some(o.remove(i))
    }) {
        d.close();
    }
    pump(Some(Duration::ZERO));
    answer
}

/// MESSAGEBOX / MESSAGEDLG / SHOWMESSAGE / MSGBOX: `text` and `labels`'
/// buttons (the first the default), `icon` left of the text, titled
/// `title`; the button chosen, or `None` (Escape, the close box). `beep`:
/// the icon's sound as it shows (Windows' MessageBox; never under a test,
/// nor on the headless host).
pub(super) fn choice(title: &str, text: &str, labels: &[&str], icon: Option<rapidr_value::dialogs::MsgIcon>, beep: bool) -> Option<usize> {
    // (timers during native menu tracking: a tracking tick's handler can't
    // wait inside the system's loop. A box with one button answers it now
    // and shows once the menu has closed; a choice is answered dismissed)
    if super::held() {
        if labels.len() > 1 {
            super::held_cannot("a MESSAGEBOX / MESSAGEDLG with a choice");
            return None;
        }
        let answer = (!labels.is_empty()).then_some(0);
        let (title, text, labels) = (title.to_string(), text.to_string(), labels.iter().map(|l| l.to_string()).collect::<Vec<_>>());
        super::after_held(Box::new(move || {
            let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
            choice(&title, &text, &labels, icon, beep);
        }));
        return answer;
    }
    let d = Dialog::message(next_id(), title, text, labels, icon);
    ensure_host();
    if beep && !crate::ui::testhooks::under_test() && with_kern(|k| k.host.headless()) == Some(false) {
        rapidr_ui_host_winit::platform::beep(icon);
    }
    match run(d) {
        Answer::Button(b) => b,
        _ => None,
    }
}

/// `Dialog.Execute` for the file, colour and font dialogs.
pub(super) fn execute(name: &str, comp_type: &str) -> Value {
    // (timers during native menu tracking: a tracking tick's handler can't
    // wait inside the system's loop — Cancel)
    if super::held() {
        super::held_cannot("a file, colour or font dialog");
        return v_int(0);
    }
    if let Some((save, multi)) = crate::ui::file_dialog::kind(name, comp_type) {
        return crate::ui::file_dialog::execute(name, save, multi, pick_files);
    }
    match comp_type {
        // (Color is &HBBGGRR, RapidQ's LONG; the custom colours come back
        // either way)
        "RCOLORDIALOG" => crate::ui::choose_dialogs::color_execute(name, |title, state| {
            let custom = state.custom;
            match run(Dialog::color(next_id(), title, state)) {
                Answer::Color(c, custom) => (c, custom),
                _ => (None, custom),
            }
        }),
        // (the faces: the shared ones and the system's, as fontique finds
        // them; Apply stores the font so far and fires OnApply)
        "RFONTDIALOG" => crate::ui::choose_dialogs::font_execute(name, |title, req| {
            ensure_host();
            let names = crate::ui::choose_dialogs::font_names(|| with_kern(|k| k.desk.text.family_names()).unwrap_or_default());
            let d = Dialog::font(next_id(), title, req, &names);
            APPLY_TO.with(|a| a.borrow_mut().insert(d.id.clone(), name.to_string()));
            let id = d.id.clone();
            let answer = run(d);
            APPLY_TO.with(|a| a.borrow_mut().remove(&id));
            match answer {
                Answer::Font(f) => f,
                _ => None,
            }
        }),
        _ => v_int(0),
    }
}

/// The system's Open / Save dialog for `req` (rfd, async), over the
/// frontmost form; the paths picked (none: cancelled).
fn pick_files(req: &crate::ui::file_dialog::Request) -> Vec<String> {
    ensure_host();
    let id = next_id();
    let parent = owner();
    let host_req = FileRequest {
        save: req.save,
        multi: req.multi,
        title: req.title.clone(),
        filters: req.filters.iter().map(|f| (f.name.clone(), f.patterns.clone())).collect(),
        filter_index: req.filter_index,
        dir: req.dir.clone(),
        file_name: req.file_name.clone(),
    };
    with_kern(|k| k.desk.cmds.push(HostCmd::FileDialog { id, form: parent.clone(), req: host_req }));
    // (app-modal: the form under the sheet takes the input there is)
    if let Some(p) = &parent {
        st(|s| s.modal.push(p.clone()));
    }
    // (made now, inside a pump)
    pump(Some(Duration::ZERO));
    let paths = loop {
        if let Some(paths) = with_kern(|k| k.host.file_dialog(id)).flatten() {
            break paths;
        }
        step(None);
    };
    if let Some(p) = &parent {
        st(|s| {
            if let Some(i) = s.modal.iter().rposition(|m| m == p) {
                s.modal.remove(i);
            }
        });
    }
    paths
}

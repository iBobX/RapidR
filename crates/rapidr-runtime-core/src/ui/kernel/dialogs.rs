//! Dialogs on the kernel host (the dialogs lane, plan Stage 8). **Never
//! block on a native dialog**: each one is waited for by stepping (the
//! windows paint, timers tick — native handlers run, the VM's wait until
//! the builtin returns, as on FLTK).
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
use rapidr_ui_kernel::dialogs::{self as kd, Answer, Dialog, FontChoice};
use rapidr_ui_kernel::{KernelEvent, Store};

use super::{ensure_host, invalidate, pump, screen, st, step, with_kern, RtStore};
use crate::object::{rp_comp_get, rp_comp_set};
use crate::value::{v_int, v_str, Value};

thread_local! {
    /// The kernel-drawn dialogs open now, innermost last.
    static OPEN: RefCell<Vec<Dialog>> = const { RefCell::new(Vec::new()) };
    /// Their answers, by form id, until their wait takes them.
    static ANSWERS: RefCell<HashMap<String, Answer>> = RefCell::new(HashMap::new());
    static NEXT: Cell<u64> = const { Cell::new(1) };
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
    let answer = OPEN.with(|o| o.borrow_mut().iter_mut().find(|d| d.id == form).and_then(|d| d.event(&ev)));
    if let Some(a) = answer {
        ANSWERS.with(|m| {
            m.borrow_mut().entry(form.to_string()).or_insert(a);
        });
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
    if let Some((save, multi)) = crate::ui::file_dialog::kind(name, comp_type) {
        return crate::ui::file_dialog::execute(name, save, multi, pick_files);
    }
    let caption = |default: &str| Some(rp_comp_get(name, "caption").to_string_val()).filter(|c| !c.is_empty()).unwrap_or_else(|| default.to_string());
    match comp_type {
        "RCOLORDIALOG" => {
            // (Color is &HBBGGRR, RapidQ's LONG)
            let current = rp_comp_get(name, "color").to_i64();
            match run(Dialog::color(next_id(), &caption("Color"), current, &[])) {
                Answer::Color(Some(c)) => {
                    rp_comp_set(name, "color", v_int(c));
                    v_int(1)
                }
                _ => v_int(0),
            }
        }
        "RFONTDIALOG" => {
            let font_name = rp_comp_get(name, "fontname").to_string_val();
            let size = rp_comp_get(name, "fontsize").to_i64();
            let chosen = FontChoice {
                name: if font_name.trim().is_empty() { "Arial".into() } else { font_name },
                size: if size > 0 { size } else { 12 },
                bold: rp_comp_get(name, "fontbold").to_bool(),
                italic: rp_comp_get(name, "fontitalic").to_bool(),
            };
            match run(Dialog::font(next_id(), &caption("Font"), &chosen, &kd::FONT_NAMES)) {
                Answer::Font(Some(f)) => {
                    rp_comp_set(name, "fontname", v_str(&f.name));
                    rp_comp_set(name, "fontsize", v_int(f.size));
                    rp_comp_set(name, "fontbold", v_int(i64::from(f.bold)));
                    rp_comp_set(name, "fontitalic", v_int(i64::from(f.italic)));
                    v_int(1)
                }
                _ => v_int(0),
            }
        }
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

//! Dialogs on the kernel host (the dialogs lane, plan Stage 8): the
//! desktop's half. Which dialog shows, its answers and the waits for them
//! are `rapidr_ui_app::dialogs`' (shared with the web); here are what the
//! desktop host does for them — `Windows`' dialog methods: a kernel-drawn
//! dialog's window (a `MemStore` form the runtime's store answers for,
//! `kernel_store.rs`; centred on the screen, a dialog's frame), rfd's Open /
//! Save sheet ([`HostCmd::FileDialog`], made inside a pump: a sheet on
//! macOS; its future polled by the host), the beep, the system's fonts —
//! and the wait.
//!
//! **Never block on a native dialog.** A native build steps until the
//! dialog answers ([`block`]: the windows paint, timers tick and their
//! handlers run); the interpreter is left a wait it serves itself between
//! instructions (`Wait::Dialog`), so its timers' handlers run during the
//! dialog too, and the wait ends with the builtin's result
//! (`waits::answered` in `gui_pump_wait`). A tracking tick's handler (a
//! native menu held open) can't wait at all: see `choice` and `execute`.

use rapidr_ui_app::dialogs::{self, Pending};
use rapidr_ui_app::file_dialog::Request;
use rapidr_ui_app::waits::{self, Wait};
use rapidr_ui_host_winit::{FileRequest, Frame, HostCmd, Icon, WindowSpec};

use super::{ensure_host, screen, step, with_kern, RtStore};
use crate::ui::program::Rt;
use crate::value::{v_int, v_null, Value};

/// The application's icon (Application.Icon) for a dialog's window.
fn app_icon() -> Option<Icon> {
    let (w, h, rgba, _) = rapidr_value::globals::application_icon().and_then(|v| rapidr_value::objects::icon_pixels(&v))?;
    Some(Icon { width: w as u32, height: h as u32, rgba })
}

/// (`Windows::open_dialog`) Dialog `id`'s window: centred on the screen, a
/// dialog's frame (bsDialog: not resizable, a close box), shown.
pub(super) fn open_window(id: &str, title: &str, (w, h): (i64, i64)) {
    ensure_host();
    let (sw, sh) = screen();
    let (fw, fh) = rapidr_value::layout::form_frame(3);
    let spec = WindowSpec {
        title: title.to_string(),
        size: (w, h),
        position: Some(((sw - w - fw) / 2, (sh - h - fh) / 2)),
        border: true,
        icon: app_icon(),
        frame: Frame { resizable: false, close: true, minimize: false, maximize: false },
        state: 0,
        modified: false,
    };
    with_kern(|k| {
        k.desk.ensure_form(&RtStore, id, false, spec);
        k.desk.show(id);
    });
}

/// (`Windows::close_dialog`) Dialog `id`'s window gone for good.
pub(super) fn close_window(id: &str) {
    with_kern(|k| k.desk.forget(id));
}

/// (`Windows::dialog_resized`) A colour dialog's editor opened: a wider
/// window.
pub(super) fn resized(id: &str, (w, h): (i64, i64)) {
    with_kern(|k| {
        k.desk.resized(id, w, h);
        k.desk.cmds.push(HostCmd::Size(id.to_string()));
    });
}

/// (`Windows::beep`) The icon's sound (never on the headless host).
pub(super) fn beep(icon: Option<rapidr_value::dialogs::MsgIcon>) {
    if with_kern(|k| k.host.headless()) == Some(false) {
        rapidr_ui_host_winit::platform::beep(icon);
    }
}

/// (`Windows::font_families`) The faces fontique finds on the system.
pub(super) fn font_families() -> Vec<String> {
    with_kern(|k| k.desk.text.family_names()).unwrap_or_default()
}

/// (`Windows::ask_files`) The system's Open / Save dialog for `req`
/// (rfd's async one, made in the next pump), over window `form`.
pub(super) fn ask_files(id: u64, form: Option<&str>, req: &Request) {
    ensure_host();
    let host_req = FileRequest {
        save: req.save,
        multi: req.multi,
        title: req.title.clone(),
        filters: req.filters.iter().map(|f| (f.name.clone(), f.patterns.clone())).collect(),
        filter_index: req.filter_index,
        dir: req.dir.clone(),
        file_name: req.file_name.clone(),
        folder: req.folder,
    };
    with_kern(|k| k.desk.cmds.push(HostCmd::FileDialog { id, form: form.map(str::to_string), req: host_req }));
}

/// (`Windows::files_answer`) Request `id`'s paths once its sheet closed.
pub(super) fn files_answer(id: u64) -> Option<Vec<String>> {
    with_kern(|k| k.host.file_dialog(id)).flatten()
}

/// The program waits for a dialog shown: answered already, or — a native
/// build — steps until it answers; the interpreter is left a wait it serves
/// itself (`Wait::Dialog`), whose result the VM puts in place of this one.
fn wait(p: Pending) -> Value {
    match p {
        Pending::Done(v) => v,
        Pending::Open(id) if waits::cooperative() => {
            waits::start(Wait::Dialog(id));
            v_null()
        }
        Pending::Open(id) => block(id),
    }
}

/// Steps until the dialog of wait `id` answered: its builtin's result. (A
/// VM lent to a wait below this one — a box a held menu put off, shown from
/// a pump — runs its handlers meanwhile; a native build lends nothing.)
fn block(id: u64) -> Value {
    loop {
        if let Some(v) = dialogs::finished(id) {
            return v;
        }
        step(None);
        crate::object::rp_serve_program();
    }
}

/// MESSAGEBOX / MESSAGEDLG / SHOWMESSAGE / MSGBOX: `then` maps the button
/// chosen (`None`: Escape, the close box) to the builtin's result
/// (`rapidr_ui_app::dialogs::message`).
pub(super) fn choice(
    title: &str,
    text: &str,
    labels: &[&str],
    icon: Option<rapidr_value::dialogs::MsgIcon>,
    beep: bool,
    then: impl FnOnce(Option<usize>) -> Value + 'static,
) -> Value {
    // (timers during native menu tracking: a tracking tick's handler can't
    // wait inside the system's loop. A box with one button answers it now
    // and shows once the menu has closed; a choice is answered dismissed)
    if super::held() {
        if labels.len() > 1 {
            super::held_cannot("a MESSAGEBOX / MESSAGEDLG with a choice");
            return then(None);
        }
        let answer = (!labels.is_empty()).then_some(0);
        let (title, text, labels) = (title.to_string(), text.to_string(), labels.iter().map(|l| l.to_string()).collect::<Vec<_>>());
        super::after_held(Box::new(move || {
            let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
            // (shown from a pump, not by the program: waited for here)
            if let Pending::Open(id) = dialogs::message(Rt, &title, &text, &labels, icon, beep, |_| v_null()) {
                block(id);
            }
        }));
        return then(answer);
    }
    wait(dialogs::message(Rt, title, text, labels, icon, beep, then))
}

/// `Dialog.Execute` for the file, colour and font dialogs.
pub(super) fn execute(name: &str, comp_type: &str) -> Value {
    // (timers during native menu tracking: a tracking tick's handler can't
    // wait inside the system's loop — Cancel)
    if super::held() {
        super::held_cannot("a file, colour or font dialog");
        return v_int(0);
    }
    wait(dialogs::execute(Rt, name, comp_type))
}

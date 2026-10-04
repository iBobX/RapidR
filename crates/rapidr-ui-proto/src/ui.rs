//! What runtime-core's `src/ui/kernel.rs` would be (Stage 0 spike A): the
//! program's side of the host. Program code — handlers, timers, script
//! steps — runs only here, in [`step`], *between* pumps, never inside a
//! winit callback; so a handler may block in [`show_modal`], which calls
//! `step` again (a nested pump, legal because no callback is on the stack).
//!
//! ```text
//! step(max_wait):
//!   t = min(max_wait, next timer, next script step)
//!   host.pump(t)                 -> HostEvents queued by the callbacks
//!   dispatch HostEvents          (handlers; may nest step())
//!   fire due timers              (heap; re-armed from now)
//!   one script step              (when due and nothing is queued)
//! ```

use std::cell::RefCell;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::future::Future;
use std::path::PathBuf;
use std::rc::Rc;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use crate::form::{Event, Form, Key, Kind, Mods};
use crate::host::Host;
use crate::kernel::{Dialog, FormId, HostCmd, HostEvent, Kernel};
use crate::script::{Script, Step};

pub type TimerId = usize;

struct Timer {
    interval: Duration,
    enabled: bool,
}

#[derive(Default)]
pub struct UiStats {
    pub ticks: u64,
    pub max_late: Duration,
    /// Each step's longest stall between two timer ticks it fired.
    pub steps: u64,
    pub max_nesting: usize,
}

struct Ui {
    host: Box<dyn Host>,
    k: Kernel,
    timers: Vec<Timer>,
    heap: BinaryHeap<Reverse<(Instant, TimerId)>>,
    script: Option<Script>,
    capture: Option<String>,
    shots: usize,
    stats: UiStats,
    nesting: usize,
    /// QBUTTON.ModalResult: a click closes the form with it.
    results: HashMap<(FormId, String), i64>,
    /// (ticks at the phase start, worst lateness in it): script `print`s.
    phase: (u64, Duration),
}

type Handler = Rc<dyn Fn()>;

#[derive(Default)]
#[allow(clippy::type_complexity)]
struct Handlers {
    on: HashMap<(FormId, String, &'static str), Handler>,
    any: HashMap<FormId, Rc<dyn Fn(&Event)>>,
    close: HashMap<FormId, Handler>,
    resize: HashMap<FormId, Rc<dyn Fn(i64, i64)>>,
    menu: Option<Rc<dyn Fn(&str)>>,
    timers: HashMap<TimerId, Handler>,
}

thread_local! {
    static UI: RefCell<Option<Ui>> = const { RefCell::new(None) };
    static HANDLERS: RefCell<Handlers> = RefCell::new(Handlers::default());
}

/// Starts the UI with a host (once, before any form shows).
pub fn init(host: Box<dyn Host>, script: Option<Script>, capture: Option<String>) {
    let mut k = Kernel::new();
    k.ignore_user = script.is_some() && host.headless();
    UI.with(|u| {
        *u.borrow_mut() =
            Some(Ui { host, k, timers: Vec::new(), heap: BinaryHeap::new(), script, capture, shots: 0, stats: UiStats::default(), nesting: 0, results: HashMap::new(), phase: (0, Duration::ZERO) })
    });
}

fn with<R>(f: impl FnOnce(&mut Ui) -> R) -> R {
    UI.with(|u| {
        let mut u = u.try_borrow_mut().expect("ui: re-entered while borrowed (program code inside a host callback?)");
        f(u.as_mut().expect("ui::init first"))
    })
}

pub fn host_name() -> &'static str {
    with(|u| u.host.name())
}

pub fn host_report() -> String {
    with(|u| u.host.report())
}

// ------------------------------------------------------------- forms --

pub fn create_form(name: &str, form: Form) -> FormId {
    with(|u| u.k.add(name, form))
}

pub fn with_form<R>(f: FormId, g: impl FnOnce(&mut Form) -> R) -> R {
    with(|u| {
        let k = &mut u.k.forms[f];
        k.dirty = true;
        g(&mut k.form)
    })
}

pub fn set_caption(f: FormId, comp: &str, s: &str) {
    with_form(f, |form| {
        if let Some(i) = form.find(comp) {
            form.components[i].set_caption(s);
        }
    });
}

pub fn caption(f: FormId, comp: &str) -> String {
    with(|u| {
        let form = &u.k.forms[f].form;
        form.find(comp).map_or(String::new(), |i| match &form.components[i].kind {
            Kind::Label { caption } | Kind::Button { caption } => caption.clone(),
            Kind::Edit(e) => e.model.text(),
            _ => String::new(),
        })
    })
}

pub fn shown(f: FormId) -> bool {
    with(|u| u.k.forms[f].shown)
}

/// Form.Show: the window exists (one zero pump) before this returns.
pub fn show(f: FormId) {
    with(|u| {
        u.k.show(f);
        let Ui { host, k, .. } = u;
        host.pump(Some(Duration::ZERO), k);
    });
}

/// Form.Close (ModalResult kept).
pub fn close(f: FormId) {
    with(|u| u.k.hide(f));
    let h = HANDLERS.with(|h| h.borrow().close.get(&f).cloned());
    if let Some(h) = h {
        h();
    }
}

/// Form.ModalResult = r: a modal form closes.
pub fn set_modal_result(f: FormId, r: i64) {
    with(|u| u.k.forms[f].modal_result = r);
    if r != 0 {
        close(f);
    }
}

pub fn set_button_result(f: FormId, comp: &str, r: i64) {
    with(|u| u.results.insert((f, comp.to_ascii_lowercase()), r));
}

/// Form.ShowModal: blocks, pumping, until the form closes; its
/// ModalResult. Nests (a handler may call it).
pub fn show_modal(f: FormId) -> i64 {
    with(|u| {
        u.k.forms[f].modal_result = 0;
        u.k.modal.push(f);
        u.nesting += 1;
        u.stats.max_nesting = u.stats.max_nesting.max(u.nesting);
    });
    show(f);
    while shown(f) {
        step(None);
    }
    with(|u| {
        u.k.modal.retain(|&m| m != f);
        u.nesting -= 1;
        u.k.forms[f].modal_result
    })
}

/// The program's end: the process exits (the event loop is never told to
/// exit). Pending window commands run first, so closed forms' windows go
/// while everything is alive; then the host is leaked: dropping winit
/// windows during the main thread's TLS teardown (which `exit` runs on
/// macOS) panics inside winit's window delegate, a non-unwinding frame,
/// and aborts.
pub fn end(code: i32) -> ! {
    with(|u| {
        let Ui { host, k, .. } = u;
        host.pump(Some(Duration::ZERO), k);
    });
    UI.with(|u| std::mem::forget(u.borrow_mut().take()));
    std::process::exit(code)
}

/// The main loop after the program's main: until no form is shown.
pub fn run() {
    while with(|u| u.k.forms.iter().any(|f| f.shown)) {
        step(None);
    }
}

// ----------------------------------------------------------- handlers --

pub fn on(f: FormId, comp: &str, what: &'static str, h: impl Fn() + 'static) {
    HANDLERS.with(|hs| hs.borrow_mut().on.insert((f, comp.to_ascii_lowercase(), what), Rc::new(h)));
}

/// Every OnClick / OnChange of a form (the demo's dispatcher).
pub fn on_any(f: FormId, h: impl Fn(&Event) + 'static) {
    HANDLERS.with(|hs| hs.borrow_mut().any.insert(f, Rc::new(h)));
}

#[allow(dead_code)] // (Stage 3: QFORM.OnClose)
pub fn on_close(f: FormId, h: impl Fn() + 'static) {
    HANDLERS.with(|hs| hs.borrow_mut().close.insert(f, Rc::new(h)));
}

pub fn on_resize(f: FormId, h: impl Fn(i64, i64) + 'static) {
    HANDLERS.with(|hs| hs.borrow_mut().resize.insert(f, Rc::new(h)));
}

pub fn on_menu(h: impl Fn(&str) + 'static) {
    HANDLERS.with(|hs| hs.borrow_mut().menu = Some(Rc::new(h)));
}

// -------------------------------------------------------------- timers --

/// A QTIMER: `h` every `interval` while enabled, whenever `step` runs.
pub fn add_timer(interval: Duration, h: impl Fn() + 'static) -> TimerId {
    let id = with(|u| {
        u.timers.push(Timer { interval, enabled: true });
        let id = u.timers.len() - 1;
        u.heap.push(Reverse((Instant::now() + interval, id)));
        id
    });
    HANDLERS.with(|hs| hs.borrow_mut().timers.insert(id, Rc::new(h)));
    id
}

#[allow(dead_code)] // (Stage 3: QTIMER.Enabled)
pub fn set_timer_enabled(t: TimerId, on: bool) {
    with(|u| {
        let was = u.timers[t].enabled;
        u.timers[t].enabled = on;
        if on && !was {
            let at = Instant::now() + u.timers[t].interval;
            u.heap.push(Reverse((at, t)));
        }
    });
}

pub fn stats() -> (u64, Duration, usize, u64, usize, u64) {
    with(|u| (u.stats.ticks, u.stats.max_late, u.k.stats.held_pumps.len(), u.k.stats.pumps, u.stats.max_nesting, u.k.stats.redraws))
}

/// (frames drawn by vello_cpu into windows, their total time, Resized
/// events seen in live resize)
pub fn frame_stats() -> (u64, Duration, u64) {
    with(|u| (u.k.stats.cpu_frames, u.k.stats.cpu_frame_time, u.k.stats.live_resizes))
}

pub fn held_pumps() -> Vec<crate::kernel::Held> {
    with(|u| u.k.stats.held_pumps.clone())
}

// ------------------------------------------------------------- dialogs --

/// An open-file dialog that doesn't block the event loop: rfd's async
/// dialog, created inside a pump (a sheet on macOS), polled after each
/// step; the windows keep painting and timers keep ticking meanwhile.
/// `RAPIDR_TEST_FILE_DIALOG` answers without any UI.
pub fn open_file_dialog(parent: FormId) -> Option<PathBuf> {
    let id = with(|u| {
        let id = u.k.next_dialog;
        u.k.next_dialog += 1;
        if let Some(p) = std::env::var_os("RAPIDR_TEST_FILE_DIALOG") {
            u.k.dialogs.push((id, Dialog::Done(Some(p.into()))));
        } else {
            u.k.dialogs.push((id, Dialog::Requested));
            u.k.cmds.push(HostCmd::OpenFileDialog { id, parent });
        }
        id
    });
    loop {
        let done = with(|u| {
            let waker = u.host.waker();
            let i = u.k.dialogs.iter().position(|d| d.0 == id)?;
            let ready = match &mut u.k.dialogs[i].1 {
                Dialog::Done(p) => Some(p.take()),
                Dialog::Pending(fut) => match fut.as_mut().poll(&mut Context::from_waker(&waker)) {
                    Poll::Ready(p) => Some(p),
                    Poll::Pending => None,
                },
                Dialog::Requested => None,
            };
            if ready.is_some() {
                u.k.dialogs.remove(i);
            }
            ready
        });
        if let Some(p) = done {
            return p;
        }
        step(None);
    }
}

/// Spike: rfd's *async* dialog created between pumps (program code), as a
/// naive port would: NSApp isn't running then, so rfd falls back to a
/// blocking runModal.
pub fn async_dialog_between_pumps() -> Option<PathBuf> {
    let fut = rfd::AsyncFileDialog::new().set_title("Async, created between pumps").pick_file();
    let waker = with(|u| u.host.waker());
    let mut fut = Box::pin(fut);
    loop {
        if let Poll::Ready(p) = fut.as_mut().poll(&mut Context::from_waker(&waker)) {
            return p.map(|h| h.path().to_path_buf());
        }
        step(None);
    }
}

/// Spike: a blocking rfd dialog in program code (between pumps).
pub fn blocking_dialog_between_pumps() -> Option<PathBuf> {
    rfd::FileDialog::new().set_title("Open (blocking, between pumps)").pick_file()
}

/// Spike: a blocking rfd dialog run inside a winit callback.
pub fn blocking_dialog_in_callback() {
    with(|u| u.k.cmds.push(HostCmd::BlockingDialogInCallback));
}

// ---------------------------------------------------------------- step --

/// One step of the innermost wait (see the module's doc).
pub fn step(max_wait: Option<Duration>) {
    let (events, due, action) = with(|u| {
        u.stats.steps += 1;
        let now = Instant::now();
        let mut t = max_wait;
        let mut min = |d: Duration| t = Some(t.map_or(d, |t| t.min(d)));
        if let Some(Reverse((at, _))) = u.heap.peek() {
            min(at.saturating_duration_since(now));
        }
        if let Some(at) = u.script.as_ref().and_then(Script::next_at) {
            min(at.saturating_duration_since(now));
        }
        if !u.k.events.is_empty() {
            min(Duration::ZERO);
        }
        let Ui { host, k, .. } = u;
        host.pump(t, k);
        let events = std::mem::take(&mut u.k.events);
        // due timers, re-armed from now (as FLTK's repeat_timeout drifts)
        let now = Instant::now();
        let mut due = Vec::new();
        while let Some(&Reverse((at, id))) = u.heap.peek() {
            if at > now {
                break;
            }
            u.heap.pop();
            if !u.timers[id].enabled {
                continue;
            }
            u.stats.max_late = u.stats.max_late.max(now - at);
            u.phase.1 = u.phase.1.max(now - at);
            if now - at > Duration::from_millis(100) {
                eprintln!("[ui] timer {id} fired {:.0} ms late (step {})", (now - at).as_secs_f64() * 1e3, u.stats.steps);
            }
            u.stats.ticks += 1;
            u.heap.push(Reverse((now + u.timers[id].interval, id)));
            due.push(id);
        }
        // one script step, when due and nothing is waiting to be handled
        let action = if events.is_empty() { u.script.as_mut().and_then(|s| s.take_due(now)) } else { None };
        (events, due, action)
    });
    for e in events {
        dispatch(e);
    }
    for id in due {
        let h = HANDLERS.with(|hs| hs.borrow().timers.get(&id).cloned());
        if let Some(h) = h {
            h();
        }
    }
    if let Some(a) = action {
        run_script_step(a);
    }
}

fn dispatch(e: HostEvent) {
    match e {
        HostEvent::Kernel(f, ev) => {
            let (i, what) = match ev {
                Event::Click(i) => (i, "click"),
                Event::Change(i) => (i, "change"),
            };
            let name = with(|u| u.k.forms[f].form.components[i].name.to_ascii_lowercase());
            let (h, any) = HANDLERS.with(|hs| {
                let hs = hs.borrow();
                (hs.on.get(&(f, name.clone(), what)).cloned(), hs.any.get(&f).cloned())
            });
            if let Some(any) = any {
                any(&ev);
            }
            if let Some(h) = h {
                h();
            }
            if what == "click" {
                if let Some(r) = with(|u| u.results.get(&(f, name)).copied()) {
                    set_modal_result(f, r);
                }
            }
        }
        HostEvent::Close(f) => {
            // the close box: mrCancel for a modal form
            with(|u| {
                if u.k.modal.contains(&f) && u.k.forms[f].modal_result == 0 {
                    u.k.forms[f].modal_result = 2;
                }
            });
            close(f);
        }
        HostEvent::Resized(f, w, h) => {
            let h2 = HANDLERS.with(|hs| hs.borrow().resize.get(&f).cloned());
            if let Some(r) = h2 {
                r(w, h);
            }
        }
        HostEvent::Menu(id) => {
            let h = HANDLERS.with(|hs| hs.borrow().menu.clone());
            if let Some(h) = h {
                h(&id);
            }
        }
        HostEvent::Wake => {}
    }
}

// -------------------------------------------------------------- script --

fn center(u: &Ui, f: FormId, comp: &str) -> Option<(f64, f64)> {
    let form = &u.k.forms[f].form;
    let c = &form.components[form.find(comp)?];
    Some((c.left as f64 + c.width as f64 / 2.0, c.top as f64 + c.height as f64 / 2.0))
}

/// Script steps go through the kernel's input entry points, the ones the
/// host's callbacks call: hit testing, focus, capture and modality apply.
fn run_script_step(s: Step) {
    match s {
        Step::Wait(_) => {}
        Step::Click(form, comp) => with(|u| {
            let Some(f) = u.k.find(&form) else {
                return eprintln!("script: no form {form}");
            };
            let Some((x, y)) = center(u, f, &comp) else {
                return eprintln!("script: no {form}.{comp}");
            };
            u.k.mouse_move(f, x, y);
            u.k.mouse_down(f, x, y, Mods::default());
            u.k.mouse_up(f, x, y);
        }),
        Step::Key(form, key) => with(|u| {
            let Some(f) = u.k.find(&form) else { return };
            let typed = match &key {
                Key::Char(c) => Some(c.to_string()),
                _ => None,
            };
            u.k.key(f, key, Mods::default(), typed.as_deref(), &mut crate::demo::NoClipboard(None));
        }),
        Step::Close(form) => with(|u| {
            if let Some(f) = u.k.find(&form) {
                u.k.close_box(f);
            }
        }),
        Step::Menu(id) => with(|u| u.k.events.push(HostEvent::Menu(id))),
        Step::Resize(form, w, h) => with(|u| {
            if let Some(f) = u.k.find(&form) {
                u.k.resized(f, w, h);
                u.k.cmds.push(HostCmd::SetSize(f));
            }
        }),
        Step::Capture => capture_all(),
        Step::Expect(form, comp, want) => {
            let f = with(|u| u.k.find(&form));
            let got = f.map(|f| caption(f, &comp)).unwrap_or_default();
            let ok = got.contains(want.as_str());
            println!("script: expect {form}.{comp} contains {want:?}: {} (got {got:?})", if ok { "PASS" } else { "FAIL" });
            if !ok {
                with(|u| u.script.as_mut().map(|s| s.failures += 1));
            }
        }
        Step::Print(s) => {
            let (ticks, late, live) = with(|u| {
                let r = (u.stats.ticks - u.phase.0, u.phase.1, u.k.stats.live_resizes);
                u.phase = (u.stats.ticks, Duration::ZERO);
                r
            });
            println!("script: (previous phase: {ticks} timer ticks, worst lateness {:.0} ms; Resized-in-live-resize so far {live})", late.as_secs_f64() * 1e3);
            println!("script: {s}");
        }
        Step::LiveResize(..) | Step::CmdKey(..) | Step::CancelPanels(_) | Step::Poke(..) | Step::MenuTrack(..) => probe(s),
    }
}

#[cfg(target_os = "macos")]
fn probe(s: Step) {
    use crate::macos_probe as p;
    let view = |form: &str| with(|u| u.k.find(form).and_then(|f| u.host.native_view(f)));
    match s {
        Step::LiveResize(form, dx, ms) => match view(&form).and_then(p::window_info) {
            Some(info) => p::live_resize(info, dx, ms),
            None => eprintln!("script: liveresize: {form} has no window (headless?)"),
        },
        Step::CmdKey(form, c, code) => match view(&form).and_then(p::window_info) {
            Some(info) => p::key_equivalent(info, c, code),
            None => eprintln!("script: cmdkey: {form} has no window"),
        },
        Step::CancelPanels(ms) => p::cancel_panels_after(ms),
        Step::Poke(form, ms) => match view(&form) {
            Some(v) => p::poke_after(v, ms),
            None => eprintln!("script: poke: {form} has no window"),
        },
        Step::MenuTrack(form, ms) => match view(&form) {
            Some(v) => p::menu_tracking(v, ms),
            None => eprintln!("script: menutrack: {form} has no window"),
        },
        _ => {}
    }
}

#[cfg(not(target_os = "macos"))]
fn probe(s: Step) {
    eprintln!("script: {s:?} is a macOS probe");
}

/// Every shown form, bottom to top, through the CPU renderer at its
/// window's scale: `<prefix>-<n>.bmp`.
pub fn capture_all() {
    with(|u| {
        let Some(prefix) = u.capture.clone() else {
            return eprintln!("script: capture without RAPIDR_CAPTURE / --shots");
        };
        for f in u.k.stacking() {
            let scale = u.host.scale(f);
            let k = &mut u.k.forms[f];
            let px = crate::cpu::capture(&mut k.form, &mut u.k.text, scale);
            u.shots += 1;
            let path = format!("{prefix}-{}.bmp", u.shots);
            match std::fs::write(&path, rapidr_value::objects::codec::encode_bmp(&px)) {
                Ok(()) => println!("capture: {path} = {} ({}x{} at {scale}x)", k.name, px.width, px.height),
                Err(e) => eprintln!("capture: {path}: {e}"),
            }
        }
    });
}

pub fn script_failures() -> Option<u32> {
    with(|u| u.script.as_ref().map(|s| s.failures))
}

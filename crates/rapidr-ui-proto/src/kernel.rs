//! The kernel's state as the hosts see it (Stage 0 spike A): every form,
//! the text system, and the queues between the host's callbacks and the
//! program. Nothing here runs program code: input is routed into the
//! forms' models and what the program must hear about is queued as
//! [`HostEvent`]s, which `ui::step` dispatches after the pump returns.

use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::time::Duration;

use crate::form::{Event, Form, Key, Mods};
use crate::text::TextSystem;

pub type FormId = usize;

/// What the program hears about (queued in callbacks, dispatched by `step`).
#[derive(Debug, Clone, PartialEq)]
pub enum HostEvent {
    /// OnClick / OnChange from the kernel's input routing.
    Kernel(FormId, Event),
    /// The window's close box (or a script's `close`).
    Close(FormId),
    /// The client area changed (logical pixels).
    Resized(FormId, i64, i64),
    /// A menu item picked (muda, through the event loop proxy).
    Menu(String),
    /// A future's waker (an async dialog finished).
    Wake,
}

/// What the program asks of the host; run where an `ActiveEventLoop` is
/// available (the shim's `new_events` / `about_to_wait`).
#[derive(Debug, Clone, PartialEq)]
pub enum HostCmd {
    Show(FormId),
    Hide(FormId),
    SetSize(FormId),
    /// An rfd async file dialog, created inside a pump (rfd needs
    /// NSApp running to make a sheet; between pumps it falls back to a
    /// blocking runModal).
    OpenFileDialog {
        id: u64,
        parent: FormId,
    },
    /// Spike: a blocking rfd dialog inside a winit callback.
    BlockingDialogInCallback,
}

pub enum Dialog {
    Requested,
    Pending(Pin<Box<dyn Future<Output = Option<PathBuf>>>>),
    Done(Option<PathBuf>),
}

pub struct KForm {
    pub name: String,
    pub form: Form,
    pub shown: bool,
    pub modal_result: i64,
    /// Needs repainting (and its accessibility tree updating).
    pub dirty: bool,
    /// Stacking order (higher = shown later = on top).
    pub z: u64,
}

/// Measurements of the host (spike evidence).
#[derive(Default, Debug)]
pub struct Stats {
    pub pumps: u64,
    /// Pumps that returned more than 40 ms after their timeout (AppKit's
    /// live-resize / menu-tracking loops hold the pump).
    pub held_pumps: Vec<Held>,
    /// Redraws done inside callbacks.
    pub redraws: u64,
    pub resizes: u64,
    /// Resized events that arrived while AppKit said the window was in
    /// live resize (`-[NSWindow inLiveResize]`).
    pub live_resizes: u64,
    /// Window frames drawn by vello_cpu and shown with softbuffer.
    pub cpu_frames: u64,
    pub cpu_frame_time: Duration,
}

#[derive(Debug, Clone)]
pub struct Held {
    pub over: Duration,
    pub resized: u32,
    pub redraws: u32,
    pub sizes: Option<((i64, i64), (i64, i64))>,
}

pub struct Kernel {
    pub forms: Vec<KForm>,
    pub text: TextSystem,
    pub events: Vec<HostEvent>,
    pub cmds: Vec<HostCmd>,
    /// The modal forms, innermost last: input to any other is dropped.
    pub modal: Vec<FormId>,
    pub dialogs: Vec<(u64, Dialog)>,
    pub next_dialog: u64,
    next_z: u64,
    /// Scripted runs drop the user's input.
    pub ignore_user: bool,
    pub stats: Stats,
    /// Inside a host callback (a pump): program code must not run.
    pub in_callback: bool,
}

impl Kernel {
    pub fn new() -> Self {
        Kernel {
            forms: Vec::new(),
            text: TextSystem::new(),
            events: Vec::new(),
            cmds: Vec::new(),
            modal: Vec::new(),
            dialogs: Vec::new(),
            next_dialog: 1,
            next_z: 1,
            ignore_user: false,
            stats: Stats::default(),
            in_callback: false,
        }
    }

    pub fn add(&mut self, name: &str, form: Form) -> FormId {
        self.forms.push(KForm { name: name.into(), form, shown: false, modal_result: 0, dirty: true, z: 0 });
        self.forms.len() - 1
    }

    pub fn find(&self, name: &str) -> Option<FormId> {
        self.forms.iter().position(|f| f.name.eq_ignore_ascii_case(name))
    }

    pub fn show(&mut self, f: FormId) {
        let k = &mut self.forms[f];
        k.shown = true;
        k.dirty = true;
        k.z = self.next_z;
        self.next_z += 1;
        self.cmds.push(HostCmd::Show(f));
    }

    pub fn hide(&mut self, f: FormId) {
        self.forms[f].shown = false;
        self.cmds.push(HostCmd::Hide(f));
    }

    /// Shown forms, bottom to top.
    pub fn stacking(&self) -> Vec<FormId> {
        let mut v: Vec<FormId> = (0..self.forms.len()).filter(|&f| self.forms[f].shown).collect();
        v.sort_by_key(|&f| self.forms[f].z);
        v
    }

    /// Input may reach `f` (no modal form, or `f` is the innermost).
    pub fn accepts_input(&self, f: FormId) -> bool {
        self.modal.last().is_none_or(|&m| m == f)
    }

    // ---- input, the same entry points for the OS and for scripts ----

    pub fn mouse_move(&mut self, f: FormId, x: f64, y: f64) {
        if !self.accepts_input(f) {
            return;
        }
        let k = &mut self.forms[f];
        let (redraw, ev) = k.form.mouse_move(x, y, &mut self.text);
        k.dirty |= redraw || !ev.is_empty();
        self.events.extend(ev.into_iter().map(|e| HostEvent::Kernel(f, e)));
    }

    pub fn mouse_down(&mut self, f: FormId, x: f64, y: f64, m: Mods) {
        if !self.accepts_input(f) {
            return;
        }
        let k = &mut self.forms[f];
        let ev = k.form.mouse_down(x, y, m, &mut self.text);
        k.dirty = true;
        self.events.extend(ev.into_iter().map(|e| HostEvent::Kernel(f, e)));
    }

    pub fn mouse_up(&mut self, f: FormId, x: f64, y: f64) {
        if !self.accepts_input(f) {
            return;
        }
        let k = &mut self.forms[f];
        let ev = k.form.mouse_up(x, y);
        k.dirty = true;
        self.events.extend(ev.into_iter().map(|e| HostEvent::Kernel(f, e)));
    }

    pub fn key(&mut self, f: FormId, key: Key, m: Mods, typed: Option<&str>, clip: &mut dyn crate::form::Clipboard) {
        if !self.accepts_input(f) {
            return;
        }
        let k = &mut self.forms[f];
        let ev = k.form.key(key, m, typed, &mut self.text, clip);
        k.dirty = true;
        self.events.extend(ev.into_iter().map(|e| HostEvent::Kernel(f, e)));
    }

    pub fn commit_text(&mut self, f: FormId, s: &str) {
        if !self.accepts_input(f) {
            return;
        }
        let k = &mut self.forms[f];
        let ev = k.form.commit_text(s, &mut self.text);
        k.dirty = true;
        self.events.extend(ev.into_iter().map(|e| HostEvent::Kernel(f, e)));
    }

    pub fn close_box(&mut self, f: FormId) {
        if self.accepts_input(f) {
            self.events.push(HostEvent::Close(f));
        }
    }

    /// The client area resized (a user's drag, or a script's): the
    /// kernel's layout follows at once (callback-safe, no program code);
    /// OnResize is queued.
    pub fn resized(&mut self, f: FormId, w: i64, h: i64) {
        let k = &mut self.forms[f];
        if (k.form.width, k.form.height) == (w, h) {
            return;
        }
        k.form.width = w;
        k.form.height = h;
        layout(&mut k.form);
        k.dirty = true;
        self.stats.resizes += 1;
        self.events.push(HostEvent::Resized(f, w, h));
    }
}

/// The spike's stand-in for Align / Anchors: components named `*Right`
/// keep their distance to the right edge, `lblLog*` stretch.
pub fn layout(form: &mut Form) {
    let w = form.width;
    for c in &mut form.components {
        if c.name.ends_with("Right") {
            c.left = (w - c.width - 8).max(0);
        } else if c.name.starts_with("lblLog") {
            c.width = (w - c.left - 8).max(10);
        }
    }
}

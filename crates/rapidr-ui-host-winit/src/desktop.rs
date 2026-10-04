//! Every form the kernel shows, as the hosts see them: each one's kernel
//! side ([`FormUi`]) and window ([`WindowSpec`]), the shared text system,
//! the modal forms, and the two queues between the program and the host —
//! [`HostCmd`]s the program asks for (run inside a pump, where winit has an
//! `ActiveEventLoop`) and [`HostEvent`]s the host's callbacks queue for the
//! program (dispatched by runtime-core after the pump returns).
//!
//! Input enters through the same methods whether the OS or a test script
//! sends it: [`Desktop::mouse_down`], [`Desktop::key_down`] … route it into
//! the form's kernel (hit test, capture, focus, the models) and queue what
//! the program hears about. Nothing here runs program code.

use std::collections::BTreeMap;

use rapidr_ui_kernel::{Clipboard, FormUi, KernelEvent, Mods, Store, TextSystem};
use rapidr_value::input::Button;
use rapidr_value::objects::a11y::Action;

/// What the program hears about, queued inside a pump.
#[derive(Clone, Debug, PartialEq)]
pub enum HostEvent {
    /// Form `form`'s kernel's event (a click, a key, the close box …).
    Kernel(String, KernelEvent),
    /// Something to look at (a future's waker: an async dialog finished).
    Wake,
}

/// What the program asks of the windows; run inside a pump.
#[derive(Clone, Debug, PartialEq)]
pub enum HostCmd {
    /// Its window made (the first time) and shown, on top, focused.
    Show(String),
    Hide(String),
    /// [`WindowSpec::title`] again.
    Title(String),
    /// [`WindowSpec::size`] again.
    Size(String),
    /// [`WindowSpec::position`] again.
    Position(String),
    /// [`WindowSpec::border`] again.
    Border(String),
    /// [`WindowSpec::icon`] again.
    Icon(String),
    Minimize(String),
    /// Pop-up menu `menu` shown by the system at (x, y) of form `form`'s
    /// window's inside (logical) — hosts with native menus
    /// ([`crate::Host::native_menus`]); its pick comes back as a
    /// `KernelEvent::MenuPick`.
    Popup { form: String, menu: String, x: i64, y: i64 },
    // (the dialogs and platform lane's)
    /// An Open / Save dialog (`dialogs.rs`), on form `form`'s window when
    /// given (a sheet on macOS); its answer through `Host::file_dialog(id)`.
    FileDialog { id: u64, form: Option<String>, req: crate::dialogs::FileRequest },
    /// Form `id`'s window gone for good (a kernel-drawn dialog closed).
    Forget(String),
}

/// A window's picture (RGBA, straight).
#[derive(Clone, Debug, PartialEq)]
pub struct Icon {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// What a form's window looks like, as the program set it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WindowSpec {
    pub title: String,
    /// Its inside (client area and an in-window menu bar), logical pixels.
    pub size: (i64, i64),
    /// Its frame's top left on the screen (Left, Top), logical pixels.
    pub position: Option<(i64, i64)>,
    /// A frame and title bar (BorderStyle <> bsNone).
    pub border: bool,
    pub icon: Option<Icon>,
    /// Resizing and the title bar's buttons (BorderStyle, BorderIcons).
    pub frame: crate::platform::Frame,
}

pub struct Form {
    pub ui: FormUi,
    pub spec: WindowSpec,
    pub shown: bool,
    /// Stacking order (higher: shown later, on top).
    pub z: u64,
    /// The scale its window shows at (device pixels per logical pixel).
    pub scale: f64,
}

/// Where input comes from: the OS (dropped under a test, or for a window
/// under a modal one) or a test's script (`RAPIDR_TEST_EVENTS`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    User,
    Script,
}

pub struct Desktop {
    pub forms: BTreeMap<String, Form>,
    pub text: TextSystem,
    pub events: Vec<HostEvent>,
    pub cmds: Vec<HostCmd>,
    /// The forms shown modally, innermost last: input to any other window
    /// is dropped.
    pub modal: Vec<String>,
    /// A test drives the program: the user's input is dropped.
    pub ignore_user: bool,
    pub clipboard: Box<dyn Clipboard>,
    /// Screen.Cursor (0: each component's own; platform.rs).
    pub screen_cursor: i64,
    next_z: u64,
}

impl Desktop {
    pub fn new(clipboard: Box<dyn Clipboard>) -> Desktop {
        Desktop { forms: BTreeMap::new(), text: TextSystem::new(), events: Vec::new(), cmds: Vec::new(), modal: Vec::new(), ignore_user: false, clipboard, screen_cursor: 0, next_z: 1 }
    }

    /// Form `id`'s kernel side, made from the store the first time.
    pub fn ensure_form(&mut self, store: &dyn Store, id: &str, menu_in_window: bool, spec: WindowSpec) -> &mut Form {
        let key = id.to_lowercase();
        self.forms.entry(key.clone()).or_insert_with(|| Form { ui: FormUi::build(store, &key, menu_in_window), spec, shown: false, z: 0, scale: 1.0 })
    }

    pub fn form(&mut self, id: &str) -> Option<&mut Form> {
        self.forms.get_mut(&id.to_lowercase())
    }

    pub fn show(&mut self, id: &str) {
        let z = self.next_z;
        if let Some(f) = self.form(id) {
            f.shown = true;
            f.z = z;
            f.ui.dirty = true;
            self.next_z += 1;
            self.cmds.push(HostCmd::Show(id.to_lowercase()));
        }
    }

    pub fn hide(&mut self, id: &str) {
        if let Some(f) = self.form(id) {
            f.shown = false;
            self.cmds.push(HostCmd::Hide(id.to_lowercase()));
        }
    }

    /// Form `id` and its window gone for good (a kernel-drawn dialog
    /// closed).
    pub fn forget(&mut self, id: &str) {
        let key = id.to_lowercase();
        if self.forms.remove(&key).is_some() {
            self.cmds.push(HostCmd::Forget(key));
        }
    }

    /// The shown forms, bottom to top.
    pub fn stacking(&self) -> Vec<String> {
        let mut v: Vec<(&String, u64)> = self.forms.iter().filter(|(_, f)| f.shown).map(|(k, f)| (k, f.z)).collect();
        v.sort_by_key(|(_, z)| *z);
        v.into_iter().map(|(k, _)| k.clone()).collect()
    }

    /// Input may reach form `id` (no modal form, or it's the innermost).
    pub fn accepts_input(&self, id: &str) -> bool {
        self.modal.last().is_none_or(|m| m.eq_ignore_ascii_case(id))
    }

    fn admits(&self, id: &str, src: Source) -> bool {
        match src {
            Source::Script => true,
            Source::User => !self.ignore_user && self.accepts_input(id),
        }
    }

    /// Form `id`'s kernel events into the queue.
    fn collect(&mut self, id: &str) {
        let key = id.to_lowercase();
        if let Some(f) = self.forms.get_mut(&key) {
            for e in f.ui.take_events() {
                self.events.push(HostEvent::Kernel(key.clone(), e));
            }
        }
    }

    /// Runs `g` on form `id`'s kernel (with the text system), then queues
    /// its events.
    fn route(&mut self, id: &str, src: Source, g: impl FnOnce(&mut FormUi, &mut TextSystem, &mut dyn Clipboard)) {
        if !self.admits(id, src) {
            return;
        }
        let Desktop { forms, text, clipboard, .. } = self;
        let Some(f) = forms.get_mut(&id.to_lowercase()) else { return };
        g(&mut f.ui, text, clipboard.as_mut());
        self.collect(id);
    }

    // ---- input: the same entry points for the OS and for scripts ----

    /// A mouse button pressed at `(x, y)` of the window's inside (logical).
    pub fn mouse_down(&mut self, store: &dyn Store, id: &str, (x, y): (f64, f64), button: Button, mods: Mods, src: Source) {
        self.route(id, src, |f, ts, _| f.mouse_down(store, ts, x, y, button, mods));
    }

    pub fn mouse_move(&mut self, store: &dyn Store, id: &str, x: f64, y: f64, mods: Mods, src: Source) {
        self.route(id, src, |f, ts, _| f.mouse_move(store, ts, x, y, mods));
    }

    pub fn mouse_up(&mut self, store: &dyn Store, id: &str, (x, y): (f64, f64), button: Button, mods: Mods, src: Source) {
        self.route(id, src, |f, ts, _| f.mouse_up(store, ts, x, y, button, mods));
    }

    pub fn mouse_leave(&mut self, store: &dyn Store, id: &str, src: Source) {
        self.route(id, src, |f, ts, _| f.mouse_leave(store, ts));
    }

    pub fn key_down(&mut self, store: &dyn Store, id: &str, vk: i64, text: &str, mods: Mods, src: Source) {
        self.route(id, src, |f, ts, clip| f.key_down(store, ts, vk, text, mods, clip));
    }

    pub fn key_up(&mut self, id: &str, vk: i64, mods: Mods, src: Source) {
        self.route(id, src, |f, _, _| f.key_up(vk, mods));
    }

    pub fn ime_preedit(&mut self, store: &dyn Store, id: &str, text: &str, cursor: Option<(usize, usize)>, src: Source) {
        self.route(id, src, |f, ts, _| f.ime_preedit(store, ts, text, cursor));
    }

    pub fn ime_commit(&mut self, store: &dyn Store, id: &str, text: &str, src: Source) {
        self.route(id, src, |f, ts, _| f.ime_commit(store, ts, text));
    }

    /// The window's close box.
    pub fn close_box(&mut self, id: &str, src: Source) {
        self.route(id, src, |f, _, _| f.close_box());
    }

    /// A test hook's component step (`__item_i`, `__node_i` …; the lists
    /// lane's `FormUi::test_action`): whether component `comp` took it.
    pub fn test_action(&mut self, store: &dyn Store, id: &str, comp: &str, action: &str) -> bool {
        let mut done = false;
        self.route(id, Source::Script, |f, ts, _| {
            f.sync(store);
            done = f.test_action(store, ts, comp, action);
        });
        done
    }

    /// A screen reader's request (as the user's input would be).
    pub fn access_action(&mut self, store: &dyn Store, id: &str, target: u64, action: Action, value: Option<rapidr_ui_kernel::AccessValue>) -> bool {
        let mut done = false;
        self.route(id, Source::User, |f, ts, _| done = f.access_action(store, ts, target, action, value));
        done
    }

    /// The window's inside resized to `w` × `h` logical pixels (an
    /// in-window menu bar included), by the user or the system: the kernel
    /// lays out, OnResize after the pump.
    pub fn resized(&mut self, id: &str, w: i64, h: i64) {
        if let Some(f) = self.form(id) {
            f.spec.size = (w, h);
            let menu = f.ui.menu_offset;
            f.ui.resized(w, (h - menu).max(0));
        }
        self.collect(id);
    }

    /// The window's frame moved to (left, top) on the screen.
    pub fn moved(&mut self, id: &str, left: i64, top: i64) {
        if let Some(f) = self.form(id) {
            if f.spec.position != Some((left, top)) {
                f.spec.position = Some((left, top));
                f.ui.moved(left, top);
            }
        }
        self.collect(id);
    }

    pub fn scale_changed(&mut self, id: &str, scale: f64) {
        if let Some(f) = self.form(id) {
            if (f.scale - scale).abs() > f64::EPSILON {
                f.scale = scale;
                f.ui.scale_changed(scale);
            }
        }
        self.collect(id);
    }
}

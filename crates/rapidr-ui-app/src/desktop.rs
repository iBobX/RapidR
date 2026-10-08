//! Every form a kernel host shows, as the hosts see them (moved from
//! `rapidr-ui-host-winit` in Stage W3, docs/web-host-plan.md: the desktop's
//! winit and headless hosts and the web host share it): each one's kernel
//! side ([`FormUi`]) and window ([`WindowSpec`]), the shared text system,
//! the modal forms, and the two queues between the program and the host —
//! [`HostCmd`]s the program asks for (run in the host's turn: inside a pump
//! on the desktop, where winit has an `ActiveEventLoop`; at once on the
//! page) and [`HostEvent`]s the host's callbacks queue for the program
//! (dispatched by the runtime after the host's turn).
//!
//! Input enters through the same methods whether the user or a test script
//! sends it: [`Desktop::mouse_down`], [`Desktop::key_down`] … route it into
//! the form's kernel (hit test, capture, focus, the models) and queue what
//! the program hears about. Nothing here runs program code.
//!
//! Also here, for every host: the program's window commands into the forms
//! ([`Desktop::apply`], [`Desktop::sync_forms`], [`window_spec`]), a GUI
//! test's input through the kernel's routing ([`script_input`]) and the
//! accessibility trees it writes ([`Desktop::access_trees`]).

use std::collections::BTreeMap;

use rapidr_ui_kernel::tick::Instant;
use rapidr_ui_kernel::{Clipboard, FormUi, KernelEvent, Mods, Store, TextSystem};
use rapidr_value::input::{Button, Cursor, Mouse};
use rapidr_value::objects::a11y::Action;
use rapidr_value::Value;

use crate::windows::{ScriptInput, WindowOp};
use crate::{forms, Program};

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
    /// (the desktop host's `native_menus`); its pick comes back as a
    /// `KernelEvent::MenuPick`.
    Popup { form: String, menu: String, x: i64, y: i64 },
    // (the dialogs and platform lane's)
    /// An Open / Save dialog (`dialogs.rs`), on form `form`'s window when
    /// given (a sheet on macOS); its answer through `Host::file_dialog(id)`.
    FileDialog { id: u64, form: Option<String>, req: FileRequest },
    /// Form `id`'s window gone for good (a kernel-drawn dialog closed).
    Forget(String),
    // (the input lane's)
    /// A QSTATUSBAR's size grip dragged: form `form`'s window's inside to
    /// `w` × `h` (logical) — winit asks the system (`request_inner_size`),
    /// the headless host resizes at once; either way the resize comes back
    /// as the user's ([`Desktop::resized`]: OnResize, Width / Height).
    Resize { form: String, w: i64, h: i64 },
    // (the dialogs and WindowState lane's)
    /// [`WindowSpec::state`] again: the window maximized, minimized or
    /// restored (QFORM.WindowState).
    State(String),
    // (the DirectX lane's)
    /// Form `f`'s window covers the screen, without a frame (a QDXSCREEN's
    /// FullScreen); the system's new size comes back as a resize. The
    /// headless host leaves it to runtime-core.
    Fullscreen(String),
    /// [`WindowSpec::modified`] again (Form.Modified).
    Modified(String),
}

pub use crate::windows::Icon;

/// A form's window frame (BorderStyle, BorderIcons): the kernel's, which
/// the web host and the form designer draw (`rapidr_ui_kernel::frame`).
pub use rapidr_ui_kernel::frame::{frame_of, Frame, BI_DEFAULT};

/// What an Open / Save dialog shows (`crate::file_dialog`'s request, in the
/// host's terms).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FileRequest {
    pub save: bool,
    pub multi: bool,
    pub title: Option<String>,
    /// (description, patterns: `*.txt`, `*.*`) in the program's order.
    pub filters: Vec<(String, Vec<String>)>,
    /// The one shown first (FilterIndex, from 0).
    pub filter_index: usize,
    pub dir: Option<String>,
    pub file_name: Option<String>,
    /// A folder is chosen (QOPENDIALOG.PickFolder, RapidR's).
    pub folder: bool,
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
    pub frame: Frame,
    /// (the WindowState lane's) wsNormal 0, wsMinimized 1, wsMaximized 2:
    /// how the program asked it to show.
    pub state: i64,
    /// Form.Modified: changes not saved (macOS' close button shows its
    /// dot; the web asks before the page is left).
    pub modified: bool,
}

pub struct Form {
    pub ui: FormUi,
    pub spec: WindowSpec,
    pub shown: bool,
    /// Stacking order (higher: shown later, on top).
    pub z: u64,
    /// The scale its window shows at (device pixels per logical pixel).
    pub scale: f64,
    /// (the WindowState lane's) What its window is now (wsNormal …): the
    /// program's state once the host made it so, or the user's own
    /// maximize / minimize ([`Desktop::window_state`]).
    pub state: i64,
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
    /// Carets blink (off under a test, so captures are steady).
    pub blinks: bool,
    /// The scale a form has before its window is made (the host's
    /// `default_scale`: the screen a new window opens on). What the program
    /// draws before it shows — a QCANVAS's, a QDIGDISPLAY's digits — is
    /// kept at it (`forms::after_show`), so the window's first frame and a
    /// test's capture are sharp, not an enlarged 1× picture.
    pub default_scale: f64,
    next_z: u64,
}

impl Desktop {
    pub fn new(clipboard: Box<dyn Clipboard>) -> Desktop {
        Desktop { forms: BTreeMap::new(), text: TextSystem::new(), events: Vec::new(), cmds: Vec::new(), modal: Vec::new(), ignore_user: false, clipboard, screen_cursor: 0, blinks: true, default_scale: 1.0, next_z: 1 }
    }

    /// Form `id`'s kernel side, made from the store the first time.
    pub fn ensure_form(&mut self, store: &dyn Store, id: &str, menu_in_window: bool, spec: WindowSpec) -> &mut Form {
        let key = id.to_lowercase();
        let (blinks, scale) = (self.blinks, self.default_scale);
        self.forms.entry(key.clone()).or_insert_with(|| {
            // (`RAPIDR_TEST_NOFOCUS`: shown with nothing focused, as the
            // form designer shows a form — the WYSIWYG comparison)
            let mut ui = if crate::testhooks::no_focus() { FormUi::build_unfocused(store, &key, menu_in_window) } else { FormUi::build(store, &key, menu_in_window) };
            ui.blinks = blinks;
            ui.scale = scale;
            Form { ui, spec, shown: false, z: 0, scale, state: 0 }
        })
    }

    /// When the shown forms' next deadline is (a caret's blink, a held
    /// scroll bar's repeat, a component's tick): the step loop pumps no
    /// longer than that.
    pub fn next_wake(&self) -> Option<Instant> {
        self.forms.values().filter(|f| f.shown).filter_map(|f| f.ui.next_wake()).min()
    }

    /// Runs the shown forms' deadlines due at `now` (what they change is
    /// drawn again; their events queued).
    pub fn tick(&mut self, store: &dyn Store, now: Instant) {
        let due: Vec<String> = self.forms.iter().filter(|(_, f)| f.shown && f.ui.next_wake().is_some_and(|at| at <= now)).map(|(k, _)| k.clone()).collect();
        for id in due {
            let Desktop { forms, text, .. } = self;
            if let Some(f) = forms.get_mut(&id) {
                f.ui.tick(store, text, now);
            }
            self.collect(&id);
        }
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

    /// Input may reach form `id`: no modal form, it's the innermost, or it
    /// was shown after the innermost went up — Windows' modal loop disables
    /// the windows there are when it starts; a window shown from it (a
    /// RapidQ program's `Form2.Show` under `Form.ShowModal`) works.
    pub fn accepts_input(&self, id: &str) -> bool {
        let Some(m) = self.modal.last() else { return true };
        if m.eq_ignore_ascii_case(id) {
            return true;
        }
        let z = |name: &str| self.forms.get(&name.to_lowercase()).filter(|f| f.shown).map(|f| f.z);
        matches!((z(id), z(m)), (Some(a), Some(b)) if a > b)
    }

    fn admits(&self, id: &str, src: Source) -> bool {
        match src {
            Source::Script => true,
            Source::User => !self.ignore_user && self.accepts_input(id),
        }
    }

    /// Form `id`'s kernel events into the queue (a size grip's request is
    /// the host's: a window command).
    fn collect(&mut self, id: &str) {
        let key = id.to_lowercase();
        if let Some(f) = self.forms.get_mut(&key) {
            for e in f.ui.take_events() {
                // (the input lane's)
                if let KernelEvent::Container(rapidr_ui_kernel::components::form::Container::Resize { form, w, h }) = e {
                    self.cmds.push(HostCmd::Resize { form, w, h });
                    continue;
                }
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
        self.route(id, src, |f, ts, clip| {
            f.mouse_up(store, ts, x, y, button, mods);
            // (an edit's context menu picked: Cut, Copy, Paste …)
            f.edit_commands(store, ts, clip);
        });
    }

    /// The mouse wheel turned `dx`, `dy` notches (positive: right, down)
    /// with the mouse at (x, y) of the window's inside.
    pub fn mouse_wheel(&mut self, store: &dyn Store, id: &str, (x, y): (f64, f64), (dx, dy): (f64, f64), mods: Mods, src: Source) {
        self.route(id, src, |f, ts, _| f.mouse_wheel(store, ts, (x, y), (dx, dy), mods));
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

    /// (the WindowState lane's) Form `id`'s window is now `state` (the
    /// system's word: the user maximized, restored or minimized it, or the
    /// host made the program's state so): kept, and when it changed the
    /// program hears it as the form's WindowState set.
    pub fn window_state(&mut self, id: &str, state: i64) {
        let key = id.to_lowercase();
        let Some(f) = self.forms.get_mut(&key) else { return };
        if f.state != state {
            f.state = state;
            f.spec.state = state;
            self.events.push(HostEvent::Kernel(key.clone(), KernelEvent::Set { id: key, prop: "windowstate".into(), value: state }));
        }
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

// ------------------------------------------- the program's windows, shared --
//
// (Stage W3) What runtime-core's `sync_desk` and test hooks did with the
// desktop's `Desktop`, for every host: the program's window commands into
// the forms, and a GUI test's input through the kernel's routing.

impl Desktop {
    /// A window command of the program's ([`WindowOp`], `take_ops`) into
    /// its form: the form's kernel side made the first time it shows
    /// (`spec`: its window as the program set it, [`window_spec`]), the
    /// window's description changed and the host's command queued.
    pub fn apply(&mut self, store: &dyn Store, op: WindowOp, menu_in_window: bool, spec: impl FnOnce(&str) -> WindowSpec) {
        match op {
            WindowOp::Show(f) => {
                if !self.forms.contains_key(&f) {
                    let s = spec(&f);
                    self.ensure_form(store, &f, menu_in_window, s);
                }
                self.show(&f);
            }
            WindowOp::Hide(f) => self.hide(&f),
            WindowOp::Title(f, t) => {
                if let Some(w) = self.form(&f) {
                    w.spec.title = t;
                    self.cmds.push(HostCmd::Title(f));
                }
            }
            WindowOp::Size(f, size) => {
                if let Some(w) = self.form(&f) {
                    if w.spec.size != size {
                        w.spec.size = size;
                        self.cmds.push(HostCmd::Size(f));
                    }
                }
            }
            WindowOp::Position(f, p) => {
                if let Some(w) = self.form(&f) {
                    w.spec.position = Some(p);
                    self.cmds.push(HostCmd::Position(f));
                }
            }
            WindowOp::Border(f, b) => {
                if let Some(w) = self.form(&f) {
                    w.spec.border = b;
                    self.cmds.push(HostCmd::Border(f));
                }
            }
            WindowOp::Icon(f, i) => {
                if let Some(w) = self.form(&f) {
                    w.spec.icon = i;
                    self.cmds.push(HostCmd::Icon(f));
                }
            }
            WindowOp::Minimize(f) => self.cmds.push(HostCmd::Minimize(f)),
            WindowOp::Modified(f, m) => {
                if let Some(w) = self.form(&f) {
                    if w.spec.modified != m {
                        w.spec.modified = m;
                        self.cmds.push(HostCmd::Modified(f));
                    }
                }
            }
            WindowOp::Popup(form, menu, x, y) => self.cmds.push(HostCmd::Popup { form, menu, x, y }),
            WindowOp::State(f, state) => {
                if let Some(w) = self.form(&f) {
                    w.spec.state = state;
                    self.cmds.push(HostCmd::State(f));
                }
            }
            // (the DirectX lane's)
            WindowOp::Fullscreen(f) => self.cmds.push(HostCmd::Fullscreen(f)),
            // (L-PANELS: SetFocus)
            WindowOp::Focus(f, comp) => {
                if let Some(w) = self.forms.get_mut(&f) {
                    w.ui.sync(store);
                    w.ui.focus_id(store, &comp);
                    w.ui.dirty = true;
                }
            }
        }
    }

    /// After the window commands: the modal list (`modal`, innermost last),
    /// and what changed (`take_notify`: paint, the tree) into every form's
    /// kernel side — its tree rebuilt, or its geometry and models read
    /// again and drawn.
    pub fn sync_forms(&mut self, store: &dyn Store, modal: Vec<String>, (paint, structure): (bool, bool)) {
        self.modal = modal;
        for (id, f) in self.forms.iter_mut() {
            f.ui.modal = self.modal.contains(id);
            if structure {
                f.ui.rebuild(store);
            } else if paint {
                f.ui.sync(store);
                f.ui.dirty = true;
            }
        }
    }

    /// Form `id`'s accessibility tree as `RAPIDR_TEST_A11Y` writes it (what
    /// AccessKit gets on the desktop, the web host's mirror shows).
    pub fn access_json(&mut self, store: &dyn Store, id: &str) -> Option<String> {
        let Desktop { forms, text, .. } = self;
        let w = forms.get_mut(&id.to_lowercase())?;
        Some(w.ui.access_tree(store, text).to_json())
    }
}

/// Form `name`'s frame: its BorderStyle and BorderIcons (all three when
/// never set).
pub fn frame<P: Program>(p: P, name: &str) -> Frame {
    let icons = match p.get(name, "bordericons") {
        Value::Null => BI_DEFAULT,
        v => v.to_i64(),
    };
    frame_of(p.get(name, "borderstyle").to_i64(), icons)
}

/// Form `name`'s window as the program set it (made when it first shows).
pub fn window_spec<P: Program>(p: P, name: &str) -> WindowSpec {
    WindowSpec {
        title: p.get(name, "caption").to_string_val(),
        size: forms::form_window_size(p, name),
        position: Some((p.get(name, "left").to_i64(), p.get(name, "top").to_i64())),
        border: p.get(name, "borderstyle").to_i64() != 0,
        icon: forms::icon_of(p, name),
        frame: frame(p, name),
        // (the WindowState lane's)
        state: rapidr_value::window_state::of(p.get(name, "windowstate").to_i64()),
        modified: p.get(name, "modified").to_bool(),
    }
}

/// The form a component is on, and where it is in that window's inside
/// (logical; the in-window menu bar included), from the kernel's tree.
pub fn place_of<P: Program>(p: P, desk: &mut Desktop, store: &dyn Store, comp: &str) -> Option<(String, (i64, i64))> {
    let form = p.form_of(comp)?;
    let comp = comp.to_lowercase();
    let f = desk.forms.get_mut(&form)?;
    f.ui.sync(store);
    if comp == form {
        return Some((form.clone(), (0, f.ui.menu_offset)));
    }
    let n = f.ui.node(&comp)?;
    Some((form.clone(), (n.abs.0, n.abs.1)))
}

/// A GUI test's input (`RAPIDR_TEST_EVENTS`, `RAPIDR_TEST_RESIZE`: the
/// script's, `crate::script`) through the kernel's routing, as the user's
/// would be. A held pump ([`ScriptInput::Hold`]) is the host's own: false.
pub fn script_input<P: Program>(p: P, desk: &mut Desktop, store: &dyn Store, input: ScriptInput) -> bool {
    match input {
        ScriptInput::Key { comp, vk, state } => script_key(p, desk, store, &comp, vk, state),
        ScriptInput::Mouse { comp, kind, x, y } => script_mouse(p, desk, store, &comp, kind, x, y),
        ScriptInput::DblClick { comp, x, y } => {
            // (press, release, press, release: the second press within
            // Windows' double-click time and distance of the first)
            script_mouse(p, desk, store, &comp, Mouse::Down, x, y);
            script_mouse(p, desk, store, &comp, Mouse::Up, x, y);
            if let Some((form, (ox, oy))) = place_of(p, desk, store, &comp) {
                let at = ((ox + x) as f64 + 0.5, (oy + y) as f64 + 0.5);
                desk.mouse_down(store, &form, at, Button::Left, Mods::NONE, Source::Script);
                desk.mouse_up(store, &form, at, Button::Left, Mods::NONE, Source::Script);
            }
        }
        ScriptInput::Step { form, comp, step } => {
            desk.test_action(store, &form, &comp, &step);
        }
        ScriptInput::Resize { w, h } => script_resize(p, desk, store, w, h),
        ScriptInput::Hold(_) => return false,
    }
    true
}

/// `comp.__key_N`: the component focused, the key pressed and released.
fn script_key<P: Program>(p: P, desk: &mut Desktop, store: &dyn Store, comp: &str, vk: i64, state: i64) {
    let Some(form) = p.form_of(comp) else { return };
    let comp = comp.to_lowercase();
    let mods = Mods { shift: state & 256 != 0, ctrl: state & 16 != 0, alt: state & 1 != 0, ..Mods::NONE };
    let text = if mods.ctrl || mods.alt { String::new() } else { rapidr_value::input::text_of_vk(vk) };
    if let Some(f) = desk.forms.get_mut(&form) {
        f.ui.sync(store);
        f.ui.focus_id(store, &comp);
    }
    desk.key_down(store, &form, vk, &text, mods, Source::Script);
    desk.key_up(&form, vk, mods, Source::Script);
}

/// `comp.__mousedown_x_y` …: the mouse at (x, y) in the component (hit
/// test, capture). A press is a single click, however soon after another.
fn script_mouse<P: Program>(p: P, desk: &mut Desktop, store: &dyn Store, comp: &str, kind: Mouse, x: i64, y: i64) {
    let Some((form, (ox, oy))) = place_of(p, desk, store, comp) else { return };
    let (x, y) = ((ox + x) as f64 + 0.5, (oy + y) as f64 + 0.5);
    match kind {
        Mouse::Down => {
            // (the input lane's)
            if let Some(f) = desk.form(&form) {
                f.ui.forget_clicks();
            }
            desk.mouse_down(store, &form, (x, y), Button::Left, Mods::NONE, Source::Script)
        }
        Mouse::Move => desk.mouse_move(store, &form, x, y, Mods::NONE, Source::Script),
        Mouse::Up => desk.mouse_up(store, &form, (x, y), Button::Left, Mods::NONE, Source::Script),
    }
}

/// `RAPIDR_TEST_RESIZE=w,h`: the frontmost form resized (Width, Height) as
/// a user dragging its border would.
fn script_resize<P: Program>(p: P, desk: &mut Desktop, store: &dyn Store, w: i64, h: i64) {
    let Some(form) = desk.stacking().last().cloned() else { return };
    let (fw, fh) = rapidr_value::layout::form_frame(p.get(&form, "borderstyle").to_i64());
    let (iw, ih) = (w - fw, h - fh);
    if let Some(f) = desk.forms.get_mut(&form) {
        f.ui.sync(store);
    }
    desk.resized(&form, iw, ih);
    crate::windows::push_op(WindowOp::Size(form, (iw, ih)));
}

/// The pointer over form `form` at `(x, y)` of its inside: Screen.Cursor
/// (`desk.screen_cursor`), else the Cursor of the component under the
/// mouse (the form's over its client area); crDefault: the component's
/// own (an enabled edit's I-beam, a splitter's resize arrows, a header's
/// or list view header's section edge), else the arrow.
pub fn cursor_at(desk: &Desktop, store: &dyn Store, form: &str, (x, y): (f64, f64)) -> Cursor {
    if desk.screen_cursor != 0 {
        return Cursor::of(desk.screen_cursor);
    }
    const CR_HSPLIT: i64 = -14;
    const CR_VSPLIT: i64 = -15;
    let Some(f) = desk.forms.get(form) else { return Cursor::Default };
    let node = f.ui.hover.and_then(|i| f.ui.nodes.get(i));
    // (the input lane's: a status bar's size grip is the window's sizing
    // corner — Windows' HTBOTTOMRIGHT arrow, whatever the bar's Cursor)
    let grip = rapidr_value::layout::STATUS_GRIP;
    if let Some(n) = node.filter(|n| n.type_name == "RSTATUSBAR" && x >= (n.abs.0 + n.abs.2 - grip) as f64 && y >= (n.abs.1 + n.abs.3 - grip) as f64) {
        if rapidr_ui_kernel::components::statusbar::has_grip(store, &n.id) {
            return Cursor::SizeNWSE;
        }
    }
    let id = node.map_or(f.ui.form.as_str(), |n| n.id.as_str());
    let code = rapidr_ui_kernel::store::int(store, id, "cursor", 0);
    // (a QSPLITTER's crHSplit / crVSplit, its Cursor at creation: the
    // splitter's direction decides, as Delphi's TSplitter swaps them when
    // its Align changes)
    let split = node.is_some_and(|n| n.type_name == "RSPLITTER") && matches!(code, CR_HSPLIT | CR_VSPLIT);
    if code != 0 && !split {
        return Cursor::of(code);
    }
    let Some(n) = node else { return Cursor::Default };
    let (lx, ly) = ((x as i64) - n.abs.0, (y as i64) - n.abs.1);
    match n.type_name.as_str() {
        "REDIT" | "RMEMO" | "RRICHEDIT" if n.enabled => Cursor::IBeam,
        "RSPLITTER" if rapidr_ui_kernel::components::splitter::vertical(store, &n.id) => Cursor::SizeNS,
        "RSPLITTER" => Cursor::SizeWE,
        "RHEADER" if rapidr_value::objects::with_header(&n.id, |h| h.on_grip(lx)).unwrap_or(false) => Cursor::SizeWE,
        "RLISTVIEW" if rapidr_value::objects::with_listview(&n.id, |l| l.on_grip(lx, ly)).unwrap_or(false) => Cursor::SizeWE,
        // (a QFORMMDI child's sizing border: every edge and corner, as Windows')
        "RMDICHILD" => match rapidr_ui_kernel::components::mdi::edges_at(store, &n.id, n.abs.2, n.abs.3, lx, ly).map(|e| e.pointer()) {
            Some("we") => Cursor::SizeWE,
            Some("ns") => Cursor::SizeNS,
            Some("nwse") => Cursor::SizeNWSE,
            Some(_) => Cursor::SizeNESW,
            None => Cursor::Default,
        },
        // (the dock manager's splitters, the document area's between groups
        // and between a document's two views)
        "RDOCKMANAGER" | "RDOCKDOCS" => rapidr_ui_kernel::components::dock::splitter_cursor(store, &n.type_name, &n.id, lx, ly).map_or(Cursor::Default, |row| if row { Cursor::SizeWE } else { Cursor::SizeNS }),
        // (I4: the designer's handles, the form's edges, the placing tool)
        "RDESIGNSURFACE" => {
            use rapidr_value::objects::design::Pointer;
            let p = rapidr_value::objects::with_design(&n.id, |d| {
                let (ox, oy) = d.client_origin();
                d.pointer_at(lx - ox, ly - oy)
            });
            match p.unwrap_or(Pointer::Default) {
                Pointer::Default => Cursor::Default,
                Pointer::Move => Cursor::Move,
                Pointer::SizeWE => Cursor::SizeWE,
                Pointer::SizeNS => Cursor::SizeNS,
                Pointer::SizeNWSE => Cursor::SizeNWSE,
                Pointer::SizeNESW => Cursor::SizeNESW,
                Pointer::Cross => Cursor::Cross,
            }
        }
        _ => Cursor::Default,
    }
}

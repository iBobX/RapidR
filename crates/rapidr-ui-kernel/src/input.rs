//! Input routing: the host passes the window's mouse, keys and input
//! methods in (logical pixels of the client area, the in-window menu
//! bar's included; Windows' virtual-key codes); the kernel finds the
//! component (hit test, the mouse captured by what it was pressed on,
//! hover), moves its model, keeps the focus, and queues what the program
//! hears about as [`KernelEvent`]s — in the order RapidQ's runtimes fire
//! them:
//!
//! - a key: OnKeyDown along the focus chain, then the model (Tab moves the
//!   focus, Enter clicks the Default button, Escape the Cancel one, a
//!   component takes its keys), then OnKeyPress for a key that types;
//! - a press: the model (a track bar's page, a tab picked; the focus), then
//!   OnMouseDown; a release: OnClick for a button let go on, then
//!   OnMouseUp.
//!
//! Nothing here calls program code: runtime-core dispatches the events
//! after the host's pump returns (`KeyPreview` through
//! `rapidr_value::input::key_targets`, ModalResult through its OnClick).

use rapidr_value::input::{self, Button, Mouse};
use rapidr_value::objects::ops::Rect;

use crate::components::{ComponentKind, Cx, Ime, KeyIn, MouseIn, MouseKind, MouseOut, State};
use crate::store::Store;
use crate::text::TextSystem;
use crate::tree::FormUi;

/// Modifier keys. `command` is the shortcut key (Cmd on macOS, Ctrl
/// elsewhere); `word` moves by words (Option on macOS, Ctrl elsewhere): the
/// host sets them for its platform.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mods {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub command: bool,
    pub word: bool,
}

impl Mods {
    pub const NONE: Mods = Mods { shift: false, ctrl: false, alt: false, command: false, word: false };
    pub const SHIFT: Mods = Mods { shift: true, ctrl: false, alt: false, command: false, word: false };

    /// RapidQ's Shift argument (ssShift 256, ssCtrl 16, ssAlt 1).
    pub fn shift_state(&self) -> i64 {
        input::shift_state(self.shift, self.ctrl, self.alt)
    }
}

/// The clipboard as edits use it (Copy, Cut, Paste). runtime-core passes
/// one over `globals::Platform` (which keeps `RAPIDR_TEST_CLIPBOARD`).
pub trait Clipboard {
    fn get_text(&mut self) -> Option<String>;
    fn set_text(&mut self, text: &str);
}

/// A clipboard in memory (tests, scripted input).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MemClipboard(pub Option<String>);

impl Clipboard for MemClipboard {
    fn get_text(&mut self) -> Option<String> {
        self.0.clone()
    }
    fn set_text(&mut self, text: &str) {
        self.0 = Some(text.to_string());
    }
}

/// What the program hears about. Ids are lowercase.
#[derive(Clone, Debug, PartialEq)]
pub enum KernelEvent {
    /// OnClick: a button clicked (by the mouse, Space / Enter, Enter for
    /// the Default button, Escape for the Cancel one, its mnemonic, a
    /// screen reader). runtime-core applies its ModalResult / Kind after.
    Click(String),
    /// OnChange: a track bar's position, a tab control's tab, an edit's
    /// text changed by the user.
    Change(String),
    /// OnKeyDown along `chain` (the focused component up to its form);
    /// `text` is what the key types (INKEY$ reads it).
    KeyDown { chain: Vec<String>, vk: i64, shift: i64, text: String },
    /// OnKeyPress: the character typed (`input::press_code`).
    KeyPress { chain: Vec<String>, key: i64 },
    KeyUp { chain: Vec<String>, vk: i64, shift: i64 },
    /// OnMouseDown / OnMouseMove / OnMouseUp, at (x, y) in `id` (a
    /// component, or the form: its client area).
    Mouse { id: String, kind: Mouse, button: Button, x: i64, y: i64, shift: i64 },
    /// The window's close box.
    Close(String),
    /// The client area resized (logical pixels).
    Resized(String, i64, i64),
    /// The window moved (its Left, Top on the screen).
    Moved(String, i64, i64),
    /// Its screen's scale changed (OnScaleChanged).
    ScaleChanged(String, f64),
    /// A menu item picked (by id).
    MenuPick(String),
}

impl FormUi {
    /// Runs `f` with node `i`'s kind and context.
    pub(crate) fn with_cx<R>(&mut self, store: &dyn Store, ts: &mut TextSystem, i: usize, f: impl FnOnce(&'static dyn ComponentKind, &mut Cx) -> R) -> Option<R> {
        let kind = self.nodes[i].kind?;
        let state = State {
            focused: self.focus == Some(i),
            hover: self.hover == Some(i),
            pressed: self.pressed == Some(i) && self.hover == Some(i),
            held: self.pressed == Some(i),
            enabled: self.nodes[i].enabled,
            caret_on: self.caret_on,
            default_frame: false,
        };
        let scale = self.scale;
        let node = &mut self.nodes[i];
        let font = store.font(&node.id);
        let mut cx = Cx { store, text: ts, id: &node.id, rect: node.abs, font, state, ui: &mut node.ui, events: &mut self.events, scale };
        Some(f(kind, &mut cx))
    }

    fn mouse_to(&mut self, store: &dyn Store, ts: &mut TextSystem, i: usize, m: MouseIn) -> MouseOut {
        self.with_cx(store, ts, i, |k, cx| {
            let (x0, y0, _, _) = cx.rect;
            k.mouse(cx, &MouseIn { x: m.x - x0 as f64, y: m.y - y0 as f64, ..m })
        })
        .unwrap_or_default()
    }

    /// OnMouseDown / Move / Up for `target` (a component, or the form).
    fn mouse_event(&mut self, target: Option<usize>, kind: Mouse, button: Button, x: f64, y: f64, mods: Mods) {
        let (id, ox, oy) = match target {
            Some(i) => (self.nodes[i].id.clone(), self.nodes[i].abs.0, self.nodes[i].abs.1),
            None => (self.form.clone(), 0, self.menu_offset),
        };
        let (x, y) = ((x - ox as f64).floor() as i64, (y - oy as f64).floor() as i64);
        self.events.push(KernelEvent::Mouse { id, kind, button, x, y, shift: mods.shift_state() });
    }

    /// A component the mouse can reach (shown, enabled: a disabled
    /// control gets no mouse, as on Windows).
    fn live(&self, i: Option<usize>) -> bool {
        i.is_none_or(|i| self.nodes[i].shown && self.nodes[i].enabled)
    }

    /// A mouse button pressed at (x, y) of the client area.
    pub fn mouse_down(&mut self, store: &dyn Store, ts: &mut TextSystem, x: f64, y: f64, button: Button, mods: Mods) {
        self.dirty = true;
        if y < self.menu_offset as f64 {
            // (the in-window menu bar: the menu lane's)
            return;
        }
        let target = self.hit(x, y);
        if !self.live(target) {
            return;
        }
        self.capture = Some(target);
        if let Some(i) = target {
            if button == Button::Left {
                let out = self.mouse_to(store, ts, i, MouseIn { kind: MouseKind::Down, x, y, button, mods, inside: true, captured: true });
                if out.press {
                    self.pressed = Some(i);
                }
                if out.focus.unwrap_or(true) && self.can_focus(store, i) {
                    self.set_focus(Some(i));
                }
            }
        }
        self.caret_on = true;
        self.mouse_event(target, Mouse::Down, button, x, y, mods);
    }

    /// The mouse moved to (x, y) of the client area.
    pub fn mouse_move(&mut self, store: &dyn Store, ts: &mut TextSystem, x: f64, y: f64, mods: Mods) {
        let hit = self.hit(x, y);
        if hit != self.hover {
            if let Some(old) = self.hover {
                self.mouse_to(store, ts, old, MouseIn { kind: MouseKind::Leave, x, y, button: Button::Left, mods, inside: false, captured: false });
            }
            self.hover = hit;
            self.dirty = true;
        }
        let target = match self.capture {
            Some(c) => c,
            None => hit,
        };
        if !self.live(target) {
            return;
        }
        if let Some(i) = target {
            let captured = self.capture.is_some();
            self.mouse_to(store, ts, i, MouseIn { kind: MouseKind::Move, x, y, button: Button::Left, mods, inside: hit == Some(i), captured });
            if captured {
                self.dirty = true;
            }
        }
        if self.capture.is_some() || y >= self.menu_offset as f64 {
            self.mouse_event(target, Mouse::Move, Button::Left, x, y, mods);
        }
    }

    /// A mouse button released at (x, y) of the client area.
    pub fn mouse_up(&mut self, store: &dyn Store, ts: &mut TextSystem, x: f64, y: f64, button: Button, mods: Mods) {
        self.dirty = true;
        let hit = self.hit(x, y);
        let target = match self.capture.take() {
            Some(c) => c,
            None if y < self.menu_offset as f64 => return,
            None => hit,
        };
        if !self.live(target) {
            self.pressed = None;
            return;
        }
        if let (Some(i), Button::Left) = (target, button) {
            self.mouse_to(store, ts, i, MouseIn { kind: MouseKind::Up, x, y, button, mods, inside: hit == Some(i), captured: true });
        }
        if button == Button::Left {
            self.pressed = None;
        }
        self.mouse_event(target, Mouse::Up, button, x, y, mods);
    }

    /// The mouse left the window.
    pub fn mouse_leave(&mut self, store: &dyn Store, ts: &mut TextSystem) {
        if let Some(old) = self.hover.take() {
            self.mouse_to(store, ts, old, MouseIn { kind: MouseKind::Leave, x: -1.0, y: -1.0, button: Button::Left, mods: Mods::NONE, inside: false, captured: false });
            self.dirty = true;
        }
    }

    /// The components a key goes to: the focused one up to the form.
    pub fn key_chain(&self) -> Vec<String> {
        let mut chain: Vec<String> = self.focus.map(|f| self.ancestry(f).into_iter().map(|i| self.nodes[i].id.clone()).collect()).unwrap_or_default();
        chain.push(self.form.clone());
        chain
    }

    /// A key pressed: `vk` its Windows virtual-key code, `text` what it
    /// types (empty: nothing).
    pub fn key_down(&mut self, store: &dyn Store, ts: &mut TextSystem, vk: i64, text: &str, mods: Mods, clip: &mut dyn Clipboard) {
        self.dirty = true;
        self.caret_on = true;
        let chain = self.key_chain();
        self.events.push(KernelEvent::KeyDown { chain: chain.clone(), vk, shift: mods.shift_state(), text: text.to_string() });
        let shortcut = mods.command || mods.ctrl;
        let mut handled = false;
        if vk == 9 && !shortcut && !mods.alt {
            self.move_focus(store, mods.shift);
            handled = true;
        } else if mods.alt && !mods.ctrl && (65..=90).contains(&vk) {
            handled = self.mnemonic(store, ts, (vk as u8 + 32) as char);
        } else if let Some(f) = self.focus {
            handled = self.with_cx(store, ts, f, |k, cx| k.key(cx, &KeyIn { vk, text, mods }, clip)).unwrap_or(false);
        }
        if !handled && !shortcut && !mods.alt {
            let button = match vk {
                13 => self.default_button(store),
                27 => self.cancel_button(store),
                _ => None,
            };
            if let Some(b) = button {
                self.events.push(KernelEvent::Click(self.nodes[b].id.clone()));
            }
        }
        // (Alt + a letter is WM_SYSCHAR, a shortcut types nothing: no OnKeyPress)
        if !mods.alt && !mods.command {
            if let Some(key) = input::press_code(vk, text) {
                self.events.push(KernelEvent::KeyPress { chain, key });
            }
        }
    }

    pub fn key_up(&mut self, vk: i64, mods: Mods) {
        let chain = self.key_chain();
        self.events.push(KernelEvent::KeyUp { chain, vk, shift: mods.shift_state() });
    }

    /// Alt + `letter`: the button whose caption marks it clicked, or the
    /// component after the label that marks it focused.
    fn mnemonic(&mut self, store: &dyn Store, _ts: &mut TextSystem, letter: char) -> bool {
        let found = (0..self.nodes.len()).find(|&i| {
            let n = &self.nodes[i];
            n.shown && n.enabled && n.kind.and_then(|k| k.mnemonic(store, &n.id)) == Some(letter)
        });
        let Some(i) = found else { return false };
        if self.nodes[i].kind.is_some_and(|k| k.mnemonic_clicks()) {
            if self.can_focus(store, i) {
                self.set_focus(Some(i));
            }
            self.events.push(KernelEvent::Click(self.nodes[i].id.clone()));
        } else if let Some(next) = self.next_in_order(store, i) {
            self.set_focus(Some(next));
        }
        true
    }

    /// An input method's composition changed (`text` empty: it ended
    /// without text). Not the program's text until committed.
    pub fn ime_preedit(&mut self, store: &dyn Store, ts: &mut TextSystem, text: &str, cursor: Option<(usize, usize)>) {
        let Some(f) = self.focus else { return };
        self.dirty = true;
        self.with_cx(store, ts, f, |k, cx| k.ime(cx, &Ime::Preedit(text.to_string(), cursor)));
    }

    /// Text an input method committed (a dead key's letter, a CJK word)
    /// into the focused component: OnChange, then an OnKeyPress per
    /// character (Windows' WM_CHARs).
    pub fn ime_commit(&mut self, store: &dyn Store, ts: &mut TextSystem, text: &str) {
        let Some(f) = self.focus else { return };
        self.dirty = true;
        self.caret_on = true;
        let took = self.with_cx(store, ts, f, |k, cx| k.ime(cx, &Ime::Commit(text.to_string()))).unwrap_or(false);
        if took {
            let chain = self.key_chain();
            for c in text.chars() {
                if let Some(key) = input::press_code(0, &c.to_string()) {
                    self.events.push(KernelEvent::KeyPress { chain: chain.clone(), key });
                }
            }
        }
    }

    /// Whether the focused component takes text from input methods (the
    /// host allows IME only then, so other components' keys aren't
    /// swallowed).
    pub fn wants_ime(&self) -> bool {
        self.focus.is_some_and(|f| self.nodes[f].type_name == "REDIT")
    }

    /// Where the input method's window goes: the focused editor's caret
    /// (logical, in the client area).
    pub fn ime_area(&mut self, store: &dyn Store, ts: &mut TextSystem) -> Option<Rect> {
        let f = self.focus?;
        self.with_cx(store, ts, f, |k, cx| k.ime_area(cx)).flatten()
    }

    /// The window's close box clicked.
    pub fn close_box(&mut self) {
        self.events.push(KernelEvent::Close(self.form.clone()));
    }

    /// The window's client area resized (by the user or the system):
    /// OnResize after the pump.
    pub fn resized(&mut self, width: i64, height: i64) {
        if self.client != (width, height) {
            self.client = (width, height);
            self.dirty = true;
            self.events.push(KernelEvent::Resized(self.form.clone(), width, height));
        }
    }

    pub fn moved(&mut self, left: i64, top: i64) {
        self.events.push(KernelEvent::Moved(self.form.clone(), left, top));
    }

    pub fn scale_changed(&mut self, scale: f64) {
        self.scale = scale;
        self.dirty = true;
        self.events.push(KernelEvent::ScaleChanged(self.form.clone(), scale));
    }
}

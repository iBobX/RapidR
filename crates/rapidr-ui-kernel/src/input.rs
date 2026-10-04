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
//!   OnMouseUp; a double click's second press: OnDblClick, then
//!   OnMouseDown, and its release only OnMouseUp (the VCL's order).
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
    /// OnDblClick: a double click's second press on a panel, a label, a
    /// group box, a scroll box, an image or the form's open area (before
    /// that press's OnMouseDown, as the VCL's WM_LBUTTONDBLCLK).
    DblClick(String),
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
    /// A menu item picked (by id): its OnClick.
    MenuPick(String),
    /// A container's action runtime-core carries out (components scrolled,
    /// a splitter dragged, an MDI child's frame used).
    Container(crate::components::form::Container),
    /// The user changed a plain property of `id` (a check box's Checked, a
    /// cool button's Down): runtime-core stores it before the events after
    /// it (its OnClick).
    Set { id: String, prop: String, value: i64 },
    /// What the user did to a list, tree, grid, list view or header that
    /// the program answers (the lists lane's; `components::list`).
    List(String, crate::components::list::ListAction),
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
        let (scale, system_corner) = (self.scale, self.system_corner);
        let node = &mut self.nodes[i];
        let font = store.font(&node.id);
        let mut cx = Cx { store, text: ts, id: &node.id, rect: node.abs, font, state, ui: &mut node.ui, events: &mut self.events, scale, system_corner };
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
        // (an open menu, the in-window menu bar: components/menubar.rs)
        if self.menu_mouse_down(store, x, y) {
            return;
        }
        // (an open drop-down list, over everything but menus)
        if crate::components::combo::popup_mouse_down(self, store, x, y) {
            return;
        }
        // (scroll bars take the mouse next, over the components)
        if button == Button::Left && crate::components::scrollbox::bars_down(self, store, x, y) {
            return;
        }
        // (the input lane's: a status bar's size grip, as Windows' sizing border)
        if button == Button::Left && crate::components::statusbar::grip_down(self, store, x, y) {
            return;
        }
        let target = self.hit(x, y);
        if !self.live(target) {
            return;
        }
        self.capture = Some(target);
        let clicks = self.count_click(target, x, y, button);
        // (the input lane's: Alt isn't pressed alone any more — menubar.rs)
        self.menus.alt_alone = false;
        if let Some(i) = target {
            if button == Button::Left {
                let out = self.mouse_to(store, ts, i, MouseIn { kind: MouseKind::Down, x, y, button, mods, inside: true, captured: true, clicks });
                if out.press {
                    self.pressed = Some(i);
                }
                if out.focus.unwrap_or(true) && self.can_focus(store, i) {
                    self.set_focus(Some(i));
                }
            }
        } else if button == Button::Left && clicks >= 2 && clicks.is_multiple_of(2) {
            // (the input lane's: the form's open area double-clicked)
            self.events.push(KernelEvent::DblClick(self.form.clone()));
        }
        self.reset_caret();
        self.mouse_event(target, Mouse::Down, button, x, y, mods);
        crate::components::combo::after_input(self, store);
    }

    /// A press's click count (Windows: within the double-click time and
    /// distance of the last press, on the same component, the same button
    /// — the left one).
    fn count_click(&mut self, target: Option<usize>, x: f64, y: f64, button: Button) -> u8 {
        let now = crate::tick::now();
        let n = match self.last_click {
            Some((at, t, px, py, n))
                if button == Button::Left
                    && t == target
                    && now.saturating_duration_since(at) <= crate::tick::DOUBLE_CLICK
                    && (x - px).abs() <= crate::tick::DOUBLE_CLICK_DISTANCE
                    && (y - py).abs() <= crate::tick::DOUBLE_CLICK_DISTANCE =>
            {
                n.saturating_add(1)
            }
            _ => 1,
        };
        self.last_click = (button == Button::Left).then_some((now, target, x, y, n));
        n
    }

    /// (the input lane's) The click count of the left press on `target`
    /// now being let go (1 if there's none).
    fn press_clicks(&self, target: Option<usize>) -> u8 {
        self.last_click.filter(|c| c.1 == target).map_or(1, |c| c.4)
    }

    /// (the input lane's) The next press starts a click count over (a
    /// test's script: its presses are single clicks unless it double-clicks).
    pub fn forget_clicks(&mut self) {
        self.last_click = None;
    }

    /// The mouse moved to (x, y) of the client area.
    pub fn mouse_move(&mut self, store: &dyn Store, ts: &mut TextSystem, x: f64, y: f64, mods: Mods) {
        if self.menu_mouse_move(store, x, y) {
            return;
        }
        crate::components::combo::popup_mouse_move(self, store, x, y);
        if crate::components::scrollbox::bars_drag(self, store, x, y) {
            return;
        }
        // (the input lane's)
        if crate::components::statusbar::grip_drag(self, x, y) {
            return;
        }
        let hit = self.hit(x, y);
        if hit != self.hover {
            if let Some(old) = self.hover {
                self.mouse_to(store, ts, old, MouseIn { kind: MouseKind::Leave, x, y, button: Button::Left, mods, inside: false, captured: false, clicks: 0 });
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
            self.mouse_to(store, ts, i, MouseIn { kind: MouseKind::Move, x, y, button: Button::Left, mods, inside: hit == Some(i), captured, clicks: 0 });
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
        if self.menu_mouse_up(store, x, y) {
            return;
        }
        if button == Button::Left && crate::components::scrollbox::bars_up(self, store) {
            return;
        }
        // (the input lane's)
        if button == Button::Left && crate::components::statusbar::grip_up(self) {
            return;
        }
        let hit = self.hit(x, y);
        let captured = self.capture.take();
        let target = match captured {
            Some(c) => c,
            None if y < self.menu_offset as f64 => return,
            None => hit,
        };
        if !self.live(target) {
            self.pressed = None;
            return;
        }
        // (the input lane's: a release is its press's — a single click or a
        // double click's second)
        let clicks = if button == Button::Left { self.press_clicks(target) } else { 0 };
        if let (Some(i), Button::Left) = (target, button) {
            self.mouse_to(store, ts, i, MouseIn { kind: MouseKind::Up, x, y, button, mods, inside: hit == Some(i), captured: captured.is_some(), clicks });
        }
        // (the form's open area pressed and let go on: OnClick for a single
        // click)
        let on_form = hit.is_none() && x >= 0.0 && x < self.client.0 as f64 && y >= self.menu_offset as f64 && y < (self.menu_offset + self.client.1) as f64;
        if captured == Some(None) && button == Button::Left && on_form && clicks % 2 == 1 {
            self.events.push(KernelEvent::Click(self.form.clone()));
        }
        if button == Button::Left {
            self.pressed = None;
        }
        self.mouse_event(target, Mouse::Up, button, x, y, mods);
        // (a right click let go on an edit: its context menu, unless the
        // program gave it a PopupMenu — components/edit.rs)
        if let (Some(i), Button::Right) = (target, button) {
            if hit == Some(i) {
                self.open_edit_menu(store, ts, i, x, y);
            }
        }
    }

    /// The mouse left the window.
    pub fn mouse_leave(&mut self, store: &dyn Store, ts: &mut TextSystem) {
        if let Some(old) = self.hover.take() {
            self.mouse_to(store, ts, old, MouseIn { kind: MouseKind::Leave, x: -1.0, y: -1.0, button: Button::Left, mods: Mods::NONE, inside: false, captured: false, clicks: 0 });
            self.dirty = true;
        }
    }

    /// The mouse wheel turned `dx`, `dy` notches (positive: right, down;
    /// fractions from touchpads) with the mouse at (x, y): the component
    /// under the mouse scrolls (a memo, a list), else the one it's in,
    /// else the scroll box or form whose bars it's over — Windows 10's
    /// "scroll inactive windows" rule, not the focused control's.
    pub fn mouse_wheel(&mut self, store: &dyn Store, ts: &mut TextSystem, (x, y): (f64, f64), (dx, dy): (f64, f64), mods: Mods) {
        if self.menu_open() || crate::components::combo::popup_wheel(self, store, x, y, dy) {
            return;
        }
        let chain = self.hit(x, y).map(|i| self.ancestry(i)).unwrap_or_default();
        for i in chain {
            if !self.nodes[i].enabled {
                continue;
            }
            if self.with_cx(store, ts, i, |k, cx| k.wheel(cx, dx, dy, mods)).unwrap_or(false) {
                self.dirty = true;
                return;
            }
        }
        // (the containers' bars take whole notches)
        let (rx, ry) = (self.wheel_rest.0 + dx, self.wheel_rest.1 + dy);
        let (nx, ny) = (rx.trunc(), ry.trunc());
        self.wheel_rest = (rx - nx, ry - ny);
        let (notches, horizontal) = if ny != 0.0 { (ny as i64, mods.shift) } else { (nx as i64, true) };
        if notches != 0 {
            crate::components::scrollbox::bars_wheel(self, store, x, y, notches, horizontal);
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
        self.reset_caret();
        // (the input lane's: Alt pressed alone selects the menu bar when it's
        // let go — unless a menu or the bar's selection takes this Alt)
        self.menus.alt_alone = vk == 18 && !mods.ctrl && !mods.shift && !self.menus.keyboard && !self.menu_open();
        // (an open menu takes the keys; a main menu's ShortCut is picked
        // before the key reaches anything: components/menubar.rs)
        if self.menu_key(store, vk, mods) {
            // (an edit's context menu's pick, done now)
            self.edit_commands(store, ts, clip);
            return;
        }
        let chain = self.key_chain();
        self.events.push(KernelEvent::KeyDown { chain: chain.clone(), vk, shift: mods.shift_state(), text: text.to_string() });
        // (the input lane's: F10 selects the in-window menu bar, after its
        // OnKeyDown — components/menubar.rs)
        if vk == 121 && !mods.shift && !mods.ctrl && !mods.alt && self.select_bar(store) {
            return;
        }
        let shortcut = mods.command || mods.ctrl;
        let mut handled = false;
        // (a memo with WantTabs takes a plain Tab: components/memo.rs)
        if vk == 9 && !shortcut && !mods.alt && !crate::components::memo::takes_tab(self, store) {
            self.move_focus(store, mods.shift);
            handled = true;
        } else if mods.alt && !mods.ctrl && (65..=90).contains(&vk) {
            let letter = (vk as u8 + 32) as char;
            handled = self.mnemonic(store, ts, letter) || self.menu_mnemonic(store, letter);
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
        crate::components::combo::after_input(self, store);
        // (the menu key / Shift+F10 on an edit: its context menu)
        self.edit_commands(store, ts, clip);
    }

    pub fn key_up(&mut self, vk: i64, mods: Mods) {
        let chain = self.key_chain();
        self.events.push(KernelEvent::KeyUp { chain, vk, shift: mods.shift_state() });
        // (the input lane's: a lone Alt let go selects the in-window bar's
        // first item — not with the system's menu bar)
        if vk == 18 && std::mem::take(&mut self.menus.alt_alone) && self.menu_offset > 0 && !self.menus.system_bar && !self.menu_open() {
            self.menus.keyboard = true;
            self.menus.hot_top = Some(0);
            self.dirty = true;
        }
    }

    /// Alt + `letter`: the button whose caption marks it clicked, or the
    /// component after the label that marks it focused.
    fn mnemonic(&mut self, store: &dyn Store, ts: &mut TextSystem, letter: char) -> bool {
        let found = (0..self.nodes.len()).find(|&i| {
            let n = &self.nodes[i];
            n.shown && n.enabled && n.kind.and_then(|k| k.mnemonic(store, &n.id)) == Some(letter)
        });
        let Some(i) = found else { return false };
        if self.nodes[i].kind.is_some_and(|k| k.mnemonic_clicks()) {
            if self.can_focus(store, i) {
                self.set_focus(Some(i));
            }
            self.with_cx(store, ts, i, |k, cx| k.activate(cx));
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
        self.reset_caret();
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
    pub fn wants_ime(&self, store: &dyn Store) -> bool {
        self.focus.is_some_and(|f| self.nodes[f].kind.is_some_and(|k| k.wants_ime(store, &self.nodes[f].id)))
    }

    /// Where the input method's window goes: the focused editor's caret
    /// (logical, in the client area).
    pub fn ime_area(&mut self, store: &dyn Store, ts: &mut TextSystem) -> Option<Rect> {
        let f = self.focus?;
        self.with_cx(store, ts, f, |k, cx| k.ime_area(cx)).flatten()
    }

    /// A test hook's component step (`__item_i`, `__node_i`, `__toggle_i`,
    /// `__cell_c_r`, `__edit`, `__enter`, `__escape`): component `id`
    /// focused (when it can be), then its kind synthesizes the input (a
    /// click at the row, a key). Whether its kind understood it.
    pub fn test_action(&mut self, store: &dyn Store, ts: &mut TextSystem, id: &str, action: &str) -> bool {
        self.dirty = true;
        let Some(i) = self.index_of(id) else { return false };
        if self.can_focus(store, i) {
            self.set_focus(Some(i));
        }
        let done = self.with_cx(store, ts, i, |k, cx| k.test_action(cx, action)).unwrap_or(false);
        crate::components::combo::after_input(self, store);
        done
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

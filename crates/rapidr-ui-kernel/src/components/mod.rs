//! The components the kernel draws, as a table: a RapidR type name
//! (`RTRACKBAR`) → a [`ComponentKind`], which paints, takes the mouse and
//! keys, and describes itself to screen readers by calling its shared
//! model (`rapidr_value::objects`) — the model the web runtime draws too.
//! Adding a component is a file here and a line in [`KINDS`]; the runtime
//! glue never changes.

pub mod button;
pub mod edit;
pub mod label;
pub mod tabcontrol;
pub mod trackbar;

use rapidr_value::input::Button;
use rapidr_value::objects::a11y::{AccessNode, Action};
use rapidr_value::objects::font::Font;
use rapidr_value::objects::ops::Rect;

use crate::a11y::AccessValue;
use crate::input::{Clipboard, KernelEvent, Mods};
use crate::paint::Painter;
use crate::store::Store;
use crate::text::TextSystem;
use crate::tree::NodeUi;

/// The kinds, by type name (append-only: one line per component).
pub static KINDS: &[(&str, &dyn ComponentKind)] = &[
    ("RLABEL", &label::Label),
    ("RBUTTON", &button::PushButton),
    ("REDIT", &edit::Edit),
    ("RTRACKBAR", &trackbar::Trackbar),
    ("RTABCONTROL", &tabcontrol::Tabs),
];

/// The kind drawing components of `type_name` (`None`: the kernel only
/// places it, as a container, and routes its mouse events).
pub fn kind_of(type_name: &str) -> Option<&'static dyn ComponentKind> {
    KINDS.iter().find(|(t, _)| t.eq_ignore_ascii_case(type_name)).map(|(_, k)| *k)
}

/// How a component is shown now.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct State {
    pub focused: bool,
    /// The mouse is over it.
    pub hover: bool,
    /// Pressed with the mouse on it (a button drawn pushed).
    pub pressed: bool,
    /// Pressed, wherever the mouse is now.
    pub held: bool,
    /// Enabled, and its parents.
    pub enabled: bool,
    /// The caret's blink is on.
    pub caret_on: bool,
    /// The form's Default button while no other button has the focus (the
    /// dark frame).
    pub default_frame: bool,
}

/// What a component's methods get: the store, the text system, itself
/// (id, absolute rectangle, font, state, UI state) and the event queue.
pub struct Cx<'a> {
    pub store: &'a dyn Store,
    pub text: &'a mut TextSystem,
    /// Its id, lowercase.
    pub id: &'a str,
    /// Where it is in the window's client area.
    pub rect: Rect,
    pub font: Font,
    pub state: State,
    pub ui: &'a mut NodeUi,
    pub events: &'a mut Vec<KernelEvent>,
    /// The scale the form was last painted at (device pixels per logical
    /// pixel): editors lay out at it.
    pub scale: f64,
}

impl Cx<'_> {
    pub fn width(&self) -> i64 {
        self.rect.2
    }

    pub fn height(&self) -> i64 {
        self.rect.3
    }

    /// OnClick.
    pub fn click(&mut self) {
        self.events.push(KernelEvent::Click(self.id.to_string()));
    }

    /// OnChange.
    pub fn change(&mut self) {
        self.events.push(KernelEvent::Change(self.id.to_string()));
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseKind {
    Down,
    Move,
    Up,
    /// The mouse left it (HotTrack ends).
    Leave,
}

/// The mouse, in the component's own logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MouseIn {
    pub kind: MouseKind,
    pub x: f64,
    pub y: f64,
    pub button: Button,
    pub mods: Mods,
    /// Over it (a release elsewhere isn't a click).
    pub inside: bool,
    /// It has the mouse captured (pressed on it, not released yet).
    pub captured: bool,
}

/// What a component did with the mouse.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MouseOut {
    /// Draw it pushed while held (a button).
    pub press: bool,
    /// Take the focus or not (`None`: if it can).
    pub focus: Option<bool>,
}

/// A key on the focused component: Windows' virtual-key code, what it
/// typed (empty if nothing) and the modifiers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KeyIn<'a> {
    pub vk: i64,
    pub text: &'a str,
    pub mods: Mods,
}

/// An input method's composition (dead keys, CJK).
#[derive(Clone, Debug, PartialEq)]
pub enum Ime {
    /// The text being composed, its caret (byte range in it; `None`:
    /// hidden). Empty: the composition ended without text.
    Preedit(String, Option<(usize, usize)>),
    /// Text committed.
    Commit(String),
}

pub trait ComponentKind: Sync {
    /// Its RapidR type name.
    fn name(&self) -> &'static str;

    /// Takes the keyboard focus (when shown and enabled).
    fn focusable(&self, _store: &dyn Store, _id: &str) -> bool {
        true
    }

    /// Draws it, with (0, 0) its top left, clipped to it.
    fn paint(&self, cx: &mut Cx, p: &mut Painter);

    fn mouse(&self, _cx: &mut Cx, _m: &MouseIn) -> MouseOut {
        MouseOut::default()
    }

    /// A key while it has the focus: whether it was its (Tab, Enter and
    /// Escape go to the form when it isn't).
    fn key(&self, _cx: &mut Cx, _k: &KeyIn, _clip: &mut dyn Clipboard) -> bool {
        false
    }

    /// An input method's composition while it has the focus: whether it
    /// takes text.
    fn ime(&self, _cx: &mut Cx, _ime: &Ime) -> bool {
        false
    }

    /// Where the input method's candidate window goes (logical, absolute):
    /// its caret.
    fn ime_area(&self, _cx: &mut Cx) -> Option<Rect> {
        None
    }

    /// Its accessibility node (bounds absolute, children included).
    fn describe(&self, cx: &mut Cx) -> AccessNode;

    /// A screen reader's request on it (or on its part `part`, a tab):
    /// done as the user's input would be, so the same events fire.
    fn access(&self, _cx: &mut Cx, _action: Action, _part: Option<usize>, _value: Option<&AccessValue>) -> bool {
        false
    }

    /// Its caption's `&` letter (Alt + it activates it).
    fn mnemonic(&self, _store: &dyn Store, _id: &str) -> Option<char> {
        None
    }

    /// What Alt + its mnemonic does: `true` to click it (a button); a label
    /// focuses the next component instead.
    fn mnemonic_clicks(&self) -> bool {
        false
    }

    /// A test hook's component-specific step (`__item_i`, `__cell_c_r` …):
    /// whether it was understood. Stage 3's driver.
    fn test_action(&self, _cx: &mut Cx, _action: &str) -> bool {
        false
    }
}

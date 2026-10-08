//! The components the kernel draws, as a table: a RapidR type name
//! (`RTRACKBAR`) → a [`ComponentKind`], which paints, takes the mouse and
//! keys, and describes itself to screen readers by calling its shared
//! model (`rapidr_value::objects`) — the model the web runtime draws too.
//! Adding a component is a file here and a line in [`KINDS`]; the runtime
//! glue never changes.

pub mod button;
pub mod check;
pub mod coolbtn;
pub mod edit;
pub mod form;
pub mod groupbox;
pub mod label;
pub mod mdi;
pub mod memo;
pub mod richedit;
pub mod panel;
pub mod scrollbar;
pub mod scrollbox;
pub mod splitter;
pub mod statusbar;
pub mod menubar;
pub mod ovalbtn;
pub mod popupmenu;
pub mod progress;
pub mod radio;
pub mod tabcontrol;
pub mod trackbar;
pub mod updown;
// (the lists lane's)
pub mod combo;
pub mod dirtree;
pub mod filelist;
pub mod grid;
pub mod header;
pub mod list;
pub mod listview;
pub mod tree;
// (the surfaces lane's)
pub mod canvas;
pub mod image;
// (the DirectX lane's)
pub mod dxscreen;
// (RapidQ's QGLASSFRAME)
pub mod glass;
// (Stage 10: the IDE's)
pub mod codeedit;
pub mod design;
// (I1: RapidR Studio's docking)
pub mod dock;
// (I1 / L-PANELS: RapidR Studio's panels — rapidr_value::panels)
pub mod panels;
// (the data-science lane's: RPLOT on a form)
pub mod plot;

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
    ("RPANEL", &panel::Panel),
    ("RGROUPBOX", &groupbox::GroupBox),
    ("RSCROLLBOX", &scrollbox::ScrollBox),
    ("RSPLITTER", &splitter::Splitter),
    ("RSTATUSBAR", &statusbar::StatusBar),
    ("RMDICHILD", &mdi::ChildFrame),
    ("RCHECKBOX", &check::CheckBox),
    ("RRADIOBUTTON", &radio::RadioButton),
    ("RCOOLBTN", &coolbtn::CoolBtn),
    ("ROVALBTN", &ovalbtn::OvalBtn),
    ("RPROGRESSBAR", &progress::Progress),
    ("RPROGRESS", &progress::Progress),
    ("RUPDOWN", &updown::UpDown),
    ("RLISTBOX", &list::ListBox),
    ("RFILELISTBOX", &filelist::FileListBox),
    ("RCOMBOBOX", &combo::ComboBox),
    ("RLISTVIEW", &listview::ListViewBox),
    ("RSTRINGGRID", &grid::Grid),
    ("RHEADER", &header::HeaderBar),
    ("RTREEVIEW", &tree::Tree),
    ("RDIRTREE", &dirtree::DirTreeBox),
    ("RCANVAS", &canvas::Canvas),
    ("RIMAGE", &image::Image),
    // (the text lane's)
    ("RMEMO", &memo::Memo),
    ("RRICHEDIT", &richedit::RichEdit),
    // (Stage 10: the IDE's)
    ("RDESIGNSURFACE", &design::Design),
    ("RCODEEDITOR", &codeedit::CodeEditor),
    // (the dialogs lane's: what only a kernel-drawn dialog draws)
    ("RDLGPART", &crate::dialogs::Part),
    // (the DirectX lane's)
    ("RDXSCREEN", &dxscreen::DxScreen),
    // (RapidQ's custom components: QBevel.inc's, QDigDisplay.inc's)
    ("RBEVEL", &panel::Bevel),
    ("RDIGDISPLAY", &canvas::DigDisplay),
    ("RGLASSFRAME", &glass::GlassFrame),
    ("RSCROLLBAR", &scrollbar::ScrollBar),
    // (I1: RapidR Studio's docking — rapidr_value::dock)
    ("RDOCKMANAGER", &dock::DockManager),
    ("RDOCKGROUP", &dock::DockGroup),
    ("RDOCKDOCS", &dock::DockDocs),
    // (I1 / L-PANELS: RapidR Studio's panels — rapidr_value::panels)
    ("RPROPERTYINSPECTOR", &panels::inspector::Inspector),
    ("RTOOLBOX", &panels::toolbox::Toolbox),
    ("RPROJECTTREE", &panels::project_tree::ProjectTree),
    ("ROUTPUTCONSOLE", &panels::console::Console),
    ("RTOOLBAR", &panels::toolbar::ToolBar),
    ("RCOMMANDPALETTE", &panels::palette::Palette),
    // (the data-science lane's: a chart on a form — rapidr_value::datascience)
    ("RPLOT", &plot::Plot),
];

/// The kind drawing components of `type_name` (`None`: the kernel only
/// places it, as a container, and routes its mouse events).
pub fn kind_of(type_name: &str) -> Option<&'static dyn ComponentKind> {
    KINDS.iter().find(|(t, _)| t.eq_ignore_ascii_case(type_name)).map(|(_, k)| *k)
}

/// A component's node as the shared rules describe it (the web's too:
/// `rapidr_value::objects::a11y::describe`), placed where it is: it and the
/// parts whose place the model knows (tabs, header sections) moved to its
/// rectangle; the kind places the others.
pub fn shared_describe(cx: &Cx, type_name: &str) -> AccessNode {
    fn place(n: &mut AccessNode, dx: i64, dy: i64) {
        if n.bounds.2 > 0 || n.bounds.3 > 0 {
            n.bounds.0 += dx;
            n.bounds.1 += dy;
        }
        for c in &mut n.children {
            place(c, dx, dy);
        }
    }
    let mut n = rapidr_value::objects::a11y::describe(cx.id, type_name, &|p| cx.store.get(cx.id, p), (cx.width(), cx.height()), &cx.font);
    place(&mut n, cx.rect.0, cx.rect.1);
    n
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
    /// Its form's window corner is the system's
    /// ([`FormUi::system_corner`](crate::tree::FormUi::system_corner)).
    pub system_corner: bool,
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

    /// The user changed one of its plain properties (a check box's
    /// Checked, a cool button's Down): stored before the events after it.
    pub fn set(&mut self, id: &str, prop: &str, value: i64) {
        self.events.push(KernelEvent::Set { id: id.to_lowercase(), prop: prop.to_string(), value });
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
    /// A press's click count: 1, 2 for a double click, 3 … (Windows'
    /// double-click time and distance, on the same component); a
    /// release's is its press's.
    pub clicks: u8,
}

// (the input lane's)
impl MouseIn {
    /// Windows' double click (WM_LBUTTONDBLCLK): the second press of a
    /// pair — a third starts over as a single press, a fourth is a double
    /// again. On a release: let go of such a press.
    pub fn double(&self) -> bool {
        self.clicks >= 2 && self.clicks.is_multiple_of(2)
    }
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

    /// Where it may draw, in its own pixels: itself (what it paints is
    /// clipped to it), but an AutoSize label's text a little beyond — its
    /// size is RapidQ's measure in every theme, its text the theme's face.
    fn room(&self, _store: &dyn Store, _id: &str, _font: &Font, w: i64, h: i64) -> Rect {
        (0, 0, w, h)
    }

    /// A box of text or items (a text box, a list, a grid, a tree …): in
    /// RapidR's look, the focus is the theme's ring around the whole box,
    /// drawn over what it painted ([`Painter::items_focus`]). Others draw
    /// their own focus.
    fn field(&self) -> bool {
        false
    }

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

    /// Where its components go and what shows them, in its own pixels
    /// (Left / Top of a child count from this rectangle's corner; children
    /// are clipped to it): all of it, but a QSCROLLBOX's inside its edge.
    fn client_area(&self, _store: &dyn Store, _id: &str, w: i64, h: i64) -> Rect {
        (0, 0, w, h)
    }

    /// Draws over its components (a QSCROLLBOX's scroll bars), with (0, 0)
    /// its top left, unclipped (it clips what it draws).
    fn paint_over(&self, _store: &dyn Store, _id: &str, _w: i64, _h: i64, _p: &mut Painter) {}

    /// A test hook's component-specific step (`__item_i`, `__cell_c_r` …):
    /// whether it was understood. Stage 3's driver.
    fn test_action(&self, _cx: &mut Cx, _action: &str) -> bool {
        false
    }

    /// What a click on it does besides OnClick (Alt + its mnemonic, a
    /// screen reader's click): a check box's Checked turns over first.
    fn activate(&self, cx: &mut Cx) {
        cx.click();
    }

    /// The mouse wheel over it turned `dx`, `dy` notches (positive: right,
    /// down; fractions from touchpads): whether it scrolled (else the
    /// wheel goes to its parent, then the form's scroll bars).
    fn wheel(&self, _cx: &mut Cx, _dx: f64, _dy: f64, _mods: Mods) -> bool {
        false
    }

    /// Its deadline came (it set `NodeUi::wake`; tick.rs).
    fn tick(&self, _cx: &mut Cx) {}

    /// A tooltip of its own at (x, y) of it (tooltip.rs): a cut-short title,
    /// a button's name; `None`: its Hint, if ShowHint.
    fn tip_at(&self, _store: &dyn crate::store::Store, _id: &str, _x: f64, _y: f64) -> Option<String> {
        None
    }

    /// A right click let go on it (or the menu key): an edit's context
    /// menu — Undo, Cut, Copy, Paste, Delete, Select All, which of them
    /// apply now — that the kernel shows there; `None`: it has none.
    fn context_menu(&self, _cx: &mut Cx) -> Option<edit::MenuState> {
        None
    }

    /// Takes text from input methods while it has the focus (the host
    /// allows IME only then).
    fn wants_ime(&self, _store: &dyn Store, _id: &str) -> bool {
        false
    }
}

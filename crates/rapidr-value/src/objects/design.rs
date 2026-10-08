//! RDESIGNSURFACE, RapidR's form designer surface, on the designer model
//! (`crate::designer`, docs/ide-plan.md I4): the designed form's components
//! (a CREATE tree), the selection (Shift / Ctrl+click, a rubber band), and
//! what the mouse does to them — a drag moves the selection with the grid
//! and smart guides (edges, centres, baselines, margins, equal spacing;
//! Alt off), the primary selection's eight handles resize it, its anchor
//! pins toggle akLeft / akTop / akRight / akBottom, and the form's corner
//! drags the **resize preview**: the components placed where the running
//! program places them at that size (the runtimes' own layout, Align,
//! Anchors, Constraints, AutoSize). Moves and resizes are written when the
//! mouse is let go, one undo step each (Undo / Redo). Shared by every
//! runtime; what the program hears comes back as [`DesignEvent`]s:
//! OnSelect (Index), OnDblClick (Index), OnBgClick (X, Y), OnMove (Index,
//! X, Y, W, H).
//!
//! **What the surface shows** (lane L-DVIEW, WYSIWYG): the designed form as
//! the running program shows it — its frame and title bar, its main menu,
//! each component drawn by the UI kernel's own component from a
//! design-time store (a QLABEL is a label, a QSTRINGGRID a string grid:
//! `rapidr_ui_kernel::components::design`), on a backdrop a margin from
//! the surface's corner ([`DesignSurface::form_rect`]); under the form, a
//! tray strip of the non-visual components (QTIMER, QFILEDIALOG …:
//! [`DesignSurface::tray`]). Over that, the designer's own chrome
//! ([`DesignSurface::chrome_ops`]: the selection's frames and handles, the
//! anchor pins, the guides, the rubber band, the form's corner grip) and
//! the grid's dots on the form's face ([`DesignSurface::grid_ops`]), in the
//! theme's tokens. RapidQ has no designer: the chrome is RapidR's own.
//!
//! **Coordinates**: the API (GetCompX …, OnBgClick, OnMove) and the mouse
//! functions here speak the designed form's client coordinates — (0, 0) is
//! its client area's top left, below its title bar and menu bar
//! ([`DesignSurface::client_origin`] is where that is on the surface). The
//! form's size is its own (its Width / Height); a form made through the API
//! (AddComponent on the default form) fills the surface instead
//! ([`DesignSurface::set_size`]).

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use super::font::Font;
use super::ops::{Op, Place, Rect};
use super::trackbar::Shape;
use crate::designer::snap::{self, Edges, Target};
use crate::designer::{arrange, value, Command, Designer, FormDesign, Guide, GuideKind, Layout, NodeId, Side, Snapper, Subtree};
use crate::layout::{Rect as LRect, FORM_BORDER, FORM_CAPTION, MAIN_MENU_HEIGHT};
use crate::{v_int, v_str, Value};

pub mod inline;
pub mod menus;
mod outside;
mod taborder;

/// The grid moves and resizes snap to (and its dots are drawn on).
pub const GRID: i64 = 8;
/// How near a handle the mouse grabs it (either way).
const GRAB: i64 = 5;
/// The smallest a resize leaves a component.
const MIN_SIZE: i64 = 16;
/// A handle's size, drawn.
const HANDLE: i64 = 7;
/// The backdrop shown around a designed form that has a frame.
pub const MARGIN: i64 = 12;
/// The tray strip under the form: its height, an item's icon, the gap
/// above it.
pub const TRAY_H: i64 = 36;
pub const TRAY_ICON: i64 = 20;
pub const TRAY_GAP: i64 = 8;
/// The form's corner grip, outside its corner.
const CORNER: i64 = 10;
/// How far inside / outside the form's right and bottom edges a press
/// grabs them.
const EDGE_IN: i64 = 3;
const EDGE_OUT: i64 = 5;
/// The smallest a dragged form gets.
const MIN_FORM: i64 = 80;
/// Zoom's range and the steps Ctrl / ⌘ + and − go through (Xcode's and
/// Office's).
pub const MIN_ZOOM: f64 = 0.25;
pub const MAX_ZOOM: f64 = 4.0;
pub const ZOOM_STEPS: [f64; 15] = [0.25, 0.33, 0.5, 0.67, 0.75, 0.9, 1.0, 1.1, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0, 4.0];

/// A designed component.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DesignComp {
    /// Its node in the designer model.
    pub id: NodeId,
    pub name: String,
    /// Its type as the program wrote it (`QBUTTON`, `RBUTTON` …).
    pub type_name: String,
    /// RapidR's name for its type (`RBUTTON`).
    pub canonical: String,
    /// Where it is on the form's client area (its parents' places added;
    /// during a drag, where the drag puts it).
    pub x: i64,
    pub y: i64,
    pub w: i64,
    pub h: i64,
    /// It shows on the form (else it is in the tray strip, or a menu).
    pub visual: bool,
    /// Its properties (lowercase names), as the CREATE block sets them.
    pub props: BTreeMap<String, String>,
}

impl DesignComp {
    pub fn prop(&self, name: &str) -> Option<&str> {
        self.props.get(name).map(String::as_str)
    }

    pub fn bounds(&self) -> Rect {
        (self.x, self.y, self.w, self.h)
    }
}

/// A non-visual component in the tray strip under the form: its index (as
/// the API counts components), name and type as written, and its place
/// (client coordinates).
#[derive(Clone, Debug, PartialEq)]
pub struct TrayItem {
    pub index: usize,
    pub name: String,
    pub type_name: String,
    pub rect: Rect,
}

/// What a press grabbed: the selection to move, one of the primary
/// selection's eight handles, an anchor pin, the designed form's corner
/// (the resize preview), or the background (a rubber band).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Grip {
    #[default]
    Move,
    Left,
    Top,
    Right,
    Bottom,
    TopLeft,
    TopRight,
    BottomLeft,
    Corner,
    /// The form's bottom-right corner, right edge, bottom edge: the form
    /// resized (its components previewed where the running program puts
    /// them; the new size written when the mouse is let go).
    FormCorner,
    FormRight,
    FormBottom,
    /// A rubber band from the background.
    Band,
    /// A tray item pressed (it doesn't move).
    Tray,
    /// The placing tool (PlaceType): a new component's rectangle drawn.
    Place,
    /// A menu item held in the menu editor (dragged: moved elsewhere).
    MenuItem,
}

impl Grip {
    /// The form's size is being dragged.
    fn form(self) -> bool {
        matches!(self, Grip::FormCorner | Grip::FormRight | Grip::FormBottom)
    }

    /// The selection is being moved or resized.
    fn moves(self) -> bool {
        matches!(self, Grip::Move | Grip::Left | Grip::Top | Grip::Right | Grip::Bottom | Grip::TopLeft | Grip::TopRight | Grip::BottomLeft | Grip::Corner)
    }
}

/// The mouse pointer the surface wants at a place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pointer {
    Default,
    /// Over a component that moves (the four arrows).
    Move,
    SizeWE,
    SizeNS,
    SizeNWSE,
    SizeNESW,
    /// The placing tool armed (a cross).
    Cross,
}

impl Grip {
    fn edges(self) -> Edges {
        let (left, top, right, bottom) = match self {
            Grip::Left => (true, false, false, false),
            Grip::Top => (false, true, false, false),
            Grip::Right => (false, false, true, false),
            Grip::Bottom => (false, false, false, true),
            Grip::TopLeft => (true, true, false, false),
            Grip::TopRight => (false, true, true, false),
            Grip::BottomLeft => (true, false, false, true),
            Grip::Corner => (false, false, true, true),
            _ => (false, false, false, false),
        };
        Edges { left, top, right, bottom }
    }
}

/// What the program hears.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DesignEvent {
    /// OnSelect (Index): a component selected (pressed, Tab, added, a
    /// rubber band's first); -1: nothing (the form itself).
    Select(i64),
    /// OnDblClick (Index): a component pressed twice (or Enter on it).
    DblClick(usize),
    /// OnBgClick (X, Y): the background pressed (nothing selected now).
    BgClick(i64, i64),
    /// OnMove (Index, X, Y, W, H): the selected component dragged or
    /// resized (each step of the drag).
    Move { index: usize, x: i64, y: i64, w: i64, h: i64 },
    /// OnSourceEdit (StartLine, StartCol, EndLine, EndCol, Text): the
    /// program's source changed by the designer — the text from (StartLine,
    /// StartCol) to (EndLine, EndCol) replaced by `Text` (lines from 0,
    /// columns in characters from 0); a change's edits come in order, each
    /// in the text as the ones before left it.
    SourceEdit(SourceEdit),
    /// OnChange: the designed form changed (after its OnSourceEdits).
    Change,
    /// OnSourceStep (Continues): a change's OnSourceEdits follow, as one
    /// undo step of the file's history — or, `Continues` True, as part of
    /// the step before (a caption typed right after adding the component).
    Step(bool),
    /// OnUndo (Redo): with SharedUndo, Undo (0) or Redo (1) asked on the
    /// designer (Ctrl+Z, its methods): the program undoes in the code
    /// editor that holds the file's history, then sets Source again.
    Undo(bool),
}

impl DesignEvent {
    /// The event's name (lowercase, as handlers are stored).
    pub fn event(&self) -> &'static str {
        match self {
            DesignEvent::Select(_) => "onselect",
            DesignEvent::DblClick(_) => "ondblclick",
            DesignEvent::BgClick(..) => "onbgclick",
            DesignEvent::Move { .. } => "onmove",
            DesignEvent::SourceEdit(_) => "onsourceedit",
            DesignEvent::Change => "onchange",
            DesignEvent::Step(_) => "onsourcestep",
            DesignEvent::Undo(_) => "onundo",
        }
    }

    /// Its arguments.
    pub fn args(&self) -> Vec<Value> {
        match self {
            DesignEvent::Select(i) => vec![v_int(*i)],
            DesignEvent::DblClick(i) => vec![v_int(*i as i64)],
            DesignEvent::BgClick(x, y) => vec![v_int(*x), v_int(*y)],
            DesignEvent::Move { index, x, y, w, h } => vec![v_int(*index as i64), v_int(*x), v_int(*y), v_int(*w), v_int(*h)],
            DesignEvent::SourceEdit(e) => vec![v_int(e.start.0 as i64), v_int(e.start.1 as i64), v_int(e.end.0 as i64), v_int(e.end.1 as i64), v_str(&e.text)],
            DesignEvent::Change => Vec::new(),
            DesignEvent::Step(c) => vec![v_int(i64::from(*c))],
            DesignEvent::Undo(r) => vec![v_int(i64::from(*r))],
        }
    }
}

// ------------------------------------------------- the program's source --

/// An edit of the program's source: from `start` to `end` (line, column:
/// both from 0, columns in characters) replaced by `text`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceEdit {
    pub start: (usize, usize),
    pub end: (usize, usize),
    pub text: String,
}

impl SourceEdit {
    /// The edit replacing bytes `start..end` of `text` by `insert`.
    pub fn of(text: &str, start: usize, end: usize, insert: &str) -> SourceEdit {
        let at = |b: usize| {
            let b = b.min(text.len());
            let before = &text[..b];
            let line = before.matches('\n').count();
            let col = before[before.rfind('\n').map_or(0, |i| i + 1)..].chars().count();
            (line, col)
        };
        SourceEdit { start: at(start), end: at(end), text: insert.to_string() }
    }
}

/// A hook told whenever a surface's form or selection changed (an
/// inspector reads it again): the surface's id.
pub type ChangeHook = fn(surface: &str);

thread_local! {
    static CHANGE_HOOK: std::cell::Cell<Option<ChangeHook>> = const { std::cell::Cell::new(None) };
}

/// Installs the change hook (S-PANELS' inspectors).
pub fn set_change_hook(hook: ChangeHook) {
    CHANGE_HOOK.with(|h| h.set(Some(hook)));
}

/// Surface `id` sent `events`: the change hook told when one was a change
/// or a selection (the kernel and the runtimes call it as they fire them).
pub fn notify(id: &str, events: &[DesignEvent]) {
    if events.iter().any(|e| matches!(e, DesignEvent::Change | DesignEvent::Select(_) | DesignEvent::BgClick(..))) {
        if let Some(h) = CHANGE_HOOK.with(|h| h.get()) {
            h(id);
        }
    }
}

/// A program's source open in the designer: what RDESIGNSURFACE's Source
/// reads into. The parser lives above this crate, so a runtime with the
/// designer (RapidR Studio's: `rapidr_studio::design`, on
/// `rapidr-designer`'s Document) installs the reader
/// ([`set_source_reader`]). The surface designs one of its forms; every
/// change it makes is written back as the smallest text edits, one undo
/// step each ("the source is the truth", docs/ide-plan.md I4).
pub trait SourceDoc {
    /// The text as the designer's edits left it.
    fn text(&self) -> String;
    /// The code changed elsewhere (the code editor): read again. Whether
    /// it differs (the designer's undo history then starts over: its
    /// edits' places are gone).
    fn set_text(&mut self, text: &str) -> bool;
    /// Its top-level CREATE blocks: name and type as written.
    fn forms(&self) -> Vec<(String, String)>;
    /// Form `form`'s designer, as the text says it.
    fn designer(&self, form: usize) -> Option<Designer>;
    /// Writes `designer`'s changes (its journal) into the text as one undo
    /// step: the edits, and form `form`'s designer as the text now says.
    fn commit(&mut self, form: usize, designer: Designer) -> (Vec<SourceEdit>, Option<Designer>);
    /// Undoes the last committed change: the edits that put the exact
    /// text back (`None`: nothing to undo).
    fn undo(&mut self) -> Option<Vec<SourceEdit>>;
    fn redo(&mut self) -> Option<Vec<SourceEdit>>;
    fn can_undo(&self) -> bool;
    fn can_redo(&self) -> bool;
    /// Whether `name` is already a name in the file (a variable, a SUB,
    /// another form's component): a new component mustn't take it.
    fn name_taken(&self, name: &str) -> bool;
    /// What the parser reported (the text doesn't compile as it is).
    fn diagnostics(&self) -> Vec<String>;
    /// The first error's line (from 1) when the text doesn't parse.
    fn error_line(&self) -> Option<usize> {
        None
    }
    /// The handler of `component`'s `event` ("": its type's default
    /// event) — the SUB `OnX = …` names, else a new one written with its
    /// wiring (one undo step): the SUB's name, the line (from 0) the caret
    /// goes to (`None`: not in this file), the edits (`rapidr-designer`'s `Document::create_handler`).
    fn create_handler(&mut self, _form: usize, _component: &str, _event: &str) -> Result<(String, Option<usize>, Vec<SourceEdit>), String> {
        Err("handlers aren't available here".into())
    }
    /// A new form named `name` written at the end of the file with the
    /// line that shows it (`rapidr-designer`'s `Document::add_form`): the
    /// edits (one undo step).
    fn add_form(&mut self, _name: &str) -> Vec<SourceEdit> {
        Vec::new()
    }
}

/// Reads a program's source (its text, its file's path for `$INCLUDE`s).
pub type SourceReader = fn(text: &str, path: &str) -> Box<dyn SourceDoc>;

thread_local! {
    static READER: std::cell::Cell<Option<SourceReader>> = const { std::cell::Cell::new(None) };
    /// A component being dragged in from elsewhere (a toolbox): its type,
    /// until the mouse is let go ([`begin_drop`]).
    static DROP: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// The runtime's source reader (RapidR Studio's runtimes install it).
pub fn set_source_reader(reader: SourceReader) {
    READER.with(|r| r.set(Some(reader)));
}

fn reader() -> Option<SourceReader> {
    READER.with(|r| r.get())
}

/// A new component of `type_name` dragged in from elsewhere (DragComponent):
/// while the mouse stays down, a design surface it moves over shows where
/// it would go, and letting go there adds it.
pub fn begin_drop(type_name: &str) {
    DROP.with(|d| *d.borrow_mut() = (!type_name.trim().is_empty()).then(|| type_name.trim().to_string()));
}

/// The type being dragged in, if any.
pub fn drop_pending() -> Option<String> {
    DROP.with(|d| d.borrow().clone())
}

/// The drag ended (the mouse let go anywhere).
pub fn end_drop() {
    DROP.with(|d| *d.borrow_mut() = None);
}

/// The source a surface designs: the document and which of its forms.
#[derive(Clone)]
struct Attached {
    doc: Rc<RefCell<Box<dyn SourceDoc>>>,
    /// The designed form (`None`: the file makes none).
    form: Option<usize>,
}

impl std::fmt::Debug for Attached {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Attached(form {:?})", self.form)
    }
}

/// A drag in progress.
#[derive(Clone, Debug, Default, PartialEq)]
struct Drag {
    grip: Grip,
    /// Where the press was.
    from: (i64, i64),
    /// The press's offset in the primary selection (moving); the form's
    /// size less the press's place (the form's corner).
    offset: (i64, i64),
    /// The selection's rectangles when the drag began (form coordinates).
    start: Vec<(NodeId, LRect)>,
    /// Where they are now (not written until the mouse is let go).
    now: Vec<(NodeId, LRect)>,
    moved: bool,
    /// (the placing tool) Shift held: it stays armed after this one.
    keep: bool,
    /// Where the mouse is now.
    last: (i64, i64),
}

/// The surface: the designed form (the designer model: its components, the
/// last created on top), the selection, a drag in progress, the guides it
/// shows and the resize preview.
#[derive(Clone, Debug)]
pub struct DesignSurface {
    pub designer: Designer,
    /// The designed form's caption (FormCaption), when its CREATE block sets
    /// none.
    pub form_caption: String,
    /// SelectComp's last number, as the program gave it (-1: none).
    selected_raw: i64,
    /// The surface's size.
    size: (i64, i64),
    /// How far it's scrolled (a form bigger than the surface).
    scroll: (i64, i64),
    /// The form fills the surface (a form made through the API); else it
    /// has its own size (a form read from a program).
    pub fit: bool,
    drag: Option<Drag>,
    /// The guides of the drag in progress (form coordinates).
    pub guides: Vec<Guide>,
    /// The resize preview: the designed form at this size (its Width ×
    /// Height), its components where the running program puts them
    /// (PreviewWidth / PreviewHeight, or the form's corner dragged).
    pub preview: Option<(i64, i64)>,
    pub show_guides: bool,
    /// The grid's dots on the form's face.
    pub show_grid: bool,
    /// The anchor pins are drawn on the primary selection.
    pub show_pins: bool,
    /// The selection's frames, handles and pins are drawn (off: the
    /// designed form alone, as the running program shows it).
    pub show_selection: bool,
    /// The rubber band being drawn (form coordinates).
    band: Option<LRect>,
    /// The program's source, when Source was given (else the form is the
    /// one made through the API).
    source: Option<Attached>,
    /// SourceFile: where the source's `$INCLUDE`s are found from.
    pub source_file: String,
    /// FormName asked for (empty: the file's first form).
    want_form: String,
    /// What the program hears next (the kernel and the runtimes fire it).
    outbox: Vec<DesignEvent>,
    /// The placing tool (PlaceType): the type the next press on the form
    /// places ("": off).
    pub place_type: String,
    /// Where a new component would go (the placing tool under the mouse,
    /// a component dragged in): its rectangle (client coordinates) and type.
    pub ghost: Option<(LRect, String)>,
    /// What a screen reader is told last (the surface's live region).
    pub announcement: String,
    /// The container a drag would move the selection into (highlighted).
    drop_target: Option<NodeId>,
    /// The code doesn't parse (its first error's line): the designer
    /// shows the last good state, read-only, under a banner.
    pub code_error: Option<usize>,
    /// Copy / Cut's components (Paste's).
    clip: Option<crate::designer::Clip>,
    /// Where a menu item being dragged would go: under which menu, before
    /// which item, and the insertion line (client coordinates).
    menu_drop: Option<(NodeId, Option<NodeId>, Rect)>,
    /// A component just added whose Caption / Text the keyboard types:
    /// it, the property, what's typed so far.
    typing: Option<(NodeId, &'static str, String)>,
    /// HandlerLine: where CreateHandler's SUB is (its caret's line, from 0;
    /// -1: none).
    pub handler_line: i64,
    /// The form laid out as designed, for the design it was laid out from.
    laid: RefCell<Option<(FormDesign, Rc<Layout>)>>,
    /// The same at the preview's size.
    previewed: RefCell<Option<((i64, i64), Rc<Layout>)>>,
    /// The view's magnification (Zoom / 100): the form, its tray and the
    /// ghost drawn this many times larger — at the screen's resolution, so
    /// crisp — while every coordinate stays RapidQ's pixels; the
    /// designer's own marks (handles, pins, readouts) keep their size.
    pub zoom: f64,
    /// The components the program creates outside the designed form (its
    /// top-level non-visual CREATE blocks: shown in the tray).
    outside: Vec<outside::Outside>,
    /// The one of them selected (the form's selection is empty then).
    outside_sel: Option<usize>,
    /// The in-place editor: a caption (F2, a slow click) or a menu item's
    /// caption and ShortCut (the menu editor).
    pub editing: Option<inline::InlineEdit>,
    /// The Tab-order editor (TabOrderMode): the components clicked so far,
    /// in order.
    pub tab_order: Option<Vec<NodeId>>,
    /// The menu editor: the menus open on the designed form (a bar item's,
    /// then its submenus').
    pub menu_open: Vec<NodeId>,
    /// A press on the component already selected alone: let go without a
    /// move, it edits its caption (a slow click, as Explorer renames).
    slow: Option<NodeId>,
    /// SharedUndo: the file's undo history is the code editor's — Undo and
    /// Redo here ask the program (OnUndo) instead of using the designer's.
    pub shared_undo: bool,
    /// The next change's edits continue the step before (a caption typed
    /// right after adding: one undo step with the add).
    continues: bool,
    /// A component just dropped or placed (client coordinates): the kernel
    /// settles it in (a 100 ms animation).
    dropped: Option<Rect>,
    /// Where the mouse last was (client coordinates): zoom's fixed point.
    pointer: Option<(i64, i64)>,
}

impl Default for DesignSurface {
    fn default() -> Self {
        let mut design = FormDesign::new("Form", "QFORM");
        let root = design.root();
        for (p, v) in [("Width", "640"), ("Height", "480")] {
            let _ = Command::SetProp { node: root, name: p.into(), value: Some(v.into()) }.apply(&mut design);
        }
        DesignSurface {
            designer: Designer::new(design),
            form_caption: "Form1".into(),
            selected_raw: -1,
            size: (640, 480),
            scroll: (0, 0),
            fit: true,
            drag: None,
            guides: Vec::new(),
            preview: None,
            show_guides: true,
            show_grid: true,
            show_pins: true,
            show_selection: true,
            band: None,
            source: None,
            source_file: String::new(),
            want_form: String::new(),
            outbox: Vec::new(),
            place_type: String::new(),
            ghost: None,
            announcement: String::new(),
            drop_target: None,
            code_error: None,
            clip: None,
            menu_drop: None,
            handler_line: -1,
            typing: None,
            laid: RefCell::new(None),
            previewed: RefCell::new(None),
            zoom: 1.0,
            outside: Vec::new(),
            outside_sel: None,
            editing: None,
            tab_order: None,
            menu_open: Vec::new(),
            slow: None,
            shared_undo: false,
            continues: false,
            dropped: None,
            pointer: None,
        }
    }
}

/// How a SetProp value is written: as the program gave it for numbers,
/// colours, Booleans, constants (the registry's type); in quotes otherwise.
fn source_value(type_name: &str, prop: &str, value: &str) -> String {
    let ty = rapidr_lang::component(type_name).and_then(|c| c.property(prop)).map(|p| p.ty);
    match ty {
        Some(rapidr_lang::Type::Int | rapidr_lang::Type::Float | rapidr_lang::Type::Bool | rapidr_lang::Type::Color | rapidr_lang::Type::Enum | rapidr_lang::Type::Set) => value.to_string(),
        _ => value::write_str(value),
    }
}

/// A value as GetProp gives it back: a string's text, else as written.
fn plain_value(text: &str) -> String {
    match value::read(text) {
        value::PropValue::Str(s) => s,
        _ => text.to_string(),
    }
}

/// What a component of this type shows before it is named (the ghost of
/// one being placed): its caption property and its type's name
/// (`Button`).
pub fn sample_caption(type_name: &str) -> Option<(&'static str, String)> {
    let c = rapidr_lang::component(type_name)?;
    let prop = ["Caption", "Text"].into_iter().find(|p| c.property(p).is_some())?;
    Some((prop, c.display.to_string()))
}

/// The tray's font (its items' names).
pub fn tray_font() -> Font {
    Font::default()
}

impl DesignSurface {
    /// A surface showing `designer`'s form at its own size (a form read from
    /// a program: `rapidr-designer`'s `Document::designer`).
    pub fn with_designer(designer: Designer) -> DesignSurface {
        let mut d = DesignSurface { designer, fit: false, ..DesignSurface::default() };
        d.form_caption = d.root_text("Caption").unwrap_or_default();
        d
    }

    /// Shows `designer`'s form instead (at its own size), nothing selected.
    pub fn set_designer(&mut self, designer: Designer) {
        let snapper = self.designer.snapper;
        self.designer = designer;
        self.designer.snapper = snapper;
        self.designer.selection.clear();
        self.forget_gestures();
        self.fit = false;
        self.form_caption = self.root_text("Caption").unwrap_or_default();
    }

    /// A drag, the preview, the guides, the ghost all gone.
    fn forget_gestures(&mut self) {
        self.menu_drop = None;
        self.slow = None;
        self.drag = None;
        self.drop_target = None;
        self.preview = None;
        self.guides.clear();
        self.band = None;
        self.ghost = None;
        self.selected_raw = -1;
    }

    // ------------------------------------------------ the program's source --

    /// Source = text: the program's source read (a reader installed: else
    /// nothing happens), its form designed — the one FormName names, else
    /// its first QFORM. The same text again changes nothing; another text
    /// is read again, the selection kept where its components still are.
    pub fn open_source(&mut self, text: &str) -> bool {
        let first = match &self.source {
            Some(a) => {
                if !a.doc.borrow_mut().set_text(text) {
                    return true;
                }
                false
            }
            None => {
                let Some(read) = reader() else { return false };
                let doc = read(text, &self.source_file);
                self.source = Some(Attached { doc: Rc::new(RefCell::new(doc)), form: None });
                self.designer.selection.clear();
                true
            }
        };
        // (code with errors: the last good form stays, read-only)
        self.code_error = self.source.as_ref().and_then(|a| a.doc.borrow().error_line());
        if let Some(line) = self.code_error.filter(|_| !first) {
            self.forget_gestures();
            self.say(format!("The code has errors at line {line}: the designer is read-only until they are fixed"));
            return true;
        }
        self.pick_form(false);
        true
    }

    /// AddForm: a form for a file that has none (the designer's empty
    /// state offers it) — `CREATE Form1 AS QFORM … END CREATE` and
    /// `Form1.ShowModal` written at the end of the code, one undo step —
    /// then designed. Its name ("": Form1, Form2 … a name the file
    /// doesn't use).
    pub fn add_form(&mut self, name: &str) -> Option<String> {
        let a = self.source.clone()?;
        if self.read_only() {
            return None;
        }
        let taken = |n: &str| a.doc.borrow().name_taken(n);
        let name = match name.trim() {
            n if !n.is_empty() && !taken(n) => n.to_string(),
            _ => (1..).map(|k| format!("Form{k}")).find(|n| !taken(n)).unwrap_or_default(),
        };
        let edits = a.doc.borrow_mut().add_form(&name);
        if edits.is_empty() {
            return None;
        }
        self.step_begins();
        self.outbox.extend(edits.into_iter().map(DesignEvent::SourceEdit));
        self.want_form = name.clone();
        self.pick_form(true);
        self.outbox.push(DesignEvent::Change);
        self.say(format!("Added {name} (QFORM): drop components on it"));
        Some(name)
    }

    /// The empty state's "Add a form" button (surface pixels), when the
    /// source makes no form.
    pub fn add_form_button(&self) -> Option<Rect> {
        (self.no_form() && !self.read_only()).then(|| {
            let (w, h) = (120, 28);
            ((self.size.0 - w) / 2, self.size.1 / 2 + 24, w, h)
        })
    }

    /// The designer can't change the form now (the code has errors).
    pub fn read_only(&self) -> bool {
        self.code_error.is_some()
    }

    /// The banner over the designer, when the code has errors.
    pub fn banner(&self) -> Option<String> {
        self.code_error.map(|line| format!("The code has errors at line {line}. The designer shows the form as it last compiled and changes nothing until they are fixed."))
    }

    /// The form the surface designs, chosen again (`fresh`: another form —
    /// nothing selected).
    fn pick_form(&mut self, fresh: bool) {
        let Some(a) = &self.source else { return };
        let forms = a.doc.borrow().forms();
        let named = (!self.want_form.is_empty()).then(|| forms.iter().position(|(n, _)| n.eq_ignore_ascii_case(&self.want_form))).flatten();
        let form = named.or_else(|| forms.iter().position(|(_, t)| crate::designer::model::canonical_type(t) == "RFORM"));
        let changed = a.form != form;
        if let Some(a) = &mut self.source {
            a.form = form;
        }
        self.fit = false;
        match form {
            Some(_) => self.reload(fresh || changed),
            None => {
                self.designer = Designer::new(FormDesign::new("", "QFORM"));
                self.forget_gestures();
                self.outside.clear();
                self.outside_sel = None;
            }
        }
    }

    /// The designed form as the source now says it (the selection kept
    /// where its components still are, unless `fresh`).
    fn reload(&mut self, fresh: bool) {
        let Some(a) = &self.source else { return };
        let Some(form) = a.form else { return };
        let Some(mut d) = a.doc.borrow().designer(form) else { return };
        d.selection = if fresh { Default::default() } else { self.designer.selection.clone() };
        d.selection.retain(&d.design);
        d.snapper = self.designer.snapper;
        self.designer = d;
        if fresh {
            self.forget_gestures();
        }
        self.form_caption = self.root_text("Caption").unwrap_or_else(|| self.root_name());
        self.refresh_outside();
        self.menu_open.retain(|&m| self.designer.design.node(m).is_some());
    }

    /// The designed form's name ("" with none).
    pub fn root_name(&self) -> String {
        let d = &self.designer.design;
        d.node(d.root()).map(|n| n.name.clone()).unwrap_or_default()
    }

    /// The source is read but makes no form: the surface says so (its
    /// empty state) and a dropped component has nowhere to go.
    pub fn no_form(&self) -> bool {
        self.source.as_ref().is_some_and(|a| a.form.is_none())
    }

    /// What the surface says where there is no form to show.
    pub fn empty_text(&self) -> Option<&'static str> {
        self.no_form().then_some("This file creates no form.\nAdd a form — or press Enter — to design one here.")
    }

    /// The designer's changes since the last commit written into the
    /// source (one undo step) — OnSourceEdit for each edit, then OnChange;
    /// without a source, they stay the designer's own (its Undo / Redo).
    pub fn commit(&mut self) {
        if !self.designer.has_applied() {
            return;
        }
        // (the code has errors: nothing is written — the change is dropped)
        if self.read_only() {
            while self.designer.undo() {}
            self.designer.take_applied();
            return;
        }
        let d = self.designer.clone();
        self.designer.take_applied();
        if let Some(Attached { doc, form: Some(form) }) = self.source.clone() {
            self.step_begins();
            let (edits, after) = doc.borrow_mut().commit(form, d);
            if let Some(mut after) = after {
                after.selection = self.designer.selection.clone();
                after.selection.retain(&after.design);
                after.snapper = self.designer.snapper;
                self.designer = after;
            }
            self.outbox.extend(edits.into_iter().map(DesignEvent::SourceEdit));
        }
        self.outbox.push(DesignEvent::Change);
    }

    /// A change's edits begin: OnSourceStep (one undo step, or part of the
    /// one before when [`DesignSurface::continues`] says so).
    fn step_begins(&mut self) {
        let c = std::mem::take(&mut self.continues);
        self.outbox.push(DesignEvent::Step(c));
    }

    /// The handler of component `name`'s `event` ("": the default event):
    /// its SUB, made and wired when there's none (one undo step, its edits
    /// heard as OnSourceEdit); [`DesignSurface::handler_line`] is where it is.
    pub fn create_handler(&mut self, name: &str, event: &str) -> Option<String> {
        let Some(Attached { doc, form: Some(form) }) = self.source.clone() else { return None };
        self.commit();
        let r = doc.borrow_mut().create_handler(form, name, event);
        match r {
            Ok((sub, line, edits)) => {
                self.handler_line = line.map_or(-1, |l| l as i64);
                if !edits.is_empty() {
                    self.reload(false);
                    self.step_begins();
                    self.outbox.extend(edits.into_iter().map(DesignEvent::SourceEdit));
                    self.outbox.push(DesignEvent::Change);
                }
                Some(sub)
            }
            Err(e) => {
                self.handler_line = -1;
                self.say(e);
                None
            }
        }
    }

    /// What the program hasn't heard yet, oldest first.
    pub fn take_events(&mut self) -> Vec<DesignEvent> {
        std::mem::take(&mut self.outbox)
    }

    /// Undo: the last change undone (through the source's history when
    /// there is one: its exact text back).
    pub fn undo(&mut self) -> bool {
        self.history_step(true)
    }

    pub fn redo(&mut self) -> bool {
        self.history_step(false)
    }

    fn history_step(&mut self, back: bool) -> bool {
        self.forget_gestures();
        self.editing = None;
        // (the file's history is the code editor's: the program undoes there)
        if self.shared_undo && self.source.is_some() {
            self.outbox.push(DesignEvent::Undo(!back));
            return true;
        }
        match self.source.clone() {
            Some(a) => {
                let edits = if back { a.doc.borrow_mut().undo() } else { a.doc.borrow_mut().redo() };
                let Some(edits) = edits else { return false };
                self.reload(false);
                self.step_begins();
                self.outbox.extend(edits.into_iter().map(DesignEvent::SourceEdit));
                self.outbox.push(DesignEvent::Change);
                self.say(if back { "Undone" } else { "Redone" });
                true
            }
            None => {
                let done = if back { self.designer.undo() } else { self.designer.redo() };
                if done {
                    self.designer.take_applied();
                    self.outbox.push(DesignEvent::Change);
                }
                done
            }
        }
    }

    pub fn can_undo(&self) -> bool {
        match &self.source {
            Some(a) => a.doc.borrow().can_undo(),
            None => self.designer.history.can_undo(),
        }
    }

    pub fn can_redo(&self) -> bool {
        match &self.source {
            Some(a) => a.doc.borrow().can_redo(),
            None => self.designer.history.can_redo(),
        }
    }

    /// The live region's text.
    fn say(&mut self, text: impl Into<String>) {
        self.announcement = text.into();
    }

    /// What a screen reader is told about component `id`: its name, type,
    /// place and size.
    fn describe_comp(&self, id: NodeId) -> String {
        let Some(n) = self.designer.design.node(id) else { return String::new() };
        match self.rect_of(id).filter(|_| self.on_form(id)) {
            Some(r) => format!("{} ({}), {}, {}, {} × {}", n.name, n.type_written, r.left, r.top, r.width, r.height),
            None => format!("{} ({})", n.name, n.type_written),
        }
    }

    /// The selection changed: OnSelect with the primary's index (-1: none),
    /// and the screen reader told.
    fn selection_changed(&mut self) {
        let i = self.selection().map_or(-1, |i| i as i64);
        self.selected_raw = i;
        self.outbox.push(DesignEvent::Select(i));
        let text = match self.designer.selection.primary() {
            Some(id) if self.designer.selection.len() > 1 => format!("{}, and {} more selected", self.describe_comp(id), self.designer.selection.len() - 1),
            Some(id) => self.describe_comp(id),
            None => format!("{} ({}), nothing selected", self.root_name(), self.designer.design.node(self.designer.design.root()).map(|n| n.type_written.clone()).unwrap_or_default()),
        };
        self.say(text);
    }

    /// The designed components, in creation order (the form not counted):
    /// what the API's indexes count.
    pub fn ids(&self) -> Vec<NodeId> {
        self.designer.design.ids().into_iter().skip(1).collect()
    }

    fn id_of(&self, index: i64) -> Option<NodeId> {
        usize::try_from(index).ok().and_then(|i| self.ids().get(i).copied())
    }

    fn index_of(&self, id: NodeId) -> Option<usize> {
        self.ids().iter().position(|&i| i == id)
    }

    /// The primary selection's index, when it is one (an outside
    /// component's: after the form's components).
    pub fn selection(&self) -> Option<usize> {
        if let Some(k) = self.outside_sel {
            return Some(self.ids().len() + k);
        }
        self.designer.selection.primary().and_then(|id| self.index_of(id))
    }

    /// The selected components' indexes.
    pub fn selected(&self) -> Vec<usize> {
        if let Some(k) = self.outside_sel {
            return vec![self.ids().len() + k];
        }
        self.designer.selection.ids().iter().filter_map(|&id| self.index_of(id)).collect()
    }

    /// Selects component `i` alone (a screen reader's click).
    pub fn select(&mut self, i: usize) -> bool {
        let n = self.ids().len();
        if i >= n {
            return self.select_outside(i - n);
        }
        self.outside_sel = None;
        match self.id_of(i as i64) {
            Some(id) => {
                self.designer.selection.set(id);
                self.selected_raw = i as i64;
                true
            }
            None => false,
        }
    }

    // ------------------------------------------------------ geometry --

    /// A property of the form itself, as its CREATE block writes it.
    fn root_text(&self, prop: &str) -> Option<String> {
        let d = &self.designer.design;
        d.node(d.root()).and_then(|n| n.prop(prop)).map(plain_value)
    }

    fn root_int(&self, prop: &str) -> Option<i64> {
        let d = &self.designer.design;
        d.node(d.root()).and_then(|n| n.prop(prop)).and_then(|t| d.read_value(t).int())
    }

    /// The designed form's title: its Caption, else FormCaption.
    pub fn title(&self) -> String {
        self.root_text("Caption").unwrap_or_else(|| self.form_caption.clone())
    }

    /// The form has a frame (BorderStyle isn't bsNone).
    pub fn border(&self) -> bool {
        self.root_int("BorderStyle").unwrap_or(2) != 0
    }

    /// The form's main menu bar's height (0 without a QMAINMENU).
    pub fn menu_height(&self) -> i64 {
        let d = &self.designer.design;
        if d.children(d.root()).iter().any(|&c| d.node(c).is_some_and(|n| n.canonical == "RMAINMENU")) {
            MAIN_MENU_HEIGHT
        } else {
            0
        }
    }

    /// The backdrop around the form (none for a form without a frame that
    /// fills the surface).
    pub fn margin(&self) -> i64 {
        if self.border() || !self.fit {
            MARGIN
        } else {
            0
        }
    }

    /// The form's inside starts this far into its window (its frame's left
    /// and top: border, title bar).
    fn inset(&self) -> (i64, i64) {
        if self.border() {
            (FORM_BORDER, FORM_BORDER + FORM_CAPTION)
        } else {
            (0, 0)
        }
    }

    /// The form's window on the surface: where (surface pixels), and its
    /// Width × Height (the preview's while it shows; RapidQ's pixels — on
    /// the surface it is [`DesignSurface::zoom`] times that).
    pub fn form_rect(&self) -> Rect {
        let m = self.margin();
        let (w, h) = self.layout().form_size();
        (m - self.scroll.0, m - self.scroll.1, w, h)
    }

    /// The form's frame and menu bar above and left of its client area
    /// (RapidQ's pixels): where the client area's (0, 0) is in its window.
    pub fn client_inset(&self) -> (i64, i64) {
        let (ix, iy) = self.inset();
        (ix, iy + self.menu_height())
    }

    /// Where the form's client area (0, 0) is on the surface.
    pub fn client_origin(&self) -> (i64, i64) {
        let (fx, fy, _, _) = self.form_rect();
        let (ix, iy) = self.client_inset();
        (fx + self.zv(ix), fy + self.zv(iy))
    }

    /// A length of RapidQ's pixels as the surface shows it (zoomed).
    pub fn zv(&self, v: i64) -> i64 {
        (v as f64 * self.zoom).round() as i64
    }

    /// A rectangle of the client area as the surface shows it, from the
    /// client area's origin on the surface (zoomed; its edges on whole
    /// pixels).
    pub fn view_rect(&self, (x, y, w, h): Rect) -> Rect {
        let (x0, y0, x1, y1) = (self.zv(x), self.zv(y), self.zv(x + w), self.zv(y + h));
        (x0, y0, x1 - x0, y1 - y0)
    }

    /// A distance of the surface's pixels in RapidQ's (a handle's reach at
    /// any zoom), at least 1.
    fn reach(&self, v: i64) -> i64 {
        ((v as f64 / self.zoom).round() as i64).max(1)
    }

    /// A point of the surface (from the client area's origin) in the form's
    /// client coordinates: what the mouse points at, at any zoom.
    pub fn client_point(&self, x: f64, y: f64) -> (i64, i64) {
        let (ox, oy) = self.client_origin();
        (((x - ox as f64) / self.zoom).floor() as i64, ((y - oy as f64) / self.zoom).floor() as i64)
    }

    /// Zoom: the view's magnification (1 = 100 %), kept between 25 % and
    /// 400 %; the point (`at`: surface pixels, `None`: the form's top left)
    /// stays where it is. Whether it changed.
    pub fn set_zoom(&mut self, zoom: f64, at: Option<(f64, f64)>) -> bool {
        let z = (zoom.clamp(MIN_ZOOM, MAX_ZOOM) * 100.0).round() / 100.0;
        if (z - self.zoom).abs() < 1e-9 {
            return false;
        }
        let (fx, fy, _, _) = self.form_rect();
        let (ax, ay) = at.unwrap_or((fx as f64, fy as f64));
        // (the content point under `at`, before and after)
        let (cx, cy) = ((ax - fx as f64) / self.zoom, (ay - fy as f64) / self.zoom);
        self.zoom = z;
        let (nx, ny) = (fx as f64 + cx * z, fy as f64 + cy * z);
        self.scroll_by((nx - ax).round() as i64, (ny - ay).round() as i64);
        self.say(format!("Zoom {} %", (z * 100.0).round()));
        true
    }

    /// The next zoom step in (`dir` > 0) or out (< 0); 0: 100 %.
    pub fn zoom_step(&mut self, dir: i32, at: Option<(f64, f64)>) -> bool {
        let z = self.zoom;
        let to = match dir {
            0 => 1.0,
            d if d > 0 => ZOOM_STEPS.iter().copied().find(|&s| s > z + 1e-6).unwrap_or(MAX_ZOOM),
            _ => ZOOM_STEPS.iter().rev().copied().find(|&s| s < z - 1e-6).unwrap_or(MIN_ZOOM),
        };
        self.set_zoom(to, at)
    }

    /// The surface scrolled by (dx, dy) pixels (the mouse wheel), within
    /// what it shows (the form, its tray, the backdrop's margin); whether
    /// it moved.
    pub fn scroll_by(&mut self, dx: i64, dy: i64) -> bool {
        let m = self.margin();
        let (w, h) = self.layout().form_size();
        let tray = if self.tray().is_empty() { 0 } else { TRAY_GAP + TRAY_H };
        let (cw, ch) = (self.zv(w) + 2 * m + CORNER, self.zv(h + tray) + 2 * m + CORNER);
        let max = ((cw - self.size.0).max(0), (ch - self.size.1).max(0));
        let to = ((self.scroll.0 + dx).clamp(0, max.0), (self.scroll.1 + dy).clamp(0, max.1));
        let moved = to != self.scroll;
        self.scroll = to;
        moved
    }

    /// The form's inside below its menu (its scroll bars included).
    pub fn client_size(&self) -> (i64, i64) {
        let (_, _, w, h) = self.form_rect();
        crate::layout::form_client_size(w, h, if self.border() { 2 } else { 0 }, self.menu_height())
    }

    /// The tray strip under the form (client coordinates): its items, left
    /// to right — the form's non-visual components, then those its program
    /// creates outside it (notepad.bas's OpenDialog and SaveDialog, their
    /// indexes after the form's components); empty when there are none.
    pub fn tray(&self) -> Vec<TrayItem> {
        let d = &self.designer.design;
        let (ix, iy) = self.client_inset();
        let (_, _, _, fh) = self.form_rect();
        let top = fh + TRAY_GAP - iy;
        let mut x = -ix + 4;
        let font = tray_font();
        let ids = self.ids();
        let n_ids = ids.len();
        let mut entries: Vec<(usize, String, String)> = ids
            .into_iter()
            .enumerate()
            .filter_map(|(index, id)| {
                let n = d.node(id)?;
                if n.is_visual() || matches!(n.canonical.as_str(), "RMAINMENU" | "RMENUITEM") {
                    return None;
                }
                Some((index, n.name.clone(), n.type_written.clone()))
            })
            .collect();
        entries.extend(self.outside.iter().enumerate().map(|(k, o)| (n_ids + k, o.name.clone(), o.type_name.clone())));
        entries
            .into_iter()
            .map(|(index, name, type_name)| {
                let w = 4 + TRAY_ICON + 4 + super::text::text_size(&name, &font).0 + 8;
                let rect = (x, top + 4, w, TRAY_H - 8);
                x += w + 4;
                TrayItem { index, name, type_name, rect }
            })
            .collect()
    }

    /// The tray strip's rectangle (client coordinates), when it shows.
    pub fn tray_rect(&self) -> Option<Rect> {
        let items = self.tray();
        let last = items.last()?;
        let (ix, iy) = self.client_inset();
        let (_, _, fw, fh) = self.form_rect();
        let w = fw.max(last.rect.0 + last.rect.2 + 4 + ix);
        Some((-ix, fh + TRAY_GAP - iy, w, TRAY_H))
    }

    /// The surface's size: a form made through the API fills it (less the
    /// margin and the tray strip).
    pub fn set_size(&mut self, w: i64, h: i64) {
        if w <= 0 || h <= 0 {
            return;
        }
        self.size = (w, h);
        if !self.fit {
            self.scroll_by(0, 0);
            return;
        }
        let m = self.margin();
        let tray = if self.tray().is_empty() { 0 } else { TRAY_GAP + TRAY_H };
        let unzoom = |v: i64| (v as f64 / self.zoom).floor() as i64;
        let (fw, fh) = (unzoom(w - 2 * m).max(MIN_SIZE), (unzoom(h - 2 * m) - tray).max(MIN_SIZE));
        let root = self.designer.design.root();
        for (p, v) in [("Width", fw), ("Height", fh)] {
            if self.root_int(p) != Some(v) {
                let _ = Command::SetProp { node: root, name: p.into(), value: Some(v.to_string()) }.apply(&mut self.designer.design);
            }
        }
    }

    /// The form laid out as designed (kept while the design is unchanged).
    fn laid_out(&self) -> Rc<Layout> {
        let mut laid = self.laid.borrow_mut();
        if let Some((d, l)) = laid.as_ref() {
            if *d == self.designer.design {
                return l.clone();
            }
        }
        let l = Rc::new(self.designer.layout());
        *laid = Some((self.designer.design.clone(), l.clone()));
        self.previewed.borrow_mut().take();
        l
    }

    /// The form laid out: as designed, or at the preview's size (both kept
    /// while the design and the preview are unchanged).
    pub fn layout(&self) -> Rc<Layout> {
        let base = self.laid_out();
        let Some(size) = self.preview else { return base };
        let mut previewed = self.previewed.borrow_mut();
        if let Some((s, l)) = previewed.as_ref() {
            if *s == size {
                return l.clone();
            }
        }
        let mut l = (*base).clone();
        l.resize(size.0, size.1);
        let l = Rc::new(l);
        *previewed = Some((size, l.clone()));
        l
    }

    /// Each component as drawn, in creation order (one per index): its
    /// rectangle on the form's client area (during a drag, where the drag
    /// puts it), its properties' values.
    pub fn components(&self) -> Vec<DesignComp> {
        let l = self.layout();
        let d = &self.designer.design;
        let dragging = self.drag.as_ref().filter(|g| g.grip.moves());
        self.ids()
            .into_iter()
            .filter_map(|id| {
                let n = d.node(id)?;
                let r = dragging.and_then(|g| g.now.iter().find(|(i, _)| *i == id).map(|(_, r)| *r)).or_else(|| {
                    let r = l.rect(id)?;
                    let o = l.origin(d, id);
                    Some(LRect::new(r.left + o.0, r.top + o.1, r.width, r.height))
                });
                let r = r.unwrap_or_default();
                let visual = n.is_visual() && crate::layout::default_size(&n.canonical).is_some();
                let props = n.props().map(|p| (p.name.to_lowercase(), plain_value(&p.value))).collect();
                Some(DesignComp { id, name: n.name.clone(), type_name: n.type_written.clone(), canonical: n.canonical.clone(), x: r.left, y: r.top, w: r.width, h: r.height, visual, props })
            })
            .collect()
    }

    /// Every component's place in its parent's client area as drawn now
    /// (Left, Top, Width, Height: the layout's, a drag's, the preview's):
    /// what the design-time store gives the kernel's components.
    pub fn placed(&self) -> Vec<(NodeId, LRect)> {
        let l = self.layout();
        let d = &self.designer.design;
        let moving = self.drag.as_ref().filter(|g| g.moved && g.grip.moves());
        let mut out: Vec<(NodeId, LRect)> = l.rects();
        if let Some(g) = moving {
            for (id, abs) in &g.now {
                let o = l.origin(d, *id);
                if let Some(r) = out.iter_mut().find(|(i, _)| i == id) {
                    r.1 = LRect::new(abs.left - o.0, abs.top - o.1, abs.width, abs.height);
                }
            }
        }
        out
    }

    fn rect_of(&self, id: NodeId) -> Option<LRect> {
        let l = self.layout();
        let r = l.rect(id)?;
        let o = l.origin(&self.designer.design, id);
        Some(LRect::new(r.left + o.0, r.top + o.1, r.width, r.height))
    }

    /// AddComponent: the new one is selected.
    pub fn add(&mut self, type_name: &str, name: &str, bounds: (i64, i64, i64, i64)) {
        let (x, y, w, h) = bounds;
        let d = &self.designer.design;
        let name = if name.is_empty() || d.find(name).is_some() { d.new_name(type_name) } else { name.to_string() };
        let tree = Subtree::new(&name, type_name, &[("Caption", value::write_str(&name)), ("Left", x.to_string()), ("Top", y.to_string()), ("Width", w.to_string()), ("Height", h.to_string())]);
        let root = self.designer.design.root();
        let index = self.designer.design.node(root).map_or(0, |n| n.body.len());
        if self.designer.execute(Command::Insert { parent: root, index, tree }).is_ok() {
            if let Some(id) = self.designer.design.find(&name) {
                self.designer.selection.set(id);
                self.selected_raw = self.index_of(id).map_or(-1, |i| i as i64);
            }
        }
        self.commit();
    }

    /// RemoveComponent: the selection goes.
    pub fn remove(&mut self, i: i64) -> bool {
        let Some(id) = self.id_of(i) else { return false };
        let ok = self.designer.execute(Command::Remove { node: id }).is_ok();
        self.designer.selection.clear();
        self.selected_raw = -1;
        self.commit();
        ok
    }

    /// ClearAll.
    pub fn clear(&mut self) {
        let cmds = self.designer.design.children(self.designer.design.root()).into_iter().map(|node| Command::Remove { node }).collect();
        let _ = self.designer.execute(Command::Batch(cmds));
        self.designer.selection.clear();
        self.selected_raw = -1;
        self.commit();
    }

    // ------------------------------------------------- adding components --

    /// The container a new component placed at (x, y) (client
    /// coordinates) goes into — the topmost panel, group box, scroll box …
    /// there, else the form — and where its children's (0, 0) is.
    pub fn container_at(&self, x: i64, y: i64) -> (NodeId, (i64, i64)) {
        self.container_at_except(x, y, &[])
    }

    /// The same, leaving out `moving` and what's inside them (a drag's
    /// drop target).
    fn container_at_except(&self, x: i64, y: i64, moving: &[NodeId]) -> (NodeId, (i64, i64)) {
        let d = &self.designer.design;
        let mut best = (d.root(), (0, 0));
        for c in self.components() {
            let inside = x >= c.x && y >= c.y && x < c.x + c.w && y < c.y + c.h;
            let moved = moving.iter().any(|&m| d.is_within(c.id, m));
            if c.visual && inside && !moved && d.node(c.id).is_some_and(|n| n.is_container() && !n.is_form()) {
                best = (c.id, (c.x, c.y));
            }
        }
        best
    }

    /// A name for a new component of `type_written` that nothing in the
    /// form or the file has (`Button1`, `Button2` …).
    fn fresh_name(&self, type_written: &str) -> String {
        let d = &self.designer.design;
        let first = d.new_name(type_written);
        let base = first.trim_end_matches(|c: char| c.is_ascii_digit()).to_string();
        let taken = |n: &str| d.find(n).is_some() || self.source.as_ref().is_some_and(|a| a.doc.borrow().name_taken(n));
        (1..).map(|k| format!("{base}{k}")).find(|n| !taken(n)).unwrap_or(first)
    }

    /// Where a new component of `type_name` would go with its top left at
    /// (x, y) (client coordinates): its type's default size, the top left
    /// on the grid (unless `free`).
    pub fn new_rect(&self, type_name: &str, x: i64, y: i64, free: bool) -> LRect {
        let canonical = crate::designer::model::canonical_type(type_name);
        let (w, h) = crate::layout::default_size(&canonical).unwrap_or((TRAY_ICON, TRAY_ICON));
        let (x, y) = if free { (x, y) } else { (self.snap_point(x), self.snap_point(y)) };
        LRect::new(x, y, w, h)
    }

    fn snap_point(&self, v: i64) -> i64 {
        let g = self.designer.snapper.grid.max(1);
        if self.designer.snapper.snap_to_grid {
            (v as f64 / g as f64).round() as i64 * g
        } else {
            v
        }
    }

    /// Adds a new component of `type_name` (any of its names) at `rect`
    /// (client coordinates; `None`: its default size at `at`), into the
    /// container there: its CREATE block written (Delphi's names, RapidQ's
    /// type names), selected, announced. Returns its index; `None` when it
    /// can't go there (a form on a form, a menu item without a menu, no
    /// form to put it on).
    pub fn add_at(&mut self, type_name: &str, at: (i64, i64), rect: Option<LRect>) -> Option<usize> {
        if self.no_form() || self.read_only() {
            return None;
        }
        let canonical = crate::designer::model::canonical_type(type_name);
        if canonical == "RFORM" || canonical.is_empty() {
            return None;
        }
        let comp = rapidr_lang::component(&canonical);
        let visual = comp.is_none_or(|c| c.visual) && crate::layout::default_size(&canonical).is_some() && !matches!(canonical.as_str(), "RMAINMENU" | "RPOPUPMENU" | "RMENUITEM");
        let d = &self.designer.design;
        let (parent, origin) = if canonical == "RMENUITEM" {
            // (under the selected menu or item, else the form's main menu)
            let sel = self.designer.selection.primary().filter(|&p| d.node(p).is_some_and(|n| matches!(n.canonical.as_str(), "RMAINMENU" | "RPOPUPMENU" | "RMENUITEM")));
            let menu = sel.or_else(|| d.children(d.root()).into_iter().find(|&c| d.node(c).is_some_and(|n| n.canonical == "RMAINMENU")))?;
            (menu, (0, 0))
        } else if visual {
            self.container_at(at.0, at.1)
        } else {
            (d.root(), (0, 0))
        };
        let (cw, ch) = self.client_size();
        if visual && (at.0 < 0 || at.1 < 0 || at.0 >= cw || at.1 >= ch) {
            self.say("A component goes inside the form");
            return None;
        }
        let r = match rect {
            Some(r) if r.width >= 4 && r.height >= 4 => r,
            _ => self.new_rect(type_name, at.0, at.1, rect.is_some()),
        };
        let local = LRect::new(r.left - origin.0, r.top - origin.1, r.width, r.height);
        let mut tree = crate::designer::text::new_component(d, type_name, local);
        let name = self.fresh_name(&tree.type_written);
        if name != tree.name {
            if tree.prop("Caption").is_some() {
                tree.set_prop("Caption", value::write_str(&name));
            }
            tree.name = name.clone();
        }
        let index = d.node(parent).map_or(0, |p| p.body.len());
        self.designer.execute(Command::Insert { parent, index, tree }).ok()?;
        let id = self.designer.design.find(&name)?;
        self.designer.selection.set(id);
        self.commit();
        self.selection_changed();
        let what = self.describe_comp(id);
        self.say(format!("Added {what}"));
        // (typing now writes its Caption / Text, as Delphi's designer does)
        let prop = rapidr_lang::component(&canonical).and_then(|c| ["Caption", "Text"].into_iter().find(|p| c.property(p).is_some()));
        self.typing = prop.map(|p| (id, p, String::new()));
        self.index_of(id)
    }

    /// The same, from the mouse (a toolbox's drop, the placing tool): the
    /// new component settles in where it was let go.
    pub fn add_dropped(&mut self, type_name: &str, at: (i64, i64), rect: Option<LRect>) -> Option<usize> {
        let i = self.add_at(type_name, at, rect)?;
        let id = self.ids().get(i).copied();
        self.dropped = id.and_then(|id| self.rect_of(id)).filter(|_| id.is_some_and(|id| self.on_form(id))).map(|r| (r.left, r.top, r.width, r.height));
        Some(i)
    }

    /// The component just dropped, once (the kernel's animation).
    pub fn take_dropped(&mut self) -> Option<Rect> {
        self.dropped.take()
    }

    /// Where the mouse last was on the surface (zoom keeps that point).
    pub fn last_pointer(&self) -> Option<(f64, f64)> {
        let (ox, oy) = self.client_origin();
        self.pointer.map(|(x, y)| (ox as f64 + x as f64 * self.zoom, oy as f64 + y as f64 * self.zoom))
    }

    /// The selection deleted (one undo step).
    pub fn delete_selection(&mut self) -> bool {
        if self.designer.selection.is_empty() {
            return false;
        }
        let n = self.designer.selection.len();
        let ok = self.designer.delete().is_ok();
        self.commit();
        self.selection_changed();
        self.say(if n == 1 { "Deleted 1 component".to_string() } else { format!("Deleted {n} components") });
        ok
    }

    /// The primary selection's handle at (x, y), if the mouse is on one
    /// (the corners first, then the sides' middles).
    pub fn grip_at(&self, x: i64, y: i64) -> Option<Grip> {
        if !self.show_selection {
            return None;
        }
        let id = self.designer.selection.primary()?;
        if !self.on_form(id) {
            return None;
        }
        let r = self.rect_of(id)?;
        let grab = self.reach(GRAB);
        let near = |hx: i64, hy: i64| (x - hx).abs() <= grab && (y - hy).abs() <= grab;
        let (l, t, rr, b, cx, cy) = (r.left, r.top, r.left + r.width, r.top + r.height, r.left + r.width / 2, r.top + r.height / 2);
        [(rr, b, Grip::Corner), (l, t, Grip::TopLeft), (rr, t, Grip::TopRight), (l, b, Grip::BottomLeft), (rr, cy, Grip::Right), (cx, b, Grip::Bottom), (l, cy, Grip::Left), (cx, t, Grip::Top)]
            .into_iter()
            .find(|&(hx, hy, _)| near(hx, hy))
            .map(|(_, _, g)| g)
    }

    /// Component `id` is drawn on the form (not a tray item or a menu).
    fn on_form(&self, id: NodeId) -> bool {
        self.designer.design.node(id).is_some_and(|n| n.is_visual() && crate::layout::default_size(&n.canonical).is_some())
    }

    /// The primary selection's anchor pin at (x, y): the side it toggles.
    pub fn pin_at(&self, x: i64, y: i64) -> Option<Side> {
        if !self.show_pins || !self.show_selection {
            return None;
        }
        let id = self.designer.selection.primary()?;
        if !self.on_form(id) {
            return None;
        }
        // (the pins keep their size at any zoom: found on the surface)
        let r = self.rect_of(id)?;
        let (vx, vy, vw, vh) = self.view_rect((r.left, r.top, r.width, r.height));
        let (px, py) = (self.zv(x), self.zv(y));
        pins(LRect::new(vx, vy, vw, vh)).into_iter().find(|(_, p)| px >= p.left && px < p.left + p.width && py >= p.top && py < p.top + p.height).map(|(s, _)| s)
    }

    /// The topmost component at (x, y) (its edges included), or the tray
    /// item there.
    pub fn component_at(&self, x: i64, y: i64) -> Option<usize> {
        if let Some(t) = self.tray().into_iter().find(|t| x >= t.rect.0 && x < t.rect.0 + t.rect.2 && y >= t.rect.1 && y < t.rect.1 + t.rect.3) {
            return Some(t.index);
        }
        let (cw, ch) = self.client_size();
        if x < 0 || y < 0 || x >= cw || y >= ch {
            return None;
        }
        self.components().iter().rposition(|c| c.visual && x >= c.x && x <= c.x + c.w && y >= c.y && y <= c.y + c.h)
    }

    /// The form's corner grip (the size's), client coordinates.
    fn corner_rect(&self) -> Rect {
        let (ix, iy) = self.client_inset();
        let (_, _, fw, fh) = self.form_rect();
        (fw - ix - self.reach(3), fh - iy - self.reach(3), self.reach(CORNER + 3), self.reach(CORNER + 3))
    }

    /// The form's own sizing grip at (x, y): its corner, its right edge or
    /// its bottom edge (a few pixels either side of its frame), as a
    /// window's border is dragged.
    pub fn form_grip_at(&self, x: i64, y: i64) -> Option<Grip> {
        if !self.show_selection || self.no_form() || self.fit {
            return None;
        }
        let (ix, iy) = self.client_inset();
        let (_, _, fw, fh) = self.form_rect();
        let (left, top, right, bottom) = (-ix, -iy, fw - ix, fh - iy);
        let (cx, cy, cw, ch) = self.corner_rect();
        if x >= cx && y >= cy && x < cx + cw && y < cy + ch {
            return Some(Grip::FormCorner);
        }
        let (inn, out) = (self.reach(EDGE_IN), self.reach(EDGE_OUT));
        let near = |v: i64, edge: i64| v >= edge - inn && v <= edge + out;
        match (near(x, right) && y >= top && y <= bottom, near(y, bottom) && x >= left && x <= right) {
            (true, true) => Some(Grip::FormCorner),
            (true, false) => Some(Grip::FormRight),
            (false, true) => Some(Grip::FormBottom),
            _ => None,
        }
    }

    /// The pointer for the mouse at (x, y) (client coordinates): the
    /// handles' and the form's edges' sizing arrows, the placing tool's
    /// cross, a selected component's move arrows.
    pub fn pointer_at(&self, x: i64, y: i64) -> Pointer {
        if let Some(g) = self.drag.as_ref().map(|d| d.grip) {
            return grip_pointer(g);
        }
        if !self.place_type.is_empty() && !self.no_form() {
            return Pointer::Cross;
        }
        if let Some(g) = self.grip_at(x, y).or_else(|| self.form_grip_at(x, y)) {
            return grip_pointer(g);
        }
        match self.component_at(x, y).and_then(|i| self.ids().get(i).copied()) {
            Some(id) if self.designer.selection.contains(id) && self.on_form(id) => Pointer::Move,
            _ => Pointer::Default,
        }
    }

    /// The mouse pressed at (x, y) of the form's client area (`double`: the
    /// second press of a double click; `add`: Shift / Ctrl held): with the
    /// placing tool armed, a new component's rectangle begins; else an
    /// anchor pin toggled, a handle or the form's edge grabbed (nothing
    /// heard), a component selected (OnSelect / OnDblClick; with `add`, in
    /// or out of the selection), or the background (nothing selected,
    /// OnBgClick, a rubber band).
    pub fn mouse_down_with(&mut self, x: i64, y: i64, double: bool, add: bool) -> Option<DesignEvent> {
        self.mouse_down_keys(x, y, double, add, add)
    }

    /// The same, saying whether Shift is held (`shift`: the placing tool
    /// stays armed for another, as in Delphi's designer; Ctrl / ⌘ don't).
    pub fn mouse_down_keys(&mut self, x: i64, y: i64, double: bool, add: bool, shift: bool) -> Option<DesignEvent> {
        self.guides.clear();
        self.typing = None;
        if self.no_form() || self.read_only() {
            return None;
        }
        // (the in-place editor: a double click's second press drops what a
        // slow click began; a press in it stays in it; elsewhere it writes)
        if double && self.editing.is_some() {
            self.editing = None;
        }
        if self.editing.is_some() && self.edit_press(x, y) {
            return None;
        }
        // (the Tab-order editor takes every click)
        if self.tab_order.is_some() {
            self.tab_order_click(x, y);
            return None;
        }
        let (cw, ch) = self.client_size();
        let inside = x >= 0 && y >= 0 && x < cw && y < ch;
        if !self.place_type.is_empty() && inside {
            let (sx, sy) = (self.snap_point(x), self.snap_point(y));
            self.start_drag(Grip::Place, sx, sy);
            if let Some(d) = &mut self.drag {
                d.keep = shift;
            }
            self.ghost = Some((LRect::new(sx, sy, 0, 0), self.place_type.clone()));
            return None;
        }
        // (the menu editor: the bar, the open menus, their Type Here)
        if let Some(e) = self.menu_press(x, y, double, add) {
            return e;
        }
        if let Some(side) = self.pin_at(x, y) {
            let _ = self.designer.toggle_anchor(side);
            self.commit();
            if let Some(id) = self.designer.selection.primary() {
                let on = self.designer.anchors(id) & side.bit() != 0;
                self.say(format!("{} anchor {}", side_name(side), if on { "on" } else { "off" }));
            }
            return None;
        }
        if let Some(grip) = self.grip_at(x, y) {
            self.start_drag(grip, x, y);
            return None;
        }
        if let Some(grip) = self.form_grip_at(x, y) {
            if self.component_at(x, y).is_none() || grip == Grip::FormCorner {
                self.start_drag(grip, x, y);
                return None;
            }
        }
        match self.component_at(x, y) {
            // (a component the program creates outside the form, in the tray)
            Some(i) if i >= self.ids().len() => {
                self.select_outside(i - self.ids().len());
                self.selected_raw = i as i64;
                self.start_drag(Grip::Tray, x, y);
                Some(DesignEvent::Select(i as i64))
            }
            Some(i) => {
                let id = self.ids()[i];
                self.outside_sel = None;
                let was = self.designer.selection.ids().to_vec();
                // (pressed again alone and let go without a move: a slow
                // click, the caption edited in place)
                self.slow = (!add && !double && was == [id] && self.on_form(id)).then_some(id);
                if add {
                    self.designer.selection.toggle(id);
                } else if !self.designer.selection.contains(id) {
                    self.designer.selection.set(id);
                } else {
                    // (pressed in the selection: that one leads the drag)
                    let rest: Vec<NodeId> = self.designer.selection.ids().iter().copied().filter(|&o| o != id).collect();
                    self.designer.selection.set_all(std::iter::once(id).chain(rest).collect());
                }
                self.selected_raw = i as i64;
                if self.designer.selection.ids() != was.as_slice() {
                    let text = self.describe_comp(id);
                    self.say(text);
                }
                if self.designer.selection.contains(id) {
                    self.start_drag(if self.on_form(id) { Grip::Move } else { Grip::Tray }, x, y);
                }
                Some(if double { DesignEvent::DblClick(i) } else { DesignEvent::Select(i as i64) })
            }
            None => {
                self.outside_sel = None;
                if !add {
                    self.designer.selection.clear();
                    let text = format!("{} ({}), nothing selected", self.root_name(), self.designer.design.node(self.designer.design.root()).map(|n| n.type_written.clone()).unwrap_or_default());
                    self.say(text);
                }
                self.selected_raw = -1;
                self.start_drag(Grip::Band, x, y);
                Some(DesignEvent::BgClick(x, y))
            }
        }
    }

    /// A plain press (no Shift / Ctrl).
    pub fn mouse_down(&mut self, x: i64, y: i64, double: bool) -> Option<DesignEvent> {
        self.mouse_down_with(x, y, double, false)
    }

    fn start_drag(&mut self, grip: Grip, x: i64, y: i64) {
        let start: Vec<(NodeId, LRect)> = if grip.moves() { self.designer.selection.ids().iter().filter(|&&id| self.on_form(id)).filter_map(|&id| self.rect_of(id).map(|r| (id, r))).collect() } else { Vec::new() };
        let offset = if grip.form() {
            let (_, _, w, h) = self.form_rect();
            (w - x, h - y)
        } else {
            start.first().map_or((0, 0), |(_, r)| (x - r.left, y - r.top))
        };
        self.drag = Some(Drag { grip, from: (x, y), offset, now: start.clone(), start, moved: false, keep: false, last: (x, y) });
    }

    /// The siblings the dragged components line up with: the primary's
    /// parent's other children (form coordinates).
    fn targets(&self, primary: NodeId) -> (Vec<Target>, (i64, i64), (i64, i64)) {
        let d = &self.designer.design;
        let l = self.layout();
        let parent = d.parent(primary).unwrap_or(d.root());
        let origin = if parent == d.root() { (0, 0) } else { self.rect_of(parent).map_or((0, 0), |r| (r.left, r.top)) };
        let area = l.client_of(parent).map_or(self.client_size(), |c| (c.width, c.height));
        let targets = d
            .children(parent)
            .into_iter()
            .filter(|c| !self.designer.selection.contains(*c) && self.on_form(*c))
            .filter_map(|c| {
                let r = l.rect(c)?;
                let n = d.node(c)?;
                Some(Target { rect: r, baseline: snap::baseline(&n.canonical, r.height, &l.font(c)) })
            })
            .collect();
        (targets, area, origin)
    }

    /// The mouse dragged to (x, y) (`free`: Alt / Option held, snapping
    /// off): the selection moves (or the grabbed handle resizes it) with
    /// the grid and the guides — OnMove at every step; the form's edge or
    /// corner resizes the form (its components where the running program
    /// puts them); a rubber band selects what it touches; the placing tool
    /// draws the new component's rectangle.
    pub fn mouse_drag_with(&mut self, x: i64, y: i64, free: bool) -> Option<DesignEvent> {
        let mut drag = self.drag.take()?;
        let out = self.drag_to(&mut drag, x, y, free);
        self.drag = Some(drag);
        out
    }

    pub fn mouse_drag(&mut self, x: i64, y: i64) -> Option<DesignEvent> {
        self.mouse_drag_with(x, y, false)
    }

    /// The mouse moved over the surface with no button down (`free`: Alt):
    /// the placing tool's ghost follows it.
    pub fn mouse_hover(&mut self, x: i64, y: i64, free: bool) {
        self.pointer = Some((x, y));
        // (a component dragged in: the kernel's drop routing shows its ghost)
        if drop_pending().is_some() {
            return;
        }
        if self.place_type.is_empty() || self.no_form() {
            self.ghost = None;
            return;
        }
        let (cw, ch) = self.client_size();
        self.ghost = (x >= 0 && y >= 0 && x < cw && y < ch).then(|| (self.new_rect(&self.place_type, x, y, free), self.place_type.clone()));
    }

    /// The mouse left the surface.
    pub fn mouse_leave(&mut self) {
        if self.drag.is_none() {
            self.ghost = None;
        }
    }

    fn drag_to(&mut self, drag: &mut Drag, x: i64, y: i64, free: bool) -> Option<DesignEvent> {
        drag.last = (x, y);
        self.drop_target = None;
        match drag.grip {
            Grip::Tray => None,
            Grip::MenuItem => {
                self.menu_drag(drag, x, y);
                None
            }
            Grip::FormCorner | Grip::FormRight | Grip::FormBottom => {
                let (_, _, fw, fh) = self.form_rect_designed();
                let w = if drag.grip == Grip::FormBottom { fw } else { (x + drag.offset.0).max(MIN_FORM) };
                let h = if drag.grip == Grip::FormRight { fh } else { (y + drag.offset.1).max(MIN_FORM) };
                drag.moved = (w, h) != (fw, fh) || drag.moved;
                self.preview = Some((w, h));
                // (the size the form's Constraints allow)
                let (w, h) = self.layout().form_size();
                self.say(format!("{} × {}", w, h));
                None
            }
            Grip::Place => {
                let (x0, y0) = drag.from;
                let (x1, y1) = if free { (x, y) } else { (self.snap_point(x), self.snap_point(y)) };
                let r = LRect::new(x0.min(x1), y0.min(y1), (x1 - x0).abs(), (y1 - y0).abs());
                drag.moved = r.width >= 4 || r.height >= 4;
                self.ghost = Some((if drag.moved { r } else { self.new_rect(&self.place_type, x0, y0, true) }, self.place_type.clone()));
                None
            }
            Grip::Band => {
                let (x0, y0) = drag.from;
                let band = LRect::new(x0.min(x), y0.min(y), (x - x0).abs(), (y - y0).abs());
                self.band = Some(band);
                let touched: Vec<NodeId> = self
                    .ids()
                    .into_iter()
                    .filter(|&id| self.designer.design.parent(id) == Some(self.designer.design.root()) && self.on_form(id))
                    .filter(|&id| self.rect_of(id).is_some_and(|r| r.left < band.left + band.width && band.left < r.left + r.width && r.top < band.top + band.height && band.top < r.top + r.height))
                    .collect();
                self.designer.selection.set_all(touched);
                drag.moved = true;
                None
            }
            grip => {
                let (&(primary, r0), _) = (drag.start.first()?, ());
                let (targets, area, origin) = self.targets(primary);
                let snapper = Snapper { snap_to_guides: self.show_guides, ..self.designer.snapper };
                let local = |r: LRect| LRect::new(r.left - origin.0, r.top - origin.1, r.width, r.height);
                let snapped = if grip == Grip::Move {
                    let proposed = LRect::new(x - drag.offset.0, y - drag.offset.1, r0.width, r0.height);
                    let n = self.designer.design.node(primary)?;
                    let baseline = snap::baseline(&n.canonical, r0.height, &self.laid_out().font(primary));
                    snapper.snap_move(local(proposed), baseline, &targets, area, free)
                } else {
                    let (dx, dy) = (x - drag.from.0, y - drag.from.1);
                    let e = grip.edges();
                    let mut r = r0;
                    if e.left {
                        r.left += dx;
                        r.width -= dx;
                    }
                    if e.right {
                        r.width += dx;
                    }
                    if e.top {
                        r.top += dy;
                        r.height -= dy;
                    }
                    if e.bottom {
                        r.height += dy;
                    }
                    let mut s = snapper.snap_resize(local(r), e, &targets, area, free);
                    // (never smaller than the designer's least)
                    if s.rect.width < MIN_SIZE {
                        if e.left {
                            s.rect.left -= MIN_SIZE - s.rect.width;
                        }
                        s.rect.width = MIN_SIZE;
                    }
                    if s.rect.height < MIN_SIZE {
                        if e.top {
                            s.rect.top -= MIN_SIZE - s.rect.height;
                        }
                        s.rect.height = MIN_SIZE;
                    }
                    s
                };
                let to = LRect::new(snapped.rect.left + origin.0, snapped.rect.top + origin.1, snapped.rect.width, snapped.rect.height);
                self.guides = snapped.guides.iter().map(|g| shift_guide(*g, origin)).collect();
                if grip == Grip::Move {
                    let (dx, dy) = (to.left - r0.left, to.top - r0.top);
                    drag.now = drag.start.iter().map(|&(id, r)| (id, LRect::new(r.left + dx, r.top + dy, r.width, r.height))).collect();
                    // (over another container: it's where they'll go)
                    let moving: Vec<NodeId> = drag.start.iter().map(|(id, _)| *id).collect();
                    let (target, _) = self.container_at_except(x, y, &moving);
                    if Some(target) != self.designer.design.parent(primary) {
                        self.drop_target = Some(target);
                    }
                } else {
                    drag.now = drag.start.iter().map(|&(id, r)| (id, if id == primary { to } else { r })).collect();
                }
                drag.moved = drag.now != drag.start;
                let index = self.index_of(primary)?;
                // (a press without a move says nothing)
                if !drag.moved {
                    return None;
                }
                let local = LRect::new(to.left - origin.0, to.top - origin.1, to.width, to.height);
                self.say(format!("{}, {}, {} × {}", local.left, local.top, local.width, local.height));
                Some(DesignEvent::Move { index, x: to.left, y: to.top, w: to.width, h: to.height })
            }
        }
    }

    /// The mouse let go: a move or resize is written (one undo step), the
    /// form's new size written (its anchored components where the resize
    /// put them), a placed component added; the guides go.
    pub fn mouse_up(&mut self) {
        self.guides.clear();
        self.band = None;
        self.drop_target = None;
        let Some(drag) = self.drag.take() else { return };
        match drag.grip {
            g if g.form() => {
                let size = self.preview.take();
                if let (Some((w, h)), true) = (size, drag.moved) {
                    self.resize_form(w, h);
                }
            }
            Grip::Place => {
                let ty = self.place_type.clone();
                let rect = self.ghost.take().map(|(r, _)| r).filter(|_| drag.moved);
                if !drag.keep {
                    self.place_type.clear();
                }
                self.add_dropped(&ty, drag.from, rect);
            }
            Grip::Band => {
                if drag.moved {
                    self.selection_changed();
                }
            }
            Grip::MenuItem => self.menu_release(drag),
            Grip::Move if !drag.moved => {
                if self.slow.take().is_some_and(|s| self.designer.selection.ids() == [s]) {
                    self.begin_edit();
                }
            }
            g if g.moves() && drag.moved => {
                let l = self.designer.layout();
                let d = &self.designer.design;
                let mut cmds = Vec::new();
                // (dropped on another container: they go into it — the
                // CREATE blocks move — placed where they were let go)
                let moving: Vec<NodeId> = drag.start.iter().map(|(id, _)| *id).collect();
                let into = (g == Grip::Move).then(|| self.container_at_except(drag.last.0, drag.last.1, &moving));
                for (id, abs) in &drag.now {
                    let (Some(old), o) = (l.rect(*id), l.origin(d, *id)) else { continue };
                    match into {
                        Some((target, (tx, ty))) if Some(target) != d.parent(*id) && !moving.iter().any(|&m| m != *id && d.is_within(*id, m)) => {
                            cmds.push(Command::Move { node: *id, parent: target, index: usize::MAX });
                            // (its place in the new parent: every value written)
                            let new = LRect::new(abs.left - tx, abs.top - ty, abs.width, abs.height);
                            for (p, v) in [("Left", new.left), ("Top", new.top)] {
                                cmds.push(Command::SetProp { node: *id, name: p.into(), value: Some(v.to_string()) });
                            }
                        }
                        _ => cmds.extend(arrange::set_rect(*id, old, LRect::new(abs.left - o.0, abs.top - o.1, abs.width, abs.height))),
                    }
                }
                let _ = self.designer.execute(Command::Batch(cmds));
                self.commit();
                if let Some(id) = self.designer.selection.primary() {
                    let text = self.describe_comp(id);
                    self.say(text);
                }
            }
            _ => {}
        }
    }

    /// The form's size as designed (not the preview's).
    fn form_rect_designed(&self) -> Rect {
        let m = self.margin();
        let (w, h) = self.laid_out().form_size();
        (m - self.scroll.0, m - self.scroll.1, w, h)
    }

    /// The form resized to `w` × `h` (its Width / Height, frame included;
    /// its Constraints obeyed), one undo step: the new size written into
    /// its CREATE block (ClientWidth / ClientHeight where the block sets
    /// those instead), and each component its Anchors moved written where
    /// the resize put it — so the text says what the running program shows
    /// at that size.
    pub fn resize_form(&mut self, w: i64, h: i64) -> bool {
        let base = self.laid_out();
        let (ow, oh) = base.form_size();
        let mut preview = (*base).clone();
        preview.resize(w, h);
        let (w, h) = preview.form_size();
        if (w, h) == (ow, oh) {
            return false;
        }
        let d = &self.designer.design;
        let root = d.root();
        let Some(rn) = d.node(root) else { return false };
        let mut cmds = Vec::new();
        for (p, client, new, old) in [("Width", "ClientWidth", w, ow), ("Height", "ClientHeight", h, oh)] {
            if new == old {
                continue;
            }
            match (rn.prop(p), rn.int(client)) {
                (None, Some(c)) => cmds.push(Command::SetProp { node: root, name: client.into(), value: Some((c + new - old).to_string()) }),
                _ => cmds.push(Command::SetProp { node: root, name: p.into(), value: Some(new.to_string()) }),
            }
        }
        // (what the components would be with only the size written: those
        // the preview put elsewhere are written where it put them)
        let mut scratch = d.clone();
        for c in &cmds {
            let _ = c.apply(&mut scratch);
        }
        let after = Layout::of(&scratch);
        for (id, r) in preview.rects() {
            if id == root {
                continue;
            }
            let Some(n) = d.node(id) else { continue };
            let aligned = n.int("Align").unwrap_or_else(|| crate::layout::default_align(&n.canonical) as i64) != 0;
            if let Some(a) = after.rect(id).filter(|a| *a != r && !aligned) {
                cmds.extend(arrange::set_rect(id, a, r));
            }
        }
        let ok = self.designer.execute(Command::Batch(cmds)).is_ok();
        self.commit();
        self.say(format!("{} resized to {} × {}", self.root_name(), w, h));
        ok
    }

    /// A drag is in progress.
    pub fn dragging(&self) -> bool {
        self.drag.as_ref().is_some_and(|g| g.moved)
    }

    /// A property (`None`: not the surface's; the runtime keeps it).
    pub fn get(&self, prop: &str) -> Option<Value> {
        Some(match prop {
            "compcount" | "count" => v_int(self.ids().len() as i64),
            "formcaption" => v_str(&self.form_caption),
            "selcount" => v_int(self.designer.selection.len() as i64),
            "selindex" => v_int(self.selection().map_or(-1, |i| i as i64)),
            "previewwidth" => v_int(self.preview.map_or(0, |p| p.0)),
            "previewheight" => v_int(self.preview.map_or(0, |p| p.1)),
            "showguides" => Value::Boolean(self.show_guides),
            "showgrid" => Value::Boolean(self.show_grid),
            "showselection" => Value::Boolean(self.show_selection),
            "snaptogrid" => Value::Boolean(self.designer.snapper.snap_to_grid),
            "gridsize" => v_int(self.designer.snapper.grid),
            "source" => v_str(&self.source.as_ref().map(|a| a.doc.borrow().text()).unwrap_or_default()),
            "sourcefile" => v_str(&self.source_file),
            "formname" => v_str(&self.root_name()),
            "placetype" => v_str(&self.place_type),
            "canundo" => Value::Boolean(self.can_undo()),
            "canredo" => Value::Boolean(self.can_redo()),
            "statustext" => v_str(&self.announcement),
            "handlerline" => v_int(self.handler_line),
            "zoom" => v_int((self.zoom * 100.0).round() as i64),
            "tabordermode" => Value::Boolean(self.tab_order.is_some()),
            "sharedundo" => Value::Boolean(self.shared_undo),
            "editing" => Value::Boolean(self.editing.is_some()),
            _ => return None,
        })
    }

    pub fn set(&mut self, prop: &str, val: &Value) -> bool {
        match prop {
            "formcaption" => self.form_caption = val.to_string_val(),
            // (0 ends the preview)
            "previewwidth" | "previewheight" => {
                let v = val.to_i64();
                let (_, _, fw, fh) = self.form_rect();
                let (w, h) = self.preview.unwrap_or((fw, fh));
                self.preview = if v <= 0 { None } else if prop == "previewwidth" { Some((v, h)) } else { Some((w, v)) };
            }
            "showguides" => self.show_guides = val.to_bool(),
            "showgrid" => self.show_grid = val.to_bool(),
            "showselection" => self.show_selection = val.to_bool(),
            "snaptogrid" => self.designer.snapper.snap_to_grid = val.to_bool(),
            "gridsize" => self.designer.snapper.grid = val.to_i64().max(1),
            "source" => {
                self.open_source(&val.to_string_val());
            }
            "sourcefile" => self.source_file = val.to_string_val(),
            "formname" => {
                self.want_form = val.to_string_val();
                self.pick_form(true);
            }
            "placetype" => {
                self.place_type = val.to_string_val().trim().to_string();
                self.ghost = None;
                if !self.place_type.is_empty() {
                    let t = self.place_type.clone();
                    self.say(format!("Placing {t}: click or draw on the form, Escape to stop"));
                }
            }
            "zoom" => {
                self.set_zoom(val.to_i64() as f64 / 100.0, None);
            }
            "tabordermode" => self.set_tab_order_mode(val.to_bool()),
            "sharedundo" => self.shared_undo = val.to_bool(),
            "selindex" => {
                self.selected_raw = val.to_i64();
                match self.id_of(val.to_i64()) {
                    Some(id) => self.designer.selection.set(id),
                    None => self.designer.selection.clear(),
                }
            }
            _ => return false,
        }
        true
    }

    /// Where a component added by keyboard (Enter on a toolbox item,
    /// AddComponent with no place) goes: the selection's container (the
    /// selection itself when it is one), at the first free step of a
    /// cascade from its top left (client coordinates).
    pub fn free_spot(&self, type_name: &str) -> (i64, i64) {
        let d = &self.designer.design;
        let canonical = crate::designer::model::canonical_type(type_name);
        let (dw, dh) = crate::layout::default_size(&canonical).unwrap_or((0, 0));
        let container = match self.designer.selection.primary() {
            Some(p) if d.node(p).is_some_and(|n| n.is_container() && !n.is_form()) => p,
            Some(p) => d.parent(p).unwrap_or(d.root()),
            None => d.root(),
        };
        let (ox, oy) = if container == d.root() { (0, 0) } else { self.rect_of(container).map_or((0, 0), |r| (r.left, r.top)) };
        let area = self.layout().client_of(container).map_or(self.client_size(), |c| (c.width, c.height));
        let g = self.designer.snapper.grid.max(1);
        let taken: Vec<(i64, i64)> = d.children(container).into_iter().filter_map(|c| self.layout().rect(c).map(|r| (r.left, r.top))).collect();
        let step = (2 * g).max(8);
        let spot = (0..64)
            .map(|k| (g + k * step, g + k * step))
            .find(|&(x, y)| !taken.contains(&(x, y)) && (x + dw <= area.0 || x == g) && (y + dh <= area.1 || y == g))
            .unwrap_or((g, g));
        (ox + spot.0, oy + spot.1)
    }

    /// Selects the component named `name` (any case) alone; whether there
    /// is one.
    pub fn select_name(&mut self, name: &str) -> bool {
        if let Some(k) = self.outside.iter().position(|o| o.name.eq_ignore_ascii_case(name)) {
            return self.select_outside(k);
        }
        match self.designer.design.find(name).filter(|&id| id != self.designer.design.root()) {
            Some(id) => {
                self.outside_sel = None;
                self.designer.selection.set(id);
                self.selected_raw = self.index_of(id).map_or(-1, |i| i as i64);
                let text = self.describe_comp(id);
                self.say(text);
                true
            }
            None => false,
        }
    }

    /// The Format menu's commands on the selection (one undo step each):
    /// left, center, right, top, middle, bottom (to the first selected),
    /// samewidth, sameheight, samesize, spaceh, spacev (distribute),
    /// centerh, centerv (in the parent), front, back.
    pub fn arrange(&mut self, how: &str) -> bool {
        use arrange::Axis;
        let how = how.trim().to_ascii_lowercase();
        let r = match how.as_str() {
            "samewidth" => self.designer.same_size(Axis::Horizontal),
            "sameheight" => self.designer.same_size(Axis::Vertical),
            "samesize" => self.designer.same_size(Axis::Both),
            "spaceh" | "distributeh" => self.designer.distribute(Axis::Horizontal),
            "spacev" | "distributev" => self.designer.distribute(Axis::Vertical),
            "centerh" => self.designer.center_in_parent(Axis::Horizontal),
            "centerv" => self.designer.center_in_parent(Axis::Vertical),
            "front" => self.designer.bring_to_front(),
            "back" => self.designer.send_to_back(),
            other => match arrange::AlignHow::parse(other) {
                Some(a) => self.designer.align(a),
                None => return false,
            },
        };
        let changed = self.designer.has_applied();
        self.commit();
        if changed {
            self.say(format!("Arranged: {how}"));
        }
        r.is_ok()
    }

    /// The selection's CREATE blocks as text (Copy / Cut put it on the
    /// clipboard: pasted into the code, it is the source).
    pub fn copy(&mut self) -> Option<String> {
        if self.designer.selection.is_empty() {
            return None;
        }
        let clip = self.designer.copy();
        let text = clip.text.clone();
        self.clip = Some(clip);
        Some(text)
    }

    pub fn cut(&mut self) -> Option<String> {
        let text = self.copy()?;
        self.delete_selection();
        Some(text)
    }

    /// Paste: Copy's components again (names kept unique, handlers
    /// unbound), into the selected container or the selection's parent.
    pub fn paste(&mut self) -> bool {
        let Some(clip) = self.clip.clone() else { return false };
        let ok = self.designer.paste(&clip).is_ok();
        self.commit();
        self.selection_changed();
        ok
    }

    pub fn duplicate(&mut self) -> bool {
        if self.designer.selection.is_empty() {
            return false;
        }
        let ok = self.designer.duplicate().is_ok();
        self.commit();
        self.selection_changed();
        ok
    }

    /// Selects every component on the form.
    pub fn select_all(&mut self) {
        let d = &self.designer.design;
        let all = d.children(d.root()).into_iter().filter(|&c| d.node(c).is_some_and(|n| !matches!(n.canonical.as_str(), "RMAINMENU" | "RMENUITEM"))).collect();
        self.designer.selection.set_all(all);
        self.selection_changed();
    }

    /// A key on the surface (it has the focus): whether it was the
    /// designer's. Arrows nudge the selection a pixel (Shift: by the grid;
    /// Ctrl / ⌘: resize instead), Delete / Backspace delete it, Tab /
    /// Shift+Tab select the next / previous component, Escape stops the
    /// placing tool or a drag, else selects the parent, Enter is a double
    /// click (the default event's handler), Ctrl / ⌘ + Z / Y / Shift+Z undo
    /// and redo, + A selects all, + D duplicates, + V pastes (Copy and Cut
    /// are [`DesignSurface::copy`] / [`DesignSurface::cut`]: the host puts
    /// their text on the clipboard).
    pub fn key(&mut self, vk: i64, text: &str, shift: bool, ctrl: bool) -> bool {
        self.key_alt(vk, text, shift, ctrl, false)
    }

    /// The same with Alt / Option's state: the in-place editor first (a
    /// ShortCut field takes Alt+ keys), the Tab-order editor, zoom (Ctrl /
    /// ⌘ + = − 0), F2, the menu editor's keys, then the designer's.
    pub fn key_alt(&mut self, vk: i64, text: &str, shift: bool, ctrl: bool, alt: bool) -> bool {
        // (no form: Enter adds one, as the empty state says)
        if self.no_form() && vk == 13 && !ctrl && !alt {
            return self.add_form("").is_some();
        }
        if self.no_form() || self.read_only() {
            return false;
        }
        if self.editing.is_some() {
            return self.edit_key(vk, text, shift, ctrl, alt);
        }
        if alt {
            return false;
        }
        if self.tab_order.is_some() {
            if vk == 27 {
                self.set_tab_order_mode(false);
                return true;
            }
            return matches!(vk, 9 | 13 | 37..=40 | 46 | 8);
        }
        if ctrl {
            let dir = match vk {
                187 | 107 => Some(1),
                189 | 109 => Some(-1),
                48 | 96 => Some(0),
                _ => None,
            };
            if let Some(dir) = dir {
                self.zoom_step(dir, None);
                return true;
            }
        }
        if vk == 113 && !ctrl {
            return self.begin_edit();
        }
        if vk == 27 && !self.menu_open.is_empty() && self.drag.is_none() {
            self.menu_open.clear();
            self.say("Menu closed");
            return true;
        }
        if !matches!(vk, 46 | 8 | 13 | 27) && self.menu_key(vk, text, ctrl) {
            return true;
        }
        // (right after adding a component: what's typed is its Caption /
        // Text; Backspace takes a character back, Enter or Escape ends it)
        if let Some((id, prop, mut typed)) = self.typing.take().filter(|(id, _, _)| self.designer.selection.primary() == Some(*id)) {
            let printable = !ctrl && !text.is_empty() && text.chars().all(|c| !c.is_control());
            if printable || (vk == 8 && !typed.is_empty()) {
                if vk == 8 {
                    typed.pop();
                } else {
                    typed.push_str(text);
                }
                let _ = self.designer.execute(Command::SetProp { node: id, name: prop.to_string(), value: Some(value::write_str(&typed)) });
                // (one undo step with the add: Delphi's)
                self.continues = true;
                self.commit();
                self.say(format!("{prop}: {typed}"));
                self.typing = Some((id, prop, typed));
                return true;
            }
            if vk != 13 && vk != 27 {
                self.typing = Some((id, prop, typed));
            } else {
                return true;
            }
        }
        let d = &self.designer.design;
        match vk {
            // Escape
            27 => {
                if self.drag.is_some() {
                    self.forget_gestures();
                    self.say("Cancelled");
                } else if !self.place_type.is_empty() {
                    self.place_type.clear();
                    self.ghost = None;
                    self.say("Placing stopped");
                } else if let Some(p) = self.designer.selection.primary() {
                    match d.parent(p).filter(|&q| q != d.root()) {
                        Some(q) => self.designer.selection.set(q),
                        None => self.designer.selection.clear(),
                    }
                    self.selection_changed();
                } else {
                    return false;
                }
                true
            }
            // the arrows
            37..=40 => {
                if self.designer.selection.is_empty() {
                    return false;
                }
                let step = if shift { self.designer.snapper.grid.max(1) } else { 1 };
                let (dx, dy) = match vk {
                    37 => (-step, 0),
                    38 => (0, -step),
                    39 => (step, 0),
                    _ => (0, step),
                };
                let _ = self.designer.nudge(dx, dy, ctrl);
                self.commit();
                if let Some(id) = self.designer.selection.primary() {
                    let text = self.describe_comp(id);
                    self.say(text);
                }
                true
            }
            // Delete, Backspace
            46 | 8 => self.delete_selection(),
            // Tab: the next (Shift: previous) component
            9 => {
                let ids = self.ids();
                if ids.is_empty() {
                    return false;
                }
                let at = self.selection();
                let n = ids.len();
                let next = match (at, shift) {
                    (None, false) => 0,
                    (None, true) => n - 1,
                    (Some(i), false) => (i + 1) % n,
                    (Some(i), true) => (i + n - 1) % n,
                };
                self.designer.selection.set(ids[next]);
                self.selection_changed();
                true
            }
            // Enter: the selection's default event (as a double click)
            13 => match self.selection() {
                Some(i) => {
                    self.outbox.push(DesignEvent::DblClick(i));
                    true
                }
                None => false,
            },
            _ if ctrl => match vk {
                90 if shift => self.redo() || true,
                90 => self.undo() || true,
                89 => self.redo() || true,
                65 => {
                    self.select_all();
                    true
                }
                68 => self.duplicate() || true,
                86 => self.paste() || true,
                _ => false,
            },
            _ => false,
        }
    }

    /// Its methods (`None`: not one of them; Show and Hide are the host's).
    pub fn call(&mut self, method: &str, args: &[Value]) -> Option<Value> {
        let int = |i: usize, default: i64| args.get(i).map_or(default, Value::to_i64);
        let text = |i: usize| args.get(i).map(Value::to_string_val).unwrap_or_default();
        let index = int(0, -1);
        let comp = |s: &Self| s.id_of(index).and_then(|id| s.designer.design.node(id).cloned());
        let rect = |s: &Self| s.id_of(index).and_then(|id| s.rect_of(id));
        Some(match method {
            // AddComponent(Type, X, Y): a new component at (X, Y) (-1, -1:
            // a free spot in the selection's container), its index (-1:
            // it can't go there). AddComponent(Type, Name, X, Y, W, H):
            // the API's form.
            "addcomponent" if args.len() <= 3 => {
                let ty = text(0);
                let (x, y) = (int(1, -1), int(2, -1));
                let at = if x < 0 || y < 0 { self.free_spot(&ty) } else { (x, y) };
                v_int(self.add_at(&ty, at, None).map_or(-1, |i| i as i64))
            }
            "addcomponent" => {
                self.add(&text(0), &text(1), (int(2, 0), int(3, 0), int(4, 80), int(5, 25)));
                Value::Null
            }
            // DragComponent(Type): a new one dragged in from elsewhere
            "dragcomponent" => {
                begin_drop(&text(0));
                Value::Null
            }
            "getname" | "gettype" if index >= self.ids().len() as i64 => {
                let o = self.outside.get((index - self.ids().len() as i64) as usize);
                v_str(&o.map_or(String::new(), |o| if method == "getname" { o.name.clone() } else { o.type_name.clone() }))
            }
            "getname" => v_str(&comp(self).map_or(String::new(), |c| c.name)),
            "gettype" => v_str(&comp(self).map_or(String::new(), |c| c.type_written)),
            // EditCaption: F2's — the selection's caption edited in place
            "editcaption" => Value::Boolean(self.begin_edit()),
            // AddForm(Name): a form for a file without one
            "addform" => v_str(&self.add_form(&text(0)).unwrap_or_default()),
            "getcompx" => v_int(rect(self).map_or(0, |r| r.left)),
            "getcompy" => v_int(rect(self).map_or(0, |r| r.top)),
            "getcompw" => v_int(rect(self).map_or(0, |r| r.width)),
            "getcomph" => v_int(rect(self).map_or(0, |r| r.height)),
            // SetProp(Index, Name, Value) / GetProp(Index, Name)
            "setprop" => {
                if let Some(c) = comp(self) {
                    let value = source_value(&c.canonical, &text(1), &text(2));
                    let _ = self.designer.execute(Command::SetProp { node: c.id, name: text(1), value: Some(value) });
                    self.commit();
                }
                Value::Null
            }
            "getprop" => v_str(&comp(self).and_then(|c| c.prop(&text(1)).map(plain_value)).unwrap_or_default()),
            // SetCompBounds(Index, X, Y, W, H)
            "setcompbounds" => {
                if let (Some(id), Some(old)) = (self.id_of(index), self.id_of(index).and_then(|id| self.designer.layout().rect(id))) {
                    let new = LRect::new(int(1, 0), int(2, 0), int(3, 80), int(4, 25));
                    let _ = self.designer.execute(Command::Batch(arrange::set_rect(id, old, new)));
                    self.commit();
                }
                Value::Null
            }
            "setname" => {
                if let Some(id) = self.id_of(index) {
                    let _ = self.designer.rename(id, &text(1));
                    self.commit();
                }
                Value::Null
            }
            "selectcomp" => {
                self.selected_raw = index;
                match self.id_of(index) {
                    Some(id) => self.designer.selection.set(id),
                    None => self.designer.selection.clear(),
                }
                Value::Null
            }
            "selectname" => Value::Boolean(self.select_name(&text(0))),
            "removecomponent" => {
                self.remove(index);
                Value::Null
            }
            "clearall" => {
                self.clear();
                Value::Null
            }
            "count" => v_int(self.ids().len() as i64),
            // (I4: through the source's history when it has one)
            "undo" => Value::Boolean(self.undo()),
            "redo" => Value::Boolean(self.redo()),
            // AlignSelection(How): left, center, right, top, middle, bottom —
            // to the first selected (Align is the surface's own property)
            "alignselection" => Value::Boolean(self.arrange(&text(0))),
            "arrange" => Value::Boolean(self.arrange(&text(0))),
            // SelectAdd(Index): Shift+click's
            "selectadd" => {
                if let Some(id) = self.id_of(index) {
                    self.designer.selection.add(id);
                }
                Value::Null
            }
            "selectall" => {
                self.select_all();
                Value::Null
            }
            "deleteselection" => Value::Boolean(self.delete_selection()),
            "copyselection" => v_str(&self.copy().unwrap_or_default()),
            "cutselection" => v_str(&self.cut().unwrap_or_default()),
            "paste" => Value::Boolean(self.paste()),
            "duplicate" => Value::Boolean(self.duplicate()),
            "resizeform" => Value::Boolean(self.resize_form(int(0, 0), int(1, 0))),
            // CreateHandler(Name, Event): the handler's SUB name ("": none)
            "createhandler" => v_str(&self.create_handler(&text(0), &text(1)).unwrap_or_default()),
            _ => return None,
        })
    }

    // -------------------------------------------------------- drawing --

    /// The grid's dots over the form's client area `w` × `h` (client
    /// coordinates), in a shade of `face` (the form's colour): drawn on the
    /// form's face, under its components.
    pub fn grid_ops(&self, w: i64, h: i64, face: u32) -> Vec<Op> {
        if !self.show_grid {
            return Vec::new();
        }
        let t = crate::theme::current();
        let dark = (face >> 16 & 0xFF) + (face >> 8 & 0xFF) + (face & 0xFF) < 3 * 0x80;
        let dot = shade(face, if dark || t.dark && face == t.face { 40 } else { -55 });
        let step = self.designer.snapper.grid.max(2);
        let mut d = Draw::default();
        for gx in (0..w.max(0)).step_by(step as usize) {
            for gy in (0..h.max(0)).step_by(step as usize) {
                d.fill((gx, gy, 1, 1), dot);
            }
        }
        d.ops
    }

    /// The designer's chrome over the drawn form, from the client area's
    /// origin on the surface, in the surface's pixels (the form's places
    /// zoomed, the marks themselves at their own size): the selection
    /// framed with the primary's eight handles and anchor pins (unless
    /// [`DesignSurface::show_selection`] is off), the guides, the rubber
    /// band, the form's corner grip and the preview's size — in the theme's
    /// tokens.
    pub fn chrome_ops(&self) -> Vec<Op> {
        let t = crate::theme::current();
        let mut d = Draw::default();
        let v = |r: Rect| self.view_rect(r);
        if self.show_selection && self.tab_order.is_none() {
            let comps = self.components();
            let tray = self.tray();
            for (k, &i) in self.selected().iter().enumerate() {
                if let Some(c) = comps.get(i).filter(|c| c.visual) {
                    let r = v(c.bounds());
                    d.rect(r, t.accent);
                    if k == 0 {
                        d.handles(r, t.accent, t.window);
                    } else {
                        d.corners(r, t.accent);
                    }
                } else if let Some(item) = tray.iter().find(|x| x.index == i) {
                    d.rect(v(item.rect), t.accent);
                }
            }
            if self.show_pins && self.drag.as_ref().is_none_or(|g| !g.moved) && self.editing.is_none() {
                if let Some(id) = self.designer.selection.primary().filter(|&id| self.on_form(id)) {
                    if let (Some(i), Some(n)) = (self.index_of(id), self.designer.design.node(id)) {
                        let anchors = n.int("Anchors").unwrap_or(crate::layout::DEFAULT_ANCHORS);
                        if let Some(c) = comps.get(i) {
                            let (x, y, w, h) = v(c.bounds());
                            for (side, p) in pins(LRect::new(x, y, w, h)) {
                                d.pin(p, anchors & side.bit() != 0, side, t.accent, t.window, t.border_strong);
                            }
                        }
                    }
                }
            }
        }
        if self.show_guides {
            for g in &self.guides {
                let shown = Guide { at: self.zv(g.at), from: self.zv(g.from), to: self.zv(g.to), ..*g };
                d.guide(&shown, (g.to - g.from).abs(), t.accent);
            }
        }
        if let Some(b) = self.band {
            d.rect(v((b.left, b.top, b.width, b.height)), t.accent);
        }
        // the container a drag goes into
        if let Some(r) = self.drop_target.and_then(|id| self.rect_of(id)) {
            let (x, y, w, h) = v((r.left, r.top, r.width, r.height));
            d.rect((x - 1, y - 1, w + 2, h + 2), t.accent);
            d.rect((x - 2, y - 2, w + 4, h + 4), t.accent);
        }
        // where a new component would go (the kernel draws the component
        // itself, faded): its outline, its type and size above
        if let Some((r, ty)) = &self.ghost {
            let (x, y, w, h) = v((r.left, r.top, r.width.max(1), r.height.max(1)));
            d.dashed((x, y, w.max(1), h.max(1)), t.accent);
            let label = if r.width > 0 && r.height > 0 { format!("{ty}  {} × {}", r.width, r.height) } else { ty.clone() };
            d.pill(x, y - 20, &label, t.accent, t.window);
        }
        // the live readout of a move or resize
        if let Some(g) = self.drag.as_ref().filter(|g| g.moved && g.grip.moves()) {
            if let Some(&(id, r)) = g.now.first() {
                let o = self.layout().origin(&self.designer.design, id);
                let label = if g.grip == Grip::Move { format!("{}, {}", r.left - o.0, r.top - o.1) } else { format!("{} × {}", r.width, r.height) };
                let (x, y, _, h) = v((r.left, r.top, r.width, r.height));
                d.pill(x, y + h + 8, &label, t.accent, t.window);
            }
        }
        if self.show_selection && !self.no_form() && self.tab_order.is_none() {
            // the form's sizing grips: its corner, the middles of its right
            // and bottom edges
            let (ix, iy) = self.client_inset();
            let (_, _, fw, fh) = self.form_rect();
            let (left, top, w, h) = v((-ix, -iy, fw, fh));
            let (right, bottom) = (left + w, top + h);
            d.corner_grip((right, bottom, CORNER, CORNER), t.accent);
            let form_selected = self.designer.selection.is_empty() && self.outside_sel.is_none();
            let (edge, fill) = if form_selected { (t.accent, t.window) } else { (t.border_strong, t.window) };
            let half = HANDLE / 2;
            for (hx, hy) in [(right + 2, top + h / 2), (left + w / 2, bottom + 2)] {
                d.fill((hx - half, hy - half, HANDLE, HANDLE), edge);
                d.fill((hx - half + 1, hy - half + 1, HANDLE - 2, HANDLE - 2), fill);
            }
            if let Some((pw, ph)) = self.preview {
                d.pill(right - 70, bottom + 10, &format!("{pw} × {ph}"), t.accent, t.window);
            }
        }
        self.tab_order_ops(&mut d.ops);
        if self.show_selection && self.tab_order.is_none() {
            self.menu_ops(&mut d.ops);
        }
        self.edit_ops(&mut d.ops);
        d.ops
    }
}

/// The pointer a grip's drag shows.
fn grip_pointer(g: Grip) -> Pointer {
    match g {
        Grip::Left | Grip::Right | Grip::FormRight => Pointer::SizeWE,
        Grip::Top | Grip::Bottom | Grip::FormBottom => Pointer::SizeNS,
        Grip::TopLeft | Grip::Corner | Grip::FormCorner => Pointer::SizeNWSE,
        Grip::TopRight | Grip::BottomLeft => Pointer::SizeNESW,
        Grip::Move => Pointer::Move,
        Grip::Place => Pointer::Cross,
        Grip::Band | Grip::Tray | Grip::MenuItem => Pointer::Default,
    }
}

fn side_name(s: Side) -> &'static str {
    match s {
        Side::Left => "Left",
        Side::Top => "Top",
        Side::Right => "Right",
        Side::Bottom => "Bottom",
    }
}

/// A guide moved from a container's client coordinates to the form's.
fn shift_guide(g: Guide, (ox, oy): (i64, i64)) -> Guide {
    if g.vertical {
        Guide { at: g.at + ox, from: g.from + oy, to: g.to + oy, ..g }
    } else {
        Guide { at: g.at + oy, from: g.from + ox, to: g.to + ox, ..g }
    }
}

/// The anchor pins of a component at `r`: a small square outside the
/// middle of each side.
fn pins(r: LRect) -> [(Side, LRect); 4] {
    let s = PIN;
    let (cx, cy) = (r.left + r.width / 2, r.top + r.height / 2);
    [
        (Side::Left, LRect::new(r.left - s - 5, cy - s / 2, s, s)),
        (Side::Top, LRect::new(cx - s / 2, r.top - s - 5, s, s)),
        (Side::Right, LRect::new(r.left + r.width + 5, cy - s / 2, s, s)),
        (Side::Bottom, LRect::new(cx - s / 2, r.top + r.height + 5, s, s)),
    ]
}

/// An anchor pin's size.
const PIN: i64 = 9;

/// A colour lighter (`d` > 0) or darker by `d` per channel (saturating).
fn shade(c: u32, d: i32) -> u32 {
    let ch = |s: u32| ((((c >> s) & 0xFF) as i32 + d).clamp(0, 255) as u32) << s;
    ch(16) | ch(8) | ch(0)
}

/// The designer's drawing calls, made as ops.
#[derive(Default)]
struct Draw {
    ops: Vec<Op>,
}

impl Draw {
    fn fill(&mut self, rect: Rect, color: u32) {
        if rect.2 > 0 && rect.3 > 0 {
            self.ops.push(Op::Fill { rect, color });
        }
    }

    /// A one-pixel outline inside `rect`.
    fn rect(&mut self, rect: Rect, color: u32) {
        if rect.2 > 0 && rect.3 > 0 {
            self.ops.push(Op::Edge { rect, light: vec![color], dark: vec![color] });
        }
    }

    /// A line through the pixels from (x0, y0) to (x1, y1), both included.
    fn line(&mut self, (x0, y0): (i64, i64), (x1, y1): (i64, i64), color: u32) {
        self.ops.push(Op::Line { from: (x0 as f64 + 0.5, y0 as f64 + 0.5), to: (x1 as f64 + 0.5, y1 as f64 + 0.5), color });
    }

    fn text(&mut self, rect: Rect, text: &str, font: Font, color: u32, place: Place) {
        if !text.is_empty() {
            self.ops.push(Op::Text { rect, text: text.to_string(), font, color, angle: 0, place });
        }
    }

    /// A filled ellipse inscribed in `rect`.
    fn pie(&mut self, (x, y, w, h): Rect, color: u32) {
        let (cx, cy, rx, ry) = (x as f64 + w as f64 / 2.0, y as f64 + h as f64 / 2.0, w as f64 / 2.0, h as f64 / 2.0);
        let steps = ((rx.max(ry) * 4.0).ceil() as usize).clamp(12, 96);
        let points = (0..steps)
            .map(|k| {
                let a = (360.0 * k as f64 / steps as f64).to_radians();
                (cx + rx * a.cos(), cy - ry * a.sin())
            })
            .collect();
        self.ops.push(Op::Shape(Shape { points, fill: Some(color), stroke: Some(color) }));
    }

    /// The primary selection's eight handles: squares in the window colour
    /// framed in the accent.
    fn handles(&mut self, (x, y, w, h): Rect, accent: u32, fill: u32) {
        let half = HANDLE / 2;
        for (hx, hy) in [(x, y), (x + w, y), (x, y + h), (x + w, y + h), (x + w / 2, y), (x + w / 2, y + h), (x, y + h / 2), (x + w, y + h / 2)] {
            self.fill((hx - half, hy - half, HANDLE, HANDLE), accent);
            self.fill((hx - half + 1, hy - half + 1, HANDLE - 2, HANDLE - 2), fill);
        }
    }

    /// The other selected components: their corners only.
    fn corners(&mut self, (x, y, w, h): Rect, accent: u32) {
        let half = HANDLE / 2;
        for (hx, hy) in [(x, y), (x + w, y), (x, y + h), (x + w, y + h)] {
            self.fill((hx - half, hy - half, HANDLE, HANDLE), accent);
        }
    }

    /// An anchor pin: filled in the accent when that side is anchored,
    /// outlined when not, with a short line to the side it pins.
    fn pin(&mut self, r: LRect, on: bool, side: Side, accent: u32, window: u32, off: u32) {
        let (x, y, w, h) = (r.left, r.top, r.width, r.height);
        let color = if on { accent } else { off };
        match side {
            Side::Left => self.line((x + w, y + h / 2), (x + w + 4, y + h / 2), color),
            Side::Right => self.line((x - 5, y + h / 2), (x - 1, y + h / 2), color),
            Side::Top => self.line((x + w / 2, y + h), (x + w / 2, y + h + 4), color),
            Side::Bottom => self.line((x + w / 2, y - 5), (x + w / 2, y - 1), color),
        }
        self.pie((x, y, w, h), color);
        if !on {
            self.pie((x + 2, y + 2, w - 4, h - 4), window);
        }
    }

    /// A guide: a line in the accent; margins and equal gaps with their
    /// distance.
    /// (`length`: the guide's length in RapidQ's pixels, a margin's label)
    fn guide(&mut self, g: &Guide, length: i64, accent: u32) {
        let (a, b) = (g.from.min(g.to), g.from.max(g.to));
        if g.vertical {
            self.line((g.at, a), (g.at, b), accent);
        } else {
            self.line((a, g.at), (b, g.at), accent);
        }
        let label = match g.kind {
            GuideKind::Margin => Some(length.to_string()),
            GuideKind::Spacing(px) => Some(px.to_string()),
            _ => None,
        };
        if let Some(text) = label {
            let mid = (a + b) / 2;
            let r = if g.vertical { (g.at + 3, mid - 7, 24, 14) } else { (mid - 12, g.at - 15, 24, 14) };
            self.text(r, &text, Font::default(), accent, Place::Center);
        }
    }

    /// A dashed one-pixel outline inside `rect`.
    fn dashed(&mut self, (x, y, w, h): Rect, color: u32) {
        let (r, b) = (x + w - 1, y + h - 1);
        let mut k = x;
        while k <= r {
            let e = (k + 3).min(r);
            self.line((k, y), (e, y), color);
            self.line((k, b), (e, b), color);
            k += 6;
        }
        let mut k = y;
        while k <= b {
            let e = (k + 3).min(b);
            self.line((x, k), (x, e), color);
            self.line((r, k), (r, e), color);
            k += 6;
        }
    }

    /// A small rounded label at (x, y) (its top left): `text` in `fg` on
    /// `bg` (a readout: a size, a place, a type).
    fn pill(&mut self, x: i64, y: i64, text: &str, bg: u32, fg: u32) {
        let font = Font::default();
        let w = super::text::text_size(text, &font).0 + 12;
        let rect = (x, y, w, 17);
        self.ops.push(Op::Round { rect, radius: 4.0, fill: Some(bg), stroke: None, width: 0.0 });
        self.text(rect, text, font, fg, Place::Center);
    }

    /// The form's corner grip: three short diagonal lines in a `w` × `h`
    /// square at (x, y).
    fn corner_grip(&mut self, (x, y, w, h): Rect, color: u32) {
        for k in [3, 6, 9] {
            let k = k * w.min(h) / 10;
            self.line((x + w - 1, y + h - 1 - k), (x + w - 1 - k, y + h - 1), color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A frameless form filling the surface, as the API made them before
    /// (client coordinates = the surface's).
    fn frameless() -> DesignSurface {
        let mut d = DesignSurface::default();
        let root = d.designer.design.root();
        let _ = Command::SetProp { node: root, name: "BorderStyle".into(), value: Some("0".into()) }.apply(&mut d.designer.design);
        d
    }

    fn surface() -> DesignSurface {
        let mut d = frameless();
        d.call("addcomponent", &[v_str("RBUTTON"), v_str("Button1"), v_int(16), v_int(16), v_int(80), v_int(24)]);
        d.call("addcomponent", &[v_str("RLABEL"), v_str("Label1"), v_int(48), v_int(32), v_int(64), v_int(16)]);
        d
    }

    #[test]
    fn methods_and_properties_answer_as_the_designer_expects() {
        let mut d = surface();
        assert_eq!(d.get("compcount"), Some(v_int(2)));
        assert_eq!(d.selection(), Some(1), "the newest is selected");
        assert_eq!(d.call("getname", &[v_int(0)]), Some(v_str("Button1")));
        assert_eq!(d.call("gettype", &[v_int(1)]), Some(v_str("RLABEL")));
        assert_eq!(d.call("getprop", &[v_int(0), v_str("Caption")]), Some(v_str("Button1")), "Caption starts as the name");
        d.call("setprop", &[v_int(0), v_str("Color"), v_str("&H0000FF")]);
        assert_eq!(d.call("getprop", &[v_int(0), v_str("color")]), Some(v_str("&H0000FF")));
        assert_eq!(d.call("getprop", &[v_int(5), v_str("color")]), Some(v_str("")));
        d.call("setcompbounds", &[v_int(1), v_int(1), v_int(2)]);
        assert_eq!(d.components()[1].bounds(), (1, 2, 80, 25), "missing sizes default");
        d.call("setname", &[v_int(1), v_str("Title")]);
        assert_eq!(d.call("getname", &[v_int(1)]), Some(v_str("Title")));
        assert_eq!(d.call("getcompw", &[v_int(9)]), Some(v_int(0)));
        d.call("selectcomp", &[v_int(7)]);
        assert_eq!(d.selection(), None, "nothing selected");
        d.call("removecomponent", &[v_int(0)]);
        assert_eq!((d.get("count"), d.selection()), (Some(v_int(1)), None));
        assert!(d.set("formcaption", &v_str("Main")));
        assert_eq!(d.get("formcaption"), Some(v_str("Main")));
        assert_eq!(d.title(), "Main", "the title bar's, while the form sets no Caption");
        assert_eq!(d.call("undo", &[]), Some(Value::Boolean(true)), "the removal undone");
        assert_eq!(d.get("count"), Some(v_int(2)));
        d.call("clearall", &[]);
        assert_eq!(d.call("count", &[]), Some(v_int(0)));
        assert_eq!(d.call("show", &[]), None, "the host's");
        assert!(d.set("showgrid", &Value::Boolean(false)) && d.set("showselection", &Value::Boolean(false)));
        assert_eq!((d.get("showgrid"), d.get("showselection")), (Some(Value::Boolean(false)), Some(Value::Boolean(false))));
    }

    #[test]
    fn the_mouse_selects_moves_and_resizes_with_guides_and_one_undo() {
        let mut d = surface();
        assert_eq!(d.mouse_down(70, 36, false), Some(DesignEvent::Select(1)));
        d.mouse_up();
        assert_eq!(d.mouse_down(20, 20, false), Some(DesignEvent::Select(0)));
        // dragged by (+13, +6): the grid, the label's edges and margins
        let e = d.mouse_drag(33, 26);
        assert!(matches!(e, Some(DesignEvent::Move { index: 0, .. })));
        assert_eq!(d.call("getcompx", &[v_int(0)]), Some(v_int(16)), "not written while dragging");
        assert!(d.placed().iter().any(|(id, r)| *id == d.ids()[0] && r.left != 16), "the drag is drawn");
        d.mouse_up();
        let x = d.call("getcompx", &[v_int(0)]).unwrap().to_i64();
        assert!(x % 8 == 0 && x != 16, "moved onto the grid or a guide: {x}");
        assert_eq!(d.call("undo", &[]), Some(Value::Boolean(true)));
        assert_eq!(d.call("getcompx", &[v_int(0)]), Some(v_int(16)), "one undo step");
        // the corner handle (no event for the press), then a resize
        assert_eq!(d.grip_at(96, 40), Some(Grip::Corner));
        assert_eq!(d.grip_at(16, 28), Some(Grip::Left));
        assert_eq!(d.mouse_down(96, 40, false), None);
        d.mouse_drag(2, 2);
        d.mouse_up();
        assert_eq!((d.call("getcompw", &[v_int(0)]), d.call("getcomph", &[v_int(0)])), (Some(v_int(16)), Some(v_int(16))), "never smaller than 16");
        // (undone), a double click, then the background
        d.call("undo", &[]);
        assert_eq!(d.mouse_down(50, 28, true), Some(DesignEvent::DblClick(0)));
        d.mouse_up();
        assert_eq!(d.mouse_down(300, 200, false), Some(DesignEvent::BgClick(300, 200)));
        assert_eq!(d.selection(), None);
        // the rubber band selects both
        d.mouse_drag(0, 0);
        d.mouse_up();
        assert_eq!(d.get("selcount"), Some(v_int(2)));
        assert_eq!(DesignEvent::Move { index: 2, x: 1, y: 2, w: 3, h: 4 }.args(), [2, 1, 2, 3, 4].map(v_int).to_vec());
        assert_eq!(DesignEvent::BgClick(5, 6).event(), "onbgclick");
    }

    #[test]
    fn guides_show_while_dragging() {
        let mut d = frameless();
        d.call("addcomponent", &[v_str("REDIT"), v_str("Ed"), v_int(100), v_int(100), v_int(120), v_int(25)]);
        d.call("addcomponent", &[v_str("RBUTTON"), v_str("B"), v_int(300), v_int(300), v_int(75), v_int(25)]);
        d.mouse_down(310, 310, false);
        // 3 px right of the edit's left edge
        d.mouse_drag(113, 250);
        assert!(d.guides.iter().any(|g| g.vertical && g.at == 100 && g.kind == GuideKind::Edge), "{:?}", d.guides);
        let ops = d.chrome_ops();
        let accent = crate::theme::current().accent;
        assert!(ops.iter().any(|o| matches!(o, Op::Line { color, from, to } if *color == accent && from.0 == to.0)), "a guide drawn");
        d.mouse_up();
        assert!(d.guides.is_empty());
        assert_eq!(d.call("getcompx", &[v_int(1)]), Some(v_int(100)));
    }

    #[test]
    fn anchor_pins_and_the_resize_preview() {
        let mut d = frameless();
        d.set_size(400, 300);
        d.call("addcomponent", &[v_str("RBUTTON"), v_str("Ok"), v_int(300), v_int(250), v_int(75), v_int(25)]);
        // the right pin, then the bottom pin; the left and top ones off
        let ok = LRect::new(300, 250, 75, 25);
        let [left, top, right, bottom] = pins(ok);
        for (_, p) in [right, bottom, left, top] {
            assert_eq!(d.mouse_down(p.left + 2, p.top + 2, false), None);
            d.mouse_up();
        }
        assert_eq!(d.call("getprop", &[v_int(0), v_str("Anchors")]), Some(v_str("akRight + akBottom")));
        // the form's corner grip dragged 100 × 50 further: the button
        // follows, and both are written when the mouse is let go
        d.fit = false;
        assert_eq!(d.form_rect(), (MARGIN, MARGIN, 400, 300));
        assert_eq!(d.form_grip_at(402, 150), Some(Grip::FormRight));
        assert_eq!(d.form_grip_at(200, 301), Some(Grip::FormBottom));
        assert_eq!(d.mouse_down(402, 302, false), None);
        d.mouse_drag(502, 352);
        assert_eq!(d.preview, Some((500, 350)));
        assert_eq!((d.components()[0].x, d.components()[0].y), (400, 300));
        d.mouse_up();
        assert_eq!(d.preview, None);
        assert_eq!(d.form_rect(), (MARGIN, MARGIN, 500, 350), "the new size written");
        assert_eq!(d.call("getcompx", &[v_int(0)]), Some(v_int(400)), "the anchored button where the resize put it");
        assert_eq!(d.call("undo", &[]), Some(Value::Boolean(true)));
        assert_eq!((d.form_rect(), d.call("getcompx", &[v_int(0)])), ((MARGIN, MARGIN, 400, 300), Some(v_int(300))), "one undo step");
        // the right edge alone
        d.mouse_down(401, 100, false);
        d.mouse_drag(441, 180);
        d.mouse_up();
        assert_eq!(d.form_rect(), (MARGIN, MARGIN, 440, 300));
        // the preview by its properties
        d.set("previewwidth", &v_int(300));
        assert_eq!(d.components()[0].x, 200);
        d.set("previewwidth", &v_int(0));
        assert_eq!(d.preview, None);
    }

    #[test]
    fn a_framed_form_its_menu_and_its_tray() {
        let mut d = DesignSurface::default();
        d.set_size(500, 400);
        assert!(d.border(), "a form has a frame unless BorderStyle = bsNone");
        assert_eq!(d.form_rect(), (MARGIN, MARGIN, 500 - 2 * MARGIN, 400 - 2 * MARGIN), "the API's form fills the surface");
        assert_eq!(d.client_origin(), (MARGIN + FORM_BORDER, MARGIN + FORM_BORDER + FORM_CAPTION));
        d.call("addcomponent", &[v_str("QBUTTON"), v_str("B"), v_int(8), v_int(8), v_int(75), v_int(25)]);
        d.call("addcomponent", &[v_str("QTIMER"), v_str("Tick"), v_int(0), v_int(0), v_int(0), v_int(0)]);
        d.call("addcomponent", &[v_str("QMAINMENU"), v_str("Menu"), v_int(0), v_int(0), v_int(0), v_int(0)]);
        d.set_size(500, 400);
        assert_eq!(d.menu_height(), MAIN_MENU_HEIGHT);
        assert_eq!(d.client_origin().1, MARGIN + FORM_BORDER + FORM_CAPTION + MAIN_MENU_HEIGHT);
        let tray = d.tray();
        assert_eq!(tray.len(), 1, "the timer, not the menu");
        assert_eq!((tray[0].index, tray[0].name.as_str()), (1, "Tick"));
        let (_, fy, _, fh) = d.form_rect();
        assert_eq!(fy + fh + TRAY_GAP + TRAY_H + MARGIN, 400, "the form leaves room for the tray");
        // a press on the tray item selects it (no move); on the button, the button
        let (tx, ty, _, _) = tray[0].rect;
        assert_eq!(d.mouse_down(tx + 2, ty + 2, false), Some(DesignEvent::Select(1)));
        d.mouse_drag(tx + 40, ty + 40);
        d.mouse_up();
        assert_eq!(d.selection(), Some(1));
        assert!(d.chrome_ops().iter().any(|o| matches!(o, Op::Edge { rect, .. } if *rect == tray[0].rect)), "the selected tray item framed");
        assert_eq!(d.mouse_down(10, 10, false), Some(DesignEvent::Select(0)));
        d.mouse_up();
        // the title: the form's Caption, else FormCaption
        assert_eq!(d.title(), "Form1");
        let root = d.designer.design.root();
        let _ = d.designer.execute(Command::SetProp { node: root, name: "Caption".into(), value: Some("\"Hello\"".into()) });
        assert_eq!(d.title(), "Hello");
        // no chrome with the selection hidden; no dots without the grid
        d.show_selection = false;
        d.show_grid = false;
        assert!(d.chrome_ops().is_empty());
        assert!(d.grid_ops(100, 100, 0xF0F0F0).is_empty());
        d.show_grid = true;
        assert_eq!(d.grid_ops(16, 16, 0xF0F0F0).len(), 4);
    }

    #[test]
    fn a_form_read_from_a_program_keeps_its_size() {
        let mut design = FormDesign::new("Main", "QFORM");
        let root = design.root();
        for (p, v) in [("Width", "300"), ("Height", "200"), ("Caption", "\"Main\"")] {
            let _ = Command::SetProp { node: root, name: p.into(), value: Some(v.into()) }.apply(&mut design);
        }
        let mut d = DesignSurface::with_designer(Designer::new(design));
        d.set_size(800, 600);
        assert_eq!(d.form_rect(), (MARGIN, MARGIN, 300, 200));
        assert_eq!(d.title(), "Main");
        assert_eq!(d.client_size(), crate::layout::form_client_size(300, 200, 2, 0));
    }
}

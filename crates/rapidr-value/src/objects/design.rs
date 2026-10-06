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
/// The form's corner grip (the resize preview's), outside its corner.
const CORNER: i64 = 10;

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
    /// The form's bottom-right corner: the resize preview.
    FormCorner,
    /// A rubber band from the background.
    Band,
    /// A tray item pressed (it doesn't move).
    Tray,
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
    /// OnSelect (Index): a component pressed.
    Select(usize),
    /// OnDblClick (Index): a component pressed twice.
    DblClick(usize),
    /// OnBgClick (X, Y): the background pressed (nothing selected now).
    BgClick(i64, i64),
    /// OnMove (Index, X, Y, W, H): the selected component dragged or
    /// resized (each step of the drag).
    Move { index: usize, x: i64, y: i64, w: i64, h: i64 },
}

impl DesignEvent {
    /// The event's name (lowercase, as handlers are stored).
    pub fn event(&self) -> &'static str {
        match self {
            DesignEvent::Select(_) => "onselect",
            DesignEvent::DblClick(_) => "ondblclick",
            DesignEvent::BgClick(..) => "onbgclick",
            DesignEvent::Move { .. } => "onmove",
        }
    }

    /// Its arguments.
    pub fn args(&self) -> Vec<i64> {
        match *self {
            DesignEvent::Select(i) | DesignEvent::DblClick(i) => vec![i as i64],
            DesignEvent::BgClick(x, y) => vec![x, y],
            DesignEvent::Move { index, x, y, w, h } => vec![index as i64, x, y, w, h],
        }
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
    /// The form laid out as designed, for the design it was laid out from.
    laid: RefCell<Option<(FormDesign, Rc<Layout>)>>,
    /// The same at the preview's size.
    previewed: RefCell<Option<((i64, i64), Rc<Layout>)>>,
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
            fit: true,
            drag: None,
            guides: Vec::new(),
            preview: None,
            show_guides: true,
            show_grid: true,
            show_pins: true,
            show_selection: true,
            band: None,
            laid: RefCell::new(None),
            previewed: RefCell::new(None),
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
        let keep = (self.size, self.show_guides, self.show_grid, self.show_pins, self.show_selection);
        *self = DesignSurface::with_designer(designer);
        (self.size, self.show_guides, self.show_grid, self.show_pins, self.show_selection) = keep;
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

    /// The primary selection's index, when it is one.
    pub fn selection(&self) -> Option<usize> {
        self.designer.selection.primary().and_then(|id| self.index_of(id))
    }

    /// The selected components' indexes.
    pub fn selected(&self) -> Vec<usize> {
        self.designer.selection.ids().iter().filter_map(|&id| self.index_of(id)).collect()
    }

    /// Selects component `i` alone (a screen reader's click).
    pub fn select(&mut self, i: usize) -> bool {
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

    /// The form's window on the surface: where, and its Width × Height (the
    /// preview's while it shows).
    pub fn form_rect(&self) -> Rect {
        let m = self.margin();
        let (w, h) = self.layout().form_size();
        (m, m, w, h)
    }

    /// Where the form's client area (0, 0) is on the surface.
    pub fn client_origin(&self) -> (i64, i64) {
        let m = self.margin();
        let (ix, iy) = self.inset();
        (m + ix, m + iy + self.menu_height())
    }

    /// The form's inside below its menu (its scroll bars included).
    pub fn client_size(&self) -> (i64, i64) {
        let (_, _, w, h) = self.form_rect();
        crate::layout::form_client_size(w, h, if self.border() { 2 } else { 0 }, self.menu_height())
    }

    /// The tray strip under the form (client coordinates): its items, left
    /// to right; empty when the form has no non-visual component.
    pub fn tray(&self) -> Vec<TrayItem> {
        let d = &self.designer.design;
        let (ox, oy) = self.client_origin();
        let (fx, fy, _, fh) = self.form_rect();
        let top = fy + fh + TRAY_GAP - oy;
        let mut x = fx - ox + 4;
        let font = tray_font();
        self.ids()
            .into_iter()
            .enumerate()
            .filter_map(|(index, id)| {
                let n = d.node(id)?;
                if n.is_visual() || matches!(n.canonical.as_str(), "RMAINMENU" | "RMENUITEM") {
                    return None;
                }
                let w = 4 + TRAY_ICON + 4 + super::text::text_size(&n.name, &font).0 + 8;
                let rect = (x, top + 4, w, TRAY_H - 8);
                x += w + 4;
                Some(TrayItem { index, name: n.name.clone(), type_name: n.type_written.clone(), rect })
            })
            .collect()
    }

    /// The tray strip's rectangle (client coordinates), when it shows.
    pub fn tray_rect(&self) -> Option<Rect> {
        let items = self.tray();
        let last = items.last()?;
        let (ox, oy) = self.client_origin();
        let (fx, fy, fw, fh) = self.form_rect();
        let w = fw.max(last.rect.0 + last.rect.2 + 4 - (fx - ox));
        Some((fx - ox, fy + fh + TRAY_GAP - oy, w, TRAY_H))
    }

    /// The surface's size: a form made through the API fills it (less the
    /// margin and the tray strip).
    pub fn set_size(&mut self, w: i64, h: i64) {
        if w <= 0 || h <= 0 {
            return;
        }
        self.size = (w, h);
        if !self.fit {
            return;
        }
        let m = self.margin();
        let tray = if self.tray().is_empty() { 0 } else { TRAY_GAP + TRAY_H };
        let (fw, fh) = ((w - 2 * m).max(MIN_SIZE), (h - 2 * m - tray).max(MIN_SIZE));
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
        let dragging = self.drag.as_ref().filter(|g| matches!(g.grip, Grip::Move | Grip::Left | Grip::Top | Grip::Right | Grip::Bottom | Grip::TopLeft | Grip::TopRight | Grip::BottomLeft | Grip::Corner));
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
        let moving = self.drag.as_ref().filter(|g| g.moved && !matches!(g.grip, Grip::FormCorner | Grip::Band | Grip::Tray));
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
    }

    /// RemoveComponent: the selection goes.
    pub fn remove(&mut self, i: i64) -> bool {
        let Some(id) = self.id_of(i) else { return false };
        let ok = self.designer.execute(Command::Remove { node: id }).is_ok();
        self.designer.selection.clear();
        self.selected_raw = -1;
        ok
    }

    /// ClearAll.
    pub fn clear(&mut self) {
        let cmds = self.designer.design.children(self.designer.design.root()).into_iter().map(|node| Command::Remove { node }).collect();
        let _ = self.designer.execute(Command::Batch(cmds));
        self.designer.selection.clear();
        self.selected_raw = -1;
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
        let near = |hx: i64, hy: i64| (x - hx).abs() <= GRAB && (y - hy).abs() <= GRAB;
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
        let r = self.rect_of(id)?;
        pins(r).into_iter().find(|(_, p)| x >= p.left && x < p.left + p.width && y >= p.top && y < p.top + p.height).map(|(s, _)| s)
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

    /// The form's corner grip (the resize preview's), client coordinates.
    fn corner_rect(&self) -> Rect {
        let (ox, oy) = self.client_origin();
        let (fx, fy, fw, fh) = self.form_rect();
        (fx + fw - ox - 3, fy + fh - oy - 3, CORNER + 3, CORNER + 3)
    }

    fn on_form_corner(&self, x: i64, y: i64) -> bool {
        let (cx, cy, cw, ch) = self.corner_rect();
        x >= cx && y >= cy && x < cx + cw && y < cy + ch
    }

    /// The mouse pressed at (x, y) of the form's client area (`double`: the
    /// second press of a double click; `add`: Shift / Ctrl held): an anchor
    /// pin toggled, a handle or the form's corner grabbed (nothing heard), a
    /// component selected (OnSelect / OnDblClick; with `add`, in or out of
    /// the selection), or the background (nothing selected, OnBgClick, a
    /// rubber band).
    pub fn mouse_down_with(&mut self, x: i64, y: i64, double: bool, add: bool) -> Option<DesignEvent> {
        self.guides.clear();
        if let Some(side) = self.pin_at(x, y) {
            let _ = self.designer.toggle_anchor(side);
            return None;
        }
        if let Some(grip) = self.grip_at(x, y) {
            self.start_drag(grip, x, y);
            return None;
        }
        if self.on_form_corner(x, y) && self.component_at(x, y).is_none() {
            self.start_drag(Grip::FormCorner, x, y);
            return None;
        }
        match self.component_at(x, y) {
            Some(i) => {
                let id = self.ids()[i];
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
                if self.designer.selection.contains(id) {
                    self.start_drag(if self.on_form(id) { Grip::Move } else { Grip::Tray }, x, y);
                }
                Some(if double { DesignEvent::DblClick(i) } else { DesignEvent::Select(i) })
            }
            None => {
                if !add {
                    self.designer.selection.clear();
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
        let start: Vec<(NodeId, LRect)> = self.designer.selection.ids().iter().filter(|&&id| self.on_form(id)).filter_map(|&id| self.rect_of(id).map(|r| (id, r))).collect();
        let offset = if grip == Grip::FormCorner {
            let (_, _, w, h) = self.form_rect();
            (w - x, h - y)
        } else {
            start.first().map_or((0, 0), |(_, r)| (x - r.left, y - r.top))
        };
        self.drag = Some(Drag { grip, from: (x, y), offset, now: start.clone(), start, moved: false });
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
    /// the grid and the guides — OnMove at every step; the form's corner
    /// resizes the preview; a rubber band selects what it touches.
    pub fn mouse_drag_with(&mut self, x: i64, y: i64, free: bool) -> Option<DesignEvent> {
        let mut drag = self.drag.take()?;
        let out = self.drag_to(&mut drag, x, y, free);
        self.drag = Some(drag);
        out
    }

    pub fn mouse_drag(&mut self, x: i64, y: i64) -> Option<DesignEvent> {
        self.mouse_drag_with(x, y, false)
    }

    fn drag_to(&mut self, drag: &mut Drag, x: i64, y: i64, free: bool) -> Option<DesignEvent> {
        match drag.grip {
            Grip::Tray => None,
            Grip::FormCorner => {
                self.preview = Some(((x + drag.offset.0).max(MIN_SIZE), (y + drag.offset.1).max(MIN_SIZE)));
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
                } else {
                    drag.now = drag.start.iter().map(|&(id, r)| (id, if id == primary { to } else { r })).collect();
                }
                drag.moved = drag.now != drag.start;
                let index = self.index_of(primary)?;
                // (a press without a move says nothing)
                if !drag.moved {
                    return None;
                }
                Some(DesignEvent::Move { index, x: to.left, y: to.top, w: to.width, h: to.height })
            }
        }
    }

    /// The mouse let go: a move or resize is written (one undo step), the
    /// guides go, the resize preview ends (the design keeps its size).
    pub fn mouse_up(&mut self) {
        self.guides.clear();
        self.band = None;
        let Some(drag) = self.drag.take() else { return };
        if drag.grip == Grip::FormCorner {
            self.preview = None;
            return;
        }
        if !drag.moved {
            return;
        }
        let l = self.designer.layout();
        let d = &self.designer.design;
        let mut cmds = Vec::new();
        for (id, abs) in &drag.now {
            let (Some(old), o) = (l.rect(*id), l.origin(d, *id)) else { continue };
            cmds.extend(arrange::set_rect(*id, old, LRect::new(abs.left - o.0, abs.top - o.1, abs.width, abs.height)));
        }
        let _ = self.designer.execute(Command::Batch(cmds));
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
            "previewwidth" => v_int(self.preview.map_or(0, |p| p.0)),
            "previewheight" => v_int(self.preview.map_or(0, |p| p.1)),
            "showguides" => Value::Boolean(self.show_guides),
            "showgrid" => Value::Boolean(self.show_grid),
            "showselection" => Value::Boolean(self.show_selection),
            "snaptogrid" => Value::Boolean(self.designer.snapper.snap_to_grid),
            "gridsize" => v_int(self.designer.snapper.grid),
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
            _ => return false,
        }
        true
    }

    /// Its methods (`None`: not one of them; Show and Hide are the host's).
    pub fn call(&mut self, method: &str, args: &[Value]) -> Option<Value> {
        let int = |i: usize, default: i64| args.get(i).map_or(default, Value::to_i64);
        let text = |i: usize| args.get(i).map(Value::to_string_val).unwrap_or_default();
        let index = int(0, -1);
        let comp = |s: &Self| s.id_of(index).and_then(|id| s.designer.design.node(id).cloned());
        let rect = |s: &Self| s.id_of(index).and_then(|id| s.rect_of(id));
        Some(match method {
            // AddComponent(Type, Name, X, Y, W, H)
            "addcomponent" => {
                self.add(&text(0), &text(1), (int(2, 0), int(3, 0), int(4, 80), int(5, 25)));
                Value::Null
            }
            "getname" => v_str(&comp(self).map_or(String::new(), |c| c.name)),
            "gettype" => v_str(&comp(self).map_or(String::new(), |c| c.type_written)),
            "getcompx" => v_int(rect(self).map_or(0, |r| r.left)),
            "getcompy" => v_int(rect(self).map_or(0, |r| r.top)),
            "getcompw" => v_int(rect(self).map_or(0, |r| r.width)),
            "getcomph" => v_int(rect(self).map_or(0, |r| r.height)),
            // SetProp(Index, Name, Value) / GetProp(Index, Name)
            "setprop" => {
                if let Some(c) = comp(self) {
                    let value = source_value(&c.canonical, &text(1), &text(2));
                    let _ = self.designer.execute(Command::SetProp { node: c.id, name: text(1), value: Some(value) });
                }
                Value::Null
            }
            "getprop" => v_str(&comp(self).and_then(|c| c.prop(&text(1)).map(plain_value)).unwrap_or_default()),
            // SetCompBounds(Index, X, Y, W, H)
            "setcompbounds" => {
                if let (Some(id), Some(old)) = (self.id_of(index), self.id_of(index).and_then(|id| self.designer.layout().rect(id))) {
                    let new = LRect::new(int(1, 0), int(2, 0), int(3, 80), int(4, 25));
                    let _ = self.designer.execute(Command::Batch(arrange::set_rect(id, old, new)));
                }
                Value::Null
            }
            "setname" => {
                if let Some(id) = self.id_of(index) {
                    let _ = self.designer.rename(id, &text(1));
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
            "removecomponent" => {
                self.remove(index);
                Value::Null
            }
            "clearall" => {
                self.clear();
                Value::Null
            }
            "count" => v_int(self.ids().len() as i64),
            // (I4: the designer model's)
            "undo" => Value::Boolean(self.designer.undo()),
            "redo" => Value::Boolean(self.designer.redo()),
            // AlignSelection(How): left, center, right, top, middle, bottom —
            // to the first selected (Align is the surface's own property)
            "alignselection" => {
                if let Some(how) = arrange::AlignHow::parse(&text(0)) {
                    let _ = self.designer.align(how);
                }
                Value::Null
            }
            // SelectAdd(Index): Shift+click's
            "selectadd" => {
                if let Some(id) = self.id_of(index) {
                    self.designer.selection.add(id);
                }
                Value::Null
            }
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

    /// The designer's chrome over the drawn form, in client coordinates: the
    /// selection framed with the primary's eight handles and anchor pins
    /// (unless [`DesignSurface::show_selection`] is off), the guides, the
    /// rubber band, the form's corner grip and the preview's size — in the
    /// theme's tokens.
    pub fn chrome_ops(&self) -> Vec<Op> {
        let t = crate::theme::current();
        let mut d = Draw::default();
        if self.show_selection {
            let comps = self.components();
            let tray = self.tray();
            for (k, &i) in self.selected().iter().enumerate() {
                let Some(c) = comps.get(i) else { continue };
                if c.visual {
                    d.rect(c.bounds(), t.accent);
                    if k == 0 {
                        d.handles(c.bounds(), t.accent, t.window);
                    } else {
                        d.corners(c.bounds(), t.accent);
                    }
                } else if let Some(item) = tray.iter().find(|x| x.index == i) {
                    d.rect(item.rect, t.accent);
                }
            }
            if self.show_pins && self.drag.as_ref().is_none_or(|g| !g.moved) {
                if let Some(id) = self.designer.selection.primary().filter(|&id| self.on_form(id)) {
                    if let (Some(i), Some(n)) = (self.index_of(id), self.designer.design.node(id)) {
                        let anchors = n.int("Anchors").unwrap_or(crate::layout::DEFAULT_ANCHORS);
                        if let Some(c) = comps.get(i) {
                            for (side, p) in pins(LRect::new(c.x, c.y, c.w, c.h)) {
                                d.pin(p, anchors & side.bit() != 0, side, t.accent, t.window, t.border_strong);
                            }
                        }
                    }
                }
            }
        }
        if self.show_guides {
            for g in &self.guides {
                d.guide(g, t.accent);
            }
        }
        if let Some(b) = self.band {
            d.rect((b.left, b.top, b.width, b.height), t.accent);
        }
        if self.show_selection {
            // the form's corner: the resize preview's grip
            let (x, y, _, _) = self.corner_rect();
            d.corner_grip((x + 3, y + 3, CORNER, CORNER), t.accent);
            if let Some((pw, ph)) = self.preview {
                d.text((x - 120, y + CORNER + 4, 130, 14), &format!("{pw} × {ph}"), Font::default(), t.window, Place::TopRight);
            }
        }
        d.ops
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
    fn guide(&mut self, g: &Guide, accent: u32) {
        let (a, b) = (g.from.min(g.to), g.from.max(g.to));
        if g.vertical {
            self.line((g.at, a), (g.at, b), accent);
        } else {
            self.line((a, g.at), (b, g.at), accent);
        }
        let label = match g.kind {
            GuideKind::Margin => Some((b - a).to_string()),
            GuideKind::Spacing(px) => Some(px.to_string()),
            _ => None,
        };
        if let Some(text) = label {
            let mid = (a + b) / 2;
            let r = if g.vertical { (g.at + 3, mid - 7, 24, 14) } else { (mid - 12, g.at - 15, 24, 14) };
            self.text(r, &text, Font::default(), accent, Place::Center);
        }
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
        assert_eq!(DesignEvent::Move { index: 2, x: 1, y: 2, w: 3, h: 4 }.args(), vec![2, 1, 2, 3, 4]);
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
        // the form's corner grip dragged 100 × 50 further: the button follows
        assert_eq!(d.form_rect(), (0, 0, 400, 300));
        assert_eq!(d.mouse_down(402, 302, false), None);
        d.mouse_drag(502, 352);
        assert_eq!(d.preview, Some((500, 350)));
        assert_eq!((d.components()[0].x, d.components()[0].y), (400, 300));
        d.mouse_up();
        assert_eq!(d.preview, None);
        assert_eq!(d.call("getcompx", &[v_int(0)]), Some(v_int(300)), "the design unchanged");
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

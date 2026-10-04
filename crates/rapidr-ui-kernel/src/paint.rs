//! Drawing a form into a [`DisplayList`]: each component paints its ops
//! at its absolute position, clipped to its rectangle and its parents'
//! (as Windows clips child windows); the form's background and an
//! in-window menu bar's place first.

use rapidr_value::objects::font::Font;
use rapidr_value::objects::ops::{Op, Place, Rect};
use rapidr_value::objects::trackbar::Shape;

use crate::components::{Cx, State};
use crate::display::{DisplayList, Item, TextItem};
use crate::store::Store;
use crate::text::{bgr_to_rgb, TextSystem};
use crate::tree::FormUi;

/// Windows' 3D colours, as the shared models draw them.
pub const FACE: u32 = 0xF0F0F0;
pub const LIGHT: u32 = 0xFFFFFF;
pub const SHADOW: u32 = 0x808080;
pub const DARK: u32 = 0x404040;
/// Disabled text (COLOR_GRAYTEXT).
pub const GRAY_TEXT: u32 = 0x808080;
/// Selected text's background and colour (COLOR_HIGHLIGHT).
pub const HIGHLIGHT: u32 = 0x0078D7;
pub const HIGHLIGHT_TEXT: u32 = 0xFFFFFF;

/// Puts ops into a display list in logical pixels, from an origin (the
/// component being drawn).
pub struct Painter<'a> {
    list: &'a mut DisplayList,
    origin: (i64, i64),
}

impl<'a> Painter<'a> {
    pub fn new(list: &'a mut DisplayList) -> Self {
        Painter { list, origin: (0, 0) }
    }

    /// Device pixels per logical pixel.
    pub fn scale(&self) -> f64 {
        self.list.scale
    }

    pub fn origin(&self) -> (i64, i64) {
        self.origin
    }

    /// Draws what `f` draws with (0, 0) at `origin` (relative to the
    /// current origin).
    pub fn at(&mut self, origin: (i64, i64), f: impl FnOnce(&mut Painter)) {
        let was = self.origin;
        self.origin = (was.0 + origin.0, was.1 + origin.1);
        f(self);
        self.origin = was;
    }

    pub fn op(&mut self, op: Op) {
        self.list.items.push(Item::Op { origin: self.origin, op });
    }

    pub fn ops(&mut self, ops: impl IntoIterator<Item = Op>) {
        for op in ops {
            self.op(op);
        }
    }

    pub fn fill(&mut self, rect: Rect, color: u32) {
        if rect.2 > 0 && rect.3 > 0 {
            self.op(Op::Fill { rect, color });
        }
    }

    /// A 3D frame (DrawEdge): `light` lines top / left, `dark` bottom /
    /// right, from the outside in.
    pub fn edge(&mut self, rect: Rect, light: &[u32], dark: &[u32]) {
        self.op(Op::Edge { rect, light: light.to_vec(), dark: dark.to_vec() });
    }

    pub fn focus(&mut self, rect: Rect) {
        if rect.2 > 0 && rect.3 > 0 {
            self.op(Op::Focus { rect });
        }
    }

    pub fn text(&mut self, rect: Rect, text: &str, font: &Font, color: u32, place: Place) {
        if !text.is_empty() {
            self.op(Op::Text { rect, text: text.to_string(), font: font.clone(), color, angle: 0, place });
        }
    }

    pub fn shape(&mut self, shape: Shape) {
        self.op(Op::Shape(shape));
    }

    pub fn line(&mut self, from: (f64, f64), to: (f64, f64), color: u32) {
        self.op(Op::Line { from, to, color });
    }

    /// Clips what `f` draws to `rect`.
    pub fn clipped(&mut self, rect: Rect, f: impl FnOnce(&mut Painter)) {
        self.op(Op::ClipPush { rect });
        f(self);
        self.op(Op::ClipPop);
    }

    /// An editor's layout (device pixels).
    pub fn editor(&mut self, item: TextItem) {
        self.list.items.push(Item::Text(item));
    }

    /// A logical point (from the origin) on the device's pixel grid.
    pub fn device_point(&self, x: i64, y: i64) -> (f64, f64) {
        let s = self.scale();
        (((self.origin.0 + x) as f64 * s).round(), ((self.origin.1 + y) as f64 * s).round())
    }
}

/// A caption with its `&` mnemonic drawn as Windows draws it: the text
/// without the `&`, the marked letter underlined (a line under it, from
/// `text_size`'s widths: as the text is drawn, unkerned).
pub fn caption(p: &mut Painter, rect: Rect, caption: &str, font: &Font, color: u32, place: Place) {
    let (shown, mark) = rapidr_value::objects::a11y::mnemonic(caption);
    p.text(rect, &shown, font, color, place);
    let Some((at, _)) = mark else { return };
    use rapidr_value::objects::text::text_size;
    let (tw, th) = text_size(&shown, font);
    let before: String = shown.chars().take(at).collect();
    let letter: String = shown.chars().skip(at).take(1).collect();
    let (x, w) = (text_size(&before, font).0, text_size(&letter, font).0);
    let (rx, ry, rw, rh) = rect;
    let left = match place {
        Place::TopLeft | Place::Left => rx,
        Place::TopRight => rx + rw - tw,
        Place::Center | Place::TopCenter => rx + (rw - tw) / 2,
    };
    let top = match place {
        Place::TopLeft | Place::TopCenter | Place::TopRight => ry,
        Place::Center | Place::Left => ry + (rh - th) / 2,
    };
    if w <= 0 {
        return;
    }
    // (the underline: the pixel row under the baseline; Liberation's
    // ascent is 0.905 em)
    let y = (top as f64 + font.pixel_size() as f64 * 0.905).floor() + 1.5;
    p.line(((left + x) as f64 + 0.5, y), ((left + x + w - 1) as f64 + 0.5, y), color);
}

impl FormUi {
    /// The form drawn for a screen of `scale` device pixels per logical
    /// pixel. Geometry and captions are read from the store now.
    pub fn paint(&mut self, store: &dyn Store, ts: &mut TextSystem, scale: f64) -> DisplayList {
        self.sync(store);
        self.scale = scale;
        let (w, h) = self.client;
        let mut list = DisplayList { size: (w, h + self.menu_offset), scale, items: Vec::new() };
        let mut p = Painter::new(&mut list);
        if self.menu_offset > 0 {
            // (the in-window menu bar: components/menubar.rs)
            self.paint_menu_bar(store, &mut p);
        }
        let color = rapidr_value::objects::form_color(&store.get(&self.form, "color"));
        p.fill((0, self.menu_offset, w, h), bgr_to_rgb(color));
        let default = self.default_button(store);
        let focused_is_button = self.focus.is_some_and(|f| self.nodes[f].type_name == "RBUTTON");
        for root in self.roots() {
            self.paint_node(root, store, ts, &mut p, default, focused_is_button);
        }
        // (the form's scroll bars, over its components)
        crate::components::scrollbox::paint_form_bars(&self.form, (w, h), &mut p, self.menu_offset);
        // (open menus over everything)
        self.paint_menus(store, &mut p);
        self.dirty = false;
        list
    }

    fn paint_node(&mut self, i: usize, store: &dyn Store, ts: &mut TextSystem, p: &mut Painter, default: Option<usize>, focused_is_button: bool) {
        if !self.nodes[i].shown {
            return;
        }
        let (x, y, w, h) = self.nodes[i].abs;
        let state = State {
            focused: self.focus == Some(i),
            hover: self.hover == Some(i),
            pressed: self.pressed == Some(i) && self.hover == Some(i),
            held: self.pressed == Some(i),
            enabled: self.nodes[i].enabled,
            caret_on: self.caret_on,
            default_frame: default == Some(i) && !focused_is_button,
        };
        let children = self.children(i);
        let scale = self.scale;
        p.at((x, y), |p| {
            p.clipped((0, 0, w, h), |p| {
                let node = &mut self.nodes[i];
                if let Some(kind) = node.kind {
                    // (painting fires nothing)
                    let mut events = Vec::new();
                    let mut cx = Cx { store, text: ts, id: &node.id, rect: node.abs, font: store.font(&node.id), state, ui: &mut node.ui, events: &mut events, scale };
                    kind.paint(&mut cx, p);
                }
            });
        });
        let kind = self.nodes[i].kind;
        let id = self.nodes[i].id.clone();
        if !children.is_empty() {
            // (children inside their parent's client area only)
            let area = kind.map_or((0, 0, w, h), |k| k.client_area(store, &id, w, h));
            p.at((x, y), |p| p.op(Op::ClipPush { rect: area }));
            for c in children {
                self.paint_node(c, store, ts, p, default, focused_is_button);
            }
            p.at((x, y), |p| p.op(Op::ClipPop));
        }
        if let Some(k) = kind {
            // (what it draws over them: a scroll box's bars)
            p.at((x, y), |p| k.paint_over(store, &id, w, h, p));
        }
    }
}

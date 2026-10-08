//! Drawing a form into a [`DisplayList`]: each component paints its ops
//! at its absolute position, clipped to its rectangle and its parents'
//! (as Windows clips child windows); the form's background and an
//! in-window menu bar's place first.
//!
//! Every colour, metric and glyph style comes from the current theme
//! (`rapidr_value::theme`, the [`Painter`]'s): the classic look's are the
//! colours this always drew, so a program that names no theme is drawn
//! as before, op for op. The fluent looks (modern, dark, high contrast)
//! draw the same components flat — rounded boxes, thin borders, focus
//! rings — in the same places and sizes.

use rapidr_value::objects::font::Font;
use rapidr_value::objects::ops::{lift, Op, Place, Rect};
use rapidr_value::objects::trackbar::Shape;
use rapidr_value::scrollbars::{Part, Scroller, BAR};
use rapidr_value::theme::{self, Theme};
use rapidr_value::Value;

use crate::components::{Cx, State};
use crate::display::{DisplayList, Item, TextItem};
use crate::store::{self, Store};
use crate::text::{bgr_to_rgb, TextSystem};
use crate::tree::FormUi;

/// Windows' 3D colours as the classic look has them (`theme::CLASSIC`'s):
/// what draws with the current theme takes [`Painter::theme`]'s instead.
pub const FACE: u32 = theme::CLASSIC.face;
pub const LIGHT: u32 = theme::CLASSIC.light;
pub const SHADOW: u32 = theme::CLASSIC.shadow;
pub const DARK: u32 = theme::CLASSIC.dark_shadow;
/// Disabled text (COLOR_GRAYTEXT).
pub const GRAY_TEXT: u32 = theme::CLASSIC.gray_text;
/// Selected text's background and colour (COLOR_HIGHLIGHT).
pub const HIGHLIGHT: u32 = theme::CLASSIC.highlight;
pub const HIGHLIGHT_TEXT: u32 = theme::CLASSIC.highlight_text;

/// Where an arrow glyph points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

/// Puts ops into a display list in logical pixels, from an origin (the
/// component being drawn), in a theme (the current one when it was made).
pub struct Painter<'a> {
    list: &'a mut DisplayList,
    origin: (i64, i64),
    theme: &'static Theme,
}

impl<'a> Painter<'a> {
    pub fn new(list: &'a mut DisplayList) -> Self {
        Painter { list, origin: (0, 0), theme: theme::current() }
    }

    /// The theme it draws with.
    pub fn theme(&self) -> &'static Theme {
        self.theme
    }

    /// Whether the theme draws flat (modern, dark, high contrast).
    pub fn fluent(&self) -> bool {
        self.theme.fluent()
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

    /// Ops as they are (a shared model's); a fluent theme draws their focus
    /// rectangles as its rings.
    pub fn ops(&mut self, ops: impl IntoIterator<Item = Op>) {
        for op in ops {
            match op {
                Op::Focus { rect } if self.fluent() => self.focus(rect),
                op => self.op(op),
            }
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

    /// The keyboard focus around `rect`: Windows' dotted rectangle
    /// (classic), the theme's ring inside it (fluent).
    pub fn focus(&mut self, rect: Rect) {
        if rect.2 > 0 && rect.3 > 0 {
            let t = self.theme;
            if t.fluent() {
                self.ring(rect, t.radius, t.focus, t.focus_width);
            } else {
                self.op(Op::Focus { rect });
            }
        }
    }

    /// A rounded rectangle, filled and / or bordered `width` pixels inside
    /// its edge (smooth).
    pub fn round(&mut self, rect: Rect, radius: f64, fill: Option<u32>, stroke: Option<u32>, width: f64) {
        if rect.2 > 0 && rect.3 > 0 && (fill.is_some() || stroke.is_some()) {
            self.op(Op::Round { rect, radius, fill, stroke, width });
        }
    }

    /// A raised control's rim (RapidR's look: a button, a combo box's
    /// button face): its rounded border `border` drawn again a step
    /// darker along the bottom, so it stands a hair off the surface.
    /// Nothing when not `raised` (pressed, disabled) or in the classic look.
    pub fn raised_rim(&mut self, (x, y, w, h): Rect, border: u32, raised: bool) {
        let t = self.theme;
        if !t.fluent() || t.contrast || !raised || w <= 0 || h <= 2 {
            return;
        }
        let rim = rapidr_value::theme::mix(border, if t.dark { 0x000000 } else { t.text }, if t.dark { 350 } else { 160 });
        self.clipped((x, y + h - 2, w, 2), |p| p.round((x, y, w, h), t.radius, None, Some(rim), 1.0));
    }

    /// The keyboard focus on a control `rect` that draws its own (a button,
    /// a check box's box): RapidR's look rings it in the focus colour —
    /// with a hairline of the window's colour inside on an accent fill, so
    /// the ring shows; the classic look's dotted rectangle.
    pub fn focus_ring(&mut self, rect: Rect, on_accent: bool) {
        let t = self.theme;
        if !t.fluent() {
            self.focus(rect);
            return;
        }
        self.ring(rect, t.radius, t.focus, t.focus_width);
        if on_accent {
            let k = t.focus_width as i64;
            self.ring((rect.0 + k, rect.1 + k, rect.2 - 2 * k, rect.3 - 2 * k), (t.radius - t.focus_width).max(1.0), t.window, 1.0);
        }
    }

    /// What floats over the window (a menu, a drop-down list, a tooltip, a
    /// window on the page) lifted off it: the theme's soft shadow around
    /// `rect`, `size` pixels deep (RapidR's look; nothing where the theme
    /// casts none — classic, high contrast). Drawn before the thing itself.
    pub fn elevate(&mut self, rect: Rect, radius: f64, size: f64) {
        let t = self.theme;
        if t.shadow_alpha > 0 && rect.2 > 0 && rect.3 > 0 {
            self.op(Op::Shadow { rect, radius, size, drop: (size / 4.0).round(), color: t.shadow_ink, alpha: t.shadow_alpha });
        }
    }

    /// A rounded frame `width` pixels wide inside `rect`.
    pub fn ring(&mut self, rect: Rect, radius: f64, color: u32, width: f64) {
        self.round(rect, radius, None, Some(color), width);
    }

    /// Line segments through `points`, `width` pixels wide (smooth).
    pub fn stroke(&mut self, points: &[(f64, f64)], color: u32, width: f64) {
        self.op(Op::Stroke { points: points.to_vec(), color, width });
    }

    // ---- the classic look's edges, in the theme's colours ----

    /// Windows' sunken client edge (EDGE_SUNKEN, two lines): a text box's,
    /// a list's, a check box's box.
    pub fn sunken_edge(&mut self, rect: Rect) {
        let t = self.theme;
        self.edge(rect, &[t.shadow, t.dark_shadow], &[t.light, t.light3d]);
    }

    /// EDGE_RAISED: a menu's, a window's border, a scroll, combo or up-down
    /// button's.
    pub fn raised_edge(&mut self, rect: Rect) {
        let t = self.theme;
        self.edge(rect, &[t.light3d, t.light], &[t.dark_shadow, t.shadow]);
    }

    /// A push button's (DFCS_BUTTONPUSH): white then COLOR_3DLIGHT above,
    /// the dark shadow then the shadow below.
    pub fn button_edge(&mut self, rect: Rect) {
        let t = self.theme;
        self.edge(rect, &[t.light, t.light3d], &[t.dark_shadow, t.shadow]);
    }

    /// One line in, shaded above (BDR_SUNKENOUTER): a status panel's, a
    /// progress bar's, a pressed flat button's.
    pub fn thin_sunken(&mut self, rect: Rect) {
        let t = self.theme;
        self.edge(rect, &[t.shadow], &[t.light]);
    }

    /// One line out, lit above (BDR_RAISEDINNER): a splitter's, a hot flat
    /// button's.
    pub fn thin_raised(&mut self, rect: Rect) {
        let t = self.theme;
        self.edge(rect, &[t.light], &[t.shadow]);
    }

    /// A frame of one colour.
    pub fn frame(&mut self, rect: Rect, color: u32) {
        self.edge(rect, &[color], &[color]);
    }

    // ---- the fluent look's shapes ----

    /// A fluent box `w` × `h` for text or items (a text box, a list, a
    /// tree, a grid): rounded, filled `fill`, a hairline border; ringed in
    /// the focus colour when `focused` is `Some(true)` (a box whose
    /// component draws its own focus; lists, trees and grids are ringed by
    /// the form, [`ComponentKind::field`]).
    ///
    /// [`ComponentKind::field`]: crate::components::ComponentKind::field
    pub fn fluent_field(&mut self, w: i64, h: i64, fill: u32, focused: Option<bool>) {
        let t = self.theme;
        self.round((0, 0, w, h), t.radius, Some(fill), Some(t.border), 1.0);
        if focused == Some(true) {
            self.field_focus(w, h);
        }
    }

    /// The keyboard focus on a text box `w` × `h`: RapidR's look rings the
    /// whole box in the theme's focus colour, two pixels (the text is three
    /// in). The classic look draws its own focus.
    pub fn field_focus(&mut self, w: i64, h: i64) {
        let t = self.theme;
        if t.fluent() {
            self.ring((0, 0, w, h), t.radius, t.focus, t.focus_width.min(2.0));
        }
    }

    /// The keyboard focus on a box of items (a [`ComponentKind::field`]: a
    /// list, a tree, a grid): its border in the focus colour — its
    /// selection, the accent's while it has the focus, says the rest (high
    /// contrast: the theme's thick ring).
    ///
    /// [`ComponentKind::field`]: crate::components::ComponentKind::field
    pub fn items_focus(&mut self, w: i64, h: i64) {
        let t = self.theme;
        if t.fluent() {
            self.ring((0, 0, w, h), t.radius, t.focus, if t.contrast { 2.0 } else { 1.0 });
        }
    }

    /// A fluent check mark in a `side` pixels square at (x, y).
    pub fn check_glyph(&mut self, x: f64, y: f64, side: f64, color: u32) {
        let k = side / 13.0;
        self.stroke(&[(x + 3.2 * k, y + 6.8 * k), (x + 5.6 * k, y + 9.2 * k), (x + 10.0 * k, y + 4.2 * k)], color, 1.5 * k.max(1.0));
    }

    /// A chevron pointing down (`down`) or right, centred on (cx, cy),
    /// `size` pixels across: a fluent drop-down's, a submenu's.
    pub fn chevron(&mut self, cx: f64, cy: f64, size: f64, down: bool, color: u32) {
        let s = size / 2.0;
        let points = if down { [(cx - s, cy - s / 2.0), (cx, cy + s / 2.0), (cx + s, cy - s / 2.0)] } else { [(cx - s / 2.0, cy - s), (cx + s / 2.0, cy), (cx - s / 2.0, cy + s)] };
        self.stroke(&points, color, 1.0);
    }

    /// Whether the screen is 1× (one device pixel per logical pixel): the
    /// classic look then draws its small round glyphs (a radio button's
    /// well, …) pixel for pixel as Windows does; on a high-DPI screen it
    /// draws them as smooth shapes at the screen's resolution instead.
    pub fn one_to_one(&self) -> bool {
        (self.scale() - 1.0).abs() < 1e-9
    }

    /// Pixel art at (x, y): `rows` of characters, one per pixel, each in
    /// the colour `palette` gives its character (others, `.`: nothing) —
    /// a row's runs of one colour as one fill.
    pub fn pixels(&mut self, x: i64, y: i64, rows: &[&str], palette: &[(char, u32)]) {
        for (j, row) in rows.iter().enumerate() {
            let chars: Vec<char> = row.chars().collect();
            let mut i = 0;
            while i < chars.len() {
                let c = chars[i];
                let start = i;
                while i < chars.len() && chars[i] == c {
                    i += 1;
                }
                if let Some(&(_, color)) = palette.iter().find(|(k, _)| *k == c) {
                    self.fill((x + start as i64, y + j as i64, (i - start) as i64, 1), color);
                }
            }
        }
    }

    /// A disc of radius `r` around (cx, cy) — or its sector from `from` to
    /// `to` degrees (counter-clockwise from 3 o'clock, y up) — as a smooth
    /// polygon, at the screen's resolution.
    pub fn sector(&mut self, c: (f64, f64), r: f64, from: f64, to: f64, color: u32) {
        self.ellipse(c, (r, r), from, to, color);
    }

    /// [`Painter::sector`] of an ellipse of radii (rx, ry).
    pub fn ellipse(&mut self, (cx, cy): (f64, f64), (rx, ry): (f64, f64), from: f64, to: f64, color: u32) {
        let r = rx.max(ry);
        let full = (to - from).abs() >= 360.0;
        let steps = ((r * self.scale() * 2.0).ceil() as usize).clamp(16, 256);
        let mut points = Vec::with_capacity(steps + 2);
        if !full {
            points.push((cx, cy));
        }
        for k in 0..=steps {
            let a = (from + (to - from) * k as f64 / steps as f64).to_radians();
            points.push((cx + rx * a.cos(), cy - ry * a.sin()));
        }
        if full {
            points.pop();
        }
        self.op(Op::Polygon { points, color });
    }

    /// Windows' classic arrow glyph (DrawFrameControl's, an up-down's
    /// halves) in button `r`, pointing `dir`: `d` rows deep — a quarter of
    /// the button's shorter side (3 in a 17 × 12 half, 4 in a 17 × 17 one) —
    /// placed as Windows places it (RapidQ's capture); a pixel down and
    /// right while pushed. Pixel for pixel at 1×; a smooth triangle over the
    /// same cells at a high-DPI screen's resolution.
    pub fn classic_arrow(&mut self, r: Rect, dir: Dir, color: u32, pushed: bool) {
        let (x, y, w, h) = r;
        let d = (w.min(h) / 4).max(1);
        let span = 2 * d - 1;
        let k = i64::from(pushed);
        // (the glyph's cell box: left, top, width, height)
        let (bx, by, bw, bh) = match dir {
            Dir::Up | Dir::Down => (x + (w - span - 2) / 2 + k, y + (h - d) / 2 + k, span, d),
            Dir::Left => (x + (w - d - 2) / 2 + k, y + (h - span) / 2 + k, d, span),
            Dir::Right => (x + (w - d + 1) / 2 + k, y + (h - span) / 2 + k, d, span),
        };
        if self.one_to_one() {
            for i in 0..d {
                // (row / column i from the tip: 2i + 1 cells)
                let (len, off) = (2 * i + 1, d - 1 - i);
                let cell = match dir {
                    Dir::Up => (bx + off, by + i, len, 1),
                    Dir::Down => (bx + off, by + d - 1 - i, len, 1),
                    Dir::Left => (bx + i, by + off, 1, len),
                    Dir::Right => (bx + d - 1 - i, by + off, 1, len),
                };
                self.fill(cell, color);
            }
            return;
        }
        let (l, t, rr, b) = (bx as f64, by as f64, (bx + bw) as f64, (by + bh) as f64);
        let points = match dir {
            Dir::Up => [(l, b), (rr, b), ((l + rr) / 2.0, t)],
            Dir::Down => [(l, t), (rr, t), ((l + rr) / 2.0, b)],
            Dir::Left => [(rr, t), (rr, b), (l, (t + b) / 2.0)],
            Dir::Right => [(l, t), (l, b), (rr, (t + b) / 2.0)],
        };
        self.op(Op::Arrow { points, color });
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

    /// Clips what `f` draws to a polygon (logical points from the origin).
    pub fn clipped_polygon(&mut self, points: Vec<(f64, f64)>, f: impl FnOnce(&mut Painter)) {
        self.op(Op::ClipPolygon { points });
        f(self);
        self.op(Op::ClipPop);
    }

    /// Clips what `f` draws to `rect`.
    pub fn clipped(&mut self, rect: Rect, f: impl FnOnce(&mut Painter)) {
        self.op(Op::ClipPush { rect });
        f(self);
        self.op(Op::ClipPop);
    }

    /// A picture drawn into `rect`: `source` names it in the display list
    /// (unique per picture; `revision` changes when its pixels do).
    pub fn picture(&mut self, source: &str, revision: u64, picture: impl Into<std::sync::Arc<crate::display::Picture>>, rect: Rect) {
        let picture = picture.into();
        if rect.2 <= 0 || rect.3 <= 0 || picture.width == 0 || picture.height == 0 {
            return;
        }
        self.list.images.insert(source.to_string(), picture);
        self.op(Op::Image { source: source.to_string(), revision, rect });
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
    // (the underline: the second pixel row under the baseline, as GDI's)
    let y = (top as f64 + f64::from(rapidr_value::objects::text::ascent(font))).round() + 1.5;
    p.line(((left + x) as f64 + 0.5, y), ((left + x + w - 1) as f64 + 0.5, y), color);
}

/// Whether the program chose component `id`'s Font.Color: any colour but
/// RapidQ's default black, or black set (`Font.Color` / `FontColor`,
/// which the store keeps as `font.color` too; a QFONT assigned sets it).
pub fn font_color_chosen(store: &dyn Store, id: &str, font: &Font) -> bool {
    font.color & 0xFFFFFF != 0 || !matches!(store.get(id, "font.color"), Value::Null)
}

/// The colour component `id`'s text is drawn in, on `background`
/// (0xRRGGBB): grey while disabled; else its Font.Color if the program
/// chose one — the program's colours win in every theme, as in RapidQ —
/// else the theme's text (what reads on `background` when that's a colour
/// the program chose).
pub fn ink(store: &dyn Store, id: &str, font: &Font, enabled: bool, background: u32) -> u32 {
    let t = theme::current();
    if !enabled {
        t.gray_text
    } else if font_color_chosen(store, id, font) {
        bgr_to_rgb(font.color)
    } else {
        t.text_on(background)
    }
}

/// [`ink`] for the component being drawn.
pub fn ink_of(cx: &Cx, background: u32) -> u32 {
    ink(cx.store, cx.id, &cx.font, cx.state.enabled, background)
}

/// `cx`'s font with its colour the one its text is drawn in on
/// `background` ([`ink`], enabled: the shared models grey what's disabled
/// themselves), as &HBBGGRR — for the models, which draw a font's colour.
pub fn inked(cx: &Cx, background: u32) -> Font {
    Font { color: theme::bgr(ink(cx.store, cx.id, &cx.font, true, background)) as i64, ..cx.font.clone() }
}

/// Component `id`'s Color as 0xRRGGBB when the program set one (a form's,
/// a panel's and a label's creation default reads as unset).
pub fn color_of(store: &dyn Store, id: &str) -> Option<u32> {
    match store.get(id, "color") {
        Value::Null => None,
        v => Some(bgr_to_rgb(rapidr_value::objects::form_color(&v))),
    }
}

/// What a component without a background of its own (a label, a check
/// box's caption) is drawn on: the nearest Color the program set — its
/// own, its parent's, … — else the theme's face. (Only the fluent looks
/// ask: the classic look's text is RapidQ's black whatever it's on.)
pub fn backdrop(store: &dyn Store, id: &str) -> u32 {
    let t = theme::current();
    if !t.fluent() {
        return t.face;
    }
    let mut at = id.to_string();
    for _ in 0..64 {
        if let Some(c) = color_of(store, &at) {
            return c;
        }
        let parent = store::string(store, &at, "parent");
        if parent.is_empty() || parent.eq_ignore_ascii_case(&at) {
            break;
        }
        at = parent;
    }
    t.face
}

/// What shows behind component `id` where it draws nothing (a pie gauge's
/// corners): the nearest Color the program set on its parents, else the
/// theme's face — in every theme.
pub fn behind(store: &dyn Store, id: &str) -> u32 {
    let mut at = store::string(store, id, "parent");
    for _ in 0..64 {
        if at.is_empty() {
            break;
        }
        if let Some(c) = color_of(store, &at) {
            return c;
        }
        let parent = store::string(store, &at, "parent");
        if parent.eq_ignore_ascii_case(&at) {
            break;
        }
        at = parent;
    }
    theme::current().face
}

/// A scroll bar model's bars for an area `w` × `h`, as the theme draws
/// them: the model's own (classic: raised buttons, a raised thumb, in the
/// theme's colours); thin (fluent: a pill of a thumb on a quiet track,
/// small arrows) — the same parts in the same places, so the mouse does
/// the same.
pub fn bar_ops(sc: &Scroller, w: i64, h: i64) -> Vec<Op> {
    let t = theme::current();
    if !t.fluent() {
        return lift(sc.ops(w, h));
    }
    let mut out = Vec::new();
    let (cw, ch) = sc.client(w, h);
    if sc.vert.shown && sc.horz.shown {
        out.push(Op::Fill { rect: (cw, ch, BAR, BAR), color: t.track });
    }
    for vertical in [false, true] {
        let Some((bx, by, bw, bh)) = sc.bar(vertical, w, h) else { continue };
        let pressed = |p: Part| sc.pressed == Some((vertical, p));
        out.push(Op::Fill { rect: (bx, by, bw, bh), color: t.track });
        let len = if vertical { bh } else { bw };
        let at = |along: i64, size: i64| if vertical { (bx, by + along, BAR, size) } else { (bx + along, by, size, BAR) };
        if let Some((th, ts)) = sc.thumb(vertical, len, w, h) {
            if pressed(Part::PageBack) {
                out.push(Op::Fill { rect: at(BAR, th - BAR), color: t.track_pressed });
            }
            if pressed(Part::PageForward) {
                out.push(Op::Fill { rect: at(th + ts, len - BAR - th - ts), color: t.track_pressed });
            }
            // (a pill across the bar's middle, 2 pixels short of each end)
            let (a, s) = (th + 2, (ts - 4).max(2));
            let rect = if vertical { (bx + 6, by + a, 5, s) } else { (bx + a, by + 6, s, 5) };
            let color = if pressed(Part::Thumb) { t.text } else { t.border_strong };
            out.push(Op::Round { rect, radius: 2.5, fill: Some(color), stroke: None, width: 1.0 });
        }
        for (part, along) in [(Part::Back, 0), (Part::Forward, len - BAR)] {
            let (x, y, rw, rh) = at(along, BAR.min(len));
            let (cx, cy) = (x as f64 + rw as f64 / 2.0, y as f64 + rh as f64 / 2.0);
            let s = 3.0;
            let points = match (vertical, part == Part::Forward) {
                (false, false) => [(cx + s / 2.0, cy - s), (cx + s / 2.0, cy + s), (cx - s / 2.0 - 0.5, cy)],
                (false, true) => [(cx - s / 2.0, cy - s), (cx - s / 2.0, cy + s), (cx + s / 2.0 + 0.5, cy)],
                (true, false) => [(cx - s, cy + s / 2.0), (cx + s, cy + s / 2.0), (cx, cy - s / 2.0 - 0.5)],
                (true, true) => [(cx - s, cy - s / 2.0), (cx + s, cy - s / 2.0), (cx, cy + s / 2.0 + 0.5)],
            };
            out.push(Op::Arrow { points, color: if pressed(part) { t.text } else { t.border_strong } });
        }
    }
    out
}

impl FormUi {
    /// The form drawn for a screen of `scale` device pixels per logical
    /// pixel. Geometry and captions are read from the store now.
    pub fn paint(&mut self, store: &dyn Store, ts: &mut TextSystem, scale: f64) -> DisplayList {
        self.sync(store);
        self.scale = scale;
        let (w, h) = self.client;
        let mut list = DisplayList { size: (w, h + self.menu_offset), scale, ..Default::default() };
        let mut p = Painter::new(&mut list);
        if self.menu_offset > 0 {
            // (the in-window menu bar: components/menubar.rs)
            self.paint_menu_bar(store, &mut p);
        }
        let color = color_of(store, &self.form).unwrap_or(p.theme().face);
        p.fill((0, self.menu_offset, w, h), color);
        // (the form's own drawing surface, under its components)
        crate::components::canvas::paint_form_surface(self, &mut p);
        let default = self.default_button(store);
        let focused_is_button = self.focus.is_some_and(|f| self.nodes[f].type_name == "RBUTTON");
        for root in self.roots() {
            self.paint_node(root, store, ts, &mut p, default, focused_is_button);
        }
        // (the form's scroll bars, over its components)
        crate::components::scrollbox::paint_form_bars(&self.form, (w, h), &mut p, self.menu_offset);
        if !self.popups_apart {
            // (an open drop-down list, over them)
            crate::components::combo::paint_popup(self, store, ts, &mut p);
            // (open menus over everything)
            self.paint_menus(store, &mut p);
            // (a tooltip over them)
            self.paint_tip(&mut p);
        }
        self.dirty = false;
        // (the caret's blink: tick.rs)
        self.arm_caret();
        list
    }

    /// With [`FormUi::popups_apart`]: the open drop-down list and menus
    /// alone, on nothing (an empty list: none open), the size of
    /// [`FormUi::paint`]'s.
    pub fn paint_popups(&mut self, store: &dyn Store, ts: &mut TextSystem, scale: f64) -> DisplayList {
        let (w, h) = self.client;
        let mut list = DisplayList { size: (w, h + self.menu_offset), scale, ..Default::default() };
        let mut p = Painter::new(&mut list);
        crate::components::combo::paint_popup(self, store, ts, &mut p);
        self.paint_menus(store, &mut p);
        self.paint_tip(&mut p);
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
            // (drawn as its own Enabled says: Windows draws the children of a
            // disabled window — a panel, a group box, a form under a modal one —
            // as they are, though they take no input, RapidQ's capture shows)
            enabled: store::flag(store, &self.nodes[i].id, "enabled", true),
            caret_on: self.caret_on,
            default_frame: default == Some(i) && !focused_is_button,
        };
        let children = self.children(i);
        let (scale, system_corner) = (self.scale, self.system_corner);
        let room = self.nodes[i].kind.map_or((0, 0, w, h), |k| k.room(store, &self.nodes[i].id, &store.font(&self.nodes[i].id), w, h));
        p.at((x, y), |p| {
            p.clipped(room, |p| {
                let node = &mut self.nodes[i];
                if let Some(kind) = node.kind {
                    // (painting fires nothing)
                    let mut events = Vec::new();
                    let mut cx = Cx { store, text: ts, id: &node.id, rect: node.abs, font: store.font(&node.id), state, ui: &mut node.ui, events: &mut events, scale, system_corner };
                    kind.paint(&mut cx, p);
                    if state.focused && kind.field() {
                        p.items_focus(w, h);
                    }
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

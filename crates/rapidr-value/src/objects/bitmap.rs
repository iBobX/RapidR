//! QBITMAP (manual, Appendix A): an off-screen image to draw on, load and
//! save as BMP. Colors are RapidQ's &HBBGGRR integers. Rectangles follow
//! Windows: the right and bottom edges are excluded.
//!
//! A QIMAGE's picture is one too (`picture`): the same loading and drawing,
//! while its Width / Height are the control's, not the picture's; drawing
//! on an image without a picture first gives it one the control's size, as
//! Delphi's TImage does.
//!
//! A QCANVAS is one too (`canvas`): the same drawing methods and text, on a
//! surface that is always the control's size, filled with its `Color`. So
//! the desktop and the web show the same pixels. RapidR's own canvas
//! methods (`DrawText`, `Cls`, `Circle(cx, cy, r)`, `FillCircle`,
//! `SetFont`, `PenColor` / `BrushColor` as the default colors) are kept.
//!
//! High-DPI screens (Retina, a browser at 2×): the pixels a program reads
//! and saves (`Pixel`, `.BMP`, flood fills) are always the bitmap's own,
//! `img`, exactly as on a 1× screen. Next to them a bitmap keeps what the
//! screen shows, `hi`, at the screen's scale (see [`set_display_scale`]):
//! every drawing call draws it too at full resolution — lines, ellipses
//! and text finer, SVGs drawn at that scale — and the runtimes show it
//! in the bitmap's size, so nothing looks blurry or blocky.

use super::codec::{base64_encode, bmp_data_url, decode_svg, encode_bmp_alpha, is_svg, Pixels, BMP_DATA_URL, MAX_PIXELS, SVG_DATA_URL};
use super::font::Font;
use crate::{v_int, v_str, Value};
use std::cell::Cell;

thread_local! {
    static DISPLAY_SCALE: Cell<usize> = const { Cell::new(1) };
    static EXACT_SCALE: Cell<f64> = const { Cell::new(1.0) };
}

/// The screen's scale (device pixels per pixel, e.g. 2 on a Retina
/// screen): bitmaps keep what they show at it (rounded up, at most 3).
pub fn set_display_scale(scale: f64) {
    let s = if scale.is_finite() { scale.ceil().clamp(1.0, 3.0) as usize } else { 1 };
    DISPLAY_SCALE.with(|d| d.set(s));
    if scale.is_finite() && scale > 0.0 {
        EXACT_SCALE.with(|d| d.set(scale));
    }
}

/// The screen's scale as noted, not rounded (1.5 on a 150 % screen): what
/// `Screen.Scale` reads on the desktop.
pub fn exact_scale() -> f64 {
    EXACT_SCALE.with(Cell::get)
}

pub fn display_scale() -> usize {
    DISPLAY_SCALE.with(Cell::get)
}

/// What a bitmap shows on a high-DPI screen: its pixels `scale` times
/// finer (with each one's opacity, for soft-edged images).
#[derive(Debug, Clone)]
pub struct HiRes {
    pub scale: usize,
    pub img: Pixels,
    pub alpha: Option<Vec<u8>>,
}

impl HiRes {
    /// `lo` shown `scale` times larger (each pixel a scale × scale block).
    fn upscaled(lo: &Pixels, alpha: Option<&[u8]>, scale: usize) -> Option<HiRes> {
        let (w, h) = (lo.width * scale, lo.height * scale);
        if w.saturating_mul(h) > MAX_PIXELS {
            return None;
        }
        let mut pixels = Vec::with_capacity(w * h);
        let mut hi_alpha = alpha.map(|_| Vec::with_capacity(w * h));
        for y in 0..h {
            let row = (y / scale) * lo.width;
            for x in 0..w {
                pixels.push(lo.pixels[row + x / scale]);
                if let (Some(out), Some(a)) = (hi_alpha.as_mut(), alpha) {
                    out.push(a[row + x / scale]);
                }
            }
        }
        Some(HiRes { scale, img: Pixels { width: w, height: h, pixels }, alpha: hi_alpha })
    }

    pub fn pixel(&self, x: i64, y: i64) -> Option<u32> {
        if x < 0 || y < 0 || x as usize >= self.img.width || y as usize >= self.img.height {
            return None;
        }
        Some(self.img.pixels[y as usize * self.img.width + x as usize])
    }

    /// Sets a device pixel (drawn: opaque).
    pub fn put(&mut self, x: i64, y: i64, c: u32) {
        if x >= 0 && y >= 0 && (x as usize) < self.img.width && (y as usize) < self.img.height {
            let i = y as usize * self.img.width + x as usize;
            self.img.pixels[i] = c;
            if let Some(a) = self.alpha.as_mut().filter(|a| a.len() > i) {
                a[i] = 255;
            }
        }
    }

    /// Fills device pixels [l, r) × [t, b).
    fn fill(&mut self, l: i64, t: i64, r: i64, b: i64, c: u32) {
        let (w, h) = (self.img.width as i64, self.img.height as i64);
        let (l, t, r, b) = (l.clamp(0, w), t.clamp(0, h), r.clamp(0, w), b.clamp(0, h));
        for y in t..b {
            for x in l..r {
                self.put(x, y, c);
            }
        }
    }

    /// Pixel (x, y) of the bitmap: its scale × scale block.
    fn block(&mut self, x: i64, y: i64, c: u32) {
        let s = self.scale as i64;
        self.fill(x * s, y * s, x * s + s, y * s + s, c);
    }

    /// The line from pixel (x1, y1) to (x2, y2) of the bitmap, a pixel
    /// wide: device-pixel steps under a square pen.
    fn line(&mut self, x1: i64, y1: i64, x2: i64, y2: i64, c: u32) {
        let s = self.scale as i64;
        bresenham(x1 * s, y1 * s, x2 * s, y2 * s, |x, y| self.fill(x, y, x + s, y + s, c));
    }

    /// The ellipse inside pixels (x1,y1)-(x2,y2) of the bitmap: filled, or
    /// outlined a pixel (scale device pixels) thick.
    fn ellipse(&mut self, x1: i64, y1: i64, x2: i64, y2: i64, c: u32, fill: bool) {
        let s = self.scale as i64;
        let (l, t, r, b) = (x1.min(x2) * s, y1.min(y2) * s, x1.max(x2) * s, y1.max(y2) * s);
        let rings = if fill { 1 } else { s };
        for k in 0..rings {
            ellipse_spans(l + k, t + k, r - k, b - k, fill, |xa, xb, y| {
                for x in xa..=xb {
                    self.put(x, y, c);
                }
            });
        }
    }

    fn to_rgba(&self, transparent: Option<u32>) -> Vec<u8> {
        let soft = self.alpha.as_deref().filter(|a| a.len() == self.img.pixels.len());
        let mut out = Vec::with_capacity(self.img.pixels.len() * 4);
        for (i, &c) in self.img.pixels.iter().enumerate() {
            let alpha = if transparent == Some(c) { 0 } else { soft.map_or(255, |a| a[i]) };
            out.extend_from_slice(&[c as u8, (c >> 8) as u8, (c >> 16) as u8, alpha]);
        }
        out
    }
}

/// The points of the line (x1, y1)–(x2, y2), both ends included.
fn bresenham(x1: i64, y1: i64, x2: i64, y2: i64, mut put: impl FnMut(i64, i64)) {
    let (dx, dy) = ((x2 - x1).abs(), -(y2 - y1).abs());
    let (sx, sy) = (if x1 < x2 { 1 } else { -1 }, if y1 < y2 { 1 } else { -1 });
    let (mut x, mut y, mut err) = (x1, y1, dx + dy);
    // Bounded so a huge coordinate can't loop for ages off-bitmap.
    for _ in 0..=(dx - dy).min(1 << 21) {
        put(x, y);
        if x == x2 && y == y2 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

/// The rows of the ellipse inside (x1,y1)-(x2,y2) (right and bottom edges
/// excluded): `span(xa, xb, y)` for each run of it, filled or outlined.
fn ellipse_spans(x1: i64, y1: i64, x2: i64, y2: i64, fill: bool, mut span: impl FnMut(i64, i64, i64)) {
    let (l, t, r, b) = (x1.min(x2) as f64, y1.min(y2) as f64, (x1.max(x2) - 1) as f64, (y1.max(y2) - 1) as f64);
    let (cx, cy, rx, ry) = ((l + r) / 2.0, (t + b) / 2.0, (r - l) / 2.0, (b - t) / 2.0);
    if rx < 0.0 || ry < 0.0 {
        return;
    }
    let mut prev: Option<(i64, i64)> = None;
    for y in t as i64..=b as i64 {
        let dy = if ry == 0.0 { 0.0 } else { (y as f64 - cy) / ry };
        let half = rx * (1.0 - dy * dy).max(0.0).sqrt();
        let (xa, xb) = ((cx - half).round() as i64, (cx + half).round() as i64);
        if fill {
            span(xa, xb, y);
        } else {
            // Each side spans to where the previous row's was, so the
            // flat top and bottom and the steep sides have no gaps.
            let (pa, pb) = prev.unwrap_or((xa, xb));
            span(xa.min(pa), xa.max(pa), y);
            span(xb.min(pb), xb.max(pb), y);
        }
        prev = Some((xa, xb));
    }
}

#[derive(Debug, Clone)]
pub struct Bitmap {
    pub img: Pixels,
    pub transparent: bool,
    pub transparent_color: u32,
    /// A QIMAGE's picture (see the module docs).
    pub picture: bool,
    /// The font TextOut draws with (`Bitmap.Font = Font`, `Font.Size`, …).
    pub font: Font,
    /// A QCANVAS (see the module docs).
    pub canvas: bool,
    /// A canvas's `Color`: what `Cls` fills with and new area shows.
    pub background: u32,
    /// A QFORM's own surface (`Form.TextOut`, `Form.Line`, …): a canvas
    /// under the form's controls whose `background` shows the form through
    /// (it is the form's `Color`), and whose font is the form's.
    pub form: bool,
    /// A canvas's `PenColor` / `BrushColor`: the colors drawing uses when
    /// none is given.
    pub pen: u32,
    pub brush: u32,
    /// Each pixel's opacity, for an image with soft edges (an SVG); used
    /// only while it matches the pixels (drawing on it makes pixels opaque).
    pub alpha: Option<Vec<u8>>,
    /// What the screen shows at a high-DPI scale (module docs); made from
    /// `img` when missing or out of date.
    pub(crate) hi: Option<Box<HiRes>>,
    /// The SVG these pixels were drawn from, until something draws on them:
    /// what's shown is drawn from it again at a new screen scale.
    pub(crate) svg: Option<std::rc::Rc<Vec<u8>>>,
}

/// `img` cut or padded (transparent) to w × h.
fn fit_hi(img: Pixels, alpha: Vec<u8>, w: usize, h: usize, scale: usize) -> HiRes {
    let mut pixels = vec![0; w * h];
    let mut a = vec![0u8; w * h];
    for y in 0..h.min(img.height) {
        for x in 0..w.min(img.width) {
            pixels[y * w + x] = img.pixels[y * img.width + x];
            a[y * w + x] = alpha[y * img.width + x];
        }
    }
    HiRes { scale, img: Pixels { width: w, height: h, pixels }, alpha: Some(a) }
}

/// `over` (RapidQ &HBBGGRR) at opacity `a` over `under`.
fn blend(under: u32, over: u32, a: u8) -> u32 {
    let a = u32::from(a);
    let ch = |shift: u32| (((over >> shift) & 0xFF) * a + ((under >> shift) & 0xFF) * (255 - a) + 127) / 255;
    ch(16) << 16 | ch(8) << 8 | ch(0)
}

/// Color of a new bitmap's pixels.
const BACKGROUND: u32 = 0xFFFFFF;

impl Default for Bitmap {
    fn default() -> Self {
        Self {
            img: Pixels { width: 0, height: 0, pixels: Vec::new() },
            transparent: false,
            transparent_color: BACKGROUND,
            picture: false,
            font: Font::default(),
            canvas: false,
            form: false,
            background: BACKGROUND,
            pen: 0,
            brush: BACKGROUND,
            alpha: None,
            hi: None,
            svg: None,
        }
    }
}

impl Bitmap {
    pub fn from_pixels(img: Pixels) -> Self {
        Self { img, ..Self::default() }
    }

    /// A QCANVAS's surface.
    pub fn new_canvas() -> Self {
        Self { canvas: true, ..Self::default() }
    }

    /// A QFORM's surface: transparent where nothing is drawn.
    pub fn new_form_surface(color: u32) -> Self {
        let color = color & 0xFFFFFF;
        Self { canvas: true, form: true, transparent: true, transparent_color: color, background: color, ..Self::default() }
    }

    /// A control's surface (QIMAGE picture, QCANVAS): Width / Height are the
    /// control's, not the bitmap's.
    fn surface(&self) -> bool {
        self.picture || self.canvas
    }

    /// A canvas takes its control's size (new area shows the background).
    pub fn fit(&mut self, width: i64, height: i64) {
        if self.canvas && (self.img.width as i64, self.img.height as i64) != (width.clamp(0, 32767), height.clamp(0, 32767)) {
            self.resize(width, height);
        }
    }

    /// Resizes, keeping the pixels that still fit.
    pub fn resize(&mut self, width: i64, height: i64) {
        let (w, h) = (width.clamp(0, 32767) as usize, height.clamp(0, 32767) as usize);
        if w * h > MAX_PIXELS {
            return;
        }
        if (w, h) != (self.img.width, self.img.height) {
            self.svg = None;
        }
        let mut pixels = vec![self.background; w * h];
        for y in 0..h.min(self.img.height) {
            for x in 0..w.min(self.img.width) {
                pixels[y * w + x] = self.img.pixels[y * self.img.width + x];
            }
        }
        // What's shown keeps its fine pixels too.
        if let Some(hi) = self.hi.as_mut() {
            let s = hi.scale;
            let (hw, hh) = (w * s, h * s);
            if hw.saturating_mul(hh) <= MAX_PIXELS && hi.alpha.is_none() {
                let mut hp = vec![self.background; hw * hh];
                for y in 0..hh.min(hi.img.height) {
                    for x in 0..hw.min(hi.img.width) {
                        hp[y * hw + x] = hi.img.pixels[y * hi.img.width + x];
                    }
                }
                hi.img = Pixels { width: hw, height: hh, pixels: hp };
            } else {
                self.hi = None;
            }
        }
        self.img = Pixels { width: w, height: h, pixels };
    }

    /// What the screen shows, at the display scale (made from the pixels
    /// when it's missing or out of date); `None` at 1×.
    fn hi_mut(&mut self) -> Option<&mut HiRes> {
        let s = display_scale();
        if s <= 1 {
            self.hi = None;
            return None;
        }
        let fits = self.hi.as_ref().is_some_and(|h| h.scale == s && h.img.width == self.img.width * s && h.img.height == self.img.height * s);
        if !fits {
            // An SVG's pixels: drawn again at this scale; others: enlarged.
            let (w, h) = (self.img.width * s, self.img.height * s);
            let from_svg = self.svg.as_ref().and_then(|svg| decode_svg(svg, s as f32).ok()).map(|(fine, a)| fit_hi(fine, a, w, h, s));
            self.hi = from_svg.or_else(|| HiRes::upscaled(&self.img, self.alpha_channel(), s)).map(Box::new);
        }
        self.hi.as_deref_mut()
    }

    /// What the screen shows, for drawing on it (text.rs); `None` at 1×.
    pub(crate) fn display_mut(&mut self) -> Option<&mut HiRes> {
        self.hi_mut()
    }

    /// Drops what the screen shows (the pixels changed some other way).
    pub fn invalidate_display(&mut self) {
        self.hi = None;
    }

    /// Takes `src`'s high-DPI pixels along with its pixels (`BMP = …`).
    pub fn take_display(&mut self, src: &mut Bitmap) {
        self.hi = src.hi.take();
        self.svg = src.svg.take();
    }

    /// Whether these pixels are an SVG's, not drawn on since.
    pub fn is_svg(&self) -> bool {
        self.svg.is_some()
    }

    /// What the screen shows: (width, height, RGBA, scale) — at the display
    /// scale when there is one, else the pixels themselves (scale 1). The
    /// runtimes draw it in the bitmap's size (width / scale × height / scale).
    pub fn display_rgba(&mut self) -> (usize, usize, Vec<u8>, usize) {
        let transparent = self.transparent.then_some(self.transparent_color);
        if self.img.pixels.is_empty() {
            return (0, 0, Vec::new(), 1);
        }
        match self.hi_mut() {
            Some(hi) => (hi.img.width, hi.img.height, hi.to_rgba(transparent), hi.scale),
            None => (self.img.width, self.img.height, self.to_rgba(), 1),
        }
    }

    pub fn pixel(&self, x: i64, y: i64) -> Option<u32> {
        if x < 0 || y < 0 || x as usize >= self.img.width || y as usize >= self.img.height {
            return None;
        }
        Some(self.img.pixels[y as usize * self.img.width + x as usize])
    }

    /// Sets one of the pixels (only: what the screen shows is the caller's).
    pub(crate) fn lo_pset(&mut self, x: i64, y: i64, c: u32) {
        self.svg = None;
        if x >= 0 && y >= 0 && (x as usize) < self.img.width && (y as usize) < self.img.height {
            let i = y as usize * self.img.width + x as usize;
            self.img.pixels[i] = c;
            // (what's drawn is opaque)
            if let Some(a) = self.alpha.as_mut().filter(|a| a.len() > i) {
                a[i] = 255;
            }
        }
    }

    pub fn pset(&mut self, x: i64, y: i64, c: u32) {
        self.lo_pset(x, y, c);
        if self.pixel(x, y).is_some() {
            if let Some(hi) = self.hi_mut() {
                hi.block(x, y, c);
            }
        }
    }

    fn lo_line(&mut self, x1: i64, y1: i64, x2: i64, y2: i64, c: u32) {
        bresenham(x1, y1, x2, y2, |x, y| self.lo_pset(x, y, c));
    }

    pub fn line(&mut self, x1: i64, y1: i64, x2: i64, y2: i64, c: u32) {
        self.lo_line(x1, y1, x2, y2, c);
        if let Some(hi) = self.hi_mut() {
            hi.line(x1, y1, x2, y2, c);
        }
    }

    fn clip(&self, x1: i64, y1: i64, x2: i64, y2: i64) -> (i64, i64, i64, i64) {
        let (w, h) = (self.img.width as i64, self.img.height as i64);
        (x1.min(x2).clamp(0, w), y1.min(y2).clamp(0, h), x1.max(x2).clamp(0, w), y1.max(y2).clamp(0, h))
    }

    pub fn fill_rect(&mut self, x1: i64, y1: i64, x2: i64, y2: i64, c: u32) {
        self.svg = None;
        let (l, t, r, b) = self.clip(x1, y1, x2, y2);
        for y in t..b {
            for x in l..r {
                self.img.pixels[y as usize * self.img.width + x as usize] = c;
            }
        }
        if let Some(hi) = self.hi_mut() {
            let s = hi.scale as i64;
            hi.fill(l * s, t * s, r * s, b * s, c);
        }
    }

    pub fn rectangle(&mut self, x1: i64, y1: i64, x2: i64, y2: i64, c: u32) {
        let (l, t, r, b) = (x1.min(x2), y1.min(y2), x1.max(x2) - 1, y1.max(y2) - 1);
        if r < l || b < t {
            return;
        }
        self.line(l, t, r, t, c);
        self.line(l, b, r, b, c);
        self.line(l, t, l, b, c);
        self.line(r, t, r, b, c);
    }

    /// The ellipse inside (x1,y1)-(x2,y2), outlined or filled.
    pub fn ellipse(&mut self, x1: i64, y1: i64, x2: i64, y2: i64, c: u32, fill: bool) {
        let mut spans = Vec::new();
        ellipse_spans(x1, y1, x2, y2, fill, |xa, xb, y| spans.push((xa, xb, y)));
        for (xa, xb, y) in spans {
            self.lo_line(xa, y, xb, y, c);
        }
        if let Some(hi) = self.hi_mut() {
            hi.ellipse(x1, y1, x2, y2, c, fill);
        }
    }

    /// Fills the region around (x, y) up to pixels of `border` color. What
    /// the screen shows gets the same region: each filled pixel's device
    /// pixels that aren't the border's color.
    pub fn flood_fill(&mut self, x: i64, y: i64, c: u32, border: u32) {
        self.svg = None;
        let mut stack = vec![(x, y)];
        let mut seen = vec![false; self.img.pixels.len()];
        let mut filled = Vec::new();
        while let Some((x, y)) = stack.pop() {
            let Some(p) = self.pixel(x, y) else { continue };
            let i = y as usize * self.img.width + x as usize;
            if p == border || seen[i] {
                continue;
            }
            seen[i] = true;
            self.img.pixels[i] = c;
            filled.push((x, y));
            stack.extend([(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)]);
        }
        if let Some(hi) = self.hi_mut() {
            let s = hi.scale as i64;
            for (x, y) in filled {
                for py in y * s..y * s + s {
                    for px in x * s..x * s + s {
                        if hi.pixel(px, py).is_some_and(|p| p != border) {
                            hi.put(px, py, c);
                        }
                    }
                }
            }
        }
    }

    /// Draws `src` with its top-left corner at (x, y), skipping its
    /// transparent color if it has one.
    pub fn draw(&mut self, x: i64, y: i64, src: &Bitmap) {
        let alpha = src.alpha_channel();
        for sy in 0..src.img.height {
            for sx in 0..src.img.width {
                let i = sy * src.img.width + sx;
                let c = src.img.pixels[i];
                if src.transparent && c == src.transparent_color {
                    continue;
                }
                let (dx, dy) = (x + sx as i64, y + sy as i64);
                match alpha.map(|a| a[i]) {
                    Some(0) => {}
                    Some(a) if a < 255 => {
                        if let Some(under) = self.pixel(dx, dy) {
                            self.lo_pset(dx, dy, blend(under, c, a));
                        }
                    }
                    _ => self.lo_pset(dx, dy, c),
                }
            }
        }
        let key = src.transparent.then_some(src.transparent_color);
        let Some(hi) = self.hi_mut() else { return };
        let s = hi.scale;
        // `src` as the screen shows it, at this scale.
        let shown = match src.hi.as_deref().filter(|h| h.scale == s && h.img.width == src.img.width * s && h.img.height == src.img.height * s) {
            Some(h) => std::borrow::Cow::Borrowed(h),
            None => {
                let (w, h) = (src.img.width * s, src.img.height * s);
                let from_svg = src.svg.as_ref().and_then(|svg| decode_svg(svg, s as f32).ok()).map(|(fine, a)| fit_hi(fine, a, w, h, s));
                match from_svg.or_else(|| HiRes::upscaled(&src.img, src.alpha_channel(), s)) {
                    Some(h) => std::borrow::Cow::Owned(h),
                    None => return,
                }
            }
        };
        let soft = shown.alpha.as_deref().filter(|a| a.len() == shown.img.pixels.len());
        let (ox, oy) = (x * s as i64, y * s as i64);
        for sy in 0..shown.img.height {
            for sx in 0..shown.img.width {
                let i = sy * shown.img.width + sx;
                let c = shown.img.pixels[i];
                if key == Some(c) {
                    continue;
                }
                let (dx, dy) = (ox + sx as i64, oy + sy as i64);
                match soft.map(|a| a[i]) {
                    Some(0) => {}
                    Some(a) if a < 255 => {
                        if let Some(under) = hi.pixel(dx, dy) {
                            hi.put(dx, dy, blend(under, c, a));
                        }
                    }
                    _ => hi.put(dx, dy, c),
                }
            }
        }
    }

    /// The opacity of each pixel, when it has one that fits.
    pub fn alpha_channel(&self) -> Option<&[u8]> {
        self.alpha.as_deref().filter(|a| a.len() == self.img.pixels.len())
    }

    /// Copies the (sx1,sy1)-(sx2,sy2) area of `src`, scaled, onto the
    /// (dx1,dy1)-(dx2,dy2) area of this bitmap.
    #[allow(clippy::too_many_arguments)]
    pub fn copy_rect(&mut self, (dx1, dy1, dx2, dy2): (i64, i64, i64, i64), src: &Bitmap, (sx1, sy1, sx2, sy2): (i64, i64, i64, i64)) {
        let (dw, dh, sw, sh) = (dx2 - dx1, dy2 - dy1, sx2 - sx1, sy2 - sy1);
        if dw <= 0 || dh <= 0 || sw <= 0 || sh <= 0 || dw * dh > MAX_PIXELS as i64 {
            return;
        }
        for y in 0..dh {
            for x in 0..dw {
                if let Some(c) = src.pixel(sx1 + x * sw / dw, sy1 + y * sh / dh) {
                    self.lo_pset(dx1 + x, dy1 + y, c);
                }
            }
        }
        let Some(hi) = self.hi_mut() else { return };
        let s = hi.scale as i64;
        // (from what `src` shows at this scale, when it has it)
        let fine = src.hi.as_deref().filter(|h| h.scale as i64 == s && h.img.width == src.img.width * s as usize && h.img.height == src.img.height * s as usize);
        let (pw, ph) = (dw * s, dh * s);
        for y in 0..ph {
            for x in 0..pw {
                // The source device pixel under this one.
                let (fx, fy) = (sx1 * s + x * sw / dw, sy1 * s + y * sh / dh);
                let c = match fine {
                    Some(f) => f.pixel(fx, fy),
                    None => src.pixel(fx.div_euclid(s), fy.div_euclid(s)),
                };
                if let Some(c) = c {
                    hi.put(dx1 * s + x, dy1 * s + y, c);
                }
            }
        }
    }

    pub fn get(&self, prop: &str) -> Option<Value> {
        if self.surface() && matches!(prop, "width" | "height") {
            return None;
        }
        if self.form {
            return None;
        }
        if let Some(p) = prop.strip_prefix("font.") {
            return self.font.get(p);
        }
        if self.canvas {
            match prop {
                "color" => return Some(v_int(self.background as i64)),
                "pencolor" => return Some(v_int(self.pen as i64)),
                "brushcolor" => return Some(v_int(self.brush as i64)),
                "fontcolor" => return Some(v_int(self.font.color)),
                "fontname" => return Some(v_str(&self.font.name)),
                // RapidR's FontSize is in pixels.
                "fontsize" => return Some(v_int(self.font.size.abs())),
                _ => {}
            }
        }
        Some(match prop {
            "width" => v_int(self.img.width as i64),
            "height" => v_int(self.img.height as i64),
            "empty" => v_int(if self.img.pixels.is_empty() { -1 } else { 0 }),
            "bmp" => v_str(&self.data_url()),
            "transparent" => v_int(if self.transparent { -1 } else { 0 }),
            "transparentcolor" => v_int(self.transparent_color as i64),
            _ => return None,
        })
    }

    /// Sets a property; `Err` for a BMP that can't be loaded.
    pub fn set(&mut self, prop: &str, val: &Value) -> Option<Result<(), String>> {
        if self.surface() && matches!(prop, "width" | "height" | "transparentcolor") {
            return None;
        }
        // A form's properties are the form's (the runtime keeps them); only
        // its color matters here: the surface shows it through.
        if self.form {
            if prop == "color" {
                let color = val.to_i64() as u32 & 0xFFFFFF;
                let old = std::mem::replace(&mut self.background, color);
                self.recolor(old, color);
                self.transparent_color = color;
            }
            return None;
        }
        if let Some(p) = prop.strip_prefix("font.") {
            return self.font.set(p, val).then_some(Ok(()));
        }
        if self.canvas {
            let color = val.to_i64() as u32 & 0xFFFFFF;
            match prop {
                // The background changes under what's drawn: only pixels
                // still showing it change.
                "color" => {
                    let old = std::mem::replace(&mut self.background, color);
                    self.recolor(old, color);
                    return None;
                }
                "pencolor" => self.pen = color,
                "brushcolor" => self.brush = color,
                "fontcolor" => self.font.color = color as i64,
                "fontname" => self.font.name = val.to_string_val(),
                "fontsize" => self.font.size = -val.to_i64().clamp(1, 1000),
                _ => return None,
            }
            return Some(Ok(()));
        }
        match prop {
            // A QIMAGE's transparent color is its bottom-left pixel's
            // (TImage's automatic mode).
            "transparent" if self.picture => {
                self.transparent = val.to_bool();
                self.auto_transparent_color();
            }
            "width" => self.resize(val.to_i64(), self.img.height as i64),
            "height" => self.resize(self.img.width as i64, val.to_i64()),
            "transparent" => self.transparent = val.to_bool(),
            "transparentcolor" => self.transparent_color = val.to_i64() as u32 & 0xFFFFFF,
            _ => return None,
        }
        Some(Ok(()))
    }

    /// Drawing methods (the ones that need nothing but this bitmap).
    pub fn call(&mut self, method: &str, args: &[Value]) -> Option<Value> {
        if self.canvas {
            // The runtime's: showing, hiding, painting again.
            if matches!(method, "update" | "refresh" | "repaint" | "show" | "hide") || (method == "paint" && args.len() < 3) {
                return None;
            }
            if let Some(v) = self.canvas_call(method, args) {
                return Some(v);
            }
        }
        let n = |i: usize| args.get(i).map_or(0, Value::to_i64);
        let c = |i: usize| n(i) as u32 & 0xFFFFFF;
        match method {
            "pset" => self.pset(n(0), n(1), c(2)),
            // `Bitmap.Pixel(x, y)` reads; with a third argument it writes.
            "pixel" if args.len() >= 3 => self.pset(n(0), n(1), c(2)),
            "pixel" => return Some(v_int(self.pixel(n(0), n(1)).map_or(-1, i64::from))),
            "line" => self.line(n(0), n(1), n(2), n(3), c(4)),
            "rectangle" => self.rectangle(n(0), n(1), n(2), n(3), c(4)),
            "fillrect" => self.fill_rect(n(0), n(1), n(2), n(3), c(4)),
            // `Circle(x1, y1, x2, y2, c, fill)`: outlined in c, filled with
            // the color fill (Delphi's pen and brush).
            "circle" => {
                if args.len() > 5 {
                    self.ellipse(n(0), n(1), n(2), n(3), c(5), true);
                }
                self.ellipse(n(0), n(1), n(2), n(3), c(4), false);
            }
            "roundrect" => self.round_rect(n(0), n(1), n(2), n(3), n(4), n(5), c(6)),
            "paint" => self.flood_fill(n(0), n(1), c(2), c(3)),
            // TextOut(x, y, text, colour, background (-1: transparent)),
            // in the bitmap's Font (objects/text.rs).
            "textout" => {
                let text = args.get(2).map(|v| v.to_string_val()).unwrap_or_default();
                let color = if args.len() > 3 { c(3) } else { self.font.color as u32 & 0xFFFFFF };
                let bg = args.get(4).map(Value::to_i64).filter(|v| *v >= 0).map(|v| v as u32 & 0xFFFFFF);
                let font = self.font.clone();
                super::text::text_out(self, n(0), n(1), &text, &font, color, bg);
            }
            "textwidth" => return Some(v_int(super::text::text_size(&args.first().map(|v| v.to_string_val()).unwrap_or_default(), &self.font).0)),
            "textheight" => return Some(v_int(super::text::text_size(&args.first().map(|v| v.to_string_val()).unwrap_or_default(), &self.font).1)),
            _ => return None,
        }
        Some(Value::Null)
    }

    /// The canvas methods that differ from a bitmap's: default colors from
    /// the pen and brush, and RapidR's own (`DrawText`, `Cls`, `Circle(cx,
    /// cy, r)`, `FillCircle`, `Ellipse`, `SetFont`, `SetPixel`).
    fn canvas_call(&mut self, method: &str, args: &[Value]) -> Option<Value> {
        let n = |i: usize| args.get(i).map_or(0, Value::to_i64);
        let color = |i: usize, default: u32| if args.len() > i { n(i) as u32 & 0xFFFFFF } else { default };
        let (pen, brush) = (self.pen, self.brush);
        match method {
            "line" => self.line(n(0), n(1), n(2), n(3), color(4, pen)),
            "rectangle" | "rect" => self.rectangle(n(0), n(1), n(2), n(3), color(4, pen)),
            "fillrect" => self.fill_rect(n(0), n(1), n(2), n(3), color(4, brush)),
            "pset" | "setpixel" => self.pset(n(0), n(1), color(2, pen)),
            "pixel" if args.len() >= 3 => self.pset(n(0), n(1), color(2, pen)),
            // RapidQ: Circle(x1, y1, x2, y2, color, fill); RapidR: Circle(cx, cy, r [, color]).
            "circle" if args.len() <= 4 => {
                let r = n(2).abs();
                self.ellipse(n(0) - r, n(1) - r, n(0) + r + 1, n(1) + r + 1, color(3, pen), false);
            }
            "circle" => {
                if args.len() > 5 {
                    self.ellipse(n(0), n(1), n(2), n(3), n(5) as u32 & 0xFFFFFF, true);
                }
                self.ellipse(n(0), n(1), n(2), n(3), color(4, pen), false);
            }
            "fillcircle" => {
                let r = n(2).abs();
                self.ellipse(n(0) - r, n(1) - r, n(0) + r + 1, n(1) + r + 1, color(3, brush), true);
            }
            // Ellipse(x1, y1, x2, y2 [, color [, fill]]).
            "ellipse" => {
                if args.len() > 5 {
                    self.ellipse(n(0), n(1), n(2), n(3), n(5) as u32 & 0xFFFFFF, true);
                }
                self.ellipse(n(0), n(1), n(2), n(3), color(4, pen), false);
            }
            "clear" | "cls" => {
                self.svg = None;
                let bg = self.background;
                self.img.pixels.iter_mut().for_each(|p| *p = bg);
                if let Some(hi) = self.hi_mut() {
                    hi.img.pixels.iter_mut().for_each(|p| *p = bg);
                }
            }
            // DrawText(text, x, y [, color [, size]]) or (x, y, text …);
            // the size in pixels.
            "drawtext" => {
                let text_first = !matches!(args.first(), Some(Value::Integer(_) | Value::Double(_)));
                let (text, x, y) = if text_first {
                    (args.first().map(Value::to_string_val), n(1), n(2))
                } else {
                    (args.get(2).map(Value::to_string_val), n(0), n(1))
                };
                let mut font = self.font.clone();
                if let Some(size) = args.get(4).map(Value::to_i64).filter(|s| *s > 0) {
                    font.size = -size.min(1000);
                }
                let fg = color(3, font.color as u32 & 0xFFFFFF);
                super::text::text_out(self, x, y, &text.unwrap_or_default(), &font, fg, None);
            }
            "setfont" => {
                self.font.name = args.first().map(Value::to_string_val).filter(|s| !s.trim().is_empty()).unwrap_or_else(|| "Arial".into());
                self.font.size = -args.get(1).map_or(12, Value::to_i64).clamp(1, 1000);
            }
            _ => return None,
        }
        Some(Value::Null)
    }

    /// Pixels of color `old` (the background showing) become `new`.
    fn recolor(&mut self, old: u32, new: u32) {
        self.svg = None;
        self.img.pixels.iter_mut().filter(|p| **p == old).for_each(|p| *p = new);
        if let Some(hi) = self.hi.as_mut() {
            hi.img.pixels.iter_mut().filter(|p| **p == old).for_each(|p| *p = new);
        }
    }

    /// A filled rectangle with corners rounded by an ellipse of w × h.
    #[allow(clippy::too_many_arguments)]
    fn round_rect(&mut self, x1: i64, y1: i64, x2: i64, y2: i64, w: i64, h: i64, c: u32) {
        let (l, t, r, b) = (x1.min(x2), y1.min(y2), x1.max(x2), y1.max(y2));
        let (w, h) = (w.clamp(0, r - l), h.clamp(0, b - t));
        self.fill_rect(l + w / 2, t, r - w / 2, b, c);
        self.fill_rect(l, t + h / 2, r, b - h / 2, c);
        for (cx, cy) in [(l, t), (r - w, t), (l, b - h), (r - w, b - h)] {
            self.ellipse(cx, cy, cx + w, cy + h, c, true);
        }
    }

    /// The `.BMP` value: a `data:` URL of the image, with the transparent
    /// color (if any) as a `#transparent=` fragment so drawing it elsewhere
    /// keeps it (browsers ignore the fragment when showing the image).
    pub fn data_url(&self) -> String {
        let url = match (self.svg.as_deref(), self.alpha_channel()) {
            (Some(svg), _) => format!("{SVG_DATA_URL}{}", base64_encode(svg)),
            (None, a) => match a {
                Some(a) => format!("{BMP_DATA_URL}{}", base64_encode(&encode_bmp_alpha(&self.img, a))),
                None => bmp_data_url(&self.img),
            },
        };
        if self.transparent {
            format!("{url}#transparent={}", self.transparent_color)
        } else {
            url
        }
    }

    /// Pixels as RGBA bytes for drawing on screen; the transparent color (if
    /// the bitmap has one) gets alpha 0.
    pub fn to_rgba(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.img.pixels.len() * 4);
        let soft = self.alpha_channel();
        for (i, &c) in self.img.pixels.iter().enumerate() {
            let alpha = if self.transparent && c == self.transparent_color { 0 } else { soft.map_or(255, |a| a[i]) };
            out.extend_from_slice(&[c as u8, (c >> 8) as u8, (c >> 16) as u8, alpha]);
        }
        out
    }

    /// Loads a BMP — or an SVG, drawn at its size (with its soft edges).
    pub fn load_bmp_bytes(&mut self, bytes: &[u8]) -> Result<(), String> {
        self.hi = None;
        self.svg = None;
        if is_svg(bytes) {
            let (img, alpha) = decode_svg(bytes, 1.0)?;
            // What the screen shows: the SVG drawn at the display scale.
            let s = display_scale();
            if s > 1 {
                if let Ok((fine, fine_alpha)) = decode_svg(bytes, s as f32) {
                    self.hi = Some(Box::new(fit_hi(fine, fine_alpha, img.width * s, img.height * s, s)));
                }
            }
            self.img = img;
            self.alpha = Some(alpha);
            self.svg = Some(std::rc::Rc::new(bytes.to_vec()));
            return Ok(());
        }
        let (img, alpha) = super::codec::decode_raster(bytes)?;
        self.img = img;
        self.alpha = alpha;
        self.auto_transparent_color();
        Ok(())
    }

    /// A QIMAGE's transparent color: its picture's bottom-left pixel.
    pub fn auto_transparent_color(&mut self) {
        if self.picture && self.img.height > 0 {
            if let Some(c) = self.pixel(0, self.img.height as i64 - 1) {
                self.transparent_color = c;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bmp(w: i64, h: i64) -> Bitmap {
        let mut b = Bitmap::default();
        b.resize(w, h);
        b
    }

    #[test]
    fn drawing() {
        let mut b = bmp(10, 10);
        assert_eq!(b.pixel(0, 0), Some(0xFFFFFF));
        b.fill_rect(0, 0, 5, 5, 0xFF);
        assert_eq!((b.pixel(4, 4), b.pixel(5, 5)), (Some(0xFF), Some(0xFFFFFF)));
        b.line(0, 9, 9, 9, 0);
        assert!((0..10).all(|x| b.pixel(x, 9) == Some(0)));
        b.rectangle(6, 0, 10, 4, 0x00FF00);
        assert_eq!((b.pixel(6, 0), b.pixel(9, 3), b.pixel(7, 1)), (Some(0xFF00), Some(0xFF00), Some(0xFFFFFF)));
        b.flood_fill(7, 1, 0xAA, 0xFF00);
        assert_eq!((b.pixel(8, 2), b.pixel(6, 5)), (Some(0xAA), Some(0xFFFFFF)));
        assert_eq!(b.pixel(-1, 0), None);
        b.resize(12, 3);
        assert_eq!((b.pixel(0, 0), b.pixel(11, 2)), (Some(0xFF), Some(0xFFFFFF)));
    }

    /// Every kind of drawing, on a bitmap at the current display scale.
    fn scene() -> Bitmap {
        let mut b = bmp(40, 30);
        b.fill_rect(2, 2, 12, 8, 0xFF);
        b.line(0, 29, 39, 0, 0x00FF00);
        b.rectangle(20, 2, 30, 10, 0);
        b.ellipse(5, 12, 25, 28, 0xFF0000, false);
        b.ellipse(28, 14, 38, 24, 0x00FFFF, true);
        b.pset(39, 29, 0x123456);
        b.flood_fill(24, 5, 0xAA00AA, 0);
        let mut font = Font::default();
        font.size = 10;
        super::super::text::text_out(&mut b, 3, 14, "Ag", &font, 0x0000FF, Some(0xEEEEEE));
        let mut sprite = bmp(3, 2);
        sprite.fill_rect(0, 0, 3, 2, 0x404040);
        sprite.pset(1, 0, 0xFFFFFF);
        sprite.transparent = true;
        b.draw(33, 3, &sprite);
        b.copy_rect((0, 26, 8, 30), &sprite, (0, 0, 3, 2));
        b
    }

    #[test]
    fn high_dpi_keeps_the_pixels_programs_read() {
        set_display_scale(1.0);
        let one = scene();
        set_display_scale(2.0);
        let mut two = scene();
        assert_eq!(one.img, two.img, "the pixels a program reads are the same at 2×");
        assert!(one.hi.is_none());
        let (w, h, rgba, s) = two.display_rgba();
        assert_eq!((w, h, s, rgba.len()), (80, 60, 2, 80 * 60 * 4));
        set_display_scale(1.0);
    }

    #[test]
    fn high_dpi_shows_the_same_picture_finer() {
        set_display_scale(2.0);
        let b = scene();
        let hi = b.hi.as_deref().expect("drawn at 2×");
        // Where edges are straight, each pixel's 2 × 2 block is its color.
        for (x, y) in [(3, 3), (11, 7), (20, 5), (29, 9), (24, 5), (33, 3), (35, 4), (39, 29), (1, 27), (0, 0)] {
            let c = b.pixel(x, y).unwrap();
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                assert_eq!(hi.pixel(x * 2 + dx, y * 2 + dy), Some(c), "({x}, {y}) +({dx}, {dy})");
            }
        }
        // The sprite's transparent pixel shows what's under it, as in the pixels.
        assert_eq!(b.pixel(34, 3), hi.pixel(68, 6));
        // Text: glyphs drawn from device pixels (not 2 × 2 blocks).
        let blocky = (28..=60).step_by(2).all(|y| (6..40).step_by(2).all(|x| hi.pixel(x, y) == hi.pixel(x + 1, y) && hi.pixel(x, y) == hi.pixel(x, y + 1)));
        assert!(!blocky, "the text is finer than 2 × 2 blocks");
        set_display_scale(1.0);
    }

    #[test]
    fn high_dpi_svg_and_redraws() {
        set_display_scale(2.0);
        let mut icon = Bitmap::default();
        icon.load_bmp_bytes(br##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><circle cx="5" cy="5" r="4" fill="#ff0000"/></svg>"##).unwrap();
        let hi = icon.hi.as_deref().expect("the SVG drawn at 2×");
        assert_eq!((hi.img.width, hi.img.height, icon.img.width), (20, 20, 10));
        // Loaded before the screen's scale was known: drawn again at it.
        set_display_scale(1.0);
        let mut early = Bitmap::default();
        early.load_bmp_bytes(br##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><circle cx="5" cy="5" r="4" fill="#ff0000"/></svg>"##).unwrap();
        assert!(early.hi.is_none());
        set_display_scale(2.0);
        let (w, _, rgba, _) = early.display_rgba();
        assert_eq!(w, 20);
        let (_, _, enlarged, _) = { let mut b = early.clone(); b.svg = None; b.hi = None; b.display_rgba() };
        assert_ne!(rgba, enlarged, "drawn again, not enlarged");
        // Drawn onto a canvas, it stays fine; loading a BMP drops it.
        let mut c = bmp(20, 20);
        c.draw(2, 2, &icon);
        assert_eq!(c.hi.as_deref().unwrap().pixel(14, 14), Some(0x0000FF));
        let mut plain = bmp(2, 2);
        plain.fill_rect(0, 0, 2, 2, 0);
        let bytes = super::super::codec::encode_bmp(&plain.img);
        icon.load_bmp_bytes(&bytes).unwrap();
        assert!(icon.hi.is_none());
        // Resizing keeps what's shown, at its new size.
        c.resize(25, 5);
        let hi = c.hi.as_deref().unwrap();
        assert_eq!((hi.img.width, hi.img.height), (50, 10));
        set_display_scale(1.0);
    }

    #[test]
    fn filled_circle_and_draw() {
        let mut b = bmp(11, 11);
        b.ellipse(0, 0, 11, 11, 0, true);
        assert_eq!((b.pixel(5, 5), b.pixel(0, 0), b.pixel(5, 0), b.pixel(0, 5)), (Some(0), Some(0xFFFFFF), Some(0), Some(0)));
        let mut sprite = bmp(2, 1);
        sprite.pset(0, 0, 0x123456);
        sprite.transparent = true; // white is transparent
        let mut dest = bmp(4, 4);
        dest.fill_rect(0, 0, 4, 4, 0);
        dest.draw(1, 1, &sprite);
        assert_eq!((dest.pixel(1, 1), dest.pixel(2, 1)), (Some(0x123456), Some(0)));
    }
}

//! QBITMAP (manual, Appendix A): an off-screen image to draw on, load and
//! save as BMP. Colors are RapidQ's &HBBGGRR integers. Rectangles follow
//! Windows: the right and bottom edges are excluded.

use super::codec::{bmp_data_url, decode_bmp, Pixels, MAX_PIXELS};
use crate::{v_int, v_str, Value};

#[derive(Debug, Clone)]
pub struct Bitmap {
    pub img: Pixels,
    pub transparent: bool,
    pub transparent_color: u32,
}

/// Color of a new bitmap's pixels.
const BACKGROUND: u32 = 0xFFFFFF;

impl Default for Bitmap {
    fn default() -> Self {
        Self { img: Pixels { width: 0, height: 0, pixels: Vec::new() }, transparent: false, transparent_color: BACKGROUND }
    }
}

impl Bitmap {
    pub fn from_pixels(img: Pixels) -> Self {
        Self { img, ..Self::default() }
    }

    /// Resizes, keeping the pixels that still fit.
    pub fn resize(&mut self, width: i64, height: i64) {
        let (w, h) = (width.clamp(0, 32767) as usize, height.clamp(0, 32767) as usize);
        if w * h > MAX_PIXELS {
            return;
        }
        let mut pixels = vec![BACKGROUND; w * h];
        for y in 0..h.min(self.img.height) {
            for x in 0..w.min(self.img.width) {
                pixels[y * w + x] = self.img.pixels[y * self.img.width + x];
            }
        }
        self.img = Pixels { width: w, height: h, pixels };
    }

    pub fn pixel(&self, x: i64, y: i64) -> Option<u32> {
        if x < 0 || y < 0 || x as usize >= self.img.width || y as usize >= self.img.height {
            return None;
        }
        Some(self.img.pixels[y as usize * self.img.width + x as usize])
    }

    pub fn pset(&mut self, x: i64, y: i64, c: u32) {
        if x >= 0 && y >= 0 && (x as usize) < self.img.width && (y as usize) < self.img.height {
            self.img.pixels[y as usize * self.img.width + x as usize] = c;
        }
    }

    pub fn line(&mut self, x1: i64, y1: i64, x2: i64, y2: i64, c: u32) {
        let (dx, dy) = ((x2 - x1).abs(), -(y2 - y1).abs());
        let (sx, sy) = (if x1 < x2 { 1 } else { -1 }, if y1 < y2 { 1 } else { -1 });
        let (mut x, mut y, mut err) = (x1, y1, dx + dy);
        // Bounded so a huge coordinate can't loop for ages off-bitmap.
        for _ in 0..=(dx - dy).min(1 << 20) {
            self.pset(x, y, c);
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

    fn clip(&self, x1: i64, y1: i64, x2: i64, y2: i64) -> (i64, i64, i64, i64) {
        let (w, h) = (self.img.width as i64, self.img.height as i64);
        (x1.min(x2).clamp(0, w), y1.min(y2).clamp(0, h), x1.max(x2).clamp(0, w), y1.max(y2).clamp(0, h))
    }

    pub fn fill_rect(&mut self, x1: i64, y1: i64, x2: i64, y2: i64, c: u32) {
        let (l, t, r, b) = self.clip(x1, y1, x2, y2);
        for y in t..b {
            for x in l..r {
                self.img.pixels[y as usize * self.img.width + x as usize] = c;
            }
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
                self.line(xa, y, xb, y, c);
            } else {
                // Each side spans to where the previous row's was, so the
                // flat top and bottom and the steep sides have no gaps.
                let (pa, pb) = prev.unwrap_or((xa, xb));
                self.line(xa.min(pa), y, xa.max(pa), y, c);
                self.line(xb.min(pb), y, xb.max(pb), y, c);
            }
            prev = Some((xa, xb));
        }
    }

    /// Fills the region around (x, y) up to pixels of `border` color.
    pub fn flood_fill(&mut self, x: i64, y: i64, c: u32, border: u32) {
        let mut stack = vec![(x, y)];
        let mut seen = vec![false; self.img.pixels.len()];
        while let Some((x, y)) = stack.pop() {
            let Some(p) = self.pixel(x, y) else { continue };
            let i = y as usize * self.img.width + x as usize;
            if p == border || seen[i] {
                continue;
            }
            seen[i] = true;
            self.img.pixels[i] = c;
            stack.extend([(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)]);
        }
    }

    /// Draws `src` with its top-left corner at (x, y), skipping its
    /// transparent color if it has one.
    pub fn draw(&mut self, x: i64, y: i64, src: &Bitmap) {
        for sy in 0..src.img.height {
            for sx in 0..src.img.width {
                let c = src.img.pixels[sy * src.img.width + sx];
                if !(src.transparent && c == src.transparent_color) {
                    self.pset(x + sx as i64, y + sy as i64, c);
                }
            }
        }
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
                    self.pset(dx1 + x, dy1 + y, c);
                }
            }
        }
    }

    pub fn get(&self, prop: &str) -> Option<Value> {
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
        match prop {
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
            "circle" => self.ellipse(n(0), n(1), n(2), n(3), c(4), args.len() > 5 && n(5) != 0),
            "roundrect" => self.round_rect(n(0), n(1), n(2), n(3), n(4), n(5), c(6)),
            "paint" => self.flood_fill(n(0), n(1), c(2), c(3)),
            _ => return None,
        }
        Some(Value::Null)
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
        let url = bmp_data_url(&self.img);
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
        for &c in &self.img.pixels {
            let alpha = if self.transparent && c == self.transparent_color { 0 } else { 255 };
            out.extend_from_slice(&[c as u8, (c >> 8) as u8, (c >> 16) as u8, alpha]);
        }
        out
    }

    pub fn load_bmp_bytes(&mut self, bytes: &[u8]) -> Result<(), String> {
        self.img = decode_bmp(bytes)?;
        Ok(())
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

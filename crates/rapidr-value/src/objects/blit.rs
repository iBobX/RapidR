//! Pixels moved in whole rows: the engine under `Draw`, `CopyRect` and
//! `StretchDraw` (scaled copies) and under `SetPixels` / `SetPixels8` (a
//! block of colours put into a bitmap). A program that draws a whole image
//! from data (an emulator's video memory, an image filter, a heat map, a
//! cellular automaton) pays for these once per frame, so they work a row
//! at a time on the pixel slices: no per-pixel call, no per-pixel checks.
//!
//! The functions know nothing of bitmaps: a *plane* is `w` × `h` colours
//! (`&BBGGRR` integers, as [`super::bitmap::Bitmap::img`] keeps them), row
//! by row; a bitmap that is shown at a high-DPI scale has a second, finer
//! plane, and the same function draws into both.

/// The source column or row each of `n` destination positions takes —
/// `start + i * length / n`, the integer arithmetic every scaled copy here
/// has always used (a position outside the source plane, `limit`, takes
/// none: that destination pixel is left as it was).
fn nearest_index(i: i64, start: i64, length: i64, n: i64, limit: usize) -> Option<usize> {
    let at = start + i * length / n;
    (at >= 0 && at < limit as i64).then_some(at as usize)
}

/// One bilinear sample position: the two neighbours and how much of the
/// second (0 to 256).
#[derive(Clone, Copy)]
struct Tap {
    a: usize,
    b: usize,
    w: u32,
}

/// The two source pixels around the centre of destination position `i` —
/// the source rectangle `start`, `length` mapped onto `n` destination
/// positions, edge pixels repeated at the rectangle's edges (so a copied
/// part never blends in what lies next to it).
fn tap(i: i64, start: i64, length: i64, n: i64, limit: usize) -> Option<Tap> {
    nearest_index(i, start, length, n, limit)?;
    // The centre in source pixels (in 1/256 ths): ((i + ½) · length / n) − ½.
    let c256 = ((2 * i + 1) * length * 256) / (2 * n) - 128;
    let lo = start.max(0);
    let hi = (start + length - 1).min(limit as i64 - 1);
    let c256 = (start * 256 + c256).clamp(lo * 256, hi * 256);
    let a = (c256 >> 8) as usize;
    let b = (a + 1).min(hi as usize);
    Some(Tap { a, b, w: (c256 & 255) as u32 })
}

/// `a` towards `b` by `w` 256ths, every colour channel at once.
#[inline]
fn lerp(a: u32, b: u32, w: u32) -> u32 {
    if w == 0 || a == b {
        return a;
    }
    let rb = (((a & 0x00FF_00FF) * (256 - w) + (b & 0x00FF_00FF) * w) >> 8) & 0x00FF_00FF;
    let g = (((a & 0x0000_FF00) * (256 - w) + (b & 0x0000_FF00) * w) >> 8) & 0x0000_FF00;
    rb | g
}

/// A plane to draw into: its colours, its pixels' opacity (set to 255
/// where drawn: what is drawn is opaque), its size.
pub struct Target<'a> {
    pub px: &'a mut [u32],
    pub alpha: Option<&'a mut [u8]>,
    pub w: usize,
    pub h: usize,
}

/// A plane to draw from.
#[derive(Clone, Copy)]
pub struct Source<'a> {
    pub px: &'a [u32],
    pub w: usize,
    pub h: usize,
}

/// A rectangle: left, top, width, height.
pub type Rect = (i64, i64, i64, i64);

/// Copies the `s` rectangle of `src` onto the `d` rectangle of `dst`,
/// scaled to fit: nearest pixel (every destination pixel takes the source
/// pixel at `start + i * length / n`, crisp pixel art), or with `smooth`
/// bilinear (the four around the centre, blended). The parts of `d` outside
/// `dst` are not drawn; a destination pixel whose source falls outside
/// `src` is left as it was.
pub fn scale(dst: &mut Target, d: Rect, src: Source, s: Rect, smooth: bool) {
    let (dx, dy, dw, dh) = d;
    let (sx, sy, sw, sh) = s;
    if dw <= 0 || dh <= 0 || sw <= 0 || sh <= 0 || src.px.len() < src.w * src.h || dst.px.len() < dst.w * dst.h {
        return;
    }
    // The part of the destination that shows.
    let (x0, x1) = ((-dx).max(0), dw.min(dst.w as i64 - dx));
    let (y0, y1) = ((-dy).max(0), dh.min(dst.h as i64 - dy));
    if x0 >= x1 || y0 >= y1 {
        return;
    }
    let n = (x1 - x0) as usize;
    let at = |y: i64| (dy + y) as usize * dst.w + (dx + x0) as usize;
    // One to one: whole rows are copied.
    if (sw, sh) == (dw, dh) && sx >= 0 && sy >= 0 && sx + x1 <= src.w as i64 && sy + y1 <= src.h as i64 {
        for y in y0..y1 {
            let from = (sy + y) as usize * src.w + (sx + x0) as usize;
            let to = at(y);
            dst.px[to..to + n].copy_from_slice(&src.px[from..from + n]);
            if let Some(a) = dst.alpha.as_deref_mut() {
                a[to..to + n].fill(255);
            }
        }
        return;
    }
    if smooth {
        let cols: Vec<Option<Tap>> = (x0..x1).map(|i| tap(i, sx, sw, dw, src.w)).collect();
        let whole = cols.iter().all(Option::is_some);
        // (the last row drawn, and what it sampled: a row that samples the
        // same pair with the same weight is that row again)
        let mut prev: Option<((usize, usize, u32), i64)> = None;
        for y in y0..y1 {
            let Some(t) = tap(y, sy, sh, dh, src.h) else { continue };
            let (ra, rb) = (t.a * src.w, t.b * src.w);
            let to = at(y);
            let key = (t.a, t.b, t.w);
            match prev {
                Some((k, prev_y)) if whole && k == key => dst.px.copy_within(at(prev_y)..at(prev_y) + n, to),
                _ => {
                    for (k, c) in cols.iter().enumerate() {
                        let Some(c) = c else { continue };
                        let top = lerp(src.px[ra + c.a], src.px[ra + c.b], c.w);
                        dst.px[to + k] = if t.w == 0 { top } else { lerp(top, lerp(src.px[rb + c.a], src.px[rb + c.b], c.w), t.w) };
                    }
                }
            }
            prev = Some((key, y));
            if let Some(a) = dst.alpha.as_deref_mut() {
                for (k, c) in cols.iter().enumerate() {
                    if c.is_some() {
                        a[to + k] = 255;
                    }
                }
            }
        }
        return;
    }
    // Nearest. The source column of each destination column, once.
    let cols: Vec<Option<usize>> = (x0..x1).map(|i| nearest_index(i, sx, sw, dw, src.w)).collect();
    let whole = cols.iter().all(Option::is_some);
    let cols_fast: Vec<usize> = if whole { cols.iter().map(|c| c.unwrap_or(0)).collect() } else { Vec::new() };
    let mut last: Option<(usize, i64)> = None;
    for y in y0..y1 {
        let Some(row) = nearest_index(y, sy, sh, dh, src.h) else { continue };
        let to = at(y);
        if whole {
            // A source row used again (an enlargement) is a row copied.
            match last {
                Some((r, prev_y)) if r == row => dst.px.copy_within(at(prev_y)..at(prev_y) + n, to),
                _ => {
                    let from = row * src.w;
                    let line = &src.px[from..from + src.w];
                    for (d, &c) in dst.px[to..to + n].iter_mut().zip(&cols_fast) {
                        *d = line[c];
                    }
                }
            }
            last = Some((row, y));
        } else {
            let from = row * src.w;
            for (k, c) in cols.iter().enumerate() {
                if let Some(c) = c {
                    dst.px[to + k] = src.px[from + c];
                }
            }
        }
        if let Some(a) = dst.alpha.as_deref_mut() {
            for (k, c) in cols.iter().enumerate() {
                if c.is_some() {
                    a[to + k] = 255;
                }
            }
        }
    }
}

/// Puts the `w` × `h` colours of `block` (row by row) into `dst` with its
/// top-left corner at (`x`, `y`) — which must lie inside — each pixel made
/// a `scale` × `scale` square of the plane (1: the plane of the pixels
/// themselves; 2 on a Retina screen: the finer plane of what is shown).
pub fn put_block(dst: &mut Target, x: usize, y: usize, w: usize, h: usize, block: &[u32], scale: usize) {
    if w == 0 || h == 0 || scale == 0 || block.len() < w * h {
        return;
    }
    let (x, y, bw) = (x * scale, y * scale, w * scale);
    if x + bw > dst.w || (y + h * scale) > dst.h {
        return;
    }
    let mut line = vec![0u32; if scale > 1 { bw } else { 0 }];
    for row in 0..h {
        let colours = &block[row * w..(row + 1) * w];
        let from: &[u32] = if scale == 1 {
            colours
        } else {
            for (cell, &c) in line.chunks_exact_mut(scale).zip(colours) {
                cell.fill(c);
            }
            &line
        };
        for k in 0..scale {
            let to = (y + row * scale + k) * dst.w + x;
            dst.px[to..to + bw].copy_from_slice(from);
            if let Some(a) = dst.alpha.as_deref_mut() {
                a[to..to + bw].fill(255);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plane(w: usize, h: usize, f: impl Fn(usize, usize) -> u32) -> Vec<u32> {
        (0..w * h).map(|i| f(i % w, i / w)).collect()
    }

    /// The scaled copy the bitmap had before it worked in rows: one pixel
    /// at a time. Kept here so the rows are held to its answers.
    #[allow(clippy::too_many_arguments)]
    fn old_scale(dst: &mut [u32], dw_plane: usize, dh_plane: usize, d: Rect, src: &[u32], sw_plane: usize, sh_plane: usize, s: Rect) {
        let ((dx, dy, dw, dh), (sx, sy, sw, sh)) = (d, s);
        for y in 0..dh {
            for x in 0..dw {
                let (fx, fy) = (sx + x * sw / dw, sy + y * sh / dh);
                if fx < 0 || fy < 0 || fx as usize >= sw_plane || fy as usize >= sh_plane {
                    continue;
                }
                let (tx, ty) = (dx + x, dy + y);
                if tx >= 0 && ty >= 0 && (tx as usize) < dw_plane && (ty as usize) < dh_plane {
                    dst[ty as usize * dw_plane + tx as usize] = src[fy as usize * sw_plane + fx as usize];
                }
            }
        }
    }

    #[test]
    fn nearest_scaling_is_the_old_per_pixel_answer() {
        let src = plane(9, 7, |x, y| (x * 31 + y * 17 + 1) as u32);
        // enlarged, shrunk, odd ratios, partly off the destination, a
        // source rectangle partly off the source, a mirrored-size one
        let cases: [(Rect, Rect); 9] = [
            ((0, 0, 18, 14), (0, 0, 9, 7)),
            ((2, 3, 5, 4), (0, 0, 9, 7)),
            ((-4, -3, 20, 15), (0, 0, 9, 7)),
            ((10, 8, 30, 30), (1, 1, 5, 4)),
            ((0, 0, 13, 11), (3, 2, 9, 9)),
            ((0, 0, 9, 7), (0, 0, 9, 7)),
            ((1, 1, 9, 7), (0, 0, 9, 7)),
            ((0, 0, 4, 3), (-2, -1, 9, 7)),
            ((0, 0, 27, 21), (2, 2, 3, 3)),
        ];
        for (d, s) in cases {
            let mut old = plane(24, 20, |x, y| (x + y * 100 + 7_000_000) as u32);
            let mut new = old.clone();
            old_scale(&mut old, 24, 20, d, &src, 9, 7, s);
            scale(&mut Target { px: &mut new, alpha: None, w: 24, h: 20 }, d, Source { px: &src, w: 9, h: 7 }, s, false);
            assert_eq!(new, old, "dest {d:?} from {s:?}");
        }
    }

    #[test]
    fn smooth_blends_and_nearest_stays_crisp() {
        // black | white, enlarged four times
        let src = vec![0u32, 0xFFFFFF];
        let mut crisp = vec![9u32; 8];
        scale(&mut Target { px: &mut crisp, alpha: None, w: 8, h: 1 }, (0, 0, 8, 1), Source { px: &src, w: 2, h: 1 }, (0, 0, 2, 1), false);
        assert_eq!(crisp, [0, 0, 0, 0, 0xFFFFFF, 0xFFFFFF, 0xFFFFFF, 0xFFFFFF]);
        let mut soft = vec![9u32; 8];
        scale(&mut Target { px: &mut soft, alpha: None, w: 8, h: 1 }, (0, 0, 8, 1), Source { px: &src, w: 2, h: 1 }, (0, 0, 2, 1), true);
        assert_eq!((soft[0], soft[7]), (0, 0xFFFFFF), "the ends are the source's");
        assert!(soft[3] > 0 && soft[3] < 0xFFFFFF && soft[4] >= soft[3], "the middle is a blend, rising: {soft:x?}");
        assert!(soft.windows(2).all(|w| w[0] <= w[1]), "a ramp: {soft:x?}");
        // each channel blends on its own (red and blue do not bleed)
        let mut mix = vec![0u32; 3];
        scale(&mut Target { px: &mut mix, alpha: None, w: 3, h: 1 }, (0, 0, 3, 1), Source { px: &[0x0000FF, 0xFF0000], w: 2, h: 1 }, (0, 0, 2, 1), true);
        assert_eq!(mix[1] & 0x00FF00, 0, "no green from red and blue");
    }

    #[test]
    fn scaled_pixels_are_opaque_and_one_to_one_copies_rows() {
        let src = plane(4, 4, |x, y| (x + y * 4) as u32);
        let mut px = vec![0u32; 36];
        let mut alpha = vec![0u8; 36];
        scale(&mut Target { px: &mut px, alpha: Some(&mut alpha), w: 6, h: 6 }, (1, 1, 4, 4), Source { px: &src, w: 4, h: 4 }, (0, 0, 4, 4), false);
        assert_eq!(px[7], 0);
        assert_eq!(px[4 * 6 + 4], 15);
        assert_eq!((alpha[0], alpha[7], alpha[28]), (0, 255, 255));
        assert_eq!(alpha.iter().filter(|a| **a == 255).count(), 16);
    }

    #[test]
    fn a_block_fills_its_squares() {
        let mut px = vec![0u32; 8 * 6];
        put_block(&mut Target { px: &mut px, alpha: None, w: 8, h: 6 }, 1, 1, 2, 2, &[1, 2, 3, 4], 2);
        let rows: Vec<&[u32]> = px.chunks(8).collect();
        assert_eq!(rows[0], [0; 8]);
        assert_eq!(rows[1], [0; 8], "nothing above the block");
        assert_eq!(rows[2], [0, 0, 1, 1, 2, 2, 0, 0]);
        assert_eq!(rows[2], rows[3]);
        assert_eq!(rows[4], [0, 0, 3, 3, 4, 4, 0, 0]);
        assert_eq!(rows[4], rows[5]);
    }
}

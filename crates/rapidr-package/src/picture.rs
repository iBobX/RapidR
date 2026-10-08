//! Pictures (tiny-skia's premultiplied RGBA pixmaps): PNG in and out, and
//! resizing for icons — area-averaged when smaller (every source pixel
//! counts, so a 1024 px picture makes a clean 16 px one), bilinear when
//! larger.

use resvg::tiny_skia::{Pixmap, PremultipliedColorU8};

/// The largest side RapidR reads an icon picture at (a corrupt header can't
/// ask for gigabytes).
pub const MAX_SIDE: u32 = 4096;

pub fn decode_png(bytes: &[u8]) -> Result<Pixmap, String> {
    let p = Pixmap::decode_png(bytes).map_err(|e| format!("not a PNG RapidR can read ({e})"))?;
    if p.width() > MAX_SIDE || p.height() > MAX_SIDE {
        return Err(format!("the picture is {} × {}: at most {MAX_SIDE} × {MAX_SIDE}", p.width(), p.height()));
    }
    Ok(p)
}

pub fn encode_png(p: &Pixmap) -> Vec<u8> {
    // (only fails for a zero-sized pixmap, which can't be made)
    p.encode_png().unwrap_or_default()
}

/// A new transparent picture.
pub fn blank(w: u32, h: u32) -> Pixmap {
    Pixmap::new(w.max(1), h.max(1)).expect("a picture's size")
}

/// Straight (not premultiplied) RGBA bytes, row by row.
pub fn to_rgba(p: &Pixmap) -> Vec<u8> {
    let mut out = Vec::with_capacity(p.pixels().len() * 4);
    for c in p.pixels() {
        let c = c.demultiply();
        out.extend_from_slice(&[c.red(), c.green(), c.blue(), c.alpha()]);
    }
    out
}

/// A picture from straight RGBA bytes.
pub fn from_rgba(w: u32, h: u32, rgba: &[u8]) -> Pixmap {
    let mut p = blank(w, h);
    for (dst, s) in p.pixels_mut().iter_mut().zip(rgba.chunks_exact(4)) {
        *dst = resvg::tiny_skia::ColorU8::from_rgba(s[0], s[1], s[2], s[3]).premultiply();
    }
    p
}

/// `p` centred on a transparent square (a square picture as it is).
pub fn squared(p: &Pixmap) -> Pixmap {
    let (w, h) = (p.width(), p.height());
    if w == h {
        return p.clone();
    }
    let side = w.max(h);
    let mut out = blank(side, side);
    out.draw_pixmap(
        ((side - w) / 2) as i32,
        ((side - h) / 2) as i32,
        p.as_ref(),
        &resvg::tiny_skia::PixmapPaint::default(),
        resvg::tiny_skia::Transform::identity(),
        None,
    );
    out
}

/// Each output pixel's source pixels and their weights along one axis.
fn weights(src: u32, dst: u32) -> Vec<Vec<(usize, f32)>> {
    let scale = f64::from(src) / f64::from(dst);
    (0..dst)
        .map(|d| {
            if src > dst {
                // area: the source span this pixel covers, partial pixels weighed
                let (a, b) = (f64::from(d) * scale, f64::from(d + 1) * scale);
                let mut taps = Vec::new();
                let mut s = a.floor() as usize;
                while (s as f64) < b && s < src as usize {
                    let cover = (b.min(s as f64 + 1.0) - a.max(s as f64)).max(0.0);
                    if cover > 0.0 {
                        taps.push((s, (cover / scale) as f32));
                    }
                    s += 1;
                }
                taps
            } else {
                // bilinear between the two nearest source pixels
                let x = ((f64::from(d) + 0.5) * scale - 0.5).max(0.0);
                let i = (x.floor() as usize).min(src as usize - 1);
                let j = (i + 1).min(src as usize - 1);
                let t = (x - i as f64) as f32;
                if i == j || t <= 0.0 {
                    vec![(i, 1.0)]
                } else {
                    vec![(i, 1.0 - t), (j, t)]
                }
            }
        })
        .collect()
}

/// `p` resized to `w` × `h`.
pub fn resize(p: &Pixmap, w: u32, h: u32) -> Pixmap {
    let (sw, sh) = (p.width(), p.height());
    if (sw, sh) == (w, h) {
        return p.clone();
    }
    let src: Vec<[f32; 4]> = p.pixels().iter().map(|c| [c.red(), c.green(), c.blue(), c.alpha()].map(f32::from)).collect();
    // across, then down
    let wx = weights(sw, w);
    let mut mid = vec![[0f32; 4]; (w * sh) as usize];
    for y in 0..sh as usize {
        for (x, taps) in wx.iter().enumerate() {
            let mut acc = [0f32; 4];
            for &(s, k) in taps {
                let c = src[y * sw as usize + s];
                for i in 0..4 {
                    acc[i] += c[i] * k;
                }
            }
            mid[y * w as usize + x] = acc;
        }
    }
    let wy = weights(sh, h);
    let mut out = blank(w, h);
    let pixels = out.pixels_mut();
    for (y, taps) in wy.iter().enumerate() {
        for x in 0..w as usize {
            let mut acc = [0f32; 4];
            for &(s, k) in taps {
                let c = mid[s * w as usize + x];
                for i in 0..4 {
                    acc[i] += c[i] * k;
                }
            }
            let a = acc[3].round().clamp(0.0, 255.0) as u8;
            let ch = |v: f32| (v.round().clamp(0.0, 255.0) as u8).min(a);
            pixels[y * w as usize + x] = PremultipliedColorU8::from_rgba(ch(acc[0]), ch(acc[1]), ch(acc[2]), a).expect("premultiplied");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(w: u32, h: u32, rgba: [u8; 4]) -> Pixmap {
        from_rgba(w, h, &rgba.repeat((w * h) as usize))
    }

    #[test]
    fn resizing_keeps_colour_and_coverage() {
        let red = solid(1024, 1024, [255, 0, 0, 255]);
        for side in [16, 20, 24, 32, 48, 256, 2048] {
            let r = resize(&red, side, side);
            assert_eq!((r.width(), r.height()), (side, side));
            assert!(to_rgba(&r).chunks_exact(4).all(|p| p == [255, 0, 0, 255]), "{side}");
        }
        // half see-through stays half see-through
        let half = resize(&solid(64, 64, [0, 0, 255, 128]), 16, 16);
        assert!(to_rgba(&half).chunks_exact(4).all(|p| p[3] == 128 && p[2] >= 254));
        // a 2 × 2 checker averages to grey
        let checker = from_rgba(2, 2, &[0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 255]);
        let one = to_rgba(&resize(&checker, 1, 1));
        assert!((127..=128).contains(&one[0]) && one[3] == 255, "{one:?}");
    }

    #[test]
    fn png_round_trip_and_squaring() {
        let p = solid(30, 10, [10, 20, 30, 255]);
        let back = decode_png(&encode_png(&p)).unwrap();
        assert_eq!(to_rgba(&back), to_rgba(&p));
        let sq = squared(&p);
        assert_eq!((sq.width(), sq.height()), (30, 30));
        assert_eq!(to_rgba(&sq)[3], 0, "the padding is see-through");
        assert_eq!(&to_rgba(&sq)[(15 * 30 + 15) * 4..][..4], &[10, 20, 30, 255]);
        assert!(decode_png(b"not a png").is_err());
    }
}

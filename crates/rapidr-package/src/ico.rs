//! Windows icons (`.ico`): written with PNG-compressed pictures (Windows
//! Vista and later), read with PNG or BMP pictures (1, 4, 8, 24 and 32 bits
//! per pixel with their see-through mask — RapidQ's 766-byte 32 × 32
//! sixteen-colour icons among them).

use resvg::tiny_skia::Pixmap;

use crate::picture;

/// The sizes a program's icon has on Windows: Explorer's views from small
/// icons to extra large, at 100 % to 250 % scaling.
pub const SIZES: [u32; 8] = [16, 20, 24, 32, 40, 48, 64, 256];

/// An `.ico` file holding `pictures` (each square, at most 256 px).
pub fn write(pictures: &[Pixmap]) -> Vec<u8> {
    let blobs: Vec<Vec<u8>> = pictures.iter().map(picture::encode_png).collect();
    let mut out = Vec::new();
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&(pictures.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * pictures.len();
    for (p, blob) in pictures.iter().zip(&blobs) {
        // (256 is written as 0)
        out.push((p.width() % 256) as u8);
        out.push((p.height() % 256) as u8);
        out.extend_from_slice(&[0, 0]);
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(&(blob.len() as u32).to_le_bytes());
        out.extend_from_slice(&(offset as u32).to_le_bytes());
        offset += blob.len();
    }
    for blob in blobs {
        out.extend_from_slice(&blob);
    }
    out
}

/// Whether `b` starts like an icon file.
pub fn is_ico(b: &[u8]) -> bool {
    b.len() >= 6 && b[0..4] == [0, 0, 1, 0] && u16::from_le_bytes([b[4], b[5]]) > 0
}

fn u16_at(b: &[u8], i: usize) -> Option<u32> {
    Some(u32::from(u16::from_le_bytes(b.get(i..i + 2)?.try_into().ok()?)))
}

fn u32_at(b: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(i..i + 4)?.try_into().ok()?))
}

/// Every picture of an `.ico` file.
pub fn read(b: &[u8]) -> Result<Vec<Pixmap>, String> {
    if !is_ico(b) {
        return Err("not a Windows icon (.ico)".into());
    }
    let count = u16_at(b, 4).unwrap_or(0) as usize;
    let mut out = Vec::new();
    let mut last_error = None;
    for i in 0..count {
        let e = 6 + i * 16;
        let (Some(size), Some(offset)) = (u32_at(b, e + 8), u32_at(b, e + 12)) else {
            return Err("the icon file is cut short".into());
        };
        let Some(data) = b.get(offset as usize..(offset as usize).saturating_add(size as usize)) else {
            return Err("the icon file is cut short".into());
        };
        match read_picture(data) {
            Ok(p) => out.push(p),
            Err(e) => last_error = Some(e),
        }
    }
    if out.is_empty() {
        return Err(last_error.unwrap_or_else(|| "the icon file has no pictures".into()));
    }
    Ok(out)
}

/// One picture of an icon: a PNG, or a BMP without its file header (its
/// height counting the see-through mask too).
fn read_picture(d: &[u8]) -> Result<Pixmap, String> {
    if d.starts_with(b"\x89PNG\r\n\x1a\n") {
        return picture::decode_png(d);
    }
    let bad = || "a picture in the icon file RapidR can't read".to_string();
    let header = u32_at(d, 0).ok_or_else(bad)? as usize;
    let w = u32_at(d, 4).ok_or_else(bad)? as i32;
    let h = (u32_at(d, 8).ok_or_else(bad)? as i32) / 2;
    let bpp = u16_at(d, 14).ok_or_else(bad)?;
    let compression = u32_at(d, 16).ok_or_else(bad)?;
    if header < 40 || w <= 0 || h <= 0 || w as u32 > 1024 || h as u32 > 1024 || compression != 0 {
        return Err(bad());
    }
    let (w, h) = (w as usize, h as usize);
    let colours = if bpp <= 8 {
        match u32_at(d, 32).unwrap_or(0) {
            0 => 1usize << bpp,
            n => (n as usize).min(256),
        }
    } else {
        0
    };
    let palette = header;
    let pixels_at = header + colours * 4;
    let stride = (w * bpp as usize).div_ceil(32) * 4;
    let mask_stride = w.div_ceil(32) * 4;
    let mask_at = pixels_at + stride * h;
    let mut rgba = vec![0u8; w * h * 4];
    let mut any_alpha = false;
    for row in 0..h {
        // (bottom row first)
        let line = d.get(pixels_at + (h - 1 - row) * stride..pixels_at + (h - row) * stride).ok_or_else(bad)?;
        for x in 0..w {
            let px = match bpp {
                32 => {
                    let p = &line[x * 4..x * 4 + 4];
                    any_alpha |= p[3] != 0;
                    [p[2], p[1], p[0], p[3]]
                }
                24 => {
                    let p = &line[x * 3..x * 3 + 3];
                    [p[2], p[1], p[0], 255]
                }
                1 | 4 | 8 => {
                    let bits = bpp as usize;
                    let bit = x * bits;
                    let index = (line[bit / 8] >> (8 - bits - bit % 8)) & ((1u16 << bits) - 1) as u8;
                    let c = d.get(palette + index as usize * 4..palette + index as usize * 4 + 4).ok_or_else(bad)?;
                    [c[2], c[1], c[0], 255]
                }
                _ => return Err(format!("an icon picture with {bpp} bits per pixel")),
            };
            rgba[(row * w + x) * 4..][..4].copy_from_slice(&px);
        }
    }
    // A 32-bit picture says how see-through it is itself; the others (and a
    // 32-bit one whose alpha is all 0) by their mask: a set bit is see-through.
    if bpp != 32 || !any_alpha {
        for row in 0..h {
            let Some(line) = d.get(mask_at + (h - 1 - row) * mask_stride..mask_at + (h - row) * mask_stride) else { break };
            for x in 0..w {
                let see_through = line[x / 8] >> (7 - x % 8) & 1 == 1;
                rgba[(row * w + x) * 4 + 3] = if see_through { 0 } else { 255 };
            }
        }
    }
    Ok(picture::from_rgba(w as u32, h as u32, &rgba))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn writes_and_reads_png_icons() {
        let pics: Vec<Pixmap> = SIZES.iter().map(|&s| picture::from_rgba(s, s, &[1, 2, 3, 255].repeat((s * s) as usize))).collect();
        let ico = write(&pics);
        assert!(is_ico(&ico));
        assert_eq!(u16_at(&ico, 4), Some(SIZES.len() as u32));
        // 256 is written as 0
        assert_eq!(ico[6 + 7 * 16], 0);
        let back = read(&ico).unwrap();
        assert_eq!(back.iter().map(Pixmap::width).collect::<Vec<_>>(), SIZES);
        assert_eq!(&picture::to_rgba(&back[0])[..4], &[1, 2, 3, 255]);
    }

    /// A RapidQ-style icon: 32 × 32, 16 colours, with its mask (766 bytes).
    pub(crate) fn rapidq_icon() -> Vec<u8> {
        let mut d = Vec::new();
        // the directory and its one entry
        d.extend_from_slice(&[0, 0, 1, 0, 1, 0, 32, 32, 16, 0, 1, 0, 4, 0]);
        d.extend_from_slice(&744u32.to_le_bytes());
        d.extend_from_slice(&22u32.to_le_bytes());
        // BITMAPINFOHEADER: 32 × 64 (with the mask), 4 bits
        d.extend_from_slice(&40u32.to_le_bytes());
        d.extend_from_slice(&32u32.to_le_bytes());
        d.extend_from_slice(&64u32.to_le_bytes());
        d.extend_from_slice(&1u16.to_le_bytes());
        d.extend_from_slice(&4u16.to_le_bytes());
        d.extend_from_slice(&[0; 24]);
        // 16 colours: 0 black, 1 red, the rest grey
        d.extend_from_slice(&[0, 0, 0, 0, 0, 0, 255, 0]);
        for _ in 2..16 {
            d.extend_from_slice(&[128, 128, 128, 0]);
        }
        // pixels: every row red (index 1)
        d.extend(std::iter::repeat_n(0x11u8, 16 * 32));
        // mask: the left half see-through
        for _ in 0..32 {
            d.extend_from_slice(&[0xFF, 0xFF, 0, 0]);
        }
        d
    }

    #[test]
    fn reads_rapidq_sixteen_colour_icons() {
        let ico = rapidq_icon();
        assert_eq!(ico.len(), 766);
        let pics = read(&ico).unwrap();
        assert_eq!((pics[0].width(), pics[0].height()), (32, 32));
        let rgba = picture::to_rgba(&pics[0]);
        assert_eq!(rgba[3], 0, "the mask's set bits are see-through");
        assert_eq!(&rgba[20 * 4..20 * 4 + 4], &[255, 0, 0, 255]);
        assert!(read(b"\0\0\x01\0\x01\0").is_err());
        assert!(read(b"GIF89a").is_err());
    }
}

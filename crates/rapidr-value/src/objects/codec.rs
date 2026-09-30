//! Byte-level helpers for the objects: RapidQ strings as bytes, base64, and
//! reading/writing uncompressed Windows BMP files (the format QBITMAP and
//! QIMAGELIST use). Written here rather than pulling in image crates, since
//! uncompressed BMP is all RapidQ's own objects read and write.

/// A RapidQ string as bytes: characters up to U+00FF are one byte each (as
/// CHR$ makes them); anything else is written as its UTF-8 bytes.
pub fn string_to_bytes(s: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    for c in s.chars() {
        if (c as u32) < 256 {
            out.push(c as u32 as u8);
        } else {
            let mut buf = [0u8; 4];
            out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
        }
    }
    out
}

/// Bytes as a RapidQ string, one character per byte (the inverse of CHR$).
pub fn bytes_to_string(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| char::from(b)).collect()
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let n = (chunk[0] as u32) << 16 | (*chunk.get(1).unwrap_or(&0) as u32) << 8 | *chunk.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(B64[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

pub fn base64_decode(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let (mut acc, mut bits) = (0u32, 0);
    for c in text.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' | b'\n' | b'\r' | b' ' => continue,
            _ => return None,
        };
        acc = acc << 6 | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
}

/// A decoded image: `pixels` are RapidQ colors (&HBBGGRR), row-major, top
/// row first.
#[derive(Debug, Clone, PartialEq)]
pub struct Pixels {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u32>,
}

fn u16_at(b: &[u8], i: usize) -> Option<u32> {
    Some(u16::from_le_bytes(b.get(i..i + 2)?.try_into().ok()?) as u32)
}

fn u32_at(b: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(i..i + 4)?.try_into().ok()?))
}

/// Largest image accepted (pixels), so a corrupt header can't exhaust memory.
pub const MAX_PIXELS: usize = 64 * 1024 * 1024;

/// Whether `b` is an SVG image (text starting with `<svg`, or XML holding one).
pub fn is_svg(b: &[u8]) -> bool {
    let head = String::from_utf8_lossy(&b[..b.len().min(1024)]).to_ascii_lowercase();
    let t = head.trim_start_matches('\u{feff}').trim_start();
    t.starts_with("<svg") || ((t.starts_with("<?xml") || t.starts_with("<!--") || t.starts_with("<!doctype")) && head.contains("<svg"))
}

/// Draws an SVG image at `scale` times its size (resvg): its pixels, and
/// each one's opacity (0..255) — SVGs have soft edges RapidQ's color-key
/// transparency can't hold.
pub fn decode_svg(b: &[u8], scale: f32) -> Result<(Pixels, Vec<u8>), String> {
    let tree = resvg::usvg::Tree::from_data(b, &resvg::usvg::Options::default()).map_err(|e| format!("not an SVG image RapidR can read ({e})"))?;
    let size = tree.size();
    let scale = if scale.is_finite() && scale > 0.0 { scale } else { 1.0 };
    let (w, h) = ((size.width() * scale).ceil().max(1.0) as u32, (size.height() * scale).ceil().max(1.0) as u32);
    if w as usize * h as usize > MAX_PIXELS {
        return Err("SVG image too large".into());
    }
    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h).ok_or("SVG image too large")?;
    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    let mut pixels = Vec::with_capacity((w * h) as usize);
    let mut alpha = Vec::with_capacity((w * h) as usize);
    for p in pixmap.pixels() {
        let c = p.demultiply();
        pixels.push(u32::from(c.blue()) << 16 | u32::from(c.green()) << 8 | u32::from(c.red()));
        alpha.push(c.alpha());
    }
    Ok((Pixels { width: w as usize, height: h as usize, pixels }, alpha))
}

/// Decodes an uncompressed BMP (1, 4, 8, 24 or 32 bits per pixel).
pub fn decode_bmp(b: &[u8]) -> Result<Pixels, String> {
    decode_bmp_alpha(b).map(|(img, _)| img)
}

/// Decodes a BMP, with its pixels' opacity when it is a 32-bit one whose
/// fourth bytes hold one (RapidR's own soft-edged images; other programs
/// leave them all 0 or all 255).
pub fn decode_bmp_alpha(b: &[u8]) -> Result<(Pixels, Option<Vec<u8>>), String> {
    let img = decode_bmp_pixels(b)?;
    let alpha = (u16_at(b, 28) == Some(32) && u32_at(b, 30) == Some(0))
        .then(|| {
            let data_offset = u32_at(b, 10)? as usize;
            let raw_height = u32_at(b, 22)? as i32;
            let row_bytes = img.width * 4;
            let mut alpha = vec![0u8; img.width * img.height];
            for row in 0..img.height {
                let src_row = if raw_height > 0 { img.height - 1 - row } else { row };
                let line = b.get(data_offset + src_row * row_bytes..data_offset + (src_row + 1) * row_bytes)?;
                for x in 0..img.width {
                    alpha[row * img.width + x] = line[x * 4 + 3];
                }
            }
            Some(alpha)
        })
        .flatten()
        .filter(|a| a.iter().any(|&v| v != 0) && a.iter().any(|&v| v != 255));
    Ok((img, alpha))
}

fn decode_bmp_pixels(b: &[u8]) -> Result<Pixels, String> {
    let bad = || "not a BMP file RapidR can read (uncompressed 1/4/8/24/32-bit)".to_string();
    if b.get(0..2) != Some(b"BM") {
        return Err(bad());
    }
    let data_offset = u32_at(b, 10).ok_or_else(bad)? as usize;
    let header_size = u32_at(b, 14).ok_or_else(bad)? as usize;
    let width = u32_at(b, 18).ok_or_else(bad)? as i32;
    let raw_height = u32_at(b, 22).ok_or_else(bad)? as i32;
    let bpp = u16_at(b, 28).ok_or_else(bad)?;
    let compression = u32_at(b, 30).ok_or_else(bad)?;
    // BI_RGB, or BI_BITFIELDS for 32-bit files with the standard masks.
    if width <= 0 || raw_height == 0 || !(compression == 0 || compression == 3 && bpp == 32) {
        return Err(bad());
    }
    let (width, height) = (width as usize, raw_height.unsigned_abs() as usize);
    if width.saturating_mul(height) > MAX_PIXELS {
        return Err("BMP image is too large".into());
    }
    let palette: Vec<u32> = if bpp <= 8 {
        let count = match u32_at(b, 46).unwrap_or(0) {
            0 => 1usize << bpp,
            n => (n as usize).min(256),
        };
        (0..count)
            .map(|i| {
                let at = 14 + header_size + i * 4;
                // Palette entries are B, G, R, 0 — RapidQ colors are &HBBGGRR.
                u32_at(b, at).map_or(0, |bgr0| {
                    let (bl, g, r) = (bgr0 & 0xFF, bgr0 >> 8 & 0xFF, bgr0 >> 16 & 0xFF);
                    bl << 16 | g << 8 | r
                })
            })
            .collect()
    } else {
        Vec::new()
    };
    let row_bytes = (width * bpp as usize).div_ceil(32) * 4;
    let mut pixels = vec![0u32; width * height];
    for row in 0..height {
        let src_row = if raw_height > 0 { height - 1 - row } else { row };
        let start = data_offset + src_row * row_bytes;
        let line = b.get(start..start + row_bytes).ok_or_else(bad)?;
        for x in 0..width {
            let color = match bpp {
                24 | 32 => {
                    let at = x * (bpp as usize / 8);
                    (line[at] as u32) << 16 | (line[at + 1] as u32) << 8 | line[at + 2] as u32
                }
                8 => *palette.get(line[x] as usize).unwrap_or(&0),
                4 => *palette.get((line[x / 2] >> if x % 2 == 0 { 4 } else { 0 } & 0xF) as usize).unwrap_or(&0),
                1 => *palette.get((line[x / 8] >> (7 - x % 8) & 1) as usize).unwrap_or(&0),
                _ => return Err(bad()),
            };
            pixels[row * width + x] = color;
        }
    }
    Ok(Pixels { width, height, pixels })
}

/// Encodes a 24-bit uncompressed BMP.
pub fn encode_bmp(img: &Pixels) -> Vec<u8> {
    let row_bytes = (img.width * 3).div_ceil(4) * 4;
    let image_size = row_bytes * img.height;
    let mut out = Vec::with_capacity(54 + image_size);
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&(54 + image_size as u32).to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&54u32.to_le_bytes());
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(img.width as i32).to_le_bytes());
    out.extend_from_slice(&(img.height as i32).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&24u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&(image_size as u32).to_le_bytes());
    out.extend_from_slice(&2835u32.to_le_bytes()); // 72 DPI
    out.extend_from_slice(&2835u32.to_le_bytes());
    out.extend_from_slice(&[0; 8]);
    for row in (0..img.height).rev() {
        for x in 0..img.width {
            let c = img.pixels[row * img.width + x];
            out.extend_from_slice(&[(c >> 16) as u8, (c >> 8) as u8, c as u8]);
        }
        out.resize(out.len() + row_bytes - img.width * 3, 0);
    }
    out
}

/// Encodes a 32-bit BMP whose fourth bytes are the pixels' opacity.
pub fn encode_bmp_alpha(img: &Pixels, alpha: &[u8]) -> Vec<u8> {
    let image_size = img.width * 4 * img.height;
    let mut out = Vec::with_capacity(54 + image_size);
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&(54 + image_size as u32).to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&54u32.to_le_bytes());
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(img.width as i32).to_le_bytes());
    out.extend_from_slice(&(img.height as i32).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&(image_size as u32).to_le_bytes());
    out.extend_from_slice(&2835u32.to_le_bytes());
    out.extend_from_slice(&2835u32.to_le_bytes());
    out.extend_from_slice(&[0; 8]);
    for row in (0..img.height).rev() {
        for x in 0..img.width {
            let i = row * img.width + x;
            let c = img.pixels[i];
            out.extend_from_slice(&[(c >> 16) as u8, (c >> 8) as u8, c as u8, alpha.get(i).copied().unwrap_or(255)]);
        }
    }
    out
}

/// The prefix of the `data:` URLs a QBITMAP's `.BMP` property returns.
pub const BMP_DATA_URL: &str = "data:image/bmp;base64,";

/// The prefix of the `.BMP` of an image that is an SVG's (not drawn on):
/// the SVG itself, so what it's drawn onto can draw it at any scale.
pub const SVG_DATA_URL: &str = "data:image/svg+xml;base64,";

pub fn bmp_data_url(img: &Pixels) -> String {
    format!("{BMP_DATA_URL}{}", base64_encode(&encode_bmp(img)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_round_trip() {
        for s in ["", "a", "ab", "abc", "hello world!"] {
            assert_eq!(base64_decode(&base64_encode(s.as_bytes())).unwrap(), s.as_bytes());
        }
        assert_eq!(base64_encode(b"Man"), "TWFu");
        assert_eq!(base64_encode(b"Ma"), "TWE=");
    }

    #[test]
    fn bmp_round_trip() {
        let img = Pixels { width: 3, height: 2, pixels: vec![0x0000FF, 0x00FF00, 0xFF0000, 0, 0xFFFFFF, 0x123456] };
        let bytes = encode_bmp(&img);
        assert_eq!(decode_bmp(&bytes).unwrap(), img);
        assert!(decode_bmp(b"GIF89a").is_err());
    }

    #[test]
    fn svg_images() {
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="2"><rect width="2" height="2" fill="#ff0000"/></svg>"##;
        assert!(is_svg(svg) && is_svg(b"<?xml version='1.0'?>\n<svg/>") && !is_svg(b"BM...."));
        let (img, alpha) = decode_svg(svg, 1.0).unwrap();
        assert_eq!((img.width, img.height), (4, 2));
        assert_eq!((img.pixels[0], alpha[0]), (0x0000FF, 255));
        assert_eq!(alpha[3], 0);
        // (kept through a BMP: `.BMP` of a soft-edged image)
        let (back, soft) = decode_bmp_alpha(&encode_bmp_alpha(&img, &alpha)).unwrap();
        assert_eq!((back, soft), (img.clone(), Some(alpha.clone())));
        let (big, _) = decode_svg(svg, 2.0).unwrap();
        assert_eq!((big.width, big.height), (8, 4));
    }

    #[test]
    fn strings_are_bytes() {
        let s = bytes_to_string(&[72, 0, 200, 255]);
        assert_eq!(string_to_bytes(&s), [72, 0, 200, 255]);
    }
}

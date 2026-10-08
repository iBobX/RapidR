//! macOS icons (`.icns`), as Apple's `iconutil` writes them: PNG pictures
//! from 32 to 1024 px (16 to 512 points at 1x and 2x) and the 16 and 32 px
//! ones as run-length-encoded ARGB. Reading takes every PNG or ARGB picture
//! of a file (JPEG 2000 ones, from old icons, are skipped).

use resvg::tiny_skia::Pixmap;

use crate::picture;

/// Each entry `iconutil` writes: its type and its pixel size.
pub const ENTRIES: [(&[u8; 4], u32); 10] = [
    (b"ic12", 64),   // 32 pt @2x
    (b"ic07", 128),  // 128 pt
    (b"ic13", 256),  // 128 pt @2x
    (b"ic08", 256),  // 256 pt
    (b"ic04", 16),   // 16 pt (ARGB)
    (b"ic14", 512),  // 256 pt @2x
    (b"ic09", 512),  // 512 pt
    (b"ic05", 32),   // 32 pt (ARGB)
    (b"ic10", 1024), // 512 pt @2x
    (b"ic11", 32),   // 16 pt @2x
];

/// The pixel sizes an `.icns` holds.
pub const SIZES: [u32; 6] = [16, 32, 64, 128, 256, 512];

/// An `.icns` file: `picture(px)` gives each size.
pub fn write(mut picture_at: impl FnMut(u32) -> Pixmap) -> Vec<u8> {
    let mut body = Vec::new();
    let mut cache: Vec<(u32, Pixmap)> = Vec::new();
    for (kind, px) in ENTRIES {
        let p = match cache.iter().find(|(s, _)| *s == px) {
            Some((_, p)) => p.clone(),
            None => {
                let p = picture_at(px);
                cache.push((px, p.clone()));
                p
            }
        };
        let data = if kind == b"ic04" || kind == b"ic05" { argb(&p) } else { picture::encode_png(&p) };
        body.extend_from_slice(kind);
        body.extend_from_slice(&((data.len() + 8) as u32).to_be_bytes());
        body.extend_from_slice(&data);
    }
    let mut out = Vec::with_capacity(body.len() + 8);
    out.extend_from_slice(b"icns");
    out.extend_from_slice(&((body.len() + 8) as u32).to_be_bytes());
    out.extend_from_slice(&body);
    out
}

/// `ARGB` then the alpha, red, green and blue planes, each run-length
/// encoded (a byte n < 128: n + 1 bytes follow as they are; n ≥ 128: the
/// next byte n − 125 times).
fn argb(p: &Pixmap) -> Vec<u8> {
    let rgba = picture::to_rgba(p);
    let mut out = b"ARGB".to_vec();
    for channel in [3, 0, 1, 2] {
        let plane: Vec<u8> = rgba.chunks_exact(4).map(|px| px[channel]).collect();
        rle(&plane, &mut out);
    }
    out
}

fn rle(data: &[u8], out: &mut Vec<u8>) {
    let mut i = 0;
    let mut literal: Vec<u8> = Vec::new();
    let flush = |literal: &mut Vec<u8>, out: &mut Vec<u8>| {
        for chunk in literal.chunks(128) {
            out.push((chunk.len() - 1) as u8);
            out.extend_from_slice(chunk);
        }
        literal.clear();
    };
    while i < data.len() {
        let mut run = 1;
        while i + run < data.len() && data[i + run] == data[i] && run < 130 {
            run += 1;
        }
        if run >= 3 {
            flush(&mut literal, out);
            out.push((run + 125) as u8);
            out.push(data[i]);
            i += run;
        } else {
            literal.push(data[i]);
            i += 1;
        }
    }
    flush(&mut literal, out);
}

fn unrle(data: &[u8], len: usize) -> Option<(Vec<u8>, usize)> {
    let mut out = Vec::with_capacity(len);
    let mut i = 0;
    while out.len() < len {
        let n = *data.get(i)? as usize;
        i += 1;
        if n < 128 {
            out.extend_from_slice(data.get(i..i + n + 1)?);
            i += n + 1;
        } else {
            out.extend(std::iter::repeat_n(*data.get(i)?, n - 125));
            i += 1;
        }
    }
    out.truncate(len);
    Some((out, i))
}

/// Whether `b` starts like an `.icns` file.
pub fn is_icns(b: &[u8]) -> bool {
    b.starts_with(b"icns")
}

/// Every picture of an `.icns` file that RapidR can read.
pub fn read(b: &[u8]) -> Result<Vec<Pixmap>, String> {
    if !is_icns(b) || b.len() < 8 {
        return Err("not a macOS icon (.icns)".into());
    }
    let end = (u32::from_be_bytes(b[4..8].try_into().unwrap()) as usize).min(b.len());
    let mut at = 8;
    let mut out = Vec::new();
    while at + 8 <= end {
        let kind = &b[at..at + 4];
        let len = u32::from_be_bytes(b[at + 4..at + 8].try_into().unwrap()) as usize;
        if len < 8 || at + len > end {
            return Err("the .icns file is cut short".into());
        }
        let data = &b[at + 8..at + len];
        if data.starts_with(b"\x89PNG\r\n\x1a\n") {
            if let Ok(p) = picture::decode_png(data) {
                out.push(p);
            }
        } else if data.starts_with(b"ARGB") {
            let side = match kind {
                b"ic04" => Some(16),
                b"ic05" => Some(32),
                _ => None,
            };
            if let Some(side) = side {
                let n = side * side;
                let mut planes = Vec::new();
                let mut rest = &data[4..];
                for _ in 0..4 {
                    let Some((plane, used)) = unrle(rest, n) else { break };
                    planes.push(plane);
                    rest = &rest[used..];
                }
                if planes.len() == 4 {
                    let rgba: Vec<u8> = (0..n).flat_map(|i| [planes[1][i], planes[2][i], planes[3][i], planes[0][i]]).collect();
                    out.push(picture::from_rgba(side as u32, side as u32, &rgba));
                }
            }
        }
        at += len;
    }
    if out.is_empty() {
        return Err("the .icns file has no picture RapidR can read (PNG or ARGB)".into());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_every_size_and_reads_them_back() {
        let icns = write(|px| picture::from_rgba(px, px, &[200, 100, 50, if px == 16 { 128 } else { 255 }].repeat((px * px) as usize)));
        assert!(is_icns(&icns));
        assert_eq!(u32::from_be_bytes(icns[4..8].try_into().unwrap()) as usize, icns.len());
        let pics = read(&icns).unwrap();
        let mut sides: Vec<u32> = pics.iter().map(Pixmap::width).collect();
        sides.sort();
        assert_eq!(sides, [16, 32, 32, 64, 128, 256, 256, 512, 512, 1024]);
        // the ARGB 16 px picture comes back as it was
        let p16 = pics.iter().find(|p| p.width() == 16).unwrap();
        let px = &picture::to_rgba(p16)[..4];
        assert_eq!(px[3], 128);
        assert!((px[0] as i32 - 200).abs() <= 1 && (px[1] as i32 - 100).abs() <= 1, "{px:?}");
    }

    #[test]
    fn run_length_round_trip() {
        for data in [vec![], vec![7u8; 300], (0..=255u8).collect(), vec![1, 1, 2, 2, 2, 2, 3, 4, 4]] {
            let mut enc = Vec::new();
            rle(&data, &mut enc);
            assert_eq!(unrle(&enc, data.len()).unwrap().0, data);
        }
        assert!(read(b"icns\0\0\0\x08").is_err());
        assert!(read(b"PNG").is_err());
    }
}

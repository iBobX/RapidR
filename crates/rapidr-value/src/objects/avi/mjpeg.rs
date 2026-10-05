//! Motion JPEG ('MJPG', 'AVRn', 'dmb1', 'MJPA'): every frame a baseline
//! JPEG, decoded by the JPEG decoder RapidR has (`codec::decode_jpeg`).
//! Two things set MJPEG apart (the OpenDML AVI File Format Extensions):
//! frames may leave out the Huffman tables (DHT) — they are then the
//! standard ones of ITU T.81 Annex K.3, put in before the scan here — and
//! an interlaced frame holds its two fields as two JPEGs one after the
//! other, each half the height: they are woven back together (or the
//! first one line-doubled when the second can't be read).

use std::borrow::Cow;

use super::super::codec;

/// (bits per code length, values) of Annex K's four tables: DC and AC,
/// luminance (table 0) and chrominance (table 1).
const DC_COUNTS: [[u8; 16]; 2] = [[0, 1, 5, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0], [0, 3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0]];
const DC_VALUES: [u8; 12] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
const AC_COUNTS: [[u8; 16]; 2] = [[0, 2, 1, 3, 3, 2, 4, 3, 5, 5, 4, 4, 0, 0, 1, 0x7D], [0, 2, 1, 2, 4, 4, 3, 4, 7, 5, 4, 4, 0, 1, 2, 0x77]];
const AC_LUMA: [u8; 162] = [
    0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12, 0x21, 0x31, 0x41, 0x06, 0x13, 0x51, 0x61, 0x07, 0x22, 0x71, 0x14, 0x32, 0x81, 0x91, 0xA1, 0x08, 0x23, 0x42, 0xB1, 0xC1, 0x15, 0x52, 0xD1, 0xF0, 0x24, 0x33,
    0x62, 0x72, 0x82, 0x09, 0x0A, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2A, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3A, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4A, 0x53, 0x54, 0x55,
    0x56, 0x57, 0x58, 0x59, 0x5A, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6A, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7A, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8A, 0x92, 0x93, 0x94, 0x95, 0x96,
    0x97, 0x98, 0x99, 0x9A, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6, 0xA7, 0xA8, 0xA9, 0xAA, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xB7, 0xB8, 0xB9, 0xBA, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7, 0xC8, 0xC9, 0xCA, 0xD2, 0xD3, 0xD4,
    0xD5, 0xD6, 0xD7, 0xD8, 0xD9, 0xDA, 0xE1, 0xE2, 0xE3, 0xE4, 0xE5, 0xE6, 0xE7, 0xE8, 0xE9, 0xEA, 0xF1, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xF7, 0xF8, 0xF9, 0xFA,
];
const AC_CHROMA: [u8; 162] = [
    0x00, 0x01, 0x02, 0x03, 0x11, 0x04, 0x05, 0x21, 0x31, 0x06, 0x12, 0x41, 0x51, 0x07, 0x61, 0x71, 0x13, 0x22, 0x32, 0x81, 0x08, 0x14, 0x42, 0x91, 0xA1, 0xB1, 0xC1, 0x09, 0x23, 0x33, 0x52, 0xF0, 0x15, 0x62,
    0x72, 0xD1, 0x0A, 0x16, 0x24, 0x34, 0xE1, 0x25, 0xF1, 0x17, 0x18, 0x19, 0x1A, 0x26, 0x27, 0x28, 0x29, 0x2A, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3A, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4A, 0x53, 0x54,
    0x55, 0x56, 0x57, 0x58, 0x59, 0x5A, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6A, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7A, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8A, 0x92, 0x93, 0x94,
    0x95, 0x96, 0x97, 0x98, 0x99, 0x9A, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6, 0xA7, 0xA8, 0xA9, 0xAA, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xB7, 0xB8, 0xB9, 0xBA, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7, 0xC8, 0xC9, 0xCA, 0xD2,
    0xD3, 0xD4, 0xD5, 0xD6, 0xD7, 0xD8, 0xD9, 0xDA, 0xE2, 0xE3, 0xE4, 0xE5, 0xE6, 0xE7, 0xE8, 0xE9, 0xEA, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xF7, 0xF8, 0xF9, 0xFA,
];

/// The DHT segment holding the four standard tables.
fn standard_dht() -> Vec<u8> {
    let mut seg = vec![0xFF, 0xC4, 0, 0];
    for (class_id, counts, values) in [
        (0x00, &DC_COUNTS[0], &DC_VALUES[..]),
        (0x10, &AC_COUNTS[0], &AC_LUMA[..]),
        (0x01, &DC_COUNTS[1], &DC_VALUES[..]),
        (0x11, &AC_COUNTS[1], &AC_CHROMA[..]),
    ] {
        seg.push(class_id);
        seg.extend_from_slice(counts);
        seg.extend_from_slice(values);
    }
    let len = (seg.len() - 2) as u16;
    seg[2..4].copy_from_slice(&len.to_be_bytes());
    seg
}

/// What a JPEG's markers say: where its scan starts, whether it has a DHT
/// segment, its size (SOFn).
struct Markers {
    sos: Option<usize>,
    dht: bool,
    size: Option<(usize, usize)>,
}

fn markers(jpeg: &[u8]) -> Markers {
    let mut m = Markers { sos: None, dht: false, size: None };
    let mut p = 2;
    while p + 4 <= jpeg.len() && jpeg[p] == 0xFF {
        let len = (jpeg[p + 2] as usize) << 8 | jpeg[p + 3] as usize;
        match jpeg[p + 1] {
            0xFF => {
                // (fill byte)
                p += 1;
                continue;
            }
            0x01 | 0xD0..=0xD8 => {
                p += 2;
                continue;
            }
            0xDA => {
                m.sos = Some(p);
                break;
            }
            0xC4 => m.dht = true,
            0xC0..=0xCF if !matches!(jpeg[p + 1], 0xC8 | 0xCC) => {
                if let Some(d) = jpeg.get(p + 5..p + 9) {
                    m.size = Some(((d[2] as usize) << 8 | d[3] as usize, (d[0] as usize) << 8 | d[1] as usize));
                }
            }
            _ => {}
        }
        p += 2 + len;
    }
    m
}

/// The JPEG with the standard tables put in before its scan when it has
/// no DHT of its own.
pub(super) fn with_tables(jpeg: &[u8]) -> Cow<'_, [u8]> {
    match markers(jpeg) {
        Markers { sos: Some(at), dht: false, .. } => {
            let mut out = Vec::with_capacity(jpeg.len() + 420);
            out.extend_from_slice(&jpeg[..at]);
            out.extend_from_slice(&standard_dht());
            out.extend_from_slice(&jpeg[at..]);
            Cow::Owned(out)
        }
        _ => Cow::Borrowed(jpeg),
    }
}

/// Where a second JPEG starts after the first's end (an interlaced
/// frame's second field).
fn second_field(b: &[u8]) -> Option<&[u8]> {
    let eoi = b.windows(2).position(|w| w == [0xFF, 0xD9])?;
    let rest = b.get(eoi + 2..)?;
    let soi = rest.windows(3).position(|w| w == [0xFF, 0xD8, 0xFF])?;
    Some(&rest[soi..])
}

/// The JPEG at the start of `b`, when it is no bigger than `max` pixels
/// (a damaged size would only make a big picture slowly).
fn decode_one(b: &[u8], max: usize) -> Option<codec::Pixels> {
    let start = b.windows(2).position(|w| w == [0xFF, 0xD8])?;
    let jpeg = &b[start..];
    let (w, h) = markers(jpeg).size?;
    if w * h > max {
        return None;
    }
    codec::decode_jpeg(&with_tables(jpeg)).ok()
}

/// Decodes one frame into `rgb` (w × h); false when it isn't a JPEG
/// RapidR can read (the picture is then left as it was).
pub(super) fn decode(data: &[u8], w: usize, h: usize, rgb: &mut [u32]) -> bool {
    let max = (w * h).max(64 * 64) * 2;
    let Some(first) = decode_one(data, max) else { return false };
    let fields = first.height < h && first.height * 2 >= h;
    let second = if fields {
        let start = data.windows(2).position(|w| w == [0xFF, 0xD8]).unwrap_or(0);
        second_field(&data[start..]).and_then(|b| decode_one(b, max)).filter(|s| s.width == first.width && s.height == first.height)
    } else {
        None
    };
    for y in 0..h {
        let (src, sy) = match (&second, fields) {
            (Some(s), true) if y % 2 == 1 => (s, y / 2),
            (_, true) => (&first, y / 2),
            _ => (&first, y),
        };
        if sy >= src.height {
            continue;
        }
        let n = w.min(src.width);
        rgb[y * w..y * w + n].copy_from_slice(&src.pixels[sy * src.width..sy * src.width + n]);
    }
    true
}

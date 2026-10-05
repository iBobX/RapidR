//! Uncompressed DIB frames (BI_RGB, BI_BITFIELDS): the rows of a Windows
//! bitmap without its file header, as Video for Windows stores them —
//! bottom-up unless biHeight is negative, each row padded to 4 bytes.

/// How an uncompressed frame's pixels are laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Layout {
    pub bits: u16,
    pub top_down: bool,
    /// BI_BITFIELDS' red, green and blue masks (16 and 32 bits; 16-bit
    /// BI_RGB is X1R5G5B5, 32-bit BGRX).
    pub masks: [u32; 3],
}

impl Layout {
    pub fn new(bits: u16, top_down: bool, masks: Option<[u32; 3]>) -> Layout {
        let masks = masks.filter(|m| m.iter().all(|&v| v != 0)).unwrap_or(match bits {
            16 => [0x7C00, 0x03E0, 0x001F],
            _ => [0x00FF_0000, 0x0000_FF00, 0x0000_00FF],
        });
        Layout { bits, top_down, masks }
    }

    /// Bytes a row takes in a frame of `len` bytes (4-byte aligned, or
    /// unpadded when the frame is exactly that size: some writers did).
    fn stride(&self, w: usize, h: usize, len: usize) -> usize {
        let packed = (w * self.bits as usize).div_ceil(8);
        let padded = packed.div_ceil(4) * 4;
        if len != padded * h && len == packed * h {
            packed
        } else {
            padded
        }
    }

    /// The image row a stored row lands on.
    fn row(&self, stored: usize, h: usize) -> usize {
        if self.top_down {
            stored
        } else {
            h - 1 - stored
        }
    }
}

/// A 1/4/8-bit frame's palette indexes, into `idx` (w × h, top row first).
pub(super) fn decode_indexed(data: &[u8], w: usize, h: usize, layout: &Layout, idx: &mut [u8]) {
    let stride = layout.stride(w, h, data.len());
    if stride == 0 {
        return;
    }
    for (stored, line) in data.chunks_exact(stride).take(h).enumerate() {
        let out = &mut idx[layout.row(stored, h) * w..][..w];
        match layout.bits {
            8 => out.copy_from_slice(&line[..w]),
            4 => {
                for (x, o) in out.iter_mut().enumerate() {
                    *o = line[x / 2] >> if x % 2 == 0 { 4 } else { 0 } & 0xF;
                }
            }
            _ => {
                for (x, o) in out.iter_mut().enumerate() {
                    *o = line[x / 8] >> (7 - x % 8) & 1;
                }
            }
        }
    }
}

/// One channel of a 16/32-bit pixel as 0..255: its mask's bits, widened by
/// repeating them (so 5-bit 31 is 255) through a table.
struct Channel {
    shift: u32,
    bits: u32,
    wide: [u8; 256],
}

impl Channel {
    fn new(mask: u32) -> Channel {
        let shift = mask.trailing_zeros().min(31);
        let bits = (mask >> shift).trailing_ones().clamp(1, 32);
        let mut wide = [0u8; 256];
        if bits < 8 {
            for (v, o) in wide.iter_mut().enumerate().take(1 << bits) {
                // (abcde → abcdeabc)
                let (mut out, mut have) = (0u32, 0);
                while have < 8 {
                    out = out << bits | v as u32;
                    have += bits;
                }
                *o = (out >> (have - 8)) as u8;
            }
        }
        Channel { shift, bits, wide }
    }

    #[inline]
    fn get(&self, px: u32) -> u32 {
        let v = (px >> self.shift) & ((1u64 << self.bits) - 1) as u32;
        if self.bits >= 8 {
            v >> (self.bits - 8)
        } else {
            u32::from(self.wide[v as usize])
        }
    }
}

/// A 16/24/32-bit frame as &HBBGGRR colors, into `rgb`.
pub(super) fn decode_rgb(data: &[u8], w: usize, h: usize, layout: &Layout, rgb: &mut [u32]) {
    let stride = layout.stride(w, h, data.len());
    if stride == 0 {
        return;
    }
    let [r, g, b] = layout.masks.map(Channel::new);
    let standard32 = layout.masks == [0x00FF_0000, 0x0000_FF00, 0x0000_00FF];
    for (stored, line) in data.chunks_exact(stride).take(h).enumerate() {
        let out = &mut rgb[layout.row(stored, h) * w..][..w];
        match layout.bits {
            16 => {
                for (o, p) in out.iter_mut().zip(line.as_chunks::<2>().0) {
                    let px = u32::from(u16::from_le_bytes([p[0], p[1]]));
                    *o = b.get(px) << 16 | g.get(px) << 8 | r.get(px);
                }
            }
            24 => {
                for (o, p) in out.iter_mut().zip(line.as_chunks::<3>().0) {
                    *o = u32::from(p[0]) << 16 | u32::from(p[1]) << 8 | u32::from(p[2]);
                }
            }
            _ if standard32 => {
                for (o, p) in out.iter_mut().zip(line.as_chunks::<4>().0) {
                    *o = u32::from(p[0]) << 16 | u32::from(p[1]) << 8 | u32::from(p[2]);
                }
            }
            _ => {
                for (o, p) in out.iter_mut().zip(line.as_chunks::<4>().0) {
                    let px = u32::from_le_bytes([p[0], p[1], p[2], p[3]]);
                    *o = b.get(px) << 16 | g.get(px) << 8 | r.get(px);
                }
            }
        }
    }
}

//! Microsoft Video 1 ('CRAM', 'MSVC', 'WHAM'): 4×4 blocks, the bottom row
//! of blocks first, each left to right. A block starts with a 16-bit word:
//! 0x84xx–0x87xx skips that many blocks (they keep the frame before), a
//! word with the top bit clear holds the block's 16 pixels' bits (bit 0
//! its bottom-left pixel, a row of four at a time going up) and two colors
//! follow — or, eight colors, a pair per 2×2 quadrant (bottom-left,
//! bottom-right, top-left, top-right): 8-bit streams mark those with a top
//! byte of 0x90 or more, 16-bit (RGB555) ones with the first color's top
//! bit —, and anything else fills the block with one color (8-bit: the
//! word's low byte; 16-bit: the word).

/// Where the next block goes, and how many to skip.
struct Blocks {
    w: usize,
    h: usize,
    across: usize,
    total: usize,
    n: usize,
}

impl Blocks {
    fn new(w: usize, h: usize) -> Blocks {
        Blocks { w, h, across: w / 4, total: (w / 4) * (h / 4), n: 0 }
    }

    /// The image offset of pixel (x, y-from-the-bottom) of block `n`.
    #[inline]
    fn at(&self, x: usize, y: usize) -> usize {
        let (bx, row) = (self.n % self.across, self.n / self.across);
        (self.h - 1 - (row * 4 + y)) * self.w + bx * 4 + x
    }
}

/// Which color of a block's eight (or two) a pixel takes.
#[inline]
fn pick(flags: u16, x: usize, y: usize, eight: bool) -> usize {
    let one = flags >> (y * 4 + x) & 1 == 1;
    let pair = if eight { ((y & 2) << 1) + (x & 2) } else { 0 };
    pair + if one { 0 } else { 1 }
}

/// An 8-bit (palettized) frame into `idx`, over what is there.
pub(super) fn decode8(data: &[u8], w: usize, h: usize, idx: &mut [u8]) {
    let mut b = Blocks::new(w, h);
    let mut p = 0;
    while b.n < b.total {
        let Some(&[lo, hi]) = data.get(p..p + 2) else { break };
        p += 2;
        if (0x84..=0x87).contains(&hi) {
            b.n += (hi as usize - 0x84) << 8 | lo as usize;
            continue;
        }
        let flags = u16::from_le_bytes([lo, hi]);
        let colors: &[u8] = if hi < 0x80 {
            let Some(c) = data.get(p..p + 2) else { break };
            p += 2;
            c
        } else if hi >= 0x90 {
            let Some(c) = data.get(p..p + 8) else { break };
            p += 8;
            c
        } else {
            std::slice::from_ref(&data[p - 2])
        };
        for y in 0..4 {
            for x in 0..4 {
                let c = match colors.len() {
                    1 => colors[0],
                    n => colors[pick(flags, x, y, n == 8)],
                };
                idx[b.at(x, y)] = c;
            }
        }
        b.n += 1;
    }
}

/// RGB555 as &HBBGGRR (5 bits widened by repeating them).
#[inline]
fn color555(c: u16) -> u32 {
    let wide = |v: u16| {
        let v = u32::from(v & 0x1F);
        v << 3 | v >> 2
    };
    wide(c) << 16 | wide(c >> 5) << 8 | wide(c >> 10)
}

/// A 16-bit frame into `rgb`, over what is there.
pub(super) fn decode16(data: &[u8], w: usize, h: usize, rgb: &mut [u32]) {
    let mut b = Blocks::new(w, h);
    let mut p = 0;
    let word = |p: usize| data.get(p..p + 2).map(|s| u16::from_le_bytes([s[0], s[1]]));
    let mut colors = [0u32; 8];
    while b.n < b.total {
        let Some(flags) = word(p) else { break };
        p += 2;
        let hi = (flags >> 8) as u8;
        if (0x84..=0x87).contains(&hi) {
            b.n += (hi as usize - 0x84) << 8 | (flags & 0xFF) as usize;
            continue;
        }
        let count = if hi & 0x80 == 0 {
            let Some(first) = word(p) else { break };
            let n = if first & 0x8000 != 0 { 8 } else { 2 };
            for (i, c) in colors.iter_mut().take(n).enumerate() {
                let Some(v) = word(p + i * 2) else { return };
                *c = color555(v);
            }
            p += n * 2;
            n
        } else {
            colors[0] = color555(flags);
            1
        };
        for y in 0..4 {
            for x in 0..4 {
                rgb[b.at(x, y)] = if count == 1 { colors[0] } else { colors[pick(flags, x, y, count == 8)] };
            }
        }
        b.n += 1;
    }
}

//! Cinepak ('cvid', SuperMac/Radius): vector quantization of 4×4 blocks.
//!
//! A frame is a 10-byte header (flags, 24-bit size, width, height, strip
//! count; all big-endian) and horizontal strips. A strip (id 0x10 key /
//! 0x11 inter, 24-bit size, then top, left, bottom, right — a top of 0
//! meaning "below the strip before", its bottom then a height) holds
//! chunks (16-bit id, 16-bit size):
//!
//! - codebooks, 0x20xx–0x27xx: 256 entries each for V4 and V1 (id bit
//!   0x0200 V1), an entry 2×2 pixels — four Y and a shared U, V (6 bytes),
//!   or four bytes (bit 0x0400: grey levels, or palette indexes in an 8-bit
//!   file). Bit 0x0100: selective, a 32-bit flag word before each 32
//!   entries says which follow.
//! - vectors, 0x30xx: each 4×4 block, top-down, is a V1 index (one entry
//!   blown up 2×) or four V4 indexes (one entry a quadrant: top-left,
//!   top-right, bottom-left, bottom-right). 32-bit flag words, read when
//!   their bits run out, say which (1: V4); 0x3100 adds a bit before each
//!   block (0: the block keeps the frame before); 0x3200 is all V1.
//!
//! The codebooks belong to the strip and live on to the next frame; unless
//! the frame's flags have bit 0, a strip starts from the one above's.

/// A strip's codebooks, entries already as colors (top-left, top-right,
/// bottom-left, bottom-right).
#[derive(Clone)]
struct Books {
    v1: Vec<[u32; 4]>,
    v4: Vec<[u32; 4]>,
}

impl Books {
    fn new() -> Books {
        Books { v1: vec![[0; 4]; 256], v4: vec![[0; 4]; 256] }
    }
}

/// What carries from frame to frame.
#[derive(Clone, Default)]
pub(super) struct State {
    strips: Vec<Books>,
}

/// Strips a frame can have (more is a broken file).
const MAX_STRIPS: usize = 32;

fn u16be(b: &[u8], i: usize) -> Option<usize> {
    b.get(i..i + 2).map(|s| (s[0] as usize) << 8 | s[1] as usize)
}

fn u24be(b: &[u8], i: usize) -> Option<usize> {
    b.get(i..i + 3).map(|s| (s[0] as usize) << 16 | (s[1] as usize) << 8 | s[2] as usize)
}

#[inline]
fn bgr(r: i32, g: i32, b: i32) -> u32 {
    (b.clamp(0, 255) as u32) << 16 | (g.clamp(0, 255) as u32) << 8 | r.clamp(0, 255) as u32
}

/// Decodes one frame into `rgb` (w × h), over the frame before.
/// `palette`: an 8-bit file's, whose four-byte entries are indexes.
pub(super) fn decode(st: &mut State, data: &[u8], w: usize, h: usize, palette: Option<&[u32]>, rgb: &mut [u32]) {
    let (Some(&flags), Some(count)) = (data.first(), u16be(data, 8)) else { return };
    let mut pos = 10;
    let mut y0 = 0;
    for i in 0..count.min(MAX_STRIPS) {
        let (Some(size), Some(top), Some(left), Some(bottom), Some(right)) =
            (u24be(data, pos + 1), u16be(data, pos + 4), u16be(data, pos + 6), u16be(data, pos + 8), u16be(data, pos + 10))
        else {
            return;
        };
        if size < 12 {
            return;
        }
        let end = (pos + size).min(data.len());
        let (top, bottom) = if top == 0 { (y0, y0 + bottom) } else { (top, bottom) };
        let right = if right == 0 { w } else { right };
        y0 = bottom;
        while st.strips.len() <= i {
            st.strips.push(Books::new());
        }
        if i > 0 && flags & 1 == 0 {
            let (above, this) = st.strips.split_at_mut(i);
            this[0].v1.copy_from_slice(&above[i - 1].v1);
            this[0].v4.copy_from_slice(&above[i - 1].v4);
        }
        let books = &mut st.strips[i];
        let mut c = pos + 12;
        while c + 4 <= end {
            let (Some(id), Some(csize)) = (u16be(data, c), u16be(data, c + 2)) else { break };
            if csize < 4 {
                break;
            }
            let body = &data[c + 4..(c + csize).min(end)];
            match id >> 8 {
                0x20..=0x27 => {
                    let book = if id & 0x0200 != 0 { &mut books.v1 } else { &mut books.v4 };
                    load_book(book, body, id & 0x0400 != 0, id & 0x0100 != 0, palette);
                }
                0x30..=0x32 => vectors(books, body, id >> 8, (left, top, right, bottom), w, h, rgb),
                _ => {}
            }
            c += csize;
        }
        pos += size;
    }
}

fn load_book(book: &mut [[u32; 4]], body: &[u8], short: bool, selective: bool, palette: Option<&[u32]>) {
    let n = if short { 4 } else { 6 };
    let entry = |e: &[u8]| -> [u32; 4] {
        if n == 4 {
            match palette {
                Some(pal) => [0, 1, 2, 3].map(|k| pal.get(e[k] as usize).copied().unwrap_or(0)),
                None => [0, 1, 2, 3].map(|k| u32::from(e[k]) * 0x01_01_01),
            }
        } else {
            let (u, v) = (i32::from(e[4] as i8), i32::from(e[5] as i8));
            [0, 1, 2, 3].map(|k| {
                let y = i32::from(e[k]);
                bgr(y + 2 * v, y - u / 2 - v, y + 2 * u)
            })
        }
    };
    let mut p = 0;
    if !selective {
        for slot in book.iter_mut() {
            let Some(e) = body.get(p..p + n) else { return };
            *slot = entry(e);
            p += n;
        }
        return;
    }
    let mut i = 0;
    while i < book.len() {
        let Some(f) = body.get(p..p + 4) else { return };
        let flag = u32::from_be_bytes([f[0], f[1], f[2], f[3]]);
        p += 4;
        for bit in 0..32 {
            if flag & (0x8000_0000 >> bit) != 0 && i < book.len() {
                let Some(e) = body.get(p..p + n) else { return };
                book[i] = entry(e);
                p += n;
            }
            i += 1;
        }
    }
}

/// The flag bits of a vector chunk, a 32-bit word at a time.
struct FlagBits {
    word: u32,
    mask: u32,
}

impl FlagBits {
    fn next(&mut self, body: &[u8], p: &mut usize) -> Option<bool> {
        if self.mask == 0 {
            let f = body.get(*p..*p + 4)?;
            self.word = u32::from_be_bytes([f[0], f[1], f[2], f[3]]);
            self.mask = 0x8000_0000;
            *p += 4;
        }
        let bit = self.word & self.mask != 0;
        self.mask >>= 1;
        Some(bit)
    }
}

fn vectors(books: &Books, body: &[u8], kind: usize, (left, top, right, bottom): (usize, usize, usize, usize), w: usize, h: usize, rgb: &mut [u32]) {
    let mut bits = FlagBits { word: 0, mask: 0 };
    let mut p = 0;
    let (right, bottom) = (right.min(w.next_multiple_of(4)), bottom.min(h.next_multiple_of(4)));
    // (a 2×2 of one color at (x, y), clipped)
    let mut put = |x: usize, y: usize, c: [u32; 4]| {
        for (k, &v) in c.iter().enumerate() {
            let (px, py) = (x + (k & 1), y + (k >> 1));
            if px < w && py < h {
                rgb[py * w + px] = v;
            }
        }
    };
    for y in (top..bottom).step_by(4) {
        for x in (left..right).step_by(4) {
            if kind == 0x31 {
                let Some(coded) = bits.next(body, &mut p) else { return };
                if !coded {
                    continue;
                }
            }
            let v4 = kind != 0x32 && {
                let Some(b) = bits.next(body, &mut p) else { return };
                b
            };
            if v4 {
                let Some(ix) = body.get(p..p + 4) else { return };
                p += 4;
                for (q, &i) in ix.iter().enumerate() {
                    put(x + (q & 1) * 2, y + (q >> 1) * 2, books.v4[i as usize]);
                }
            } else {
                let Some(&i) = body.get(p) else { return };
                p += 1;
                let e = books.v1[i as usize];
                for (q, &c) in e.iter().enumerate() {
                    put(x + (q & 1) * 2, y + (q >> 1) * 2, [c; 4]);
                }
            }
        }
    }
}

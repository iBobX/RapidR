//! Microsoft RLE (BI_RLE8, BI_RLE4 — the Windows bitmap format's run-length
//! encoding, 'mrle' in AVIs): byte pairs from the bottom row up. A count
//! and a value repeat the value (RLE4: its two nibbles in turn); a zero
//! count is an escape — 0 end of line, 1 end of bitmap, 2 a delta (right
//! dx, up dy), 3..255 that many pixels as they are, padded to a 16-bit
//! boundary. What a frame doesn't reach keeps the frame before.

/// Decodes one frame into `idx` (w × h palette indexes, top row first),
/// over what is there.
pub(super) fn decode(data: &[u8], w: usize, h: usize, four: bool, idx: &mut [u8]) {
    let (mut x, mut line) = (0usize, 0usize);
    let mut p = 0;
    let mut put = |x: usize, line: usize, v: u8| {
        if x < w && line < h {
            idx[(h - 1 - line) * w + x] = v;
        }
    };
    while p + 1 < data.len() && line < h {
        let (count, value) = (data[p] as usize, data[p + 1]);
        p += 2;
        if count > 0 {
            for i in 0..count {
                let v = if !four {
                    value
                } else if i % 2 == 0 {
                    value >> 4
                } else {
                    value & 0xF
                };
                put(x.saturating_add(i), line, v);
            }
            x = x.saturating_add(count);
            continue;
        }
        match value {
            0 => {
                x = 0;
                line += 1;
            }
            1 => break,
            2 => {
                let Some(d) = data.get(p..p + 2) else { break };
                x = x.saturating_add(d[0] as usize);
                line += d[1] as usize;
                p += 2;
            }
            n => {
                let n = n as usize;
                let bytes = if four { n.div_ceil(2) } else { n };
                let Some(lit) = data.get(p..p + bytes) else { break };
                for i in 0..n {
                    let v = if !four {
                        lit[i]
                    } else if i % 2 == 0 {
                        lit[i / 2] >> 4
                    } else {
                        lit[i / 2] & 0xF
                    };
                    put(x.saturating_add(i), line, v);
                }
                x = x.saturating_add(n);
                p += bytes + bytes % 2;
            }
        }
    }
}

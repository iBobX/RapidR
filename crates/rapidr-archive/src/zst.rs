//! zstd (read): ruzstd, pure Rust. Its window is held to ruzstd's default
//! 128 MB (a frame asking for more is refused before anything is made).

use crate::read_capped;

/// A zstd file — its frames one after another, as `zstd` writes a long
/// stream — unpacked to at most `cap` bytes.
pub(crate) fn unzstd(bytes: &[u8], cap: u64) -> Result<Vec<u8>, String> {
    let mut rest = bytes;
    let mut out = Vec::new();
    while !rest.is_empty() {
        // (a skippable frame — 0x184D2A5? — or padding of zeros: done)
        if rest.len() < 4 || rest[..4] != [0x28, 0xB5, 0x2F, 0xFD] {
            break;
        }
        let mut d = ruzstd::decoding::StreamingDecoder::new(&mut rest).map_err(|e| e.to_string())?;
        let left = cap - out.len() as u64;
        out.extend(read_capped(&mut d, left)?);
    }
    Ok(out)
}

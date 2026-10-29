//! xz (read): lzma-rust2, pure Rust. Its decoder's memory is held to
//! MaxSize too (a dictionary bigger than the archive may unpack to is
//! refused before it's made), and grows with the data it produces.

use crate::read_capped;

/// An xz file (its streams one after another) unpacked to at most `cap`
/// bytes; its decoder may use at most `max_size` bytes of memory.
pub(crate) fn unxz(bytes: &[u8], cap: u64, max_size: u64) -> Result<Vec<u8>, String> {
    let mem_kb = u32::try_from(max_size / 1024).unwrap_or(u32::MAX).max(64 * 1024);
    let mut r = lzma_rust2::XzReader::new_mem_limit(bytes, true, mem_kb);
    read_capped(&mut r, cap)
}

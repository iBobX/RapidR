//! gzip, zlib and raw Deflate (RFC 1950-1952), through flate2 on zlib-rs
//! (pure Rust): unpacked within a cap, packed at a level.

use std::io::Write;

use flate2::read::{DeflateDecoder, MultiGzDecoder, ZlibDecoder};
use flate2::write::{DeflateEncoder, ZlibEncoder};
use flate2::{Compression, GzBuilder};

use crate::{read_capped, Unpacked};

/// A gzip file (its members one after another, as `cat a.gz b.gz` makes),
/// unpacked to at most `cap` bytes, with the name and time its header
/// keeps.
pub(crate) fn gunzip(bytes: &[u8], cap: u64) -> Result<Unpacked, String> {
    let mut d = MultiGzDecoder::new(bytes);
    let data = read_capped(&mut d, cap)?;
    let header = d.header();
    let name = header.and_then(|h| h.filename()).map(|n| match std::str::from_utf8(n) {
        Ok(s) => s.to_string(),
        // (RFC 1952: the name is Latin-1)
        Err(_) => n.iter().map(|&c| char::from(c)).collect(),
    });
    // (a name with folders in it: only its last part)
    let name = name.map(|n| n.rsplit(['/', '\\']).next().unwrap_or("").to_string());
    let mtime = header.map(|h| h.mtime()).filter(|&t| t != 0).map(i64::from);
    Ok(Unpacked { data, name, mtime })
}

/// zlib (`header`) or raw Deflate, unpacked to at most `cap` bytes.
pub(crate) fn inflate(bytes: &[u8], header: bool, cap: u64) -> Result<Vec<u8>, String> {
    if header {
        read_capped(&mut ZlibDecoder::new(bytes), cap)
    } else {
        read_capped(&mut DeflateDecoder::new(bytes), cap)
    }
}

/// `data` as a gzip file: `name` and `mtime` (seconds since 1970; 0 for
/// none) in its header.
pub(crate) fn gzip(data: &[u8], name: Option<&str>, mtime: i64, level: u32) -> Vec<u8> {
    let mut b = GzBuilder::new().mtime(u32::try_from(mtime).unwrap_or(0));
    if let Some(n) = name {
        // (no folders, no NUL: the header's name is one file's)
        let base: String = n.rsplit('/').next().unwrap_or(n).chars().filter(|&c| c != '\0').collect();
        b = b.filename(base.into_bytes());
    }
    let mut w = b.write(Vec::new(), Compression::new(level));
    // (writing to a Vec can't fail)
    let _ = w.write_all(data);
    w.finish().unwrap_or_default()
}

/// zlib (`header`) or raw Deflate at `level`.
pub(crate) fn deflate(data: &[u8], header: bool, level: u32) -> Vec<u8> {
    let c = Compression::new(level);
    if header {
        let mut w = ZlibEncoder::new(Vec::new(), c);
        let _ = w.write_all(data);
        w.finish().unwrap_or_default()
    } else {
        let mut w = DeflateEncoder::new(Vec::new(), c);
        let _ = w.write_all(data);
        w.finish().unwrap_or_default()
    }
}

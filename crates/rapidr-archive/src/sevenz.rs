//! 7z (read): sevenz-rust2 (Apache-2.0), pure Rust — LZMA, LZMA2, Deflate,
//! BCJ, Delta, stored. Its header's counts are bounded by the header's own
//! length and its dictionaries grow with the data, so a tiny file can't
//! claim a huge allocation; RapidR adds MaxSize and MaxRatio on the sizes
//! the archive declares (refused at open) and on the bytes really unpacked.
//! Encrypted archives are refused (no AES here).

use std::io::{self, Cursor, Read};

use sevenz_rust2::{Archive, ArchiveEntry, BlockDecoder, EncoderMethod, Error, Password};

use crate::limit::{self, OVER, RATIO_FLOOR};
use crate::{read_capped, Entry, Kind, Limits, Modified};

/// Seconds between 1601 (Windows' FILETIME) and 1970.
const FILETIME_TO_UNIX: i64 = 11_644_473_600;

fn words(e: Error) -> String {
    match e {
        Error::PasswordRequired | Error::MaybeBadPassword(_) => "the 7z archive is encrypted: RArchive doesn't open password-protected archives".into(),
        Error::UnsupportedCompressionMethod(m) if m.contains("AES") => "the 7z archive is encrypted: RArchive doesn't open password-protected archives".into(),
        Error::UnsupportedCompressionMethod(m) => format!("the 7z archive is packed in a way RapidR can't unpack ({m}; LZMA, LZMA2 and Deflate are)"),
        Error::BadSignature(_) => "not a 7z archive".into(),
        e => {
            let s = e.to_string();
            if s.to_ascii_lowercase().contains("password") || s.contains("AES") {
                "the 7z archive is encrypted: RArchive doesn't open password-protected archives".into()
            } else {
                limit::damaged("the 7z archive", &s)
            }
        }
    }
}

fn read_archive(bytes: &[u8]) -> Result<Archive, String> {
    Archive::read(&mut Cursor::new(bytes), &Password::empty()).map_err(words)
}

/// A link: a Unix mode kept in the high half (the 0x8000 flag) that
/// says so, or a Windows reparse point.
fn is_link(f: &ArchiveEntry) -> bool {
    if !f.has_windows_attributes {
        return false;
    }
    let a = f.windows_attributes;
    a & 0x400 != 0 || (a & 0x8000 != 0 && (a >> 16) & 0o170000 == 0o120000)
}

/// The entries of the 7z in `bytes`; refused at open if what it says it
/// unpacks to is past MaxSize or MaxRatio.
pub(crate) fn list(bytes: &[u8], limits: &Limits) -> Result<Vec<Entry>, String> {
    let a = read_archive(bytes)?;
    let encrypted_block: Vec<bool> = a.blocks.iter().map(|b| b.coders.iter().any(|c| c.encoder_method_id() == EncoderMethod::ID_AES256_SHA256)).collect();
    let mut out = Vec::with_capacity(a.files.len());
    let mut total = 0u64;
    for (i, f) in a.files.iter().enumerate() {
        let block = a.stream_map.file_block_index.get(i).copied().flatten();
        let encrypted = block.is_some_and(|b| encrypted_block.get(b) == Some(&true));
        let kind = if f.is_directory {
            Kind::Folder
        } else if is_link(f) {
            Kind::Link
        } else if f.is_anti_item {
            Kind::Special
        } else {
            Kind::File
        };
        total = total.saturating_add(f.size);
        let modified = f.has_last_modified_date.then(|| Modified::Unix((u64::from(f.last_modified_date) / 10_000_000) as i64 - FILETIME_TO_UNIX));
        let compressed = if f.compressed_size > 0 { f.compressed_size } else { f.size };
        out.push(Entry { name: f.name.clone(), size: f.size, compressed_size: compressed, kind, encrypted, modified });
    }
    if total > limits.max_size {
        return Err(limit::too_big("the 7z archive", limits.max_size));
    }
    if limits.max_ratio > 0 && total > RATIO_FLOOR && total > (bytes.len() as u64).max(1).saturating_mul(limits.max_ratio) {
        return Err(format!("the 7z archive unpacks to more than MaxRatio ({}) times its size: not opened — a zip bomb, or set MaxRatio higher", limits.max_ratio));
    }
    Ok(out)
}

/// `entry`'s index in `archive.files` (it's one of them).
fn index_of(archive: &Archive, entry: &ArchiveEntry) -> Option<usize> {
    let base = archive.files.as_ptr() as usize;
    let at = (entry as *const ArchiveEntry as usize).checked_sub(base)? / std::mem::size_of::<ArchiveEntry>().max(1);
    (at < archive.files.len()).then_some(at)
}

/// Reads one entry's data (`r`) within what's left of `cap`; `Err` with
/// the reason, or [`OVER`].
fn take(r: &mut dyn Read, cap: u64) -> Result<Vec<u8>, String> {
    read_capped(r, cap)
}

/// Entry `i`'s bytes, unpacked within the limits: its block decoded from
/// the start (a solid block's earlier files read and dropped).
pub(crate) fn extract(bytes: &[u8], i: usize, limits: &Limits) -> Result<Vec<u8>, String> {
    let archive = read_archive(bytes)?;
    let Some(f) = archive.files.get(i) else { return Err(format!("no entry {i}")) };
    let what = format!("\"{}\"", f.name);
    let Some(block) = archive.stream_map.file_block_index.get(i).copied().flatten() else {
        // (an empty file: no data)
        return Ok(Vec::new());
    };
    let cap = limit::cap(if f.compressed_size > 0 { f.compressed_size } else { bytes.len() as u64 }, limits);
    let password = Password::empty();
    let mut src = Cursor::new(bytes);
    let mut got: Option<Result<Vec<u8>, String>> = None;
    let r = BlockDecoder::new(1, block, &archive, &password, &mut src).for_each_entries(&mut |entry: &ArchiveEntry, r: &mut dyn Read| {
        if index_of(&archive, entry) == Some(i) {
            got = Some(take(r, cap));
            return Ok(false);
        }
        // (an earlier file of a solid block: read and dropped — its size was
        // checked against MaxSize at open)
        io::copy(&mut r.take(limits.max_size), &mut io::sink())?;
        Ok(true)
    });
    match (got, r) {
        (Some(Ok(data)), _) => Ok(data),
        (Some(Err(e)), _) if e == OVER => Err(limit::explain(e, &what, f.compressed_size.max(1), limits)),
        (Some(Err(e)), _) => Err(limit::damaged(&what, &e)),
        (None, Err(e)) => Err(words(e)),
        (None, Ok(_)) => Err(limit::damaged(&what, "its data wasn't found")),
    }
}

/// Every file's bytes (folders and links left out), unpacked within the
/// limits: at most MaxSize in all.
pub(crate) fn extract_all(bytes: &[u8], limits: &Limits) -> Result<Vec<(usize, Vec<u8>)>, String> {
    let archive = read_archive(bytes)?;
    let password = Password::empty();
    let mut out: Vec<(usize, Vec<u8>)> = Vec::new();
    let mut total = 0u64;
    let mut failed: Option<String> = None;
    for block in 0..archive.blocks.len() {
        let mut src = Cursor::new(bytes);
        let r = BlockDecoder::new(1, block, &archive, &password, &mut src).for_each_entries(&mut |entry: &ArchiveEntry, r: &mut dyn Read| {
            let Some(i) = index_of(&archive, entry) else { return Ok(true) };
            if entry.is_directory || is_link(entry) || entry.is_anti_item {
                io::copy(&mut r.take(limits.max_size), &mut io::sink())?;
                return Ok(true);
            }
            match take(r, limits.max_size - total) {
                Ok(data) => {
                    total += data.len() as u64;
                    out.push((i, data));
                    Ok(true)
                }
                Err(e) if e == OVER => {
                    failed = Some(limit::too_big("everything together", limits.max_size));
                    Ok(false)
                }
                Err(e) => {
                    failed = Some(limit::damaged(&format!("\"{}\"", entry.name), &e));
                    Ok(false)
                }
            }
        });
        if let Some(e) = failed.take() {
            return Err(e);
        }
        r.map_err(words)?;
    }
    // (files with no data: empty)
    for (i, f) in archive.files.iter().enumerate() {
        if archive.stream_map.file_block_index.get(i).copied().flatten().is_none() && !f.is_directory && !is_link(f) && !f.is_anti_item {
            out.push((i, Vec::new()));
        }
    }
    out.sort_by_key(|(i, _)| *i);
    Ok(out)
}

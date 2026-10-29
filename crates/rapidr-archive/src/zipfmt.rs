//! zip, through the `zip` crate (MIT): stored and deflated entries, zip64,
//! UTF-8 names and old tools' code page 437 names (the crate's); written
//! with UTF-8 names, deflated at the level asked (0: stored).

use std::io::{Cursor, Write};

use zip::result::ZipError;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, ZipArchive, ZipWriter};

use crate::limit::{self, OVER};
use crate::{read_capped, Entry, Kind, Limits, Modified, NewEntry, Stamp};

/// The zip crate's error, in RapidR's words.
fn words(e: ZipError) -> String {
    match e {
        ZipError::Io(e) => e.to_string(),
        ZipError::InvalidArchive(why) => format!("not a valid zip ({why})"),
        ZipError::UnsupportedArchive(why) => format!("a zip RapidR can't read ({why})"),
        ZipError::FileNotFound => "no such entry".into(),
        e => e.to_string(),
    }
}

/// An opened zip: its list of files read once.
pub(crate) type Zip = ZipArchive<Cursor<Vec<u8>>>;

/// The zip in `bytes`, opened, and its entries (nothing unpacked).
pub(crate) fn list(bytes: Vec<u8>) -> Result<(Zip, Vec<Entry>), String> {
    let mut z = ZipArchive::new(Cursor::new(bytes)).map_err(|e| match e {
        ZipError::InvalidArchive(_) => "not a zip, or a damaged one (its list of files can't be found)".to_string(),
        e => words(e),
    })?;
    let mut out = Vec::with_capacity(z.len().min(1 << 16));
    for i in 0..z.len() {
        let f = z.by_index_raw(i).map_err(words)?;
        let name = f.name().map(|n| n.into_owned()).unwrap_or_else(|_| String::from_utf8_lossy(f.name_raw()).into_owned());
        let kind = if f.is_symlink() {
            Kind::Link
        } else if f.is_dir() {
            Kind::Folder
        } else {
            Kind::File
        };
        let modified = f.last_modified().map(|d| {
            Modified::Local(Stamp { year: i64::from(d.year()), month: u32::from(d.month()), day: u32::from(d.day()), hour: u32::from(d.hour()), minute: u32::from(d.minute()), second: u32::from(d.second()) })
        });
        out.push(Entry { name, size: f.size(), compressed_size: f.compressed_size(), kind, encrypted: f.encrypted(), modified });
    }
    Ok((z, out))
}

/// Entry `i`'s bytes (`name` for messages), unpacked within the limits —
/// the bytes really unpacked counted, not the size the zip claims; its CRC
/// checked.
pub(crate) fn extract(z: &mut Zip, i: usize, name: &str, limits: &Limits) -> Result<Vec<u8>, String> {
    let what = format!("\"{name}\"");
    let mut f = z.by_index(i).map_err(|e| match e {
        ZipError::UnsupportedArchive(why) if why.contains("ompression") => format!("{what} is packed in a way RapidR can't unpack (only stored and Deflate): {why}"),
        e => limit::damaged(&what, &words(e)),
    })?;
    let packed = f.compressed_size();
    let cap = limit::cap(packed, limits);
    match read_capped(&mut f, cap) {
        Ok(data) => Ok(data),
        Err(e) if e == OVER => Err(limit::explain(e, &what, packed, limits)),
        Err(e) => Err(limit::damaged(&what, &e)),
    }
}

/// A zip of `entries` at `level` (0: stored); `offset`: the local time
/// zone's seconds east (zip keeps local times).
pub(crate) fn write(entries: &[NewEntry], level: u32, offset: i64) -> Result<Vec<u8>, String> {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    for e in entries {
        let s = Stamp::from_unix(e.mtime, offset);
        let when = DateTime::from_date_and_time(u16::try_from(s.year).unwrap_or(1980), s.month as u8, s.day as u8, s.hour as u8, s.minute as u8, s.second as u8).unwrap_or_default();
        let base = SimpleFileOptions::default().last_modified_time(when);
        if e.folder {
            w.add_directory(e.name.as_str(), base.unix_permissions(0o755)).map_err(words)?;
            continue;
        }
        let opts = if level == 0 { base.compression_method(CompressionMethod::Stored) } else { base.compression_method(CompressionMethod::Deflated).compression_level(Some(i64::from(level))) };
        let opts = opts.unix_permissions(0o644).large_file(e.data.len() as u64 >= u64::from(u32::MAX));
        w.start_file(e.name.as_str(), opts).map_err(words)?;
        w.write_all(&e.data).map_err(|e| e.to_string())?;
    }
    Ok(w.finish().map_err(words)?.into_inner())
}

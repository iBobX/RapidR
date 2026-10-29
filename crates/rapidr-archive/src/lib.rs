//! RArchive's engine: archives and compressed files read and written in
//! memory, for every runtime (the desktop, native builds and the web — no
//! file system here: the component reads and writes the bytes,
//! `rapidr_value::objects::archive`).
//!
//! What it opens (the format found from the first bytes, the file's
//! extension only a hint): zip, tar, tar.gz, a single .gz file, and with
//! their features 7z, xz / tar.xz and zstd / tar.zst. What it makes: zip,
//! tar, tar.gz and .gz. RAR is recognised and refused in plain words (its
//! format isn't open); bzip2 is recognised and not read yet.
//!
//! An archive is untrusted input:
//! - nothing unpacks to more than [`Limits::max_size`] bytes (one entry, a
//!   whole `.tar.gz` read at open, everything [`Archive::extract_all`]
//!   unpacks), and no entry past 1 MB unpacks to more than
//!   [`Limits::max_ratio`] times its packed size: a zip bomb is an error,
//!   never a full memory — the count is of the bytes really unpacked, never
//!   of the sizes the archive claims;
//! - names that would leave the folder they're extracted to (absolute,
//!   `..`, a drive, a device) are refused by [`safe_path`], and links are
//!   never extracted (the component's ExtractToFile / ExtractAll);
//! - encrypted entries are refused (no password support);
//! - a malformed archive is an error, never a panic
//!   (`tests/robustness.rs`).

use std::io::Read;

mod limit;
mod names;
mod tar;

#[cfg(feature = "gzip")]
mod deflate;
#[cfg(feature = "7z")]
mod sevenz;
#[cfg(feature = "xz")]
mod xz;
#[cfg(feature = "zip")]
mod zipfmt;
#[cfg(feature = "zstd")]
mod zst;

pub use limit::RATIO_FLOOR;
pub use names::safe_path;

/// An archive's or a compressed file's format.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Format {
    Zip,
    Tar,
    TarGz,
    /// One file compressed with gzip (`.gz`).
    Gzip,
    SevenZ,
    TarXz,
    Xz,
    TarZst,
    Zstd,
    TarBz2,
    Bzip2,
    /// Recognised to say why it can't be opened.
    Rar,
}

impl Format {
    /// Its name, as programs read it (`Archive.Format`).
    pub fn name(self) -> &'static str {
        match self {
            Format::Zip => "zip",
            Format::Tar => "tar",
            Format::TarGz => "tar.gz",
            Format::Gzip => "gzip",
            Format::SevenZ => "7z",
            Format::TarXz => "tar.xz",
            Format::Xz => "xz",
            Format::TarZst => "tar.zst",
            Format::Zstd => "zstd",
            Format::TarBz2 => "tar.bz2",
            Format::Bzip2 => "bzip2",
            Format::Rar => "rar",
        }
    }

    /// A format by a name or an extension a program gives (`"zip"`,
    /// `".tgz"`, `"tar.gz"`, `"GZ"` …); `None` for one RapidR doesn't know.
    pub fn from_name(s: &str) -> Option<Format> {
        let s = s.trim().trim_start_matches('.').to_ascii_lowercase();
        Some(match s.as_str() {
            "zip" | "jar" => Format::Zip,
            "tar" => Format::Tar,
            "tar.gz" | "tgz" | "targz" => Format::TarGz,
            "gz" | "gzip" => Format::Gzip,
            "7z" | "7zip" => Format::SevenZ,
            "tar.xz" | "txz" => Format::TarXz,
            "xz" => Format::Xz,
            "tar.zst" | "tar.zstd" | "tzst" => Format::TarZst,
            "zst" | "zstd" => Format::Zstd,
            "tar.bz2" | "tbz2" | "tbz" => Format::TarBz2,
            "bz2" | "bzip2" => Format::Bzip2,
            "rar" => Format::Rar,
            _ => return None,
        })
    }

    /// The format a file's name says (`games.tar.gz` → tar.gz), if any.
    pub fn from_file_name(path: &str) -> Option<Format> {
        let name = path.rsplit(['/', '\\']).next().unwrap_or(path).to_ascii_lowercase();
        for double in ["tar.gz", "tar.xz", "tar.zst", "tar.zstd", "tar.bz2"] {
            if name.ends_with(&format!(".{double}")) {
                return Format::from_name(double);
            }
        }
        let ext = name.rsplit_once('.')?.1;
        Format::from_name(ext)
    }

    /// Whether RapidR can make archives of this format.
    pub fn writable(self) -> bool {
        matches!(self, Format::Tar) || (cfg!(feature = "gzip") && matches!(self, Format::TarGz | Format::Gzip)) || (cfg!(feature = "zip") && self == Format::Zip)
    }

    /// The compressed stream around a tar, or the single file's codec.
    fn codec(self) -> Option<Codec> {
        match self {
            Format::TarGz | Format::Gzip => Some(Codec::Gzip),
            Format::TarXz | Format::Xz => Some(Codec::Xz),
            Format::TarZst | Format::Zstd => Some(Codec::Zstd),
            _ => None,
        }
    }

    /// The tar inside a single-stream format (gzip → tar.gz).
    fn with_tar(self) -> Format {
        match self {
            Format::Gzip => Format::TarGz,
            Format::Xz => Format::TarXz,
            Format::Zstd => Format::TarZst,
            Format::Bzip2 => Format::TarBz2,
            f => f,
        }
    }
}

/// The format of `bytes` from their first bytes (the "magic"), `None` when
/// they're no archive or compressed file RapidR knows (a `.z80` snapshot, a
/// text). A gzip's, xz's or zstd's inner tar is found when it's opened.
pub fn detect(bytes: &[u8]) -> Option<Format> {
    let starts = |m: &[u8]| bytes.starts_with(m);
    if starts(b"PK\x03\x04") || starts(b"PK\x05\x06") || starts(b"PK\x07\x08") {
        Some(Format::Zip)
    } else if starts(&[0x1F, 0x8B]) {
        Some(Format::Gzip)
    } else if starts(&[b'7', b'z', 0xBC, 0xAF, 0x27, 0x1C]) {
        Some(Format::SevenZ)
    } else if starts(&[0xFD, b'7', b'z', b'X', b'Z', 0x00]) {
        Some(Format::Xz)
    } else if starts(&[0x28, 0xB5, 0x2F, 0xFD]) {
        Some(Format::Zstd)
    } else if starts(b"BZh") && bytes.get(3).is_some_and(|c| (b'1'..=b'9').contains(c)) {
        Some(Format::Bzip2)
    } else if starts(b"Rar!\x1A\x07") {
        Some(Format::Rar)
    } else if tar::looks_like_tar(bytes) {
        Some(Format::Tar)
    } else {
        None
    }
}

/// How much an archive may unpack to: its guard against zip bombs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// The most bytes one entry, all of ExtractAll, or a whole compressed
    /// tar read at open may unpack to.
    pub max_size: u64,
    /// The most an entry past [`RATIO_FLOOR`] bytes may grow by
    /// (unpacked ÷ packed); 0: no ratio check.
    pub max_ratio: u64,
}

impl Limits {
    /// The desktop's default MaxSize: 1 GB.
    pub const DESKTOP_MAX_SIZE: u64 = 1 << 30;
    /// The browser's default MaxSize: 256 MB (a page's memory is smaller).
    pub const WEB_MAX_SIZE: u64 = 256 << 20;
    /// The default MaxRatio: real files rarely pack better than 100 to 1 past
    /// a megabyte; a bomb packs thousands to one.
    pub const DEFAULT_MAX_RATIO: u64 = 1000;

    /// This build's defaults (the web's smaller MaxSize).
    pub fn default_here() -> Limits {
        let max_size = if cfg!(target_arch = "wasm32") { Self::WEB_MAX_SIZE } else { Self::DESKTOP_MAX_SIZE };
        Limits { max_size, max_ratio: Self::DEFAULT_MAX_RATIO }
    }
}

/// What an entry is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    File,
    Folder,
    /// A symbolic or hard link: listed, never extracted (it could point
    /// outside the folder).
    Link,
    /// A device, a pipe, a sparse file …: listed, never extracted.
    Special,
}

/// A date and time as the archive keeps it (local time for zip; tar's, 7z's
/// and gzip's are UTC, turned local by the caller's offset).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Stamp {
    pub year: i64,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

impl Stamp {
    /// From seconds since 1970 (UTC), `offset` seconds east added.
    pub fn from_unix(secs: i64, offset: i64) -> Stamp {
        let t = secs.saturating_add(offset);
        let days = t.div_euclid(86_400);
        let rem = t.rem_euclid(86_400);
        // (Howard Hinnant's days-to-civil, public domain)
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z.rem_euclid(146_097);
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        let year = yoe + era * 400 + i64::from(month <= 2);
        Stamp { year, month, day, hour: (rem / 3600) as u32, minute: (rem % 3600 / 60) as u32, second: (rem % 60) as u32 }
    }

    /// Seconds since 1970 of this time read as UTC, less `offset` (its
    /// time zone's seconds east): the inverse of [`Stamp::from_unix`].
    pub fn to_unix(self, offset: i64) -> i64 {
        let (y, m) = if self.month <= 2 { (self.year - 1, i64::from(self.month) + 9) } else { (self.year, i64::from(self.month) - 3) };
        let era = y.div_euclid(400);
        let yoe = y.rem_euclid(400);
        let doy = (153 * m + 2) / 5 + i64::from(self.day) - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        let days = era * 146_097 + doe - 719_468;
        days * 86_400 + i64::from(self.hour) * 3600 + i64::from(self.minute) * 60 + i64::from(self.second) - offset
    }
}

impl std::fmt::Display for Stamp {
    /// `2026-10-29 14:05:00`: sorts as text, as RapidR's dates do.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04}-{:02}-{:02} {:02}:{:02}:{:02}", self.year, self.month, self.day, self.hour, self.minute, self.second)
    }
}

/// When an entry was modified, as its format keeps it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Modified {
    /// Seconds since 1970, UTC (tar, 7z, gzip).
    Unix(i64),
    /// A local time with no zone (zip's DOS time).
    Local(Stamp),
}

impl Modified {
    /// As a local time: a UTC one moved by `offset` (seconds east).
    pub fn local(self, offset: i64) -> Stamp {
        match self {
            Modified::Unix(s) => Stamp::from_unix(s, offset),
            Modified::Local(s) => s,
        }
    }
}

/// One file, folder or link in an archive.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// Its name in the archive, `/` between folders (a folder's ends in
    /// `/` only if the archive wrote it so).
    pub name: String,
    /// Its size unpacked, as the archive says.
    pub size: u64,
    /// Its size packed (a tar's: its size; a compressed tar's whole stream
    /// is shared, so each entry's is its size too).
    pub compressed_size: u64,
    pub kind: Kind,
    pub encrypted: bool,
    pub modified: Option<Modified>,
}

/// An opened archive: its entries, and what's needed to unpack them.
pub struct Archive {
    format: Format,
    entries: Vec<Entry>,
    body: Body,
}

enum Body {
    /// The zip, its list of files read: each entry unpacked when asked.
    #[cfg(feature = "zip")]
    Zip(std::cell::RefCell<zipfmt::Zip>),
    /// A tar's bytes (unpacked already, if it was compressed) and where
    /// each entry's data is in them.
    Tar { data: Vec<u8>, spans: Vec<(usize, usize)> },
    /// A single compressed file's bytes, unpacked.
    Single(Vec<u8>),
    /// The 7z's bytes.
    #[cfg(feature = "7z")]
    SevenZ(Vec<u8>),
}

impl std::fmt::Debug for Archive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Archive").field("format", &self.format).field("entries", &self.entries).finish_non_exhaustive()
    }
}

impl std::fmt::Debug for Writer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Writer").field("format", &self.format).field("level", &self.level).field("count", &self.entries.len()).finish_non_exhaustive()
    }
}

/// "RAR files can't be opened: …" — the same words wherever one is met.
pub const RAR_REFUSED: &str = "RAR files can't be opened: the format isn't open. Ask for a .zip or .7z.";

fn not_in_build(f: Format) -> String {
    format!("this program was built without {} support", f.name())
}

impl Archive {
    /// Opens `bytes` (a whole archive). `file_name` (may be "") names the
    /// file: its extension is a hint where the first bytes don't say
    /// (a zip with something before it), and a `.gz` file's name gives its
    /// one entry's when the gzip doesn't.
    pub fn open(bytes: Vec<u8>, file_name: &str, limits: &Limits) -> Result<Archive, String> {
        if bytes.is_empty() {
            return Err("the file is empty: not an archive".into());
        }
        let hint = Format::from_file_name(file_name);
        let format = match (detect(&bytes), hint) {
            (Some(f), _) => f,
            // (a self-extracting or prefixed zip: the extension says)
            (None, Some(Format::Zip)) => Format::Zip,
            (None, _) => return Err("not an archive RapidR knows (zip, 7z, tar, tar.gz, gz, xz, zst)".into()),
        };
        if format == Format::Rar {
            return Err(RAR_REFUSED.into());
        }
        if matches!(format, Format::Bzip2 | Format::TarBz2) {
            return Err("bzip2 files can't be opened yet: ask for a .zip, .7z or .tar.gz".into());
        }
        if bytes.len() as u64 > limits.max_size {
            return Err(format!("the archive is larger than MaxSize ({} bytes)", limits.max_size));
        }
        match format {
            Format::Zip => Self::open_zip(bytes),
            Format::Tar => Self::open_tar(Format::Tar, bytes),
            Format::SevenZ => Self::open_7z(bytes, limits),
            f => Self::open_stream(f, bytes, file_name, limits),
        }
    }

    #[cfg(feature = "zip")]
    fn open_zip(bytes: Vec<u8>) -> Result<Archive, String> {
        let (zip, entries) = zipfmt::list(bytes)?;
        Ok(Archive { format: Format::Zip, entries, body: Body::Zip(std::cell::RefCell::new(zip)) })
    }

    #[cfg(not(feature = "zip"))]
    fn open_zip(_: Vec<u8>) -> Result<Archive, String> {
        Err(not_in_build(Format::Zip))
    }

    #[cfg(feature = "7z")]
    fn open_7z(bytes: Vec<u8>, limits: &Limits) -> Result<Archive, String> {
        let entries = sevenz::list(&bytes, limits)?;
        Ok(Archive { format: Format::SevenZ, entries, body: Body::SevenZ(bytes) })
    }

    #[cfg(not(feature = "7z"))]
    fn open_7z(_: Vec<u8>, _: &Limits) -> Result<Archive, String> {
        Err(not_in_build(Format::SevenZ))
    }

    fn open_tar(format: Format, data: Vec<u8>) -> Result<Archive, String> {
        let (entries, spans) = tar::read(&data)?.into_iter().unzip();
        Ok(Archive { format, entries, body: Body::Tar { data, spans } })
    }

    /// A gzip, xz or zstd file: unpacked whole (within the limits); a tar
    /// inside is the archive, else it's one file.
    fn open_stream(format: Format, bytes: Vec<u8>, file_name: &str, limits: &Limits) -> Result<Archive, String> {
        let codec = format.codec().ok_or_else(|| not_in_build(format))?;
        let what = if file_name.is_empty() { format!("the {} data", format.name()) } else { format!("\"{}\"", base_name(file_name)) };
        let unpacked = codec.unpack(&bytes, &what, limits)?;
        if tar::looks_like_tar(&unpacked.data) {
            return Self::open_tar(format.with_tar(), unpacked.data);
        }
        // (one file: its name the gzip's own, else the file's less its
        // extension, else "data")
        let name = unpacked.name.filter(|n| !n.is_empty()).unwrap_or_else(|| {
            let base = base_name(file_name);
            match base.rsplit_once('.') {
                Some((stem, ext)) if !stem.is_empty() && Format::from_name(ext).is_some() => stem.to_string(),
                _ if !base.is_empty() => format!("{base}.out"),
                _ => "data".to_string(),
            }
        });
        let size = unpacked.data.len() as u64;
        let entry = Entry { name, size, compressed_size: bytes.len() as u64, kind: Kind::File, encrypted: false, modified: unpacked.mtime.map(Modified::Unix) };
        Ok(Archive { format, entries: vec![entry], body: Body::Single(unpacked.data) })
    }

    /// Its format (tar.gz for a gzip with a tar inside).
    pub fn format(&self) -> Format {
        self.format
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// The index of the entry called `name` (`\` and `/` alike; a folder
    /// found with or without its last `/`), `None` if there's none.
    pub fn find(&self, name: &str, case_sensitive: bool) -> Option<usize> {
        let want = name.replace('\\', "/");
        let want = want.trim_start_matches("./").trim_end_matches('/');
        let same = |a: &str| if case_sensitive { a == want } else { a.eq_ignore_ascii_case(want) || a.to_lowercase() == want.to_lowercase() };
        self.entries.iter().position(|e| same(e.name.replace('\\', "/").trim_start_matches("./").trim_end_matches('/')))
    }

    /// Entry `i`'s bytes, unpacked within the limits. A folder's are none;
    /// an encrypted entry, a link or a device is an error.
    pub fn extract(&self, i: usize, limits: &Limits) -> Result<Vec<u8>, String> {
        let e = self.entries.get(i).ok_or_else(|| format!("no entry {i} (the archive has {})", self.entries.len()))?;
        refuse_kind(e)?;
        if e.kind == Kind::Folder {
            return Ok(Vec::new());
        }
        match &self.body {
            #[cfg(feature = "zip")]
            Body::Zip(zip) => zipfmt::extract(&mut zip.borrow_mut(), i, &e.name, limits),
            Body::Tar { data, spans } => {
                let (start, len) = spans[i];
                if len as u64 > limits.max_size {
                    return Err(limit::too_big(&e.name, limits.max_size));
                }
                Ok(data[start..start + len].to_vec())
            }
            Body::Single(data) => Ok(data.clone()),
            #[cfg(feature = "7z")]
            Body::SevenZ(bytes) => sevenz::extract(bytes, i, limits),
        }
    }

    /// Every file's bytes (folders, links and devices left out: the
    /// component makes the folders), unpacked within the limits — in all,
    /// at most MaxSize. An encrypted file stops it: nothing is returned.
    pub fn extract_all(&self, limits: &Limits) -> Result<Vec<(usize, Vec<u8>)>, String> {
        #[cfg(feature = "7z")]
        if let Body::SevenZ(bytes) = &self.body {
            if let Some(e) = self.entries.iter().find(|e| e.encrypted) {
                refuse_kind(e)?;
            }
            return sevenz::extract_all(bytes, limits);
        }
        let mut out = Vec::new();
        let mut total = 0u64;
        for (i, e) in self.entries.iter().enumerate() {
            if e.kind != Kind::File {
                continue;
            }
            let left = Limits { max_size: limits.max_size - total, ..*limits };
            let data = self.extract(i, &left).map_err(|err| if total > 0 && err.contains("MaxSize") { format!("everything together unpacks to more than MaxSize ({} bytes)", limits.max_size) } else { err })?;
            total += data.len() as u64;
            out.push((i, data));
        }
        Ok(out)
    }
}

/// Why entry `e` can't be unpacked (encrypted, a link, a device), if so.
fn refuse_kind(e: &Entry) -> Result<(), String> {
    if e.encrypted {
        return Err(format!("\"{}\" is encrypted: RArchive doesn't open password-protected entries", e.name));
    }
    match e.kind {
        Kind::Link => Err(format!("\"{}\" is a link: links aren't extracted (one could point outside the folder)", e.name)),
        Kind::Special => Err(format!("\"{}\" is a device, a pipe or a sparse file: not extracted", e.name)),
        _ => Ok(()),
    }
}

/// A path's last part.
fn base_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// A single compressed stream's codec (`COMPRESS$` / `DECOMPRESS$`, and the
/// stream around a tar).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Codec {
    /// RFC 1952: a header (a name, a time), Deflate, a CRC.
    Gzip,
    /// RFC 1950: a 2-byte header, Deflate, an Adler-32.
    Zlib,
    /// RFC 1951: Deflate alone (no header: never detected, only named).
    Deflate,
    Xz,
    Zstd,
}

impl Codec {
    pub fn from_name(s: &str) -> Option<Codec> {
        Some(match s.trim().trim_start_matches('.').to_ascii_lowercase().as_str() {
            "gzip" | "gz" => Codec::Gzip,
            "zlib" => Codec::Zlib,
            "deflate" | "raw" => Codec::Deflate,
            "xz" => Codec::Xz,
            "zstd" | "zst" => Codec::Zstd,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Codec::Gzip => "gzip",
            Codec::Zlib => "zlib",
            Codec::Deflate => "deflate",
            Codec::Xz => "xz",
            Codec::Zstd => "zstd",
        }
    }

    /// The codec `bytes` start with (gzip, zlib, xz, zstd); Deflate has no
    /// header, so it's never found.
    pub fn detect(bytes: &[u8]) -> Option<Codec> {
        match detect(bytes) {
            Some(Format::Gzip) => Some(Codec::Gzip),
            Some(Format::Xz) => Some(Codec::Xz),
            Some(Format::Zstd) => Some(Codec::Zstd),
            // (zlib: CM 8, a window ≤ 32 KB, the header a multiple of 31)
            _ if bytes.len() >= 2 && bytes[0] & 0x0F == 8 && bytes[0] >> 4 <= 7 && (u16::from(bytes[0]) << 8 | u16::from(bytes[1])) % 31 == 0 => Some(Codec::Zlib),
            _ => None,
        }
    }

    fn unpack(self, bytes: &[u8], what: &str, limits: &Limits) -> Result<Unpacked, String> {
        let cap = limit::cap(bytes.len() as u64, limits);
        let fail = |e: String| limit::explain(e, what, bytes.len() as u64, limits);
        match self {
            #[cfg(feature = "gzip")]
            Codec::Gzip => deflate::gunzip(bytes, cap).map_err(fail),
            #[cfg(feature = "gzip")]
            Codec::Zlib => deflate::inflate(bytes, true, cap).map(Unpacked::data).map_err(fail),
            #[cfg(feature = "gzip")]
            Codec::Deflate => deflate::inflate(bytes, false, cap).map(Unpacked::data).map_err(fail),
            #[cfg(feature = "xz")]
            Codec::Xz => xz::unxz(bytes, cap, limits.max_size).map(Unpacked::data).map_err(fail),
            #[cfg(feature = "zstd")]
            Codec::Zstd => zst::unzstd(bytes, cap).map(Unpacked::data).map_err(fail),
            #[allow(unreachable_patterns)]
            _ => Err(format!("this program was built without {} support", self.name())),
        }
    }
}

/// What a stream unpacked to, and what its header said (gzip's).
struct Unpacked {
    data: Vec<u8>,
    name: Option<String>,
    mtime: Option<i64>,
}

impl Unpacked {
    fn data(data: Vec<u8>) -> Unpacked {
        Unpacked { data, name: None, mtime: None }
    }
}

/// `DECOMPRESS$`: `bytes` unpacked by `codec` (found from the first bytes
/// when `None`: gzip, zlib, xz, zstd), within the limits.
pub fn decompress(bytes: &[u8], codec: Option<Codec>, limits: &Limits) -> Result<Vec<u8>, String> {
    let codec = match codec.or_else(|| Codec::detect(bytes)) {
        Some(c) => c,
        None if bytes.is_empty() => return Ok(Vec::new()),
        None => return Err("not compressed data RapidR knows (gzip, zlib, xz, zstd; name \"deflate\" for raw Deflate)".into()),
    };
    codec.unpack(bytes, &format!("the {} data", codec.name()), limits).map(|u| u.data)
}

/// `COMPRESS$`: `bytes` packed by `codec` at `level` (0 store … 9 best).
/// gzip, zlib and deflate; xz and zstd are read only.
pub fn compress(bytes: &[u8], codec: Codec, level: u32) -> Result<Vec<u8>, String> {
    let level = level.min(9);
    match codec {
        #[cfg(feature = "gzip")]
        Codec::Gzip => Ok(deflate::gzip(bytes, None, 0, level)),
        #[cfg(feature = "gzip")]
        Codec::Zlib => Ok(deflate::deflate(bytes, true, level)),
        #[cfg(feature = "gzip")]
        Codec::Deflate => Ok(deflate::deflate(bytes, false, level)),
        Codec::Xz | Codec::Zstd => Err(format!("{} data can be read (DECOMPRESS$) but not made yet: use \"gzip\"", codec.name())),
        #[allow(unreachable_patterns)]
        _ => Err(format!("this program was built without {} support", codec.name())),
    }
}

/// One entry for [`Writer`].
struct NewEntry {
    name: String,
    data: Vec<u8>,
    folder: bool,
    /// Seconds since 1970 (UTC).
    mtime: i64,
}

/// A new archive, built in memory and made by [`Writer::finish`].
pub struct Writer {
    format: Format,
    level: u32,
    /// The local time zone's seconds east (zip keeps local times).
    offset: i64,
    entries: Vec<NewEntry>,
}

impl Writer {
    /// A new archive of `format` (zip, tar, tar.gz, gzip), packed at
    /// `level` (0 store … 9 best); `offset`: the local time zone's seconds
    /// east of UTC (a zip keeps local times).
    pub fn new(format: Format, level: u32, offset: i64) -> Result<Writer, String> {
        if format == Format::Rar {
            return Err(RAR_REFUSED.into());
        }
        if !format.writable() {
            return Err(format!("{} archives can be opened but not made: make a .zip, .tar, .tar.gz or .gz", format.name()));
        }
        Ok(Writer { format, level: level.min(9), offset, entries: Vec::new() })
    }

    pub fn format(&self) -> Format {
        self.format
    }

    pub fn count(&self) -> usize {
        self.entries.len()
    }

    /// The name an entry is kept under: `/` between folders, relative, no
    /// `..` (RapidR never makes an archive another tool would extract
    /// outside its folder).
    fn entry_name(name: &str, folder: bool) -> Result<String, String> {
        let parts = safe_path(name)?;
        let mut n = parts.join("/");
        if folder {
            n.push('/');
        }
        Ok(n)
    }

    /// Adds a file `name` (folders before it with `/`) holding `data`,
    /// modified at `mtime` (seconds since 1970, UTC). A name already there
    /// is replaced.
    pub fn add(&mut self, name: &str, data: Vec<u8>, mtime: i64) -> Result<(), String> {
        let name = Self::entry_name(name, false)?;
        if self.format == Format::Gzip && self.entries.iter().any(|e| e.name != name) {
            return Err("a .gz file holds one file: make a .tar.gz or .zip for more".into());
        }
        self.entries.retain(|e| e.name != name);
        self.entries.push(NewEntry { name, data, folder: false, mtime });
        Ok(())
    }

    /// Adds an empty folder `name`.
    pub fn add_folder(&mut self, name: &str, mtime: i64) -> Result<(), String> {
        if self.format == Format::Gzip {
            return Err("a .gz file holds one file, no folders: make a .tar.gz or .zip".into());
        }
        let name = Self::entry_name(name, true)?;
        if !self.entries.iter().any(|e| e.name == name) {
            self.entries.push(NewEntry { name, data: Vec::new(), folder: true, mtime });
        }
        Ok(())
    }

    /// The archive's bytes.
    pub fn finish(self) -> Result<Vec<u8>, String> {
        match self.format {
            Format::Tar => Ok(tar::write(&self.entries)),
            #[cfg(feature = "gzip")]
            Format::TarGz => Ok(deflate::gzip(&tar::write(&self.entries), None, self.entries.iter().map(|e| e.mtime).max().unwrap_or(0), self.level)),
            #[cfg(feature = "gzip")]
            Format::Gzip => {
                let e = self.entries.into_iter().next();
                let (name, data, mtime) = e.map_or((None, Vec::new(), 0), |e| (Some(e.name), e.data, e.mtime));
                Ok(deflate::gzip(&data, name.as_deref(), mtime, self.level))
            }
            #[cfg(feature = "zip")]
            Format::Zip => zipfmt::write(&self.entries, self.level, self.offset),
            f => Err(not_in_build(f)),
        }
    }
}

/// Reads all of `r`, at most `limit` bytes (more is an error the caller
/// words).
fn read_capped(r: &mut dyn Read, limit: u64) -> Result<Vec<u8>, String> {
    limit::read_capped(r, limit)
}

#[cfg(test)]
mod tests;

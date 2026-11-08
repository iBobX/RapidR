//! A bundle: one target's runner files, unpacked in memory.
//!
//! On the host (and in the cache) a bundle is one `<target>.zst`: a `tar`,
//! compressed with zstd, holding the target's runner program(s) (names.rs),
//! `THIRD-PARTY-NOTICES.txt`, and for `web` the interpreter and its fonts.
//! Unpacking holds to RArchive's guards (the size the signed list promised,
//! no links, no names that leave the folder) and then to the list: the
//! files in the tar are exactly the files the list names.

use rapidr_archive::{Archive, Kind, Limits};

use crate::manifest::Entry;
use crate::names;

/// A target's files, by name (`/` between folders).
#[derive(Clone, Debug)]
pub struct Bundle {
    target: String,
    files: Vec<(String, Vec<u8>)>,
}

impl Bundle {
    /// A bundle of `files`, for `target` (a canonical name).
    pub fn new(target: &str, mut files: Vec<(String, Vec<u8>)>) -> Bundle {
        files.sort_by(|a, b| a.0.cmp(&b.0));
        Bundle { target: target.to_string(), files }
    }

    pub fn target(&self) -> &str {
        &self.target
    }

    /// Every file: its name and bytes.
    pub fn files(&self) -> &[(String, Vec<u8>)] {
        &self.files
    }

    /// The bytes of the file called `name`.
    pub fn file(&self, name: &str) -> Option<&[u8]> {
        self.files.iter().find(|(n, _)| n == name).map(|(_, b)| b.as_slice())
    }

    /// The runner program to start an app from (names.rs: one per target;
    /// `windowed` changes nothing, the packer sets Windows' subsystem). `None`
    /// for `web`, which has no program, and for a bundle without it.
    pub fn runner(&self, windowed: bool) -> Option<&[u8]> {
        if self.target == "web" {
            return None;
        }
        self.file(names::runner_file(&self.target, windowed))
    }

    /// The `THIRD-PARTY-NOTICES.txt` an app built from this runner ships.
    pub fn notices(&self) -> Option<&[u8]> {
        self.file(names::NOTICES_FILE)
    }

    /// Every file a bundle for its target must have is here.
    pub fn missing(&self) -> Vec<&'static str> {
        names::required_files(&self.target).into_iter().filter(|f| self.file(f).is_none()).collect()
    }
}

/// Unpacks `bytes` (a `.zst` as served, already checked against the list)
/// into the bundle `entry` describes. A tar that is not exactly what the
/// list says, or that has a link, a device or a name that leaves the folder,
/// is refused with the reason.
pub fn unpack(entry: &Entry, bytes: &[u8]) -> Result<Bundle, String> {
    // (what the signed list promised, and no more — the tar's own headers and
    // padding on top: at most 1 KB a file, the two blocks that end it, and the
    // whole records (10 KB) Python's tarfile and GNU tar pad an archive to. A
    // ratio check is off; the size is checked to the byte below)
    let overhead = (entry.files.len() as u64 + 2) * 1024 + 10240;
    let limits = Limits { max_size: entry.unpacked.saturating_add(overhead), max_ratio: 0 };
    let archive = Archive::open(bytes.to_vec(), &format!("{}.tar.zst", entry.target), &limits)?;
    let mut names_seen: Vec<String> = Vec::new();
    let mut total: u64 = 0;
    for e in archive.entries() {
        match e.kind {
            Kind::Folder => continue,
            Kind::File => {}
            Kind::Link | Kind::Special => return Err(format!("\"{}\" is a link or a device, which a runner bundle does not have", e.name.escape_debug())),
        }
        if e.encrypted {
            return Err(format!("\"{}\" is encrypted", e.name.escape_debug()));
        }
        let parts = rapidr_archive::safe_path(&e.name)?;
        names_seen.push(parts.join("/"));
        total = total.saturating_add(e.size);
    }
    names_seen.sort();
    let mut listed = entry.files.clone();
    listed.sort();
    if names_seen != listed {
        return Err("its files are not the files the list names".into());
    }
    if total != entry.unpacked {
        return Err(format!("it unpacks to {total} bytes, the list says {}", entry.unpacked));
    }
    let mut files = Vec::new();
    for (i, data) in archive.extract_all(&limits)? {
        let name = rapidr_archive::safe_path(&archive.entries()[i].name)?.join("/");
        files.push((name, data));
    }
    let bundle = Bundle::new(&entry.target, files);
    let missing = bundle.missing();
    if !missing.is_empty() {
        return Err(format!("it lacks {}", missing.join(", ")));
    }
    Ok(bundle)
}

/// Makes the tar a bundle is, from `files` — the format the release
/// scripts write (tools/release/runners.py is the one that does for a
/// release) — for the tests of everything that reads one.
#[cfg(any(test, feature = "testing"))]
pub fn tar_for_tests(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut tar = Vec::new();
    for (name, data) in files {
        let mut header = [0u8; 512];
        header[..name.len()].copy_from_slice(name.as_bytes());
        header[100..107].copy_from_slice(b"0000755");
        header[108..115].copy_from_slice(b"0000000");
        header[116..123].copy_from_slice(b"0000000");
        header[124..135].copy_from_slice(format!("{:011o}", data.len()).as_bytes());
        header[136..147].copy_from_slice(b"00000000000");
        header[156] = b'0';
        header[257..263].copy_from_slice(b"ustar\0");
        header[263..265].copy_from_slice(b"00");
        header[148..156].fill(b' ');
        let sum: u32 = header.iter().map(|&b| u32::from(b)).sum();
        header[148..155].copy_from_slice(format!("{sum:06o}\0").as_bytes());
        header[155] = b' ';
        tar.extend_from_slice(&header);
        tar.extend_from_slice(data);
        tar.resize(tar.len().div_ceil(512) * 512, 0);
    }
    tar.extend_from_slice(&[0u8; 1024]);
    tar
}

/// [`tar_for_tests`] padded with zeros to whole records of 10240 bytes, as
/// Python's `tarfile` (tools/release/runners.py) and GNU tar end an archive.
#[cfg(any(test, feature = "testing"))]
pub fn tar_in_records_for_tests(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut tar = tar_for_tests(files);
    tar.resize(tar.len().div_ceil(10240) * 10240, 0);
    tar
}

//! tar, RapidR's own reader and writer (POSIX.1-2001 ustar with pax
//! extended headers; GNU's long names read): 512-byte headers, each file's
//! data after its header, padded to 512, two zero blocks at the end. Read
//! from bytes in memory — every field and length checked against them, so
//! a damaged tar is an error, never a panic.

use crate::{Entry, Kind, Modified, NewEntry};

const BLOCK: usize = 512;
/// A pax header's or a GNU long name's largest: 1 MB (a real one is a few
/// hundred bytes).
const MAX_META: u64 = 1 << 20;

/// Whether `b` starts with a tar header: ustar's magic, or an old (v7) tar
/// header whose checksum is right.
pub(crate) fn looks_like_tar(b: &[u8]) -> bool {
    let Some(h) = b.get(..BLOCK) else { return false };
    if h.iter().all(|&c| c == 0) {
        return false;
    }
    if &h[257..262] == b"ustar" {
        return true;
    }
    // (a v7 tar: a name, octal fields, a right checksum)
    h[0] != 0 && checksum_ok(h)
}

/// The header's checksum (bytes 148..156, read as spaces) as its stored
/// octal says — unsigned, or signed as some old tars summed.
fn checksum_ok(h: &[u8]) -> bool {
    let Some(stored) = octal(&h[148..156]) else { return false };
    let (mut unsigned, mut signed) = (0u64, 0i64);
    for (i, &c) in h.iter().enumerate() {
        let c = if (148..156).contains(&i) { b' ' } else { c };
        unsigned += u64::from(c);
        signed += i64::from(c as i8);
    }
    stored == unsigned || i64::try_from(stored).ok() == Some(signed)
}

/// A numeric field: octal digits (spaces and NULs around them), or GNU's
/// base-256 (the first byte's high bit set). `None` if it's neither.
fn octal(f: &[u8]) -> Option<u64> {
    if f.first().is_some_and(|&c| c & 0x80 != 0) {
        // (base-256, big-endian; a negative number isn't a size or a time
        // RapidR reads)
        if f[0] & 0x40 != 0 {
            return None;
        }
        let mut n: u64 = u64::from(f[0] & 0x3F);
        for &c in &f[1..] {
            n = n.checked_mul(256)?.checked_add(u64::from(c))?;
        }
        return Some(n);
    }
    let s = f.iter().position(|&c| c != b' ' && c != 0).map_or(&f[0..0], |i| &f[i..]);
    let end = s.iter().position(|&c| c == b' ' || c == 0).unwrap_or(s.len());
    let digits = &s[..end];
    if digits.is_empty() {
        return Some(0);
    }
    let mut n: u64 = 0;
    for &c in digits {
        if !(b'0'..=b'7').contains(&c) {
            return None;
        }
        n = n.checked_mul(8)?.checked_add(u64::from(c - b'0'))?;
    }
    Some(n)
}

/// A text field up to its first NUL: UTF-8, else Latin-1.
fn text(f: &[u8]) -> String {
    let end = f.iter().position(|&c| c == 0).unwrap_or(f.len());
    bytes_text(&f[..end])
}

fn bytes_text(b: &[u8]) -> String {
    match std::str::from_utf8(b) {
        Ok(s) => s.to_string(),
        Err(_) => b.iter().map(|&c| char::from(c)).collect(),
    }
}

/// What a pax extended header (or a GNU long name) says of the next entry.
#[derive(Default)]
struct Pending {
    path: Option<String>,
    size: Option<u64>,
    mtime: Option<i64>,
}

/// pax records: `<length> <key>=<value>\n`, each `length` bytes long.
fn pax(data: &[u8], into: &mut Pending) -> Result<(), String> {
    let mut rest = data;
    while !rest.is_empty() {
        if rest.iter().all(|&c| c == 0) {
            break;
        }
        let space = rest.iter().position(|&c| c == b' ').ok_or("a pax record without its length")?;
        let len: usize = std::str::from_utf8(&rest[..space]).ok().and_then(|s| s.parse().ok()).ok_or("a pax record's length isn't a number")?;
        if len <= space + 1 || len > rest.len() || rest[len - 1] != b'\n' {
            return Err("a pax record's length is wrong".into());
        }
        let record = &rest[space + 1..len - 1];
        let eq = record.iter().position(|&c| c == b'=').ok_or("a pax record without '='")?;
        let (key, value) = (&record[..eq], &record[eq + 1..]);
        match key {
            b"path" => into.path = Some(bytes_text(value)),
            b"size" => into.size = std::str::from_utf8(value).ok().and_then(|s| s.parse().ok()),
            b"mtime" => into.mtime = std::str::from_utf8(value).ok().and_then(|s| s.split('.').next()?.parse().ok()),
            _ => {}
        }
        rest = &rest[len..];
    }
    Ok(())
}

/// Where an entry's data is in the tar: (start, length).
pub(crate) type Span = (usize, usize);

/// The entries of a tar in `data`, each with where its data is. A tar cut
/// short, a header that isn't one, a length past the end are errors.
pub(crate) fn read(data: &[u8]) -> Result<Vec<(Entry, Span)>, String> {
    let mut out = Vec::new();
    let mut at = 0usize;
    let mut pending = Pending::default();
    loop {
        let Some(h) = data.get(at..at + BLOCK) else {
            // (no end blocks: GNU tar accepts a tar that just stops there)
            if at >= data.len() && pending.path.is_none() {
                break;
            }
            return Err("the tar is cut short (a header is missing its end)".into());
        };
        if h.iter().all(|&c| c == 0) {
            break;
        }
        if !checksum_ok(h) {
            return Err(format!("the tar is damaged: the header at byte {at} isn't a tar header"));
        }
        let kind_byte = h[156];
        let mut size = octal(&h[124..136]).ok_or_else(|| format!("the tar is damaged: the size at byte {} isn't a number", at + 124))?;
        if let Some(s) = pending.size.filter(|_| !matches!(kind_byte, b'x' | b'g' | b'L' | b'K')) {
            size = s;
        }
        let start = at + BLOCK;
        let end = start.checked_add(usize::try_from(size).map_err(|_| "the tar is damaged: an entry's size is too large".to_string())?).filter(|&e| e <= data.len());
        let Some(end) = end else {
            return Err("the tar is cut short (an entry's data runs past its end)".into());
        };
        let body = &data[start..end];
        let next = start + (end - start).div_ceil(BLOCK) * BLOCK;
        match kind_byte {
            b'x' | b'L' | b'g' | b'K' if size > MAX_META => return Err("the tar is damaged: an extended header is too large".into()),
            // (a pax header: the next entry's path, size, time)
            b'x' => pax(body, &mut pending).map_err(|e| format!("the tar is damaged: {e}"))?,
            // (pax's global header, GNU's long link name: nothing RapidR uses)
            b'g' | b'K' => {}
            // (GNU's long name: the next entry's)
            b'L' => pending.path = Some(text(body)),
            _ => {
                let ustar = &h[257..262] == b"ustar";
                // (POSIX ustar's prefix; GNU's "ustar  " keeps other things there)
                let posix = ustar && &h[257..263] == b"ustar\0";
                let mut name = text(&h[0..100]);
                if posix && h[345] != 0 {
                    name = format!("{}/{}", text(&h[345..500]), name);
                }
                if let Some(p) = pending.path.take() {
                    name = p;
                }
                let mtime = pending.mtime.take().or_else(|| octal(&h[136..148]).and_then(|t| i64::try_from(t).ok()));
                let kind = match kind_byte {
                    b'0' | 0 | b'7' if name.ends_with('/') => Kind::Folder,
                    b'0' | 0 | b'7' => Kind::File,
                    b'5' => Kind::Folder,
                    b'1' | b'2' => Kind::Link,
                    _ => Kind::Special,
                };
                let len = if kind == Kind::File { body.len() } else { 0 };
                let entry = Entry { name, size: len as u64, compressed_size: len as u64, kind, encrypted: false, modified: mtime.map(Modified::Unix) };
                out.push((entry, (start, len)));
                pending = Pending::default();
            }
        }
        at = next;
    }
    Ok(out)
}

/// A number in an octal field of `width` bytes (its last a NUL).
fn put_octal(f: &mut [u8], n: u64) {
    let w = f.len() - 1;
    let s = format!("{n:0w$o}");
    f[..w].copy_from_slice(&s.as_bytes()[s.len() - w..]);
    f[w] = 0;
}

/// The largest number an octal field of `width` bytes holds.
fn fits(n: u64, width: usize) -> bool {
    n < 1u64 << (3 * (width - 1))
}

/// A ustar header for `name` (≤ 100 bytes; the caller sends longer ones
/// in a pax header first).
fn header(name: &str, size: u64, mtime: i64, kind: u8) -> [u8; BLOCK] {
    let mut h = [0u8; BLOCK];
    let n = name.as_bytes();
    let take = n.len().min(100);
    h[..take].copy_from_slice(&n[..take]);
    put_octal(&mut h[100..108], if kind == b'5' { 0o755 } else { 0o644 });
    put_octal(&mut h[108..116], 0);
    put_octal(&mut h[116..124], 0);
    put_octal(&mut h[124..136], if fits(size, 12) { size } else { 0 });
    put_octal(&mut h[136..148], u64::try_from(mtime).ok().filter(|&t| fits(t, 12)).unwrap_or(0));
    h[156] = kind;
    h[257..263].copy_from_slice(b"ustar\0");
    h[263..265].copy_from_slice(b"00");
    put_octal(&mut h[329..337], 0);
    put_octal(&mut h[337..345], 0);
    // (the checksum: the header's bytes summed, its own field as spaces)
    h[148..156].copy_from_slice(b"        ");
    let sum: u64 = h.iter().map(|&c| u64::from(c)).sum();
    let s = format!("{sum:06o}");
    h[148..154].copy_from_slice(s.as_bytes());
    h[154] = 0;
    h[155] = b' ';
    h
}

fn pad(out: &mut Vec<u8>) {
    let rem = out.len() % BLOCK;
    if rem != 0 {
        out.resize(out.len() + BLOCK - rem, 0);
    }
}

/// A pax record (its length counts itself).
fn record(key: &str, value: &str) -> String {
    let body = format!(" {key}={value}\n");
    let mut len = body.len() + 1;
    while format!("{len}").len() + body.len() != len {
        len += 1;
    }
    format!("{len}{body}")
}

/// A tar of `entries` (folders as folders), in pax-extended ustar: a name
/// longer than 100 bytes or not ASCII, a size past 8 GB, goes in a pax
/// header before its entry.
pub(crate) fn write(entries: &[NewEntry]) -> Vec<u8> {
    let mut out = Vec::new();
    for e in entries {
        let size = if e.folder { 0 } else { e.data.len() as u64 };
        let mut extended = String::new();
        if e.name.len() > 100 || !e.name.is_ascii() {
            extended.push_str(&record("path", &e.name));
        }
        if !fits(size, 12) {
            extended.push_str(&record("size", &size.to_string()));
        }
        if !extended.is_empty() {
            let base = e.name.trim_end_matches('/').rsplit('/').next().unwrap_or("");
            let pax_name: String = format!("PaxHeaders/{base}").chars().filter(char::is_ascii).take(100).collect();
            out.extend_from_slice(&header(&pax_name, extended.len() as u64, e.mtime, b'x'));
            out.extend_from_slice(extended.as_bytes());
            pad(&mut out);
        }
        let short: String = if e.name.is_ascii() { e.name.chars().take(100).collect() } else { e.name.chars().filter(char::is_ascii).take(100).collect() };
        out.extend_from_slice(&header(&short, size, e.mtime, if e.folder { b'5' } else { b'0' }));
        if !e.folder {
            out.extend_from_slice(&e.data);
            pad(&mut out);
        }
    }
    // (the end: two zero blocks, the whole a multiple of 10 KB — tar's
    // default record)
    out.resize(out.len() + 2 * BLOCK, 0);
    let rem = out.len() % (20 * BLOCK);
    if rem != 0 {
        out.resize(out.len() + 20 * BLOCK - rem, 0);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, data: &[u8]) -> NewEntry {
        NewEntry { name: name.into(), data: data.to_vec(), folder: false, mtime: 1_700_000_000 }
    }

    #[test]
    fn round_trip_with_long_and_unicode_names() {
        let long = format!("{}/game.z80", "d".repeat(150));
        let mut folder = entry("roms/", b"");
        folder.folder = true;
        let tar = write(&[folder, entry("roms/manic.z80", &[0xF3, 0xAF, 0x11]), entry(&long, b"x"), entry("café/naïve.txt", b"hola")]);
        assert_eq!(tar.len() % (20 * BLOCK), 0);
        assert!(looks_like_tar(&tar));
        let got = read(&tar).unwrap();
        let names: Vec<_> = got.iter().map(|(e, _)| e.name.as_str()).collect();
        assert_eq!(names, ["roms/", "roms/manic.z80", long.as_str(), "café/naïve.txt"]);
        assert_eq!(got[0].0.kind, Kind::Folder);
        let (s, l) = got[1].1;
        assert_eq!(&tar[s..s + l], &[0xF3, 0xAF, 0x11]);
        assert_eq!(got[1].0.modified, Some(Modified::Unix(1_700_000_000)));
        let (s, l) = got[3].1;
        assert_eq!(&tar[s..s + l], b"hola");
    }

    #[test]
    fn damaged_tars_are_errors() {
        let tar = write(&[entry("a.txt", &[7u8; 2000])]);
        // (cut inside the data)
        assert!(read(&tar[..1200]).unwrap_err().contains("cut short"));
        // (a header's byte changed: the checksum no longer agrees)
        let mut bad = tar.clone();
        bad[10] ^= 0x55;
        assert!(read(&bad).unwrap_err().contains("isn't a tar header"));
        // (a size of 8^11 - 1 in a short file)
        let mut huge = header("x", 0o77777777777, 0, b'0').to_vec();
        huge.resize(4096, 0);
        assert!(read(&huge).unwrap_err().contains("cut short"));
    }

    #[test]
    fn gnu_long_names_and_links() {
        // (GNU tar's ././@LongLink then the entry; a symlink)
        let name = "n".repeat(120);
        let mut t = header("././@LongLink", name.len() as u64 + 1, 0, b'L').to_vec();
        t.extend_from_slice(name.as_bytes());
        t.push(0);
        pad(&mut t);
        t.extend_from_slice(&header("short", 1, 0, b'0'));
        t.push(b'z');
        pad(&mut t);
        t.extend_from_slice(&header("evil-link", 0, 0, b'2'));
        t.resize(t.len() + 1024, 0);
        let got = read(&t).unwrap();
        assert_eq!(got[0].0.name, name);
        assert_eq!(got[1].0.kind, Kind::Link);
    }

    #[test]
    fn numbers() {
        assert_eq!(octal(b"0000644\0"), Some(0o644));
        assert_eq!(octal(b"   12 \0"), Some(0o12));
        assert_eq!(octal(b"\0\0\0"), Some(0));
        assert_eq!(octal(b"0009"), None);
        assert_eq!(octal(&[0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0]), Some(256));
        assert_eq!(octal(&[0xFF; 12]), None);
        assert_eq!(record("path", "x"), "9 path=x\n");
        assert_eq!(record("path", "abcdef"), "15 path=abcdef\n");
    }
}

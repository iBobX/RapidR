//! The engine against archives other tools made (tests/fixtures, made by
//! make_fixtures.py with zip, 7zz, bsdtar, gzip, xz, zstd and Python),
//! and its own archives read back.

use super::*;

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

/// The made-up snapshot every games.* fixture holds as roms/hello.z80.
const Z80_START: [u8; 8] = [0xF3, 0xAF, 0x11, 0xFF, 0xFF, 0xC3, 0xCB, 0x11];

fn names(a: &Archive) -> Vec<String> {
    a.entries().iter().map(|e| e.name.clone()).collect()
}

fn limits() -> Limits {
    Limits { max_size: Limits::DESKTOP_MAX_SIZE, max_ratio: Limits::DEFAULT_MAX_RATIO }
}

#[test]
fn every_format_other_tools_make_opens_the_same() {
    for (file, format) in [
        ("games.zip", Format::Zip),
        ("games.7z", Format::SevenZ),
        ("games.tar", Format::Tar),
        ("games.tar.gz", Format::TarGz),
        ("games.tar.xz", Format::TarXz),
        ("games.tar.zst", Format::TarZst),
    ] {
        let a = Archive::open(fixture(file), file, &limits()).unwrap_or_else(|e| panic!("{file}: {e}"));
        assert_eq!(a.format(), format, "{file}");
        let i = a.find("roms/hello.z80", false).unwrap_or_else(|| panic!("{file}: {:?}", names(&a)));
        let e = &a.entries()[i];
        assert_eq!((e.size, e.kind, e.encrypted), (64, Kind::File, false), "{file}");
        let data = a.extract(i, &limits()).unwrap();
        assert_eq!(data.len(), 64, "{file}");
        assert_eq!(data[..8], Z80_START, "{file}");
        let r = a.find("README.TXT", false).unwrap();
        assert!(a.extract(r, &limits()).unwrap().starts_with(b"Hello from RapidR!"), "{file}");
        assert_eq!(a.find("README.TXT", true), None, "{file}: case-sensitive");
        let all = a.extract_all(&limits()).unwrap();
        assert_eq!(all.len(), 2, "{file}: the two files, not the folder");
    }
    // (zip keeps local times as they were written)
    let z = Archive::open(fixture("games.zip"), "", &limits()).unwrap();
    let e = &z.entries()[z.find("roms/hello.z80", false).unwrap()];
    assert_eq!(e.modified.unwrap().local(0).to_string(), "2026-10-29 12:00:00");
    assert_eq!(z.entries()[z.find("roms", false).unwrap()].kind, Kind::Folder);
}

#[test]
fn a_single_compressed_file_is_one_entry() {
    for (file, format) in [("hello.z80.gz", Format::Gzip), ("hello.z80.xz", Format::Xz), ("hello.z80.zst", Format::Zstd)] {
        let a = Archive::open(fixture(file), &format!("some/folder/{file}"), &limits()).unwrap_or_else(|e| panic!("{file}: {e}"));
        assert_eq!(a.format(), format);
        assert_eq!(names(&a), ["hello.z80"], "{file}");
        let data = a.extract(0, &limits()).unwrap();
        assert_eq!((data.len(), &data[..8]), (64, &Z80_START[..]));
    }
    // (no file name: a gzip's own, else "data")
    assert_eq!(names(&Archive::open(fixture("hello.z80.gz"), "", &limits()).unwrap()), ["hello.z80"]);
    assert_eq!(names(&Archive::open(fixture("hello.z80.xz"), "", &limits()).unwrap()), ["data"]);
}

#[test]
fn what_isnt_an_archive_says_so() {
    let snapshot = fixture("games.tar")[512..576].to_vec();
    assert_eq!(detect(&snapshot), None);
    assert!(Archive::open(snapshot, "hello.z80", &limits()).unwrap_err().contains("not an archive"));
    assert!(Archive::open(Vec::new(), "x.zip", &limits()).unwrap_err().contains("empty"));
    assert_eq!(Archive::open(fixture("not-open.rar"), "x.rar", &limits()).err().as_deref(), Some(RAR_REFUSED));
    assert!(Archive::open(b"BZh91AY&SY".to_vec(), "x.bz2", &limits()).unwrap_err().contains("bzip2"));
    // (named .zip but not one: the zip reader says it's damaged)
    assert!(Archive::open(b"just text".to_vec(), "x.zip", &limits()).unwrap_err().contains("not a zip"));
}

#[test]
fn formats_by_name_and_extension() {
    assert_eq!(Format::from_file_name("C:\\Games\\Pack.TAR.GZ"), Some(Format::TarGz));
    assert_eq!(Format::from_file_name("a.tgz"), Some(Format::TarGz));
    assert_eq!(Format::from_file_name("x.7z"), Some(Format::SevenZ));
    assert_eq!(Format::from_file_name("hello.z80"), None);
    assert_eq!(Format::from_name(".ZIP"), Some(Format::Zip));
    assert_eq!(Format::from_name("tar.zst").map(Format::name), Some("tar.zst"));
    assert!(Format::Zip.writable() && Format::TarGz.writable() && !Format::SevenZ.writable());
}

#[test]
fn zip_slip_names_are_listed_but_never_a_path() {
    let a = Archive::open(fixture("evil-slip.zip"), "", &limits()).unwrap();
    assert_eq!(names(&a), ["ok.txt", "../evil.txt", "/tmp/rapidr-abs.txt", "sub/../../up.txt"]);
    assert!(safe_path(&a.entries()[0].name).is_ok());
    for e in &a.entries()[1..] {
        assert!(safe_path(&e.name).is_err(), "{}", e.name);
    }
    // (a link in a tar: listed as one, refused when unpacked)
    let t = Archive::open(fixture("evil-link.tar"), "", &limits()).unwrap();
    assert_eq!(t.entries()[1].kind, Kind::Link);
    assert!(t.extract(1, &limits()).unwrap_err().contains("is a link"));
    // (ExtractAll's files: no link among them)
    let all: Vec<usize> = t.extract_all(&limits()).unwrap().into_iter().map(|(i, _)| i).collect();
    assert_eq!(all, [0, 2]);
}

#[test]
fn zip_bombs_are_errors_not_full_memory() {
    let a = Archive::open(fixture("bomb.zip"), "bomb.zip", &limits()).unwrap();
    // (the zip says 30 MB in about 30 KB: listed, refused when unpacked)
    assert_eq!(a.entries()[0].size, 30 << 20);
    let e = a.extract(0, &limits()).unwrap_err();
    assert!(e.contains("MaxRatio") && e.contains("zip bomb"), "{e}");
    let small = Limits { max_size: 1 << 20, max_ratio: 0 };
    let e = a.extract(0, &small).unwrap_err();
    assert!(e.contains("MaxSize (1048576 bytes)"), "{e}");
    // (the limits lifted: it does unpack)
    assert_eq!(a.extract(0, &Limits { max_size: 64 << 20, max_ratio: 0 }).unwrap().len(), 30 << 20);
    // (a .tar.gz is unpacked at open: refused there)
    let big = Writer::new(Format::TarGz, 9, 0).map(|mut w| {
        w.add("zeros", vec![0; 8 << 20], 0).unwrap();
        w.finish().unwrap()
    });
    let e = Archive::open(big.unwrap(), "z.tar.gz", &Limits { max_size: 4 << 20, max_ratio: 0 }).err().unwrap();
    assert!(e.contains("MaxSize"), "{e}");
    // (DECOMPRESS$ too)
    let packed = compress(&vec![0; 4 << 20], Codec::Gzip, 9).unwrap();
    assert!(decompress(&packed, None, &Limits { max_size: 1 << 20, max_ratio: 0 }).unwrap_err().contains("MaxSize"));
    assert!(decompress(&packed, None, &limits()).unwrap_err().contains("MaxRatio"));
}

#[test]
fn encrypted_entries_are_refused() {
    let z = Archive::open(fixture("encrypted.zip"), "", &limits()).unwrap();
    assert!(z.entries()[0].encrypted);
    let e = z.extract(0, &limits()).unwrap_err();
    assert!(e.contains("encrypted") && e.contains("password"), "{e}");
    assert!(z.extract_all(&limits()).unwrap_err().contains("encrypted"));
    // (7z with its files encrypted: listed, refused; with its list
    // encrypted: refused at open)
    let s = Archive::open(fixture("encrypted-files.7z"), "", &limits()).unwrap();
    assert!(s.entries()[0].encrypted);
    assert!(s.extract(0, &limits()).unwrap_err().contains("encrypted"));
    let e = Archive::open(fixture("encrypted-list.7z"), "", &limits()).err().unwrap();
    assert!(e.contains("encrypted"), "{e}");
}

#[test]
fn cp437_names_of_old_zips() {
    let a = Archive::open(fixture("cp437.zip"), "", &limits()).unwrap();
    assert_eq!(names(&a), ["café.txt"]);
    assert_eq!(a.extract(0, &limits()).unwrap(), b"cp437\n");
}

#[test]
fn what_rapidr_writes_reads_back_and_other_tools_agree() {
    for format in [Format::Zip, Format::Tar, Format::TarGz] {
        for level in [0, 6, 9] {
            let mut w = Writer::new(format, level, -3 * 3600).unwrap();
            w.add("readme.txt", b"Hello".to_vec(), 1_793_000_000).unwrap();
            w.add_folder("roms", 1_793_000_000).unwrap();
            w.add("roms\\hello.z80", Z80_START.to_vec(), 1_793_000_000).unwrap();
            w.add("ünïcode/naïve ☕.txt", b"utf-8".to_vec(), 1_793_000_000).unwrap();
            w.add(&"long/".repeat(40), b"deep".to_vec(), 1_793_000_000).unwrap_or(());
            assert!(w.add("../evil", Vec::new(), 0).is_err(), "RapidR never makes a zip-slip archive");
            let bytes = w.finish().unwrap();
            let a = Archive::open(bytes, "", &limits()).unwrap();
            assert_eq!(a.format(), format);
            let i = a.find("roms/hello.z80", true).unwrap_or_else(|| panic!("{format:?}: {:?}", names(&a)));
            assert_eq!(a.extract(i, &limits()).unwrap(), Z80_START);
            let u = a.find("ünïcode/naïve ☕.txt", true).unwrap();
            assert_eq!(a.extract(u, &limits()).unwrap(), b"utf-8");
            assert_eq!(a.entries()[a.find("roms", false).unwrap()].kind, Kind::Folder);
            // (the time: zip's local (UTC-3 here), tar's UTC)
            let t = a.entries()[i].modified.unwrap();
            assert_eq!(t.local(-3 * 3600).to_string(), Stamp::from_unix(1_793_000_000, -3 * 3600).to_string());
        }
    }
    // (a .gz: one file, its name in the header)
    let mut w = Writer::new(Format::Gzip, 9, 0).unwrap();
    w.add("hello.z80", Z80_START.to_vec(), 1_793_000_000).unwrap();
    assert!(w.add("second.txt", Vec::new(), 0).unwrap_err().contains("one file"));
    let a = Archive::open(w.finish().unwrap(), "", &limits()).unwrap();
    assert_eq!((a.format(), names(&a)), (Format::Gzip, vec!["hello.z80".to_string()]));
    assert_eq!(a.entries()[0].modified, Some(Modified::Unix(1_793_000_000)));
    assert!(Writer::new(Format::SevenZ, 6, 0).unwrap_err().contains("can be opened but not made"));
    assert_eq!(Writer::new(Format::Rar, 6, 0).err().as_deref(), Some(RAR_REFUSED));
}

/// RapidR's zip and tar.gz checked by the system's own tools (unzip -t,
/// bsdtar, gzip -t) where they are.
#[cfg(unix)]
#[test]
fn system_tools_accept_what_rapidr_writes() {
    use std::process::Command;
    let dir = std::env::temp_dir().join(format!("rapidr-archive-tools-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let make = |format, file: &str| {
        let mut w = Writer::new(format, 6, 0).unwrap();
        w.add("readme.txt", b"Hello from RapidR".to_vec(), 1_793_000_000).unwrap();
        w.add("roms/hello.z80", Z80_START.to_vec(), 1_793_000_000).unwrap();
        w.add("ünïcode.txt", b"x".to_vec(), 1_793_000_000).unwrap();
        let p = dir.join(file);
        std::fs::write(&p, w.finish().unwrap()).unwrap();
        p
    };
    let zip = make(Format::Zip, "t.zip");
    let tgz = make(Format::TarGz, "t.tar.gz");
    let have = |tool: &str| Command::new("sh").args(["-c", &format!("command -v {tool}")]).output().is_ok_and(|o| o.status.success());
    if have("unzip") {
        let o = Command::new("unzip").arg("-t").arg(&zip).output().unwrap();
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stdout));
        assert!(String::from_utf8_lossy(&o.stdout).contains("No errors detected"));
    }
    if have("bsdtar") {
        let o = Command::new("bsdtar").arg("-tzf").arg(&tgz).output().unwrap();
        assert!(o.status.success());
        // (its third name printed as the system's locale shows it)
        let listed: Vec<String> = String::from_utf8_lossy(&o.stdout).lines().map(str::to_string).collect();
        assert_eq!(listed.len(), 3);
        assert_eq!(listed[..2], ["readme.txt", "roms/hello.z80"]);
    }
    if have("gzip") {
        assert!(Command::new("gzip").arg("-t").arg(&tgz).status().unwrap().success());
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn compress_and_decompress_round_trip() {
    let text = "RapidR ".repeat(1000).into_bytes();
    for codec in [Codec::Gzip, Codec::Zlib, Codec::Deflate] {
        let packed = compress(&text, codec, 6).unwrap();
        assert!(packed.len() < text.len() / 10, "{codec:?}");
        let named = decompress(&packed, Some(codec), &limits()).unwrap();
        assert_eq!(named, text, "{codec:?}");
        if codec != Codec::Deflate {
            assert_eq!(Codec::detect(&packed), Some(codec));
            assert_eq!(decompress(&packed, None, &limits()).unwrap(), text, "{codec:?} found");
        }
    }
    // (level 0: stored, still a valid gzip)
    assert_eq!(decompress(&compress(b"abc", Codec::Gzip, 0).unwrap(), None, &limits()).unwrap(), b"abc");
    // (xz and zstd made by other tools)
    assert_eq!(decompress(&fixture("hello.z80.xz"), None, &limits()).unwrap()[..8], Z80_START);
    assert_eq!(decompress(&fixture("hello.z80.zst"), None, &limits()).unwrap()[..8], Z80_START);
    assert!(compress(b"x", Codec::Xz, 6).unwrap_err().contains("not made yet"));
    assert!(decompress(b"plain text", None, &limits()).unwrap_err().contains("not compressed data"));
    assert!(decompress(&[0x1F, 0x8B, 8, 0, 0, 0], None, &limits()).unwrap_err().contains("damaged"));
    assert_eq!(decompress(b"", None, &limits()).unwrap(), b"");
}

#[test]
fn stamps() {
    let s = Stamp::from_unix(0, 0);
    assert_eq!(s.to_string(), "1970-01-01 00:00:00");
    assert_eq!(Stamp::from_unix(1_793_000_000, 0).to_string(), "2026-10-26 07:33:20");
    assert_eq!(Stamp::from_unix(1_793_000_000, -3 * 3600).to_string(), "2026-10-26 04:33:20");
    assert_eq!(Stamp::from_unix(951_782_400, 0).to_string(), "2000-02-29 00:00:00");
    for t in [0i64, 951_782_400, 1_793_000_000, -86_400 * 365] {
        assert_eq!(Stamp::from_unix(t, 7200).to_unix(7200), t);
    }
}

//! Malformed archives never crash a program (a RapidR program gets an
//! error, never a panic): every reader is fed archives other tools made,
//! damaged at random — bytes changed, cut short, bytes inserted, blocks
//! repeated — and random bytes behind each format's magic. Opening,
//! listing, finding, unpacking one entry and all of them may fail, with a
//! message; none may panic, hang or unpack past the limits.
//!
//!   cargo test -p rapidr-archive --test robustness
//!   PROPTEST_CASES=20000 cargo test -p rapidr-archive --release --test robustness

use proptest::prelude::*;
use rapidr_archive::{decompress, detect, safe_path, Archive, Codec, Limits};

const FIXTURES: &[&str] = &[
    "games.zip",
    "games.7z",
    "games.tar",
    "games.tar.gz",
    "games.tar.xz",
    "games.tar.zst",
    "hello.z80.gz",
    "hello.z80.xz",
    "hello.z80.zst",
    "evil-slip.zip",
    "evil-link.tar",
    "encrypted.zip",
    "encrypted-files.7z",
    "cp437.zip",
];

/// Each format's first bytes, for random archives that get past detection.
const MAGICS: &[&[u8]] = &[
    b"PK\x03\x04",
    b"PK\x05\x06",
    &[0x1F, 0x8B, 8],
    &[b'7', b'z', 0xBC, 0xAF, 0x27, 0x1C, 0, 4],
    &[0xFD, b'7', b'z', b'X', b'Z', 0],
    &[0x28, 0xB5, 0x2F, 0xFD],
    &[0x78, 0x9C],
];

/// Small limits: a damaged archive that "unpacks" to a lot stops early.
const LIMITS: Limits = Limits { max_size: 4 << 20, max_ratio: 200 };

fn fixtures() -> Vec<Vec<u8>> {
    FIXTURES.iter().map(|f| std::fs::read(format!("{}/tests/fixtures/{f}", env!("CARGO_MANIFEST_DIR"))).unwrap()).collect()
}

/// Everything a program can ask of an archive.
fn exercise(bytes: Vec<u8>, name: &str) {
    let _ = detect(&bytes);
    let _ = decompress(&bytes, None, &LIMITS);
    let _ = decompress(&bytes, Some(Codec::Deflate), &LIMITS);
    let Ok(a) = Archive::open(bytes, name, &LIMITS) else { return };
    for (i, e) in a.entries().iter().enumerate() {
        let _ = safe_path(&e.name);
        let _ = a.find(&e.name, false);
        if let Ok(data) = a.extract(i, &LIMITS) {
            assert!(data.len() as u64 <= LIMITS.max_size, "{name}: entry {i} past MaxSize");
        }
    }
    if let Ok(all) = a.extract_all(&LIMITS) {
        assert!(all.iter().map(|(_, d)| d.len() as u64).sum::<u64>() <= LIMITS.max_size, "{name}: all past MaxSize");
    }
    let _ = a.extract(usize::MAX, &LIMITS);
}

#[derive(Clone, Debug)]
enum Damage {
    Flip(usize, u8),
    Cut(usize),
    Insert(usize, Vec<u8>),
    Repeat(usize, usize),
    Zero(usize, usize),
}

fn damage() -> impl Strategy<Value = Damage> {
    prop_oneof![
        4 => (any::<usize>(), 1u8..=255).prop_map(|(at, x)| Damage::Flip(at, x)),
        1 => any::<usize>().prop_map(Damage::Cut),
        1 => (any::<usize>(), proptest::collection::vec(any::<u8>(), 1..32)).prop_map(|(at, b)| Damage::Insert(at, b)),
        1 => (any::<usize>(), 1usize..600).prop_map(|(at, n)| Damage::Repeat(at, n)),
        1 => (any::<usize>(), 1usize..64).prop_map(|(at, n)| Damage::Zero(at, n)),
    ]
}

fn apply(mut b: Vec<u8>, d: &Damage) -> Vec<u8> {
    if b.is_empty() {
        return b;
    }
    let at = |i: usize, len: usize| i % len;
    match d {
        Damage::Flip(i, x) => {
            let i = at(*i, b.len());
            b[i] ^= x;
        }
        Damage::Cut(i) => b.truncate(at(*i, b.len())),
        Damage::Insert(i, bytes) => {
            let i = at(*i, b.len() + 1);
            b.splice(i..i, bytes.iter().copied());
        }
        Damage::Repeat(i, n) => {
            let i = at(*i, b.len());
            let end = (i + n).min(b.len());
            let chunk = b[i..end].to_vec();
            b.splice(i..i, chunk);
        }
        Damage::Zero(i, n) => {
            let i = at(*i, b.len());
            let end = (i + n).min(b.len());
            b[i..end].fill(0);
        }
    }
    b
}

proptest! {
    // (512 cases each by default; PROPTEST_CASES for a longer run)
    #![proptest_config(ProptestConfig { cases: std::env::var("PROPTEST_CASES").ok().and_then(|s| s.parse().ok()).unwrap_or(512), failure_persistence: None, ..ProptestConfig::default() })]

    /// A fixture, damaged one to four times.
    #[test]
    fn damaged_archives_are_errors_not_crashes(which in 0..FIXTURES.len(), damages in proptest::collection::vec(damage(), 1..5)) {
        let mut b = fixtures().swap_remove(which);
        for d in &damages {
            b = apply(b, d);
        }
        exercise(b, FIXTURES[which]);
    }

    /// Random bytes behind a format's magic.
    #[test]
    fn random_bytes_behind_a_magic(m in 0..MAGICS.len(), rest in proptest::collection::vec(any::<u8>(), 0..2048)) {
        let mut b = MAGICS[m].to_vec();
        b.extend(rest);
        exercise(b, "random");
    }

    /// Random bytes in a 512-byte tar header that checks out (a correct
    /// checksum), so the tar reader's fields are what's random.
    #[test]
    fn random_tar_headers(mut h in proptest::collection::vec(any::<u8>(), 512..2048), kind in prop_oneof![Just(b'0'), Just(b'x'), Just(b'L'), Just(b'5'), Just(b'2'), any::<u8>()]) {
        h[156] = kind;
        h[257..263].copy_from_slice(b"ustar\0");
        h[148..156].copy_from_slice(b"        ");
        let sum: u64 = h[..512].iter().map(|&c| u64::from(c)).sum();
        h[148..155].copy_from_slice(format!("{sum:06o}\0").as_bytes());
        exercise(h, "random.tar");
    }
}

/// Every fixture cut at every length: the truncations that hit a header's
/// edge exactly.
#[test]
fn every_truncation_of_every_fixture() {
    for (bytes, name) in fixtures().into_iter().zip(FIXTURES) {
        if bytes.len() > 4096 {
            for cut in (0..bytes.len()).step_by(37) {
                exercise(bytes[..cut].to_vec(), name);
            }
        } else {
            for cut in 0..bytes.len() {
                exercise(bytes[..cut].to_vec(), name);
            }
        }
    }
}

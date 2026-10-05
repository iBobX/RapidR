//! The fixtures tools/make_avi_fixtures.py writes, every frame against the
//! pattern it draws (computed again here).

use super::super::codec;
use super::*;

const W: usize = 32;
const H: usize = 24;

macro_rules! fixture {
    ($name:literal) => {
        ($name, include_bytes!(concat!("../../../../../tests/fixtures/video/", $name)) as &[u8])
    };
}

const FIXTURES: [(&str, &[u8]); 14] = [
    fixture!("dib1.avi"),
    fixture!("dib4.avi"),
    fixture!("dib8.avi"),
    fixture!("dib16.avi"),
    fixture!("dib16bf.avi"),
    fixture!("dib24.avi"),
    fixture!("dib32.avi"),
    fixture!("rle8.avi"),
    fixture!("rle4.avi"),
    fixture!("cram8.avi"),
    fixture!("cram16.avi"),
    fixture!("cinepak.avi"),
    fixture!("mjpeg.avi"),
    fixture!("audio.avi"),
];

fn fixture(name: &str) -> &'static [u8] {
    FIXTURES.iter().find(|f| f.0 == name).unwrap().1
}

/// The script's colors (Cinepak's YUV entries as RGB).
const PALETTE: [(u32, u32, u32); 8] =
    [(16, 16, 16), (200, 200, 200), (220, 60, 20), (60, 125, 220), (100, 175, 80), (240, 135, 240), (40, 60, 100), (240, 215, 200)];

/// The script's `index_at`.
fn index_at(f: usize, x: usize, y: usize) -> usize {
    let (bx, by) = (x / 4, y / 4);
    let sx = f.min(4);
    if bx == 0 && by == 0 {
        return usize::from(x % 2 == 1 && y.is_multiple_of(2));
    }
    if (sx..=sx + 1).contains(&bx) && (1..=2).contains(&by) {
        return if bx == sx && by == 1 && x % 4 < 2 && y % 4 < 2 { 7 } else { 2 };
    }
    if bx == 7 && by == 5 {
        return (3 + f) % 8;
    }
    [0, 6, 1][(bx + by) % 3]
}

fn bgr((r, g, b): (u32, u32, u32)) -> u32 {
    b << 16 | g << 8 | r
}

/// What a fixture's frame `f` shows at (x, y).
fn expected(name: &str, f: usize, x: usize, y: usize) -> u32 {
    let f = if name == "dib16.avi" && f == 5 { 4 } else { f };
    let i = index_at(f, x, y);
    let (r, g, b) = PALETTE[i];
    let q5 = |v: u32| (v >> 3) << 3 | v >> 5;
    let q6 = |v: u32| (v >> 2) << 2 | v >> 6;
    bgr(match name {
        "dib1.avi" if i == 1 => (255, 255, 255),
        "dib1.avi" => (0, 0, 0),
        "rle8.avi" if f >= 6 && i == 1 => (250, 250, 10),
        "dib16.avi" | "cram16.avi" => (q5(r), q5(g), q5(b)),
        "dib16bf.avi" => (q5(r), q6(g), q5(b)),
        _ => (r, g, b),
    })
}

/// Mean absolute difference per channel between a frame and the pattern.
fn mean_error(name: &str, f: usize, px: &[u32]) -> f64 {
    let mut sum = 0u64;
    for y in 0..H {
        for x in 0..W {
            let (a, b) = (px[y * W + x], expected(name, f, x, y));
            for shift in [0, 8, 16] {
                sum += u64::from((a >> shift & 0xFF).abs_diff(b >> shift & 0xFF));
            }
        }
    }
    sum as f64 / (W * H * 3) as f64
}

fn check_frame(name: &str, f: usize, px: &[u32]) {
    assert_eq!(px.len(), W * H, "{name}");
    if name == "mjpeg.avi" {
        let e = mean_error(name, f, px);
        assert!(e <= 6.0, "{name} frame {f}: mean error {e}");
        return;
    }
    for y in 0..H {
        for x in 0..W {
            assert_eq!(px[y * W + x], expected(name, f, x, y), "{name} frame {f} pixel ({x}, {y})");
        }
    }
}

#[test]
fn fixtures_parse() {
    let codecs = [
        ("dib1.avi", "DIB"),
        ("dib16bf.avi", "DIB"),
        ("rle8.avi", "RLE8"),
        ("rle4.avi", "RLE4"),
        ("cram8.avi", "CRAM"),
        ("cram16.avi", "CRAM"),
        ("cinepak.avi", "cvid"),
        ("mjpeg.avi", "MJPG"),
    ];
    for (name, bytes) in FIXTURES {
        let avi = parse(bytes).unwrap_or_else(|| panic!("{name}"));
        assert_eq!((avi.width, avi.height, avi.frames), (32, 24, 8), "{name}");
        assert_eq!(avi.us_per_frame, 100_000.0, "{name}");
        assert_eq!(avi.length_ms(), 800, "{name}");
        assert_eq!(avi.chunks.len(), 8, "{name}");
        if let Some((_, codec)) = codecs.iter().find(|c| c.0 == name) {
            assert_eq!(avi.codec_name(), *codec, "{name}");
        }
        assert_eq!(avi.audio.is_some(), name == "audio.avi", "{name}");
    }
}

#[test]
fn every_frame_matches_the_pattern() {
    for (name, bytes) in FIXTURES {
        let avi = parse(bytes).unwrap();
        let mut d = Decoder::new(&avi);
        for f in 0..8 {
            check_frame(name, f, d.frame(&avi, f as u32));
        }
        // (past the end: the last frame)
        check_frame(name, 7, d.frame(&avi, 99));
    }
}

#[test]
fn seeks_equal_sequential_decoding() {
    for (name, bytes) in FIXTURES {
        let avi = parse(bytes).unwrap();
        let mut d = Decoder::new(&avi);
        let sequential: Vec<Vec<u32>> = (0..8).map(|f| d.frame(&avi, f).to_vec()).collect();
        let mut seeking = Decoder::new(&avi);
        let mut seed = 0x2545_F491u32;
        for _ in 0..60 {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            let f = seed % 8;
            assert_eq!(seeking.frame(&avi, f), &sequential[f as usize][..], "{name} seek to {f}");
        }
        for f in [7, 3, 6, 0, 5, 4, 2, 1, 7] {
            let mut fresh = Decoder::new(&avi);
            assert_eq!(fresh.frame(&avi, f), &sequential[f as usize][..], "{name} fresh decoder at {f}");
        }
    }
}

#[test]
fn key_frames_as_mci_has_them() {
    // (Windows 11's MCIAVI, seek to 3: uncompressed 3, delta codecs and
    // files without idx1 0)
    let at = |name: &str, n: u32| parse(fixture(name)).unwrap().key_frame_at(n);
    for name in ["dib1.avi", "dib4.avi", "dib8.avi", "dib16.avi", "dib16bf.avi", "dib32.avi", "audio.avi", "mjpeg.avi"] {
        assert_eq!(at(name, 3), 3, "{name}");
    }
    for name in ["cinepak.avi", "cram8.avi", "cram16.avi", "rle8.avi", "rle4.avi", "dib24.avi"] {
        assert_eq!(at(name, 3), 0, "{name}");
    }
    assert_eq!(at("rle8.avi", 5), 4);
    assert_eq!(at("cinepak.avi", 7), 4);
    // (dib16's frame 5 is an empty "drop" chunk, not a key frame)
    assert_eq!(at("dib16.avi", 5), 4);
    assert_eq!(at("dib8.avi", 1000), 7);
}

#[test]
fn palette_change_recolors() {
    let avi = parse(fixture("rle8.avi")).unwrap();
    assert_eq!(avi.palchanges.len(), 1);
    let mut d = Decoder::new(&avi);
    // (index 1 is the corner block's detail and a background color)
    assert_eq!(d.frame(&avi, 5)[1], bgr((200, 200, 200)));
    assert_eq!(d.frame(&avi, 6)[1], bgr((250, 250, 10)));
    assert_eq!(d.frame(&avi, 3)[1], bgr((200, 200, 200)));
}

#[test]
fn pcm_track() {
    let avi = parse(fixture("audio.avi")).unwrap();
    let a = avi.audio.as_ref().unwrap();
    assert_eq!((a.rate, a.bits, a.channels), (11025, 8, 1));
    assert_eq!(a.data.len(), 8820);
    for (i, &s) in a.data.iter().enumerate() {
        assert_eq!(s, if (i * 882 / 11025) % 2 == 0 { 0xC0 } else { 0x40 }, "sample {i}");
    }
}

#[test]
fn not_avis() {
    assert!(parse(b"").is_none());
    assert!(parse(b"RIFF\x04\0\0\0WAVE").is_none());
    assert!(parse(include_bytes!("../../../../../tests/fixtures/media_song.mid")).is_none());
    // (a codec RapidR doesn't decode: MCI's "cannot be played")
    let mut b = fixture("cinepak.avi").to_vec();
    for i in 0..b.len() - 4 {
        if &b[i..i + 4] == b"cvid" {
            b[i..i + 4].copy_from_slice(b"IV50");
        }
    }
    assert!(parse(&b).is_none());
}

#[test]
fn idx1_offsets_either_way() {
    // (dib16's are absolute; dib8's from movi; dib24 has none)
    for name in ["dib16.avi", "dib8.avi", "dib24.avi"] {
        let avi = parse(fixture(name)).unwrap();
        assert!(avi.chunks.iter().enumerate().all(|(i, c)| c.len == if name == "dib16.avi" && i == 5 { 0 } else { W * H * avi_bpp(name) / 8 }), "{name}");
    }
    // (an idx1 pointing nowhere: the movi list is scanned instead)
    let mut b = fixture("dib8.avi").to_vec();
    let at = b.windows(4).rposition(|w| w == b"idx1").unwrap();
    for e in b[at + 8..].as_chunks_mut::<16>().0 {
        e[8..12].copy_from_slice(&0x7000u32.to_le_bytes());
    }
    let avi = parse(&b).unwrap();
    let mut d = Decoder::new(&avi);
    for f in 0..8 {
        check_frame("dib8.avi", f, d.frame(&avi, f as u32));
    }
    assert_eq!(avi.key_frame_at(3), 0);
}

fn avi_bpp(name: &str) -> usize {
    match name {
        "dib16.avi" => 16,
        "dib24.avi" => 24,
        _ => 8,
    }
}

/// xorshift: the same "random" bytes every run.
fn noise(seed: &mut u64) -> u64 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    *seed
}

fn decode_all(bytes: &[u8]) {
    if let Some(avi) = parse(bytes) {
        // (a damaged size is a big picture of nothing: slow, not new)
        if avi.width * avi.height > 1 << 16 {
            return;
        }
        let mut d = Decoder::new(&avi);
        for f in 0..avi.frames.min(16) {
            let px = d.frame(&avi, f);
            assert_eq!(px.len(), avi.width as usize * avi.height as usize);
        }
        d.frame(&avi, 0);
        d.frame(&avi, u32::MAX);
        avi.key_frame_at(u32::MAX);
        avi.length_ms();
    }
}

#[test]
fn truncated_and_damaged_files_never_panic() {
    let mut seed = 0x9E37_79B9_7F4A_7C15u64;
    for (_, bytes) in FIXTURES {
        let step = (bytes.len() / 200).max(1);
        for n in (0..bytes.len()).step_by(step) {
            decode_all(&bytes[..n]);
        }
        for _ in 0..150 {
            let mut b = bytes.to_vec();
            for _ in 0..1 + noise(&mut seed) % 8 {
                let i = (noise(&mut seed) % b.len() as u64) as usize;
                b[i] = noise(&mut seed) as u8;
            }
            decode_all(&b);
        }
    }
    for _ in 0..300 {
        let len = (noise(&mut seed) % 600) as usize;
        let mut b: Vec<u8> = (0..len).map(|_| noise(&mut seed) as u8).collect();
        if len >= 12 {
            b[0..4].copy_from_slice(b"RIFF");
            b[8..12].copy_from_slice(b"AVI ");
        }
        decode_all(&b);
    }
}

#[test]
fn codecs_survive_garbage() {
    let mut seed = 0x1234_5678_9ABC_DEF1u64;
    let (w, h) = (37, 29);
    let mut idx = vec![0u8; w * h];
    let mut rgb = vec![0u32; w * h];
    let mut st = cinepak::State::default();
    let pal = vec![0u32; 256];
    for round in 0..400 {
        let len = (noise(&mut seed) % 2000) as usize;
        let mut b: Vec<u8> = (0..len).map(|_| noise(&mut seed) as u8).collect();
        if round % 2 == 0 && len > 12 {
            // (a plausible Cinepak header, so strips get read)
            b[8] = 0;
            b[9] = (round % 5) as u8;
            b[10] = 0x10;
            b[11] = 0;
        }
        rle::decode(&b, w, h, false, &mut idx);
        rle::decode(&b, w, h, true, &mut idx);
        cram::decode8(&b, w, h, &mut idx);
        cram::decode16(&b, w, h, &mut rgb);
        cinepak::decode(&mut st, &b, w, h, None, &mut rgb);
        cinepak::decode(&mut st, &b, w, h, Some(&pal), &mut rgb);
        mjpeg::decode(&b, w, h, &mut rgb);
        for bits in [1, 4, 8] {
            dib::decode_indexed(&b, w, h, &dib::Layout::new(bits, round % 3 == 0, None), &mut idx);
        }
        for bits in [16, 24, 32] {
            dib::decode_rgb(&b, w, h, &dib::Layout::new(bits, false, Some([0xF0, 0x0F00, 0x1F_0000])), &mut rgb);
        }
    }
    // (an MJPEG frame with two JPEGs in it, as interlaced files have)
    let avi = parse(fixture("mjpeg.avi")).unwrap();
    let c = avi.chunks[0];
    let jpeg = &avi.data[c.start..c.start + c.len];
    let two = [jpeg, jpeg].concat();
    let mut tall = vec![0u32; 32 * 48];
    assert!(mjpeg::decode(&two, 32, 48, &mut tall));
    assert!(mjpeg::decode(jpeg, 32, 48, &mut tall));
}

#[test]
fn mjpeg_without_dht_gets_the_standard_tables() {
    let avi = parse(fixture("mjpeg.avi")).unwrap();
    let c = avi.chunks[0];
    let jpeg = &avi.data[c.start..c.start + c.len];
    assert!(!jpeg.windows(2).any(|w| w == [0xFF, 0xC4]));
    // (jpeg-decoder alone refuses it: no AVI1 marker to imply the tables)
    assert!(codec::decode_jpeg(jpeg).is_err());
    assert!(codec::decode_jpeg(&mjpeg::with_tables(jpeg)).is_ok());
}

/// RapidQ's own example video (QAnimate's scan.avi, RLE8, 57×53, 31
/// frames): every frame decodes, seeks agree. MCI: 2067 ms.
#[test]
#[ignore]
fn rapidq_scan_avi() {
    let path = std::env::var("RAPIDQ_SCAN_AVI").unwrap_or_else(|_| format!("{}/Downloads/Rapidq/examples/QAnimate/scan.avi", std::env::var("HOME").unwrap_or_default()));
    let Ok(bytes) = std::fs::read(&path) else {
        eprintln!("{path} isn't there: skipped");
        return;
    };
    let avi = parse(&bytes).unwrap();
    assert_eq!((avi.width, avi.height, avi.frames, avi.codec_name()), (57, 53, 31, "RLE8"));
    assert_eq!(avi.length_ms(), 2067);
    let mut d = Decoder::new(&avi);
    let frames: Vec<Vec<u32>> = (0..31).map(|f| d.frame(&avi, f).to_vec()).collect();
    assert!(frames.windows(2).filter(|w| w[0] != w[1]).count() > 20);
    for f in (0..31).rev() {
        assert_eq!(Decoder::new(&avi).frame(&avi, f), &frames[f as usize][..], "frame {f}");
    }
}

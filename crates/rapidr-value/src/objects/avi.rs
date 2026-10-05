//! AVI files as QVIDEO plays them (D. Glodt's `QVideo.inc` drove Windows'
//! MCIAVI): Microsoft's RIFF AVI format (the Video for Windows "AVI RIFF
//! File Reference", OpenDML's AVI File Format Extensions for files past
//! 1 GB), its video decoded by RapidR's own decoders for the codecs AVIs of
//! RapidQ's time used — uncompressed DIB, Microsoft RLE, Microsoft Video 1,
//! Cinepak and Motion JPEG — and its PCM sound track. Pure Rust, no
//! threads, no files: the same on the desktop and in the browser.
//! docs/io-media-plan.md §6.
//!
//! How it's used: [`parse`] reads the file once (`None`: not an AVI, or a
//! codec RapidR doesn't decode — MCI's "cannot be played"); a [`Decoder`]
//! made for it gives frames as &HBBGGRR pixels, sequential ones decoding
//! one chunk each, a jump starting from the key frame at or before it.
//!
//! What's read: `RIFF AVI ` with `LIST hdrl` (`avih`, a `LIST strl` per
//! stream: `strh`, `strf`), `LIST movi` (`##dc` / `##db` video, `##wb`
//! sound, `##pc` palette changes; `LIST rec ` groups) and `idx1`, whose
//! offsets are from the `movi` list or from the file's start (both were
//! written; the first entry tells). Without a usable index the `movi`
//! list is scanned (frame 0 the only key frame then, as MCI has it).
//! OpenDML files' further `RIFF AVIX` parts are scanned for their `movi`
//! chunks. Everything is bounds-checked:
//! a truncated or damaged file gives what can be read, never a panic.

mod cinepak;
mod cram;
mod dib;
mod mjpeg;
mod rle;

use std::rc::Rc;

/// An AVI's PCM sound track (WAVE_FORMAT_PCM, or WAVE_FORMAT_EXTENSIBLE
/// carrying PCM; 8 or 16 bits, mono or stereo).
#[derive(Debug, Clone, PartialEq)]
pub struct AviAudio {
    pub rate: u32,
    pub bits: u16,
    pub channels: u16,
    /// Every sound chunk's bytes in order (whole sample frames).
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Codec {
    Dib(dib::Layout),
    Rle8,
    Rle4,
    Cram8,
    Cram16,
    Cinepak { paletted: bool },
    Mjpeg,
}

impl Codec {
    /// Whether frames are palette indexes (shown through the palette as it
    /// is now: `##pc` changes recolor what a delta frame leaves).
    fn indexed(self) -> bool {
        match self {
            Codec::Dib(l) => l.bits <= 8,
            Codec::Rle8 | Codec::Rle4 | Codec::Cram8 => true,
            _ => false,
        }
    }

    /// Whether every frame stands alone.
    fn intra(self) -> bool {
        matches!(self, Codec::Dib(_) | Codec::Mjpeg)
    }
}

/// A video frame's bytes in the file, and whether the index flags it a key frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Chunk {
    start: usize,
    len: usize,
    key: bool,
}

/// An AVI file as QVIDEO plays it. Frames come from a [`Decoder`]
/// (`Decoder::new(&avi)`, then `decoder.frame(&avi, n)`); the file's bytes
/// are shared (`Rc`), so cloning an `Avi` is cheap.
#[derive(Clone)]
pub struct Avi {
    pub width: u32,
    pub height: u32,
    /// Video frames (the video stream's strh dwLength, or the chunks found
    /// when that's 0).
    pub frames: u32,
    /// Microseconds a frame: the video stream's dwScale / dwRate (avih
    /// dwMicroSecPerFrame when those are unusable).
    pub us_per_frame: f64,
    /// The PCM sound track, when there is one.
    pub audio: Option<AviAudio>,
    data: Rc<[u8]>,
    codec: Codec,
    /// The palette strf gives (256 entries, &HBBGGRR).
    palette: Vec<u32>,
    chunks: Vec<Chunk>,
    /// `##pc` chunks: (the video frame they come before, start, length).
    palchanges: Vec<(u32, usize, usize)>,
}

impl std::fmt::Debug for Avi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Avi")
            .field("codec", &self.codec_name())
            .field("width", &self.width)
            .field("height", &self.height)
            .field("frames", &self.frames)
            .field("us_per_frame", &self.us_per_frame)
            .field("audio", &self.audio.as_ref().map(|a| (a.rate, a.bits, a.channels, a.data.len())))
            .finish()
    }
}

impl Avi {
    /// Length in milliseconds (frames × µs a frame / 1000, rounded — MCI's
    /// `status length` with time format milliseconds).
    pub fn length_ms(&self) -> i64 {
        (f64::from(self.frames) * self.us_per_frame / 1000.0).round() as i64
    }

    /// The key frame at or before frame `n` (clamped to the last frame):
    /// where MCIAVI's `seek` lands (with "seek exactly off", its default).
    /// As MCI does (checked in Windows 11): the index's AVIIF_KEYFRAME flags
    /// as written, for every codec; without an index (or in OpenDML's AVIX
    /// parts) only frame 0.
    pub fn key_frame_at(&self, n: u32) -> u32 {
        self.last_at_or_before(n, |c| c.key)
    }

    /// Where decoding frame `n` starts: any non-empty frame for codecs
    /// whose frames stand alone, a non-empty key frame for the others.
    fn decode_start(&self, n: u32) -> u32 {
        if self.codec.intra() {
            self.last_at_or_before(n, |c| c.len > 0)
        } else {
            self.last_at_or_before(n, |c| c.key && c.len > 0)
        }
    }

    fn last_at_or_before(&self, n: u32, is: impl Fn(&Chunk) -> bool) -> u32 {
        let n = (n.min(self.frames.saturating_sub(1)) as usize).min(self.chunks.len().saturating_sub(1));
        self.chunks.get(..=n).and_then(|c| c.iter().rposition(is)).unwrap_or(0) as u32
    }

    /// The video codec, as a name (for messages and tests).
    pub fn codec_name(&self) -> &'static str {
        match self.codec {
            Codec::Dib(_) => "DIB",
            Codec::Rle8 => "RLE8",
            Codec::Rle4 => "RLE4",
            Codec::Cram8 | Codec::Cram16 => "CRAM",
            Codec::Cinepak { .. } => "cvid",
            Codec::Mjpeg => "MJPG",
        }
    }
}

/// Reads an AVI: `None` when it isn't one or its video codec isn't one
/// RapidR decodes (MCI: "cannot be played").
pub fn parse(bytes: &[u8]) -> Option<Avi> {
    if bytes.get(0..4)? != b"RIFF" || bytes.get(8..12)? != b"AVI " {
        return None;
    }
    let end = riff_end(bytes, 0);
    let mut hdrl = None;
    let mut movi = None;
    let mut idx1 = None;
    // (a RIFF size too small for what follows: look on to the file's end)
    for limit in [end, bytes.len()] {
        for (id, s, l) in Chunks::new(bytes, 12, limit) {
            match &id {
                b"LIST" => match bytes.get(s..s + 4) {
                    Some(b"hdrl") if hdrl.is_none() => hdrl = Some((s + 4, s + l)),
                    Some(b"movi") if movi.is_none() => movi = Some((s, s + l)),
                    _ => {}
                },
                b"idx1" if idx1.is_none() => idx1 = Some((s, s + l)),
                _ => {}
            }
        }
        if movi.is_some() {
            break;
        }
    }
    let (hs, he) = hdrl?;
    let head = Header::read(bytes, hs, he);
    let (vnum, video) = head.streams.iter().enumerate().find(|(_, s)| &s.kind == b"vids")?;
    let strf = bytes.get(video.strf.0..video.strf.0 + video.strf.1)?;
    let (width, height, codec, palette) = video_format(strf, video.handler, &head)?;
    if width == 0 || height == 0 || width > 16384 || height > 16384 || width as usize * height as usize > 1 << 24 {
        return None;
    }
    let audio_stream = head.streams.iter().enumerate().find_map(|(i, s)| {
        let strf = bytes.get(s.strf.0..s.strf.0 + s.strf.1)?;
        (&s.kind == b"auds").then(|| pcm_format(strf).map(|f| (i, f))).flatten()
    });
    let ids = Ids { video: digits(vnum), audio: audio_stream.map(|(i, _)| digits(i)) };

    // The chunks: idx1's, else the movi list's; then OpenDML's AVIX parts.
    let mut found = Found::default();
    let indexed = match (movi, idx1) {
        (Some((ms, _)), Some(ix)) => from_idx1(bytes, ix, ms, &ids, &mut found),
        _ => false,
    };
    if !indexed {
        found = Found::default();
        if let Some((ms, me)) = movi {
            scan(bytes, ms + 4, me, &ids, &mut found, 0);
        }
    }
    let mut pos = end + (end & 1);
    let mut avix = false;
    while bytes.get(pos..pos + 4) == Some(b"RIFF") && bytes.get(pos + 8..pos + 12) == Some(b"AVIX") {
        let part_end = riff_end(bytes, pos);
        for (id, s, l) in Chunks::new(bytes, pos + 12, part_end) {
            if &id == b"LIST" && bytes.get(s..s + 4) == Some(b"movi") {
                scan(bytes, s + 4, s + l, &ids, &mut found, 0);
                avix = true;
            }
        }
        if part_end <= pos {
            break;
        }
        pos = part_end + (part_end & 1);
    }

    let count = found.chunks.len() as u32;
    let frames = match video.length {
        0 => count,
        n if avix => n.max(count),
        n => n,
    };
    let us = if video.scale > 0 && video.rate > 0 { 1e6 * f64::from(video.scale) / f64::from(video.rate) } else { 0.0 };
    let us_per_frame = if (1.0..=1e8).contains(&us) {
        us
    } else if head.avih_us > 0 {
        f64::from(head.avih_us)
    } else {
        1e6 / 15.0
    };
    let audio = audio_stream.map(|(_, (rate, bits, channels))| {
        let mut data = found.audio;
        let align = usize::from(channels) * usize::from(bits / 8);
        data.truncate(data.len() / align * align);
        AviAudio { rate, bits, channels, data }
    });
    Some(Avi {
        width,
        height,
        frames,
        us_per_frame,
        audio,
        data: Rc::from(bytes),
        codec,
        palette,
        chunks: found.chunks,
        palchanges: found.palchanges,
    })
}

fn u16le(b: &[u8], i: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(i..i + 2)?.try_into().ok()?))
}

fn u32le(b: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(i..i + 4)?.try_into().ok()?))
}

fn fourcc(b: &[u8], i: usize) -> Option<[u8; 4]> {
    b.get(i..i + 4)?.try_into().ok()
}

/// Where the RIFF chunk at `pos` ends (its size, or the file's end when
/// that's 0 or past it, as files cut off while being written have).
fn riff_end(b: &[u8], pos: usize) -> usize {
    match u32le(b, pos + 4) {
        Some(0) | None => b.len(),
        Some(n) => (pos + 8).saturating_add(n as usize).min(b.len()),
    }
}

/// The chunks between two offsets: (id, data start, data length), the
/// length cut to what's there.
struct Chunks<'a> {
    b: &'a [u8],
    pos: usize,
    end: usize,
}

impl<'a> Chunks<'a> {
    fn new(b: &'a [u8], pos: usize, end: usize) -> Chunks<'a> {
        Chunks { b, pos, end: end.min(b.len()) }
    }
}

impl Iterator for Chunks<'_> {
    type Item = ([u8; 4], usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos.checked_add(8)? > self.end {
            return None;
        }
        let id = fourcc(self.b, self.pos)?;
        let size = u32le(self.b, self.pos + 4)? as usize;
        let start = self.pos + 8;
        let len = size.min(self.end - start);
        self.pos = start.saturating_add(size).saturating_add(size & 1);
        Some((id, start, len))
    }
}

/// A stream's strh and strf.
struct Stream {
    kind: [u8; 4],
    handler: [u8; 4],
    scale: u32,
    rate: u32,
    length: u32,
    /// strf's (start, length) in the file.
    strf: (usize, usize),
}

/// What `LIST hdrl` says.
struct Header {
    avih_us: u32,
    avih_width: u32,
    avih_height: u32,
    streams: Vec<Stream>,
}

impl Header {
    fn read(b: &[u8], start: usize, end: usize) -> Header {
        let mut h = Header { avih_us: 0, avih_width: 0, avih_height: 0, streams: Vec::new() };
        for (id, s, l) in Chunks::new(b, start, end) {
            match &id {
                b"avih" => {
                    let d = &b[s..s + l];
                    h.avih_us = u32le(d, 0).unwrap_or(0);
                    h.avih_width = u32le(d, 32).unwrap_or(0);
                    h.avih_height = u32le(d, 36).unwrap_or(0);
                }
                b"LIST" if b.get(s..s + 4) == Some(b"strl") => {
                    let mut st = Stream { kind: [0; 4], handler: [0; 4], scale: 0, rate: 0, length: 0, strf: (0, 0) };
                    for (id, s, l) in Chunks::new(b, s + 4, s + l) {
                        let d = &b[s..s + l];
                        match &id {
                            b"strh" => {
                                st.kind = fourcc(d, 0).unwrap_or([0; 4]);
                                st.handler = fourcc(d, 4).unwrap_or([0; 4]);
                                st.scale = u32le(d, 20).unwrap_or(0);
                                st.rate = u32le(d, 24).unwrap_or(0);
                                st.length = u32le(d, 32).unwrap_or(0);
                            }
                            b"strf" => st.strf = (s, l),
                            _ => {}
                        }
                    }
                    h.streams.push(st);
                }
                _ => {}
            }
        }
        h
    }
}

/// The video's size, codec and palette from its BITMAPINFOHEADER (strf),
/// the codec by biCompression, else by strh's fccHandler.
fn video_format(strf: &[u8], handler: [u8; 4], head: &Header) -> Option<(u32, u32, Codec, Vec<u32>)> {
    let header_size = u32le(strf, 0)? as usize;
    let w = u32le(strf, 4)? as i32;
    let h = u32le(strf, 8)? as i32;
    let bits = u16le(strf, 14)?;
    let compression = fourcc(strf, 16)?;
    let width = if w == 0 { head.avih_width } else { w.unsigned_abs() };
    let height = if h == 0 { head.avih_height } else { h.unsigned_abs() };
    let by_fourcc = |cc: [u8; 4]| -> Option<Codec> {
        Some(match &cc.map(|c| c.to_ascii_uppercase()) {
            b"MRLE" | b"RLE " if bits == 4 => Codec::Rle4,
            b"MRLE" | b"RLE " | b"RLE8" => Codec::Rle8,
            b"RLE4" => Codec::Rle4,
            b"CRAM" | b"MSVC" | b"WHAM" => match bits {
                8 => Codec::Cram8,
                15 | 16 => Codec::Cram16,
                _ => return None,
            },
            b"CVID" => Codec::Cinepak { paletted: bits == 8 },
            b"MJPG" | b"AVRN" | b"DMB1" | b"MJPA" => Codec::Mjpeg,
            _ => return None,
        })
    };
    let masks = || Some([u32le(strf, 40)?, u32le(strf, 44)?, u32le(strf, 48)?]);
    let codec = match &compression.map(|c| c.to_ascii_uppercase()) {
        [0, 0, 0, 0] | b"DIB " | b"RGB " | b"RAW " if matches!(bits, 1 | 4 | 8 | 16 | 24 | 32) => Codec::Dib(dib::Layout::new(bits, h < 0, None)),
        [3, 0, 0, 0] if matches!(bits, 16 | 32) => Codec::Dib(dib::Layout::new(bits, h < 0, masks())),
        [1, 0, 0, 0] => Codec::Rle8,
        [2, 0, 0, 0] => Codec::Rle4,
        _ => by_fourcc(compression).or_else(|| by_fourcc(handler))?,
    };
    // (the palette: biClrUsed entries, or 2^bits, after the header)
    let mut palette = vec![0u32; 256];
    if bits <= 8 {
        let count = match u32le(strf, 32).unwrap_or(0) {
            n @ 1..=256 => n as usize,
            _ => 1usize << bits,
        };
        let at = if (40..strf.len()).contains(&header_size) { header_size } else { 40 };
        for (i, p) in palette.iter_mut().enumerate().take(count) {
            if let Some(q) = strf.get(at + i * 4..at + i * 4 + 3) {
                *p = u32::from(q[0]) << 16 | u32::from(q[1]) << 8 | u32::from(q[2]);
            }
        }
    }
    Some((width, height, codec, palette))
}

/// (rate, bits, channels) of a PCM WAVEFORMATEX (or EXTENSIBLE).
fn pcm_format(strf: &[u8]) -> Option<(u32, u16, u16)> {
    let tag = u16le(strf, 0)?;
    let pcm = tag == 1 || tag == 0xFFFE && u16le(strf, 16)? >= 22 && u32le(strf, 24)? == 1;
    let (channels, rate, bits) = (u16le(strf, 2)?, u32le(strf, 4)?, u16le(strf, 14)?);
    (pcm && rate > 0 && matches!(bits, 8 | 16) && matches!(channels, 1 | 2)).then_some((rate, bits, channels))
}

/// A stream number as chunk ids start with it ("00", "01", …).
fn digits(n: usize) -> [u8; 2] {
    [b'0' + (n / 10 % 10) as u8, b'0' + (n % 10) as u8]
}

/// Which chunk ids are the video's and the sound's.
struct Ids {
    video: [u8; 2],
    audio: Option<[u8; 2]>,
}

enum Kind {
    Video,
    Palette,
    Audio,
    Other,
}

impl Ids {
    fn kind(&self, id: &[u8; 4]) -> Kind {
        let (num, two) = ([id[0], id[1]], [id[2].to_ascii_lowercase(), id[3].to_ascii_lowercase()]);
        if num == self.video {
            match &two {
                b"dc" | b"db" => Kind::Video,
                b"pc" => Kind::Palette,
                _ => Kind::Other,
            }
        } else if Some(num) == self.audio && &two == b"wb" {
            Kind::Audio
        } else {
            Kind::Other
        }
    }
}

#[derive(Default)]
struct Found {
    chunks: Vec<Chunk>,
    palchanges: Vec<(u32, usize, usize)>,
    audio: Vec<u8>,
}

impl Found {
    fn add(&mut self, bytes: &[u8], kind: Kind, start: usize, len: usize, key: bool) {
        match kind {
            Kind::Video => self.chunks.push(Chunk { start, len, key }),
            Kind::Palette => self.palchanges.push((self.chunks.len() as u32, start, len)),
            Kind::Audio => self.audio.extend_from_slice(&bytes[start..start + len]),
            Kind::Other => {}
        }
    }
}

/// The chunks idx1 lists; false when it doesn't fit the file (then the
/// movi list is scanned instead).
fn from_idx1(bytes: &[u8], (start, end): (usize, usize), movi: usize, ids: &Ids, found: &mut Found) -> bool {
    let entries = || bytes[start..end].as_chunks::<16>().0.iter().filter(|e| &e[0..4] != b"rec " && u32le(&e[..], 4).unwrap_or(0) & 1 == 0);
    // (offsets from the 'movi' fourcc, as the reference says, or from the
    // file's start, as some writers did: whichever the first entry fits)
    let Some(first) = entries().next() else { return false };
    let off = u32le(first, 8).unwrap_or(0) as usize;
    let Some(base) = [movi, 0].into_iter().find(|&base| base.checked_add(off).and_then(|at| bytes.get(at..at.checked_add(4)?)) == Some(&first[0..4])) else {
        return false;
    };
    let (mut video, mut wrong) = (0usize, 0usize);
    for e in entries() {
        let (Some(id), Some(flags), Some(off), Some(size)) = (fourcc(e, 0), u32le(e, 4), u32le(e, 8), u32le(e, 12)) else { continue };
        let kind = ids.kind(&id);
        if matches!(kind, Kind::Other) {
            continue;
        }
        let pos = base.saturating_add(off as usize);
        let fits = bytes.get(pos..pos.saturating_add(4)) == Some(&id[..]);
        let data = pos.saturating_add(8);
        let len = if fits && data <= bytes.len() { (size as usize).min(bytes.len() - data) } else { 0 };
        if matches!(kind, Kind::Video) {
            video += 1;
            wrong += usize::from(!fits);
        }
        found.add(bytes, kind, data.min(bytes.len()), len, flags & 0x10 != 0);
    }
    video > 0 && wrong * 2 <= video
}

/// The chunks of a movi list (and its `LIST rec ` groups) in order.
fn scan(bytes: &[u8], start: usize, end: usize, ids: &Ids, found: &mut Found, depth: u32) {
    for (id, s, l) in Chunks::new(bytes, start, end) {
        if &id == b"LIST" {
            if depth < 4 && l >= 4 {
                scan(bytes, s + 4, s + l, ids, found, depth + 1);
            }
            continue;
        }
        let kind = ids.kind(&id);
        found.add(bytes, kind, s, l, false);
    }
}

/// Decodes an [`Avi`]'s frames, keeping what delta frames build on (the
/// picture, the palette as `##pc` chunks changed it, Cinepak's codebooks).
pub struct Decoder {
    /// The frame `rgb` holds.
    at: Option<u32>,
    rgb: Vec<u32>,
    /// Palette indexes, for the palettized codecs.
    idx: Vec<u8>,
    palette: Vec<u32>,
    /// `##pc` chunks applied so far.
    pal_next: usize,
    cinepak: cinepak::State,
}

impl Decoder {
    pub fn new(avi: &Avi) -> Decoder {
        let n = avi.width as usize * avi.height as usize;
        Decoder {
            at: None,
            rgb: vec![0; n],
            idx: if avi.codec.indexed() { vec![0; n] } else { Vec::new() },
            palette: avi.palette.clone(),
            pal_next: 0,
            cinepak: cinepak::State::default(),
        }
    }

    /// Frame `n` (0-based, clamped to the last) as width × height pixels,
    /// top row first, each &HBBGGRR. Sequential frames decode one chunk; a
    /// jump decodes from the key frame at or before `n`. An empty chunk (a
    /// "drop" frame) keeps the picture before. Black when the file has no
    /// frames.
    pub fn frame(&mut self, avi: &Avi, n: u32) -> &[u32] {
        if avi.frames == 0 || self.rgb.len() != avi.width as usize * avi.height as usize {
            return &self.rgb;
        }
        let n = n.min(avi.frames - 1);
        if self.at == Some(n) {
            return &self.rgb;
        }
        let key = avi.decode_start(n);
        let from = match self.at {
            Some(at) if at < n && at >= key => at + 1,
            _ => {
                self.restart(avi);
                key
            }
        };
        // (frames past the last chunk show it again; palette changes after
        // it still count)
        let last = n.min(avi.chunks.len().saturating_sub(1) as u32);
        for f in from..=last.max(from) {
            self.step(avi, f);
        }
        self.palette_until(avi, n);
        self.at = Some(n);
        if avi.codec.indexed() {
            for (o, &i) in self.rgb.iter_mut().zip(&self.idx) {
                *o = self.palette[i as usize];
            }
        }
        &self.rgb
    }

    fn restart(&mut self, avi: &Avi) {
        self.rgb.fill(0);
        self.idx.fill(0);
        self.palette.clone_from(&avi.palette);
        self.pal_next = 0;
        self.cinepak = cinepak::State::default();
    }

    /// Applies the `##pc` chunks that come before frame `f`.
    fn palette_until(&mut self, avi: &Avi, f: u32) {
        while let Some(&(before, s, l)) = avi.palchanges.get(self.pal_next) {
            if before > f {
                break;
            }
            apply_palchange(&mut self.palette, &avi.data[s..s + l]);
            self.pal_next += 1;
        }
    }

    fn step(&mut self, avi: &Avi, f: u32) {
        self.palette_until(avi, f);
        let Some(c) = avi.chunks.get(f as usize) else { return };
        if c.len == 0 {
            return;
        }
        let data = &avi.data[c.start..c.start + c.len];
        let (w, h) = (avi.width as usize, avi.height as usize);
        match avi.codec {
            Codec::Dib(layout) if layout.bits <= 8 => dib::decode_indexed(data, w, h, &layout, &mut self.idx),
            Codec::Dib(layout) => dib::decode_rgb(data, w, h, &layout, &mut self.rgb),
            Codec::Rle8 => rle::decode(data, w, h, false, &mut self.idx),
            Codec::Rle4 => rle::decode(data, w, h, true, &mut self.idx),
            Codec::Cram8 => cram::decode8(data, w, h, &mut self.idx),
            Codec::Cram16 => cram::decode16(data, w, h, &mut self.rgb),
            Codec::Cinepak { paletted } => {
                cinepak::decode(&mut self.cinepak, data, w, h, paletted.then_some(&self.palette[..]), &mut self.rgb)
            }
            Codec::Mjpeg => {
                mjpeg::decode(data, w, h, &mut self.rgb);
            }
        }
    }
}

/// An AVIPALCHANGE (bFirstEntry, bNumEntries — 0 for 256 —, wFlags, then
/// PALETTEENTRYs: red, green, blue, flags), or several in a row.
fn apply_palchange(palette: &mut [u32], d: &[u8]) {
    let mut p = 0;
    while let Some(&[first, num, _, _]) = d.get(p..p + 4) {
        let num = if num == 0 { 256 } else { num as usize };
        p += 4;
        for k in 0..num {
            let Some(e) = d.get(p..p + 4) else { return };
            if let Some(slot) = palette.get_mut(first as usize + k) {
                *slot = u32::from(e[2]) << 16 | u32::from(e[1]) << 8 | u32::from(e[0]);
            }
            p += 4;
        }
    }
}

#[cfg(test)]
mod tests;

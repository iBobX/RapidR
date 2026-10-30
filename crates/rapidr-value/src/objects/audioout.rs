//! RAudioOut: sound a program makes sample by sample, played as it
//! comes — an emulator's beeper, a synthesizer, a science simulation. The
//! same model on every runtime; the runtimes only give it a sound device
//! ([`set_device`]) and ask it, between the program's turns, which streams
//! want more ([`due`], OnNeedData).
//!
//! - **The queue.** The program writes samples into a lock-free queue (a
//!   ring of `f32`s in atomics, one writer and one reader, no lock, no
//!   `unsafe`); the sound device reads them on its own thread through a
//!   [`Consumer`], which never waits for the program. The queue holds two
//!   seconds; Write takes what fits and says how many.
//! - **Formats.** Integers as 16-bit signed samples (-32768 to 32767) or,
//!   with Bits = 8, unsigned bytes (0 to 255, 128 the middle) — from an
//!   array, a binary string (16-bit little-endian pairs) or an
//!   RMemoryStream; WriteFloat takes -1 to 1. Stereo is interleaved: left,
//!   right, left …
//! - **When it runs dry** (an underrun: the program didn't write in time)
//!   the device plays silence, after a 2 ms fade from the last sample (no
//!   click), Underruns counts it, and the sound that comes next fades in
//!   over 2 ms. Stop fades out the same way; Volume changes glide (10 ms).
//!   Start gives the device 40 ms of silence first (the lead-in): a device
//!   that takes a burst as it begins takes it from that.
//! - **Sample rates.** The stream's SampleRate is the program's; the device
//!   converts to its own (rodio on the desktop, [`Resampled`] in the
//!   browser).
//! - **The test sink.** Under the tests a clocked sink stands in for the
//!   sound card: it takes samples in real time and keeps what it heard
//!   (`__Heard`: how many, their sum and the first ones as 16-bit numbers),
//!   so the tests check exact samples on every runtime. Without a sound
//!   device the same sink plays silently, so a program's timing goes on.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crate::{v_int, v_str, Value};

/// The type name.
pub const TYPE: &str = "RAUDIOOUT";

/// SampleRate's range (what browsers' audio buffers take too).
pub const MIN_RATE: i64 = 8000;
pub const MAX_RATE: i64 = 96000;
/// BufferMs' range.
pub const MIN_BUFFER_MS: i64 = 20;
pub const MAX_BUFFER_MS: i64 = 1000;
/// How much the queue holds.
pub const QUEUE_SECONDS: usize = 2;
/// The fades around a gap and a Stop, in ms.
pub const FADE_MS: f64 = 2.0;
/// How long a Volume change takes to glide from 0 to full, in ms.
pub const GLIDE_MS: f64 = 10.0;
/// Silence the device gets first after Start, in ms, before the queue's
/// samples: a device that takes a burst as it begins (a Bluetooth headset)
/// takes it from this, not from the program's sound.
pub const LEAD_IN_MS: f64 = 40.0;
/// How many heard samples the test sink keeps one by one.
const HEARD_KEPT: usize = 64;

// ------------------------------------------------------------- the queue --

/// The lock-free queue between the program (one writer) and the sound
/// device (one reader). Positions count samples since the start and never
/// wrap (2^64 samples are millions of years of sound).
pub struct Ring {
    slots: Box<[AtomicU32]>,
    /// Samples written (the program's).
    write: AtomicU64,
    /// Samples read (the device's).
    read: AtomicU64,
    /// Clear: the reader skips up to here (the program can't move `read`).
    clear_to: AtomicU64,
}

impl Ring {
    pub fn new(capacity: usize) -> Ring {
        Ring { slots: (0..capacity.max(1)).map(|_| AtomicU32::new(0)).collect(), write: AtomicU64::new(0), read: AtomicU64::new(0), clear_to: AtomicU64::new(0) }
    }

    pub fn capacity(&self) -> usize {
        self.slots.len()
    }

    /// How many samples wait (the writer's view).
    pub fn queued(&self) -> usize {
        let w = self.write.load(Ordering::Acquire);
        let r = self.read.load(Ordering::Acquire).max(self.clear_to.load(Ordering::Acquire));
        w.saturating_sub(r) as usize
    }

    /// Adds samples (the writer only); how many fit.
    pub fn push(&self, samples: &[f32]) -> usize {
        let w = self.write.load(Ordering::Relaxed);
        let r = self.read.load(Ordering::Acquire).max(self.clear_to.load(Ordering::Acquire));
        let free = self.capacity().saturating_sub(w.saturating_sub(r) as usize);
        let n = samples.len().min(free);
        let cap = self.capacity() as u64;
        for (i, s) in samples[..n].iter().enumerate() {
            self.slots[((w + i as u64) % cap) as usize].store(s.to_bits(), Ordering::Relaxed);
        }
        self.write.store(w + n as u64, Ordering::Release);
        n
    }

    /// Takes up to `out.len()` samples, a multiple of `whole` (a frame's
    /// channels: half a frame waits for the rest), into `out` (the reader
    /// only); how many.
    pub fn pop(&self, out: &mut [f32], whole: usize) -> usize {
        let mut r = self.read.load(Ordering::Relaxed);
        r = r.max(self.clear_to.load(Ordering::Acquire));
        let w = self.write.load(Ordering::Acquire);
        let mut n = (w.saturating_sub(r) as usize).min(out.len());
        n -= n % whole.max(1);
        let cap = self.capacity() as u64;
        for (i, o) in out[..n].iter_mut().enumerate() {
            *o = f32::from_bits(self.slots[((r + i as u64) % cap) as usize].load(Ordering::Relaxed));
        }
        self.read.store(r + n as u64, Ordering::Release);
        n
    }

    /// Drops what waits (the writer only: the reader skips it).
    pub fn clear(&self) {
        self.clear_to.store(self.write.load(Ordering::Relaxed), Ordering::Release);
    }
}

// ------------------------------------------------------- the shared part --

/// What the test sink heard: how many samples, their sum and the first
/// ones, as 16-bit numbers.
#[derive(Default, Clone, Debug, PartialEq)]
pub struct Heard {
    pub count: u64,
    pub sum: i64,
    pub first: Vec<i32>,
}

impl Heard {
    fn add(&mut self, samples: &[f32]) {
        for s in samples {
            let v = to_i16(*s);
            self.count += 1;
            self.sum += i64::from(v);
            if self.first.len() < HEARD_KEPT {
                self.first.push(v);
            }
        }
    }

    /// `__Heard`: "count sum first…".
    pub fn text(&self) -> String {
        let mut t = format!("{} {}", self.count, self.sum);
        for v in &self.first {
            t.push(' ');
            t.push_str(&v.to_string());
        }
        t
    }
}

/// A sample as a 16-bit number (the test sink's record).
pub fn to_i16(s: f32) -> i32 {
    (f64::from(s.clamp(-1.0, 1.0)) * 32768.0).round().clamp(-32768.0, 32767.0) as i32
}

/// What the program and the device share: the queue, the format, and the
/// flags the device reads. A format change makes a new one.
pub struct Shared {
    pub ring: Ring,
    pub rate: u32,
    pub channels: usize,
    /// Between Start and Stop.
    playing: AtomicBool,
    /// Enabled = False.
    paused: AtomicBool,
    /// Volume's gain (f32 bits).
    gain: AtomicU32,
    underruns: AtomicU64,
    /// Frames the device took, silence included (how far it has played).
    played: AtomicU64,
    /// The test sink: keep what's heard.
    record: AtomicBool,
    heard: Mutex<Heard>,
}

impl Shared {
    fn new(rate: u32, channels: usize, gain: f32) -> Arc<Shared> {
        Arc::new(Shared {
            ring: Ring::new(rate as usize * channels * QUEUE_SECONDS),
            rate,
            channels,
            playing: AtomicBool::new(false),
            paused: AtomicBool::new(false),
            gain: AtomicU32::new(gain.to_bits()),
            underruns: AtomicU64::new(0),
            played: AtomicU64::new(0),
            record: AtomicBool::new(false),
            heard: Mutex::new(Heard::default()),
        })
    }

    pub fn underruns(&self) -> u64 {
        self.underruns.load(Ordering::Relaxed)
    }

    /// Frames the device has taken since Start.
    pub fn played(&self) -> u64 {
        self.played.load(Ordering::Relaxed)
    }

    /// The test sink keeps what it hears from now on.
    pub fn record(&self) {
        self.record.store(true, Ordering::Relaxed);
    }
}

// ------------------------------------------------- the device's reader --

/// The sound device's end of a stream: [`Consumer::render`] fills the
/// device's buffer from the queue, on the device's thread, never waiting.
pub struct Consumer {
    shared: Arc<Shared>,
    /// The gain now (gliding to Volume's).
    gain: f32,
    /// The last frame played (a fade starts from it).
    last: [f32; 2],
    /// Frames of the fade out still to play.
    fading_out: usize,
    /// Frames of fade-in the next sound gets (after a gap or a pause).
    fade_in: usize,
    /// It played the program's sound since Start (a gap after that is an underrun).
    had_sound: bool,
    /// It ran dry and hasn't had sound since.
    starved: bool,
    fade_frames: usize,
    glide_step: f32,
    finished: bool,
    /// Frames of the lead-in's silence still to give ([`LEAD_IN_MS`]).
    lead_in: usize,
}

impl Consumer {
    fn new(shared: Arc<Shared>) -> Consumer {
        let gain = f32::from_bits(shared.gain.load(Ordering::Relaxed));
        let fade_frames = ((f64::from(shared.rate) * FADE_MS / 1000.0).round() as usize).max(1);
        let glide_step = (1000.0 / (f64::from(shared.rate) * GLIDE_MS)) as f32;
        let lead_in = (f64::from(shared.rate) * LEAD_IN_MS / 1000.0).round() as usize;
        Consumer { shared, gain, last: [0.0; 2], fading_out: 0, fade_in: 0, had_sound: false, starved: false, fade_frames, glide_step, finished: false, lead_in }
    }

    pub fn rate(&self) -> u32 {
        self.shared.rate
    }

    pub fn channels(&self) -> usize {
        self.shared.channels
    }

    pub fn shared(&self) -> &Arc<Shared> {
        &self.shared
    }

    /// Fills `out` (interleaved frames, the stream's rate and channels):
    /// the program's samples, else a fade to silence. `false` once the
    /// stream was stopped and has faded out (the device can let it go).
    pub fn render(&mut self, out: &mut [f32]) -> bool {
        let ch = self.shared.channels.max(1);
        let frames = out.len() / ch;
        let out = &mut out[..frames * ch];
        self.shared.played.fetch_add(frames as u64, Ordering::Relaxed);
        // (the lead-in: silence first, the queue untouched)
        if self.lead_in > 0 && self.shared.playing.load(Ordering::Acquire) {
            let n = self.lead_in.min(frames);
            out[..n * ch].fill(0.0);
            self.lead_in -= n;
            return self.take(&mut out[n * ch..]);
        }
        self.take(out)
    }

    /// [`Consumer::render`] after the lead-in.
    fn take(&mut self, out: &mut [f32]) -> bool {
        let ch = self.shared.channels.max(1);
        let playing = self.shared.playing.load(Ordering::Acquire);
        let paused = self.shared.paused.load(Ordering::Acquire);
        if !playing || paused {
            if !self.starved {
                self.starved = true;
                self.fading_out = self.fade_frames;
            }
            // (what comes after a pause fades in; the first sound doesn't)
            if self.had_sound {
                self.fade_in = self.fade_frames;
            }
            self.silence(out, 0);
            if !playing && self.fading_out == 0 {
                self.finished = true;
            }
            return !self.finished;
        }
        let got = self.shared.ring.pop(out, ch);
        let target = f32::from_bits(self.shared.gain.load(Ordering::Relaxed));
        for (f, frame) in out[..got].chunks_exact_mut(ch).enumerate() {
            if self.gain != target {
                self.gain = if self.gain < target { (self.gain + self.glide_step).min(target) } else { (self.gain - self.glide_step).max(target) };
            }
            let mut k = self.gain;
            if self.fade_in > 0 && f < self.fade_in {
                k *= (f + 1) as f32 / (self.fade_frames + 1) as f32;
            }
            for s in frame.iter_mut() {
                *s *= k;
            }
        }
        if got > 0 {
            self.fade_in = self.fade_in.saturating_sub(got / ch);
            self.had_sound = true;
            self.starved = false;
            self.fading_out = 0;
            for c in 0..ch.min(2) {
                self.last[c] = out[got - ch + c];
            }
            if self.shared.record.load(Ordering::Relaxed) {
                if let Ok(mut h) = self.shared.heard.lock() {
                    h.add(&out[..got]);
                }
            }
        }
        if got < out.len() {
            // (the queue ran dry: a gap the program didn't fill in time)
            if !self.starved {
                if self.had_sound {
                    self.shared.underruns.fetch_add(1, Ordering::Relaxed);
                }
                self.starved = true;
                self.fading_out = self.fade_frames;
            }
            if self.had_sound {
                self.fade_in = self.fade_frames;
            }
            self.silence(out, got);
        }
        true
    }

    /// From sample `from` on: the fade from the last frame, then silence.
    fn silence(&mut self, out: &mut [f32], from: usize) {
        let ch = self.shared.channels.max(1);
        for frame in out[from..].chunks_exact_mut(ch) {
            let k = self.fading_out as f32 / (self.fade_frames + 1) as f32;
            for (c, s) in frame.iter_mut().enumerate() {
                *s = self.last[c.min(1)] * k;
            }
            self.fading_out = self.fading_out.saturating_sub(1);
        }
        if self.fading_out == 0 {
            self.last = [0.0; 2];
        }
    }
}

/// A [`Consumer`] at another rate and channel count (the browser's audio
/// context): linear interpolation between the stream's frames; one
/// channel to two by copying, two to one by their mean.
pub struct Resampled {
    consumer: Consumer,
    out_rate: u32,
    /// Where between `prev` and `next` the next output frame is (0 to 1).
    pos: f64,
    prev: [f32; 2],
    next: [f32; 2],
    block: Vec<f32>,
    at: usize,
    alive: bool,
    primed: bool,
}

/// Frames taken from the stream at a time.
const BLOCK_FRAMES: usize = 128;

impl Resampled {
    pub fn new(consumer: Consumer, out_rate: u32) -> Resampled {
        Resampled { consumer, out_rate: out_rate.max(1), pos: 0.0, prev: [0.0; 2], next: [0.0; 2], block: Vec::new(), at: 0, alive: true, primed: false }
    }

    pub fn shared(&self) -> &Arc<Shared> {
        self.consumer.shared()
    }

    fn pull(&mut self) -> [f32; 2] {
        let ch = self.consumer.channels();
        if self.at >= self.block.len() {
            self.block.resize(BLOCK_FRAMES * ch, 0.0);
            self.alive = self.consumer.render(&mut self.block) && self.alive;
            self.at = 0;
        }
        let f = &self.block[self.at..self.at + ch];
        self.at += ch;
        if ch == 1 {
            [f[0], f[0]]
        } else {
            [f[0], f[1]]
        }
    }

    /// Fills `out` (interleaved, `out_channels` a frame, at the rate given
    /// to [`Resampled::new`]). `false` once the stream has stopped.
    pub fn render(&mut self, out: &mut [f32], out_channels: usize) -> bool {
        let step = f64::from(self.consumer.rate()) / f64::from(self.out_rate);
        let oc = out_channels.max(1);
        if !self.primed {
            self.primed = true;
            self.next = self.pull();
        }
        for frame in out.chunks_exact_mut(oc) {
            while self.pos >= 1.0 {
                self.prev = self.next;
                self.next = self.pull();
                self.pos -= 1.0;
            }
            let t = self.pos as f32;
            let l = self.prev[0] + (self.next[0] - self.prev[0]) * t;
            let r = self.prev[1] + (self.next[1] - self.prev[1]) * t;
            match oc {
                1 => frame[0] = (l + r) * 0.5,
                _ => {
                    frame[0] = l;
                    frame[1] = r;
                    for s in frame.iter_mut().skip(2) {
                        *s = 0.0;
                    }
                }
            }
            self.pos += step;
        }
        self.alive
    }
}

// ------------------------------------------------------ the sound device --

/// How a runtime plays a stream: `start` takes its [`Consumer`] (the device
/// reads it on its own thread from then on); `Err` says why there's no
/// sound (the runtime then plays it silently, in time). `stop` lets go of
/// it at once (the program ends, the stream is freed).
#[derive(Clone, Copy)]
pub struct Device {
    pub start: fn(id: &str, consumer: Consumer) -> Result<(), String>,
    pub stop: fn(id: &str),
}

thread_local! {
    static DEVICE: std::cell::Cell<Option<Device>> = const { std::cell::Cell::new(None) };
    static STREAMS: RefCell<HashMap<String, AudioOut>> = RefCell::new(HashMap::new());
}

/// The runtime's sound device (once, before the first Start).
pub fn set_device(d: Device) {
    DEVICE.with(|x| x.set(Some(d)));
}

/// Whether a runtime gave a device.
pub fn has_device() -> bool {
    DEVICE.with(|d| d.get().is_some())
}

// ---------------------------------------------------- the program's side --

/// An RAudioOut as the program sees it.
pub struct AudioOut {
    rate: i64,
    channels: i64,
    bits: i64,
    buffer_ms: i64,
    volume: i64,
    enabled: bool,
    started: bool,
    error: String,
    shared: Arc<Shared>,
    /// When OnNeedData was raised (the runtime's clock, ms), until a Write.
    asked_at: Option<f64>,
}

/// Volume (0–100) as a gain: by the ear — 50 sounds about half as loud.
pub fn gain_of(volume: i64) -> f32 {
    let v = volume.clamp(0, 100) as f32 / 100.0;
    v * v
}

impl Default for AudioOut {
    fn default() -> Self {
        AudioOut { rate: 44100, channels: 1, bits: 16, buffer_ms: 60, volume: 100, enabled: true, started: false, error: String::new(), shared: Shared::new(44100, 1, 1.0), asked_at: None }
    }
}

impl AudioOut {
    /// Samples (not frames) in `ms` of sound.
    fn samples_in(&self, ms: f64) -> usize {
        (ms * self.rate as f64 / 1000.0).round() as usize * self.channels as usize
    }

    /// Queued, in ms.
    pub fn queued_ms(&self) -> i64 {
        let frames = self.shared.ring.queued() / self.channels.max(1) as usize;
        (frames as f64 * 1000.0 / self.rate as f64).floor() as i64
    }

    fn playing(&self) -> bool {
        self.started && self.enabled
    }

    /// A new queue for a new format: started again when it played.
    fn reformat(&mut self, id: &str) {
        let was = self.started;
        if was {
            self.stop(id);
        }
        self.shared = Shared::new(self.rate as u32, self.channels as usize, gain_of(self.volume));
        if was {
            self.start(id);
        }
    }

    fn start(&mut self, id: &str) {
        if self.started {
            return;
        }
        self.started = true;
        self.asked_at = None;
        self.shared.paused.store(!self.enabled, Ordering::Release);
        self.shared.playing.store(true, Ordering::Release);
        self.shared.played.store(0, Ordering::Relaxed);
        self.shared.underruns.store(0, Ordering::Relaxed);
        let consumer = Consumer::new(self.shared.clone());
        self.error.clear();
        match DEVICE.with(|d| d.get()) {
            Some(d) => {
                if let Err(e) = (d.start)(id, consumer) {
                    self.error = e;
                }
            }
            None => self.error = "No sound device here: the stream is silent".into(),
        }
    }

    fn stop(&mut self, id: &str) {
        let _ = id;
        if !self.started {
            self.shared.ring.clear();
            return;
        }
        self.started = false;
        self.asked_at = None;
        // (the device fades out and lets the stream go by itself: no click)
        self.shared.playing.store(false, Ordering::Release);
        self.shared.ring.clear();
        // (a new queue for the next Start: the old reader may still be
        // fading; Underruns and what the test sink heard still readable)
        let old = std::mem::replace(&mut self.shared, Shared::new(self.rate as u32, self.channels as usize, gain_of(self.volume)));
        self.shared.underruns.store(old.underruns(), Ordering::Relaxed);
        let heard = old.heard.lock().map(|h| h.clone()).unwrap_or_default();
        if let Ok(mut new) = self.shared.heard.lock() {
            *new = heard;
        }
    }

    /// Adds samples; how many the queue took.
    fn push(&mut self, samples: &[f32]) -> i64 {
        self.asked_at = None;
        let n = self.shared.ring.push(samples);
        n as i64
    }

    fn get(&self, prop: &str) -> Option<Value> {
        Some(match prop {
            "samplerate" => v_int(self.rate),
            "channels" => v_int(self.channels),
            "bits" => v_int(self.bits),
            "bufferms" => v_int(self.buffer_ms),
            "volume" => v_int(self.volume),
            "enabled" => v_int(i64::from(self.enabled)),
            "playing" => v_int(i64::from(self.playing())),
            "queued" => v_int(self.queued_ms()),
            "underruns" => v_int(self.shared.underruns() as i64),
            "error" => v_str(&self.error),
            "__heard" => v_str(&self.shared.heard.lock().map(|h| h.text()).unwrap_or_default()),
            _ => return None,
        })
    }

    fn set(&mut self, id: &str, prop: &str, val: &Value) -> Option<()> {
        let n = val.to_f64().round() as i64;
        match prop {
            "samplerate" => {
                let r = n.clamp(MIN_RATE, MAX_RATE);
                if r != n {
                    self.error = format!("SampleRate is from {MIN_RATE} to {MAX_RATE}: {r} it is");
                }
                if r != self.rate {
                    self.rate = r;
                    self.reformat(id);
                }
            }
            "channels" => {
                let c = n.clamp(1, 2);
                if c != n {
                    self.error = "Channels is 1 (mono) or 2 (stereo)".into();
                }
                if c != self.channels {
                    self.channels = c;
                    self.reformat(id);
                }
            }
            "bits" => {
                if n == 8 || n == 16 {
                    self.bits = n;
                } else {
                    self.error = "Bits is 16 (-32768 to 32767) or 8 (0 to 255)".into();
                }
            }
            "bufferms" => {
                self.buffer_ms = n.clamp(MIN_BUFFER_MS, MAX_BUFFER_MS);
                if self.buffer_ms != n {
                    self.error = format!("BufferMs is from {MIN_BUFFER_MS} to {MAX_BUFFER_MS}: {} it is", self.buffer_ms);
                }
            }
            "volume" => {
                self.volume = n.clamp(0, 100);
                self.shared.gain.store(gain_of(self.volume).to_bits(), Ordering::Relaxed);
            }
            "enabled" => {
                self.enabled = crate::panels::runtime::truth(val);
                self.shared.paused.store(!self.enabled, Ordering::Release);
                self.asked_at = None;
            }
            // (read-only: Playing, Queued, Underruns, Error)
            "playing" | "queued" | "underruns" | "error" => {}
            _ => return None,
        }
        Some(())
    }

    /// One sample as Bits says: a 16-bit signed number, or a byte.
    fn int_sample(&self, v: f64) -> f32 {
        if self.bits == 8 {
            ((v.round().clamp(0.0, 255.0) - 128.0) / 128.0) as f32
        } else {
            (v.round().clamp(-32768.0, 32767.0) / 32768.0) as f32
        }
    }

    /// Samples from bytes, as Bits says (16-bit little-endian pairs, or bytes).
    fn byte_samples(&self, bytes: &[u8]) -> Vec<f32> {
        if self.bits == 8 {
            bytes.iter().map(|b| (f32::from(*b) - 128.0) / 128.0).collect()
        } else {
            bytes.as_chunks::<2>().0.iter().map(|p| f32::from(i16::from_le_bytes(*p)) / 32768.0).collect()
        }
    }
}

/// `(Count, Offset)` of a Write as a range of `len` items.
fn span(args: &[Value], len: usize) -> std::ops::Range<usize> {
    let offset = args.get(2).map_or(0, |v| v.to_i64().max(0) as usize).min(len);
    let count = match args.get(1) {
        Some(v) if !matches!(v, Value::Null) => v.to_i64().max(0) as usize,
        _ => len,
    };
    offset..offset.saturating_add(count).min(len)
}

/// The samples a Write gives: an array, a binary string, an RMemoryStream
/// (from its Position, which moves past what the queue takes), a number.
/// `float`: -1 to 1 (WriteFloat), else integers as Bits says.
fn samples_of(s: &AudioOut, args: &[Value], float: bool) -> (Vec<f32>, Option<(String, usize)>) {
    let conv = |v: f64| if float { v.clamp(-1.0, 1.0) as f32 } else { s.int_sample(v) };
    match args.first() {
        Some(Value::Array(a)) => {
            let a = a.borrow();
            let r = span(args, a.data.len());
            (a.data[r].iter().map(|v| conv(v.to_f64())).collect(), None)
        }
        Some(Value::String(text)) => {
            // (an RMemoryStream named, or a binary string)
            let name = text.as_str();
            let is_stream = !name.is_empty() && name.len() < 64 && super::with_stream(name, |_| ()).is_some();
            if is_stream {
                let bytes = super::with_stream(name, |m| {
                    let from = usize::try_from(m.pos).unwrap_or(0).min(m.data.len());
                    m.data[from..].to_vec()
                })
                .unwrap_or_default();
                let width = if s.bits == 8 { 1 } else { 2 };
                let all = if float { Vec::new() } else { s.byte_samples(&bytes) };
                let r = span(args, all.len());
                let skip = r.start * width;
                return (all[r].to_vec(), Some((name.to_string(), skip)));
            }
            let bytes = super::codec::string_to_bytes(text);
            let all = s.byte_samples(&bytes);
            let r = span(args, all.len());
            (all[r].to_vec(), None)
        }
        Some(v @ (Value::Integer(_) | Value::Double(_) | Value::Boolean(_))) => (vec![conv(v.to_f64())], None),
        _ => (Vec::new(), None),
    }
}

// --------------------------------------------------- the store, by name --

pub fn is_type(type_name: &str) -> bool {
    type_name.eq_ignore_ascii_case(TYPE)
}

/// Makes stream `id` (the runtime's create).
pub fn create(id: &str, type_name: &str) -> bool {
    if !is_type(type_name) {
        return false;
    }
    STREAMS.with(|s| {
        s.borrow_mut().entry(id.to_lowercase()).or_default();
    });
    true
}

pub fn exists(id: &str) -> bool {
    STREAMS.with(|s| s.borrow().contains_key(&id.to_lowercase()))
}

fn with<R>(id: &str, f: impl FnOnce(&mut AudioOut) -> R) -> Option<R> {
    STREAMS.with(|s| s.borrow_mut().get_mut(&id.to_lowercase()).map(f))
}

/// A property (lower case); `None` when `id` isn't a stream or it isn't one of its.
pub fn get(id: &str, prop: &str) -> Option<Value> {
    with(id, |s| s.get(prop)).flatten()
}

/// Sets a property (lower case); `None` when not one of its.
pub fn set(id: &str, prop: &str, val: &Value) -> Option<()> {
    with(id, |s| s.set(id, prop, val)).flatten()
}

/// A method (lower case); `None` when not one of its.
pub fn call(id: &str, method: &str, args: &[Value]) -> Option<Value> {
    if !exists(id) {
        return None;
    }
    Some(match method {
        "start" => {
            with(id, |s| s.start(id));
            Value::Null
        }
        "stop" => {
            with(id, |s| s.stop(id));
            Value::Null
        }
        "clear" => {
            with(id, |s| {
                s.shared.ring.clear();
                s.asked_at = None;
            });
            Value::Null
        }
        "write" | "writefloat" => {
            let float = method == "writefloat";
            let (samples, from_stream) = with(id, |s| samples_of(s, args, float)).unwrap_or_default();
            let taken = with(id, |s| s.push(&samples)).unwrap_or(0);
            // (a stream's Position moves past what the queue took)
            if let Some((name, skip)) = from_stream {
                let width = with(id, |s| if s.bits == 8 { 1 } else { 2 }).unwrap_or(2);
                super::with_stream_mut(&name, |m| m.pos = m.pos.max(0) + (skip + taken as usize * width) as i64);
            }
            v_int(taken)
        }
        "writesample" => {
            let v = args.first().map_or(0.0, Value::to_f64);
            v_int(with(id, |s| {
                let x = s.int_sample(v);
                s.push(&[x])
            })
            .unwrap_or(0))
        }
        _ => return None,
    })
}

/// Forgets stream `id`, its sound stopped at once.
pub fn remove(id: &str) -> bool {
    let Some(s) = STREAMS.with(|s| s.borrow_mut().remove(&id.to_lowercase())) else { return false };
    s.shared.playing.store(false, Ordering::Release);
    if let Some(d) = DEVICE.with(|d| d.get()) {
        (d.stop)(id);
    }
    true
}

/// Whether a stream plays (the runtime then asks [`due`] often).
pub fn active() -> bool {
    STREAMS.with(|s| s.borrow().values().any(AudioOut::playing))
}

/// The streams that play and want more — their queue under BufferMs — with
/// how many samples fill it again (OnNeedData's Count): asked once, then
/// again after a Write or half of BufferMs (`now_ms`: the runtime's clock).
/// `wants(id)`: the program has an OnNeedData for it.
pub fn due(now_ms: f64, wants: &dyn Fn(&str) -> bool) -> Vec<(String, i64)> {
    let ids: Vec<String> = STREAMS.with(|s| s.borrow().iter().filter(|(_, s)| s.playing()).map(|(k, _)| k.clone()).collect());
    let mut out = Vec::new();
    for id in ids {
        if !wants(&id) {
            continue;
        }
        let ask = with(&id, |s| {
            let target = s.samples_in(s.buffer_ms as f64);
            let queued = s.shared.ring.queued();
            if queued >= target {
                return None;
            }
            if let Some(at) = s.asked_at {
                if now_ms - at < s.buffer_ms as f64 / 2.0 {
                    return None;
                }
            }
            s.asked_at = Some(now_ms);
            let ch = s.channels as usize;
            let want = (target - queued).div_ceil(ch) * ch;
            Some(want as i64)
        })
        .flatten();
        if let Some(count) = ask {
            out.push((id, count));
        }
    }
    out
}

/// Sets stream `id`'s Error (the device's word: a browser waiting for a click).
pub fn set_error(id: &str, error: &str) {
    with(id, |s| s.error = error.to_string());
}

/// Stream `id`'s Error.
pub fn error(id: &str) -> String {
    with(id, |s| s.error.clone()).unwrap_or_default()
}

/// Every stream that plays (the web's device checks them).
pub fn playing_ids() -> Vec<String> {
    STREAMS.with(|s| s.borrow().iter().filter(|(_, s)| s.playing()).map(|(k, _)| k.clone()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    fn stream_with(rate: u32, channels: usize) -> (Arc<Shared>, Consumer) {
        let shared = Shared::new(rate, channels, 1.0);
        shared.playing.store(true, Ordering::Release);
        let mut c = Consumer::new(shared.clone());
        c.lead_in = 0;
        (shared, c)
    }

    /// The device gets 40 ms of silence first; the queue waits meanwhile,
    /// and then plays exactly (the test sink hears only the program's).
    #[test]
    fn the_lead_in() {
        let shared = Shared::new(1000, 1, 1.0);
        shared.playing.store(true, Ordering::Release);
        shared.record();
        let mut c = Consumer::new(shared.clone());
        shared.ring.push(&[0.5, 0.25, -0.5]);
        let mut out = [9.0f32; 30];
        c.render(&mut out);
        assert!(out.iter().all(|s| *s == 0.0));
        assert_eq!(shared.ring.queued(), 3, "nothing taken during the lead-in");
        let mut out = [9.0f32; 13];
        c.render(&mut out);
        assert!(out[..10].iter().all(|s| *s == 0.0));
        assert_eq!(&out[10..], &[0.5, 0.25, -0.5]);
        assert_eq!(shared.underruns(), 0);
        assert_eq!(shared.heard.lock().unwrap().first, vec![16384, 8192, -16384]);
    }

    #[test]
    fn the_ring_keeps_order_and_its_size() {
        let r = Ring::new(8);
        assert_eq!(r.push(&[1.0, 2.0, 3.0]), 3);
        assert_eq!(r.queued(), 3);
        let mut out = [0.0; 2];
        assert_eq!(r.pop(&mut out, 1), 2);
        assert_eq!(out, [1.0, 2.0]);
        // (round the end: 8 slots, written 3 + 7 = 10, read 2)
        assert_eq!(r.push(&[4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0]), 7);
        assert_eq!(r.queued(), 8);
        assert_eq!(r.push(&[11.0]), 0, "full: nothing more");
        let mut all = [0.0; 8];
        assert_eq!(r.pop(&mut all, 1), 8);
        assert_eq!(all, [3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0]);
        // (whole frames only: a half frame waits)
        r.push(&[1.0, 2.0, 3.0]);
        let mut two = [0.0; 4];
        assert_eq!(r.pop(&mut two, 2), 2);
        assert_eq!(r.queued(), 1);
        // (Clear: the reader skips what was written)
        r.push(&[4.0, 5.0]);
        r.clear();
        assert_eq!(r.queued(), 0);
        assert_eq!(r.pop(&mut two, 1), 0);
        r.push(&[9.0]);
        assert_eq!(r.pop(&mut two, 1), 1);
        assert_eq!(two[0], 9.0);
    }

    /// The program on its thread, the device on another: every sample
    /// arrives once, in order.
    #[test]
    fn the_ring_across_threads() {
        let ring = Arc::new(Ring::new(1024));
        let reader = {
            let ring = ring.clone();
            std::thread::spawn(move || {
                let mut got = Vec::new();
                let mut buf = [0.0f32; 100];
                while got.len() < 100_000 {
                    let n = ring.pop(&mut buf, 1);
                    got.extend_from_slice(&buf[..n]);
                    if n == 0 {
                        std::thread::yield_now();
                    }
                }
                got
            })
        };
        let mut i = 0;
        while i < 100_000 {
            let chunk: Vec<f32> = (i..(i + 77).min(100_000)).map(|v| v as f32).collect();
            i += ring.push(&chunk);
            std::thread::yield_now();
        }
        let got = reader.join().unwrap();
        assert!(got.iter().enumerate().all(|(k, v)| *v == k as f32));
    }

    #[test]
    fn exact_samples_then_a_fade_and_one_underrun() {
        let (shared, mut c) = stream_with(1000, 1);
        shared.record();
        shared.ring.push(&[0.5, -0.5, 0.25]);
        let mut out = [9.0f32; 8];
        assert!(c.render(&mut out));
        // (the program's three, then from 0.25 down to silence in 2 ms = 2 frames)
        assert_eq!(&out[..3], &[0.5, -0.5, 0.25]);
        assert!((out[3] - 0.25 * 2.0 / 3.0).abs() < 1e-6 && (out[4] - 0.25 / 3.0).abs() < 1e-6, "{out:?}");
        assert_eq!(&out[5..], &[0.0, 0.0, 0.0]);
        assert_eq!(shared.underruns(), 1);
        // (still dry: the same gap, not another)
        c.render(&mut out);
        assert_eq!(shared.underruns(), 1);
        // (sound again: it fades in over 2 frames)
        shared.ring.push(&[0.9, 0.9, 0.9, 0.9]);
        c.render(&mut out[..4]);
        assert!((out[0] - 0.3).abs() < 1e-6 && (out[1] - 0.6).abs() < 1e-6 && out[2] == 0.9, "{out:?}");
        let heard = shared.heard.lock().unwrap().clone();
        assert_eq!(heard.count, 7);
        assert_eq!(heard.first[..3], [16384, -16384, 8192]);
    }

    #[test]
    fn starting_empty_is_no_underrun() {
        let (shared, mut c) = stream_with(1000, 1);
        let mut out = [1.0f32; 16];
        c.render(&mut out);
        assert!(out.iter().all(|s| *s == 0.0));
        assert_eq!(shared.underruns(), 0);
        // (the first sound doesn't fade in: it is exactly the program's)
        shared.ring.push(&[0.5, 0.5]);
        let mut two = [0.0f32; 2];
        c.render(&mut two);
        assert_eq!(two, [0.5, 0.5]);
    }

    #[test]
    fn stop_fades_out_then_lets_go() {
        let (shared, mut c) = stream_with(1000, 2);
        shared.ring.push(&[0.6, -0.6, 0.6, -0.6]);
        let mut out = [0.0f32; 4];
        assert!(c.render(&mut out));
        shared.playing.store(false, Ordering::Release);
        let mut tail = [0.0f32; 8];
        assert!(!c.render(&mut tail), "faded out within the block: let go");
        assert!((tail[0] - 0.4).abs() < 1e-6 && (tail[1] + 0.4).abs() < 1e-6, "{tail:?}");
        assert!(tail[4..].iter().all(|s| *s == 0.0));
    }

    #[test]
    fn volume_glides_and_full_volume_is_exact() {
        let (shared, mut c) = stream_with(1000, 1);
        shared.gain.store(0.25f32.to_bits(), Ordering::Relaxed);
        shared.ring.push(&[1.0; 20]);
        let mut out = [0.0f32; 20];
        c.render(&mut out);
        // (1000 Hz: a full glide in 10 frames, 0.1 a frame)
        assert!((out[0] - 0.9).abs() < 1e-5 && (out[4] - 0.5).abs() < 1e-5 && (out[10] - 0.25).abs() < 1e-5, "{out:?}");
        assert_eq!(gain_of(100), 1.0);
        assert_eq!(gain_of(50), 0.25);
        assert_eq!(gain_of(0), 0.0);
    }

    #[test]
    fn resampling_up_down_and_channels() {
        // (a ramp at 1000 Hz, read at 2000 Hz: halfway points between)
        let (shared, c) = stream_with(1000, 1);
        shared.ring.push(&(0..200).map(|i| i as f32 / 1000.0).collect::<Vec<_>>());
        let mut r = Resampled::new(c, 2000);
        let mut out = vec![0.0f32; 200];
        r.render(&mut out, 2);
        // (frame k of the output: input frame k/2 - 1 — the interpolator's one frame late —, both channels)
        for k in 4..100 {
            let want = (k as f32 / 2.0 - 1.0) / 1000.0;
            assert!((out[k * 2] - want).abs() < 1e-6 && out[k * 2 + 1] == out[k * 2], "frame {k}: {} vs {want}", out[k * 2]);
        }
        // (44100 Hz to 48000 Hz: the right number of frames, a sine stays a sine)
        let (shared, c) = stream_with(44100, 2);
        let sine: Vec<f32> = (0..4410).flat_map(|i| {
            let v = (2.0 * std::f32::consts::PI * 441.0 * i as f32 / 44100.0).sin();
            [v, -v]
        }).collect();
        shared.ring.push(&sine);
        let mut r = Resampled::new(c, 48000);
        let mut out = vec![0.0f32; 4800];
        r.render(&mut out, 1);
        // (left and right cancel out: two channels to one is their mean)
        assert!(out.iter().all(|s| s.abs() < 1e-6));
        let (shared, c) = stream_with(44100, 1);
        shared.ring.push(&sine.iter().step_by(2).copied().collect::<Vec<_>>());
        let mut r = Resampled::new(c, 48000);
        let mut out = vec![0.0f32; 4000];
        r.render(&mut out, 1);
        for (k, s) in out.iter().enumerate().skip(10) {
            let t = (k as f32 - 48000.0 / 44100.0) / 48000.0;
            let want = (2.0 * std::f32::consts::PI * 441.0 * t).sin();
            assert!((s - want).abs() < 0.01, "frame {k}: {s} vs {want}");
        }
    }

    #[test]
    fn formats_from_arrays_strings_and_numbers() {
        let mut s = AudioOut::default();
        let arr = Value::Array(Rc::new(RefCell::new(crate::BasicArray::new(vec![(0, 3)], v_int(0)).unwrap())));
        if let Value::Array(a) = &arr {
            a.borrow_mut().data = vec![v_int(16384), v_int(-32768), Value::Double(40000.0), v_int(0)];
        }
        let (v, _) = samples_of(&s, std::slice::from_ref(&arr), false);
        assert_eq!(v, vec![0.5, -1.0, 32767.0 / 32768.0, 0.0]);
        // (Count and Offset)
        let (v, _) = samples_of(&s, &[arr.clone(), v_int(2), v_int(1)], false);
        assert_eq!(v, vec![-1.0, 32767.0 / 32768.0]);
        // (floats: -1 to 1, clipped)
        if let Value::Array(a) = &arr {
            a.borrow_mut().data = vec![Value::Double(0.5), Value::Double(-2.0)];
        }
        let (v, _) = samples_of(&s, std::slice::from_ref(&arr), true);
        assert_eq!(v, vec![0.5, -1.0]);
        // (a binary string: 16-bit little-endian pairs; 8-bit bytes)
        let bin: String = [0x00u8, 0x40, 0x00, 0xC0, 0x01].iter().map(|b| *b as char).collect();
        let (v, _) = samples_of(&s, &[v_str(&bin)], false);
        assert_eq!(v, vec![0.5, -0.5]);
        s.bits = 8;
        let (v, _) = samples_of(&s, &[v_str(&bin)], false);
        assert_eq!(v, vec![-1.0, -0.5, -1.0, 0.5, -127.0 / 128.0]);
        let (v, _) = samples_of(&s, &[v_int(192)], false);
        assert_eq!(v, vec![0.5]);
    }

    #[test]
    fn the_component_its_properties_and_asking() {
        let id = "ut_stream";
        create(id, "RAudioOut");
        assert_eq!(get(id, "samplerate").unwrap().to_i64(), 44100);
        assert_eq!(get(id, "bufferms").unwrap().to_i64(), 60);
        assert_eq!(get(id, "enabled").unwrap().to_i64(), 1);
        set(id, "bufferms", &v_int(5));
        assert_eq!(get(id, "bufferms").unwrap().to_i64(), MIN_BUFFER_MS);
        assert!(get(id, "error").unwrap().to_string_val().contains("BufferMs"));
        set(id, "bufferms", &v_int(100));
        // (written, not started: 4410 samples are 100 ms)
        let arr = Value::Array(Rc::new(RefCell::new(crate::BasicArray::new(vec![(0, 4409)], v_int(1000)).unwrap())));
        assert_eq!(call(id, "write", &[arr]).unwrap().to_i64(), 4410);
        assert_eq!(get(id, "queued").unwrap().to_i64(), 100);
        assert_eq!(call(id, "writesample", &[v_int(5)]).unwrap().to_i64(), 1);
        call(id, "clear", &[]);
        assert_eq!(get(id, "queued").unwrap().to_i64(), 0);
        // (no device here: it says so, and asks for its 100 ms once)
        call(id, "start", &[]);
        assert_eq!(get(id, "playing").unwrap().to_i64(), 1);
        assert!(get(id, "error").unwrap().to_string_val().contains("No sound device"));
        let asks = due(0.0, &|_| true);
        assert_eq!(asks, vec![(id.to_string(), 4410)]);
        assert!(due(10.0, &|_| true).is_empty(), "asked: not again so soon");
        assert_eq!(due(60.0, &|_| true).len(), 1, "half of BufferMs later: again");
        assert!(due(70.0, &|_| false).is_empty(), "no OnNeedData: never asked");
        call(id, "write", &[v_int(0)]);
        assert_eq!(due(71.0, &|_| true).len(), 1, "after a Write: at once");
        set(id, "enabled", &v_int(0));
        assert_eq!(get(id, "playing").unwrap().to_i64(), 0);
        assert!(due(200.0, &|_| true).is_empty());
        set(id, "enabled", &v_int(1));
        call(id, "stop", &[]);
        assert_eq!(get(id, "playing").unwrap().to_i64(), 0);
        // (after Stop, Underruns still says how it went; Start counts again)
        with(id, |s| s.shared.underruns.store(3, Ordering::Relaxed));
        assert_eq!(get(id, "underruns").unwrap().to_i64(), 3);
        call(id, "start", &[]);
        assert_eq!(get(id, "underruns").unwrap().to_i64(), 0);
        call(id, "stop", &[]);
        assert!(!active());
        assert!(remove(id));
    }
}

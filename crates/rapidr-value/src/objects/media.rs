//! D. Glodt's media objects — QMIDI, QWAVE, QVIDEO, QCDAUDIO (RapidQ's
//! `QMidi.inc`, `QWave.inc`, `QVideo.inc`, `Qcdaudio.inc`, the manual's
//! Appendix A) — on one model: Windows' MCI as those libraries drove it,
//! the device's answers replaced by RapidR's own. docs/io-media-plan.md
//! §4–§7.
//!
//! What every one shares, as the libraries wrote it:
//!
//! - **State** (CLOSE 0, PLAY 1, PAUSE 2, STOP 3; QWAVE's RECORD 4),
//!   **FileOpen** (QCDAUDIO's AudioOpen), **Error** (MCI's text when Open
//!   failed), **Lenght** (sic: milliseconds; QVIDEO frames), **Volume**
//!   (0 until set; up to 100).
//! - **Timer**, a QTIMER of the object's own (Interval 1000, off): Play
//!   turns it on, Stop / Pause / Close off; at each tick the position and
//!   the state are read again (CurrentFrame, CurrentPos, …: fields the
//!   ticks update, as the libraries' were), a play that ended is Stopped,
//!   then **OnChange** fires with the position. The runtime ticks it as a
//!   timer (`objects::rqlib::media_tick`).
//! - The position runs by the runtime's clock (`directx::clock_ms`), not
//!   by the device: the same with or without a sound card, in tests too.

use std::collections::VecDeque;
use std::rc::Rc;

use crate::{v_int, v_str, Value};

use super::directx::{self, SoundPlay, Wav};
use super::midifile::Song;

pub const CLOSE: i64 = 0;
pub const PLAY: i64 = 1;
pub const PAUSE: i64 = 2;
pub const STOP: i64 = 3;
pub const RECORD: i64 = 4;

/// MCI's texts (`mciGetErrorString`, Windows 11 — checked in the VM).
pub const FILE_NOT_FOUND: &str = "Cannot find the specified file.  Make sure the path and filename are correct.";
pub const CANNOT_PLAY: &str = "The specified file cannot be played on the specified MCI device.  The file may be corrupt, not in the correct format, or no file handler available for this format.";
pub const ALIAS_IN_USE: &str = "The specified alias is already being used in this application.  Use a unique alias.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Midi,
    Wave,
    Video,
    CdAudio,
}

/// What plays a QMIDI's song: the runtime's MIDI output (none: silent).
#[derive(Clone, Copy)]
pub struct MidiDevice {
    /// Plays `song` from `from_us` (replacing what object `id` played).
    pub play: fn(id: &str, song: Rc<Song>, from_us: u64, gain: f32),
    pub stop: fn(id: &str),
    pub volume: fn(id: &str, gain: f32),
}

/// What QWAVE records from: the runtime's input (none: Record does
/// nothing, as MCI without a recording device).
#[derive(Clone, Copy)]
pub struct WaveInput {
    /// Starts recording for `id`: frames a second, bits, channels.
    pub start: fn(id: &str, rate: u32, bits: u16, channels: u16) -> bool,
    /// What came since, as PCM bytes in that format.
    pub take: fn(id: &str) -> Vec<u8>,
    pub stop: fn(id: &str),
}

thread_local! {
    static MIDI: std::cell::Cell<Option<MidiDevice>> = const { std::cell::Cell::new(None) };
    static INPUT: std::cell::Cell<Option<WaveInput>> = const { std::cell::Cell::new(None) };
}

pub fn set_midi_device(d: MidiDevice) {
    MIDI.with(|m| m.set(Some(d)));
}

pub fn set_wave_input(i: WaveInput) {
    INPUT.with(|m| m.set(Some(i)));
}

/// The tests' recording input (`RAPIDR_TEST_WAVE_IN`): a square wave of
/// `hz`, what any format needs, by the clock — never a microphone.
pub fn test_tone(rate: u32, bits: u16, channels: u16, from_frame: u64, frames: usize, hz: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(frames * usize::from(channels) * usize::from(bits / 8));
    let period = (rate / hz.max(1)).max(2) as u64;
    for f in 0..frames as u64 {
        let high = ((from_frame + f) % period) < period / 2;
        for _ in 0..channels {
            if bits == 8 {
                out.push(if high { 192 } else { 64 });
            } else {
                out.extend((if high { 8192i16 } else { -8192i16 }).to_le_bytes());
            }
        }
    }
    out
}

thread_local! {
    static TONE_HZ: std::cell::Cell<u32> = const { std::cell::Cell::new(440) };
    /// Scripted recordings: their format and when they began.
    static TONES: std::cell::RefCell<std::collections::HashMap<String, (u32, u16, u16, f64)>> = std::cell::RefCell::new(Default::default());
}

/// The tests' input on every runtime (`RAPIDR_TEST_WAVE_IN=tone:HZ`, the
/// page's `RAPIDR_TEST_WAVE_IN`): [`test_tone`] for as long as it ran.
pub fn install_tone_input(hz: u32) {
    set_wave_input(tone_input(hz));
}

/// [`install_tone_input`]'s input.
pub fn tone_input(hz: u32) -> WaveInput {
    TONE_HZ.with(|t| t.set(hz));
    WaveInput {
        start: |id, rate, bits, ch| {
            TONES.with(|t| t.borrow_mut().insert(id.to_string(), (rate, bits, ch, now())));
            true
        },
        take: |id| {
            let Some((rate, bits, ch, t0)) = TONES.with(|t| t.borrow().get(id).copied()) else { return Vec::new() };
            let frames = ((now() - t0).max(0.0) * f64::from(rate) / 1000.0) as usize;
            test_tone(rate, bits, ch, 0, frames, TONE_HZ.with(std::cell::Cell::get))
        },
        stop: |id| {
            TONES.with(|t| t.borrow_mut().remove(id));
        },
    }
}

/// `RAPIDR_TEST_WAVE_IN`'s script: `tone:HZ`'s HZ.
pub fn tone_script(script: &str) -> Option<u32> {
    script.trim().strip_prefix("tone:").and_then(|h| h.trim().parse().ok())
}

/// A wave's sound as MCI's waveaudio keeps it (any format it records).
#[derive(Debug, Clone, PartialEq)]
pub struct Sound {
    pub rate: u32,
    pub bits: u16,
    pub channels: u16,
    pub data: Vec<u8>,
}

impl Sound {
    fn bytes_per_ms(&self) -> f64 {
        f64::from(self.rate) * f64::from(self.channels) * f64::from(self.bits / 8) / 1000.0
    }
    /// Its length in milliseconds (rounded, as MCI's `status length`).
    pub fn length_ms(&self) -> i64 {
        (self.data.len() as f64 / self.bytes_per_ms().max(1e-9)).round() as i64
    }
    /// The byte where millisecond `ms` starts (a whole frame).
    fn byte_at(&self, ms: i64) -> usize {
        let block = usize::from(self.channels) * usize::from(self.bits / 8);
        let frame = (ms.max(0) as f64 * f64::from(self.rate) / 1000.0).round() as usize;
        (frame * block.max(1)).min(self.data.len())
    }
    pub fn wav_file(&self) -> Vec<u8> {
        let block = self.channels * (self.bits / 8);
        let mut b = Vec::with_capacity(44 + self.data.len());
        b.extend(b"RIFF");
        b.extend(((36 + self.data.len()) as u32).to_le_bytes());
        b.extend(b"WAVEfmt ");
        b.extend(16u32.to_le_bytes());
        b.extend(1u16.to_le_bytes());
        b.extend(self.channels.to_le_bytes());
        b.extend(self.rate.to_le_bytes());
        b.extend((self.rate * u32::from(block)).to_le_bytes());
        b.extend(block.to_le_bytes());
        b.extend(self.bits.to_le_bytes());
        b.extend(b"data");
        b.extend((self.data.len() as u32).to_le_bytes());
        b.extend(&self.data);
        b
    }
}

pub struct Media {
    pub kind: Kind,
    pub timer_interval: i64,
    pub timer_enabled: bool,
    /// The timer was turned on or off, or given another interval, by a
    /// method: the runtime schedules it again.
    pub timer_changed: bool,
    pub state: i64,
    pub file_open: bool,
    pub error: String,
    pub length: i64,
    pub volume: i64,
    /// CurrentFrame / CurrentPos as the last tick (or Pause, Stop) left it.
    pub pos: i64,
    /// Playing (or recording) since: (the clock's ms, the position then).
    since: Option<(f64, i64)>,
    song: Option<Rc<Song>>,
    sound: Option<Sound>,
    /// QWAVE: recording, and what it records into (the position it began).
    recording: Option<i64>,
    pub device_type: String,
    /// QWAVE's Bits / Frequence / Mode as the program set them.
    pub bits: i64,
    pub frequence: i64,
    pub mode: i64,
    /// QVIDEO's fields (the library's; nothing shows: §6).
    pub video: VideoFields,
    /// QCDAUDIO's (no drive: §7).
    pub cd: CdFields,
    pub events: VecDeque<(&'static str, Vec<Value>)>,
    /// The volume's gain once the program set it (none: the system's).
    gain: Option<f32>,
}

#[derive(Default, Clone)]
pub struct VideoFields {
    pub length_time: i64,
    pub handle: i64,
    pub parent: i64,
    pub border_style: i64,
    pub img_width: i64,
    pub img_height: i64,
    pub left: i64,
    pub top: i64,
    pub width: i64,
    pub height: i64,
    pub audio_off: i64,
    pub caption: String,
    pub window_state: i64,
}

#[derive(Default, Clone)]
pub struct CdFields {
    pub time: String,
    pub track_time: String,
    pub track_number: i64,
    pub time_position: String,
    pub position: i64,
    pub present: i64,
    pub current_track: i64,
}

fn now() -> f64 {
    directx::clock_ms()
}

fn bool_val(b: bool) -> Value {
    // (the libraries' True: RAPIDQ.INC's 1)
    v_int(i64::from(b))
}

impl Media {
    pub fn new(kind: Kind) -> Self {
        Media {
            kind,
            timer_interval: 1000,
            timer_enabled: false,
            timer_changed: false,
            state: CLOSE,
            file_open: false,
            error: String::new(),
            length: 0,
            volume: 0,
            pos: 0,
            since: None,
            song: None,
            sound: None,
            recording: None,
            device_type: String::new(),
            bits: 0,
            frequence: 0,
            mode: 0,
            video: VideoFields::default(),
            cd: CdFields::default(),
            events: VecDeque::new(),
            gain: None,
        }
    }

    fn set_timer(&mut self, on: bool) {
        if self.timer_enabled != on {
            self.timer_enabled = on;
            self.timer_changed = true;
        }
    }

    /// Where it plays (or records) now, by the clock.
    fn live_position(&self) -> i64 {
        match self.since {
            Some((t0, p0)) => {
                let p = p0 + (now() - t0).max(0.0) as i64;
                if self.recording.is_some() {
                    p.min(self.length.max(p0))
                } else {
                    p.min(self.length)
                }
            }
            None => self.pos,
        }
    }

    /// MCI's `status … mode`: what the device does now.
    fn live_state(&self) -> i64 {
        match (self.since, self.state) {
            (Some(_), PLAY) if self.live_position() >= self.length => STOP,
            (Some(_), RECORD) if self.live_position() >= self.length => STOP,
            (_, s) => s,
        }
    }

    pub fn get(&self, prop: &str) -> Option<Value> {
        let k = self.kind;
        Some(match prop {
            "timer.interval" | "interval" => v_int(self.timer_interval),
            "timer.enabled" | "enabled" => bool_val(self.timer_enabled),
            "state" => v_int(self.state),
            "error" => v_str(&self.error),
            "volume" if k != Kind::CdAudio => v_int(self.volume),
            "lenght" | "length" if k != Kind::CdAudio => v_int(self.length),
            "fileopen" if k != Kind::CdAudio => bool_val(self.file_open),
            "currentframe" if matches!(k, Kind::Midi | Kind::Video) => v_int(self.pos),
            "currentpos" if k == Kind::Wave => v_int(self.pos),
            "bits" if k == Kind::Wave => v_int(self.bits),
            "frequence" if k == Kind::Wave => v_int(self.frequence),
            "mode" if k == Kind::Wave => v_int(self.mode),
            "devicetype" if k == Kind::Wave => v_str(&self.device_type),
            _ if k == Kind::Video => {
                let v = &self.video;
                match prop {
                    "lenghttime" | "lengthtime" => v_int(v.length_time),
                    "handle" => v_int(v.handle),
                    "parent" => v_int(v.parent),
                    "borderstyle" => v_int(v.border_style),
                    "imgwidth" => v_int(v.img_width),
                    "imgheight" => v_int(v.img_height),
                    "left" => v_int(v.left),
                    "top" => v_int(v.top),
                    "width" => v_int(v.width),
                    "height" => v_int(v.height),
                    "audiooff" => v_int(v.audio_off),
                    "caption" => v_str(&v.caption),
                    "windowstate" => v_int(v.window_state),
                    _ => return None,
                }
            }
            _ if k == Kind::CdAudio => {
                let c = &self.cd;
                match prop {
                    "time" => v_str(&c.time),
                    "tracktime" => v_str(&c.track_time),
                    "tracknumber" => v_int(c.track_number),
                    "timeposition" => v_str(&c.time_position),
                    "position" => v_int(c.position),
                    "audioopen" => bool_val(self.file_open),
                    "present" => v_int(c.present),
                    "currenttrack" => v_int(c.current_track),
                    _ => return None,
                }
            }
            _ => return None,
        })
    }

    pub fn set(&mut self, prop: &str, val: &Value) -> Option<()> {
        let n = val.to_i64();
        let (stopped, k) = (matches!(self.state, STOP | PAUSE), self.kind);
        match prop {
            "timer.interval" | "interval" => {
                self.timer_interval = n;
                self.timer_changed = true;
            }
            "timer.enabled" | "enabled" => self.set_timer(n != 0),
            // (fields the libraries' own code writes; a program may too)
            "state" => self.state = n,
            "error" => self.error = val.to_string_val(),
            "lenght" | "length" if k != Kind::CdAudio => self.length = n,
            "fileopen" if k != Kind::CdAudio => self.file_open = n != 0,
            "volume" if k != Kind::CdAudio => {
                if n <= 100 {
                    self.volume = n;
                    let gain = (n.clamp(0, 100) as f32) / 100.0;
                    self.gain = Some(gain);
                    self.device_volume(gain);
                }
            }
            // SetCurrentFrame / SetCurrentPos: only when open and stopped
            // or paused; kept within 0 … Lenght.
            "currentframe" if matches!(k, Kind::Midi | Kind::Video) => {
                if self.file_open && stopped {
                    self.pos = n.clamp(0, self.length.max(0));
                }
            }
            "currentpos" if k == Kind::Wave => {
                if self.file_open && stopped {
                    self.pos = n.clamp(0, self.length.max(0));
                }
            }
            // SetBits / SetFrequence / SetMode: the values the library
            // takes, when open and stopped or paused (the format a new
            // wave records in).
            "bits" if k == Kind::Wave => {
                if matches!(n, 8 | 16) && self.file_open && stopped {
                    self.bits = n;
                    self.reformat();
                }
            }
            "frequence" if k == Kind::Wave => {
                if matches!(n, 8000 | 11025 | 44100) && self.file_open && stopped {
                    self.frequence = n;
                    self.reformat();
                }
            }
            "mode" if k == Kind::Wave => {
                if matches!(n, 1 | 2) && self.file_open && stopped {
                    self.mode = n;
                    self.reformat();
                }
            }
            "devicetype" if k == Kind::Wave => self.device_type = val.to_string_val(),
            _ if k == Kind::Video => self.set_video(prop, val)?,
            _ if k == Kind::CdAudio => self.set_cd(prop, val)?,
            _ => return None,
        }
        Some(())
    }

    fn set_video(&mut self, prop: &str, val: &Value) -> Option<()> {
        let n = val.to_i64();
        let open = self.file_open && self.video.handle != 0;
        let v = &mut self.video;
        match prop {
            "lenghttime" | "lengthtime" => v.length_time = n,
            "handle" => v.handle = n,
            "parent" => v.parent = n,
            "borderstyle" => v.border_style = n,
            "imgwidth" => v.img_width = n,
            "imgheight" => v.img_height = n,
            // (SetLeft … SetHeight: only with a video open)
            "left" if open => v.left = n,
            "top" if open => v.top = n,
            "width" if open => v.width = n,
            "height" if open => v.height = n,
            "left" | "top" | "width" | "height" | "audiooff" => {}
            "caption" => v.caption = val.to_string_val(),
            "windowstate" => {
                if self.file_open && v.parent == 0 {
                    v.window_state = if (0..3).contains(&n) { n } else { 0 };
                }
            }
            _ => return None,
        }
        Some(())
    }

    fn set_cd(&mut self, prop: &str, val: &Value) -> Option<()> {
        let n = val.to_i64();
        let c = &mut self.cd;
        match prop {
            "time" => c.time = val.to_string_val(),
            "tracktime" => c.track_time = val.to_string_val(),
            "tracknumber" => c.track_number = n,
            "timeposition" => c.time_position = val.to_string_val(),
            "position" => c.position = n,
            "audioopen" => self.file_open = n != 0,
            "present" => c.present = n,
            // (SetCurrentTrack: only with a disc open)
            "currenttrack" => {
                if self.file_open {
                    c.current_track = n.clamp(1, c.track_number.max(1));
                }
            }
            _ => return None,
        }
        Some(())
    }

    /// A new wave's format from Bits / Frequence / Mode (an empty one
    /// only: recorded sound keeps its own).
    fn reformat(&mut self) {
        if let Some(s) = self.sound.as_mut().filter(|s| s.data.is_empty()) {
            s.bits = self.bits as u16;
            s.rate = self.frequence as u32;
            s.channels = self.mode as u16;
        }
    }

    fn device_volume(&self, gain: f32) {
        if self.since.is_none() {
            return;
        }
        match self.kind {
            Kind::Midi => {
                if let Some(d) = MIDI.with(std::cell::Cell::get) {
                    (d.volume)(&self.id_hint(), gain);
                }
            }
            Kind::Wave => directx::device_volume(&self.id_hint(), gain),
            _ => {}
        }
    }

    // (the store sets the object's id before each call: `with_id`)
    fn id_hint(&self) -> String {
        CURRENT_ID.with(|c| c.borrow().clone())
    }

    /// `Open(FileName)` (`bytes`: the file read, or why not), QCDAUDIO's
    /// `Open`: True (1) or False (0), Error set.
    pub fn open(&mut self, file: &str, bytes: Result<Vec<u8>, String>) -> i64 {
        if self.kind == Kind::CdAudio {
            // (the library closes first; no drive's disc is ever there:
            // MCI's `open cdaudio` with `media present` false — no Error)
            self.close();
            return 0;
        }
        if file.is_empty() {
            return 0;
        }
        if self.file_open {
            // (the libraries' one alias, MEDIA / sound, still open: MCI's
            // error 289; FlagOpen False closes the open one)
            self.error = ALIAS_IN_USE.to_string();
            self.close();
            return 0;
        }
        let loaded = match bytes {
            Err(_) => Err(FILE_NOT_FOUND),
            Ok(b) => self.load(&b).ok_or(CANNOT_PLAY),
        };
        match loaded {
            Ok(length) => {
                self.length = length;
                self.state = STOP;
                self.pos = 0;
                self.file_open = true;
                1
            }
            Err(text) => {
                self.error = text.to_string();
                self.close();
                0
            }
        }
    }

    /// The file as this object plays it: its length (ms; QVIDEO frames).
    fn load(&mut self, b: &[u8]) -> Option<i64> {
        match self.kind {
            Kind::Midi => {
                let song = super::midifile::parse(b)?;
                let ms = song.length_ms();
                self.song = Some(Rc::new(song));
                Some(ms)
            }
            Kind::Wave => {
                let w = directx::parse_wav(b).ok()?;
                let s = Sound { rate: w.rate, bits: w.bits, channels: w.channels, data: w.data.to_vec() };
                (self.bits, self.frequence, self.mode) = (i64::from(s.bits), i64::from(s.rate), i64::from(s.channels));
                let ms = s.length_ms();
                self.sound = Some(s);
                Some(ms)
            }
            // (no video decoder yet: every file "cannot be played" — §6)
            Kind::Video | Kind::CdAudio => None,
        }
    }

    pub fn close(&mut self) {
        self.set_timer(false);
        self.halt_device();
        self.since = None;
        self.recording = None;
        self.file_open = false;
        self.length = 0;
        self.pos = 0;
        self.state = CLOSE;
        self.song = None;
        self.sound = None;
        if self.kind == Kind::Video {
            let caption = std::mem::take(&mut self.video.caption);
            self.video = VideoFields { caption, ..VideoFields::default() };
        }
        if self.kind == Kind::CdAudio {
            let present = self.cd.present;
            self.cd = CdFields { present, ..CdFields::default() };
        }
    }

    fn halt_device(&mut self) {
        if self.since.is_none() {
            return;
        }
        let id = self.id_hint();
        match self.kind {
            Kind::Midi => {
                if let Some(d) = MIDI.with(std::cell::Cell::get) {
                    (d.stop)(&id);
                }
            }
            Kind::Wave => {
                if self.recording.is_some() {
                    self.finish_recording();
                } else {
                    directx::device_stop(&id);
                }
            }
            _ => {}
        }
    }

    pub fn play(&mut self) {
        if !self.file_open || self.kind == Kind::Video {
            return;
        }
        if self.kind == Kind::CdAudio {
            return;
        }
        self.set_timer(true);
        self.halt_device();
        let from = self.pos;
        let id = self.id_hint();
        match self.kind {
            Kind::Midi => {
                if let (Some(d), Some(song)) = (MIDI.with(std::cell::Cell::get), self.song.clone()) {
                    (d.play)(&id, song, from as u64 * 1000, self.gain.unwrap_or(1.0));
                }
            }
            Kind::Wave => {
                if let Some(s) = &self.sound {
                    if !s.data.is_empty() && matches!(s.bits, 8 | 16) && matches!(s.channels, 1 | 2) {
                        let wav = Rc::new(Wav { channels: s.channels, rate: s.rate, bits: s.bits, data: Rc::from(s.data.as_slice()) });
                        let frame = s.byte_at(from) / (usize::from(s.channels) * usize::from(s.bits / 8)).max(1);
                        directx::device_play(&SoundPlay { id: id.clone(), wav, from: frame, speed: 1.0, gain: self.gain.unwrap_or(1.0), pan: (1.0, 1.0), looped: false });
                    }
                }
            }
            _ => {}
        }
        self.since = Some((now(), from));
        self.state = PLAY;
    }

    pub fn stop(&mut self) {
        if !self.file_open || self.kind == Kind::CdAudio {
            return;
        }
        self.halt_device();
        self.since = None;
        self.set_timer(false);
        self.state = STOP;
        self.pos = 0;
        // (QWAVE's Stop reads the length again: a recording's)
        if let Some(s) = &self.sound {
            self.length = s.length_ms();
        }
    }

    pub fn pause(&mut self) {
        if !(self.file_open && self.state == PLAY) {
            return;
        }
        let at = self.live_position();
        self.halt_device();
        self.since = None;
        self.state = PAUSE;
        self.set_timer(false);
        self.pos = at;
    }

    /// QWAVE's New: an empty wave, MCI's waveaudio defaults (8 bits,
    /// 11025 Hz, mono — checked in the VM).
    pub fn new_wave(&mut self) {
        if self.file_open {
            return;
        }
        self.sound = Some(Sound { rate: 11025, bits: 8, channels: 1, data: Vec::new() });
        (self.bits, self.frequence, self.mode) = (8, 11025, 1);
        self.state = STOP;
        self.set_timer(false);
        self.pos = 0;
        self.file_open = true;
    }

    /// QWAVE's Record: from CurrentPos up to Lenght, as MCI's `record sound
    /// to Lenght` (inserted there). Without an input, nothing.
    pub fn record(&mut self) {
        if !(self.file_open && matches!(self.state, STOP | PAUSE)) {
            return;
        }
        let Some(s) = self.sound.as_ref() else { return };
        let Some(input) = INPUT.with(std::cell::Cell::get) else { return };
        if !(input.start)(&self.id_hint(), s.rate, s.bits, s.channels) {
            return;
        }
        self.recording = Some(self.pos);
        self.since = Some((now(), self.pos));
        self.state = RECORD;
        self.set_timer(true);
    }

    /// What was recorded, put in the wave where recording began.
    fn finish_recording(&mut self) {
        let Some(at) = self.recording.take() else { return };
        let Some(input) = INPUT.with(std::cell::Cell::get) else { return };
        let id = self.id_hint();
        let mut got = (input.take)(&id);
        (input.stop)(&id);
        let end = self.live_position();
        if let Some(s) = self.sound.as_mut() {
            // (exactly the time it ran: the device's bytes cut or padded)
            let block = usize::from(s.channels) * usize::from(s.bits / 8);
            let want = ((end - at).max(0) as f64 * f64::from(s.rate) / 1000.0).round() as usize * block;
            let silence = if s.bits == 8 { 128 } else { 0 };
            got.resize(want, silence);
            let start = s.byte_at(at);
            s.data.splice(start..start, got);
        }
    }

    /// QWAVE's Save(FileName): the WAV file to write (stopped, open).
    pub fn save(&self) -> Option<Vec<u8>> {
        (self.file_open && self.state == STOP).then(|| self.sound.as_ref().map(Sound::wav_file)).flatten()
    }

    /// QWAVE's Delete(Pos1, Pos2) (ms).
    pub fn delete(&mut self, from: i64, to: i64) {
        if !(self.file_open && self.state == STOP) {
            return;
        }
        let to = to.min(self.length);
        let from = from.max(0);
        if let Some(s) = self.sound.as_mut() {
            let (a, b) = (s.byte_at(from), s.byte_at(to));
            if a < b {
                s.data.drain(a..b);
            }
            self.length = s.length_ms();
        }
    }

    /// The timer's tick: the position and state read again, an end
    /// Stopped, OnChange.
    pub fn tick(&mut self) {
        if self.kind == Kind::CdAudio {
            // (no disc: the timer never runs — Play needs AudioOpen)
            return;
        }
        let state = self.live_state();
        let pos = self.live_position();
        self.pos = pos;
        // (QWAVE's GetState also reads "stopped" while recording ends)
        if state == STOP && matches!(self.state, PLAY | RECORD) {
            self.state = STOP;
            self.stop();
        } else {
            self.state = state;
        }
        match self.kind {
            Kind::Video => {
                let time = if self.length > 0 { (self.pos as f64 * (self.video.length_time as f64 / self.length as f64)) as i64 } else { 0 };
                self.events.push_back(("onchange", vec![v_int(self.pos), v_int(time)]));
            }
            _ => self.events.push_back(("onchange", vec![v_int(self.pos)])),
        }
    }

    pub fn call(&mut self, method: &str, args: &[Value], read: &dyn Fn(&str) -> Result<Vec<u8>, String>) -> Option<Value> {
        let arg = |i: usize| args.get(i).cloned().unwrap_or(Value::Null);
        let k = self.kind;
        Some(match method {
            "open" => {
                let file = if k == Kind::CdAudio { String::new() } else { arg(0).to_string_val() };
                let bytes = if file.is_empty() { Err(String::new()) } else { read(&file) };
                v_int(self.open(&file, bytes))
            }
            "close" => {
                self.close();
                Value::Null
            }
            "play" => {
                self.play();
                Value::Null
            }
            "stop" => {
                self.stop();
                Value::Null
            }
            "pause" => {
                self.pause();
                Value::Null
            }
            "new" if k == Kind::Wave => {
                self.new_wave();
                Value::Null
            }
            "record" if k == Kind::Wave => {
                self.record();
                Value::Null
            }
            "delete" if k == Kind::Wave => {
                self.delete(arg(0).to_i64(), arg(1).to_i64());
                Value::Null
            }
            "show" if k == Kind::Video => Value::Null,
            "eject" if k == Kind::CdAudio => {
                self.close();
                self.cd.present = 0;
                Value::Null
            }
            _ => return None,
        })
    }
}

thread_local! {
    /// The object a call is for (its device is told by its id).
    static CURRENT_ID: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
}

/// Runs `f` as object `id`'s call.
pub fn with_id<R>(id: &str, f: impl FnOnce() -> R) -> R {
    CURRENT_ID.with(|c| *c.borrow_mut() = id.to_lowercase());
    f()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    thread_local! {
        static T: Cell<f64> = const { Cell::new(0.0) };
    }
    fn clock() -> f64 {
        T.with(Cell::get)
    }
    fn at(ms: f64) {
        T.with(|t| t.set(ms));
    }
    fn none(_: &str) -> Result<Vec<u8>, String> {
        Err("missing".into())
    }

    fn wav_bytes(ms: usize) -> Vec<u8> {
        Sound { rate: 11025, bits: 8, channels: 1, data: vec![128; 11025 * ms / 1000] }.wav_file()
    }

    #[test]
    fn wave_play_ticks_and_end() {
        directx::set_clock(clock);
        at(1000.0);
        let mut w = Media::new(Kind::Wave);
        assert_eq!(w.call("open", &[v_str("nosuch.wav")], &none), Some(v_int(0)));
        assert_eq!((w.error.as_str(), w.state), (FILE_NOT_FOUND, CLOSE));
        let b = wav_bytes(2000);
        assert_eq!(with_id("w", || w.open("a.wav", Ok(b.clone()))), 1);
        assert_eq!((w.length, w.state, w.bits, w.frequence, w.mode), (2000, STOP, 8, 11025, 1));
        // (opened again: MCI's alias in use, and the open one closed)
        assert_eq!(w.open("a.wav", Ok(b.clone())), 0);
        assert_eq!((w.error.as_str(), w.file_open), (ALIAS_IN_USE, false));
        w.open("a.wav", Ok(b));
        w.set("currentpos", &v_int(500));
        w.play();
        assert!(w.timer_enabled);
        at(1700.0);
        // (CurrentPos is the last tick's)
        assert_eq!(w.get("currentpos"), Some(v_int(500)));
        w.tick();
        assert_eq!((w.pos, w.state), (1200, PLAY));
        assert_eq!(w.events.pop_front().map(|e| e.1[0].to_i64()), Some(1200));
        w.set("currentpos", &v_int(0));
        assert_eq!(w.pos, 1200);
        at(3000.0);
        w.tick();
        assert_eq!((w.pos, w.state, w.timer_enabled), (0, STOP, false));
        assert_eq!(w.events.pop_front().map(|e| e.1[0].to_i64()), Some(0));
    }

    #[test]
    fn wave_record_save_delete() {
        directx::set_clock(clock);
        set_wave_input(WaveInput {
            start: |_, _, _, _| true,
            take: |_| test_tone(11025, 8, 1, 0, 11025, 440),
            stop: |_| {},
        });
        at(0.0);
        let mut w = Media::new(Kind::Wave);
        w.new_wave();
        assert_eq!((w.bits, w.frequence, w.mode, w.length), (8, 11025, 1, 0));
        w.length = 1000;
        w.record();
        assert_eq!(w.state, RECORD);
        at(1500.0);
        w.tick();
        assert_eq!((w.state, w.length), (STOP, 1000));
        let file = w.save().unwrap();
        assert_eq!(directx::parse_wav(&file).unwrap().data.len(), 11025);
        w.delete(0, 500);
        assert_eq!(w.length, 500);
    }

    #[test]
    fn cd_without_a_drive() {
        let mut c = Media::new(Kind::CdAudio);
        assert_eq!(c.call("open", &[], &none), Some(v_int(0)));
        assert_eq!((c.state, c.file_open, c.cd.present, c.error.as_str()), (CLOSE, false, 0, ""));
        c.set("currenttrack", &v_int(3));
        assert_eq!(c.cd.current_track, 0);
        c.play();
        assert!(!c.timer_enabled);
    }
}

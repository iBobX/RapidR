//! The media objects' devices on the desktop (rapidr_value::objects::media;
//! docs/io-media-plan.md §4–§5):
//!
//! - **QMIDI** plays its song on the system's first MIDI output through
//!   midir (MIT; ALSA on Linux — the library rodio already needs —,
//!   CoreMIDI on macOS, winmm on Windows, whose first output is Microsoft's
//!   GS Wavetable Synth: what MCI's sequencer played on). A thread sends
//!   the messages at their times; Volume scales the notes' velocities.
//!   No output port (macOS and Linux have no synthesizer of their own):
//!   RapidR's built-in one (`objects::synth`, procedural General MIDI
//!   instruments, no SoundFont) plays it on the sound device instead.
//! - **QWAVE** plays through QDXSOUND's device (sound.rs, rodio) and
//!   records from the default input through cpal (rodio's).
//!
//! Never under a test: `RAPIDR_TEST_MIDI` (set, even empty, by the test
//! runners) leaves QMIDI silent; `RAPIDR_TEST_WAVE_IN` gives QWAVE a
//! scripted input (`tone:440`, or empty: none) instead of a microphone;
//! the GUI tests have no sound device at all (sound.rs).

#[cfg(feature = "audio")]
use std::collections::HashMap;
#[cfg(feature = "audio")]
use std::rc::Rc;
#[cfg(feature = "audio")]
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
#[cfg(feature = "audio")]
use std::sync::{Arc, Mutex};

use rapidr_value::objects::media;
#[cfg(feature = "audio")]
use rapidr_value::objects::media::{MidiDevice, WaveInput};
#[cfg(feature = "audio")]
use rapidr_value::objects::midifile::Song;

/// Installs the media devices, once (the first QMIDI or QWAVE made).
pub fn install() {
    thread_local! {
        static DONE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }
    if DONE.with(|d| d.replace(true)) {
        return;
    }
    crate::sound::install_dx_device();
    #[cfg(feature = "audio")]
    if std::env::var_os("RAPIDR_TEST_MIDI").is_none() && !crate::sound::testing() {
        media::set_midi_device(MidiDevice { play: midi_play, stop: midi_stop, volume: midi_volume });
    }
    match std::env::var("RAPIDR_TEST_WAVE_IN") {
        Ok(script) => {
            if let Some(hz) = media::tone_script(&script) {
                media::install_tone_input(hz);
            }
        }
        #[cfg(feature = "audio")]
        Err(_) if !crate::sound::testing() => media::set_wave_input(WaveInput { start: mic_start, take: mic_take, stop: mic_stop }),
        Err(_) => {}
    }
}

// ----------------------------------------------------------------- MIDI --

#[cfg(feature = "audio")]
struct Playing {
    stop: Arc<AtomicBool>,
    /// Volume's gain ×1000.
    gain: Arc<AtomicU32>,
}

#[cfg(feature = "audio")]
thread_local! {
    static PLAYING: std::cell::RefCell<HashMap<String, Playing>> = std::cell::RefCell::new(HashMap::new());
}

/// Whether the system has a MIDI output to play songs on.
#[cfg(feature = "audio")]
fn has_midi_output() -> bool {
    midir::MidiOutput::new("RapidR").is_ok_and(|o| o.port_count() > 0)
}

#[cfg(feature = "audio")]
fn midi_play(id: &str, song: Rc<Song>, from_us: u64, gain: f32) {
    midi_stop(id);
    if !has_midi_output() {
        synth_play(id, &song, from_us, gain);
        return;
    }
    let stop = Arc::new(AtomicBool::new(false));
    let g = Arc::new(AtomicU32::new((gain.clamp(0.0, 1.0) * 1000.0) as u32));
    let events: Vec<(u64, Vec<u8>)> = song.events.iter().filter(|e| e.at_us >= from_us).map(|e| (e.at_us - from_us, e.bytes.clone())).collect();
    let (s2, g2) = (stop.clone(), g.clone());
    let started = std::thread::Builder::new().name("rapidr-midi".into()).spawn(move || {
        let Ok(out) = midir::MidiOutput::new("RapidR") else { return };
        let ports = out.ports();
        let Some(port) = ports.first() else { return };
        let Ok(mut conn) = out.connect(port, "RapidR QMIDI") else { return };
        let t0 = std::time::Instant::now();
        for (at, bytes) in events {
            loop {
                if s2.load(Ordering::Relaxed) {
                    break;
                }
                let now = t0.elapsed().as_micros() as u64;
                if now >= at {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_micros((at - now).min(20_000)));
            }
            if s2.load(Ordering::Relaxed) {
                break;
            }
            let mut m = bytes;
            // (Volume: a note's velocity scaled)
            if m.len() == 3 && m[0] & 0xF0 == 0x90 && m[2] > 0 {
                m[2] = ((f32::from(m[2]) * g2.load(Ordering::Relaxed) as f32 / 1000.0).round() as u8).clamp(1, 127);
            }
            let _ = conn.send(&m);
        }
        // (all notes off, every channel)
        for ch in 0..16u8 {
            let _ = conn.send(&[0xB0 | ch, 123, 0]);
            let _ = conn.send(&[0xB0 | ch, 120, 0]);
        }
    });
    if started.is_ok() {
        PLAYING.with(|p| p.borrow_mut().insert(id.to_string(), Playing { stop, gain: g }));
    }
}

#[cfg(feature = "audio")]
fn midi_stop(id: &str) {
    if let Some(p) = PLAYING.with(|p| p.borrow_mut().remove(id)) {
        p.stop.store(true, Ordering::Relaxed);
        crate::sound::stop_source(&synth_id(id));
    }
}

#[cfg(feature = "audio")]
fn midi_volume(id: &str, gain: f32) {
    PLAYING.with(|p| {
        if let Some(p) = p.borrow().get(id) {
            p.gain.store((gain.clamp(0.0, 1.0) * 1000.0) as u32, Ordering::Relaxed);
        }
    });
}

// ------------------------------------------------- built-in synthesizer --

/// The sound device's id for QMIDI `id`'s synthesizer.
#[cfg(feature = "audio")]
fn synth_id(id: &str) -> String {
    format!("{id}.synth")
}

/// The built-in synthesizer's output rate.
#[cfg(feature = "audio")]
const SYNTH_RATE: u32 = 44100;

/// QMIDI's song on the built-in synthesizer, rendered as the sound device
/// asks for it (rodio's thread).
#[cfg(feature = "audio")]
struct SynthSource {
    player: rapidr_value::objects::synth::Player,
    gain: Arc<AtomicU32>,
    applied: u32,
    buf: Vec<f32>,
    at: usize,
    playing: bool,
}

#[cfg(feature = "audio")]
impl Iterator for SynthSource {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if self.at >= self.buf.len() {
            if !self.playing {
                return None;
            }
            let g = self.gain.load(Ordering::Relaxed);
            if g != self.applied {
                self.applied = g;
                self.player.set_gain(g as f32 / 1000.0);
            }
            self.playing = self.player.render(&mut self.buf);
            self.at = 0;
        }
        self.at += 1;
        Some(self.buf[self.at - 1])
    }
}

#[cfg(feature = "audio")]
impl rodio::Source for SynthSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        2
    }
    fn sample_rate(&self) -> u32 {
        SYNTH_RATE
    }
    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }
}

#[cfg(feature = "audio")]
fn synth_play(id: &str, song: &Song, from_us: u64, gain: f32) {
    let events = song.events.iter().map(|e| (e.at_us, e.bytes.clone())).collect();
    let applied = (gain.clamp(0.0, 1.0) * 1000.0) as u32;
    let g = Arc::new(AtomicU32::new(applied));
    let player = rapidr_value::objects::synth::Player::new(events, SYNTH_RATE, from_us, gain.clamp(0.0, 1.0));
    // (about 12 ms a block)
    let source = SynthSource { player, gain: g.clone(), applied, buf: vec![0.0; 1024], at: 1024, playing: true };
    if crate::sound::play_source(&synth_id(id), source) {
        PLAYING.with(|p| p.borrow_mut().insert(id.to_string(), Playing { stop: Arc::new(AtomicBool::new(false)), gain: g }));
    }
}

// ----------------------------------------------------------- microphone --

#[cfg(feature = "audio")]
struct Capture {
    _stream: rodio::cpal::Stream,
    bytes: Arc<Mutex<Vec<u8>>>,
}

#[cfg(feature = "audio")]
thread_local! {
    static CAPTURES: std::cell::RefCell<HashMap<String, Capture>> = std::cell::RefCell::new(HashMap::new());
}

/// Records from the default input, converted to the wave's format (the
/// device's first channel, resampled by nearest frame).
#[cfg(feature = "audio")]
fn mic_start(id: &str, rate: u32, bits: u16, channels: u16) -> bool {
    use rodio::cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    let host = rodio::cpal::default_host();
    let Some(device) = host.default_input_device() else { return false };
    let Ok(config) = device.default_input_config() else { return false };
    let (dev_rate, dev_ch) = (config.sample_rate().0, usize::from(config.channels()));
    let bytes = Arc::new(Mutex::new(Vec::new()));
    let into = bytes.clone();
    let step = f64::from(dev_rate) / f64::from(rate.max(1));
    let mut phase = 0.0f64;
    let stream = device.build_input_stream(
        &config.into(),
        move |data: &[f32], _: &rodio::cpal::InputCallbackInfo| {
            let mut out = into.lock().unwrap();
            let frames = data.len() / dev_ch.max(1);
            while (phase as usize) < frames {
                let s = data[(phase as usize) * dev_ch.max(1)].clamp(-1.0, 1.0);
                for _ in 0..channels {
                    if bits == 8 {
                        out.push(((s * 127.0) + 128.0).round() as u8);
                    } else {
                        out.extend(((s * 32767.0).round() as i16).to_le_bytes());
                    }
                }
                phase += step;
            }
            phase -= frames as f64;
        },
        |_| {},
        None,
    );
    let Ok(stream) = stream else { return false };
    if stream.play().is_err() {
        return false;
    }
    CAPTURES.with(|c| c.borrow_mut().insert(id.to_string(), Capture { _stream: stream, bytes }));
    true
}

#[cfg(feature = "audio")]
fn mic_take(id: &str) -> Vec<u8> {
    CAPTURES.with(|c| c.borrow().get(id).map(|c| std::mem::take(&mut *c.bytes.lock().unwrap()))).unwrap_or_default()
}

#[cfg(feature = "audio")]
fn mic_stop(id: &str) {
    CAPTURES.with(|c| c.borrow_mut().remove(id));
}

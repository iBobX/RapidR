//! The built-in General MIDI synthesizer QMIDI plays its songs on where
//! there's no system one (macOS, Linux, a browser without Web MIDI).
//! RapidQ's QMIDI (D. Glodt's `QMidi.inc`) handed the file to Windows' MCI
//! sequencer, which played it on the GS Wavetable Synth's samples. We don't
//! bundle a SoundFont (its size, its licence), so every instrument here is
//! a recipe — oscillators, a filter, envelopes, noise — and nothing is
//! sampled: each program sounds like its family, not like Windows' set.
//!
//! - 16 channels (channel 10 the drum kit), the 128 GM programs as their
//!   16 families of 8 (`GM`), the GM percussion map 35–81 (`drum`).
//! - Plain data: no threads, no clock, no allocation while rendering, noise
//!   from an integer LCG. It runs on rodio's audio thread on the desktop
//!   and in the browser's audio callback (wasm), and the same messages
//!   render the same samples.
//! - `Player` plays a parsed song (`midifile::Song`'s events) from a
//!   position, as `media::MidiDevice::play` asks: what came before the
//!   position (programs, controllers, bends, sysex) is chased first.

use std::f32::consts::PI;

/// Frames between control updates (pitch, filter, LFOs, channel gains).
const BLOCK: usize = 32;
/// Voices sounding before one is stolen …
const POLY: usize = 48;
/// … and room for the stolen ones fading out.
const POOL: usize = 64;
const SINE_N: usize = 2048;
/// Below this an envelope is silence (-80 dB).
const SILENT: f32 = 1e-4;
/// Output level before the soft clipper (headroom for dense songs).
const MASTER: f32 = 0.5;
/// A steal or "all sound off" fades over this (no click).
const FAST: f32 = 0.004;

// ------------------------------------------------------------ recipes --

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Wave {
    Off,
    Sine,
    Tri,
    Saw,
    Square,
    /// (a quarter-width pulse: nasal, reedy)
    Pulse,
}
use Wave::*;

/// How a melodic program is made. Times are seconds (decay and release:
/// to -60 dB); `cutoff` counts harmonics of the note (0: no filter).
#[derive(Debug, Clone, Copy)]
struct Recipe {
    /// The main oscillator, `unison` copies of it spread over `detune` cents.
    wave: Wave,
    unison: u8,
    detune: f32,
    /// A second oscillator at `ratio2` × the note (`level2` 0: none).
    wave2: Wave,
    ratio2: f32,
    level2: f32,
    /// 2-operator FM on a sine main oscillator: a modulator at `fm_ratio`
    /// × the note, `fm_index` (radians) falling to a third over `fm_decay`
    /// (0: constant).
    fm_ratio: f32,
    fm_index: f32,
    fm_decay: f32,
    /// Noise (breath, air, effects) mixed in before the filter.
    noise: f32,
    /// A soft-clipping overdrive before the filter.
    drive: f32,
    attack: f32,
    decay: f32,
    /// 0: the note dies away by itself (plucked, struck).
    sustain: f32,
    release: f32,
    cutoff: f32,
    /// More harmonics at the onset, gone over `fdecay`.
    env_cutoff: f32,
    fdecay: f32,
    reso: f32,
    /// Vibrato in cents (fading in), at `vib_rate` Hz.
    vibrato: f32,
    vib_rate: f32,
    /// Amplitude wobble (0…1) at `trem_rate` Hz.
    tremolo: f32,
    trem_rate: f32,
    /// The note starts this many semitones sharp and falls into tune (drums).
    drop: f32,
    gain: f32,
}

/// The defaults the recipes start from: a plain sine organ.
const R: Recipe = Recipe {
    wave: Sine,
    unison: 1,
    detune: 0.0,
    wave2: Off,
    ratio2: 1.0,
    level2: 0.0,
    fm_ratio: 1.0,
    fm_index: 0.0,
    fm_decay: 0.0,
    noise: 0.0,
    drive: 0.0,
    attack: 0.005,
    decay: 1.0,
    sustain: 1.0,
    release: 0.15,
    cutoff: 0.0,
    env_cutoff: 0.0,
    fdecay: 0.3,
    reso: 0.0,
    vibrato: 0.0,
    vib_rate: 5.5,
    tremolo: 0.0,
    trem_rate: 5.0,
    drop: 0.0,
    gain: 1.0,
};

// (Piano 0–7)
const PIANO: Recipe = Recipe { wave: Saw, unison: 2, detune: 4.0, wave2: Sine, level2: 0.7, attack: 0.002, decay: 5.0, sustain: 0.0, release: 0.35, cutoff: 1.5, env_cutoff: 8.0, fdecay: 0.5, gain: 0.9, ..R };
const PIANO_BRIGHT: Recipe = Recipe { cutoff: 3.0, env_cutoff: 12.0, ..PIANO };
const PIANO_ELECTRIC: Recipe = Recipe { wave: Pulse, unison: 1, cutoff: 2.5, env_cutoff: 9.0, decay: 4.0, ..PIANO };
const HONKY_TONK: Recipe = Recipe { detune: 22.0, ..PIANO_BRIGHT };
const EPIANO1: Recipe = Recipe { fm_index: 1.8, fm_decay: 0.5, attack: 0.002, decay: 4.0, sustain: 0.0, release: 0.3, tremolo: 0.15, trem_rate: 4.5, gain: 1.1, ..R };
const EPIANO2: Recipe = Recipe { fm_index: 3.2, fm_decay: 0.25, wave2: Sine, ratio2: 2.0, level2: 0.25, ..EPIANO1 };
const HARPSICHORD: Recipe = Recipe { wave: Saw, wave2: Square, ratio2: 2.0, level2: 0.3, attack: 0.001, decay: 2.0, sustain: 0.0, release: 0.08, cutoff: 9.0, gain: 0.7, ..R };
const CLAVINET: Recipe = Recipe { wave: Pulse, attack: 0.001, decay: 1.5, sustain: 0.0, release: 0.05, cutoff: 3.0, env_cutoff: 10.0, fdecay: 0.08, reso: 0.5, gain: 0.8, ..R };
// (Chromatic percussion 8–15)
const CELESTA: Recipe = Recipe { fm_ratio: 4.0, fm_index: 1.2, fm_decay: 0.3, attack: 0.001, decay: 1.6, sustain: 0.0, release: 0.4, ..R };
const GLOCKENSPIEL: Recipe = Recipe { fm_ratio: 3.5, fm_index: 1.0, fm_decay: 0.2, wave2: Sine, ratio2: 2.76, level2: 0.3, decay: 1.4, ..CELESTA };
const MUSIC_BOX: Recipe = Recipe { fm_ratio: 5.0, fm_index: 1.0, fm_decay: 0.15, decay: 1.2, ..CELESTA };
const VIBRAPHONE: Recipe = Recipe { wave2: Sine, ratio2: 4.0, level2: 0.3, attack: 0.001, decay: 3.0, sustain: 0.0, release: 0.5, tremolo: 0.35, trem_rate: 5.5, ..R };
const MARIMBA: Recipe = Recipe { wave2: Sine, ratio2: 4.0, level2: 0.35, attack: 0.001, decay: 0.7, sustain: 0.0, release: 0.2, gain: 1.2, ..R };
const XYLOPHONE: Recipe = Recipe { ratio2: 3.0, level2: 0.4, decay: 0.45, ..MARIMBA };
const TUBULAR_BELLS: Recipe = Recipe { fm_ratio: 3.5, fm_index: 2.5, fm_decay: 1.0, wave2: Sine, ratio2: 2.0, level2: 0.3, attack: 0.001, decay: 5.0, sustain: 0.0, release: 1.0, ..R };
const DULCIMER: Recipe = Recipe { wave: Saw, unison: 2, detune: 6.0, attack: 0.001, decay: 2.0, sustain: 0.0, release: 0.3, cutoff: 3.0, env_cutoff: 10.0, fdecay: 0.3, gain: 0.8, ..R };
// (Organ 16–23)
const DRAWBAR_ORGAN: Recipe = Recipe { wave2: Sine, ratio2: 2.0, level2: 0.5, fm_ratio: 3.0, fm_index: 0.6, attack: 0.004, release: 0.06, tremolo: 0.1, trem_rate: 6.0, gain: 0.55, ..R };
const PERC_ORGAN: Recipe = Recipe { fm_index: 1.8, fm_decay: 0.25, ..DRAWBAR_ORGAN };
const ROCK_ORGAN: Recipe = Recipe { fm_ratio: 2.0, fm_index: 1.2, drive: 1.5, tremolo: 0.25, trem_rate: 6.5, ..DRAWBAR_ORGAN };
const CHURCH_ORGAN: Recipe = Recipe { wave: Saw, unison: 2, detune: 3.0, wave2: Square, ratio2: 0.5, level2: 0.5, attack: 0.08, release: 0.5, cutoff: 5.0, gain: 0.6, ..R };
const REED_ORGAN: Recipe = Recipe { wave: Pulse, noise: 0.02, attack: 0.04, release: 0.15, cutoff: 4.0, gain: 0.7, ..R };
const ACCORDION: Recipe = Recipe { wave: Saw, unison: 2, detune: 12.0, attack: 0.03, release: 0.1, cutoff: 4.0, reso: 0.2, gain: 0.7, ..R };
const HARMONICA: Recipe = Recipe { wave: Square, noise: 0.04, attack: 0.03, release: 0.1, cutoff: 3.0, reso: 0.3, vibrato: 12.0, gain: 0.6, ..R };
const TANGO_ACCORDION: Recipe = Recipe { wave: Pulse, detune: 7.0, ..ACCORDION };
// (Guitar 24–31)
const NYLON_GUITAR: Recipe = Recipe { wave: Tri, wave2: Saw, level2: 0.3, attack: 0.002, decay: 2.2, sustain: 0.0, release: 0.2, cutoff: 2.5, env_cutoff: 6.0, fdecay: 0.15, ..R };
const STEEL_GUITAR: Recipe = Recipe { wave: Saw, unison: 2, detune: 3.0, attack: 0.001, decay: 2.8, sustain: 0.0, release: 0.2, cutoff: 3.0, env_cutoff: 12.0, fdecay: 0.25, gain: 0.8, ..R };
const JAZZ_GUITAR: Recipe = Recipe { wave: Tri, wave2: Sine, level2: 0.4, attack: 0.002, decay: 2.4, sustain: 0.0, release: 0.2, cutoff: 2.0, env_cutoff: 3.0, ..R };
const CLEAN_GUITAR: Recipe = Recipe { wave: Pulse, wave2: Sine, level2: 0.4, attack: 0.002, decay: 3.0, sustain: 0.0, release: 0.2, cutoff: 3.0, env_cutoff: 6.0, gain: 0.8, ..R };
const MUTED_GUITAR: Recipe = Recipe { wave: Saw, attack: 0.001, decay: 0.25, sustain: 0.0, release: 0.05, cutoff: 2.0, env_cutoff: 4.0, fdecay: 0.05, ..R };
const OVERDRIVE_GUITAR: Recipe = Recipe { wave: Saw, unison: 2, detune: 6.0, drive: 3.0, attack: 0.003, decay: 3.0, sustain: 0.5, release: 0.15, cutoff: 5.0, reso: 0.2, gain: 0.55, ..R };
const DISTORTION_GUITAR: Recipe = Recipe { detune: 10.0, drive: 8.0, wave2: Saw, ratio2: 0.5, level2: 0.5, sustain: 0.7, ..OVERDRIVE_GUITAR };
const GUITAR_HARMONICS: Recipe = Recipe { wave2: Sine, ratio2: 2.0, level2: 0.5, fm_ratio: 2.0, fm_index: 0.4, attack: 0.002, decay: 1.6, sustain: 0.0, release: 0.3, ..R };
// (Bass 32–39)
const ACOUSTIC_BASS: Recipe = Recipe { wave: Tri, wave2: Sine, level2: 0.5, attack: 0.002, decay: 1.6, sustain: 0.0, release: 0.1, cutoff: 2.0, env_cutoff: 3.0, fdecay: 0.1, gain: 1.2, ..R };
const FINGER_BASS: Recipe = Recipe { wave: Saw, wave2: Sine, level2: 0.6, attack: 0.002, decay: 2.0, sustain: 0.0, release: 0.1, cutoff: 1.5, env_cutoff: 4.0, fdecay: 0.12, gain: 1.1, ..R };
const PICK_BASS: Recipe = Recipe { cutoff: 2.5, env_cutoff: 10.0, fdecay: 0.08, decay: 1.8, ..FINGER_BASS };
const FRETLESS_BASS: Recipe = Recipe { wave: Tri, wave2: Saw, level2: 0.3, attack: 0.02, decay: 2.5, sustain: 0.4, release: 0.1, cutoff: 2.0, vibrato: 6.0, gain: 1.2, ..R };
const SLAP_BASS: Recipe = Recipe { wave: Saw, wave2: Sine, level2: 0.6, attack: 0.001, decay: 1.0, sustain: 0.0, release: 0.08, cutoff: 3.0, env_cutoff: 18.0, fdecay: 0.04, reso: 0.3, ..R };
const SLAP_BASS2: Recipe = Recipe { wave: Pulse, reso: 0.45, ..SLAP_BASS };
const SYNTH_BASS1: Recipe = Recipe { wave: Saw, attack: 0.002, decay: 1.0, sustain: 0.5, release: 0.08, cutoff: 1.5, env_cutoff: 10.0, fdecay: 0.15, reso: 0.6, gain: 0.9, ..R };
const SYNTH_BASS2: Recipe = Recipe { wave: Square, unison: 2, detune: 6.0, decay: 0.8, sustain: 0.6, env_cutoff: 7.0, fdecay: 0.2, reso: 0.5, gain: 0.7, ..SYNTH_BASS1 };
// (Strings 40–47)
const VIOLIN: Recipe = Recipe { wave: Saw, attack: 0.06, decay: 1.0, sustain: 0.9, release: 0.25, cutoff: 5.0, reso: 0.1, vibrato: 14.0, vib_rate: 5.6, gain: 0.8, ..R };
const VIOLA: Recipe = Recipe { cutoff: 4.0, ..VIOLIN };
const CELLO: Recipe = Recipe { cutoff: 3.5, vibrato: 12.0, ..VIOLIN };
const CONTRABASS: Recipe = Recipe { cutoff: 2.5, vibrato: 8.0, gain: 1.0, ..VIOLIN };
const TREMOLO_STRINGS: Recipe = Recipe { unison: 2, detune: 8.0, attack: 0.05, cutoff: 4.0, tremolo: 0.6, trem_rate: 9.0, vibrato: 6.0, ..VIOLIN };
const PIZZICATO: Recipe = Recipe { wave: Saw, wave2: Tri, level2: 0.5, attack: 0.002, decay: 0.45, sustain: 0.0, release: 0.1, cutoff: 1.8, env_cutoff: 5.0, fdecay: 0.08, ..R };
const HARP: Recipe = Recipe { wave: Tri, wave2: Sine, ratio2: 2.0, level2: 0.2, attack: 0.002, decay: 2.5, sustain: 0.0, release: 0.5, cutoff: 4.0, env_cutoff: 6.0, fdecay: 0.15, ..R };
const TIMPANI: Recipe = Recipe { wave2: Sine, ratio2: 1.5, level2: 0.3, noise: 0.15, attack: 0.002, decay: 1.8, sustain: 0.0, release: 0.6, cutoff: 3.0, drop: 0.6, gain: 1.3, ..R };
// (Ensemble 48–55)
const STRINGS: Recipe = Recipe { wave: Saw, unison: 3, detune: 14.0, attack: 0.15, decay: 1.0, sustain: 0.9, release: 0.45, cutoff: 4.0, vibrato: 6.0, gain: 0.8, ..R };
const SLOW_STRINGS: Recipe = Recipe { attack: 0.45, release: 0.7, ..STRINGS };
const SYNTH_STRINGS1: Recipe = Recipe { detune: 20.0, attack: 0.1, release: 0.5, cutoff: 5.0, reso: 0.2, vibrato: 0.0, ..STRINGS };
const SYNTH_STRINGS2: Recipe = Recipe { wave: Pulse, detune: 16.0, attack: 0.2, release: 0.6, cutoff: 3.0, env_cutoff: 3.0, fdecay: 1.0, vibrato: 0.0, ..STRINGS };
const CHOIR_AAHS: Recipe = Recipe { wave: Saw, unison: 3, detune: 12.0, noise: 0.03, attack: 0.25, release: 0.5, cutoff: 1.8, reso: 0.55, vibrato: 10.0, vib_rate: 5.0, gain: 0.9, ..R };
const VOICE_OOHS: Recipe = Recipe { wave: Tri, unison: 2, detune: 10.0, attack: 0.2, release: 0.4, cutoff: 1.5, reso: 0.3, vibrato: 8.0, ..R };
const SYNTH_VOICE: Recipe = Recipe { unison: 3, detune: 12.0, wave2: Tri, ratio2: 2.0, level2: 0.3, attack: 0.12, release: 0.4, vibrato: 12.0, ..R };
const ORCHESTRA_HIT: Recipe = Recipe { wave: Saw, unison: 3, detune: 25.0, wave2: Saw, ratio2: 0.5, level2: 0.7, noise: 0.15, attack: 0.002, decay: 0.6, sustain: 0.0, release: 0.2, cutoff: 6.0, env_cutoff: 8.0, fdecay: 0.1, ..R };
// (Brass 56–63)
const TRUMPET: Recipe = Recipe { wave: Saw, attack: 0.03, decay: 0.5, sustain: 0.8, release: 0.12, cutoff: 3.0, env_cutoff: 5.0, fdecay: 0.12, vibrato: 8.0, gain: 0.85, ..R };
const TROMBONE: Recipe = Recipe { attack: 0.04, cutoff: 2.2, env_cutoff: 4.0, ..TRUMPET };
const TUBA: Recipe = Recipe { attack: 0.05, cutoff: 1.6, env_cutoff: 3.0, wave2: Sine, level2: 0.5, gain: 1.0, ..TRUMPET };
const MUTED_TRUMPET: Recipe = Recipe { wave: Pulse, attack: 0.02, cutoff: 3.0, env_cutoff: 2.0, reso: 0.55, gain: 0.8, ..TRUMPET };
const FRENCH_HORN: Recipe = Recipe { attack: 0.07, release: 0.25, cutoff: 1.6, env_cutoff: 2.0, fdecay: 0.2, wave2: Sine, level2: 0.4, ..TRUMPET };
const BRASS_SECTION: Recipe = Recipe { unison: 3, detune: 8.0, attack: 0.04, sustain: 0.85, release: 0.18, cutoff: 3.0, env_cutoff: 6.0, fdecay: 0.15, ..TRUMPET };
const SYNTH_BRASS1: Recipe = Recipe { wave: Saw, unison: 2, detune: 10.0, attack: 0.01, decay: 0.6, sustain: 0.8, release: 0.15, cutoff: 1.5, env_cutoff: 10.0, fdecay: 0.35, reso: 0.3, gain: 0.8, ..R };
const SYNTH_BRASS2: Recipe = Recipe { wave: Square, detune: 12.0, attack: 0.06, cutoff: 2.0, env_cutoff: 6.0, fdecay: 0.5, gain: 0.6, ..SYNTH_BRASS1 };
// (Reed 64–71)
const ALTO_SAX: Recipe = Recipe { wave: Saw, wave2: Square, level2: 0.4, noise: 0.05, attack: 0.025, decay: 0.5, sustain: 0.85, release: 0.1, cutoff: 3.5, env_cutoff: 3.0, fdecay: 0.1, reso: 0.25, vibrato: 10.0, vib_rate: 5.0, gain: 1.1, ..R };
const SOPRANO_SAX: Recipe = Recipe { cutoff: 4.5, ..ALTO_SAX };
const TENOR_SAX: Recipe = Recipe { cutoff: 3.0, ..ALTO_SAX };
const BARITONE_SAX: Recipe = Recipe { cutoff: 2.5, ..ALTO_SAX };
const OBOE: Recipe = Recipe { wave: Pulse, attack: 0.03, release: 0.1, cutoff: 6.0, reso: 0.35, vibrato: 8.0, gain: 0.75, ..R };
const ENGLISH_HORN: Recipe = Recipe { cutoff: 4.5, ..OBOE };
const BASSOON: Recipe = Recipe { attack: 0.04, cutoff: 3.0, reso: 0.3, vibrato: 4.0, ..OBOE };
const CLARINET: Recipe = Recipe { wave: Square, attack: 0.03, release: 0.1, cutoff: 4.0, vibrato: 4.0, gain: 0.6, ..R };
// (Pipe 72–79)
const FLUTE: Recipe = Recipe { wave2: Tri, ratio2: 2.0, level2: 0.12, noise: 0.12, attack: 0.06, decay: 0.5, sustain: 0.9, release: 0.12, cutoff: 4.0, vibrato: 10.0, vib_rate: 5.0, gain: 0.7, ..R };
const RECORDER: Recipe = Recipe { wave: Tri, noise: 0.06, attack: 0.03, release: 0.08, cutoff: 4.0, vibrato: 4.0, ..R };
const PAN_FLUTE: Recipe = Recipe { noise: 0.35, attack: 0.05, release: 0.15, cutoff: 2.0, vibrato: 8.0, ..R };
const BLOWN_BOTTLE: Recipe = Recipe { noise: 0.5, attack: 0.08, release: 0.15, cutoff: 1.3, reso: 0.6, ..R };
const SHAKUHACHI: Recipe = Recipe { noise: 0.3, attack: 0.07, release: 0.15, cutoff: 2.5, vibrato: 20.0, vib_rate: 4.5, ..R };
const WHISTLE: Recipe = Recipe { attack: 0.03, release: 0.08, vibrato: 14.0, vib_rate: 6.0, gain: 0.9, ..R };
const OCARINA: Recipe = Recipe { wave2: Sine, ratio2: 2.0, level2: 0.05, noise: 0.03, attack: 0.03, release: 0.1, cutoff: 3.0, vibrato: 6.0, ..R };
// (Synth lead 80–87)
const SQUARE_LEAD: Recipe = Recipe { wave: Square, attack: 0.005, release: 0.08, cutoff: 8.0, gain: 0.6, ..R };
const SAW_LEAD: Recipe = Recipe { wave: Saw, unison: 2, detune: 6.0, attack: 0.005, release: 0.08, cutoff: 8.0, gain: 0.7, ..R };
const CALLIOPE: Recipe = Recipe { wave: Tri, wave2: Sine, ratio2: 2.0, level2: 0.3, noise: 0.15, attack: 0.02, release: 0.1, cutoff: 4.0, vibrato: 8.0, ..R };
const CHIFF_LEAD: Recipe = Recipe { wave: Tri, noise: 0.1, attack: 0.005, release: 0.1, cutoff: 2.0, env_cutoff: 8.0, fdecay: 0.05, ..R };
const CHARANG: Recipe = Recipe { wave: Saw, drive: 4.0, attack: 0.003, decay: 1.5, sustain: 0.5, release: 0.1, cutoff: 4.0, env_cutoff: 6.0, fdecay: 0.2, gain: 0.55, ..R };
const LEAD_VOICE: Recipe = Recipe { wave: Saw, unison: 2, detune: 8.0, attack: 0.05, release: 0.2, cutoff: 1.8, reso: 0.6, vibrato: 10.0, gain: 0.8, ..R };
const FIFTHS_LEAD: Recipe = Recipe { wave: Saw, wave2: Saw, ratio2: 1.5, level2: 0.7, attack: 0.005, release: 0.1, cutoff: 6.0, gain: 0.6, ..R };
const BASS_LEAD: Recipe = Recipe { wave: Saw, wave2: Saw, ratio2: 0.5, level2: 0.8, attack: 0.003, release: 0.1, cutoff: 5.0, env_cutoff: 6.0, fdecay: 0.2, gain: 0.6, ..R };
// (Synth pad 88–95)
const NEW_AGE_PAD: Recipe = Recipe { unison: 2, detune: 8.0, wave2: Tri, level2: 0.5, fm_ratio: 2.0, fm_index: 2.0, fm_decay: 1.5, attack: 0.1, decay: 3.0, sustain: 0.6, release: 1.0, ..R };
const WARM_PAD: Recipe = Recipe { wave: Saw, unison: 3, detune: 10.0, attack: 0.35, release: 0.9, cutoff: 1.8, gain: 0.8, ..R };
const POLYSYNTH: Recipe = Recipe { wave: Saw, unison: 2, detune: 12.0, attack: 0.01, decay: 1.5, sustain: 0.6, release: 0.4, cutoff: 2.0, env_cutoff: 6.0, fdecay: 0.4, gain: 0.8, ..R };
const CHOIR_PAD: Recipe = Recipe { wave: Tri, unison: 3, detune: 14.0, noise: 0.02, attack: 0.5, release: 1.0, cutoff: 2.0, reso: 0.4, vibrato: 6.0, ..R };
const BOWED_PAD: Recipe = Recipe { wave: Saw, unison: 2, detune: 6.0, attack: 0.4, release: 0.8, cutoff: 1.5, env_cutoff: 3.0, fdecay: 1.0, vibrato: 8.0, gain: 0.8, ..R };
const METALLIC_PAD: Recipe = Recipe { unison: 2, detune: 6.0, fm_ratio: 1.41, fm_index: 1.8, attack: 0.2, release: 1.0, ..R };
const HALO_PAD: Recipe = Recipe { wave: Tri, unison: 3, detune: 18.0, wave2: Sine, ratio2: 2.0, level2: 0.3, noise: 0.04, attack: 0.6, release: 1.2, cutoff: 3.0, ..R };
const SWEEP_PAD: Recipe = Recipe { wave: Saw, unison: 3, detune: 12.0, attack: 0.3, release: 1.0, cutoff: 0.8, env_cutoff: 10.0, fdecay: 2.5, reso: 0.6, gain: 0.8, ..R };
// (Synth effects 96–103)
const RAIN: Recipe = Recipe { fm_ratio: 2.7, fm_index: 1.5, fm_decay: 0.3, noise: 0.2, attack: 0.002, decay: 2.5, sustain: 0.3, release: 1.0, cutoff: 6.0, ..R };
const SOUNDTRACK: Recipe = Recipe { wave: Saw, unison: 3, detune: 15.0, attack: 0.8, release: 1.5, cutoff: 2.0, env_cutoff: 4.0, fdecay: 2.0, gain: 0.8, ..R };
const CRYSTAL: Recipe = Recipe { fm_ratio: 3.5, fm_index: 2.0, fm_decay: 0.5, wave2: Sine, ratio2: 5.0, level2: 0.2, attack: 0.002, decay: 2.5, sustain: 0.2, release: 1.0, ..R };
const ATMOSPHERE: Recipe = Recipe { fm_index: 1.0, fm_decay: 1.0, wave2: Saw, level2: 0.3, attack: 0.05, decay: 2.0, sustain: 0.5, release: 1.0, cutoff: 3.0, ..R };
const BRIGHTNESS: Recipe = Recipe { wave: Saw, unison: 3, detune: 14.0, attack: 0.2, release: 1.0, cutoff: 10.0, gain: 0.6, ..R };
const GOBLINS: Recipe = Recipe { wave: Square, attack: 0.4, release: 1.0, cutoff: 1.5, reso: 0.7, vibrato: 40.0, vib_rate: 1.5, gain: 0.6, ..R };
const ECHOES: Recipe = Recipe { wave2: Tri, ratio2: 2.0, level2: 0.4, attack: 0.1, decay: 2.0, sustain: 0.5, release: 1.5, tremolo: 0.6, trem_rate: 3.0, ..R };
const SCI_FI: Recipe = Recipe { fm_ratio: 0.5, fm_index: 4.0, fm_decay: 2.0, attack: 0.1, release: 1.0, vibrato: 30.0, vib_rate: 3.0, ..R };
// (Ethnic 104–111)
const SITAR: Recipe = Recipe { wave: Saw, wave2: Saw, ratio2: 2.0, level2: 0.2, attack: 0.002, decay: 3.0, sustain: 0.0, release: 0.3, cutoff: 6.0, env_cutoff: 8.0, fdecay: 0.5, reso: 0.6, drop: 0.3, gain: 0.7, ..R };
const BANJO: Recipe = Recipe { wave: Pulse, attack: 0.001, decay: 0.9, sustain: 0.0, release: 0.1, cutoff: 5.0, env_cutoff: 10.0, fdecay: 0.1, gain: 0.8, ..R };
const SHAMISEN: Recipe = Recipe { wave: Saw, attack: 0.001, decay: 0.8, sustain: 0.0, release: 0.1, cutoff: 4.0, env_cutoff: 10.0, fdecay: 0.08, reso: 0.3, gain: 0.8, ..R };
const KOTO: Recipe = Recipe { wave: Tri, wave2: Saw, level2: 0.4, attack: 0.001, decay: 1.6, sustain: 0.0, release: 0.2, cutoff: 3.0, env_cutoff: 6.0, fdecay: 0.2, ..R };
const KALIMBA: Recipe = Recipe { fm_ratio: 5.0, fm_index: 1.0, fm_decay: 0.05, attack: 0.001, decay: 0.8, sustain: 0.0, release: 0.2, gain: 1.2, ..R };
const BAGPIPE: Recipe = Recipe { wave: Saw, unison: 2, detune: 5.0, wave2: Saw, ratio2: 0.5, level2: 0.6, attack: 0.05, release: 0.1, cutoff: 5.0, reso: 0.3, gain: 0.6, ..R };
const FIDDLE: Recipe = Recipe { cutoff: 6.0, vibrato: 20.0, ..VIOLIN };
const SHANAI: Recipe = Recipe { wave: Pulse, attack: 0.03, release: 0.1, cutoff: 7.0, reso: 0.4, vibrato: 18.0, gain: 0.7, ..R };
// (Percussive 112–119)
const TINKLE_BELL: Recipe = Recipe { fm_ratio: 3.5, fm_index: 1.5, fm_decay: 0.2, attack: 0.001, decay: 0.9, sustain: 0.0, release: 0.3, ..R };
const AGOGO: Recipe = Recipe { fm_ratio: 1.48, fm_index: 2.0, fm_decay: 0.1, decay: 0.5, ..TINKLE_BELL };
const STEEL_DRUMS: Recipe = Recipe { fm_ratio: 2.0, fm_index: 1.2, fm_decay: 0.15, wave2: Sine, ratio2: 3.0, level2: 0.2, decay: 1.2, ..TINKLE_BELL };
const WOODBLOCK: Recipe = Recipe { wave2: Sine, ratio2: 2.5, level2: 0.3, attack: 0.001, decay: 0.12, sustain: 0.0, release: 0.05, drop: 2.0, gain: 1.2, ..R };
const TAIKO: Recipe = Recipe { noise: 0.3, attack: 0.001, decay: 0.9, sustain: 0.0, release: 0.3, cutoff: 2.0, drop: 4.0, gain: 1.4, ..R };
const MELODIC_TOM: Recipe = Recipe { noise: 0.1, attack: 0.001, decay: 0.6, sustain: 0.0, release: 0.2, cutoff: 3.0, drop: 5.0, gain: 1.3, ..R };
const SYNTH_DRUM: Recipe = Recipe { wave: Tri, attack: 0.001, decay: 0.5, sustain: 0.0, release: 0.2, drop: 12.0, gain: 1.2, ..R };
const REVERSE_CYMBAL: Recipe = Recipe { wave: Off, noise: 1.0, attack: 1.5, release: 0.2, cutoff: 20.0, gain: 0.45, ..R };
// (Sound effects 120–127)
const FRET_NOISE: Recipe = Recipe { wave: Saw, noise: 0.6, attack: 0.001, decay: 0.15, sustain: 0.0, release: 0.05, cutoff: 6.0, drop: 1.0, gain: 0.7, ..R };
const BREATH_NOISE: Recipe = Recipe { wave: Off, noise: 1.0, attack: 0.08, release: 0.2, cutoff: 3.0, reso: 0.3, gain: 0.8, ..R };
const SEASHORE: Recipe = Recipe { wave: Off, noise: 1.0, attack: 1.2, release: 2.0, cutoff: 3.0, tremolo: 0.8, trem_rate: 0.12, gain: 0.7, ..R };
const BIRD_TWEET: Recipe = Recipe { wave2: Sine, ratio2: 2.0, level2: 0.1, attack: 0.02, decay: 0.3, sustain: 0.6, release: 0.1, vibrato: 250.0, vib_rate: 14.0, ..R };
const TELEPHONE: Recipe = Recipe { wave: Square, cutoff: 6.0, tremolo: 1.0, trem_rate: 16.0, gain: 0.5, ..R };
const HELICOPTER: Recipe = Recipe { wave: Off, noise: 1.0, cutoff: 2.0, reso: 0.3, tremolo: 0.9, trem_rate: 11.0, gain: 0.9, ..R };
const APPLAUSE: Recipe = Recipe { wave: Off, noise: 1.0, attack: 0.4, release: 0.8, cutoff: 10.0, tremolo: 0.5, trem_rate: 7.0, gain: 0.5, ..R };
const GUNSHOT: Recipe = Recipe { wave: Off, noise: 1.0, attack: 0.001, decay: 0.6, sustain: 0.0, release: 0.2, cutoff: 8.0, env_cutoff: 20.0, fdecay: 0.05, gain: 1.2, ..R };

/// The General MIDI programs, in their families of 8.
#[rustfmt::skip]
const GM: [Recipe; 128] = [
    // Piano
    PIANO, PIANO_BRIGHT, PIANO_ELECTRIC, HONKY_TONK, EPIANO1, EPIANO2, HARPSICHORD, CLAVINET,
    // Chromatic percussion
    CELESTA, GLOCKENSPIEL, MUSIC_BOX, VIBRAPHONE, MARIMBA, XYLOPHONE, TUBULAR_BELLS, DULCIMER,
    // Organ
    DRAWBAR_ORGAN, PERC_ORGAN, ROCK_ORGAN, CHURCH_ORGAN, REED_ORGAN, ACCORDION, HARMONICA, TANGO_ACCORDION,
    // Guitar
    NYLON_GUITAR, STEEL_GUITAR, JAZZ_GUITAR, CLEAN_GUITAR, MUTED_GUITAR, OVERDRIVE_GUITAR, DISTORTION_GUITAR, GUITAR_HARMONICS,
    // Bass
    ACOUSTIC_BASS, FINGER_BASS, PICK_BASS, FRETLESS_BASS, SLAP_BASS, SLAP_BASS2, SYNTH_BASS1, SYNTH_BASS2,
    // Strings
    VIOLIN, VIOLA, CELLO, CONTRABASS, TREMOLO_STRINGS, PIZZICATO, HARP, TIMPANI,
    // Ensemble
    STRINGS, SLOW_STRINGS, SYNTH_STRINGS1, SYNTH_STRINGS2, CHOIR_AAHS, VOICE_OOHS, SYNTH_VOICE, ORCHESTRA_HIT,
    // Brass
    TRUMPET, TROMBONE, TUBA, MUTED_TRUMPET, FRENCH_HORN, BRASS_SECTION, SYNTH_BRASS1, SYNTH_BRASS2,
    // Reed
    SOPRANO_SAX, ALTO_SAX, TENOR_SAX, BARITONE_SAX, OBOE, ENGLISH_HORN, BASSOON, CLARINET,
    // Pipe
    FLUTE, FLUTE, RECORDER, PAN_FLUTE, BLOWN_BOTTLE, SHAKUHACHI, WHISTLE, OCARINA,
    // Synth lead
    SQUARE_LEAD, SAW_LEAD, CALLIOPE, CHIFF_LEAD, CHARANG, LEAD_VOICE, FIFTHS_LEAD, BASS_LEAD,
    // Synth pad
    NEW_AGE_PAD, WARM_PAD, POLYSYNTH, CHOIR_PAD, BOWED_PAD, METALLIC_PAD, HALO_PAD, SWEEP_PAD,
    // Synth effects
    RAIN, SOUNDTRACK, CRYSTAL, ATMOSPHERE, BRIGHTNESS, GOBLINS, ECHOES, SCI_FI,
    // Ethnic
    SITAR, BANJO, SHAMISEN, KOTO, KALIMBA, BAGPIPE, FIDDLE, SHANAI,
    // Percussive
    TINKLE_BELL, AGOGO, STEEL_DRUMS, WOODBLOCK, TAIKO, MELODIC_TOM, SYNTH_DRUM, REVERSE_CYMBAL,
    // Sound effects
    FRET_NOISE, BREATH_NOISE, SEASHORE, BIRD_TWEET, TELEPHONE, HELICOPTER, APPLAUSE, GUNSHOT,
];

/// A percussion sound: a sine tone falling from `tone` to `tone_end` Hz
/// (over `drop` s; `tone2` a second, fixed one), and noise plus a metallic
/// cluster of square waves through a high-pass at `hp` and a low-pass at
/// `lp`, each with its own decay (to -60 dB). `rattle` Hz chops the noise
/// for its first `rattle_len` s (claps, guiro).
#[derive(Debug, Clone, Copy)]
struct Drum {
    tone: f32,
    tone_end: f32,
    drop: f32,
    tone2: f32,
    tone_decay: f32,
    tone_lvl: f32,
    noise: f32,
    metal: f32,
    noise_decay: f32,
    hp: f32,
    lp: f32,
    rattle: f32,
    rattle_len: f32,
    /// Where the kit places it (-1 left … 1 right).
    pan: f32,
    gain: f32,
}

const D: Drum = Drum { tone: 0.0, tone_end: 0.0, drop: 0.03, tone2: 0.0, tone_decay: 0.1, tone_lvl: 0.0, noise: 0.0, metal: 0.0, noise_decay: 0.1, hp: 20.0, lp: 16000.0, rattle: 0.0, rattle_len: 0.0, pan: 0.0, gain: 1.0 };

const KICK: Drum = Drum { tone: 140.0, tone_end: 50.0, drop: 0.03, tone_decay: 0.45, tone_lvl: 1.0, noise: 0.15, noise_decay: 0.015, hp: 100.0, lp: 4000.0, gain: 1.3, ..D };
const SNARE: Drum = Drum { tone: 220.0, tone_end: 180.0, drop: 0.02, tone_decay: 0.15, tone_lvl: 0.6, noise: 0.9, noise_decay: 0.25, hp: 1200.0, lp: 9000.0, ..D };
const TOM: Drum = Drum { drop: 0.1, tone_decay: 0.6, tone_lvl: 1.0, noise: 0.1, noise_decay: 0.05, hp: 200.0, lp: 3000.0, gain: 1.1, ..D };
const HIHAT: Drum = Drum { metal: 0.6, noise: 0.5, noise_decay: 0.06, hp: 7000.0, lp: 16000.0, pan: 0.3, gain: 0.7, ..D };
const CYMBAL: Drum = Drum { metal: 0.5, noise: 0.8, noise_decay: 1.8, hp: 4000.0, lp: 14000.0, pan: -0.3, gain: 0.6, ..D };
const HAND: Drum = Drum { drop: 0.01, tone_decay: 0.2, tone_lvl: 1.0, noise: 0.08, noise_decay: 0.02, hp: 800.0, lp: 6000.0, pan: 0.2, ..D };
const BLOCK_HIT: Drum = Drum { drop: 0.005, tone_decay: 0.07, tone_lvl: 1.0, pan: -0.2, ..D };
const SHAKER: Drum = Drum { noise: 0.6, noise_decay: 0.08, hp: 6000.0, lp: 14000.0, pan: 0.4, gain: 0.6, ..D };
/// (an unmapped note: a short click)
const CLICK: Drum = Drum { noise: 0.5, noise_decay: 0.02, hp: 1000.0, gain: 0.5, ..D };

/// The GM percussion map (one kit; notes 35–81).
fn drum(note: u8) -> Drum {
    match note {
        35 => Drum { tone: 110.0, tone_end: 45.0, drop: 0.04, tone_decay: 0.5, ..KICK },
        36 => KICK,
        37 => Drum { tone: 1700.0, tone_decay: 0.03, tone_lvl: 0.5, noise: 0.5, noise_decay: 0.03, hp: 1500.0, lp: 8000.0, ..D },
        38 => SNARE,
        39 => Drum { noise: 1.0, noise_decay: 0.2, hp: 900.0, lp: 5000.0, rattle: 100.0, rattle_len: 0.03, ..D },
        40 => Drum { tone: 250.0, tone_end: 200.0, tone_decay: 0.1, noise: 1.0, noise_decay: 0.2, hp: 2000.0, lp: 12000.0, ..SNARE },
        41 => Drum { tone: 120.0, tone_end: 80.0, pan: 0.3, ..TOM },
        42 => HIHAT,
        43 => Drum { tone: 140.0, tone_end: 95.0, pan: 0.2, ..TOM },
        44 => Drum { metal: 0.5, noise: 0.3, noise_decay: 0.05, hp: 6000.0, ..HIHAT },
        45 => Drum { tone: 160.0, tone_end: 110.0, pan: 0.1, ..TOM },
        46 => Drum { noise_decay: 0.6, ..HIHAT },
        47 => Drum { tone: 185.0, tone_end: 130.0, pan: 0.0, ..TOM },
        48 => Drum { tone: 210.0, tone_end: 150.0, pan: -0.1, ..TOM },
        49 => CYMBAL,
        50 => Drum { tone: 240.0, tone_end: 175.0, pan: -0.2, ..TOM },
        51 => Drum { metal: 0.7, noise: 0.25, noise_decay: 1.4, hp: 5000.0, pan: 0.35, ..CYMBAL },
        52 => Drum { metal: 0.8, noise: 0.8, noise_decay: 1.2, hp: 2500.0, lp: 8000.0, ..CYMBAL },
        53 => Drum { tone: 900.0, tone_decay: 0.8, tone_lvl: 0.4, metal: 0.6, noise: 0.1, noise_decay: 0.9, hp: 3000.0, pan: 0.35, ..CYMBAL },
        54 => Drum { noise: 0.7, metal: 0.4, noise_decay: 0.2, hp: 6000.0, rattle: 30.0, rattle_len: 0.1, ..SHAKER },
        55 => Drum { noise_decay: 0.8, hp: 5000.0, pan: 0.2, ..CYMBAL },
        56 => Drum { tone: 560.0, tone2: 845.0, tone_decay: 0.3, tone_lvl: 0.6, pan: 0.2, ..D },
        57 => Drum { noise_decay: 2.0, hp: 3500.0, pan: 0.3, ..CYMBAL },
        58 => Drum { noise: 0.8, noise_decay: 0.8, hp: 2000.0, lp: 6000.0, rattle: 25.0, rattle_len: 0.8, ..D },
        59 => Drum { metal: 0.6, noise: 0.25, noise_decay: 1.6, hp: 4000.0, pan: 0.35, ..CYMBAL },
        60 => Drum { tone: 420.0, tone_end: 400.0, tone_decay: 0.15, ..HAND },
        61 => Drum { tone: 320.0, tone_end: 300.0, ..HAND },
        62 => Drum { tone: 300.0, tone_end: 280.0, tone_decay: 0.08, pan: -0.2, ..HAND },
        63 => Drum { tone: 300.0, tone_end: 285.0, tone_decay: 0.3, pan: -0.2, ..HAND },
        64 => Drum { tone: 210.0, tone_end: 200.0, tone_decay: 0.35, pan: -0.2, ..HAND },
        65 => Drum { tone: 520.0, tone_end: 500.0, tone_decay: 0.3, noise: 0.2, noise_decay: 0.1, hp: 3000.0, ..HAND },
        66 => Drum { tone: 380.0, tone_end: 365.0, tone_decay: 0.35, noise: 0.2, noise_decay: 0.1, hp: 3000.0, ..HAND },
        67 => Drum { tone: 900.0, tone2: 1350.0, tone_decay: 0.3, tone_lvl: 0.6, pan: 0.3, ..D },
        68 => Drum { tone: 650.0, tone2: 975.0, tone_decay: 0.3, tone_lvl: 0.6, pan: 0.3, ..D },
        69 => Drum { noise: 0.7, noise_decay: 0.1, ..SHAKER },
        70 => Drum { noise_decay: 0.06, hp: 7000.0, ..SHAKER },
        71 => Drum { tone: 2400.0, tone_decay: 0.25, tone_lvl: 0.4, ..D },
        72 => Drum { tone: 2200.0, tone_decay: 0.8, tone_lvl: 0.4, ..D },
        73 => Drum { noise: 0.8, noise_decay: 0.15, hp: 2500.0, lp: 6000.0, rattle: 40.0, rattle_len: 0.15, ..D },
        74 => Drum { noise: 0.8, noise_decay: 0.4, hp: 2500.0, lp: 6000.0, rattle: 40.0, rattle_len: 0.4, ..D },
        75 => Drum { tone: 2500.0, tone_decay: 0.06, ..BLOCK_HIT },
        76 => Drum { tone: 1600.0, tone_end: 1500.0, ..BLOCK_HIT },
        77 => Drum { tone: 1100.0, tone_end: 1000.0, ..BLOCK_HIT },
        78 => Drum { tone: 600.0, tone_end: 900.0, drop: 0.05, tone_decay: 0.15, tone_lvl: 0.7, ..D },
        79 => Drum { tone: 500.0, tone_end: 250.0, drop: 0.15, tone_decay: 0.35, tone_lvl: 0.7, ..D },
        80 => Drum { tone: 4200.0, tone2: 6100.0, tone_decay: 0.15, tone_lvl: 0.3, pan: -0.4, ..D },
        81 => Drum { tone: 4200.0, tone2: 6100.0, tone_decay: 1.2, tone_lvl: 0.3, pan: -0.4, ..D },
        _ => CLICK,
    }
}

/// The metallic cluster's six square waves (Hz).
const METAL: [f32; 6] = [205.3, 304.4, 369.6, 522.7, 540.0, 800.0];

// ------------------------------------------------------------ helpers --

#[inline]
fn wrap(x: f32) -> f32 {
    x - x.floor()
}

/// A sine from the table, `ph` in cycles (0…1).
#[inline]
fn sin1(t: &[f32], ph: f32) -> f32 {
    let x = ph * SINE_N as f32;
    let i = (x as usize) & (SINE_N - 1);
    let f = x - x.floor();
    t[i] + (t[i + 1] - t[i]) * f
}

/// PolyBLEP: rounds a waveform's jump at phase 0 over one sample.
#[inline]
fn blep(t: f32, dt: f32) -> f32 {
    if t < dt {
        let x = t / dt;
        x + x - x * x - 1.0
    } else if t > 1.0 - dt {
        let x = (t - 1.0) / dt;
        x * x + x + x + 1.0
    } else {
        0.0
    }
}

#[inline]
fn pulse(ph: f32, dt: f32, w: f32) -> f32 {
    let y = if ph < w { 1.0 } else { -1.0 };
    y + blep(ph, dt) - blep(wrap(ph - w), dt)
}

/// One oscillator sample; `pm` (cycles) phase-modulates a sine.
#[inline]
fn osc(w: Wave, ph: f32, dt: f32, pm: f32, sine: &[f32]) -> f32 {
    match w {
        Off => 0.0,
        Sine => sin1(sine, wrap(ph + pm)),
        Tri => 4.0 * (ph - 0.5).abs() - 1.0,
        Saw => 2.0 * ph - 1.0 - blep(ph, dt),
        Square => pulse(ph, dt, 0.5),
        // (its DC removed)
        Pulse => pulse(ph, dt, 0.25) + 0.5,
    }
}

#[inline]
fn noise(seed: &mut u32) -> f32 {
    *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    (*seed >> 8) as f32 * (1.0 / 8_388_608.0) - 1.0
}

/// The per-sample factor that falls 60 dB in `secs`.
fn t60(secs: f32, rate: f32) -> f32 {
    (-6.907_755 / (secs.max(0.0005) * rate)).exp()
}

/// A rational tanh, close enough for a soft clipper.
#[inline]
fn tanh(x: f32) -> f32 {
    let x = x.clamp(-3.0, 3.0);
    x * (27.0 + x * x) / (27.0 + 9.0 * x * x)
}

/// Linear below 0.5, then softly to ±1.
#[inline]
fn soft_clip(x: f32) -> f32 {
    let a = x.abs();
    if a <= 0.5 {
        x
    } else {
        (0.5 + 0.5 * tanh((a - 0.5) * 2.0)).copysign(x)
    }
}

/// A state-variable filter (topology-preserving, stable at any setting).
#[derive(Debug, Clone, Copy, Default)]
struct Svf {
    ic1: f32,
    ic2: f32,
    a1: f32,
    a2: f32,
    a3: f32,
    k: f32,
}

impl Svf {
    fn tune(&mut self, fc: f32, rate: f32, k: f32) {
        let g = (PI * fc.clamp(20.0, rate * 0.45) / rate).tan();
        self.k = k;
        self.a1 = 1.0 / (1.0 + g * (g + k));
        self.a2 = g * self.a1;
        self.a3 = g * self.a2;
    }

    /// (low-pass, high-pass)
    #[inline]
    fn run(&mut self, x: f32) -> (f32, f32) {
        let v3 = x - self.ic2;
        let v1 = self.a1 * self.ic1 + self.a2 * v3;
        let v2 = self.ic2 + self.a2 * self.ic1 + self.a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1;
        self.ic2 = 2.0 * v2 - self.ic2;
        (v2, x - self.k * v1 - v2)
    }
}

// ------------------------------------------------------------ channels --

#[derive(Debug, Clone, Copy)]
struct Channel {
    program: u8,
    drum: bool,
    volume: u8,
    expression: u8,
    pan: u8,
    modulation: u8,
    reverb: u8,
    sustain: bool,
    /// -1…1 of the range.
    bend: f32,
    /// RPN 0: semitones and cents.
    range: (u8, u8),
    /// The selected RPN (MSB, LSB); 127 none.
    rpn: (u8, u8),
}

impl Channel {
    fn new(drum: bool) -> Channel {
        Channel { program: 0, drum, volume: 100, expression: 127, pan: 64, modulation: 0, reverb: 40, sustain: false, bend: 0.0, range: (2, 0), rpn: (127, 127) }
    }
}

/// A channel's values for one block.
#[derive(Debug, Clone, Copy, Default)]
struct Mixing {
    gain: f32,
    pan: f32,
    bend: f32,
    vib: f32,
    send: f32,
}

// -------------------------------------------------------------- voices --

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Attack,
    /// (falling to the sustain level, which may be silence)
    Decay,
    Release,
    Off,
}

#[derive(Debug, Clone, Copy)]
struct Voice {
    on: bool,
    ch: u8,
    note: u8,
    vel: u8,
    /// The key is down …
    held: bool,
    /// … or was let go while the sustain pedal was down.
    pedal: bool,
    /// Fading out after a steal (no longer counted).
    stolen: bool,
    age: u64,
    is_drum: bool,
    r: Recipe,
    d: Drum,
    stage: Stage,
    env: f32,
    att: f32,
    dec: f32,
    rel: f32,
    sus: f32,
    vel_amp: f32,
    bright: f32,
    amp: f32,
    amp_step: f32,
    first: bool,
    ph: [f32; 3],
    det: [f32; 3],
    inc: [f32; 3],
    ph2: f32,
    inc2: f32,
    mph: f32,
    minc: f32,
    /// FM depth now (cycles).
    fm: f32,
    fenv: f32,
    fmenv: f32,
    dropenv: f32,
    lfo: f32,
    trem: f32,
    /// Seconds since the note began.
    t: f32,
    filt: Svf,
    filtered: bool,
    seed: u32,
    // (drums)
    tenv: f32,
    tcoef: f32,
    nenv: f32,
    ncoef: f32,
    lp: f32,
    lpk: f32,
    metal: [f32; 6],
    minc6: [f32; 6],
    frames: u32,
}

const IDLE: Voice = Voice {
    on: false,
    ch: 0,
    note: 0,
    vel: 0,
    held: false,
    pedal: false,
    stolen: false,
    age: 0,
    is_drum: false,
    r: R,
    d: D,
    stage: Stage::Off,
    env: 0.0,
    att: 0.0,
    dec: 1.0,
    rel: 1.0,
    sus: 0.0,
    vel_amp: 0.0,
    bright: 1.0,
    amp: 0.0,
    amp_step: 0.0,
    first: true,
    ph: [0.0; 3],
    det: [1.0; 3],
    inc: [0.0; 3],
    ph2: 0.0,
    inc2: 0.0,
    mph: 0.0,
    minc: 0.0,
    fm: 0.0,
    fenv: 1.0,
    fmenv: 1.0,
    dropenv: 1.0,
    lfo: 0.0,
    trem: 0.75,
    t: 0.0,
    filt: Svf { ic1: 0.0, ic2: 0.0, a1: 0.0, a2: 0.0, a3: 0.0, k: 0.0 },
    filtered: false,
    seed: 1,
    tenv: 0.0,
    tcoef: 1.0,
    nenv: 0.0,
    ncoef: 1.0,
    lp: 0.0,
    lpk: 1.0,
    metal: [0.0; 6],
    minc6: [0.0; 6],
    frames: 0,
};

/// The block's three sums: left, right, reverb send.
struct Bus {
    l: [f32; BLOCK],
    r: [f32; BLOCK],
    s: [f32; BLOCK],
}

impl Voice {
    /// How loud it is now (for picking which fading voice to reuse).
    fn level(&self) -> f32 {
        if self.is_drum {
            self.env * self.tenv.max(self.nenv)
        } else {
            self.env
        }
    }

    fn release(&mut self) {
        self.held = false;
        self.pedal = false;
        if self.stage != Stage::Off {
            self.stage = Stage::Release;
        }
    }

    fn fade(&mut self, rate: f32) {
        self.release();
        self.rel = t60(FAST, rate);
    }

    /// Pitch, filter, LFOs and gain for the next `n` frames.
    fn control(&mut self, m: &Mixing, rate: f32, n: usize, sine: &[f32]) -> (f32, f32, f32) {
        let dt = n as f32 / rate;
        let pan = (m.pan + if self.is_drum { self.d.pan } else { 0.0 }).clamp(-1.0, 1.0);
        let pc = (pan + 1.0) * 0.125;
        let (pl, pr) = (sin1(sine, wrap(pc + 0.25)), sin1(sine, pc));
        let mut target = self.vel_amp * m.gain;
        if self.is_drum {
            let d = self.d;
            let end = if d.tone_end > 0.0 { d.tone_end } else { d.tone };
            let f = end + (d.tone - end) * self.dropenv;
            self.dropenv *= (-dt / d.drop.max(0.001)).exp();
            self.inc[0] = (f / rate).min(0.45);
            self.inc2 = (d.tone2 / rate).min(0.45);
        } else {
            let r = self.r;
            let fade = if self.t < 0.35 { self.t / 0.35 } else { 1.0 };
            let vib_cents = r.vibrato * fade + m.vib;
            let mut semis = f32::from(self.note) - 69.0 + m.bend + r.drop * self.dropenv;
            if vib_cents > 0.0 {
                semis += vib_cents * 0.01 * sin1(sine, self.lfo);
                self.lfo = wrap(self.lfo + r.vib_rate * dt);
            }
            self.dropenv *= (-dt / 0.04).exp();
            let f = 440.0 * (semis / 12.0).exp2();
            for u in 0..usize::from(r.unison) {
                self.inc[u] = (f * self.det[u] / rate).min(0.45);
            }
            self.inc2 = f * r.ratio2 / rate;
            self.minc = (f * r.fm_ratio / rate).min(0.49);
            if r.fm_index > 0.0 {
                let e = if r.fm_decay > 0.0 {
                    let e = 1.0 / 3.0 + 2.0 / 3.0 * self.fmenv;
                    self.fmenv *= (-dt / r.fm_decay).exp();
                    e
                } else {
                    1.0
                };
                self.fm = r.fm_index * e * self.bright / (2.0 * PI);
            }
            if self.filtered {
                let fc = (r.cutoff + r.env_cutoff * self.fenv) * (f + 40.0) * self.bright;
                self.fenv *= (-dt / r.fdecay.max(0.001)).exp();
                self.filt.tune(fc, rate, 1.4 * (1.0 - 0.9 * r.reso));
            }
            if r.tremolo > 0.0 {
                target *= 1.0 - r.tremolo * 0.5 * (1.0 + sin1(sine, self.trem));
                self.trem = wrap(self.trem + r.trem_rate * dt);
            }
        }
        self.t += dt;
        if self.first {
            self.amp = target;
            self.first = false;
        }
        self.amp_step = (target - self.amp) / n as f32;
        (pl, pr, m.send)
    }

    fn run(&mut self, n: usize, sine: &[f32], rate: f32, pan: (f32, f32, f32), bus: &mut Bus) {
        if self.is_drum {
            self.run_drum(n, sine, rate, pan, bus);
        } else {
            self.run_melodic(n, sine, pan, bus);
        }
    }

    #[allow(clippy::needless_range_loop)]
    fn run_melodic(&mut self, n: usize, sine: &[f32], (pl, pr, send): (f32, f32, f32), bus: &mut Bus) {
        let r = self.r;
        let uni = usize::from(r.unison);
        let second = r.level2 > 0.0 && self.inc2 < 0.45;
        for i in 0..n {
            match self.stage {
                Stage::Attack => {
                    self.env += self.att;
                    if self.env >= 1.0 {
                        self.env = 1.0;
                        self.stage = Stage::Decay;
                    }
                }
                Stage::Decay => {
                    self.env = self.sus + (self.env - self.sus) * self.dec;
                    if self.sus == 0.0 && self.env < SILENT {
                        self.stage = Stage::Off;
                    }
                }
                Stage::Release => {
                    self.env *= self.rel;
                    if self.env < SILENT {
                        self.stage = Stage::Off;
                    }
                }
                Stage::Off => break,
            }
            let mut x = 0.0;
            if r.wave != Off {
                let pm = if self.fm != 0.0 {
                    let m = self.fm * sin1(sine, self.mph);
                    self.mph = wrap(self.mph + self.minc);
                    m
                } else {
                    0.0
                };
                for u in 0..uni {
                    x += osc(r.wave, self.ph[u], self.inc[u], pm, sine);
                    self.ph[u] += self.inc[u];
                    if self.ph[u] >= 1.0 {
                        self.ph[u] -= 1.0;
                    }
                }
            }
            if second {
                x += r.level2 * osc(r.wave2, self.ph2, self.inc2, 0.0, sine);
                self.ph2 = wrap(self.ph2 + self.inc2);
            }
            if r.noise > 0.0 {
                x += r.noise * noise(&mut self.seed);
            }
            if r.drive > 0.0 {
                let y = x * (1.0 + r.drive);
                x = y / (1.0 + y.abs()) * 1.5;
            }
            if self.filtered {
                x = self.filt.run(x).0;
            }
            let out = x * self.env * self.amp;
            self.amp += self.amp_step;
            bus.l[i] += out * pl;
            bus.r[i] += out * pr;
            bus.s[i] += out * send;
        }
    }

    #[allow(clippy::needless_range_loop)]
    fn run_drum(&mut self, n: usize, sine: &[f32], rate: f32, (pl, pr, send): (f32, f32, f32), bus: &mut Bus) {
        let d = self.d;
        let rattle_frames = (d.rattle_len * rate) as u32;
        for i in 0..n {
            if self.stage == Stage::Release {
                self.env *= self.rel;
                if self.env < SILENT {
                    self.stage = Stage::Off;
                }
            }
            if self.stage == Stage::Off || (self.tenv < SILENT && self.nenv < SILENT) {
                self.stage = Stage::Off;
                break;
            }
            let mut x = 0.0;
            if d.tone_lvl > 0.0 {
                let mut t = sin1(sine, self.ph[0]);
                self.ph[0] = wrap(self.ph[0] + self.inc[0]);
                if d.tone2 > 0.0 {
                    t += 0.8 * sin1(sine, self.ph2);
                    self.ph2 = wrap(self.ph2 + self.inc2);
                }
                x += t * d.tone_lvl * self.tenv;
                self.tenv *= self.tcoef;
            }
            if self.nenv >= SILENT {
                let mut z = d.noise * noise(&mut self.seed);
                if d.metal > 0.0 {
                    let mut m = 0.0;
                    for k in 0..6 {
                        m += if self.metal[k] < 0.5 { 1.0 } else { -1.0 };
                        self.metal[k] = wrap(self.metal[k] + self.minc6[k]);
                    }
                    z += d.metal * m * (1.0 / 6.0);
                }
                z = self.filt.run(z).1;
                self.lp += self.lpk * (z - self.lp);
                let mut g = self.nenv;
                if self.frames < rattle_frames {
                    g *= 1.0 - wrap(self.frames as f32 * d.rattle / rate);
                }
                x += self.lp * g;
                self.nenv *= self.ncoef;
            }
            self.frames += 1;
            let out = x * self.env * self.amp;
            self.amp += self.amp_step;
            bus.l[i] += out * pl;
            bus.r[i] += out * pr;
            bus.s[i] += out * send;
        }
    }
}

// -------------------------------------------------------------- reverb --

/// A small Schroeder reverb: four damped combs and two all-passes a side.
struct Reverb {
    buf: Vec<f32>,
    /// (start, length, position) of 8 combs then 4 all-passes.
    lines: [(usize, usize, usize); 12],
    damp: [f32; 8],
}

const COMBS: [usize; 4] = [1117, 1201, 1289, 1373];
const ALLPASSES: [usize; 2] = [557, 443];
const SPREAD: usize = 23;
const FEEDBACK: f32 = 0.86;
const DAMPING: f32 = 0.3;

impl Reverb {
    fn new(rate: f32) -> Reverb {
        let scale = rate / 44100.0;
        let mut lines = [(0, 0, 0); 12];
        let mut at = 0;
        let mut push = |i: usize, len: usize| {
            let len = ((len as f32 * scale) as usize).max(1);
            lines[i] = (at, len, 0);
            at += len;
        };
        for side in 0..2 {
            for (c, &len) in COMBS.iter().enumerate() {
                push(side * 4 + c, len + side * SPREAD);
            }
        }
        for side in 0..2 {
            for (a, &len) in ALLPASSES.iter().enumerate() {
                push(8 + side * 2 + a, len + side * SPREAD);
            }
        }
        let total = lines.iter().map(|l| l.0 + l.1).max().unwrap_or(0);
        Reverb { buf: vec![0.0; total], lines, damp: [0.0; 8] }
    }

    #[inline]
    fn tick(&mut self, x: f32) -> (f32, f32) {
        let input = x * 0.15;
        let mut out = [0.0f32; 2];
        for (side, o) in out.iter_mut().enumerate() {
            let mut acc = 0.0;
            for c in 0..4 {
                let k = side * 4 + c;
                let (start, len, pos) = self.lines[k];
                let y = self.buf[start + pos];
                self.damp[k] = y * (1.0 - DAMPING) + self.damp[k] * DAMPING;
                self.buf[start + pos] = input + self.damp[k] * FEEDBACK;
                self.lines[k].2 = if pos + 1 == len { 0 } else { pos + 1 };
                acc += y;
            }
            for a in 0..2 {
                let k = 8 + side * 2 + a;
                let (start, len, pos) = self.lines[k];
                let b = self.buf[start + pos];
                self.buf[start + pos] = acc + b * 0.5;
                acc = b - acc;
                self.lines[k].2 = if pos + 1 == len { 0 } else { pos + 1 };
            }
            *o = acc;
        }
        (out[0], out[1])
    }
}

// --------------------------------------------------------------- synth --

/// The built-in synthesizer: 16 channels, General MIDI programs (channel 10 = drums).
pub struct Synth {
    rate: f32,
    ch: [Channel; 16],
    voices: Vec<Voice>,
    sine: Vec<f32>,
    age: u64,
    seed: u32,
    reverb: Reverb,
    /// The loudest reverb sample of the last block (for `silent`).
    tail: f32,
}

impl Synth {
    pub fn new(rate: u32) -> Synth {
        let rate = if rate == 0 { 44100.0 } else { rate as f32 };
        let sine = (0..=SINE_N).map(|i| (2.0 * PI * i as f32 / SINE_N as f32).sin()).collect();
        Synth { rate, ch: std::array::from_fn(|i| Channel::new(i == 9)), voices: vec![IDLE; POOL], sine, age: 0, seed: 0x1234_5678, reverb: Reverb::new(rate), tail: 0.0 }
    }

    /// One MIDI message (status byte first; running status not needed). SysEx GM/GS reset resets the channels.
    pub fn message(&mut self, bytes: &[u8]) {
        let Some(&status) = bytes.first() else { return };
        if status == 0xF0 {
            return self.sysex(bytes);
        }
        if !(0x80..0xF0).contains(&status) {
            return;
        }
        let ch = usize::from(status & 0x0F);
        let need = if matches!(status & 0xF0, 0xC0 | 0xD0) { 2 } else { 3 };
        if bytes.len() < need {
            return;
        }
        let d1 = bytes[1] & 0x7F;
        let d2 = bytes.get(2).copied().unwrap_or(0) & 0x7F;
        match status & 0xF0 {
            0x80 => self.note_off(ch, d1),
            0x90 if d2 == 0 => self.note_off(ch, d1),
            0x90 => self.note_on(ch, d1, d2),
            0xB0 => self.controller(ch, d1, d2),
            0xC0 => self.ch[ch].program = d1,
            0xE0 => self.ch[ch].bend = ((i32::from(d2) << 7 | i32::from(d1)) - 8192) as f32 / 8192.0,
            // (aftertouch, poly pressure: ignored)
            _ => {}
        }
    }

    /// Adds the next frames into `out`: interleaved stereo f32 (L, R, L, R…), out.len()/2 frames.
    /// It overwrites `out` (doesn't mix into it).
    pub fn render(&mut self, out: &mut [f32]) {
        let frames = out.len() / 2;
        let mut done = 0;
        while done < frames {
            let n = (frames - done).min(BLOCK);
            self.block(&mut out[done * 2..(done + n) * 2], n);
            done += n;
        }
    }

    /// Every note silenced at once (Stop): a few milliseconds' fade, the pedals up.
    pub fn all_off(&mut self) {
        for c in &mut self.ch {
            c.sustain = false;
        }
        let rate = self.rate;
        for v in self.voices.iter_mut().filter(|v| v.on) {
            v.fade(rate);
        }
    }

    /// Voices sounding (fading ones too).
    pub fn active(&self) -> usize {
        self.voices.iter().filter(|v| v.on).count()
    }

    /// No voice sounds and the reverb has died away.
    pub fn silent(&self) -> bool {
        self.active() == 0 && self.tail < SILENT
    }

    fn block(&mut self, out: &mut [f32], n: usize) {
        let mut bus = Bus { l: [0.0; BLOCK], r: [0.0; BLOCK], s: [0.0; BLOCK] };
        let mut mix = [Mixing::default(); 16];
        for (m, c) in mix.iter_mut().zip(&self.ch) {
            let vol = f32::from(c.volume) / 127.0;
            let expr = f32::from(c.expression) / 127.0;
            m.gain = vol * vol * expr * expr;
            m.pan = ((f32::from(c.pan) - 64.0) / 63.0).clamp(-1.0, 1.0);
            m.bend = c.bend * (f32::from(c.range.0) + f32::from(c.range.1) * 0.01);
            m.vib = f32::from(c.modulation) / 127.0 * 50.0;
            m.send = f32::from(c.reverb) / 127.0;
        }
        let (rate, sine) = (self.rate, &self.sine);
        for v in self.voices.iter_mut().filter(|v| v.on) {
            let p = v.control(&mix[usize::from(v.ch)], rate, n, sine);
            v.run(n, sine, rate, p, &mut bus);
            if v.stage == Stage::Off {
                v.on = false;
            }
        }
        let mut tail = 0.0f32;
        for (i, o) in out.as_chunks_mut::<2>().0.iter_mut().enumerate() {
            let (wl, wr) = self.reverb.tick(bus.s[i]);
            tail = tail.max(wl.abs()).max(wr.abs());
            o[0] = soft_clip((bus.l[i] + wl) * MASTER);
            o[1] = soft_clip((bus.r[i] + wr) * MASTER);
        }
        self.tail = tail;
    }

    fn next_seed(&mut self) -> u32 {
        self.seed = self.seed.wrapping_mul(1_103_515_245).wrapping_add(12345);
        self.seed | 1
    }

    /// A free voice; past the polyphony the oldest released voice (else
    /// the oldest) fades out to make room.
    fn alloc(&mut self) -> usize {
        let sounding = self.voices.iter().filter(|v| v.on && !v.stolen).count();
        if sounding >= POLY {
            let victim = self.voices.iter().enumerate().filter(|(_, v)| v.on && !v.stolen).min_by_key(|(_, v)| (!(v.stage == Stage::Release || v.is_drum), v.age)).map(|(i, _)| i);
            if let Some(i) = victim {
                let rate = self.rate;
                self.voices[i].fade(rate);
                self.voices[i].stolen = true;
            }
        }
        if let Some(i) = self.voices.iter().position(|v| !v.on) {
            return i;
        }
        // (every slot fading: the quietest)
        self.voices.iter().enumerate().min_by(|a, b| a.1.level().total_cmp(&b.1.level())).map(|(i, _)| i).unwrap_or(0)
    }

    fn note_on(&mut self, ch: usize, note: u8, vel: u8) {
        let rate = self.rate;
        let c = self.ch[ch];
        // (the same key again: the old note lets go)
        for v in self.voices.iter_mut().filter(|v| v.on && usize::from(v.ch) == ch && v.note == note && v.stage != Stage::Release) {
            if c.drum {
                v.fade(rate);
            } else {
                v.release();
            }
        }
        if c.drum && matches!(note, 42 | 44) {
            // (a closed hi-hat chokes the open one)
            for v in self.voices.iter_mut().filter(|v| v.on && v.is_drum && usize::from(v.ch) == ch && v.note == 46) {
                v.release();
                v.rel = t60(0.03, rate);
            }
        }
        let i = self.alloc();
        self.age += 1;
        let seed = self.next_seed();
        let vn = f32::from(vel) / 127.0;
        let mut v = IDLE;
        v.on = true;
        v.ch = ch as u8;
        v.note = note;
        v.vel = vel;
        v.held = true;
        v.age = self.age;
        v.seed = seed;
        v.bright = 0.55 + 0.6 * vn;
        if c.drum {
            let d = drum(note);
            v.is_drum = true;
            v.d = d;
            v.stage = Stage::Decay;
            v.env = 1.0;
            v.vel_amp = vn.powf(1.5) * d.gain;
            v.tenv = if d.tone_lvl > 0.0 { 1.0 } else { 0.0 };
            v.tcoef = t60(d.tone_decay, rate);
            v.nenv = if d.noise > 0.0 || d.metal > 0.0 { 1.0 } else { 0.0 };
            v.ncoef = t60(d.noise_decay, rate);
            v.filt.tune(d.hp, rate, 1.4);
            v.lpk = 1.0 - (-2.0 * PI * d.lp.min(rate * 0.45) / rate).exp();
            for (k, f) in METAL.iter().enumerate() {
                v.minc6[k] = (f * 8.0 * (1.0 + 0.05 * k as f32) / rate).min(0.45);
            }
            v.held = false;
        } else {
            let r = GM[usize::from(c.program)];
            let uni = usize::from(r.unison.clamp(1, 3));
            v.r = Recipe { unison: uni as u8, ..r };
            v.stage = Stage::Attack;
            v.att = 1.0 / (r.attack.max(0.001) * rate);
            v.sus = r.sustain;
            let keyscale = if r.sustain == 0.0 { (-(f32::from(note) - 60.0) / 24.0).exp2().clamp(0.25, 2.0) } else { 1.0 };
            v.dec = t60(r.decay * keyscale, rate);
            v.rel = t60(r.release, rate);
            v.vel_amp = vn.powf(1.7) * r.gain / (uni as f32).sqrt();
            v.det = match uni {
                1 => [1.0; 3],
                2 => [(-r.detune / 2400.0).exp2(), (r.detune / 2400.0).exp2(), 1.0],
                _ => [(-r.detune / 1200.0).exp2(), 1.0, (r.detune / 1200.0).exp2()],
            };
            v.ph = [0.0, 0.37, 0.71];
            v.filtered = r.cutoff > 0.0;
        }
        self.voices[i] = v;
    }

    fn note_off(&mut self, ch: usize, note: u8) {
        self.keys_up(ch, Some(note));
    }

    /// Lets go of a channel's keys (`None`: all of them); the pedal keeps them sounding.
    fn keys_up(&mut self, ch: usize, note: Option<u8>) {
        let pedal = self.ch[ch].sustain;
        for v in self.voices.iter_mut().filter(|v| v.on && !v.is_drum && usize::from(v.ch) == ch && v.held && note.is_none_or(|n| n == v.note)) {
            if pedal {
                v.held = false;
                v.pedal = true;
            } else {
                v.release();
            }
        }
    }

    fn sound_off(&mut self, ch: usize) {
        let rate = self.rate;
        for v in self.voices.iter_mut().filter(|v| v.on && usize::from(v.ch) == ch) {
            v.fade(rate);
        }
    }

    fn controller(&mut self, ch: usize, cc: u8, val: u8) {
        let c = &mut self.ch[ch];
        match cc {
            1 => c.modulation = val,
            6 if c.rpn == (0, 0) => c.range.0 = val.min(24),
            38 if c.rpn == (0, 0) => c.range.1 = val.min(99),
            7 => c.volume = val,
            10 => c.pan = val,
            11 => c.expression = val,
            64 => {
                c.sustain = val >= 64;
                if !c.sustain {
                    for v in self.voices.iter_mut().filter(|v| v.on && usize::from(v.ch) == ch && v.pedal) {
                        v.release();
                    }
                }
            }
            91 => c.reverb = val,
            // (an NRPN chosen: data entry no longer reaches an RPN)
            98 | 99 => c.rpn = (127, 127),
            100 => c.rpn.1 = val,
            101 => c.rpn.0 = val,
            120 => self.sound_off(ch),
            121 => {
                c.modulation = 0;
                c.expression = 127;
                c.bend = 0.0;
                c.rpn = (127, 127);
                c.sustain = false;
                for v in self.voices.iter_mut().filter(|v| v.on && usize::from(v.ch) == ch && v.pedal) {
                    v.release();
                }
            }
            // (all notes off, and the mode changes that imply it)
            123..=127 => self.keys_up(ch, None),
            _ => {}
        }
    }

    fn sysex(&mut self, b: &[u8]) {
        let gm_on = b.len() >= 6 && b[1] == 0x7E && b[3] == 0x09 && matches!(b[4], 0x01 | 0x03);
        let gs_reset = b.len() >= 11 && b[1] == 0x41 && b[3] == 0x42 && b[4] == 0x12 && b[5..9] == [0x40, 0x00, 0x7F, 0x00];
        let xg_on = b.len() >= 9 && b[1] == 0x43 && b[2] & 0xF0 == 0x10 && b[3..8] == [0x4C, 0x00, 0x00, 0x7E, 0x00];
        if gm_on || gs_reset || xg_on {
            self.all_off();
            self.ch = std::array::from_fn(|i| Channel::new(i == 9));
            return;
        }
        // (GS: a part made a drum part, F0 41 dd 42 12 40 1p 15 vv sum F7)
        if b.len() >= 10 && b[1] == 0x41 && b[3] == 0x42 && b[4] == 0x12 && b[5] == 0x40 && b[6] & 0xF0 == 0x10 && b[7] == 0x15 {
            let ch = match b[6] & 0x0F {
                0 => 9,
                p @ 1..=9 => usize::from(p) - 1,
                p => usize::from(p),
            };
            self.ch[ch].drum = b[8] != 0;
        }
    }
}

// -------------------------------------------------------------- player --

/// A song played on the synth from a position, its messages at their times.
pub struct Player {
    synth: Synth,
    events: Vec<(u64, Vec<u8>)>,
    next: usize,
    /// Frames played.
    pos: u64,
    rate: u64,
    from_us: u64,
    gain: f32,
}

impl Player {
    /// `events`: (µs from the song's start, message bytes), in time order (Song's events); `from_us`: where to begin —
    /// the program changes, controllers, pitch bends and sysex before it are applied first (chased), no notes.
    pub fn new(events: Vec<(u64, Vec<u8>)>, rate: u32, from_us: u64, gain: f32) -> Player {
        let mut synth = Synth::new(rate);
        let mut next = 0;
        for (at, bytes) in &events {
            if *at >= from_us {
                break;
            }
            let notes = bytes.first().is_some_and(|s| matches!(s & 0xF0, 0x80 | 0x90 | 0xA0) && *s < 0xF0);
            if !notes {
                synth.message(bytes);
            }
            next += 1;
        }
        let rate = if rate == 0 { 44100 } else { u64::from(rate) };
        Player { synth, events, next, pos: 0, rate, from_us, gain: gain.clamp(0.0, 1.0) }
    }

    /// QMIDI's Volume (0…1): note-ons from now scale their velocity by it (at least 1), as the MIDI-port path does.
    pub fn set_gain(&mut self, gain: f32) {
        self.gain = gain.clamp(0.0, 1.0);
    }

    /// Renders the next frames (interleaved stereo); `false` once the song's last message has played and every voice is silent.
    pub fn render(&mut self, out: &mut [f32]) -> bool {
        let frames = out.len() / 2;
        let mut done = 0;
        while done < frames {
            while let Some((at, bytes)) = self.events.get(self.next) {
                if self.frame_of(*at) > self.pos {
                    break;
                }
                if bytes.len() == 3 && bytes[0] & 0xF0 == 0x90 && bytes[2] > 0 {
                    // (Volume: a note's velocity scaled, as the port path does)
                    let g = (self.gain * 1000.0) as u32 as f32 / 1000.0;
                    let v = ((f32::from(bytes[2]) * g).round() as u8).clamp(1, 127);
                    self.synth.message(&[bytes[0], bytes[1], v]);
                } else {
                    self.synth.message(bytes);
                }
                self.next += 1;
            }
            let until = self.events.get(self.next).map_or(u64::MAX, |e| self.frame_of(e.0));
            let n = ((frames - done) as u64).min(until - self.pos) as usize;
            self.synth.render(&mut out[done * 2..(done + n) * 2]);
            self.pos += n as u64;
            done += n;
        }
        !(self.next >= self.events.len() && self.synth.silent())
    }

    fn frame_of(&self, at_us: u64) -> u64 {
        (u128::from(at_us.saturating_sub(self.from_us)) * u128::from(self.rate) / 1_000_000) as u64
    }

    /// Every note silenced at once (Stop).
    pub fn all_off(&mut self) {
        self.synth.all_off();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 44100;

    fn render(s: &mut Synth, secs: f32) -> Vec<f32> {
        let mut out = vec![0.0; (secs * RATE as f32) as usize * 2];
        s.render(&mut out);
        out
    }

    fn peak(x: &[f32]) -> f32 {
        x.iter().fold(0.0f32, |m, v| m.max(v.abs()))
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    fn sane(x: &[f32]) {
        assert!(x.iter().all(|v| v.is_finite() && (-1.0..=1.0).contains(v)));
    }

    /// The power at `f` Hz of the left channel (Goertzel).
    fn power(x: &[f32], f: f32) -> f32 {
        let w = 2.0 * std::f64::consts::PI * f64::from(f) / f64::from(RATE);
        let c = 2.0 * w.cos();
        let (mut s1, mut s2) = (0.0f64, 0.0f64);
        for v in x.chunks(2) {
            let s0 = f64::from(v[0]) + c * s1 - s2;
            s2 = s1;
            s1 = s0;
        }
        (s1 * s1 + s2 * s2 - c * s1 * s2) as f32
    }

    #[test]
    fn notes_sound_then_die_away() {
        for program in [0u8, 4, 6, 11, 16, 19, 24, 30, 33, 38, 40, 45, 47, 48, 52, 56, 61, 65, 71, 73, 75, 80, 81, 88, 89, 95, 98, 104, 114, 119, 122, 127] {
            let mut s = Synth::new(RATE);
            s.message(&[0xC0, program]);
            s.message(&[0x90, 60, 100]);
            let on = render(&mut s, 0.6);
            sane(&on);
            assert!(rms(&on[on.len() / 2..]) > 0.003 || rms(&on) > 0.01, "program {program} is silent");
            s.message(&[0x80, 60, 0]);
            let r = GM[usize::from(program)].release;
            let off = render(&mut s, r * 1.5 + 0.05);
            sane(&off);
            assert_eq!(s.active(), 0, "program {program} still sounds {r}s after its release");
            let tail = render(&mut s, 3.0);
            assert!(peak(&tail[tail.len() - 2000..]) < 1e-4, "program {program}'s reverb never ends");
            assert!(s.silent());
        }
    }

    #[test]
    fn every_program_is_finite_and_bounded() {
        let mut s = Synth::new(RATE);
        for p in 0..128u8 {
            s.message(&[0xC0, p]);
            for n in [24u8, 60, 108] {
                s.message(&[0x90, n, 127]);
            }
            sane(&render(&mut s, 0.05));
            for n in [24u8, 60, 108] {
                s.message(&[0x80, n, 0]);
            }
        }
        s.all_off();
        sane(&render(&mut s, 0.1));
        assert_eq!(s.active(), 0);
    }

    #[test]
    fn drums_sound_and_stop_by_themselves() {
        for note in [35u8, 36, 37, 38, 39, 42, 46, 49, 51, 54, 56, 60, 64, 69, 75, 79, 81, 20, 100] {
            let mut s = Synth::new(RATE);
            s.message(&[0xC9, 25]); // (ignored: one kit)
            s.message(&[0x99, note, 110]);
            let a = render(&mut s, 0.1);
            sane(&a);
            assert!(peak(&a) > 0.02, "drum {note} is silent");
            let b = render(&mut s, 3.5);
            sane(&b);
            assert_eq!(s.active(), 0, "drum {note} never stops");
        }
        // (the closed hat chokes the open one)
        let mut s = Synth::new(RATE);
        s.message(&[0x99, 46, 100]);
        render(&mut s, 0.05);
        s.message(&[0x99, 42, 100]);
        render(&mut s, 0.15);
        assert_eq!(s.active(), 0);
    }

    #[test]
    fn pitch_and_bend() {
        let mut s = Synth::new(RATE);
        s.message(&[0xB0, 91, 0]);
        s.message(&[0xC0, 80]); // (Square Lead)
        s.message(&[0x90, 69, 100]);
        render(&mut s, 0.1);
        let a = render(&mut s, 0.5);
        let (p440, p415, p466) = (power(&a, 440.0), power(&a, 415.3), power(&a, 466.16));
        assert!(p440 > 20.0 * p415.max(p466), "{p440} {p415} {p466}");
        // (+2 semitones, the default range)
        s.message(&[0xE0, 0x7F, 0x7F]);
        render(&mut s, 0.05);
        let b = render(&mut s, 0.5);
        let (p494, p440) = (power(&b, 493.88), power(&b, 440.0));
        assert!(p494 > 20.0 * p440, "{p494} {p440}");
        // (RPN 0: a range of 12, half bend up: +6 semitones)
        for m in [[0xB0, 101, 0], [0xB0, 100, 0], [0xB0, 6, 12], [0xE0, 0, 0x60]] {
            s.message(&m);
        }
        render(&mut s, 0.05);
        let c = render(&mut s, 0.5);
        let f = 440.0 * 2f32.powf(6.0 / 12.0);
        assert!(power(&c, f) > 20.0 * power(&c, 440.0));
        // (zero crossings agree)
        let mut t = Synth::new(RATE);
        t.message(&[0xB0, 91, 0]);
        t.message(&[0xC0, 79]); // (Ocarina)
        t.message(&[0x90, 69, 100]);
        render(&mut t, 0.5);
        let x = render(&mut t, 1.0);
        let crossings = x.chunks(2).zip(x.chunks(2).skip(1)).filter(|(a, b)| a[0] < 0.0 && b[0] >= 0.0).count();
        assert!((430..=450).contains(&crossings), "{crossings}");
    }

    #[test]
    fn player_goes_to_the_audio_thread() {
        fn send<T: Send + 'static>() {}
        send::<Player>();
        send::<Synth>();
    }

    #[test]
    fn sustain_pedal_holds() {
        let mut s = Synth::new(RATE);
        s.message(&[0xC0, 48]);
        s.message(&[0xB0, 64, 127]);
        s.message(&[0x90, 60, 100]);
        render(&mut s, 0.2);
        s.message(&[0x80, 60, 0]);
        render(&mut s, 2.0);
        assert_eq!(s.active(), 1);
        s.message(&[0xB0, 64, 0]);
        render(&mut s, 2.0);
        assert_eq!(s.active(), 0);
    }

    #[test]
    fn polyphony_is_capped() {
        let mut s = Synth::new(RATE);
        s.message(&[0xC0, 48]);
        for n in 20..120u8 {
            s.message(&[0x90, n, 100]);
        }
        let x = render(&mut s, 0.2);
        sane(&x);
        assert!(s.voices.iter().filter(|v| v.on && !v.stolen).count() <= POLY);
        // (the newest notes kept)
        assert!(s.voices.iter().any(|v| v.on && v.note == 119));
    }

    #[test]
    fn gm_reset() {
        let mut s = Synth::new(RATE);
        s.message(&[0xC0, 40]);
        s.message(&[0xB0, 7, 20]);
        s.message(&[0xF0, 0x41, 0x10, 0x42, 0x12, 0x40, 0x19, 0x15, 0x02, 0x10, 0xF7]); // (GS: channel 9 drums)
        assert!(s.ch[8].drum);
        s.message(&[0xF0, 0x7E, 0x7F, 0x09, 0x01, 0xF7]);
        assert_eq!((s.ch[0].program, s.ch[0].volume, s.ch[8].drum, s.ch[9].drum), (0, 100, false, true));
        s.message(&[0xC0, 40]);
        s.message(&[0xF0, 0x41, 0x10, 0x42, 0x12, 0x40, 0x00, 0x7F, 0x00, 0x41, 0xF7]);
        assert_eq!(s.ch[0].program, 0);
    }

    #[test]
    fn player_chases_and_ends() {
        let ev = vec![(0, vec![0xC0, 5]), (0, vec![0xB0, 7, 50]), (0, vec![0x90, 60, 100]), (1_000_000, vec![0x90, 64, 100]), (1_500_000, vec![0x80, 64, 0]), (1_500_000, vec![0x80, 60, 0])];
        let mut p = Player::new(ev.clone(), RATE, 500_000, 1.0);
        assert_eq!((p.synth.ch[0].program, p.synth.ch[0].volume), (5, 50));
        assert_eq!(p.next, 3);
        let mut buf = vec![0.0; 4410 * 2];
        // (nothing until the note 0.5 s in)
        for _ in 0..4 {
            assert!(p.render(&mut buf));
            assert_eq!(peak(&buf), 0.0);
        }
        assert!(p.render(&mut buf));
        let mut frames = 5 * 4410;
        let mut loud = false;
        while p.render(&mut buf) {
            loud |= peak(&buf) > 0.01;
            frames += 4410;
            assert!(frames < RATE as usize * 20, "never ends");
        }
        assert!(loud);
        assert!(!p.render(&mut buf));
        // (the gain scales note-ons)
        let mut p = Player::new(ev, RATE, 0, 1.0);
        p.set_gain(0.5);
        p.render(&mut buf);
        assert_eq!(p.synth.voices.iter().find(|v| v.on).map(|v| v.vel), Some(50));
        p.set_gain(0.0);
        let mut big = vec![0.0; RATE as usize * 2];
        p.render(&mut big);
        assert!(p.synth.voices.iter().any(|v| v.on && v.note == 64 && v.vel == 1));
    }

    /// A short song: tempo, programs, a melody, chords and drums (format 1).
    fn song() -> Vec<u8> {
        fn track(ev: &[(u32, Vec<u8>)]) -> Vec<u8> {
            let mut t = Vec::new();
            let mut last = 0;
            for (at, m) in ev {
                let mut d = at - last;
                last = *at;
                let mut v = vec![(d & 0x7F) as u8];
                d >>= 7;
                while d > 0 {
                    v.insert(0, (d & 0x7F) as u8 | 0x80);
                    d >>= 7;
                }
                t.extend(v);
                t.extend(m);
            }
            t.extend([0, 0xFF, 0x2F, 0]);
            let mut b = b"MTrk".to_vec();
            b.extend((t.len() as u32).to_be_bytes());
            b.extend(t);
            b
        }
        let mut b = b"MThd\0\0\0\x06\0\x01\0\x03\0\x60".to_vec(); // (96 ticks a quarter)
        b.extend(track(&[(0, vec![0xFF, 0x51, 3, 0x07, 0xA1, 0x20]), (0, vec![0xF0, 5, 0x7E, 0x7F, 0x09, 0x01, 0xF7])]));
        let mut mel = vec![(0, vec![0xC0, 0]), (0, vec![0xC1, 48]), (0, vec![0xC2, 33])];
        let tune = [60u8, 62, 64, 65, 67, 69, 71, 72];
        for (i, &n) in tune.iter().enumerate() {
            let at = i as u32 * 96;
            mel.push((at, vec![0x90, n, 100]));
            mel.push((at + 90, vec![0x80, n, 0]));
        }
        for (i, root) in [48u8, 53, 55, 48].iter().enumerate() {
            let at = i as u32 * 192;
            for k in [0u8, 4, 7] {
                mel.push((at, vec![0x91, root + k, 80]));
            }
            mel.push((at, vec![0x92, root - 12, 100]));
            for k in [0u8, 4, 7] {
                mel.push((at + 180, vec![0x81, root + k, 0]));
            }
            mel.push((at + 180, vec![0x82, root - 12, 0]));
        }
        mel.sort_by_key(|e| e.0);
        b.extend(track(&mel));
        let mut drums = Vec::new();
        for i in 0..16u32 {
            let at = i * 48;
            drums.push((at, vec![0x99, 42, 90]));
            if i % 4 == 0 {
                drums.push((at, vec![0x99, 36, 120]));
            }
            if i % 4 == 2 {
                drums.push((at, vec![0x99, 38, 110]));
            }
        }
        b.extend(track(&drums));
        b
    }

    fn play(p: &mut Player) -> Vec<f32> {
        let mut all = Vec::new();
        let mut buf = vec![0.0; 1024];
        while p.render(&mut buf) {
            all.extend_from_slice(&buf);
            assert!(all.len() < RATE as usize * 2 * 600);
        }
        all
    }

    fn events(s: &super::super::midifile::Song) -> Vec<(u64, Vec<u8>)> {
        s.events.iter().map(|e| (e.at_us, e.bytes.clone())).collect()
    }

    #[test]
    fn a_song_end_to_end_and_deterministic() {
        let song = super::super::midifile::parse(&song()).unwrap();
        let a = play(&mut Player::new(events(&song), RATE, 0, 1.0));
        sane(&a);
        assert!(rms(&a) > 0.02);
        assert!(a.len() as u64 / 2 >= song.length_us * u64::from(RATE) / 1_000_000);
        let b = play(&mut Player::new(events(&song), RATE, 0, 1.0));
        assert_eq!(a, b);
        // (from the middle)
        let c = play(&mut Player::new(events(&song), RATE, song.length_us / 2, 0.8));
        assert!(rms(&c) > 0.01 && c.len() < a.len());
    }

    // ---- listening and speed (ignored: run by hand) ----

    fn scratch() -> std::path::PathBuf {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scratch");
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn write_wav(path: &std::path::Path, x: &[f32]) {
        let mut b = b"RIFF".to_vec();
        let data = (x.len() * 2) as u32;
        b.extend((36 + data).to_le_bytes());
        b.extend(b"WAVEfmt ");
        b.extend(16u32.to_le_bytes());
        b.extend([1, 0, 2, 0]);
        b.extend(RATE.to_le_bytes());
        b.extend((RATE * 4).to_le_bytes());
        b.extend([4, 0, 16, 0]);
        b.extend(b"data");
        b.extend(data.to_le_bytes());
        for v in x {
            b.extend(((v.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
        }
        std::fs::write(path, b).unwrap();
    }

    /// A dense song: 15 channels of chords and runs plus drums, ~40 voices.
    fn dense(secs: u64) -> Vec<(u64, Vec<u8>)> {
        let mut ev = Vec::new();
        let programs = [0u8, 4, 16, 24, 33, 40, 48, 52, 56, 61, 65, 73, 80, 88, 89];
        for (k, &p) in programs.iter().enumerate() {
            let ch = if k >= 9 { k + 1 } else { k } as u8;
            ev.push((0, vec![0xC0 | ch, p]));
            ev.push((0, vec![0xB0 | ch, 10, (k * 9) as u8]));
        }
        let step = 125_000u64;
        for i in 0..secs * 8 {
            let at = i * step;
            for (k, _) in programs.iter().enumerate() {
                let ch = if k >= 9 { k + 1 } else { k } as u8;
                let n = 36 + ((k as u64 * 5 + i * (k as u64 % 3 + 1) * 7) % 48) as u8;
                if i % (1 + k as u64 % 4) == 0 {
                    ev.push((at, vec![0x90 | ch, n, 70 + (k as u8 * 3)]));
                    ev.push((at + step * (1 + k as u64 % 4) - 1000, vec![0x80 | ch, n, 0]));
                }
            }
            ev.push((at, vec![0x99, 42, 90]));
            if i % 4 == 0 {
                ev.push((at, vec![0x99, 36, 120]));
            }
            if i % 8 == 4 {
                ev.push((at, vec![0x99, 38, 110]));
            }
            if i % 32 == 0 {
                ev.push((at, vec![0x99, 49, 100]));
            }
        }
        ev.sort_by_key(|e| e.0);
        ev
    }

    fn timed(name: &str, ev: Vec<(u64, Vec<u8>)>) -> Vec<f32> {
        let t0 = std::time::Instant::now();
        let mut p = Player::new(ev, RATE, 0, 1.0);
        let mut all = Vec::new();
        let mut buf = vec![0.0; 1024];
        let mut most = 0;
        while p.render(&mut buf) {
            all.extend_from_slice(&buf);
            most = most.max(p.synth.active());
        }
        let took = t0.elapsed().as_secs_f64();
        let secs = all.len() as f64 / 2.0 / f64::from(RATE);
        println!("{name}: {secs:.1} s of audio in {took:.3} s = {:.0}x real time (up to {most} voices, peak {:.2}, rms {:.3})", secs / took, peak(&all), rms(&all));
        assert!(secs / took > 1.0);
        assert!(rms(&all) > 0.005);
        all
    }

    /// The .mid files around (`~/Downloads/Rapidq`, `RAPIDR_WINDOWS_MEDIA`,
    /// the repo's fixtures) and a dense made-up song, rendered and timed;
    /// each written to scratch/ as a WAV to listen to.
    #[test]
    #[ignore]
    fn render_files_and_speed() {
        fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            let Ok(rd) = std::fs::read_dir(dir) else { return };
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, out);
                } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("mid") || x.eq_ignore_ascii_case("rmi")) {
                    out.push(p);
                }
            }
        }
        let mut files = Vec::new();
        if let Ok(home) = std::env::var("HOME") {
            walk(&std::path::Path::new(&home).join("Downloads/Rapidq"), &mut files);
        }
        if let Ok(dir) = std::env::var("RAPIDR_WINDOWS_MEDIA") {
            walk(std::path::Path::new(&dir), &mut files);
        }
        walk(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures"), &mut files);
        let out = scratch();
        for f in files {
            let Some(song) = super::super::midifile::parse(&std::fs::read(&f).unwrap()) else { continue };
            let name = f.file_stem().unwrap().to_string_lossy().to_string();
            let all = timed(&name, events(&song));
            write_wav(&out.join(format!("synth_{name}.wav")), &all);
        }
        let all = timed("dense", dense(30));
        write_wav(&out.join("synth_dense.wav"), &all);
        let s = super::super::midifile::parse(&song()).unwrap();
        write_wav(&out.join("synth_song.wav"), &timed("song", events(&s)));
    }

    /// Every program (a rising arpeggio each) and the kit, to scratch/synth_programs.wav.
    #[test]
    #[ignore]
    fn programs_wav() {
        let mut ev = Vec::new();
        let mut at = 0u64;
        for p in 0..128u8 {
            ev.push((at, vec![0xC0, p]));
            for (i, n) in [48u8, 55, 60, 64, 67].iter().enumerate() {
                let t = at + i as u64 * 120_000;
                ev.push((t, vec![0x90, *n, 100]));
                ev.push((t + 500_000, vec![0x80, *n, 0]));
            }
            at += 1_400_000;
        }
        for n in 35..=81u8 {
            ev.push((at, vec![0x99, n, 110]));
            at += 400_000;
        }
        ev.sort_by_key(|e| e.0);
        let all = timed("programs", ev);
        write_wav(&scratch().join("synth_programs.wav"), &all);
    }
}

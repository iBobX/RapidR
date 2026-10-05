//! PLAYWAV (RapidQ manual): plays a .WAV file or `$RESOURCE`
//! (rapidr_value::resources handle). One sound at a time: a new one
//! replaces the one playing; `PLAYWAV ""` stops it. Options (RAPIDQ.INC):
//! SND_SYNC = 0 waits for the end, SND_ASYNC = 1 plays in the background,
//! SND_LOOP = 8 (the manual says 3) repeats it. Without a sound device (or
//! the audio feature) nothing is played.

use crate::value::Value;

/// Plays `source` (a file name or a resource handle) with `options`.
pub fn rp_playwav(source: &Value, options: &Value) {
    let options = options.to_i64();
    let looped = options & 8 != 0 || options == 3;
    let wait = options & 1 == 0;
    let bytes = match source {
        Value::Integer(_) | Value::Double(_) => crate::value::resources::bytes(source.to_i64()).map(|b| b.to_vec()),
        _ => {
            let path = source.to_string_val();
            if path.is_empty() {
                stop();
                return;
            }
            match std::fs::read(&path) {
                Ok(b) => Some(b),
                Err(e) => {
                    eprintln!("[rapidr] PLAYWAV: can't read {path}: {e}");
                    return;
                }
            }
        }
    };
    let Some(bytes) = bytes else {
        eprintln!("[rapidr] PLAYWAV: no resource {}", source.to_i64());
        return;
    };
    play(bytes, looped, wait);
}

#[cfg(feature = "audio")]
thread_local! {
    /// The sound device, opened by the first PLAYWAV and kept open (opening
    /// it takes a moment), and the sound playing.
    static OUTPUT: std::cell::RefCell<Option<(rodio::OutputStream, rodio::OutputStreamHandle)>> = const { std::cell::RefCell::new(None) };
    static SOUND: std::cell::RefCell<Option<rodio::Sink>> = const { std::cell::RefCell::new(None) };
}

#[cfg(feature = "audio")]
fn stop() {
    SOUND.with(|s| {
        if let Some(sink) = s.borrow_mut().take() {
            sink.stop();
        }
    });
}

#[cfg(feature = "audio")]
fn play(bytes: Vec<u8>, looped: bool, wait: bool) {
    use rodio::Source;
    stop();
    let handle = OUTPUT.with(|o| {
        let mut o = o.borrow_mut();
        if o.is_none() {
            *o = rodio::OutputStream::try_default().ok();
        }
        o.as_ref().map(|(_, h)| h.clone())
    });
    let Some(handle) = handle else {
        eprintln!("[rapidr] PLAYWAV: no sound device");
        return;
    };
    let Ok(sink) = rodio::Sink::try_new(&handle) else { return };
    let decoded = match decode(bytes) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[rapidr] PLAYWAV: can't play it: {e}");
            return;
        }
    };
    if looped {
        sink.append(decoded.buffered().repeat_infinite());
    } else {
        sink.append(decoded);
    }
    if wait && !looped {
        sink.sleep_until_end();
        return;
    }
    SOUND.with(|s| *s.borrow_mut() = Some(sink));
}

/// A sound file's samples (PLAYWAV, PLAYSOUND): rodio decodes WAV, Ogg
/// Vorbis and FLAC, nanomp3 MP3 (Cargo.toml).
#[cfg(feature = "audio")]
pub(crate) fn decode(bytes: Vec<u8>) -> Result<Box<dyn rodio::Source<Item = f32> + Send>, String> {
    use rodio::Source;
    let rodio_format = [&b"RIFF"[..], b"OggS", b"fLaC"].iter().any(|m| bytes.starts_with(m));
    if !rodio_format && nanomp3::detect(&bytes) {
        return Mp3::new(bytes).map(|s| Box::new(s) as Box<dyn rodio::Source<Item = f32> + Send>);
    }
    rodio::Decoder::new(std::io::Cursor::new(bytes)).map(|d| Box::new(d.convert_samples()) as Box<dyn rodio::Source<Item = f32> + Send>).map_err(|e| e.to_string())
}

/// An MP3 stream as a rodio source, a frame at a time.
#[cfg(feature = "audio")]
struct Mp3 {
    reader: nanomp3::Reader<std::io::Cursor<Vec<u8>>, f32>,
    frame: Vec<f32>,
    at: usize,
    channels: u16,
    rate: u32,
    total: Option<u64>,
}

#[cfg(feature = "audio")]
impl Mp3 {
    fn new(bytes: Vec<u8>) -> Result<Mp3, String> {
        let reader = nanomp3::Reader::new(std::io::Cursor::new(bytes)).map_err(|e| format!("MP3: {e}"))?;
        let channels = match reader.channels() {
            Some(nanomp3::Channels::Mono) => 1,
            Some(_) => 2,
            None => return Err("MP3: no audio".into()),
        };
        let (rate, total) = (reader.sample_rate(), reader.total_samples());
        let mut mp3 = Mp3 { reader, frame: Vec::new(), at: 0, channels, rate, total };
        mp3.next_frame();
        Ok(mp3)
    }

    fn next_frame(&mut self) {
        self.at = 0;
        self.frame.clear();
        if let Ok(Some(samples)) = self.reader.read_frame() {
            self.frame.extend_from_slice(samples);
        }
    }
}

#[cfg(feature = "audio")]
impl Iterator for Mp3 {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.at >= self.frame.len() {
            self.next_frame();
        }
        let s = self.frame.get(self.at).copied()?;
        self.at += 1;
        Some(s)
    }
}

#[cfg(feature = "audio")]
impl rodio::Source for Mp3 {
    // (one format to the end: nanomp3 stops at a change of rate or channels)
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.rate
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        self.total.map(|n| std::time::Duration::from_secs_f64(n as f64 / f64::from(self.rate.max(1))))
    }
}

#[cfg(not(feature = "audio"))]
fn stop() {}

#[cfg(not(feature = "audio"))]
fn play(_bytes: Vec<u8>, _looped: bool, _wait: bool) {
    eprintln!("[rapidr] PLAYWAV: built without audio");
}

// ------------------------------------------------- QDXSOUND's device --
//
// (the DirectX lane's) A QDXSOUND's model (rapidr_value::objects::directx::
// DxSound) keeps Playing and Position by the clock; this plays what it
// asks for on the sound device, a sink per QDXSOUND: the WAV's frames as
// stereo with Pan's gains, from a frame, sped up or slowed down to
// Frequency, looped or once, at Volume's gain. Under the GUI tests (and
// without the audio feature) nothing plays.

/// Whether a test runs the program: a GUI test (`RAPIDR_CAPTURE`,
/// `RAPIDR_TEST_EVENTS`) or the conformance runner (`RAPIDR_TEST_SOUND`,
/// set even empty) — no sound device then, nothing heard.
#[cfg(feature = "audio")]
pub(crate) fn testing() -> bool {
    ["RAPIDR_CAPTURE", "RAPIDR_TEST_EVENTS", "RAPIDR_TEST_SOUND"].iter().any(|v| std::env::var_os(v).is_some())
}

/// Gives QDXSOUND the sound device (once; not under a test).
pub fn install_dx_device() {
    #[cfg(feature = "audio")]
    {
        if !testing() {
            rapidr_value::objects::directx::set_sound_device(rapidr_value::objects::directx::SoundDevice { play: dx_play, volume: dx_volume, stop: dx_stop });
        }
    }
}

#[cfg(feature = "audio")]
thread_local! {
    static DX_SINKS: std::cell::RefCell<std::collections::HashMap<String, rodio::Sink>> = std::cell::RefCell::new(std::collections::HashMap::new());
}

#[cfg(feature = "audio")]
fn dx_play(p: &rapidr_value::objects::directx::SoundPlay) {
    use rodio::Source;
    dx_stop(&p.id);
    let handle = OUTPUT.with(|o| {
        let mut o = o.borrow_mut();
        if o.is_none() {
            *o = rodio::OutputStream::try_default().ok();
        }
        o.as_ref().map(|(_, h)| h.clone())
    });
    let Some(handle) = handle else { return };
    let Ok(sink) = rodio::Sink::try_new(&handle) else { return };
    let buffer = rodio::buffer::SamplesBuffer::new(2, p.wav.rate, p.wav.stereo(p.pan));
    let from = std::time::Duration::from_secs_f64(p.from as f64 / f64::from(p.wav.rate.max(1)));
    let speed = p.speed.clamp(0.01, 100.0) as f32;
    if p.looped {
        sink.append(buffer.buffered().repeat_infinite().skip_duration(from).speed(speed));
    } else {
        sink.append(buffer.skip_duration(from).speed(speed));
    }
    sink.set_volume(p.gain);
    DX_SINKS.with(|s| s.borrow_mut().insert(p.id.clone(), sink));
}

/// Plays `source` on the sound device as `id`'s (QMIDI's built-in
/// synthesizer: media.rs), replacing what `id` played; `false` without a
/// device.
#[cfg(feature = "audio")]
pub(crate) fn play_source(id: &str, source: impl rodio::Source<Item = f32> + Send + 'static) -> bool {
    dx_stop(id);
    let handle = OUTPUT.with(|o| {
        let mut o = o.borrow_mut();
        if o.is_none() {
            *o = rodio::OutputStream::try_default().ok();
        }
        o.as_ref().map(|(_, h)| h.clone())
    });
    let Some(handle) = handle else { return false };
    let Ok(sink) = rodio::Sink::try_new(&handle) else { return false };
    sink.append(source);
    DX_SINKS.with(|s| s.borrow_mut().insert(id.to_string(), sink));
    true
}

/// Stops what [`play_source`] plays as `id`.
#[cfg(feature = "audio")]
pub(crate) fn stop_source(id: &str) {
    dx_stop(id);
}

#[cfg(feature = "audio")]
fn dx_volume(id: &str, gain: f32) {
    DX_SINKS.with(|s| {
        if let Some(sink) = s.borrow().get(id) {
            sink.set_volume(gain);
        }
    });
}

#[cfg(feature = "audio")]
fn dx_stop(id: &str) {
    DX_SINKS.with(|s| {
        if let Some(sink) = s.borrow_mut().remove(id) {
            sink.stop();
        }
    });
}

#[cfg(all(test, feature = "audio"))]
mod tests {
    use rodio::Source;

    /// MPEG-1 Layer III frames of silence (128 kbit/s, 44.1 kHz, joint
    /// stereo): a header, then side information and data all zero.
    fn silent_mp3(frames: usize) -> Vec<u8> {
        let mut frame = vec![0u8; 417];
        frame[..4].copy_from_slice(&[0xFF, 0xFB, 0x90, 0x64]);
        frame.repeat(frames)
    }

    /// A 16-bit PCM WAV of mono samples at 8 kHz.
    fn wav(samples: &[i16]) -> Vec<u8> {
        let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        let mut w = b"RIFF".to_vec();
        w.extend((36 + data.len() as u32).to_le_bytes());
        w.extend(b"WAVEfmt ");
        w.extend(16u32.to_le_bytes());
        w.extend(1u16.to_le_bytes()); // PCM
        w.extend(1u16.to_le_bytes()); // mono
        w.extend(8000u32.to_le_bytes());
        w.extend(16000u32.to_le_bytes());
        w.extend(2u16.to_le_bytes());
        w.extend(16u16.to_le_bytes());
        w.extend(b"data");
        w.extend((data.len() as u32).to_le_bytes());
        w.extend(data);
        w
    }

    #[test]
    fn decodes_mp3_with_nanomp3() {
        let source = super::decode(silent_mp3(20)).expect("an MP3");
        assert_eq!((source.channels(), source.sample_rate()), (2, 44100));
        let samples: Vec<f32> = source.collect();
        assert!(samples.len() >= 19 * 1152 * 2, "{} samples", samples.len());
        assert!(samples.iter().all(|s| *s == 0.0));
    }

    #[test]
    fn decodes_wav_with_rodio() {
        let source = super::decode(wav(&[0, 16384, -16384, 32767])).expect("a WAV");
        assert_eq!((source.channels(), source.sample_rate()), (1, 8000));
        let samples: Vec<f32> = source.collect();
        assert_eq!(samples.len(), 4);
        assert!((samples[1] - 0.5).abs() < 0.001 && (samples[2] + 0.5).abs() < 0.001);
    }

    #[test]
    fn refuses_what_isnt_sound() {
        assert!(super::decode(b"not a sound file at all".repeat(100)).is_err());
    }
}

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
    let decoded = match rodio::Decoder::new(std::io::Cursor::new(bytes)) {
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

#[cfg(not(feature = "audio"))]
fn stop() {}

#[cfg(not(feature = "audio"))]
fn play(_bytes: Vec<u8>, _looped: bool, _wait: bool) {
    eprintln!("[rapidr] PLAYWAV: built without audio");
}

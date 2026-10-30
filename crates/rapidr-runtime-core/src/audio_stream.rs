//! RAudioStream on the desktop (interpreted and native programs): the
//! model is `rapidr_value::objects::audiostream` (the web's too); this is
//! its sound device and its OnNeedData.
//!
//! - **The device.** Each stream is a rodio source on the sound device
//!   sound.rs opened (rodio, MIT OR Apache-2.0: converts the stream's rate
//!   and channels to the device's). Its samples come from the stream's
//!   lock-free queue on the device's own thread ([`StreamSource`]), which
//!   never waits for the program.
//! - **No device** (none on the computer, or a build without sound): a
//!   clocked sink takes the samples in real time on a thread of its own and
//!   plays nothing, so the program's timing goes on; Error says why.
//! - **Under the tests** (sound.rs' `testing`: the GUI tests, the
//!   conformance runner) the same clocked sink, keeping what it heard
//!   (`__Heard`) — never the sound card.
//! - **OnNeedData** is raised on the program's thread: the window loop
//!   asks every 5 ms while a stream with a handler plays (ui/kernel.rs),
//!   and so does DOEVENTS.

use std::time::{Duration, Instant};

use rapidr_value::objects::audiostream::{self, Consumer, Device};

use crate::value::Value;

/// How often the window loop asks while a stream with OnNeedData plays.
pub const POLL_EVERY: Duration = Duration::from_millis(5);

/// Gives RAudioStream its device (once: the first stream made).
pub fn install() {
    thread_local! {
        static DONE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }
    if DONE.with(|d| d.replace(true)) {
        return;
    }
    audiostream::set_device(Device { start, stop });
    // (the sound device opened now, when the stream is made: opening it
    // takes a moment — 200 ms on a Bluetooth headset — that Start then
    // doesn't spend while the program's sound is due)
    #[cfg(feature = "audio")]
    if !crate::sound::testing() {
        let _ = crate::sound::has_output();
    }
}

/// The sound device's name for stream `id`'s source (sound.rs keeps one per name).
#[cfg(feature = "audio")]
fn key(id: &str) -> String {
    format!("audiostream:{}", id.to_lowercase())
}

fn start(id: &str, consumer: Consumer) -> Result<(), String> {
    let _ = id;
    #[cfg(feature = "audio")]
    {
        if crate::sound::testing() {
            clocked(consumer, true);
            return Ok(());
        }
        if crate::sound::has_output() {
            if crate::sound::play_source(&key(id), StreamSource::new(consumer)) {
                return Ok(());
            }
            return Err("The sound device refused the stream: it plays silently".into());
        }
        clocked(consumer, false);
        Err("No sound device on this computer: the stream plays silently (its timing goes on)".into())
    }
    #[cfg(not(feature = "audio"))]
    {
        let testing = ["RAPIDR_CAPTURE", "RAPIDR_TEST_EVENTS", "RAPIDR_TEST_SOUND"].iter().any(|v| std::env::var_os(v).is_some());
        clocked(consumer, testing);
        if testing {
            Ok(())
        } else {
            Err("This program was built without sound: the stream plays silently (its timing goes on)".into())
        }
    }
}

fn stop(id: &str) {
    let _ = id;
    #[cfg(feature = "audio")]
    crate::sound::stop_source(&key(id));
}

/// A sink that takes the stream's samples in real time and plays nothing
/// (`record`: keeping what it heard, for the tests).
fn clocked(mut consumer: Consumer, record: bool) {
    if record {
        consumer.shared().record();
    }
    let spawned = std::thread::Builder::new().name("rapidr-audiostream".into()).spawn(move || {
        let rate = f64::from(consumer.rate());
        let ch = consumer.channels();
        let began = Instant::now();
        let mut taken: u64 = 0;
        let mut buf = Vec::new();
        loop {
            std::thread::sleep(Duration::from_millis(5));
            let due = (began.elapsed().as_secs_f64() * rate) as u64;
            // (at most 200 ms at once: a sleep much too long is a gap)
            let n = due.saturating_sub(taken).min((rate / 5.0) as u64) as usize;
            taken = due;
            if n == 0 {
                continue;
            }
            buf.resize(n * ch, 0.0);
            if !consumer.render(&mut buf) {
                break;
            }
        }
    });
    if let Err(e) = spawned {
        eprintln!("[rapidr] RAudioStream: no thread for its sink: {e}");
    }
}

/// A stream as a rodio source: its samples from the queue a block at a
/// time (the device's thread), ending once the stream stopped and faded.
#[cfg(feature = "audio")]
struct StreamSource {
    consumer: Consumer,
    buf: Vec<f32>,
    at: usize,
    alive: bool,
    channels: u16,
    rate: u32,
}

#[cfg(feature = "audio")]
impl StreamSource {
    fn new(consumer: Consumer) -> StreamSource {
        let (channels, rate) = (consumer.channels() as u16, consumer.rate());
        // (256 frames a block: under 6 ms at 44.1 kHz)
        StreamSource { buf: vec![0.0; 256 * usize::from(channels)], at: usize::MAX, consumer, alive: true, channels, rate }
    }
}

#[cfg(feature = "audio")]
impl Iterator for StreamSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.at >= self.buf.len() {
            if !self.alive {
                return None;
            }
            self.alive = self.consumer.render(&mut self.buf);
            self.at = 0;
        }
        let s = self.buf[self.at];
        self.at += 1;
        Some(s)
    }
}

#[cfg(feature = "audio")]
impl rodio::Source for StreamSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

thread_local! {
    /// The clock OnNeedData's asking runs on.
    static EPOCH: Instant = Instant::now();
}

/// Whether a stream with an OnNeedData plays (the window loop then asks
/// every [`POLL_EVERY`]).
pub fn busy() -> bool {
    audiostream::active() && audiostream::playing_ids().iter().any(|id| crate::object::rp_has_handler(id, "onneeddata"))
}

/// OnNeedData for each stream that wants more, on the program's thread.
pub fn poll() {
    if !audiostream::active() {
        return;
    }
    let now = EPOCH.with(|e| e.elapsed().as_secs_f64() * 1000.0);
    for (id, count) in audiostream::due(now, &|id| crate::object::rp_has_handler(id, "onneeddata")) {
        crate::object::rp_fire_event_args(&id, "onneeddata", &[Value::Integer(count)]);
    }
}

#[cfg(all(test, feature = "audio"))]
mod tests {
    use rodio::Source;

    /// rodio converts the stream's rate to the device's: 22050 Hz mono read
    /// at 44100 Hz stereo is twice the frames, each on both channels.
    #[test]
    fn rodio_converts_the_rate_and_channels() {
        let id = "rt_conv";
        rapidr_value::objects::audiostream::create(id, "RAUDIOSTREAM");
        let _ = rapidr_value::objects::audiostream::set(id, "samplerate", &crate::value::Value::Integer(22050));
        // (a consumer straight from the model: started with a device that hands it over)
        thread_local! { static TAKEN: std::cell::RefCell<Option<super::StreamSource>> = const { std::cell::RefCell::new(None) }; }
        fn take(_: &str, c: rapidr_value::objects::audiostream::Consumer) -> Result<(), String> {
            TAKEN.with(|t| *t.borrow_mut() = Some(super::StreamSource::new(c)));
            Ok(())
        }
        fn none(_: &str) {}
        rapidr_value::objects::audiostream::set_device(rapidr_value::objects::audiostream::Device { start: take, stop: none });
        let ramp: Vec<crate::value::Value> = (0..2205).map(|i| crate::value::Value::Integer(i * 10)).collect();
        let arr = crate::value::Value::Array(std::rc::Rc::new(std::cell::RefCell::new(rapidr_value::BasicArray { bounds: vec![(0, 2204)], data: ramp, overflow: Default::default() })));
        rapidr_value::objects::audiostream::call(id, "write", &[arr]);
        rapidr_value::objects::audiostream::call(id, "start", &[]);
        let source = TAKEN.with(|t| t.borrow_mut().take()).expect("started");
        assert_eq!((source.sample_rate(), source.channels()), (22050, 1));
        let converted = rodio::source::UniformSourceIterator::<_, f32>::new(source.take_duration(Duration::from_millis(100)), 2, 44100);
        let out: Vec<f32> = converted.collect();
        assert!((out.len() as i64 - 8820).abs() <= 4, "{} samples", out.len());
        // (left = right, and the ramp rises)
        assert!(out.chunks_exact(2).all(|f| f[0] == f[1]));
        assert!(out[2000] < out[4000] && out[4000] < out[8000], "{} {} {}", out[2000], out[4000], out[8000]);
    }

    use std::time::Duration;
}

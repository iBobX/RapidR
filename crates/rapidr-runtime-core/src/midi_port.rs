//! RMIDIPort on the desktop (interpreted and native programs): the model is
//! `rapidr_value::objects::midiport` (the web's too); this is its devices
//! and its delivery (docs/rmidiport-design.md).
//!
//! - **The devices** are the system's through midir (MIT; CoreMIDI on
//!   macOS, the ALSA sequencer on Linux — the system's libasound, linked
//!   dynamically —, winmm on Windows; docs/legal/review-2026-11-05-midi.md).
//!   midir calls back on a thread of its own: there the bytes only go
//!   through the model's parser into the input's inbox, and the window loop
//!   is woken ([`crate::object::rp_wake`]). No program code ever runs there.
//! - **Delivery**: the window loop's turn (ui/kernel.rs) and DOEVENTS call
//!   [`poll`], which fires the events on the program's thread.
//! - **Under the tests** `RAPIDR_TEST_MIDIPORT` (set, even empty, by the
//!   test runners) gives the model's scripted devices instead, so a test
//!   never opens a real one.
//! - A build without the `midi` feature has no devices: Error says so.

use std::rc::Rc;
use std::sync::OnceLock;
use std::time::Instant;

use rapidr_value::objects::midiport::{self, Host, TestMidi, Then};

use crate::value::{v_int, v_str, Value};

static EPOCH: OnceLock<Instant> = OnceLock::new();

/// The clock messages are stamped with (any thread): ms since the first use.
fn clock_ms() -> f64 {
    EPOCH.get_or_init(Instant::now).elapsed().as_secs_f64() * 1000.0
}

/// Gives RMIDIPort its devices (once: the first one made).
pub fn install() {
    thread_local! {
        static DONE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }
    if DONE.with(|d| d.replace(true)) {
        return;
    }
    let _ = clock_ms();
    midiport::set_clock(clock_ms);
    midiport::set_waker(crate::object::rp_wake);
    match rapidr_testmode::env_var("RAPIDR_TEST_MIDIPORT") {
        Ok(script) => midiport::set_system(Rc::new(TestMidi::parse(&script))),
        #[cfg(feature = "midi")]
        Err(_) => midiport::set_system(Rc::new(system::SystemMidi::default())),
        #[cfg(not(feature = "midi"))]
        Err(_) => {}
    }
}

fn fire_then(name: &str, event: &str, args: &[Value], then: Then) {
    crate::object::rp_fire_event_then(name, event, args, then);
}

fn host() -> Host {
    Host { has_handler: crate::object::rp_has_handler, fire_then, now_ms: clock_ms }
}

/// What arrived, delivered on the program's thread; the devices looked at
/// (about each second) for OnDevicesChanged and unplugged ones. The window
/// loop's turn and DOEVENTS call it.
pub fn poll() {
    if !midiport::system_installed() {
        return;
    }
    midiport::look(host());
    midiport::pump(host());
}

/// Whether a port is open (the window loop then turns at least every so
/// often, for the devices' looks).
pub fn busy() -> bool {
    midiport::active()
}

/// FillInputs / FillOutputs(List): the names put in a list or combo box
/// (cleared first; the open one selected, none when none is): how many.
pub fn fill(name: &str, input: bool, args: &[Value]) -> Value {
    let Some((items, at)) = midiport::items(name, input) else { return v_int(0) };
    let control = args.first().map(Value::to_string_val).unwrap_or_default();
    if control.is_empty() || crate::object::rp_comp_type(&control).is_empty() {
        return v_int(items.len() as i64);
    }
    crate::object::rp_comp_method(&control, "clear", &[]);
    for item in &items {
        crate::object::rp_comp_method(&control, "additems", &[v_str(item)]);
    }
    // (the open one selected; none when none is open: a list never shows
    // a device as chosen that isn't)
    crate::object::rp_comp_set(&control, "itemindex", v_int(at.map_or(-1, |i| i as i64)));
    v_int(items.len() as i64)
}

#[cfg(feature = "midi")]
mod system {
    use std::any::Any;
    use std::sync::Arc;

    use rapidr_value::objects::midiport::{Inbox, MidiOut, MidiSystem, OpenError};

    /// The system's devices through midir. The lists are read with one
    /// client kept for it (the devices are looked at each second while a
    /// port is open); each open port has a client of its own.
    #[derive(Default)]
    pub struct SystemMidi {
        lister_in: std::cell::RefCell<Option<midir::MidiInput>>,
        lister_out: std::cell::RefCell<Option<midir::MidiOutput>>,
    }

    /// What the ports are called to the system (a client per use).
    const CLIENT: &str = "RapidR";

    fn names<P>(ports: &[P], name: impl Fn(&P) -> Option<String>) -> Vec<String> {
        ports.iter().filter_map(name).collect()
    }

    /// The `nth` port named `wanted` (0 is the first).
    fn find<P: Clone>(ports: &[P], name: impl Fn(&P) -> Option<String>, wanted: &str, nth: usize) -> Option<P> {
        ports.iter().filter(|p| name(p).as_deref() == Some(wanted)).nth(nth).cloned()
    }

    fn connect_error(text: &str) -> OpenError {
        let t = text.to_lowercase();
        if t.contains("allocated") || t.contains("in use") || t.contains("busy") {
            OpenError::InUse
        } else if t.contains("invalid port") || t.contains("not found") {
            OpenError::NotFound
        } else {
            OpenError::Other(text.to_string())
        }
    }

    fn no_midi(e: impl std::fmt::Display) -> String {
        format!("The system's MIDI service didn't answer: {e}")
    }

    impl MidiSystem for SystemMidi {
        fn inputs(&self) -> Result<Vec<String>, String> {
            let mut slot = self.lister_in.borrow_mut();
            if slot.is_none() {
                *slot = Some(midir::MidiInput::new(CLIENT).map_err(no_midi)?);
            }
            let m = slot.as_ref().ok_or("no MIDI")?;
            Ok(names(&m.ports(), |p| m.port_name(p).ok()))
        }

        fn outputs(&self) -> Result<Vec<String>, String> {
            let mut slot = self.lister_out.borrow_mut();
            if slot.is_none() {
                *slot = Some(midir::MidiOutput::new(CLIENT).map_err(no_midi)?);
            }
            let m = slot.as_ref().ok_or("no MIDI")?;
            Ok(names(&m.ports(), |p| m.port_name(p).ok()))
        }

        fn open_input(&self, name: &str, nth: usize, inbox: Arc<Inbox>) -> Result<Box<dyn Any>, OpenError> {
            let mut m = midir::MidiInput::new(CLIENT).map_err(|e| OpenError::Other(e.to_string()))?;
            // (the model's parser ignores what the program asks it to: the
            // same everywhere)
            m.ignore(midir::Ignore::None);
            let ports = m.ports();
            let port = find(&ports, |p| m.port_name(p).ok(), name, nth).ok_or(OpenError::NotFound)?;
            let conn = m
                .connect(
                    &port,
                    "RapidR RMIDIPort",
                    move |_stamp, bytes, inbox: &mut Arc<Inbox>| {
                        // (the device's thread: parsed and queued, the loop woken)
                        inbox.push(bytes, super::clock_ms());
                    },
                    inbox,
                )
                .map_err(|e| connect_error(&e.to_string()))?;
            Ok(Box::new(conn))
        }

        fn open_output(&self, name: &str, nth: usize) -> Result<Box<dyn MidiOut>, OpenError> {
            let m = midir::MidiOutput::new(CLIENT).map_err(|e| OpenError::Other(e.to_string()))?;
            let ports = m.ports();
            let port = find(&ports, |p| m.port_name(p).ok(), name, nth).ok_or(OpenError::NotFound)?;
            let conn = m.connect(&port, "RapidR RMIDIPort").map_err(|e| connect_error(&e.to_string()))?;
            Ok(Box::new(Out(conn)))
        }
    }

    struct Out(midir::MidiOutputConnection);

    impl MidiOut for Out {
        fn send(&mut self, bytes: &[u8]) -> Result<(), String> {
            // (midir sends one message a call: the bytes split into theirs)
            for m in messages(bytes) {
                self.0.send(m).map_err(|e| format!("The MIDI output didn't take the message: {e}"))?;
            }
            Ok(())
        }
    }

    /// Checked bytes (whole messages) one message at a time.
    pub(super) fn messages(bytes: &[u8]) -> Vec<&[u8]> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < bytes.len() {
            let len = if bytes[i] == 0xF0 {
                bytes[i..].iter().position(|&b| b == 0xF7).map_or(bytes.len() - i, |e| e + 1)
            } else {
                1 + rapidr_value::objects::midiport::data_bytes(bytes[i]).unwrap_or(0)
            };
            let end = (i + len).min(bytes.len());
            out.push(&bytes[i..end]);
            i = end;
        }
        out
    }
}

#[cfg(all(test, feature = "midi"))]
mod tests {
    #[test]
    fn bytes_split_into_messages() {
        let b = [0x90, 60, 100, 0xF0, 1, 2, 0xF7, 0xC0, 5, 0xF8];
        let m = super::system::messages(&b);
        assert_eq!(m, vec![&b[0..3], &b[3..7], &b[7..9], &b[9..10]]);
    }
}

/// The real system path, end to end: a virtual MIDI output made in this
/// process (only while the test runs; nothing of the computer's MIDI setup
/// changes), opened by RMIDIPort as an input through midir. Notes, a
/// control, a 4 KB SysEx and a burst of 2000 notes come back whole and in
/// order. Ignored by default — the virtual port is seen by every MIDI
/// program running (a music program listening to all inputs would hear
/// it): run it with `cargo test -p rapidr-runtime-core loopback --
/// --ignored` when none is (macOS, Linux; Windows' winmm has no virtual
/// ports).
#[cfg(all(test, feature = "midi", any(target_os = "macos", target_os = "linux")))]
mod loopback {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::time::{Duration, Instant};

    use midir::os::unix::VirtualOutput;
    use rapidr_value::objects::midiport::{self, Host};
    use rapidr_value::{v_int, v_str, Value};

    thread_local! {
        static GOT: RefCell<Vec<(String, Vec<Value>)>> = const { RefCell::new(Vec::new()) };
    }

    fn every(_: &str, e: &str) -> bool {
        e == "onmessage" || e == "onsysex" || e == "onerror"
    }
    fn record(_: &str, e: &str, args: &[Value], then: midiport::Then) {
        GOT.with(|g| g.borrow_mut().push((e.to_string(), args.to_vec())));
        then(args);
    }

    fn until(secs: u64, mut done: impl FnMut() -> bool) -> bool {
        let t = Instant::now();
        while t.elapsed() < Duration::from_secs(secs) {
            if done() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        done()
    }

    #[test]
    #[ignore = "opens a virtual MIDI port on the system's MIDI service: --ignored, with no music program running"]
    fn a_virtual_port_loops_back() {
        let name = format!("RapidR loop test {}", std::process::id());
        let Ok(out) = midir::MidiOutput::new("RapidR test") else {
            eprintln!("no MIDI service here: skipped");
            return;
        };
        let mut virt = out.create_virtual(&name).expect("a virtual MIDI output");
        midiport::set_clock(super::clock_ms);
        midiport::set_system(Rc::new(super::system::SystemMidi::default()));
        midiport::create("lb", midiport::TYPE);
        midiport::set("lb", "ignoresysex", &v_int(0));
        // (a new port reaches the system's list a moment later; ALSA names
        // it "client:port 128:0", CoreMIDI by the port's own name)
        let mut listed = String::new();
        until(3, || {
            let n = midiport::call("lb", "inputcount", &[]).map_or(0, |v| v.to_i64());
            if let Some(found) = (0..n).map(|i| midiport::call("lb", "inputdevice", &[v_int(i)]).map(|v| v.to_string_val()).unwrap_or_default()).find(|d| d.contains(&name)) {
                listed = found;
            }
            !listed.is_empty()
        });
        assert!(!listed.is_empty(), "the virtual port wasn't listed");
        println!("listed as \"{listed}\"");
        assert_eq!(midiport::call("lb", "openinput", &[v_str(&name)]), Some(v_int(1)), "{:?}", midiport::get("lb", "error"));
        assert_eq!(midiport::get("lb", "inputname"), Some(v_str(&listed)));
        std::thread::sleep(Duration::from_millis(100));
        virt.send(&[0x90, 60, 100]).unwrap();
        virt.send(&[0x80, 60, 0]).unwrap();
        virt.send(&[0xB0, 64, 127]).unwrap();
        // (0x7D: the ID for tests and education — never a format of ours)
        let mut sysex = vec![0xF0, 0x7D];
        sysex.extend((0..4096).map(|i| (i % 128) as u8));
        sysex.push(0xF7);
        virt.send(&sysex).unwrap();
        // (2000 notes in about 70 ms: twenty times what a MIDI cable carries;
        // a pause every 32 lets ALSA's own event pool between two clients,
        // a few hundred events, drain — sent all at once, the kernel drops
        // what overflows it before any program sees it)
        for i in 0..2000u32 {
            let m = [0x90 | (i % 16) as u8, (i % 128) as u8, 1 + (i % 127) as u8];
            while virt.send(&m).is_err() {
                std::thread::sleep(Duration::from_millis(1));
            }
            if i % 32 == 31 {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        let host = Host { has_handler: every, fire_then: record, now_ms: super::clock_ms };
        let all = until(10, || {
            midiport::pump(host);
            GOT.with(|g| g.borrow().len()) >= 2004
        });
        let got = GOT.with(|g| g.borrow().clone());
        assert!(all, "{} of 2004 arrived ({:?} dropped by RapidR's queue)", got.len(), midiport::get("lb", "dropped"));
        let short = |e: &(String, Vec<Value>)| e.1.iter().map(Value::to_i64).collect::<Vec<_>>();
        assert_eq!(got[0].0, "onmessage");
        assert_eq!(short(&got[0]), vec![0x90, 60, 100]);
        assert_eq!(short(&got[1]), vec![0x80, 60, 0]);
        assert_eq!(short(&got[2]), vec![0xB0, 64, 127]);
        assert_eq!(got[3].0, "onsysex");
        assert_eq!(rapidr_value::objects::codec::string_to_bytes(&got[3].1[0].to_string_val()), sysex);
        for (i, e) in got[4..].iter().enumerate() {
            let i = i as i64;
            assert_eq!(short(e), vec![0x90 | (i % 16), i % 128, 1 + i % 127], "message {i}");
        }
        assert_eq!(midiport::get("lb", "dropped"), Some(v_int(0)));
        midiport::remove("lb");
    }
}

/// The system's devices as RMIDIPort lists them, printed; on Windows also a
/// GS Reset (Roland's documented SysEx: the synthesizer goes back to its
/// start-up settings, no sound) sent to the Microsoft GS Wavetable Synth
/// through SendHex. Ignored by default (it opens a real output): `cargo
/// test -p rapidr-runtime-core system_devices -- --ignored --nocapture`.
#[cfg(all(test, feature = "midi"))]
mod system_devices {
    use std::rc::Rc;

    use rapidr_value::objects::midiport;
    use rapidr_value::{v_int, v_str};

    #[test]
    #[ignore = "lists the system's MIDI devices and, on Windows, resets the GS Wavetable Synth"]
    fn system_devices() {
        midiport::set_clock(super::clock_ms);
        midiport::set_system(Rc::new(super::system::SystemMidi::default()));
        midiport::create("sys", midiport::TYPE);
        let n_in = midiport::call("sys", "inputcount", &[]).map_or(0, |v| v.to_i64());
        let n_out = midiport::call("sys", "outputcount", &[]).map_or(0, |v| v.to_i64());
        println!("inputs {n_in}, outputs {n_out}, error [{}]", midiport::get("sys", "error").map(|v| v.to_string_val()).unwrap_or_default());
        for i in 0..n_in {
            println!("  in {i}: {}", midiport::call("sys", "inputdevice", &[v_int(i)]).unwrap().to_string_val());
        }
        let mut gs = None;
        for i in 0..n_out {
            let name = midiport::call("sys", "outputdevice", &[v_int(i)]).unwrap().to_string_val();
            if name.contains("GS Wavetable") {
                gs = Some(name.clone());
            }
            println!("  out {i}: {name}");
        }
        if cfg!(windows) {
            let gs = gs.expect("Windows has the Microsoft GS Wavetable Synth");
            assert_eq!(midiport::call("sys", "openoutput", &[v_str("gs wavetable")]), Some(v_int(1)), "{:?}", midiport::get("sys", "error"));
            let sent = midiport::call("sys", "sendhex", &[v_str("F0 41 10 42 12 40 00 7F 00 41 F7")]);
            println!("GS Reset to {gs}: {sent:?} [{}]", midiport::get("sys", "error").map(|v| v.to_string_val()).unwrap_or_default());
            assert_eq!(sent, Some(v_int(1)));
            // (an input that isn't there: a plain error)
            assert_eq!(midiport::call("sys", "openinput", &[v_str("no such keyboard")]), Some(v_int(0)));
            println!("no such input: {}", midiport::get("sys", "error").unwrap().to_string_val());
        }
        midiport::remove("sys");
    }
}

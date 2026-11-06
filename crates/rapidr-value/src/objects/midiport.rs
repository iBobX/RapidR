//! RMIDIPort: MIDI input and output for a program, with System Exclusive
//! (SysEx) messages (docs/rmidiport-design.md). The same model on every
//! runtime; a runtime only gives it the system's devices ([`set_system`]:
//! midir on the desktop, the browser's MIDI interface in the page, the
//! tests' [`TestMidi`]) and, between the program's turns, delivers what
//! arrived ([`pump`]).
//!
//! - **The parser** ([`Parser`]) turns the bytes a device gives — whole
//!   messages, pieces of a SysEx, a raw stream with running status — into
//!   whole messages: running status expanded; real-time bytes (`F8`–`FF`)
//!   taken out wherever they are, even inside a SysEx; a SysEx joined across
//!   any number of pieces until its `F7`; a SysEx cut by another status
//!   byte dropped (said); stray data bytes and the undefined `F4`, `F5`,
//!   `F9`, `FD` ignored. IgnoreClock (`F8`, `F1`), IgnoreActiveSensing
//!   (`FE`) and IgnoreSysEx are applied here, the same everywhere.
//! - **The inbox** ([`Inbox`]) is what a device's thread hands over: it
//!   parses and queues under a lock and wakes the program's loop. Bounded:
//!   [`MAX_QUEUED`] messages and [`MAX_SYSEX_QUEUED`] bytes of SysEx
//!   waiting; past that a message is counted as dropped (the driver never
//!   waits) and OnError says how many once the program takes the next.
//! - **Delivery** ([`pump`]): one message in flight at a time — the next
//!   goes once the last one's handlers ran (`events::fire_then`), so
//!   Timestamp is the handled message's and an interpreted program's events
//!   never pile up; a native build runs handlers at once, so it takes
//!   [`TURN_MESSAGES`] a turn (or [`TURN_MS`]), then lets the window paint.
//! - **Validation**: what Send, SendSysEx and SendHex take is checked here,
//!   so a mistake gets the same plain message on every runtime.

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use crate::{v_int, v_str, Value};

/// The type name.
pub const TYPE: &str = "RMIDIPORT";

/// Messages that may wait for the program (about 10 s of a full MIDI cable).
pub const MAX_QUEUED: usize = 32_768;
/// Bytes of SysEx that may wait for the program.
pub const MAX_SYSEX_QUEUED: usize = 16 << 20;
/// The longest SysEx message taken (longer ones are dropped, said).
pub const MAX_SYSEX: usize = 1 << 20;
/// Messages a native build's turn delivers before the window paints.
pub const TURN_MESSAGES: usize = 512;
/// … or this long, in ms.
pub const TURN_MS: f64 = 15.0;
/// How often the devices are looked at for OnDevicesChanged, in ms (the
/// desktop's systems don't say when a device comes or goes).
pub const LOOK_EVERY_MS: f64 = 1000.0;

// ------------------------------------------------------------ the parser --

/// What the parser drops by itself (the program's Ignore… properties).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ignore {
    pub sysex: bool,
    /// Timing clock (`F8`) and MIDI time code quarter frames (`F1`).
    pub clock: bool,
    /// Active sensing (`FE`).
    pub active_sensing: bool,
}

impl Default for Ignore {
    fn default() -> Self {
        Ignore { sysex: true, clock: true, active_sensing: true }
    }
}

/// A whole message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Message {
    /// A short message: its status and up to two data bytes (`len` of the
    /// three are used).
    Short { bytes: [u8; 3], len: u8 },
    /// A whole SysEx message, `F0` to `F7` included.
    SysEx(Vec<u8>),
    /// Something the program is told (OnError): a SysEx cut or too long, a
    /// device gone, messages dropped.
    Problem(String),
}

impl Message {
    fn short(bytes: &[u8]) -> Message {
        let mut b = [0u8; 3];
        b[..bytes.len()].copy_from_slice(bytes);
        Message::Short { bytes: b, len: bytes.len() as u8 }
    }
}

/// How many data bytes a status takes (`None`: not a status that takes data
/// by itself — SysEx, real-time, the undefined ones).
pub fn data_bytes(status: u8) -> Option<usize> {
    match status {
        0x80..=0xBF | 0xE0..=0xEF => Some(2),
        0xC0..=0xDF => Some(1),
        0xF1 | 0xF3 => Some(1),
        0xF2 => Some(2),
        0xF6 => Some(0),
        _ => None,
    }
}

/// What a SysEx cut by another message is called.
pub const SYSEX_CUT: &str = "A SysEx message was cut by another message and dropped";
/// … one longer than [`MAX_SYSEX`].
pub const SYSEX_TOO_LONG: &str = "A SysEx message longer than 1 MB was dropped";

/// A MIDI byte stream in, whole messages out (see the module's docs).
#[derive(Debug, Default)]
pub struct Parser {
    /// The status the next data bytes belong to: a channel message's (it
    /// stays: running status) or a system common message's (it ends with it).
    status: Option<u8>,
    data: [u8; 2],
    have: usize,
    /// A SysEx being received (`None` inside: one being ignored or too long).
    sysex: Option<Option<Vec<u8>>>,
    too_long: bool,
}

impl Parser {
    pub fn new() -> Self {
        Parser::default()
    }

    /// Whether a SysEx is being received (its `F7` not seen yet).
    pub fn in_sysex(&self) -> bool {
        self.sysex.is_some()
    }

    fn end_sysex_cut(&mut self, out: &mut impl FnMut(Message)) {
        if let Some(s) = self.sysex.take() {
            if s.is_some() || self.too_long {
                out(Message::Problem(SYSEX_CUT.into()));
            }
            self.too_long = false;
        }
    }

    /// Takes `bytes` (any piece of the stream); whole messages go to `out`.
    pub fn feed(&mut self, bytes: &[u8], ignore: Ignore, out: &mut impl FnMut(Message)) {
        for &b in bytes {
            match b {
                // (real-time: anywhere, at once, interrupting nothing)
                0xF8..=0xFF => {
                    let dropped = match b {
                        0xF8 => ignore.clock,
                        0xFE => ignore.active_sensing,
                        0xF9 | 0xFD => true,
                        _ => false,
                    };
                    if !dropped {
                        out(Message::short(&[b]));
                    }
                }
                0xF0 => {
                    self.end_sysex_cut(out);
                    self.status = None;
                    self.have = 0;
                    self.too_long = false;
                    self.sysex = Some((!ignore.sysex).then(|| vec![0xF0]));
                }
                0xF7 => {
                    if let Some(s) = self.sysex.take() {
                        if self.too_long {
                            out(Message::Problem(SYSEX_TOO_LONG.into()));
                        } else if let Some(mut s) = s {
                            s.push(0xF7);
                            out(Message::SysEx(s));
                        }
                        self.too_long = false;
                    }
                }
                0x80..=0xF6 => {
                    self.end_sysex_cut(out);
                    self.have = 0;
                    match data_bytes(b) {
                        Some(0) => {
                            self.status = None;
                            out(Message::short(&[b]));
                        }
                        Some(_) => self.status = Some(b),
                        // (F4, F5: undefined — and they end running status)
                        None => self.status = None,
                    }
                }
                _ => {
                    // (a data byte)
                    if let Some(s) = self.sysex.as_mut() {
                        if let Some(v) = s.as_mut() {
                            if v.len() + 1 >= MAX_SYSEX {
                                *s = None;
                                self.too_long = true;
                            } else {
                                v.push(b);
                            }
                        }
                        continue;
                    }
                    let Some(status) = self.status else { continue };
                    let need = data_bytes(status).unwrap_or(0);
                    self.data[self.have] = b;
                    self.have += 1;
                    if self.have < need {
                        continue;
                    }
                    self.have = 0;
                    let dropped = status == 0xF1 && ignore.clock;
                    if !dropped {
                        let mut m = vec![status];
                        m.extend_from_slice(&self.data[..need]);
                        out(Message::short(&m));
                    }
                    // (system common messages have no running status)
                    if status >= 0xF0 {
                        self.status = None;
                    }
                }
            }
        }
    }
}

// ------------------------------------------------------------- the inbox --

/// A message that arrived and when (the runtime's clock, ms: [`set_clock`]).
#[derive(Clone, Debug, PartialEq)]
pub struct Arrived {
    pub at_ms: f64,
    pub message: Message,
}

#[derive(Default)]
struct InboxState {
    parser: Parser,
    ignore: Ignore,
    queue: VecDeque<Arrived>,
    sysex_bytes: usize,
    /// Messages dropped since the program was last told.
    dropped: u64,
    dropped_total: u64,
}

/// What a device's thread hands to the program (see the module's docs).
#[derive(Default)]
pub struct Inbox {
    state: Mutex<InboxState>,
}

impl Inbox {
    pub fn new(ignore: Ignore) -> Arc<Inbox> {
        Arc::new(Inbox { state: Mutex::new(InboxState { ignore, ..InboxState::default() }) })
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, InboxState> {
        self.state.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Bytes from the device (any piece of the stream), arrived at `at_ms`
    /// (the runtime's clock, which the device's thread reads itself): parsed,
    /// queued, the program's loop woken. Called on the device's thread;
    /// never waits for the program.
    pub fn push(&self, bytes: &[u8], at_ms: f64) {
        {
            let mut st = self.lock();
            let st = &mut *st;
            let ignore = st.ignore;
            let (queue, sysex_bytes, dropped, total) = (&mut st.queue, &mut st.sysex_bytes, &mut st.dropped, &mut st.dropped_total);
            st.parser.feed(bytes, ignore, &mut |message| {
                let size = if let Message::SysEx(s) = &message { s.len() } else { 0 };
                if queue.len() >= MAX_QUEUED || *sysex_bytes + size > MAX_SYSEX_QUEUED {
                    *dropped += 1;
                    *total += 1;
                    return;
                }
                *sysex_bytes += size;
                queue.push_back(Arrived { at_ms, message });
            });
        }
        wake();
    }

    /// A problem for the program, in order with the messages (a device gone).
    pub fn problem(&self, text: &str, at_ms: f64) {
        self.lock().queue.push_back(Arrived { at_ms, message: Message::Problem(text.to_string()) });
        wake();
    }

    /// The next message for the program; first, how many were dropped (when
    /// some were) as a problem.
    pub fn take(&self) -> Option<Arrived> {
        let mut st = self.lock();
        if st.dropped > 0 {
            let n = std::mem::take(&mut st.dropped);
            let at_ms = st.queue.front().map_or(0.0, |a| a.at_ms);
            return Some(Arrived { at_ms, message: Message::Problem(dropped_text(n)) });
        }
        let a = st.queue.pop_front()?;
        if let Message::SysEx(s) = &a.message {
            st.sysex_bytes = st.sysex_bytes.saturating_sub(s.len());
        }
        Some(a)
    }

    /// Messages waiting.
    pub fn queued(&self) -> usize {
        self.lock().queue.len()
    }

    /// Messages dropped since the input opened.
    pub fn dropped_total(&self) -> u64 {
        self.lock().dropped_total
    }

    /// Whether something waits for the program.
    pub fn has_more(&self) -> bool {
        let st = self.lock();
        !st.queue.is_empty() || st.dropped > 0
    }

    pub fn set_ignore(&self, ignore: Ignore) {
        self.lock().ignore = ignore;
    }

    /// What waits dropped (the input closed).
    pub fn clear(&self) {
        let mut st = self.lock();
        st.queue.clear();
        st.sysex_bytes = 0;
        st.dropped = 0;
        st.parser = Parser::new();
    }
}

fn dropped_text(n: u64) -> String {
    if n == 1 {
        "1 MIDI message was dropped: it came faster than the program handled them".into()
    } else {
        format!("{n} MIDI messages were dropped: they came faster than the program handled them")
    }
}

// ------------------------------------------------------- the system side --

/// An open output.
pub trait MidiOut {
    /// Sends whole messages (already checked).
    fn send(&mut self, bytes: &[u8]) -> Result<(), String>;
}

/// Why a device didn't open.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OpenError {
    /// Not there (any more).
    NotFound,
    /// Another program has it.
    InUse,
    /// The system's words.
    Other(String),
}

/// The system's MIDI devices: the runtime's ([`set_system`]).
pub trait MidiSystem {
    /// The inputs' names, in the system's order; `Err`: why there are none
    /// to list (no MIDI here, the browser refused …).
    fn inputs(&self) -> Result<Vec<String>, String>;
    fn outputs(&self) -> Result<Vec<String>, String>;
    /// Opens the `nth` (0: the first) input named `name`: what arrives goes
    /// into `inbox` (the system's thread may call [`Inbox::push`]). Dropping
    /// what it returns closes it.
    fn open_input(&self, name: &str, nth: usize, inbox: Arc<Inbox>) -> Result<Box<dyn Any>, OpenError>;
    fn open_output(&self, name: &str, nth: usize) -> Result<Box<dyn MidiOut>, OpenError>;
    /// Whether it can send SysEx now (a browser's permission); `Err`: why not.
    fn can_send_sysex(&self) -> Result<(), String> {
        Ok(())
    }
}

/// What runs once an event's handler ran, given the handler's arguments.
pub type Then = Box<dyn FnOnce(&[Value])>;

/// How a runtime fires an event (object, event, arguments) and runs the
/// continuation once its handler ran.
pub type FireThen = fn(&str, &str, &[Value], Then);

/// What the runtime's loop is: how events fire, how to wake it, its clock.
#[derive(Clone, Copy)]
pub struct Host {
    pub has_handler: fn(&str, &str) -> bool,
    /// Fires an event and runs the continuation once its handler ran (now in
    /// a native build, when it returns in the interpreter).
    pub fire_then: FireThen,
    /// The runtime's clock, ms (any start).
    pub now_ms: fn() -> f64,
}

fn no_wake() {}

static WAKE: std::sync::RwLock<fn()> = std::sync::RwLock::new(no_wake);

/// How a device's thread wakes the program's loop (any thread may call it).
pub fn set_waker(w: fn()) {
    if let Ok(mut x) = WAKE.write() {
        *x = w;
    }
}

fn wake() {
    let w = WAKE.read().map(|w| *w).unwrap_or(no_wake as fn());
    w();
}

thread_local! {
    static SYSTEM: RefCell<Option<Rc<dyn MidiSystem>>> = const { RefCell::new(None) };
    static CLOCK: Cell<fn() -> f64> = const { Cell::new(zero_clock) };
    static PORTS: RefCell<HashMap<String, MidiPort>> = RefCell::new(HashMap::new());
    /// Messages delivered in this turn of the loop (the interpreter's chain
    /// stops at [`TURN_MESSAGES`] and goes on in the next turn).
    static TURN: Cell<usize> = const { Cell::new(0) };
}

fn zero_clock() -> f64 {
    0.0
}

/// The runtime's devices (once; the first RMIDIPort made).
pub fn set_system(s: Rc<dyn MidiSystem>) {
    SYSTEM.with(|x| *x.borrow_mut() = Some(s));
}

pub fn system_installed() -> bool {
    SYSTEM.with(|s| s.borrow().is_some())
}

fn system() -> Option<Rc<dyn MidiSystem>> {
    SYSTEM.with(|s| s.borrow().clone())
}

/// The runtime's clock for Timestamp, in ms (any start).
pub fn set_clock(c: fn() -> f64) {
    CLOCK.with(|x| x.set(c));
}

fn now() -> f64 {
    CLOCK.with(|c| c.get())()
}

// ------------------------------------------------------ the program's side --

/// The devices as a program sees them: unique names (a second "X" is
/// "X (2)"), each with the system's name and which of those it is.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Devices(pub Vec<(String, String, usize)>);

impl Devices {
    pub fn of(raw: &[String]) -> Devices {
        let mut seen: HashMap<&str, usize> = HashMap::new();
        Devices(
            raw.iter()
                .map(|n| {
                    let k = seen.entry(n.as_str()).or_insert(0);
                    *k += 1;
                    let shown = if *k == 1 { n.clone() } else { format!("{n} ({k})") };
                    (shown, n.clone(), *k - 1)
                })
                .collect(),
        )
    }

    pub fn names(&self) -> Vec<String> {
        self.0.iter().map(|(n, _, _)| n.clone()).collect()
    }

    /// The device `which` means: a number in the list, or a name — the exact
    /// one (any case), else the first that has it as a part.
    fn find(&self, which: &Value) -> Option<usize> {
        match which {
            Value::Integer(_) | Value::Double(_) | Value::Boolean(_) => {
                let i = which.to_i64();
                (0..self.0.len() as i64).contains(&i).then_some(i as usize)
            }
            _ => {
                let want = which.to_string_val().trim().to_lowercase();
                if want.is_empty() {
                    return None;
                }
                self.0
                    .iter()
                    .position(|(n, _, _)| n.to_lowercase() == want)
                    .or_else(|| self.0.iter().position(|(n, _, _)| n.to_lowercase().contains(&want)))
            }
        }
    }
}

struct OpenInput {
    name: String,
    inbox: Arc<Inbox>,
    /// When it opened (the runtime's clock): Timestamp's zero.
    opened_ms: f64,
    /// The device went away: closed once what waits is delivered.
    closing: bool,
    _link: Box<dyn Any>,
}

/// An RMIDIPort.
pub struct MidiPort {
    ignore: Ignore,
    inputs: Option<Devices>,
    outputs: Option<Devices>,
    input: Option<OpenInput>,
    output: Option<(String, Box<dyn MidiOut>)>,
    error: String,
    timestamp: f64,
    /// A message's handlers are running (the next waits for them).
    in_flight: Rc<Cell<bool>>,
    /// The devices OnDevicesChanged last saw, and when it looked.
    seen: Option<(Vec<String>, Vec<String>)>,
    looked_at: f64,
}

impl Default for MidiPort {
    fn default() -> Self {
        MidiPort {
            ignore: Ignore::default(),
            inputs: None,
            outputs: None,
            input: None,
            output: None,
            error: String::new(),
            timestamp: 0.0,
            in_flight: Rc::new(Cell::new(false)),
            seen: None,
            looked_at: f64::NEG_INFINITY,
        }
    }
}

/// "input" or "output".
fn kind(input: bool) -> &'static str {
    if input {
        "input"
    } else {
        "output"
    }
}

/// The plain list of names for a message: "A, B" or "there are none".
fn listing(names: &[String]) -> String {
    if names.is_empty() {
        "there are none".into()
    } else {
        format!("there are: {}", names.join(", "))
    }
}

impl MidiPort {
    /// The system's list read again.
    fn list(&mut self, input: bool) -> Result<&Devices, String> {
        let raw = match system() {
            None => Err("No MIDI here: this program was built without MIDI".to_string()),
            Some(s) => {
                if input {
                    s.inputs()
                } else {
                    s.outputs()
                }
            }
        };
        let (raw, why) = match raw {
            Ok(r) => (r, None),
            Err(e) => (Vec::new(), Some(e)),
        };
        let d = Devices::of(&raw);
        let slot = if input { &mut self.inputs } else { &mut self.outputs };
        *slot = Some(d);
        match why {
            Some(e) => Err(e),
            None => Ok(slot.as_ref().unwrap_or_else(|| unreachable!())),
        }
    }

    /// The list as last read (read now if never).
    fn listed(&mut self, input: bool) -> Devices {
        let known = if input { self.inputs.clone() } else { self.outputs.clone() };
        match known {
            Some(d) => d,
            None => self.list(input).cloned().unwrap_or_default(),
        }
    }

    fn count(&mut self, input: bool) -> i64 {
        match self.list(input) {
            Ok(d) => {
                let n = d.0.len() as i64;
                self.error.clear();
                n
            }
            Err(e) => {
                self.error = e;
                0
            }
        }
    }

    fn open(&mut self, input: bool, which: &Value) -> bool {
        self.error.clear();
        let Some(sys) = system() else {
            self.error = "No MIDI here: this program was built without MIDI".into();
            return false;
        };
        // (by number: the list the program saw; by name: read again)
        let by_number = matches!(which, Value::Integer(_) | Value::Double(_) | Value::Boolean(_));
        let devices = if by_number {
            self.listed(input)
        } else {
            match self.list(input) {
                Ok(d) => d.clone(),
                Err(e) => {
                    self.error = e;
                    return false;
                }
            }
        };
        let Some(i) = devices.find(which) else {
            let names = devices.names();
            self.error = if by_number {
                let n = which.to_i64();
                if names.is_empty() {
                    format!("There is no MIDI {} number {n}: there are none", kind(input))
                } else {
                    format!("There is no MIDI {} number {n} (there are {}: 0 to {})", kind(input), names.len(), names.len() - 1)
                }
            } else {
                format!("No MIDI {} is called \"{}\" ({})", kind(input), which.to_string_val().trim(), listing(&names))
            };
            return false;
        };
        let (shown, raw, nth) = devices.0[i].clone();
        if input {
            self.close_input();
            let inbox = Inbox::new(self.ignore);
            match sys.open_input(&raw, nth, inbox.clone()) {
                Ok(link) => {
                    self.input = Some(OpenInput { name: shown, inbox, opened_ms: now(), closing: false, _link: link });
                    true
                }
                Err(e) => {
                    self.error = open_error(&e, input, &shown);
                    false
                }
            }
        } else {
            self.output = None;
            match sys.open_output(&raw, nth) {
                Ok(out) => {
                    self.output = Some((shown, out));
                    true
                }
                Err(e) => {
                    self.error = open_error(&e, input, &shown);
                    false
                }
            }
        }
    }

    fn close_input(&mut self) {
        if let Some(i) = self.input.take() {
            i.inbox.clear();
        }
        self.in_flight.set(false);
    }

    /// Sends checked bytes.
    fn send_bytes(&mut self, bytes: &[u8]) -> bool {
        let Some((_, out)) = self.output.as_mut() else {
            self.error = "No MIDI output is open: call OpenOutput first".into();
            return false;
        };
        if bytes.contains(&0xF0) {
            if let Some(Err(e)) = system().map(|s| s.can_send_sysex()) {
                self.error = e;
                return false;
            }
        }
        match out.send(bytes) {
            Ok(()) => {
                self.error.clear();
                true
            }
            Err(e) => {
                self.error = e;
                false
            }
        }
    }

    fn get(&self, prop: &str) -> Option<Value> {
        Some(match prop {
            "isinputopen" => v_int(i64::from(self.input.is_some())),
            "isoutputopen" => v_int(i64::from(self.output.is_some())),
            "inputname" => v_str(self.input.as_ref().map_or("", |i| i.name.as_str())),
            "outputname" => v_str(self.output.as_ref().map_or("", |o| o.0.as_str())),
            "ignoresysex" => v_int(i64::from(self.ignore.sysex)),
            "ignoreclock" => v_int(i64::from(self.ignore.clock)),
            "ignoreactivesensing" => v_int(i64::from(self.ignore.active_sensing)),
            "error" => v_str(&self.error),
            "timestamp" => Value::Double(self.timestamp),
            "queued" => v_int(self.input.as_ref().map_or(0, |i| i.inbox.queued() as i64)),
            "dropped" => v_int(self.input.as_ref().map_or(0, |i| i.inbox.dropped_total() as i64)),
            _ => return None,
        })
    }

    fn set(&mut self, prop: &str, val: &Value) -> Option<()> {
        let on = crate::panels::runtime::truth(val);
        match prop {
            "ignoresysex" => self.ignore.sysex = on,
            "ignoreclock" => self.ignore.clock = on,
            "ignoreactivesensing" => self.ignore.active_sensing = on,
            // (read-only)
            "isinputopen" | "isoutputopen" | "inputname" | "outputname" | "error" | "timestamp" | "queued" | "dropped" => return Some(()),
            _ => return None,
        }
        if let Some(i) = &self.input {
            i.inbox.set_ignore(self.ignore);
        }
        Some(())
    }
}

fn open_error(e: &OpenError, input: bool, name: &str) -> String {
    match e {
        OpenError::NotFound => format!("MIDI {} \"{name}\" isn't there any more (it was unplugged?)", kind(input)),
        OpenError::InUse => format!("MIDI {} \"{name}\" is in use by another program", kind(input)),
        OpenError::Other(s) => format!("MIDI {} \"{name}\" didn't open: {s}", kind(input)),
    }
}

// ----------------------------------------------------------- validation --

/// A data byte's check: `what` is 0 to 127.
fn data_byte(what: &str, v: &Value) -> Result<u8, String> {
    let n = v.to_i64();
    if (0..=127).contains(&n) {
        Ok(n as u8)
    } else {
        Err(format!("{what} is 0 to 127 (it was {n})"))
    }
}

/// A channel's check: 1 to 16, as the program says it; the status's low
/// four bits.
fn channel(v: &Value) -> Result<u8, String> {
    let n = v.to_i64();
    if (1..=16).contains(&n) {
        Ok((n - 1) as u8)
    } else {
        Err(format!("Channel is 1 to 16 (it was {n})"))
    }
}

/// Send's bytes: a status and the data it takes.
pub fn short_message(args: &[Value]) -> Result<Vec<u8>, String> {
    let status = args.first().map_or(0, Value::to_i64);
    if !(0x80..=0xFF).contains(&status) {
        return Err(format!("Status is &H80 to &HFF, 128 to 255 (it was {status}): a status byte has its top bit set"));
    }
    let status = status as u8;
    if status == 0xF0 || status == 0xF7 {
        return Err("A SysEx message goes with SendSysEx (or SendHex)".into());
    }
    if matches!(status, 0xF4 | 0xF5 | 0xF9 | 0xFD) {
        return Err(format!("&H{status:02X} isn't a MIDI message (it is undefined)"));
    }
    let need = data_bytes(status).unwrap_or(0);
    let mut out = vec![status];
    for (i, what) in ["Data1", "Data2"].iter().enumerate().take(need) {
        out.push(data_byte(what, args.get(i + 1).unwrap_or(&Value::Integer(0)))?);
    }
    Ok(out)
}

/// SendSysEx's check: `F0`, bytes 0 to 127, `F7`.
pub fn sysex_message(bytes: &[u8]) -> Result<(), String> {
    if bytes.len() < 2 || bytes[0] != 0xF0 || bytes[bytes.len() - 1] != 0xF7 {
        return Err("A SysEx message starts with F0 (240) and ends with F7 (247)".into());
    }
    if let Some((i, b)) = bytes[1..bytes.len() - 1].iter().enumerate().find(|(_, b)| **b > 0x7F) {
        return Err(format!("Byte {} of the SysEx is {b}: bytes between F0 and F7 are 0 to 127", i + 2));
    }
    if bytes.len() > MAX_SYSEX {
        return Err("A SysEx message is 1 MB at most".into());
    }
    Ok(())
}

/// The bytes hex text means: pairs of hex digits, with spaces, commas,
/// `&H` / `0x` / `$` allowed between them.
pub fn parse_hex(text: &str) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    for word in text.split(|c: char| c.is_whitespace() || c == ',' || c == ';') {
        let mut w = word.trim();
        for p in ["&h", "&H", "0x", "0X", "$"] {
            if let Some(r) = w.strip_prefix(p) {
                w = r;
                break;
            }
        }
        if w.is_empty() {
            continue;
        }
        // (several bytes run together: "F07E7F0601F7")
        let digits: Vec<char> = w.chars().collect();
        let chunks: Vec<String> = if digits.len() <= 2 { vec![w.to_string()] } else { digits.chunks(2).map(|c| c.iter().collect()).collect() };
        for c in chunks {
            match u8::from_str_radix(&c, 16) {
                Ok(b) => out.push(b),
                Err(_) => return Err(format!("\"{c}\" isn't a byte in hex (00 to FF)")),
            }
        }
    }
    if out.is_empty() {
        return Err("There are no bytes in it: write them in hex, as \"F0 7E 7F 06 01 F7\"".into());
    }
    Ok(out)
}

/// SendHex's check: whole messages only, each as Send / SendSysEx check it.
pub fn check_stream(bytes: &[u8]) -> Result<(), String> {
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == 0xF0 {
            let Some(end) = bytes[i..].iter().position(|&x| x == 0xF7) else {
                return Err("A SysEx message starts with F0 (240) and ends with F7 (247)".into());
            };
            sysex_message(&bytes[i..=i + end])?;
            i += end + 1;
            continue;
        }
        let args: Vec<Value> = std::iter::once(b).chain(bytes[i + 1..].iter().take(2).copied()).map(|x| Value::Integer(i64::from(x))).collect();
        let m = short_message(&args)?;
        if bytes.len() < i + m.len() || bytes[i + 1..i + m.len()].iter().any(|&x| x > 0x7F) {
            return Err(format!("The message starting with {b:02X} is missing its data bytes"));
        }
        i += m.len();
    }
    Ok(())
}

/// Bytes as hex text, "F0 7E 7F 06 01 F7".
pub fn hex_of(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 3);
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 {
            s.push(' ');
        }
        s.push_str(&format!("{b:02X}"));
    }
    s
}

fn bytes_of(v: &Value) -> Vec<u8> {
    super::codec::string_to_bytes(&v.to_string_val())
}

// ------------------------------------------------- the store, by name --

pub fn is_type(type_name: &str) -> bool {
    type_name.eq_ignore_ascii_case(TYPE)
}

/// Makes port `id` (the runtime's create).
pub fn create(id: &str, type_name: &str) -> bool {
    if !is_type(type_name) {
        return false;
    }
    PORTS.with(|p| {
        p.borrow_mut().entry(id.to_lowercase()).or_default();
    });
    true
}

pub fn exists(id: &str) -> bool {
    PORTS.with(|p| p.borrow().contains_key(crate::lower(id).as_ref()))
}

fn with<R>(id: &str, f: impl FnOnce(&mut MidiPort) -> R) -> Option<R> {
    PORTS.with(|p| p.borrow_mut().get_mut(crate::lower(id).as_ref()).map(f))
}

/// A property (lower case); `None` when not one of its.
pub fn get(id: &str, prop: &str) -> Option<Value> {
    with(id, |p| p.get(prop)).flatten()
}

/// Sets a property (lower case); `None` when not one of its.
pub fn set(id: &str, prop: &str, val: &Value) -> Option<()> {
    with(id, |p| p.set(prop, val)).flatten()
}

/// The names FillInputs / FillOutputs put in a list (read again), and the
/// open one's place there.
pub fn items(id: &str, input: bool) -> Option<(Vec<String>, Option<usize>)> {
    with(id, |p| {
        let names = match p.list(input).map(Devices::names) {
            Ok(n) => {
                p.error.clear();
                n
            }
            Err(e) => {
                p.error = e;
                Vec::new()
            }
        };
        let open = if input { p.input.as_ref().map(|i| i.name.clone()) } else { p.output.as_ref().map(|o| o.0.clone()) };
        let at = open.and_then(|o| names.iter().position(|n| *n == o));
        (names, at)
    })
}

/// Sets Error (the runtime's word: a browser's refusal).
pub fn set_error(id: &str, text: &str) {
    with(id, |p| p.error = text.to_string());
}

/// Whether port `id` asks for SysEx (IgnoreSysEx is False): a browser then
/// asks for that permission.
pub fn wants_sysex(id: &str) -> bool {
    with(id, |p| !p.ignore.sysex).unwrap_or(false)
}

/// A method (lower case); `None` when not one of its.
pub fn call(id: &str, method: &str, args: &[Value]) -> Option<Value> {
    if !exists(id) {
        return None;
    }
    let arg = |i: usize| args.get(i).cloned().unwrap_or(Value::Null);
    let truth = |b: bool| v_int(i64::from(b));
    Some(match method {
        "inputcount" => v_int(with(id, |p| p.count(true)).unwrap_or(0)),
        "outputcount" => v_int(with(id, |p| p.count(false)).unwrap_or(0)),
        "inputdevice" | "outputdevice" => {
            let input = method == "inputdevice";
            let i = arg(0).to_i64();
            v_str(&with(id, |p| p.listed(input).0.get(usize::try_from(i).unwrap_or(usize::MAX)).map(|d| d.0.clone()).unwrap_or_default()).unwrap_or_default())
        }
        "openinput" => truth(with(id, |p| p.open(true, &arg(0))).unwrap_or(false)),
        "openoutput" => truth(with(id, |p| p.open(false, &arg(0))).unwrap_or(false)),
        "closeinput" => {
            with(id, MidiPort::close_input);
            Value::Null
        }
        "closeoutput" => {
            with(id, |p| p.output = None);
            Value::Null
        }
        "close" => {
            with(id, |p| {
                p.close_input();
                p.output = None;
            });
            Value::Null
        }
        "send" => truth(send_checked(id, short_message(args))),
        "sendnoteon" | "sendnoteoff" => {
            let on = method == "sendnoteon";
            let m = channel(&arg(0)).and_then(|ch| {
                let note = data_byte("Note", &arg(1))?;
                let vel = data_byte("Velocity", if matches!(arg(2), Value::Null) { &Value::Integer(0) } else { &args[2] })?;
                Ok(vec![if on { 0x90 } else { 0x80 } | ch, note, vel])
            });
            truth(send_checked(id, m))
        }
        "sendcontrol" => {
            let m = channel(&arg(0)).and_then(|ch| Ok(vec![0xB0 | ch, data_byte("Controller", &arg(1))?, data_byte("Value", &arg(2))?]));
            truth(send_checked(id, m))
        }
        "sendprogram" => {
            let m = channel(&arg(0)).and_then(|ch| Ok(vec![0xC0 | ch, data_byte("Program", &arg(1))?]));
            truth(send_checked(id, m))
        }
        "sendsysex" => {
            let bytes = bytes_of(&arg(0));
            let m = sysex_message(&bytes).map(|()| bytes);
            truth(send_checked(id, m))
        }
        "sendhex" => {
            let m = parse_hex(&arg(0).to_string_val()).and_then(|b| check_stream(&b).map(|()| b));
            truth(send_checked(id, m))
        }
        "hex" => v_str(&hex_of(&bytes_of(&arg(0)))),
        _ => return None,
    })
}

fn send_checked(id: &str, m: Result<Vec<u8>, String>) -> bool {
    with(id, |p| match m {
        Ok(bytes) => p.send_bytes(&bytes),
        Err(e) => {
            p.error = e;
            false
        }
    })
    .unwrap_or(false)
}

/// Forgets port `id`, closing what it has open.
pub fn remove(id: &str) -> bool {
    PORTS.with(|p| p.borrow_mut().remove(&id.to_lowercase())).is_some()
}

/// Whether a port has an input or an output open (the runtime then looks
/// at the devices each second for OnDevicesChanged and unplugged ones).
pub fn active() -> bool {
    PORTS.with(|p| p.borrow().values().any(|x| x.input.is_some() || x.output.is_some()))
}

/// The ids of every port.
fn ids() -> Vec<String> {
    PORTS.with(|p| p.borrow().keys().cloned().collect())
}

// ------------------------------------------------------------ delivery --

/// The events one message fires, in order (only those the program handles).
fn events_of(id: &str, m: &Message, has: fn(&str, &str) -> bool) -> Vec<(&'static str, Vec<Value>)> {
    let mut out = Vec::new();
    let int = |b: u8| Value::Integer(i64::from(b));
    match m {
        Message::Short { bytes, .. } => {
            let [s, d1, d2] = *bytes;
            if has(id, "onmessage") {
                out.push(("onmessage", vec![int(s), int(d1), int(d2)]));
            }
            let ch = Value::Integer(i64::from(s & 0x0F) + 1);
            match s & 0xF0 {
                0x90 if d2 > 0 => {
                    if has(id, "onnoteon") {
                        out.push(("onnoteon", vec![ch, int(d1), int(d2)]));
                    }
                }
                0x80 | 0x90 => {
                    if has(id, "onnoteoff") {
                        out.push(("onnoteoff", vec![ch, int(d1), int(d2)]));
                    }
                }
                0xB0 if has(id, "oncontrol") => out.push(("oncontrol", vec![ch, int(d1), int(d2)])),
                _ => {}
            }
        }
        Message::SysEx(bytes) => {
            if has(id, "onsysex") {
                out.push(("onsysex", vec![v_str(&super::codec::bytes_to_string(bytes))]));
            }
        }
        Message::Problem(text) => {
            if has(id, "onerror") {
                out.push(("onerror", vec![v_str(text)]));
            }
        }
    }
    out
}

/// Fires `events` one after the other, each once the one before it ran;
/// then `done`.
fn fire_chain(host: Host, id: String, mut events: VecDeque<(&'static str, Vec<Value>)>, done: Box<dyn FnOnce()>) {
    let Some((event, args)) = events.pop_front() else {
        done();
        return;
    };
    let next_id = id.clone();
    (host.fire_then)(&id, event, &args, Box::new(move |_| fire_chain(host, next_id, events, done)));
}

/// What arrived, delivered: the runtime's loop calls it each turn (and
/// DOEVENTS does), after a wake. Each port's messages go one at a time (see
/// the module's docs).
pub fn pump(host: Host) {
    TURN.with(|t| t.set(0));
    for id in ids() {
        pump_one(host, &id);
    }
}

/// Whether something waits to be delivered (the loop turns again for it).
pub fn has_more() -> bool {
    PORTS.with(|p| p.borrow().values().any(|x| x.input.as_ref().is_some_and(|i| i.inbox.has_more())))
}

fn pump_one(host: Host, id: &str) {
    let started = (host.now_ms)();
    loop {
        if TURN.with(Cell::get) >= TURN_MESSAGES || (host.now_ms)() - started > TURN_MS {
            // (the rest in the next turn: the window paints meanwhile)
            wake();
            return;
        }
        let next = with(id, |p| {
            if p.in_flight.get() {
                return None;
            }
            let input = p.input.as_ref()?;
            let Some(a) = input.inbox.take() else {
                if input.closing {
                    p.close_input();
                }
                return None;
            };
            p.timestamp = (a.at_ms - input.opened_ms).max(0.0);
            if let Message::Problem(t) = &a.message {
                p.error = t.clone();
            }
            Some((a, p.in_flight.clone()))
        })
        .flatten();
        let Some((arrived, in_flight)) = next else { return };
        TURN.with(|t| t.set(t.get() + 1));
        let events: VecDeque<_> = events_of(id, &arrived.message, host.has_handler).into();
        if events.is_empty() {
            continue;
        }
        in_flight.set(true);
        // (the handlers ran at once — a native build: the loop goes on;
        // queued — the interpreter: the last one's continuation goes on)
        let firing = Rc::new(Cell::new(true));
        let (f2, flight2, id2) = (firing.clone(), in_flight.clone(), id.to_string());
        fire_chain(
            host,
            id.to_string(),
            events,
            Box::new(move || {
                flight2.set(false);
                if !f2.get() {
                    pump_one(host, &id2);
                }
            }),
        );
        firing.set(false);
        if in_flight.get() {
            return;
        }
    }
}

/// The runtime's look at the devices (about each second while a port is
/// open or the program handles OnDevicesChanged): an open device gone is
/// closed and said (OnError), a change fires OnDevicesChanged.
pub fn look(host: Host) {
    let t = (host.now_ms)();
    for id in ids() {
        let handles = (host.has_handler)(&id, "ondeviceschanged");
        let due = with(&id, |p| {
            let open = p.input.is_some() || p.output.is_some();
            if !(open || handles) || t - p.looked_at < LOOK_EVERY_MS {
                return false;
            }
            p.looked_at = t;
            true
        })
        .unwrap_or(false);
        if !due {
            continue;
        }
        let Some(sys) = system() else { continue };
        let ins = sys.inputs().unwrap_or_default();
        let outs = sys.outputs().unwrap_or_default();
        let changed = with(&id, |p| {
            let lists = (ins.clone(), outs.clone());
            let changed = p.seen.as_ref().is_some_and(|s| *s != lists);
            p.seen = Some(lists);
            if changed {
                p.inputs = Some(Devices::of(&ins));
                p.outputs = Some(Devices::of(&outs));
            }
            // (an open device gone: closed once its messages and the
            // problem are delivered, in order)
            let in_gone = p.input.as_ref().is_some_and(|i| !i.closing && !Devices::of(&ins).names().contains(&i.name));
            if in_gone {
                if let Some(i) = p.input.as_mut() {
                    i.closing = true;
                    let text = format!("MIDI input \"{}\" was unplugged: it is closed", i.name);
                    i.inbox.problem(&text, now());
                }
            }
            let out_gone = p.output.as_ref().is_some_and(|o| !Devices::of(&outs).names().contains(&o.0));
            if out_gone {
                let name = p.output.take().map(|o| o.0).unwrap_or_default();
                p.error = format!("MIDI output \"{name}\" was unplugged: it is closed");
            }
            (changed, out_gone.then(|| p.error.clone()))
        });
        let Some((changed, out_gone)) = changed else { continue };
        if let Some(text) = out_gone {
            if (host.has_handler)(&id, "onerror") {
                (host.fire_then)(&id, "onerror", &[v_str(&text)], Box::new(|_| {}));
            }
        }
        if changed && handles {
            (host.fire_then)(&id, "ondeviceschanged", &[], Box::new(|_| {}));
        }
    }
}

/// The devices changed (a browser said so): the next [`look`] looks now.
pub fn devices_changed() {
    PORTS.with(|p| {
        for x in p.borrow_mut().values_mut() {
            x.looked_at = f64::NEG_INFINITY;
        }
    });
}

/// The runtime's clock (what [`Inbox::push`]'s `at_ms` is on).
pub fn clock_now() -> f64 {
    now()
}

// ------------------------------------------------ a browser's permission --

/// What a browser shows before it asks the user (docs/legal/
/// review-2026-11-05-midi.md §5, C3): the title, the text, the buttons —
/// for MIDI, or (`sysex`) for SysEx — in the program's language (RapidR's
/// own catalogs: `crates/rapidr-value/lang`). The browser's own prompt is
/// the consent; this only says why it comes.
pub fn permission_text(sysex: bool) -> (String, String, [String; 2]) {
    use crate::i18n::tr_ui;
    let buttons = [tr_ui("Continue"), tr_ui("Not now")];
    if sysex {
        (
            tr_ui("SysEx messages"),
            tr_ui("This program also wants to send and receive \"system exclusive\" (SysEx) messages. These can change settings stored inside a MIDI device and, on some devices, even replace its firmware. Allow it only for programs you trust."),
            buttons,
        )
    } else {
        (
            tr_ui("MIDI devices"),
            tr_ui("This program wants to use your MIDI devices (keyboards, controllers, synthesizers). Your browser will ask you to allow it. RapidR does not send the list of your devices anywhere."),
            buttons,
        )
    }
}

/// What Error says when the browser didn't allow MIDI (`sysex`: SysEx), or
/// the person chose "Not now" (`later`), or there is no MIDI in it.
pub const REFUSED_MIDI: &str = "MIDI access was not allowed in this browser";
pub const REFUSED_SYSEX: &str = "SysEx messages were not allowed in this browser (notes and controls still work)";
pub const NOT_NOW: &str = "MIDI wasn't turned on (\"Not now\"): the program can ask again";
pub const NOT_NOW_SYSEX: &str = "SysEx wasn't turned on (\"Not now\"): the program can ask again";
pub const NO_BROWSER_MIDI: &str = "This browser has no MIDI support, or the page isn't served over HTTPS or from this computer";
pub const ASKING: &str = "The browser is still asking the user to allow MIDI";

// ------------------------------------------------------ the tests' system --

/// The tests' devices (`RAPIDR_TEST_MIDIPORT` on the desktop, the page's
/// variable in the browser): `;`-separated items — `in:NAME` (an input),
/// `out:NAME` (an output that takes anything), `loop:NAME` (an output and
/// an input of that name: what is sent comes back, a SysEx in 1 KB pieces
/// as Windows' driver gives them), `synth:NAME` (an output and an input of
/// that name, a synthesizer on a cable: an Identity Request,
/// `F0 7E id 06 01 F7`, is answered with [`TEST_IDENTITY_REPLY`]; nothing
/// else comes back), `in:NAME=HEX` (an input that plays those bytes when it
/// opens), `burst:NAME=N` (an input that plays N note messages when it
/// opens, all at once), `busy:NAME` (an input and an output another
/// program has). Never a real device.
pub struct TestMidi {
    items: RefCell<Vec<(String, String, String)>>,
    /// The loops' inboxes, by name (what the output sends goes there).
    loops: Rc<RefCell<HashMap<String, Vec<Arc<Inbox>>>>>,
    /// The script the devices were made with (used while the environment's
    /// is empty).
    first: String,
}

/// What the tests' `synth:` answers an Identity Request with: maker 7D (the
/// ID for tests and education — never a format of RapidR's), family 01 00,
/// model 02 00, version 01 00 00 00.
pub const TEST_IDENTITY_REPLY: [u8; 15] = [0xF0, 0x7E, 0x7F, 0x06, 0x02, 0x7D, 0x01, 0x00, 0x02, 0x00, 0x01, 0x00, 0x00, 0x00, 0xF7];

fn parse_test(script: &str) -> Vec<(String, String, String)> {
    script
        .split(';')
        .filter_map(|item| {
            let (kind, rest) = item.split_once(':')?;
            let (name, data) = rest.split_once('=').unwrap_or((rest, ""));
            let name = name.trim();
            (!name.is_empty()).then(|| (kind.trim().to_ascii_lowercase(), name.to_string(), data.trim().to_string()))
        })
        .collect()
}

impl TestMidi {
    pub fn parse(script: &str) -> Self {
        TestMidi { items: RefCell::new(parse_test(script)), loops: Rc::default(), first: script.to_string() }
    }

    /// The script read again (the environment's — a program's ENVIRON plugs
    /// devices in or out —, else the first).
    fn refresh(&self) {
        let now = crate::environ::get("RAPIDR_TEST_MIDIPORT");
        *self.items.borrow_mut() = parse_test(if now.is_empty() { &self.first } else { &now });
    }

    fn names(&self, input: bool) -> Vec<String> {
        self.refresh();
        self.items
            .borrow()
            .iter()
            .filter(|(k, _, _)| match k.as_str() {
                "in" | "burst" => input,
                "out" => !input,
                "loop" | "busy" | "synth" => true,
                _ => false,
            })
            .map(|(_, n, _)| n.clone())
            .collect()
    }

    fn item(&self, name: &str, input: bool) -> Option<(String, String)> {
        self.refresh();
        let items = self.items.borrow();
        items
            .iter()
            .find(|(k, n, _)| {
                n == name
                    && match k.as_str() {
                        "in" | "burst" => input,
                        "out" => !input,
                        _ => true,
                    }
            })
            .map(|(k, _, d)| (k.clone(), d.clone()))
    }
}

struct TestIn {
    name: String,
    inbox: Arc<Inbox>,
    loops: Rc<RefCell<HashMap<String, Vec<Arc<Inbox>>>>>,
}

impl Drop for TestIn {
    fn drop(&mut self) {
        if let Some(v) = self.loops.borrow_mut().get_mut(&self.name) {
            v.retain(|i| !Arc::ptr_eq(i, &self.inbox));
        }
    }
}

struct TestOut {
    name: String,
    looped: bool,
    synth: bool,
    loops: Rc<RefCell<HashMap<String, Vec<Arc<Inbox>>>>>,
}

impl MidiOut for TestOut {
    fn send(&mut self, bytes: &[u8]) -> Result<(), String> {
        let inboxes = self.loops.borrow().get(&self.name).cloned().unwrap_or_default();
        let at = now();
        if self.synth {
            if bytes.len() == 6 && bytes[..2] == [0xF0, 0x7E] && bytes[3..] == [0x06, 0x01, 0xF7] {
                for inbox in &inboxes {
                    inbox.push(&TEST_IDENTITY_REPLY, at);
                }
            }
            return Ok(());
        }
        if !self.looped {
            return Ok(());
        }
        for inbox in inboxes {
            // (a SysEx in 1 KB pieces, as Windows' driver gives a long one)
            for piece in bytes.chunks(1024) {
                inbox.push(piece, at);
            }
        }
        Ok(())
    }
}

impl MidiSystem for TestMidi {
    fn inputs(&self) -> Result<Vec<String>, String> {
        Ok(self.names(true))
    }
    fn outputs(&self) -> Result<Vec<String>, String> {
        Ok(self.names(false))
    }
    fn open_input(&self, name: &str, _nth: usize, inbox: Arc<Inbox>) -> Result<Box<dyn Any>, OpenError> {
        let (kind, data) = self.item(name, true).ok_or(OpenError::NotFound)?;
        match kind.as_str() {
            "busy" => return Err(OpenError::InUse),
            "loop" | "synth" => self.loops.borrow_mut().entry(name.to_string()).or_default().push(inbox.clone()),
            "in" if !data.is_empty() => {
                if let Ok(bytes) = parse_hex(&data) {
                    inbox.push(&bytes, now());
                }
            }
            "burst" => {
                let n: usize = data.parse().unwrap_or(1000);
                let mut bytes = Vec::with_capacity(n * 3);
                for i in 0..n {
                    bytes.extend_from_slice(&[0x90, (i % 128) as u8, 100]);
                }
                inbox.push(&bytes, now());
            }
            _ => {}
        }
        Ok(Box::new(TestIn { name: name.to_string(), inbox, loops: self.loops.clone() }))
    }
    fn open_output(&self, name: &str, _nth: usize) -> Result<Box<dyn MidiOut>, OpenError> {
        let (kind, _) = self.item(name, false).ok_or(OpenError::NotFound)?;
        if kind == "busy" {
            return Err(OpenError::InUse);
        }
        Ok(Box::new(TestOut { name: name.to_string(), looped: kind == "loop", synth: kind == "synth", loops: self.loops.clone() }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(bytes: &[u8], ignore: Ignore) -> Vec<Message> {
        let mut p = Parser::new();
        let mut out = Vec::new();
        p.feed(bytes, ignore, &mut |m| out.push(m));
        out
    }

    fn all() -> Ignore {
        Ignore { sysex: false, clock: false, active_sensing: false }
    }

    fn short(b: &[u8]) -> Message {
        Message::short(b)
    }

    #[test]
    fn running_status_is_expanded() {
        // (a note on, then two more with the status left out, then a
        // program change with running status)
        let got = parse(&[0x90, 60, 100, 62, 100, 64, 0, 0xC1, 5, 7], all());
        assert_eq!(got, vec![short(&[0x90, 60, 100]), short(&[0x90, 62, 100]), short(&[0x90, 64, 0]), short(&[0xC1, 5]), short(&[0xC1, 7])]);
    }

    #[test]
    fn real_time_bytes_interrupt_nothing() {
        // (clock between a status and its data, and inside a SysEx)
        let got = parse(&[0x90, 0xF8, 60, 0xFA, 100, 0xF0, 0x7E, 0xF8, 0x7F, 0xF7], all());
        assert_eq!(got, vec![short(&[0xF8]), short(&[0xFA]), short(&[0x90, 60, 100]), short(&[0xF8]), Message::SysEx(vec![0xF0, 0x7E, 0x7F, 0xF7])]);
    }

    #[test]
    fn sysex_in_pieces_is_joined() {
        let mut p = Parser::new();
        let mut out = Vec::new();
        let mut big = vec![0xF0, 0x43, 0x00, 0x09, 0x20, 0x00];
        big.extend((0..4096).map(|i| (i % 128) as u8));
        big.push(0xF7);
        for piece in big.chunks(1024) {
            p.feed(piece, all(), &mut |m| out.push(m));
        }
        assert_eq!(out, vec![Message::SysEx(big)]);
    }

    #[test]
    fn a_cut_sysex_is_dropped_and_said() {
        let got = parse(&[0xF0, 0x7E, 0x01, 0x90, 60, 100], all());
        assert_eq!(got, vec![Message::Problem(SYSEX_CUT.into()), short(&[0x90, 60, 100])]);
        // (a new F0 cuts the one before)
        let got = parse(&[0xF0, 0x01, 0xF0, 0x02, 0xF7], all());
        assert_eq!(got, vec![Message::Problem(SYSEX_CUT.into()), Message::SysEx(vec![0xF0, 0x02, 0xF7])]);
    }

    #[test]
    fn ignore_flags() {
        let stream = [0xF8, 0xFE, 0xF1, 0x23, 0xF0, 0x7E, 0xF7, 0xB0, 7, 100];
        assert_eq!(parse(&stream, Ignore::default()), vec![short(&[0xB0, 7, 100])]);
        assert_eq!(parse(&stream, all()).len(), 5);
        // (an ignored SysEx's data bytes aren't taken for running status)
        assert_eq!(parse(&[0x90, 60, 1, 0xF0, 1, 2, 3, 0xF7, 4, 5], Ignore::default()), vec![short(&[0x90, 60, 1])]);
    }

    #[test]
    fn system_common_and_strays() {
        // (song position: no running status after it; stray data ignored)
        let got = parse(&[5, 6, 0xF2, 1, 2, 3, 0xF6, 0xF3, 9, 0xF4, 1], all());
        assert_eq!(got, vec![short(&[0xF2, 1, 2]), short(&[0xF6]), short(&[0xF3, 9])]);
    }

    #[test]
    fn a_too_long_sysex_is_dropped() {
        let mut p = Parser::new();
        let mut out = Vec::new();
        p.feed(&[0xF0], all(), &mut |m| out.push(m));
        let chunk = vec![1u8; 64 * 1024];
        for _ in 0..17 {
            p.feed(&chunk, all(), &mut |m| out.push(m));
        }
        p.feed(&[0xF7, 0x80, 1, 2], all(), &mut |m| out.push(m));
        assert_eq!(out, vec![Message::Problem(SYSEX_TOO_LONG.into()), short(&[0x80, 1, 2])]);
    }

    #[test]
    fn the_inbox_is_bounded() {
        let inbox = Inbox::new(all());
        let notes: Vec<u8> = (0..MAX_QUEUED + 10).flat_map(|i| [0x90, (i % 128) as u8, 1]).collect();
        inbox.push(&notes, 1.0);
        assert_eq!(inbox.queued(), MAX_QUEUED);
        assert_eq!(inbox.dropped_total(), 10);
        // (the program is told first, then gets the messages in order)
        assert_eq!(inbox.take().map(|a| a.message), Some(Message::Problem(dropped_text(10))));
        assert_eq!(inbox.take().map(|a| a.message), Some(short(&[0x90, 0, 1])));
    }

    #[test]
    fn checks_and_messages() {
        let i = |n: i64| Value::Integer(n);
        assert_eq!(short_message(&[i(0x90), i(60), i(100)]), Ok(vec![0x90, 60, 100]));
        assert_eq!(short_message(&[i(0xC0), i(5)]), Ok(vec![0xC0, 5]));
        assert_eq!(short_message(&[i(0xF8)]), Ok(vec![0xF8]));
        assert!(short_message(&[i(0x90), i(200), i(1)]).unwrap_err().contains("Data1 is 0 to 127 (it was 200)"));
        assert!(short_message(&[i(60)]).is_err());
        assert!(short_message(&[i(0xF0)]).unwrap_err().contains("SendSysEx"));
        assert!(sysex_message(&[0xF0, 1, 2]).is_err());
        assert!(sysex_message(&[0xF0, 1, 200, 0xF7]).unwrap_err().contains("Byte 3"));
        assert_eq!(parse_hex("F0 7E 7F 06 01 F7"), Ok(vec![0xF0, 0x7E, 0x7F, 0x06, 0x01, 0xF7]));
        assert_eq!(parse_hex("&HF0,0x7E $7F F7"), Ok(vec![0xF0, 0x7E, 0x7F, 0xF7]));
        assert_eq!(parse_hex("F07E7F0601F7"), Ok(vec![0xF0, 0x7E, 0x7F, 0x06, 0x01, 0xF7]));
        assert!(parse_hex("G1").is_err());
        assert!(parse_hex("  ").is_err());
        assert_eq!(check_stream(&[0x90, 60, 100, 0xF0, 1, 0xF7, 0xC0, 3]), Ok(()));
        assert!(check_stream(&[0x90, 60]).is_err());
        assert!(check_stream(&[0xF0, 1, 2]).is_err());
        assert_eq!(hex_of(&[0xF0, 0x7E, 0x07, 0xF7]), "F0 7E 07 F7");
    }

    #[test]
    fn duplicate_names_are_told_apart() {
        let d = Devices::of(&["Keys".to_string(), "Synth".to_string(), "Keys".to_string()]);
        assert_eq!(d.names(), vec!["Keys", "Synth", "Keys (2)"]);
        assert_eq!(d.find(&v_str("keys (2)")), Some(2));
        assert_eq!(d.find(&v_str("syn")), Some(1));
        assert_eq!(d.find(&Value::Integer(1)), Some(1));
        assert_eq!(d.find(&Value::Integer(3)), None);
        assert_eq!(d.0[2], ("Keys (2)".to_string(), "Keys".to_string(), 1));
    }

    thread_local! {
        static FIRED: RefCell<Vec<(String, Vec<Value>)>> = const { RefCell::new(Vec::new()) };
        static QUEUED: RefCell<VecDeque<Then>> = RefCell::new(VecDeque::new());
    }

    fn handles(_: &str, _: &str) -> bool {
        true
    }
    fn fire_now(_: &str, ev: &str, args: &[Value], then: Then) {
        FIRED.with(|f| f.borrow_mut().push((ev.to_string(), args.to_vec())));
        then(args);
    }
    fn fire_later(_: &str, ev: &str, args: &[Value], then: Then) {
        FIRED.with(|f| f.borrow_mut().push((ev.to_string(), args.to_vec())));
        QUEUED.with(|q| q.borrow_mut().push_back(then));
    }
    fn clock() -> f64 {
        0.0
    }
    fn setup(script: &str) {
        set_system(Rc::new(TestMidi::parse(script)));
        FIRED.with(|f| f.borrow_mut().clear());
    }

    fn fired() -> Vec<String> {
        FIRED.with(|f| f.borrow().iter().map(|(e, _)| e.clone()).collect())
    }

    #[test]
    fn a_loop_brings_back_what_is_sent_and_events_fire_in_order() {
        setup("loop:Loop;out:Synth;busy:Taken");
        create("m", TYPE);
        assert_eq!(call("m", "inputcount", &[]), Some(v_int(2)));
        assert_eq!(call("m", "outputcount", &[]), Some(v_int(3)));
        assert_eq!(call("m", "openinput", &[v_str("loop")]), Some(v_int(1)));
        assert_eq!(call("m", "openoutput", &[v_str("Loop")]), Some(v_int(1)));
        assert_eq!(get("m", "inputname"), Some(v_str("Loop")));
        set("m", "ignoresysex", &v_int(0));
        call("m", "sendnoteon", &[v_int(1), v_int(60), v_int(100)]);
        call("m", "sendnoteoff", &[v_int(1), v_int(60)]);
        call("m", "sendcontrol", &[v_int(2), v_int(64), v_int(127)]);
        let mut sysex = vec![0xF0, 0x7D];
        sysex.extend((0..4096).map(|i| (i % 128) as u8));
        sysex.push(0xF7);
        call("m", "sendsysex", &[v_str(&super::super::codec::bytes_to_string(&sysex))]);
        assert_eq!(get("m", "queued"), Some(v_int(4)));
        pump(Host { has_handler: handles, fire_then: fire_now, now_ms: clock });
        assert_eq!(fired(), vec!["onmessage", "onnoteon", "onmessage", "onnoteoff", "onmessage", "oncontrol", "onsysex"]);
        let got = FIRED.with(|f| f.borrow().last().cloned()).unwrap();
        assert_eq!(super::super::codec::string_to_bytes(&got.1[0].to_string_val()), sysex);
        // (errors)
        assert_eq!(call("m", "openinput", &[v_str("Taken")]), Some(v_int(0)));
        assert!(get("m", "error").unwrap().to_string_val().contains("in use"));
        assert_eq!(call("m", "openoutput", &[v_str("nothing")]), Some(v_int(0)));
        assert!(get("m", "error").unwrap().to_string_val().contains("No MIDI output is called \"nothing\" (there are: Loop, Synth, Taken)"));
        assert_eq!(call("m", "send", &[v_int(0x90), v_int(300), v_int(1)]), Some(v_int(0)));
        assert_eq!(get("m", "error"), Some(v_str("Data1 is 0 to 127 (it was 300)")));
        remove("m");
    }

    #[test]
    fn the_interpreter_gets_one_message_at_a_time() {
        setup("burst:Keys=5");
        create("k", TYPE);
        assert_eq!(call("k", "openinput", &[v_int(0)]), Some(v_int(1)));
        let host = Host { has_handler: handles, fire_then: fire_later, now_ms: clock };
        pump(host);
        // (the first message's first event, queued; nothing else yet)
        assert_eq!(fired(), vec!["onmessage"]);
        // (each continuation fires the next event, until all five notes)
        while let Some(then) = QUEUED.with(|q| q.borrow_mut().pop_front()) {
            then(&[]);
        }
        assert_eq!(fired().iter().filter(|e| *e == "onnoteon").count(), 5);
        assert_eq!(get("k", "queued"), Some(v_int(0)));
        remove("k");
    }
}

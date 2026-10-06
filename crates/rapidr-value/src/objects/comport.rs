//! QCOMPORT — a serial port, as RapidQ programs have it: RAPIDQ2.INC's
//! COMPORT (John Kelly's, after Pete Kleinschmidt's ComPort.cmp; the
//! manual's QCOMPORT page), since RC.EXE's own QCOMPORT can't be built
//! (docs/io-media-plan.md §2). The runtime installs the ports
//! ([`set_ports`]: serial2 on the desktop, Web Serial in the browser, the
//! tests' [`TestPorts`]); everything else is here, the same everywhere.
//!
//! - **Settings** are read at Open (the library writes the DCB only there):
//!   Port ("COM1"; RapidR: a number n is "COMn"), BaudRate (9600 — the
//!   rate; RAPIDQ.INC's `br110` … `br115200` codes 0 … 12 are taken as the
//!   rates they name), DataBits (8), Parity (prNone … prSpace, 0 … 4),
//!   StopBits (1: see below), ReadBufSize / WriteBufSize (1024), DCBflags
//!   (20625: no flow control; 24725 hardware, 21393 XON / XOFF).
//! - **Open** opens the port and fires OnOpen; a failure fires OnComError
//!   with the library's text and Windows' own message ("There was an error
//!   opening the Comm Port.  The system cannot find the file specified." +
//!   CR LF) and leaves Handle -1, Connected 0.
//! - **WriteString(S, Wait)** / **Write(Stream, Count, Wait)** send, wait
//!   Wait ms, then OnWriteString (or OnComError when bytes are still
//!   waiting to go). **ReadString(Count, Wait)** returns what has arrived
//!   (up to Count; up to a second for the first byte, the library's
//!   timeouts), waits Wait ms, then OnReadString. Each updates the counts
//!   (BytesNotRead / BytesNotWritten: what the library's status call saw);
//!   InQue / OutQue read them now.
//! - **Close** fires OnClose (OnComError "…closing… The handle is
//!   invalid." when nothing is open); PurgeIn / PurgeOut drop what waits.
//! - RC.EXE's own members are there too: Read(Stream, Count, Wait),
//!   PendingIO (0), WaitForLastIO / AbortAllIO / AddFlowControl /
//!   DelFlowControl (nothing to do: I/O here is never left pending),
//!   OnError (fired with OnComError), OnRxChar(InQue) (RapidR fires it when
//!   bytes arrive, the runtime looking as it looks for a joystick's
//!   events); OnTxEmpty / OnRing / OnBreak never fire.

use std::collections::VecDeque;

use crate::{v_int, v_str, Value};

/// RAPIDQ.INC's `br110` … `br115200` (0 … 12).
const BAUD_CODES: [i64; 13] = [110, 300, 600, 1200, 2400, 4800, 9600, 14400, 19200, 38400, 56000, 57600, 115200];

/// A port's settings when it's opened.
#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub baud: u32,
    pub data_bits: u8,
    /// 0 none, 1 odd, 2 even, 3 mark, 4 space.
    pub parity: u8,
    /// 1 or 2.
    pub stop_bits: u8,
    /// None, Hardware (RTS / CTS), Software (XON / XOFF).
    pub flow: Flow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    None,
    Hardware,
    Software,
}

/// Why a port didn't open (Windows' messages, as RapidQ's FormatError
/// gave them).
#[derive(Debug, Clone, PartialEq)]
pub enum PortError {
    /// ERROR_FILE_NOT_FOUND.
    NotFound,
    /// ERROR_ACCESS_DENIED (in use).
    AccessDenied,
    /// ERROR_INVALID_PARAMETER (settings the port can't take).
    InvalidParameter,
    Other(String),
}

impl PortError {
    /// The text FormatMessage gave (with its CR LF).
    pub fn message(&self) -> String {
        let text = match self {
            PortError::NotFound => "The system cannot find the file specified.",
            PortError::AccessDenied => "Access is denied.",
            PortError::InvalidParameter => "The parameter is incorrect.",
            PortError::Other(s) => s.as_str(),
        };
        format!("{text}\r\n")
    }
}

/// What the line did since the runtime last looked ([`Link::line_events`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LineEvents {
    /// The ring indicator came on (a modem's incoming call), how many times.
    pub rings: usize,
    /// Break conditions that arrived (the line held low).
    pub breaks: usize,
}

/// An open port.
pub trait Link {
    fn write(&mut self, bytes: &[u8]) -> Result<(), PortError>;
    /// What has arrived, up to `max` bytes, waiting up to `wait_ms` for the
    /// first one when nothing has.
    fn read(&mut self, max: usize, wait_ms: u64) -> Result<Vec<u8>, PortError>;
    /// Bytes arrived and not read.
    fn in_queue(&mut self) -> usize;
    /// Bytes not sent yet.
    fn out_queue(&mut self) -> usize {
        0
    }
    fn purge(&mut self, input: bool, output: bool);
    /// What the line did since the last call (OnRing, OnBreak); nothing
    /// for a port that can't tell.
    fn line_events(&mut self) -> LineEvents {
        LineEvents::default()
    }
}

/// The system's ports.
pub trait Ports {
    fn open(&self, port: &str, settings: &Settings) -> Result<Box<dyn Link>, PortError>;
}

thread_local! {
    static PORTS: std::cell::RefCell<Option<std::rc::Rc<dyn Ports>>> = const { std::cell::RefCell::new(None) };
    static SLEEPER: std::cell::Cell<fn(u64)> = const { std::cell::Cell::new(no_sleep) };
}

fn no_sleep(_ms: u64) {}

/// The runtime's ports (none until installed: every port is "not found").
pub fn set_ports(ports: std::rc::Rc<dyn Ports>) {
    PORTS.with(|p| *p.borrow_mut() = Some(ports));
}

/// Whether the runtime installed its ports.
pub fn ports_installed() -> bool {
    PORTS.with(|p| p.borrow().is_some())
}

/// How a Wait (ms) waits: the runtime's sleep.
pub fn set_sleeper(sleep: fn(u64)) {
    SLEEPER.with(|s| s.set(sleep));
}

fn sleep(ms: i64) {
    if ms > 0 {
        SLEEPER.with(|s| s.get())(ms.min(600_000) as u64);
    }
}

/// The library's messages.
const OPEN_ERROR: &str = "There was an error opening the Comm Port.  ";
const SETTINGS_ERROR: &str = "There was an error changing the Comm Port settings.  ";
const CLOSE_ERROR: &str = "There was an error closing the Comm Port.  ";
const WRITE_ERROR: &str = "There was an error writing data to the Comm Port.  ";
const READ_ERROR: &str = "There was an error reading data from the Comm Port.  ";
const SEND_TIMEOUT: &str = "Could not send all the data in time specified.  Try increasing your wait time.";

pub struct ComPort {
    pub port: String,
    pub baud: i64,
    pub data_bits: i64,
    pub parity: i64,
    pub stop_bits: i64,
    pub read_buf: i64,
    pub write_buf: i64,
    pub dcb_flags: i64,
    handle: i64,
    bytes_not_read: i64,
    bytes_not_written: i64,
    link: Option<Box<dyn Link>>,
    /// Events to fire once the call returns (event, arguments).
    pub events: VecDeque<(&'static str, Vec<Value>)>,
    /// The InQue OnRxChar last told (the runtime's looks).
    rx_told: usize,
}

impl Default for ComPort {
    fn default() -> Self {
        ComPort {
            port: "COM1".into(),
            baud: 9600,
            data_bits: 8,
            parity: 0,
            stop_bits: 1,
            read_buf: 1024,
            write_buf: 1024,
            dcb_flags: 20625,
            handle: 0,
            bytes_not_read: 0,
            bytes_not_written: 0,
            link: None,
            events: VecDeque::new(),
            rx_told: 0,
        }
    }
}

static NEXT_HANDLE: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0x400);

/// Write's / Read's stream: its bytes (`None`), or bytes written into it.
pub type StreamAccess<'a> = &'a mut dyn FnMut(&str, Option<&[u8]>) -> Vec<u8>;

/// The members RC.EXE refuses to set (`C.CONNECTED is a read-only value.`).
pub const READ_ONLY: &[&str] = &["Connected", "Handle", "InQue", "OutQue", "PendingIO", "BytesNotRead", "BytesNotWritten"];

/// The events the runtime looks for (OnRxChar, OnRing, OnBreak).
pub const EVENTS: [&str; 3] = ["onrxchar", "onring", "onbreak"];

impl ComPort {
    fn settings(&self) -> Result<Settings, PortError> {
        let baud = if (0..=12).contains(&self.baud) { BAUD_CODES[self.baud as usize] } else { self.baud };
        if baud <= 0 || baud > u32::MAX as i64 || !(5..=8).contains(&self.data_bits) || !(0..=4).contains(&self.parity) || !(0..=2).contains(&self.stop_bits) {
            return Err(PortError::InvalidParameter);
        }
        // (the DCB's flow-control bits: fOutxCtsFlow 0x4 / fRtsControl
        // handshake (2 << 12) for hardware, fOutX 0x100 / fInX 0x200 for
        // XON / XOFF)
        let flow = if self.dcb_flags & 0x4 != 0 || (self.dcb_flags >> 12) & 3 == 2 {
            Flow::Hardware
        } else if self.dcb_flags & 0x0300 != 0 {
            Flow::Software
        } else {
            Flow::None
        };
        Ok(Settings {
            baud: baud as u32,
            data_bits: self.data_bits as u8,
            parity: self.parity as u8,
            // (the library's default 1 — ONE5STOPBITS in the DCB — is one
            // stop bit, as its author meant: docs/io-media-plan.md)
            stop_bits: if self.stop_bits == 2 { 2 } else { 1 },
            flow,
        })
    }

    fn error(&mut self, text: String) {
        self.events.push_back(("oncomerror", vec![v_str(&text)]));
        self.events.push_back(("onerror", vec![v_str(&text)]));
    }

    fn status(&mut self) {
        if let Some(l) = self.link.as_mut() {
            self.bytes_not_read = l.in_queue() as i64;
            self.bytes_not_written = l.out_queue() as i64;
        }
    }

    pub fn get(&mut self, prop: &str) -> Option<Value> {
        Some(match prop {
            "port" => v_str(&self.port),
            "baudrate" => v_int(self.baud),
            "databits" => v_int(self.data_bits),
            "parity" => v_int(self.parity),
            "stopbits" => v_int(self.stop_bits),
            "readbufsize" => v_int(self.read_buf),
            "writebufsize" => v_int(self.write_buf),
            "dcbflags" => v_int(self.dcb_flags),
            "handle" => v_int(self.handle),
            "connected" => v_int(i64::from(self.link.is_some())),
            "bytesnotread" => v_int(self.bytes_not_read),
            "bytesnotwritten" => v_int(self.bytes_not_written),
            "inque" | "outque" => {
                self.status();
                v_int(if prop == "inque" { self.bytes_not_read } else { self.bytes_not_written })
            }
            "pendingio" => v_int(0),
            _ => return None,
        })
    }

    pub fn set(&mut self, prop: &str, val: &Value) -> Option<()> {
        let n = || val.to_i64();
        match prop {
            "port" => {
                self.port = match val {
                    Value::Integer(_) | Value::Double(_) => format!("COM{}", n()),
                    _ => val.to_string_val(),
                }
            }
            "baudrate" => self.baud = n(),
            "databits" => self.data_bits = n(),
            "parity" => self.parity = n(),
            "stopbits" => self.stop_bits = n(),
            "readbufsize" => self.read_buf = n(),
            "writebufsize" => self.write_buf = n(),
            "dcbflags" => self.dcb_flags = n(),
            // (read-only: RC.EXE refuses it at compile time)
            "handle" | "connected" | "bytesnotread" | "bytesnotwritten" | "inque" | "outque" | "pendingio" => {}
            _ => return None,
        }
        Some(())
    }

    /// The bytes of stream `Write` sends (the runtime reads the stream).
    pub fn call(&mut self, method: &str, args: &[Value], stream: StreamAccess) -> Option<Value> {
        let arg = |i: usize| args.get(i).cloned().unwrap_or(Value::Null);
        match method {
            "open" => self.open(),
            "close" => self.close(),
            "purgein" | "purgeout" => {
                if let Some(l) = self.link.as_mut() {
                    l.purge(method == "purgein", method == "purgeout");
                }
            }
            "writestring" => {
                let bytes: Vec<u8> = arg(0).to_string_val().chars().map(|c| c as u32 as u8).collect();
                self.send(&bytes, arg(1).to_i64());
            }
            "write" => {
                let mut bytes = stream(&arg(0).to_string_val(), None);
                let count = arg(1).to_i64();
                if count > 0 {
                    bytes.truncate(count as usize);
                }
                self.send(&bytes, arg(2).to_i64());
            }
            "readstring" => {
                let got = self.receive(arg(0).to_i64(), arg(1).to_i64());
                return Some(v_str(&got.iter().map(|&b| char::from(b)).collect::<String>()));
            }
            "read" => {
                let got = self.receive(arg(1).to_i64(), arg(2).to_i64());
                stream(&arg(0).to_string_val(), Some(&got));
            }
            "waitforlastio" | "abortallio" | "addflowcontrol" | "delflowcontrol" => {}
            _ => return None,
        }
        Some(Value::Null)
    }

    /// Open's first half: the settings to open the port with, or `None`
    /// when it failed already (open; settings it can't take).
    pub fn open_begin(&mut self) -> Option<Settings> {
        if self.link.is_some() {
            // (CreateFile on a port this program has open)
            self.handle = -1;
            self.error(format!("{OPEN_ERROR}{}", PortError::AccessDenied.message()));
            return None;
        }
        match self.settings() {
            Ok(s) => Some(s),
            Err(e) => {
                self.error(format!("{SETTINGS_ERROR}{}", e.message()));
                None
            }
        }
    }

    /// Open's end: the port opened (OnOpen), or why not (OnComError).
    pub fn open_end(&mut self, opened: Result<Box<dyn Link>, PortError>) {
        match opened {
            Ok(link) => {
                self.link = Some(link);
                self.handle = NEXT_HANDLE.fetch_add(4, std::sync::atomic::Ordering::Relaxed);
                self.rx_told = 0;
                self.events.push_back(("onopen", Vec::new()));
            }
            Err(PortError::InvalidParameter) => self.error(format!("{SETTINGS_ERROR}{}", PortError::InvalidParameter.message())),
            Err(e) => {
                self.handle = -1;
                self.error(format!("{OPEN_ERROR}{}", e.message()));
            }
        }
    }

    fn open(&mut self) {
        let Some(settings) = self.open_begin() else { return };
        let ports = PORTS.with(|p| p.borrow().clone());
        let opened = match ports {
            Some(p) => p.open(&self.port, &settings),
            None => Err(PortError::NotFound),
        };
        self.open_end(opened);
    }

    fn close(&mut self) {
        if self.link.take().is_some() {
            self.handle = 0;
            self.events.push_back(("onclose", Vec::new()));
        } else {
            self.error(format!("{CLOSE_ERROR}The handle is invalid.\r\n"));
        }
    }

    fn send(&mut self, bytes: &[u8], wait: i64) {
        let Some(l) = self.link.as_mut() else {
            return self.error(format!("{WRITE_ERROR}The handle is invalid.\r\n"));
        };
        if let Err(e) = l.write(bytes) {
            return self.error(format!("{WRITE_ERROR}{}", e.message()));
        }
        sleep(wait);
        self.status();
        if self.bytes_not_written == 0 {
            self.events.push_back(("onwritestring", Vec::new()));
            // (all sent: the output buffer is empty — Windows' EV_TXEMPTY)
            self.events.push_back(("ontxempty", Vec::new()));
        } else {
            self.error(SEND_TIMEOUT.to_string());
        }
    }

    fn receive(&mut self, count: i64, wait: i64) -> Vec<u8> {
        let Some(l) = self.link.as_mut() else {
            self.error(format!("{READ_ERROR}The handle is invalid.\r\n"));
            return Vec::new();
        };
        let got = match l.read(count.max(0) as usize, 1000) {
            Ok(b) => b,
            Err(e) => {
                self.error(format!("{READ_ERROR}{}", e.message()));
                return Vec::new();
            }
        };
        sleep(wait);
        self.status();
        self.rx_told = self.bytes_not_read as usize;
        self.events.push_back(("onreadstring", Vec::new()));
        got
    }

    /// The runtime's look (while the program handles OnRxChar, OnRing or
    /// OnBreak): a break that arrived (OnBreak), the ring indicator come on
    /// (OnRing), then OnRxChar with InQue when more has arrived since it was
    /// last told.
    pub fn look(&mut self) -> Vec<(&'static str, Vec<Value>)> {
        let Some(l) = self.link.as_mut() else { return Vec::new() };
        let mut out = Vec::new();
        let line = l.line_events();
        out.extend(std::iter::repeat_n(("onbreak", Vec::new()), line.breaks.min(16)));
        out.extend(std::iter::repeat_n(("onring", Vec::new()), line.rings.min(16)));
        let n = l.in_queue();
        if n > self.rx_told {
            out.push(("onrxchar", vec![v_int(n as i64)]));
        }
        self.rx_told = n;
        out
    }
}

/// The tests' ports (`RAPIDR_TEST_COMPORT` on the desktop, the page's
/// `RAPIDR_TEST_COMPORT` in the browser): `NAME:kind` items separated by
/// `;` — `echo` (what's written comes back), `reply:TEXT` (each write is
/// answered with TEXT; `\r`, `\n` escapes), `busy` (in use); a kind
/// ending `+ring` or `+break` (or both) also rings, or sends a break, once
/// after the port opens. Any other port isn't there. Never a real device.
pub struct TestPorts {
    ports: Vec<(String, String)>,
}

impl TestPorts {
    pub fn parse(script: &str) -> Self {
        let ports = script
            .split(';')
            .filter_map(|item| {
                let (name, kind) = item.split_once(':')?;
                Some((name.trim().to_ascii_uppercase(), kind.to_string()))
            })
            .collect();
        TestPorts { ports }
    }
}

struct TestLink {
    reply: Option<Vec<u8>>,
    input: VecDeque<u8>,
    line: LineEvents,
}

impl Link for TestLink {
    fn write(&mut self, bytes: &[u8]) -> Result<(), PortError> {
        match &self.reply {
            Some(r) => self.input.extend(r.iter().copied()),
            None => self.input.extend(bytes.iter().copied()),
        }
        Ok(())
    }
    fn read(&mut self, max: usize, _wait_ms: u64) -> Result<Vec<u8>, PortError> {
        let n = max.min(self.input.len());
        Ok(self.input.drain(..n).collect())
    }
    fn in_queue(&mut self) -> usize {
        self.input.len()
    }
    fn purge(&mut self, input: bool, _output: bool) {
        if input {
            self.input.clear();
        }
    }
    fn line_events(&mut self) -> LineEvents {
        std::mem::take(&mut self.line)
    }
}

impl Ports for TestPorts {
    fn open(&self, port: &str, settings: &Settings) -> Result<Box<dyn Link>, PortError> {
        let kind = self.ports.iter().find(|(n, _)| n.eq_ignore_ascii_case(port.trim())).map(|(_, k)| k.as_str()).ok_or(PortError::NotFound)?;
        // (as a UART refuses them)
        if settings.parity > 2 {
            return Err(PortError::InvalidParameter);
        }
        // (`+ring` / `+break`: the line's events, once after it opens)
        let mut kind = kind;
        let mut line = LineEvents::default();
        loop {
            if let Some(k) = kind.strip_suffix("+ring") {
                line.rings += 1;
                kind = k;
            } else if let Some(k) = kind.strip_suffix("+break") {
                line.breaks += 1;
                kind = k;
            } else {
                break;
            }
        }
        let reply = match kind.split_once(':') {
            Some(("reply", text)) => Some(text.replace("\\r", "\r").replace("\\n", "\n").chars().map(|c| c as u32 as u8).collect()),
            _ if kind == "busy" => return Err(PortError::AccessDenied),
            _ => None,
        };
        Ok(Box::new(TestLink { reply, input: VecDeque::new(), line }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_stream(_: &str, _: Option<&[u8]>) -> Vec<u8> {
        Vec::new()
    }

    #[test]
    fn open_write_read_close() {
        set_ports(std::rc::Rc::new(TestPorts::parse("COM2:echo;COM3:busy;COM4:reply:OK\\r\\n")));
        let mut c = ComPort::default();
        c.call("open", &[], &mut no_stream);
        assert_eq!(c.events.pop_front().map(|e| e.0), Some("oncomerror"));
        assert_eq!(c.get("handle").unwrap().to_i64(), -1);
        c.events.clear();
        c.set("port", &v_int(2));
        c.call("open", &[], &mut no_stream);
        assert_eq!(c.events.pop_front().map(|e| e.0), Some("onopen"));
        assert_eq!(c.get("connected").unwrap().to_i64(), 1);
        c.call("writestring", &[v_str("hi"), v_int(0)], &mut no_stream);
        assert_eq!(c.events.pop_front().map(|e| e.0), Some("onwritestring"));
        assert_eq!(c.events.pop_front().map(|e| e.0), Some("ontxempty"));
        assert_eq!(c.get("bytesnotread").unwrap().to_i64(), 2);
        assert_eq!(c.call("readstring", &[v_int(1), v_int(0)], &mut no_stream).unwrap().to_string_val(), "h");
        assert_eq!(c.get("bytesnotread").unwrap().to_i64(), 1);
        c.call("close", &[], &mut no_stream);
        c.events.clear();
        c.call("close", &[], &mut no_stream);
        let (e, args) = c.events.pop_front().unwrap();
        assert_eq!((e, args[0].to_string_val()), ("oncomerror", "There was an error closing the Comm Port.  The handle is invalid.\r\n".to_string()));
        let mut busy = ComPort { port: "com3".into(), ..ComPort::default() };
        busy.call("open", &[], &mut no_stream);
        assert!(busy.events[0].1[0].to_string_val().ends_with("Access is denied.\r\n"));
        let mut modem = ComPort { port: "COM4".into(), baud: 6, ..ComPort::default() };
        modem.call("open", &[], &mut no_stream);
        modem.call("writestring", &[v_str("ATZ\r\n"), v_int(0)], &mut no_stream);
        assert_eq!(modem.call("readstring", &[v_int(100), v_int(0)], &mut no_stream).unwrap().to_string_val(), "OK\r\n");
        assert_eq!(modem.settings().unwrap().baud, 9600);
    }

    #[test]
    fn ring_and_break_come_to_the_look() {
        set_ports(std::rc::Rc::new(TestPorts::parse("COM5:echo+ring+break;COM6:reply:a+b+ring")));
        let mut c = ComPort { port: "COM5".into(), ..ComPort::default() };
        c.call("open", &[], &mut no_stream);
        let names: Vec<&str> = c.look().into_iter().map(|e| e.0).collect();
        assert_eq!(names, ["onbreak", "onring"]);
        assert!(c.look().is_empty(), "once");
        c.call("writestring", &[v_str("x"), v_int(0)], &mut no_stream);
        assert_eq!(c.look().into_iter().map(|e| e.0).collect::<Vec<_>>(), ["onrxchar"]);
        let mut m = ComPort { port: "COM6".into(), ..ComPort::default() };
        m.call("open", &[], &mut no_stream);
        assert_eq!(m.look().into_iter().map(|e| e.0).collect::<Vec<_>>(), ["onring"]);
        m.call("writestring", &[v_str("x"), v_int(0)], &mut no_stream);
        assert_eq!(m.call("readstring", &[v_int(9), v_int(0)], &mut no_stream).unwrap().to_string_val(), "a+b");
    }
}

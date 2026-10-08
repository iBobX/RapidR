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
//! - **RapidR's extras** (additive; `origin = "rapidr"` in the registry):
//!   the system's ports listed (ListPorts, PortCount, PortName(i),
//!   PortDescription(i), PortManufacturer(i), PortSerialNumber(i),
//!   PortVendorID(i), PortProductID(i); FillList(ListOrCombo) — the
//!   runtime fills the control), the modem lines (DTR and RTS set and read
//!   back, CTS / DSR / CD / RI read, SendBreak(ms)), whole lines (LineEnd,
//!   ReadLine(Timeout), HasLine, OnLine(Line)) and OnPortsChanged(Added,
//!   Removed) when a USB adapter is plugged in or out (the runtime's looks).

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

/// A port the system has (ListPorts): its name, and what the system says
/// of its device — a USB adapter's vendor and product IDs, its
/// description, maker and serial number (empty or 0 where it says nothing).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PortInfo {
    pub name: String,
    pub description: String,
    pub manufacturer: String,
    pub serial_number: String,
    pub vid: u16,
    pub pid: u16,
}

impl PortInfo {
    /// What a list shows for it: the name, then its description (the
    /// system's, else the USB chip its IDs name).
    pub fn item(&self) -> String {
        let d = self.describe();
        if d.is_empty() {
            self.name.clone()
        } else {
            format!("{} ({d})", self.name)
        }
    }

    /// The system's description, else the chip its USB IDs name.
    pub fn describe(&self) -> String {
        if !self.description.trim().is_empty() {
            return self.description.trim().to_string();
        }
        usb_chip(self.vid, self.pid).unwrap_or_default().to_string()
    }
}

/// The USB serial chips IoT boards carry, by vendor and product ID (public
/// USB IDs): where the system gives no description.
pub fn usb_chip(vid: u16, pid: u16) -> Option<&'static str> {
    Some(match (vid, pid) {
        (0x10C4, 0xEA60) => "CP210x USB to UART",
        (0x10C4, 0xEA70) => "CP2105 USB to UART",
        (0x10C4, 0xEA71) => "CP2108 USB to UART",
        (0x1A86, 0x7523) => "CH340 USB to serial",
        (0x1A86, 0x5523) => "CH341 USB to serial",
        (0x1A86, 0x55D4) => "CH9102 USB to serial",
        (0x1A86, 0x55D3) => "CH343 USB to serial",
        (0x0403, 0x6001) => "FT232R USB to UART",
        (0x0403, 0x6010) => "FT2232 USB to UART",
        (0x0403, 0x6011) => "FT4232 USB to UART",
        (0x0403, 0x6014) => "FT232H USB to UART",
        (0x0403, 0x6015) => "FT-X USB to UART",
        (0x067B, 0x2303) => "PL2303 USB to serial",
        (0x303A, 0x1001) => "USB JTAG/serial debug unit (ESP32)",
        (0x303A, _) => "Espressif USB serial",
        (0x2341, _) | (0x2A03, _) => "Arduino",
        (0x2E8A, _) => "Raspberry Pi Pico",
        (0x239A, _) => "Adafruit board",
        _ => return None,
    })
}

/// The lines a port reads (the other end's): Clear To Send, Data Set
/// Ready, Carrier Detect, Ring Indicator.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Signals {
    pub cts: bool,
    pub dsr: bool,
    pub cd: bool,
    pub ri: bool,
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
    /// Data Terminal Ready set (asserted) or cleared.
    fn set_dtr(&mut self, _on: bool) -> Result<(), PortError> {
        Ok(())
    }
    /// Request To Send set (asserted) or cleared.
    fn set_rts(&mut self, _on: bool) -> Result<(), PortError> {
        Ok(())
    }
    /// The lines the other end drives, now.
    fn signals(&mut self) -> Signals {
        Signals::default()
    }
    /// A break held for `ms` milliseconds.
    fn send_break(&mut self, _ms: u64) -> Result<(), PortError> {
        Ok(())
    }
    /// How many bytes, `end` included, make the first whole line that has
    /// arrived — waiting up to `wait_ms` for one; `None`: no line yet.
    fn line_length(&mut self, end: &[u8], wait_ms: u64) -> Option<usize>;
}

/// Where `end` first ends in `bytes`: the length up to and including it.
pub fn find_end<'a>(bytes: impl IntoIterator<Item = &'a u8>, end: &[u8]) -> Option<usize> {
    if end.is_empty() {
        return None;
    }
    let mut tail: VecDeque<u8> = VecDeque::with_capacity(end.len());
    for (i, &b) in bytes.into_iter().enumerate() {
        if tail.len() == end.len() {
            tail.pop_front();
        }
        tail.push_back(b);
        if tail.len() == end.len() && tail.iter().eq(end.iter()) {
            return Some(i + 1);
        }
    }
    None
}

/// The system's ports.
pub trait Ports {
    fn open(&self, port: &str, settings: &Settings) -> Result<Box<dyn Link>, PortError>;
    /// The ports there are now (ListPorts).
    fn list(&self) -> Vec<PortInfo> {
        Vec::new()
    }
}

/// The installed ports' list (none installed: none).
pub fn list_ports() -> Vec<PortInfo> {
    let ports = PORTS.with(|p| p.borrow().clone());
    ports.map(|p| p.list()).unwrap_or_default()
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
    /// DTR and RTS as the program set them (`None`: as DCBflags has them).
    dtr: Option<bool>,
    rts: Option<bool>,
    /// What ends a line (ReadLine, OnLine).
    pub line_end: String,
    /// The ports ListPorts found (`None`: not looked yet).
    ports: Option<Vec<PortInfo>>,
    /// The port names OnPortsChanged last saw, and the looks since.
    seen_ports: Option<Vec<String>>,
    looks: u32,
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
            dtr: None,
            rts: None,
            line_end: "\n".into(),
            ports: None,
            seen_ports: None,
            looks: 0,
        }
    }
}

static NEXT_HANDLE: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0x400);

/// Write's / Read's stream: its bytes (`None`), or bytes written into it.
pub type StreamAccess<'a> = &'a mut dyn FnMut(&str, Option<&[u8]>) -> Vec<u8>;

/// The members RC.EXE refuses to set (`C.CONNECTED is a read-only value.`).
pub const READ_ONLY: &[&str] = &["Connected", "Handle", "InQue", "OutQue", "PendingIO", "BytesNotRead", "BytesNotWritten"];

/// The events the runtime looks for (OnRxChar; RapidR's OnLine and
/// OnPortsChanged).
pub const EVENTS: [&str; 3] = ["onrxchar", "online", "onportschanged"];

/// Looks between two looks at the system's ports (OnPortsChanged): about a
/// second.
const PORT_LOOKS: u32 = 20;

/// ReadLine's wait when no Timeout is given (ms): the library's read
/// timeout.
const LINE_WAIT: i64 = 1000;

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

    /// DTR as the program set it, else as DCBflags' fDtrControl has it
    /// (enabled, the library's default).
    fn dtr_now(&self) -> bool {
        self.dtr.unwrap_or((self.dcb_flags >> 4) & 3 != 0)
    }

    /// RTS as the program set it, else as DCBflags' fRtsControl has it.
    fn rts_now(&self) -> bool {
        self.rts.unwrap_or((self.dcb_flags >> 12) & 3 != 0)
    }

    /// The system's ports, looked at again.
    fn list(&mut self) -> &[PortInfo] {
        self.ports.insert(list_ports())
    }

    /// The ports ListPorts found (looked at now if it never ran).
    fn ports(&mut self) -> &[PortInfo] {
        if self.ports.is_none() {
            self.list();
        }
        self.ports.as_deref().unwrap_or_default()
    }

    /// Port `i` of the list (0 is the first).
    fn port_at(&mut self, i: i64) -> Option<PortInfo> {
        usize::try_from(i).ok().and_then(|i| self.ports().get(i).cloned())
    }

    /// FillList's items: the ports, looked at again, as a list shows them.
    pub fn port_items(&mut self) -> Vec<String> {
        self.list().iter().map(PortInfo::item).collect()
    }

    fn line_end_bytes(&self) -> Vec<u8> {
        let end: Vec<u8> = self.line_end.chars().map(|c| c as u32 as u8).collect();
        if end.is_empty() {
            vec![b'\n']
        } else {
            end
        }
    }

    /// A whole line that has arrived (waiting up to `wait` ms for one),
    /// without its end — and with the default end (LF), without a CR
    /// before it: devices end lines with CR LF or LF.
    fn take_line(&mut self, wait: u64) -> Option<String> {
        let end = self.line_end_bytes();
        let l = self.link.as_mut()?;
        let n = l.line_length(&end, wait)?;
        let mut bytes = l.read(n, 0).ok()?;
        bytes.truncate(bytes.len().saturating_sub(end.len()));
        if end == b"\n" && bytes.last() == Some(&b'\r') {
            bytes.pop();
        }
        Some(bytes.iter().map(|&b| char::from(b)).collect())
    }

    fn signal_error(&mut self, e: PortError) {
        self.error(format!("{SETTINGS_ERROR}{}", e.message()));
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
            // (RapidR's)
            "dtr" => v_int(i64::from(self.dtr_now())),
            "rts" => v_int(i64::from(self.rts_now())),
            "cts" | "dsr" | "cd" | "ri" => {
                let s = self.link.as_mut().map(|l| l.signals()).unwrap_or_default();
                v_int(i64::from(match prop {
                    "cts" => s.cts,
                    "dsr" => s.dsr,
                    "cd" => s.cd,
                    _ => s.ri,
                }))
            }
            "lineend" => v_str(&self.line_end),
            "hasline" => {
                let end = self.line_end_bytes();
                v_int(i64::from(self.link.as_mut().and_then(|l| l.line_length(&end, 0)).is_some()))
            }
            "portcount" => v_int(self.ports().len() as i64),
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
            // (RapidR's: the line changes now when the port is open, else
            // at Open)
            "dtr" | "rts" => {
                let on = val.to_bool();
                if prop == "dtr" {
                    self.dtr = Some(on);
                } else {
                    self.rts = Some(on);
                }
                if let Some(l) = self.link.as_mut() {
                    let r = if prop == "dtr" { l.set_dtr(on) } else { l.set_rts(on) };
                    if let Err(e) = r {
                        self.signal_error(e);
                    }
                }
            }
            "lineend" => self.line_end = val.to_string_val(),
            "cts" | "dsr" | "cd" | "ri" | "hasline" | "portcount" => {}
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
            // (RapidR's)
            "listports" => return Some(v_int(self.list().len() as i64)),
            "portname" | "portdescription" | "portmanufacturer" | "portserialnumber" => {
                let p = self.port_at(arg(0).to_i64()).unwrap_or_default();
                return Some(v_str(&match method {
                    "portname" => p.name,
                    "portdescription" => p.describe(),
                    "portmanufacturer" => p.manufacturer,
                    _ => p.serial_number,
                }));
            }
            "portvendorid" | "portproductid" => {
                let p = self.port_at(arg(0).to_i64()).unwrap_or_default();
                return Some(v_int(i64::from(if method == "portvendorid" { p.vid } else { p.pid })));
            }
            "readline" => {
                let wait = match arg(0) {
                    Value::Null => LINE_WAIT,
                    v => v.to_i64().max(0),
                };
                if self.link.is_none() {
                    self.error(format!("{READ_ERROR}The handle is invalid.\r\n"));
                    return Some(v_str(""));
                }
                let line = self.take_line(wait as u64).unwrap_or_default();
                self.status();
                self.rx_told = self.bytes_not_read as usize;
                return Some(v_str(&line));
            }
            "sendbreak" => {
                let ms = match arg(0) {
                    Value::Null => 250,
                    v => v.to_i64().clamp(0, 10_000),
                };
                match self.link.as_mut() {
                    None => self.error(format!("{WRITE_ERROR}The handle is invalid.\r\n")),
                    Some(l) => {
                        if let Err(e) = l.send_break(ms as u64) {
                            self.error(format!("{WRITE_ERROR}{}", e.message()));
                        }
                    }
                }
            }
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
            Ok(mut link) => {
                // (the modem lines as the program set them, or as DCBflags
                // has them; RTS is the handshake's with hardware flow)
                let _ = link.set_dtr(self.dtr_now());
                if self.settings().map(|s| s.flow) != Ok(Flow::Hardware) {
                    let _ = link.set_rts(self.rts_now());
                }
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

    /// The runtime's look (while the program handles one of [`EVENTS`];
    /// `handled` says which): OnPortsChanged(Added, Removed) when ports
    /// came or went since the last look at them (about a second ago), each
    /// whole line arrived as OnLine(Line), and OnRxChar with InQue when
    /// more has arrived since it was last told.
    pub fn look(&mut self, handled: &dyn Fn(&str) -> bool) -> Vec<(&'static str, Vec<Value>)> {
        let mut out = Vec::new();
        if handled("onportschanged") {
            if self.looks % PORT_LOOKS == 0 {
                let now: Vec<String> = list_ports().into_iter().map(|p| p.name).collect();
                if let Some(before) = &self.seen_ports {
                    let added: Vec<&str> = now.iter().filter(|n| !before.contains(n)).map(String::as_str).collect();
                    let removed: Vec<&str> = before.iter().filter(|n| !now.contains(n)).map(String::as_str).collect();
                    if !added.is_empty() || !removed.is_empty() {
                        out.push(("onportschanged", vec![v_str(&added.join("\r\n")), v_str(&removed.join("\r\n"))]));
                        self.ports = None;
                    }
                }
                self.seen_ports = Some(now);
            }
            self.looks = self.looks.wrapping_add(1);
        }
        if self.link.is_none() {
            return out;
        }
        if handled("online") {
            // (a look's worth: the rest at the next)
            for _ in 0..200 {
                let Some(line) = self.take_line(0) else { break };
                out.push(("online", vec![v_str(&line)]));
            }
        }
        let Some(l) = self.link.as_mut() else { return out };
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
/// answered with TEXT; `\r`, `\n` escapes), `busy` (in use), `esp32` (an
/// ESP32 board behind a CP2102N: DTR / RTS drive its EN and IO0 as the
/// usual auto-reset circuit does, and a reset prints a boot log). Any
/// other port isn't there. Never a real device.
///
/// The lines loop back as a test plug's do: RTS to CTS, DTR to DSR and CD.
/// The script is read again whenever the ports are listed or one opens
/// (a program's `ENVIRON "RAPIDR_TEST_COMPORT=…"` plugs one in or out).
pub struct TestPorts {
    ports: std::cell::RefCell<Vec<(String, String)>>,
    /// The script the ports were made with (used while the environment's
    /// is empty).
    first: String,
}

fn parse_script(script: &str) -> Vec<(String, String)> {
    script
        .split(';')
        .filter_map(|item| {
            let (name, kind) = item.split_once(':')?;
            Some((name.trim().to_ascii_uppercase(), kind.to_string()))
        })
        .collect()
}

/// What the scripted ESP32 prints when it comes out of reset (its ROM's
/// first lines, then its program's).
/// (As a real ESP32 printed them: an M5StickC Plus, 2026-10-08.)
pub const ESP32_BOOT: &str = "ets Jun  8 2016 00:22:57\r\n\r\nrst:0x1 (POWERON_RESET),boot:0x13 (SPI_FAST_FLASH_BOOT)\r\nconfigsip: 0, SPIWP:0xee\r\nmode:DIO, clock div:2\r\nentry 0x400805e4\r\nHello from ESP32\r\n";

impl TestPorts {
    pub fn parse(script: &str) -> Self {
        TestPorts { ports: std::cell::RefCell::new(parse_script(script)), first: script.to_string() }
    }

    /// The script read again (the environment's, else the first).
    fn refresh(&self) {
        let now = crate::environ::get("RAPIDR_TEST_COMPORT");
        *self.ports.borrow_mut() = parse_script(if now.is_empty() { &self.first } else { &now });
    }
}

struct TestLink {
    reply: Option<Vec<u8>>,
    input: VecDeque<u8>,
    dtr: bool,
    rts: bool,
    esp32: bool,
}

impl TestLink {
    /// The scripted ESP32's EN is low (held in reset) while RTS is set and
    /// DTR is not (the auto-reset circuit's two transistors).
    fn in_reset(&self) -> bool {
        self.rts && !self.dtr
    }

    fn lines_changed(&mut self, was_reset: bool) {
        if self.esp32 && was_reset && !self.in_reset() {
            self.input.extend(ESP32_BOOT.bytes());
        }
    }
}

impl Link for TestLink {
    fn write(&mut self, bytes: &[u8]) -> Result<(), PortError> {
        if self.esp32 {
            return Ok(());
        }
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
    fn set_dtr(&mut self, on: bool) -> Result<(), PortError> {
        let was = self.in_reset();
        self.dtr = on;
        self.lines_changed(was);
        Ok(())
    }
    fn set_rts(&mut self, on: bool) -> Result<(), PortError> {
        let was = self.in_reset();
        self.rts = on;
        self.lines_changed(was);
        Ok(())
    }
    fn signals(&mut self) -> Signals {
        Signals { cts: self.rts, dsr: self.dtr, cd: self.dtr, ri: false }
    }
    fn line_length(&mut self, end: &[u8], _wait_ms: u64) -> Option<usize> {
        find_end(&self.input, end)
    }
}

impl Ports for TestPorts {
    fn open(&self, port: &str, settings: &Settings) -> Result<Box<dyn Link>, PortError> {
        self.refresh();
        let ports = self.ports.borrow();
        let kind = ports.iter().find(|(n, _)| n.eq_ignore_ascii_case(port.trim())).map(|(_, k)| k.as_str()).ok_or(PortError::NotFound)?;
        // (as a UART refuses them)
        if settings.parity > 2 {
            return Err(PortError::InvalidParameter);
        }
        let reply = match kind.split_once(':') {
            Some(("reply", text)) => Some(text.replace("\\r", "\r").replace("\\n", "\n").chars().map(|c| c as u32 as u8).collect()),
            _ if kind == "busy" => return Err(PortError::AccessDenied),
            _ => None,
        };
        Ok(Box::new(TestLink { reply, input: VecDeque::new(), dtr: false, rts: false, esp32: kind == "esp32" }))
    }

    fn list(&self) -> Vec<PortInfo> {
        self.refresh();
        self.ports
            .borrow()
            .iter()
            .map(|(name, kind)| match kind.as_str() {
                "esp32" => PortInfo {
                    name: name.clone(),
                    description: "CP2102N USB to UART Bridge Controller".into(),
                    manufacturer: "Silicon Labs".into(),
                    serial_number: "0001".into(),
                    vid: 0x10C4,
                    pid: 0xEA60,
                },
                _ => PortInfo { name: name.clone(), ..PortInfo::default() },
            })
            .collect()
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
    fn rapidr_extras() {
        set_ports(std::rc::Rc::new(TestPorts::parse("COM2:echo;COM5:esp32")));
        let mut c = ComPort::default();
        // (the ports listed, a USB adapter's IDs and description)
        assert_eq!(c.call("listports", &[], &mut no_stream).unwrap().to_i64(), 2);
        assert_eq!(c.get("portcount").unwrap().to_i64(), 2);
        assert_eq!(c.call("portname", &[v_int(1)], &mut no_stream).unwrap().to_string_val(), "COM5");
        assert_eq!(c.call("portvendorid", &[v_int(1)], &mut no_stream).unwrap().to_i64(), 0x10C4);
        assert_eq!(c.call("portproductid", &[v_int(1)], &mut no_stream).unwrap().to_i64(), 0xEA60);
        assert_eq!(c.call("portname", &[v_int(9)], &mut no_stream).unwrap().to_string_val(), "");
        assert_eq!(c.port_items(), vec!["COM2".to_string(), "COM5 (CP2102N USB to UART Bridge Controller)".to_string()]);
        // (DTR / RTS: DCBflags' until set; read back; looped to DSR / CTS)
        assert_eq!((c.get("dtr").unwrap().to_i64(), c.get("rts").unwrap().to_i64()), (1, 1));
        c.set("port", &v_str("COM2"));
        c.call("open", &[], &mut no_stream);
        assert_eq!((c.get("dsr").unwrap().to_i64(), c.get("cts").unwrap().to_i64()), (1, 1));
        c.set("rts", &v_int(0));
        assert_eq!((c.get("rts").unwrap().to_i64(), c.get("cts").unwrap().to_i64()), (0, 0));
        // (whole lines: LF, CR LF, another end; a part waits for the rest)
        c.call("writestring", &[v_str("one\r\ntwo\nthr"), v_int(0)], &mut no_stream);
        assert_eq!(c.get("hasline").unwrap().to_i64(), 1);
        assert_eq!(c.call("readline", &[v_int(0)], &mut no_stream).unwrap().to_string_val(), "one");
        assert_eq!(c.call("readline", &[], &mut no_stream).unwrap().to_string_val(), "two");
        assert_eq!(c.call("readline", &[v_int(0)], &mut no_stream).unwrap().to_string_val(), "");
        assert_eq!(c.get("inque").unwrap().to_i64(), 3);
        c.set("lineend", &v_str(";"));
        c.call("writestring", &[v_str("ee;"), v_int(0)], &mut no_stream);
        assert_eq!(c.call("readline", &[v_int(0)], &mut no_stream).unwrap().to_string_val(), "three");
        c.call("close", &[], &mut no_stream);
        c.events.clear();
        // (the scripted ESP32: a reset through DTR / RTS prints its boot log)
        let mut e = ComPort { port: "COM5".into(), baud: 115200, ..ComPort::default() };
        e.call("open", &[], &mut no_stream);
        assert_eq!(e.get("inque").unwrap().to_i64(), 0);
        e.set("dtr", &v_int(0));
        e.set("rts", &v_int(1));
        assert_eq!(e.get("inque").unwrap().to_i64(), 0);
        e.set("rts", &v_int(0));
        assert_eq!(e.call("readline", &[v_int(0)], &mut no_stream).unwrap().to_string_val(), "ets Jun  8 2016 00:22:57");
        // (OnLine at the runtime's looks, when the program handles it)
        let all = |_: &str| true;
        let lines: Vec<String> = e.look(&all).into_iter().filter(|(n, _)| *n == "online").map(|(_, a)| a[0].to_string_val()).collect();
        assert_eq!(lines.len(), 6);
        assert!(lines[1].starts_with("rst:0x1 (POWERON_RESET)"));
        e.call("sendbreak", &[v_int(0)], &mut no_stream);
        assert!(e.events.iter().all(|(n, _)| *n != "oncomerror"));
    }

    #[test]
    fn line_ends() {
        let q: VecDeque<u8> = b"ab\r\ncd".iter().copied().collect();
        assert_eq!(find_end(&q, b"\n"), Some(4));
        assert_eq!(find_end(&q, b"\r\n"), Some(4));
        assert_eq!(find_end(&q, b"x"), None);
        assert_eq!(find_end(&q, b""), None);
        assert_eq!(usb_chip(0x1A86, 0x7523), Some("CH340 USB to serial"));
        assert_eq!(PortInfo { name: "COM3".into(), vid: 0x303A, pid: 0x1001, ..PortInfo::default() }.item(), "COM3 (USB JTAG/serial debug unit (ESP32))");
    }
}

//! QCOMPORT's ports on the desktop (rapidr_value::objects::comport): the
//! system's serial ports through serial2 (BSD-2-Clause OR Apache-2.0; libc
//! on Unix, windows-sys on Windows — nothing to install to build), or the
//! tests' scripted ones (`RAPIDR_TEST_COMPORT`, also set empty by the test
//! runners: then no port exists, so a test never opens a real device).
//!
//! A port's name: on Windows `COM1` …; elsewhere a device path
//! (`/dev/ttyUSB0`, `/dev/cu.usbserial-…`), or `COMn` for the n-th port
//! the system lists (serial2's `available_ports`, sorted).
//! What arrives is read by a thread of the port's own into a queue, so
//! InQue / BytesNotRead and OnRxChar know what is waiting.

use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use rapidr_value::objects::comport::{self, Flow, Link, PortError, PortInfo, Ports, Settings, Signals};

/// Installs the ports, once (when the first QCOMPORT is made).
pub fn install() {
    if comport::ports_installed() {
        return;
    }
    comport::set_sleeper(|ms| std::thread::sleep(Duration::from_millis(ms)));
    match std::env::var("RAPIDR_TEST_COMPORT") {
        Ok(script) => comport::set_ports(Rc::new(comport::TestPorts::parse(&script))),
        Err(_) => comport::set_ports(Rc::new(SystemPorts)),
    }
}

struct SystemPorts;

/// The device a port name means here.
fn device(name: &str) -> Result<String, PortError> {
    let name = name.trim();
    if cfg!(windows) {
        // (`\\.\COM10`: CreateFile's name for ports past COM9)
        return Ok(if name.to_ascii_uppercase().starts_with("COM") { format!(r"\\.\{name}") } else { name.to_string() });
    }
    let upper = name.to_ascii_uppercase();
    if let Some(n) = upper.strip_prefix("COM").and_then(|n| n.parse::<usize>().ok()) {
        let mut ports = serial2::SerialPort::available_ports().unwrap_or_default();
        ports.sort();
        return ports.get(n.wrapping_sub(1)).map(|p| p.to_string_lossy().into_owned()).ok_or(PortError::NotFound);
    }
    Ok(name.to_string())
}

fn port_error(e: &std::io::Error) -> PortError {
    match e.kind() {
        std::io::ErrorKind::NotFound => PortError::NotFound,
        std::io::ErrorKind::PermissionDenied => PortError::AccessDenied,
        std::io::ErrorKind::InvalidInput => PortError::InvalidParameter,
        _ if e.raw_os_error() == Some(16) => PortError::AccessDenied, // EBUSY
        _ => PortError::Other(e.to_string()),
    }
}

/// serial2's settings for QCOMPORT's.
fn configure(s: &mut serial2::Settings, w: &Settings) -> std::io::Result<()> {
    use serial2::{CharSize, FlowControl, Parity, StopBits};
    s.set_raw();
    s.set_baud_rate(w.baud)?;
    s.set_char_size(match w.data_bits {
        5 => CharSize::Bits5,
        6 => CharSize::Bits6,
        7 => CharSize::Bits7,
        _ => CharSize::Bits8,
    });
    s.set_stop_bits(if w.stop_bits == 2 { StopBits::Two } else { StopBits::One });
    s.set_parity(match w.parity {
        0 => Parity::None,
        1 => Parity::Odd,
        2 => Parity::Even,
        // (mark / space: not every system has them)
        _ => return Err(std::io::Error::from(std::io::ErrorKind::InvalidInput)),
    });
    s.set_flow_control(match w.flow {
        Flow::None => FlowControl::None,
        Flow::Hardware => FlowControl::RtsCts,
        Flow::Software => FlowControl::XonXoff,
    });
    Ok(())
}

impl Ports for SystemPorts {
    fn open(&self, port: &str, settings: &Settings) -> Result<Box<dyn Link>, PortError> {
        let path = device(port)?;
        let w = settings.clone();
        let opened = serial2::SerialPort::open(&path, move |mut s: serial2::Settings| {
            configure(&mut s, &w)?;
            Ok(s)
        });
        let mut p = opened.map_err(|e| port_error(&e))?;
        Ok(Box::new(SystemLink::start(&mut p).map_err(|e| port_error(&e))?))
    }

    fn list(&self) -> Vec<PortInfo> {
        crate::serial_list::ports()
    }
}

/// What the reader thread put in.
#[derive(Default)]
struct Inbox {
    bytes: Mutex<VecDeque<u8>>,
    arrived: Condvar,
}

pub(crate) struct SystemLink {
    port: Arc<serial2::SerialPort>,
    inbox: Arc<Inbox>,
    stop: Arc<AtomicBool>,
}

impl SystemLink {
    /// The port, read from now on by a thread of its own.
    pub(crate) fn start(p: &mut serial2::SerialPort) -> std::io::Result<Self> {
        p.set_read_timeout(Duration::from_millis(50))?;
        let port = Arc::new(p.try_clone()?);
        let (inbox, stop) = (Arc::new(Inbox::default()), Arc::new(AtomicBool::new(false)));
        let (reader, inbox2, stop2) = (port.clone(), inbox.clone(), stop.clone());
        std::thread::Builder::new()
            .name("rapidr-comport".into())
            .spawn(move || {
                let mut buf = [0u8; 4096];
                while !stop2.load(Ordering::Relaxed) {
                    match reader.read(&mut buf) {
                        Ok(0) => {}
                        Ok(n) => {
                            inbox2.bytes.lock().unwrap().extend(&buf[..n]);
                            inbox2.arrived.notify_all();
                        }
                        Err(e) if matches!(e.kind(), std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted) => {}
                        Err(_) => std::thread::sleep(Duration::from_millis(50)),
                    }
                }
            })?;
        Ok(SystemLink { port, inbox, stop })
    }
}

impl Drop for SystemLink {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// A modem line's change refused: the port's error, else nothing.
fn line(r: std::io::Result<()>) -> Result<(), PortError> {
    r.map_err(|e| port_error(&e))
}

impl Link for SystemLink {
    fn write(&mut self, bytes: &[u8]) -> Result<(), PortError> {
        self.port.write_all(bytes).map_err(|e| port_error(&e))?;
        self.port.flush().map_err(|e| port_error(&e))
    }
    fn read(&mut self, max: usize, wait_ms: u64) -> Result<Vec<u8>, PortError> {
        let mut q = self.inbox.bytes.lock().unwrap();
        if q.is_empty() && wait_ms > 0 && max > 0 {
            q = self.inbox.arrived.wait_timeout(q, Duration::from_millis(wait_ms)).unwrap().0;
        }
        let n = max.min(q.len());
        Ok(q.drain(..n).collect())
    }
    fn in_queue(&mut self) -> usize {
        self.inbox.bytes.lock().unwrap().len()
    }
    fn purge(&mut self, input: bool, output: bool) {
        if input {
            let _ = self.port.discard_input_buffer();
            self.inbox.bytes.lock().unwrap().clear();
        }
        if output {
            let _ = self.port.discard_output_buffer();
        }
    }
    fn set_dtr(&mut self, on: bool) -> Result<(), PortError> {
        line(self.port.set_dtr(on))
    }
    fn set_rts(&mut self, on: bool) -> Result<(), PortError> {
        line(self.port.set_rts(on))
    }
    fn signals(&mut self) -> Signals {
        let p = &self.port;
        Signals { cts: p.read_cts().unwrap_or(false), dsr: p.read_dsr().unwrap_or(false), cd: p.read_cd().unwrap_or(false), ri: p.read_ri().unwrap_or(false) }
    }
    fn send_break(&mut self, ms: u64) -> Result<(), PortError> {
        line(self.port.set_break(true))?;
        std::thread::sleep(Duration::from_millis(ms));
        line(self.port.set_break(false))
    }
    fn line_length(&mut self, end: &[u8], wait_ms: u64) -> Option<usize> {
        let deadline = std::time::Instant::now() + Duration::from_millis(wait_ms);
        let mut q = self.inbox.bytes.lock().unwrap();
        loop {
            if let Some(n) = comport::find_end(q.iter(), end) {
                return Some(n);
            }
            let left = deadline.saturating_duration_since(std::time::Instant::now());
            if left.is_zero() {
                return None;
            }
            q = self.inbox.arrived.wait_timeout(q, left).unwrap().0;
        }
    }
}

// (Linux: macOS' pseudo-terminals refuse serial2's IOSSIOSPEED, which its
// real ports take)
#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    /// The real path (a device opened by its path with the program's
    /// settings, serial2, the reader thread) over a pseudo-terminal: no
    /// device needed.
    #[test]
    fn a_pseudo_terminal() {
        use std::io::{Read, Write};
        use std::os::fd::FromRawFd;
        let (mut master, mut slave) = (0, 0);
        // SAFETY: openpty fills the two descriptors; ttyname's string is
        // copied at once.
        let path = unsafe {
            assert_eq!(libc::openpty(&mut master, &mut slave, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut()), 0);
            std::ffi::CStr::from_ptr(libc::ttyname(slave)).to_string_lossy().into_owned()
        };
        let settings = Settings { baud: 9600, data_bits: 8, parity: 0, stop_bits: 1, flow: Flow::None };
        let mut link = SystemPorts.open(&path, &settings).map_err(|e| e.message()).unwrap();
        // SAFETY: the master descriptor openpty gave, owned from here on.
        let mut b = unsafe { std::fs::File::from_raw_fd(master) };
        b.write_all(b"hello").unwrap();
        let got = link.read(3, 1000).unwrap();
        assert_eq!(got, b"hel");
        // (the rest arrives in the queue)
        for _ in 0..100 {
            if link.in_queue() == 2 {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(link.in_queue(), 2);
        link.write(b"ok").unwrap();
        let mut buf = [0u8; 2];
        b.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"ok");
        link.purge(true, false);
        assert_eq!(link.in_queue(), 0);
        // SAFETY: the slave descriptor openpty gave, closed once.
        unsafe { libc::close(slave) };
    }
}

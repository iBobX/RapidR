//! Gamepads from the Linux kernel's evdev (`/dev/input/event*`), read with
//! libc alone — gilrs's Linux backend needs libudev's headers to build.
//! A device is a gamepad when it has BTN_GAMEPAD (or BTN_JOYSTICK) and an
//! X axis; its keys and axes are the kernel's gamepad codes
//! (Documentation/input/gamepad.rst) as xpad (Xbox) reports them
//! ([`super::standard_of_evdev`]). The devices are looked for again every
//! second (plugged in, unplugged); one that can't be opened (no
//! permission: logind gives the seat's user its joysticks) is left out.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::AsRawFd;
use std::time::{Duration, Instant};

use rapidr_value::objects::joystick::{Pad, Source};

const EV_KEY: u16 = 0x01;
const EV_ABS: u16 = 0x03;
const BTN_JOYSTICK: usize = 0x120;
const BTN_GAMEPAD: usize = 0x130;
const KEY_MAX: usize = 0x2ff;
/// ABS_X … ABS_HAT0Y.
const AXES: usize = 0x12;

#[repr(C)]
struct InputEvent {
    time: libc::timeval,
    kind: u16,
    code: u16,
    value: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct AbsInfo {
    value: i32,
    minimum: i32,
    maximum: i32,
    fuzz: i32,
    flat: i32,
    resolution: i32,
}

/// `_IOC(_IOC_READ, 'E', nr, size)` (the generic layout: x86, ARM, RISC-V).
fn eviocg(nr: u64, size: usize) -> u64 {
    (2 << 30) | ((size as u64) << 16) | ((b'E' as u64) << 8) | nr
}

/// ioctl `nr` reading into `buf`; false if it failed.
fn ioctl_read(file: &File, nr: u64, buf: &mut [u8]) -> bool {
    // SAFETY: the request reads at most `buf.len()` bytes (its size field)
    // into `buf`, which lives for the call.
    unsafe { libc::ioctl(file.as_raw_fd(), eviocg(nr, buf.len()) as _, buf.as_mut_ptr()) >= 0 }
}

fn bit(bits: &[u8], i: usize) -> bool {
    bits.get(i / 8).is_some_and(|b| b & (1 << (i % 8)) != 0)
}

struct Device {
    file: File,
    name: String,
    /// The axes the device has (their ranges) and their values.
    axes: [Option<AbsInfo>; AXES],
    keys: HashSet<u16>,
}

impl Device {
    /// `path` opened: the device if it's a gamepad; Err if it can't be
    /// opened (yet: logind may give the user access a moment after).
    fn open(path: &str) -> Result<Option<Device>, ()> {
        let file = OpenOptions::new().read(true).custom_flags(libc::O_NONBLOCK).open(path).map_err(|_| ())?;
        Ok(Device::gamepad(file))
    }

    fn gamepad(file: File) -> Option<Device> {
        let mut types = [0u8; 4];
        let mut keys = [0u8; KEY_MAX / 8 + 1];
        let mut abs = [0u8; 8];
        if !ioctl_read(&file, 0x20, &mut types) || !bit(&types, EV_KEY as usize) || !bit(&types, EV_ABS as usize) {
            return None;
        }
        if !ioctl_read(&file, 0x20 + EV_KEY as u64, &mut keys) || !ioctl_read(&file, 0x20 + EV_ABS as u64, &mut abs) {
            return None;
        }
        if !(bit(&keys, BTN_GAMEPAD) || bit(&keys, BTN_JOYSTICK)) || !bit(&abs, 0) {
            return None;
        }
        let mut name = [0u8; 128];
        let name = if ioctl_read(&file, 0x06, &mut name) { String::from_utf8_lossy(name.split(|&b| b == 0).next().unwrap_or(&[])).into_owned() } else { String::new() };
        let mut axes = [None; AXES];
        for (code, axis) in axes.iter_mut().enumerate().filter(|(code, _)| bit(&abs, *code)) {
            let mut info = [0u8; std::mem::size_of::<AbsInfo>()];
            if ioctl_read(&file, 0x40 + code as u64, &mut info) {
                // SAFETY: AbsInfo is six i32s, `info` its bytes as the kernel wrote them.
                *axis = Some(unsafe { std::ptr::read_unaligned(info.as_ptr().cast::<AbsInfo>()) });
            }
        }
        // (what's held down already)
        let mut down = [0u8; KEY_MAX / 8 + 1];
        let held = if ioctl_read(&file, 0x18, &mut down) { (0..=KEY_MAX).filter(|&k| bit(&down, k)).map(|k| k as u16).collect() } else { HashSet::new() };
        Some(Device { file, name, axes, keys: held })
    }

    /// The events since the last read; false when the device is gone.
    fn read(&mut self) -> bool {
        let size = std::mem::size_of::<InputEvent>();
        let mut buf = vec![0u8; size * 64];
        loop {
            match self.file.read(&mut buf) {
                Ok(0) => return true,
                Ok(n) => {
                    for chunk in buf[..n - n % size].chunks_exact(size) {
                        // SAFETY: an input_event's bytes as the kernel wrote them.
                        let e = unsafe { std::ptr::read_unaligned(chunk.as_ptr().cast::<InputEvent>()) };
                        match e.kind {
                            EV_KEY if e.value != 0 => {
                                self.keys.insert(e.code);
                            }
                            EV_KEY => {
                                self.keys.remove(&e.code);
                            }
                            EV_ABS => {
                                if let Some(Some(a)) = self.axes.get_mut(e.code as usize) {
                                    a.value = e.value;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return true,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(_) => return false,
            }
        }
    }

    fn pad(&self) -> Pad {
        let axes = self.axes.map(|a| a.map(|a| (a.value, a.minimum, a.maximum)));
        Pad::from_standard(&super::standard_of_evdev(&self.name, &axes, &self.keys))
    }
}

#[derive(Default)]
pub struct Evdev {
    /// The gamepads open, by event number.
    devices: BTreeMap<u32, Device>,
    /// Devices looked at that aren't gamepads (or can't be opened).
    skipped: HashSet<u32>,
    scanned: Option<Instant>,
}

impl Evdev {
    fn scan(&mut self) {
        let Ok(dir) = std::fs::read_dir("/dev/input") else { return };
        let present: HashMap<u32, String> = dir
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                let n = name.strip_prefix("event")?.parse().ok()?;
                Some((n, e.path().to_string_lossy().into_owned()))
            })
            .collect();
        // (gone: another device may take its number)
        self.skipped.retain(|n| present.contains_key(n));
        self.devices.retain(|n, _| present.contains_key(n));
        for (n, path) in present {
            if self.devices.contains_key(&n) || self.skipped.contains(&n) {
                continue;
            }
            match Device::open(&path) {
                Ok(Some(d)) => {
                    self.devices.insert(n, d);
                }
                Ok(None) => {
                    self.skipped.insert(n);
                }
                // (tried again at the next look)
                Err(()) => {}
            }
        }
    }
}

impl Source for Evdev {
    fn pads(&mut self) -> Vec<Pad> {
        if self.scanned.is_none_or(|t| t.elapsed() >= Duration::from_secs(1)) {
            self.scanned = Some(Instant::now());
            self.scan();
        }
        self.devices.retain(|_, d| d.read());
        self.devices.values().map(Device::pad).collect()
    }
}

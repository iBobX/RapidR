//! The system's serial ports with what the system says of their devices
//! (QCOMPORT's ListPorts: rapidr_value::objects::comport::PortInfo) — a USB
//! adapter's vendor and product IDs, its product name, maker and serial
//! number. No crate of its own: IOKit on macOS, SetupAPI on Windows
//! (windows-sys, serial2's own), sysfs on Linux. Every port serial2 finds
//! is listed, with nothing more where the system says nothing.

use rapidr_value::objects::comport::PortInfo;

/// The ports there are now, sorted by name.
pub fn ports() -> Vec<PortInfo> {
    let mut found = detailed();
    for path in serial2::SerialPort::available_ports().unwrap_or_default() {
        let name = path.to_string_lossy().into_owned();
        // (macOS: each port is there twice — `tty.` waits for a carrier,
        // `cu.` doesn't: the one a program opens)
        if cfg!(target_os = "macos") && name.starts_with("/dev/tty.") {
            continue;
        }
        if !found.iter().any(|p| p.name == name) {
            found.push(PortInfo { name, ..PortInfo::default() });
        }
    }
    found.sort_by(|a, b| a.name.cmp(&b.name));
    found
}

#[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
fn detailed() -> Vec<PortInfo> {
    Vec::new()
}

// ---------------------------------------------------------------- macOS --

#[cfg(target_os = "macos")]
fn detailed() -> Vec<PortInfo> {
    mac::ports()
}

#[cfg(target_os = "macos")]
#[allow(non_camel_case_types)]
mod mac {
    use std::ffi::{c_char, c_void, CStr};

    use rapidr_value::objects::comport::PortInfo;

    type CFTypeRef = *const c_void;
    type io_object_t = u32;
    const UTF8: u32 = 0x0800_0100;
    const NUMBER_SINT64: isize = 4;
    /// kIORegistryIterateRecursively | kIORegistryIterateParents
    const UP_THE_TREE: u32 = 3;

    #[link(name = "IOKit", kind = "framework")]
    extern "C" {
        fn IOServiceMatching(name: *const c_char) -> *mut c_void;
        fn IOServiceGetMatchingServices(main: u32, matching: *mut c_void, iter: *mut io_object_t) -> i32;
        fn IOIteratorNext(iter: io_object_t) -> io_object_t;
        fn IOObjectRelease(o: io_object_t) -> i32;
        fn IORegistryEntryCreateCFProperty(entry: io_object_t, key: CFTypeRef, alloc: CFTypeRef, options: u32) -> CFTypeRef;
        fn IORegistryEntrySearchCFProperty(entry: io_object_t, plane: *const c_char, key: CFTypeRef, alloc: CFTypeRef, options: u32) -> CFTypeRef;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFStringCreateWithCString(alloc: CFTypeRef, s: *const c_char, encoding: u32) -> CFTypeRef;
        fn CFStringGetCString(s: CFTypeRef, buf: *mut c_char, size: isize, encoding: u32) -> u8;
        fn CFGetTypeID(cf: CFTypeRef) -> usize;
        fn CFStringGetTypeID() -> usize;
        fn CFNumberGetTypeID() -> usize;
        fn CFNumberGetValue(n: CFTypeRef, kind: isize, out: *mut c_void) -> u8;
        fn CFRelease(cf: CFTypeRef);
    }

    /// A CF value's text or number (released here).
    enum Prop {
        Text(String),
        Number(i64),
        Other,
    }

    /// SAFETY (all of them): IOKit's and CoreFoundation's C calls with the
    /// objects they gave, each released once.
    unsafe fn take(v: CFTypeRef) -> Option<Prop> {
        if v.is_null() {
            return None;
        }
        let p = if CFGetTypeID(v) == CFStringGetTypeID() {
            let mut buf = [0 as c_char; 512];
            if CFStringGetCString(v, buf.as_mut_ptr(), buf.len() as isize, UTF8) != 0 {
                Prop::Text(CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned())
            } else {
                Prop::Other
            }
        } else if CFGetTypeID(v) == CFNumberGetTypeID() {
            let mut n: i64 = 0;
            CFNumberGetValue(v, NUMBER_SINT64, &mut n as *mut i64 as *mut c_void);
            Prop::Number(n)
        } else {
            Prop::Other
        };
        CFRelease(v);
        Some(p)
    }

    unsafe fn key(name: &CStr) -> CFTypeRef {
        CFStringCreateWithCString(std::ptr::null(), name.as_ptr(), UTF8)
    }

    /// The entry's own property.
    unsafe fn own(entry: io_object_t, name: &CStr) -> Option<Prop> {
        let k = key(name);
        let v = IORegistryEntryCreateCFProperty(entry, k, std::ptr::null(), 0);
        CFRelease(k);
        take(v)
    }

    /// The property, from the entry or the nearest of its parents (the USB
    /// device a serial port is on).
    unsafe fn up(entry: io_object_t, name: &CStr) -> Option<Prop> {
        let k = key(name);
        let v = IORegistryEntrySearchCFProperty(entry, c"IOService".as_ptr(), k, std::ptr::null(), UP_THE_TREE);
        CFRelease(k);
        take(v)
    }

    fn text(p: Option<Prop>) -> String {
        match p {
            Some(Prop::Text(s)) => s,
            _ => String::new(),
        }
    }

    fn number(p: Option<Prop>) -> u16 {
        match p {
            Some(Prop::Number(n)) => n as u16,
            _ => 0,
        }
    }

    pub fn ports() -> Vec<PortInfo> {
        let mut out = Vec::new();
        // SAFETY: see `take`; the matching dictionary is consumed by
        // IOServiceGetMatchingServices, the iterator and each service
        // released once.
        unsafe {
            let matching = IOServiceMatching(c"IOSerialBSDClient".as_ptr());
            if matching.is_null() {
                return out;
            }
            let mut iter: io_object_t = 0;
            if IOServiceGetMatchingServices(0, matching, &mut iter) != 0 {
                return out;
            }
            loop {
                let service = IOIteratorNext(iter);
                if service == 0 {
                    break;
                }
                let name = text(own(service, c"IOCalloutDevice"));
                if !name.is_empty() {
                    let vid = number(up(service, c"idVendor"));
                    let mut info = PortInfo { name, ..PortInfo::default() };
                    if vid != 0 {
                        info.vid = vid;
                        info.pid = number(up(service, c"idProduct"));
                        info.description = text(up(service, c"USB Product Name"));
                        if info.description.is_empty() {
                            info.description = text(up(service, c"kUSBProductString"));
                        }
                        info.manufacturer = text(up(service, c"USB Vendor Name"));
                        info.serial_number = text(up(service, c"USB Serial Number"));
                    }
                    out.push(info);
                }
                IOObjectRelease(service);
            }
            IOObjectRelease(iter);
        }
        out
    }
}

// ---------------------------------------------------------------- Linux --

#[cfg(target_os = "linux")]
fn detailed() -> Vec<PortInfo> {
    serial2::SerialPort::available_ports()
        .unwrap_or_default()
        .into_iter()
        .map(|p| linux::info(std::path::Path::new("/sys/class/tty"), &p.to_string_lossy()))
        .collect()
}

#[cfg(any(target_os = "linux", test))]
pub(crate) mod linux {
    use std::path::Path;

    use rapidr_value::objects::comport::PortInfo;

    fn read(dir: &Path, file: &str) -> String {
        std::fs::read_to_string(dir.join(file)).map(|s| s.trim().to_string()).unwrap_or_default()
    }

    /// Port `name` (`/dev/ttyUSB0`) with what sysfs (`root`: its
    /// /sys/class/tty) says of its USB device: the nearest parent of its
    /// device with `idVendor`.
    pub fn info(root: &Path, name: &str) -> PortInfo {
        let mut info = PortInfo { name: name.to_string(), ..PortInfo::default() };
        let tty = name.rsplit('/').next().unwrap_or(name);
        let Ok(mut dir) = std::fs::canonicalize(root.join(tty).join("device")) else { return info };
        loop {
            if dir.join("idVendor").is_file() {
                info.vid = u16::from_str_radix(&read(&dir, "idVendor"), 16).unwrap_or(0);
                info.pid = u16::from_str_radix(&read(&dir, "idProduct"), 16).unwrap_or(0);
                info.description = read(&dir, "product");
                info.manufacturer = read(&dir, "manufacturer");
                info.serial_number = read(&dir, "serial");
                return info;
            }
            if !dir.pop() || dir.as_os_str().len() <= 1 {
                return info;
            }
        }
    }
}

// -------------------------------------------------------------- Windows --

#[cfg(windows)]
fn detailed() -> Vec<PortInfo> {
    win::ports()
}

/// A Windows device instance ID (`USB\VID_10C4&PID_EA60\0001`): its USB
/// IDs and serial number (an ID Windows made up has a `&` in it).
#[cfg(any(windows, test))]
pub(crate) fn usb_ids(instance: &str) -> (u16, u16, String) {
    let upper = instance.to_ascii_uppercase();
    let hex = |tag: &str| upper.find(tag).and_then(|i| upper.get(i + tag.len()..i + tag.len() + 4)).and_then(|h| u16::from_str_radix(h, 16).ok()).unwrap_or(0);
    let (vid, pid) = (hex("VID_"), hex("PID_"));
    let serial = if vid == 0 {
        String::new()
    } else if upper.starts_with("FTDIBUS") {
        // (FTDI's own driver: `FTDIBUS\VID_0403+PID_6001+71D269456EA\0000`,
        // the serial number then the chip's port letter)
        let s = instance.split('\\').nth(1).and_then(|m| m.split('+').nth(2)).unwrap_or("");
        s.get(..s.len().saturating_sub(1)).unwrap_or("").to_string()
    } else {
        instance.rsplit('\\').next().filter(|s| !s.contains('&')).unwrap_or("").to_string()
    };
    (vid, pid, serial)
}

#[cfg(windows)]
mod win {
    use super::usb_ids;
    use rapidr_value::objects::comport::PortInfo;
    use windows_sys::Win32::Devices::DeviceAndDriverInstallation::{
        SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo, SetupDiGetClassDevsW, SetupDiGetDeviceInstanceIdW, SetupDiGetDeviceRegistryPropertyW, SetupDiOpenDevRegKey, DICS_FLAG_GLOBAL, DIGCF_PRESENT, DIREG_DEV, GUID_DEVCLASS_PORTS, SPDRP_FRIENDLYNAME, SPDRP_MFG, SP_DEVINFO_DATA,
    };
    use windows_sys::Win32::System::Registry::{RegCloseKey, RegQueryValueExW, KEY_READ};

    fn wide_text(buf: &[u16]) -> String {
        let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        String::from_utf16_lossy(&buf[..end])
    }

    pub fn ports() -> Vec<PortInfo> {
        let mut out = Vec::new();
        // SAFETY: SetupAPI's device list, walked and destroyed once; each
        // buffer is ours and its size given.
        unsafe {
            let set = SetupDiGetClassDevsW(&GUID_DEVCLASS_PORTS, std::ptr::null(), std::ptr::null_mut(), DIGCF_PRESENT);
            if set as isize == -1 {
                return out;
            }
            let mut i = 0;
            loop {
                let mut dev: SP_DEVINFO_DATA = std::mem::zeroed();
                dev.cbSize = std::mem::size_of::<SP_DEVINFO_DATA>() as u32;
                if SetupDiEnumDeviceInfo(set, i, &mut dev) == 0 {
                    break;
                }
                i += 1;
                // (its COM name: the device's registry key's PortName)
                let key = SetupDiOpenDevRegKey(set, &dev, DICS_FLAG_GLOBAL, 0, DIREG_DEV, KEY_READ);
                if key as isize == -1 || key.is_null() {
                    continue;
                }
                let mut buf = [0u16; 256];
                let mut size = (buf.len() * 2) as u32;
                let found = RegQueryValueExW(key, windows_sys::w!("PortName"), std::ptr::null(), std::ptr::null_mut(), buf.as_mut_ptr() as *mut u8, &mut size) == 0;
                RegCloseKey(key);
                let name = wide_text(&buf);
                if !found || !name.to_ascii_uppercase().starts_with("COM") {
                    continue;
                }
                let prop = |which: u32| {
                    let mut buf = [0u16; 512];
                    let ok = SetupDiGetDeviceRegistryPropertyW(set, &dev, which, std::ptr::null_mut(), buf.as_mut_ptr() as *mut u8, (buf.len() * 2) as u32, std::ptr::null_mut()) != 0;
                    if ok {
                        wide_text(&buf)
                    } else {
                        String::new()
                    }
                };
                // ("Silicon Labs CP210x USB to UART Bridge (COM3)": the
                // description without its port)
                let mut description = prop(SPDRP_FRIENDLYNAME);
                if let Some(cut) = description.rfind(&format!(" ({name})")) {
                    description.truncate(cut);
                }
                let mut id = [0u16; 512];
                let instance = if SetupDiGetDeviceInstanceIdW(set, &dev, id.as_mut_ptr(), id.len() as u32, std::ptr::null_mut()) != 0 { wide_text(&id) } else { String::new() };
                let (vid, pid, serial_number) = usb_ids(&instance);
                out.push(PortInfo { name, description, manufacturer: prop(SPDRP_MFG), serial_number, vid, pid });
            }
            SetupDiDestroyDeviceInfoList(set);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// sysfs as Linux has it for a CP2102 (a fake tree: no device needed).
    #[test]
    fn linux_sysfs() {
        let root = std::env::temp_dir().join(format!("rapidr-sysfs-{}", std::process::id()));
        let usb = root.join("devices/usb1/1-1");
        let iface = usb.join("1-1:1.0/ttyUSB0");
        std::fs::create_dir_all(&iface).unwrap();
        std::fs::create_dir_all(root.join("class/tty/ttyUSB0")).unwrap();
        for (f, v) in [("idVendor", "10c4\n"), ("idProduct", "ea60\n"), ("product", "CP2102N USB to UART Bridge Controller\n"), ("manufacturer", "Silicon Labs\n"), ("serial", "0001\n")] {
            std::fs::write(usb.join(f), v).unwrap();
        }
        #[cfg(unix)]
        std::os::unix::fs::symlink(&iface, root.join("class/tty/ttyUSB0/device")).unwrap();
        #[cfg(unix)]
        {
            let p = linux::info(&root.join("class/tty"), "/dev/ttyUSB0");
            assert_eq!((p.vid, p.pid), (0x10C4, 0xEA60));
            assert_eq!(p.description, "CP2102N USB to UART Bridge Controller");
            assert_eq!((p.manufacturer.as_str(), p.serial_number.as_str()), ("Silicon Labs", "0001"));
            // (a port with no USB device: its name only)
            assert_eq!(linux::info(&root.join("class/tty"), "/dev/ttyS0"), PortInfo { name: "/dev/ttyS0".into(), ..PortInfo::default() });
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn windows_instance_ids() {
        assert_eq!(usb_ids(r"USB\VID_10C4&PID_EA60\0001"), (0x10C4, 0xEA60, "0001".to_string()));
        assert_eq!(usb_ids(r"FTDIBUS\VID_0403+PID_6001+71D269456EA\0000"), (0x0403, 0x6001, "71D269456E".to_string()));
        assert_eq!(usb_ids(r"USB\VID_1A86&PID_7523\5&2E1A2B3C&0&2"), (0x1A86, 0x7523, String::new()));
        assert_eq!(usb_ids(r"ACPI\PNP0501\1"), (0, 0, String::new()));
    }

    /// The listing runs on this machine (whatever ports it has).
    #[test]
    fn this_machine() {
        let ports = ports();
        assert!(ports.windows(2).all(|w| w[0].name <= w[1].name));
        assert!(ports.iter().all(|p| !p.name.is_empty()));
    }
}

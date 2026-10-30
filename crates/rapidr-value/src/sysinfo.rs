//! **RSystemInfo**: what this computer has — its system, processor,
//! memory, graphics, disk and battery — read here, never sent anywhere
//! (docs/local-ai-plan.md §1). A program adapts to it (a `Class` in plain
//! words: small, medium, large); RapidR Studio's fit check for AI models on
//! this computer reads the same facts.
//!
//! The desktop reads the system's own tables (macOS: `sysctl` and the Mach
//! host's memory statistics; Linux: `/proc` and `/sys`; Windows: the
//! memory status, the registry, the power status) — no new dependency, no
//! process started but `pmset` for a Mac's battery, and only when the
//! battery is asked about. A web page gives less ([`Facts::browser`]): the
//! page fills what the browser tells it and `Unknown` names the rest.

use std::cell::RefCell;
use std::collections::HashMap;

use crate::Value;

const MB: u64 = 1024 * 1024;
const GB: u64 = 1024 * MB;

/// What a computer has. `None`: not known here (a browser, a system that
/// doesn't say).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Facts {
    /// "macOS", "Windows", "Linux" (a browser's: what it says it runs on).
    pub os: String,
    /// "27.2", "11 24H2 (26100)", "Ubuntu 24.04".
    pub os_version: String,
    /// "arm64", "x86_64", "x86".
    pub arch: String,
    /// The processor's name ("Apple M4 Max").
    pub cpu: String,
    /// Logical cores (0: not known).
    pub cores: u32,
    /// Bytes of memory in all.
    pub total_memory: Option<u64>,
    /// Bytes a program can have now without the system swapping: free, and
    /// what the system gives back at once (caches, purgeable memory).
    pub free_memory: Option<u64>,
    /// The graphics processor's name ("" when not known).
    pub gpu: String,
    /// Bytes of memory the graphics processor can use: its own, or on a
    /// computer whose memory is shared (unified), the part it may take.
    pub gpu_memory: Option<u64>,
    /// The processor and the graphics share one memory (Apple silicon).
    pub unified_memory: bool,
    /// The disk of the user's home folder: its size and what is free.
    pub disk_total: Option<u64>,
    pub disk_free: Option<u64>,
    /// The computer has a battery.
    pub has_battery: Option<bool>,
    /// It runs on its battery now (unplugged).
    pub on_battery: Option<bool>,
    /// The battery's charge, 0–100.
    pub battery_level: Option<u32>,
    /// Facts a web page gave: some are never known there.
    pub browser: bool,
}

impl Facts {
    /// This computer's facts, read now (the battery's only when
    /// `battery`: a Mac asks `pmset`).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn read(battery: bool) -> Facts {
        let mut f = Facts { arch: arch_name(std::env::consts::ARCH).into(), cores: std::thread::available_parallelism().map_or(0, |n| n.get() as u32), ..Facts::default() };
        os::read(&mut f, battery);
        // (the disk the user's files are on)
        let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(std::path::PathBuf::from).unwrap_or_else(|| std::path::PathBuf::from(if cfg!(windows) { "C:\\" } else { "/" }));
        if let Some((total, free)) = os::disk(&home) {
            f.disk_total = Some(total);
            f.disk_free = Some(free);
        }
        f
    }

    /// Memory, in plain words: "small" (under 8 GB, or fewer than 4
    /// cores), "large" (32 GB or more and 8 cores or more), else "medium".
    pub fn class(&self) -> &'static str {
        let mem = self.total_memory;
        let cores = self.cores;
        let small = mem.is_some_and(|m| m < 8 * GB - 256 * MB) || (cores > 0 && cores < 4);
        let large = mem.is_some_and(|m| m >= 32 * GB - 256 * MB) && cores >= 8;
        if small {
            "small"
        } else if large {
            "large"
        } else {
            "medium"
        }
    }

    /// The properties this computer (or browser) can't tell, by name.
    pub fn unknown(&self) -> Vec<&'static str> {
        let mut out = Vec::new();
        let mut miss = |known: bool, name: &'static str| {
            if !known {
                out.push(name);
            }
        };
        miss(!self.os_version.is_empty(), "OSVersion");
        miss(!self.arch.is_empty(), "Architecture");
        miss(!self.cpu.is_empty(), "CPU");
        miss(self.cores > 0, "Cores");
        miss(self.total_memory.is_some(), "TotalMemory");
        miss(self.free_memory.is_some(), "FreeMemory");
        miss(!self.gpu.is_empty(), "GPU");
        miss(self.gpu_memory.is_some(), "GPUMemory");
        miss(self.disk_total.is_some(), "DiskTotal");
        miss(self.disk_free.is_some(), "DiskFree");
        miss(self.has_battery.is_some(), "HasBattery");
        miss(self.on_battery.is_some(), "OnBattery");
        miss(self.battery_level.is_some(), "BatteryLevel");
        out
    }

    /// One sentence for people: "macOS 27.2 on an Apple M4 Max (16 cores),
    /// 48 GB of memory (31 GB free), 410 GB free on the disk, plugged in."
    pub fn summary(&self) -> String {
        let mut s = String::new();
        s.push_str(if self.os.is_empty() { "A computer" } else { &self.os });
        if !self.os_version.is_empty() {
            s.push(' ');
            s.push_str(&self.os_version);
        }
        if !self.cpu.is_empty() || self.cores > 0 {
            s.push_str(" on ");
            if !self.cpu.is_empty() {
                s.push_str(&self.cpu);
                if self.cores > 0 {
                    s.push_str(&format!(" ({} cores)", self.cores));
                }
            } else {
                s.push_str(&format!("{} cores", self.cores));
            }
        }
        if let Some(t) = self.total_memory {
            s.push_str(&format!(", {} of memory", gb_words(t)));
            if let Some(f) = self.free_memory {
                s.push_str(&format!(" ({} free)", gb_words(f)));
            }
        }
        if let Some(d) = self.disk_free {
            s.push_str(&format!(", {} free on the disk", gb_words(d)));
        }
        match (self.has_battery, self.on_battery) {
            (Some(true), Some(true)) => s.push_str(&format!(", on battery{}", self.battery_level.map_or(String::new(), |l| format!(" ({l}%)")))),
            (Some(true), Some(false)) => s.push_str(", plugged in"),
            _ => {}
        }
        s.push('.');
        s
    }
}

/// Bytes as "48 GB" (whole gigabytes from 10, one decimal below: "7.5 GB",
/// "512 MB" under one).
pub fn gb_words(bytes: u64) -> String {
    if bytes < GB {
        return format!("{} MB", bytes / MB);
    }
    let g = bytes as f64 / GB as f64;
    if g >= 10.0 {
        format!("{:.0} GB", g)
    } else {
        let t = format!("{:.1}", g);
        format!("{} GB", t.trim_end_matches(".0"))
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn arch_name(a: &str) -> &str {
    match a {
        "aarch64" => "arm64",
        "x86" | "i686" | "i586" => "x86",
        other => other,
    }
}

#[cfg(target_os = "macos")]
mod os {
    use super::Facts;
    use std::ffi::CString;

    fn sysctl_string(name: &str) -> Option<String> {
        let c = CString::new(name).ok()?;
        let mut len: libc::size_t = 0;
        // SAFETY: a size query (null buffer) for a named sysctl.
        if unsafe { libc::sysctlbyname(c.as_ptr(), std::ptr::null_mut(), &mut len, std::ptr::null_mut(), 0) } != 0 || len == 0 {
            return None;
        }
        let mut buf = vec![0u8; len];
        // SAFETY: `buf` holds `len` bytes, as the query said.
        if unsafe { libc::sysctlbyname(c.as_ptr(), buf.as_mut_ptr().cast(), &mut len, std::ptr::null_mut(), 0) } != 0 {
            return None;
        }
        buf.truncate(len);
        while buf.last() == Some(&0) {
            buf.pop();
        }
        String::from_utf8(buf).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
    }

    fn sysctl_u64(name: &str) -> Option<u64> {
        let c = CString::new(name).ok()?;
        let mut v: u64 = 0;
        let mut len = std::mem::size_of::<u64>();
        // SAFETY: `v` is 8 bytes; a 4-byte answer fills its low half (little-endian).
        let r = unsafe { libc::sysctlbyname(c.as_ptr(), (&mut v as *mut u64).cast(), &mut len, std::ptr::null_mut(), 0) };
        (r == 0 && (len == 8 || len == 4)).then_some(if len == 4 { v & 0xffff_ffff } else { v })
    }

    /// Memory a program can have now: all of it but what apps use (their
    /// own pages less what is purgeable), what is wired and what is
    /// compressed — Activity Monitor's "Memory Used" taken from the total.
    fn available(total: u64) -> Option<u64> {
        let mut stats: libc::vm_statistics64 = unsafe { std::mem::zeroed() };
        let mut count = libc::HOST_VM_INFO64_COUNT;
        // SAFETY: `stats` is a vm_statistics64 and `count` its size in
        // integers, as host_statistics64 wants them.
        #[allow(deprecated)]
        let r = unsafe { libc::host_statistics64(libc::mach_host_self(), libc::HOST_VM_INFO64, (&mut stats as *mut libc::vm_statistics64).cast(), &mut count) };
        if r != 0 {
            return None;
        }
        // SAFETY: a plain read of the system's page size.
        #[allow(deprecated)]
        let page = unsafe { libc::vm_page_size } as u64;
        let apps = (stats.internal_page_count as u64).saturating_sub(stats.purgeable_count as u64);
        let used = (apps + stats.wire_count as u64 + stats.compressor_page_count as u64) * page;
        Some(total.saturating_sub(used))
    }

    pub fn read(f: &mut Facts, battery: bool) {
        f.os = "macOS".into();
        f.os_version = sysctl_string("kern.osproductversion").unwrap_or_default();
        f.cpu = sysctl_string("machdep.cpu.brand_string").unwrap_or_default();
        if let Some(n) = sysctl_u64("hw.logicalcpu") {
            f.cores = n as u32;
        }
        f.total_memory = sysctl_u64("hw.memsize");
        if let Some(t) = f.total_memory {
            f.free_memory = available(t);
        }
        // (Apple silicon: the graphics share the memory; a GPU may take
        // about three quarters of it on 36 GB and more, two thirds below —
        // Metal's recommended working set)
        if f.arch == "arm64" {
            f.unified_memory = true;
            f.gpu = if f.cpu.is_empty() { "Apple silicon".into() } else { f.cpu.clone() };
            f.gpu_memory = f.total_memory.map(|t| if t >= 36 * super::GB { t / 4 * 3 } else { t / 3 * 2 });
        }
        if battery {
            self::battery(f);
        }
    }

    fn battery(f: &mut Facts) {
        // (`pmset -g batt`: "Now drawing from 'Battery Power'" /
        // "-InternalBattery-0 (id=…)	87%; discharging; …")
        let Ok(out) = std::process::Command::new("/usr/bin/pmset").args(["-g", "batt"]).output() else { return };
        let text = String::from_utf8_lossy(&out.stdout);
        let has = text.contains("InternalBattery");
        f.has_battery = Some(has);
        if has {
            f.on_battery = Some(text.contains("'Battery Power'"));
            f.battery_level = text.split(['\t', ' ']).find_map(|w| w.strip_suffix("%;").and_then(|n| n.parse().ok()));
        } else {
            f.on_battery = Some(false);
        }
    }

    pub fn disk(path: &std::path::Path) -> Option<(u64, u64)> {
        super::unix_disk(path)
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
mod os {
    use super::Facts;

    fn meminfo(key: &str, text: &str) -> Option<u64> {
        text.lines().find_map(|l| l.strip_prefix(key)?.strip_prefix(':')?.split_whitespace().next()?.parse::<u64>().ok()).map(|kb| kb * 1024)
    }

    pub fn read(f: &mut Facts, battery: bool) {
        f.os = std::env::consts::OS.chars().next().map(|c| c.to_ascii_uppercase().to_string() + &std::env::consts::OS[1..]).unwrap_or_default();
        if let Ok(rel) = std::fs::read_to_string("/etc/os-release") {
            let get = |k: &str| rel.lines().find_map(|l| l.strip_prefix(k)?.strip_prefix('=').map(|v| v.trim_matches('"').to_string()));
            f.os_version = get("PRETTY_NAME").or_else(|| get("VERSION_ID")).unwrap_or_default();
        }
        if let Ok(cpu) = std::fs::read_to_string("/proc/cpuinfo") {
            f.cpu = cpu.lines().find_map(|l| {
                let (k, v) = l.split_once(':')?;
                matches!(k.trim(), "model name" | "Model" | "Hardware").then(|| v.trim().to_string())
            }).unwrap_or_default();
        }
        if let Ok(m) = std::fs::read_to_string("/proc/meminfo") {
            f.total_memory = meminfo("MemTotal", &m);
            f.free_memory = meminfo("MemAvailable", &m).or_else(|| meminfo("MemFree", &m));
        }
        // (a graphics card: the first DRM card's vendor, its own memory
        // when the driver says it)
        for k in 0..4 {
            let dev = format!("/sys/class/drm/card{k}/device");
            let Ok(vendor) = std::fs::read_to_string(format!("{dev}/vendor")) else { continue };
            f.gpu = match vendor.trim() {
                "0x10de" => "NVIDIA",
                "0x1002" => "AMD",
                "0x8086" => "Intel",
                "0x1af4" | "0x1234" | "0x15ad" => "a virtual display",
                _ => "a graphics card",
            }
            .into();
            f.gpu_memory = std::fs::read_to_string(format!("{dev}/mem_info_vram_total")).ok().and_then(|v| v.trim().parse().ok());
            break;
        }
        if battery {
            let mut has = false;
            let mut on = false;
            if let Ok(rd) = std::fs::read_dir("/sys/class/power_supply") {
                for e in rd.flatten() {
                    let p = e.path();
                    let kind = std::fs::read_to_string(p.join("type")).unwrap_or_default();
                    if kind.trim() == "Battery" {
                        has = true;
                        on = on || std::fs::read_to_string(p.join("status")).unwrap_or_default().trim() == "Discharging";
                        f.battery_level = std::fs::read_to_string(p.join("capacity")).ok().and_then(|c| c.trim().parse().ok());
                    }
                }
            }
            f.has_battery = Some(has);
            f.on_battery = Some(on);
        }
    }

    pub fn disk(path: &std::path::Path) -> Option<(u64, u64)> {
        super::unix_disk(path)
    }
}

#[cfg(unix)]
fn unix_disk(path: &std::path::Path) -> Option<(u64, u64)> {
    use std::os::unix::ffi::OsStrExt;
    let c = std::ffi::CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
    // SAFETY: a NUL-ended path and a statvfs to fill.
    if unsafe { libc::statvfs(c.as_ptr(), &mut st) } != 0 {
        return None;
    }
    let unit = st.f_frsize as u64;
    Some((st.f_blocks as u64 * unit, st.f_bavail as u64 * unit))
}

#[cfg(windows)]
mod os {
    use super::Facts;
    use windows_registry::LOCAL_MACHINE;

    pub fn read(f: &mut Facts, battery: bool) {
        use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
        f.os = "Windows".into();
        if let Ok(k) = LOCAL_MACHINE.open(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion") {
            let build = k.get_string("CurrentBuild").unwrap_or_default();
            let display = k.get_string("DisplayVersion").unwrap_or_default();
            // (Windows 11 still says "Windows 10" in ProductName: the build
            // tells them apart)
            let major = if build.parse::<u32>().unwrap_or(0) >= 22000 { "11" } else { "10" };
            f.os_version = [major.to_string(), display, if build.is_empty() { String::new() } else { format!("({build})") }].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" ");
        }
        if let Ok(k) = LOCAL_MACHINE.open(r"HARDWARE\DESCRIPTION\System\CentralProcessor\0") {
            f.cpu = k.get_string("ProcessorNameString").unwrap_or_default().trim().to_string();
        }
        let mut m: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
        m.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        // SAFETY: a MEMORYSTATUSEX with its length set, as the call wants.
        if unsafe { GlobalMemoryStatusEx(&mut m) } != 0 {
            f.total_memory = Some(m.ullTotalPhys);
            f.free_memory = Some(m.ullAvailPhys);
        }
        // (the display adapter's class: the first one's name and memory)
        if let Ok(k) = LOCAL_MACHINE.open(r"SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}\0000") {
            f.gpu = k.get_string("DriverDesc").unwrap_or_default();
            f.gpu_memory = k.get_u64("HardwareInformation.qwMemorySize").ok().filter(|m| *m > 0);
        }
        if battery {
            use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
            let mut p: SYSTEM_POWER_STATUS = unsafe { std::mem::zeroed() };
            // SAFETY: a SYSTEM_POWER_STATUS to fill.
            if unsafe { GetSystemPowerStatus(&mut p) } != 0 {
                let has = p.BatteryFlag != 128 && p.BatteryFlag != 255;
                f.has_battery = Some(has);
                f.on_battery = Some(has && p.ACLineStatus == 0);
                f.battery_level = (has && p.BatteryLifePercent <= 100).then_some(p.BatteryLifePercent as u32);
            }
        }
    }

    pub fn disk(path: &std::path::Path) -> Option<(u64, u64)> {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let (mut avail, mut total, mut free) = (0u64, 0u64, 0u64);
        // SAFETY: a NUL-ended wide path and three u64s to fill.
        let ok = unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut avail, &mut total, &mut free) };
        (ok != 0).then_some((total, avail))
    }
}

#[cfg(not(any(unix, windows, target_arch = "wasm32")))]
mod os {
    use super::Facts;
    pub fn read(_f: &mut Facts, _battery: bool) {}
    pub fn disk(_path: &std::path::Path) -> Option<(u64, u64)> {
        None
    }
}

/// The component: RSystemInfo's properties and Refresh (runtime-core and
/// the web runtime route here; the web gives its own facts).
pub mod component {
    use super::*;

    thread_local! {
        /// Each RSystemInfo's facts as last read (Refresh reads them again;
        /// FreeMemory always reads the memory now).
        static READ: RefCell<HashMap<String, Facts>> = RefCell::new(HashMap::new());
    }

    /// Whether `type_name` is RSystemInfo.
    pub fn is_sysinfo(type_name: &str) -> bool {
        type_name.eq_ignore_ascii_case("RSYSTEMINFO")
    }

    fn flag(b: bool) -> Value {
        Value::Integer(if b { -1 } else { 0 })
    }

    fn mb(v: Option<u64>) -> Value {
        Value::Integer(v.map_or(0, |b| (b / MB) as i64))
    }

    /// RSystemInfo `name`'s facts: `fresh` gives them when none are kept
    /// (the desktop reads this computer's, a page the browser's).
    fn facts(name: &str, fresh: &dyn Fn(bool) -> Facts, battery: bool) -> Facts {
        let key = name.to_ascii_lowercase();
        READ.with(|r| {
            let mut r = r.borrow_mut();
            let f = r.entry(key).or_insert_with(|| fresh(battery));
            // (the battery read once, when first asked about)
            if battery && f.has_battery.is_none() && !f.browser {
                let more = fresh(true);
                f.has_battery = more.has_battery;
                f.on_battery = more.on_battery;
                f.battery_level = more.battery_level;
            }
            f.clone()
        })
    }

    /// A property of RSystemInfo `name` (`None`: not one of its own).
    pub fn get_with(name: &str, prop: &str, fresh: &dyn Fn(bool) -> Facts) -> Option<Value> {
        let battery = matches!(prop, "hasbattery" | "onbattery" | "batterylevel" | "summary" | "unknown");
        let f = facts(name, fresh, battery);
        Some(match prop {
            "os" => Value::String(f.os.clone().into()),
            "osversion" => Value::String(f.os_version.clone().into()),
            "architecture" => Value::String(f.arch.clone().into()),
            "cpu" => Value::String(f.cpu.clone().into()),
            "cores" => Value::Integer(f.cores as i64),
            "totalmemory" => mb(f.total_memory),
            // (the memory free changes all the time: read now)
            "freememory" => mb(if f.browser { f.free_memory } else { fresh(false).free_memory }),
            "gpu" => Value::String(f.gpu.clone().into()),
            "gpumemory" => mb(f.gpu_memory),
            "unifiedmemory" => flag(f.unified_memory),
            "disktotal" => mb(f.disk_total),
            "diskfree" => mb(f.disk_free),
            "hasbattery" => flag(f.has_battery == Some(true)),
            "onbattery" => flag(f.on_battery == Some(true)),
            "batterylevel" => Value::Integer(f.battery_level.map_or(-1, i64::from)),
            "class" => Value::String(f.class().into()),
            "summary" => Value::String(f.summary().into()),
            "unknown" => Value::String(f.unknown().join(", ").into()),
            "browser" => flag(f.browser),
            _ => return None,
        })
    }

    /// A property of RSystemInfo `name` on the desktop.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn get(name: &str, prop: &str) -> Option<Value> {
        get_with(name, prop, &Facts::read)
    }

    /// A method (`None`: not one of its own): Refresh reads everything
    /// again.
    pub fn call(name: &str, method: &str, _args: &[Value]) -> Option<Value> {
        match method {
            "refresh" => {
                READ.with(|r| r.borrow_mut().remove(&name.to_ascii_lowercase()));
                Some(Value::Integer(-1))
            }
            _ => None,
        }
    }

    /// Its properties, all read-only: a program setting one is told so.
    pub fn is_property(prop: &str) -> bool {
        matches!(prop, "os" | "osversion" | "architecture" | "cpu" | "cores" | "totalmemory" | "freememory" | "gpu" | "gpumemory" | "unifiedmemory" | "disktotal" | "diskfree" | "hasbattery" | "onbattery" | "batterylevel" | "class" | "summary" | "unknown" | "browser")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_is_said_in_plain_words() {
        let f = |mem_gb: u64, cores: u32| Facts { total_memory: Some(mem_gb * GB), cores, ..Facts::default() };
        assert_eq!(f(4, 4).class(), "small");
        assert_eq!(f(16, 2).class(), "small");
        assert_eq!(f(8, 4).class(), "medium");
        assert_eq!(f(16, 8).class(), "medium");
        assert_eq!(f(32, 8).class(), "large");
        assert_eq!(f(48, 16).class(), "large");
        assert_eq!(f(64, 4).class(), "medium");
    }

    #[test]
    fn sizes_are_said_as_people_say_them() {
        assert_eq!(gb_words(48 * GB), "48 GB");
        assert_eq!(gb_words(7 * GB + GB / 2), "7.5 GB");
        assert_eq!(gb_words(8 * GB), "8 GB");
        assert_eq!(gb_words(512 * MB), "512 MB");
    }

    #[test]
    fn a_browser_says_what_it_cant_know() {
        let f = Facts { os: "macOS".into(), cores: 8, total_memory: Some(8 * GB), browser: true, ..Facts::default() };
        let u = f.unknown();
        assert!(u.contains(&"FreeMemory") && u.contains(&"GPU") && u.contains(&"DiskFree") && u.contains(&"OnBattery"));
        assert!(!u.contains(&"Cores") && !u.contains(&"TotalMemory"));
        assert_eq!(f.summary(), "macOS on 8 cores, 8 GB of memory.");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn this_computer_says_its_memory_and_cores() {
        let f = Facts::read(true);
        assert!(f.cores > 0);
        let total = f.total_memory.expect("total memory");
        let free = f.free_memory.expect("free memory");
        assert!(total > GB && free <= total, "{f:?}");
        assert!(!f.os.is_empty() && !f.arch.is_empty());
        assert!(f.summary().ends_with('.'));
    }
}

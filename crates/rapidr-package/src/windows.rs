//! Windows: the program's `.exe` carries its icon (an icon group of every
//! size Explorer shows, as `MAINICON`) and its version information
//! (product name, version, company, description) as resources — written
//! into the executable after it is linked, in pure Rust (editpe), so a Mac
//! or Linux machine building for Windows writes them too.

use editpe::{Image, ResourceDirectory, VersionInfo, VersionStringTable};
use editpe::types::{FixedFileInfo, VersionU16, VersionU32};

use crate::info::AppInfo;
use crate::Icon;

/// PE machine types (`IMAGE_FILE_MACHINE_*`).
pub const MACHINE_X64: u16 = 0x8664;
pub const MACHINE_ARM64: u16 = 0xAA64;
pub const MACHINE_X86: u16 = 0x014C;

/// The machine's name for messages.
pub fn machine_name(machine: u16) -> &'static str {
    match machine {
        MACHINE_X64 => "x64",
        MACHINE_ARM64 => "ARM64",
        MACHINE_X86 => "32-bit x86",
        _ => "another processor",
    }
}

/// Where the PE header is (`PE\0\0`), from the DOS header's `e_lfanew`.
fn pe_header(exe: &[u8]) -> Result<usize, String> {
    let not = || "not a Windows executable (PE)".to_string();
    if exe.get(0..2) != Some(b"MZ") {
        return Err(not());
    }
    let at = u32::from_le_bytes(exe.get(0x3C..0x40).ok_or_else(not)?.try_into().map_err(|_| not())?) as usize;
    if exe.get(at..at + 4) != Some(b"PE\0\0") {
        return Err(not());
    }
    Ok(at)
}

/// The processor a Windows executable is for (its COFF header's Machine).
pub fn machine(exe: &[u8]) -> Result<u16, String> {
    let at = pe_header(exe)?;
    exe.get(at + 4..at + 6).map(|b| u16::from_le_bytes([b[0], b[1]])).ok_or_else(|| "not a Windows executable (PE)".into())
}

/// The subsystem Windows starts an executable in: a GUI program opens no
/// console window, a console one gets one. RapidR's windowed runner
/// (`rapidrintr-runnerw.exe`) is its console runner with this one field
/// changed (the optional header's `Subsystem`: 2 GUI, 3 console — at the
/// same place in PE32 and PE32+), so one runner makes both.
pub fn set_subsystem(exe: &mut [u8], gui: bool) -> Result<(), String> {
    let at = pe_header(exe)? + 24 + 68;
    let field = exe.get_mut(at..at + 2).ok_or("a damaged Windows executable (its optional header is cut short)")?;
    let now = u16::from_le_bytes([field[0], field[1]]);
    if now != 2 && now != 3 {
        return Err(format!("the executable's subsystem is {now}, not a Windows console or GUI program"));
    }
    field.copy_from_slice(&(if gui { 2u16 } else { 3u16 }).to_le_bytes());
    Ok(())
}

/// The executable's subsystem: true for GUI (2), false for console (3).
pub fn is_gui(exe: &[u8]) -> Result<bool, String> {
    let at = pe_header(exe)? + 24 + 68;
    let f = exe.get(at..at + 2).ok_or("a damaged Windows executable")?;
    Ok(u16::from_le_bytes([f[0], f[1]]) == 2)
}

/// `exe` (a Windows executable) with `icon` and `info` as its resources,
/// and [`LINK_MANIFEST`] when it has no manifest (a runner has none; a
/// native program is linked with it).
pub fn stamp(exe: &[u8], info: &AppInfo, icon: &Icon) -> Result<Vec<u8>, String> {
    info.check()?;
    let v = info.version_numbers()?;
    let mut image = Image::parse(exe).map_err(|e| format!("not a Windows executable RapidR can add resources to ({e})"))?;
    let mut resources: ResourceDirectory = image.resource_directory().cloned().unwrap_or_default();
    resources.remove_main_icon().map_err(|e| format!("the executable's icon: {e}"))?;
    resources.set_main_icon(icon.ico()).map_err(|e| format!("the icon: {e}"))?;
    if resources.get_manifest().map_err(|e| format!("the executable's manifest: {e}"))?.is_none() {
        resources.set_manifest(LINK_MANIFEST).map_err(|e| format!("the manifest: {e}"))?;
    }

    let version_text = format!("{}.{}.{}.{}", v[0], v[1], v[2], v[3]);
    let mut strings = Vec::new();
    let mut put = |k: &str, val: &str| {
        if !val.is_empty() {
            strings.push((k.to_string(), val.to_string()));
        }
    };
    put("Comments", info.website.trim());
    put("CompanyName", info.company.trim());
    put("FileDescription", if info.description.trim().is_empty() { info.name.trim() } else { info.description.trim() });
    put("FileVersion", &version_text);
    put("InternalName", &info.exe);
    put("LegalCopyright", &info.copyright_line());
    put("OriginalFilename", &format!("{}.exe", info.exe));
    put("ProductName", info.name.trim());
    put("ProductVersion", info.version.trim());
    let fixed = FixedFileInfo {
        file_version: VersionU32 { major: u32::from(v[0]) << 16 | u32::from(v[1]), minor: u32::from(v[2]) << 16 | u32::from(v[3]) },
        product_version: VersionU32 { major: u32::from(v[0]) << 16 | u32::from(v[1]), minor: u32::from(v[2]) << 16 | u32::from(v[3]) },
        // VOS_NT_WINDOWS32, VFT_APP
        file_os: 0x0004_0004,
        file_type: 1,
        ..FixedFileInfo::default()
    };
    let version = VersionInfo {
        info: fixed,
        // U.S. English, Unicode
        strings: vec![VersionStringTable { key: "040904B0".into(), strings: strings.into_iter().collect() }],
        vars: vec![VersionU16 { major: 0x0409, minor: 1200 }],
    };
    resources.set_version_info(&version).map_err(|e| format!("the version information: {e}"))?;
    image.set_resource_directory(resources).map_err(|e| format!("writing the resources: {e}"))?;
    Ok(image.data().to_vec())
}

/// The application manifest a native program is linked with: it runs as the
/// person who starts it (`asInvoker`) — never taken for an installer that
/// wants administrator rights because of a name like "setup" or "update" —
/// and says nothing else: no Windows version list, no DPI setting, no code
/// page, so the program behaves exactly as one without a manifest.
pub const LINK_MANIFEST: &str = concat!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n",
    "<assembly xmlns=\"urn:schemas-microsoft-com:asm.v1\" manifestVersion=\"1.0\">\n",
    "  <trustInfo xmlns=\"urn:schemas-microsoft-com:asm.v3\">\n",
    "    <security><requestedPrivileges><requestedExecutionLevel level=\"asInvoker\" uiAccess=\"false\"/></requestedPrivileges></security>\n",
    "  </trustInfo>\n",
    "</assembly>\n",
);

/// The resource script (`.rc`) a native program is *linked* with: its
/// version information and [`LINK_MANIFEST`] (the file `manifest_file`,
/// beside the script). A program with no resources at all is a thing an
/// antivirus' guesswork dislikes: the 2.121.0 release test on Windows x64
/// saw one refuse every native test program (a tiny program with no
/// resources, stripped), and accept the same program once it carried any
/// resource. [`stamp`] writes the real details (and the icon) into the
/// linked executable afterwards; these are the same details, in plain ASCII
/// (anything else is a `?` for the moment).
pub fn link_script(info: &AppInfo, manifest_file: &str) -> Result<String, String> {
    info.check()?;
    let v = info.version_numbers()?;
    let ascii = |s: &str| -> String {
        s.trim()
            .chars()
            .map(|c| if c.is_ascii_graphic() || c == ' ' { c } else { '?' })
            .collect::<String>()
            .replace('\\', "\\\\")
            .replace('"', "\"\"")
    };
    let version = format!("{}.{}.{}.{}", v[0], v[1], v[2], v[3]);
    let description = if info.description.trim().is_empty() { &info.name } else { &info.description };
    let mut s = String::new();
    s.push_str(&format!("1 24 \"{}\"\n", ascii(manifest_file)));
    s.push_str("1 VERSIONINFO\n");
    s.push_str(&format!("FILEVERSION {},{},{},{}\nPRODUCTVERSION {},{},{},{}\n", v[0], v[1], v[2], v[3], v[0], v[1], v[2], v[3]));
    // (VOS_NT_WINDOWS32, VFT_APP: as stamp() has them)
    s.push_str("FILEOS 0x40004\nFILETYPE 0x1\n{\n  BLOCK \"StringFileInfo\"\n  {\n    BLOCK \"040904B0\"\n    {\n");
    let mut put = |k: &str, val: &str| {
        if !val.trim().is_empty() {
            s.push_str(&format!("      VALUE \"{k}\", \"{}\"\n", ascii(val)));
        }
    };
    put("CompanyName", &info.company);
    put("FileDescription", description);
    put("FileVersion", &version);
    put("InternalName", &info.exe);
    put("OriginalFilename", &format!("{}.exe", info.exe));
    put("ProductName", &info.name);
    put("ProductVersion", &info.version);
    s.push_str("    }\n  }\n  BLOCK \"VarFileInfo\"\n  {\n    VALUE \"Translation\", 0x409, 1200\n  }\n}\n");
    Ok(s)
}

/// The executable's main icon (its first picture's bytes) and version, as a
/// check: (has an icon, ProductName, FileVersion).
pub fn inspect(exe: &[u8]) -> Result<(bool, String, String), String> {
    let image = Image::parse(exe).map_err(|e| e.to_string())?;
    let Some(res) = image.resource_directory() else { return Ok((false, String::new(), String::new())) };
    let icon = res.get_main_icon().map_err(|e| e.to_string())?.is_some();
    let (mut product, mut version) = (String::new(), String::new());
    if let Some(v) = res.get_version_info().map_err(|e| e.to_string())? {
        for t in &v.strings {
            product = t.strings.get("ProductName").cloned().unwrap_or_default();
            version = t.strings.get("FileVersion").cloned().unwrap_or_default();
        }
    }
    Ok((icon, product, version))
}

/// What a Windows executable says about itself, as Explorer's Properties
/// shows it: its version strings (ProductName, FileVersion, CompanyName …),
/// whether it has an icon, its manifest, its subsystem and machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Details {
    pub strings: std::collections::BTreeMap<String, String>,
    pub icon: bool,
    pub manifest: Option<String>,
    pub gui: bool,
    pub machine: u16,
}

/// The [`Details`] of `exe`.
pub fn details(exe: &[u8]) -> Result<Details, String> {
    let image = Image::parse(exe).map_err(|e| e.to_string())?;
    let mut d = Details { strings: Default::default(), icon: false, manifest: None, gui: is_gui(exe)?, machine: machine(exe)? };
    if let Some(res) = image.resource_directory() {
        d.icon = res.get_main_icon().map_err(|e| e.to_string())?.is_some();
        d.manifest = res.get_manifest().map_err(|e| e.to_string())?;
        if let Some(v) = res.get_version_info().map_err(|e| e.to_string())? {
            for t in &v.strings {
                d.strings.extend(t.strings.iter().map(|(k, v)| (k.clone(), v.clone())));
            }
        }
    }
    Ok(d)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// The smallest PE32+ executable editpe reads: headers and one code
    /// section (a `ret`), no resources; a console program for x64.
    pub(crate) fn tiny_exe() -> Vec<u8> {
        let mut b = vec![0u8; 0x400];
        b[0..2].copy_from_slice(b"MZ");
        b[0x3C..0x40].copy_from_slice(&0x40u32.to_le_bytes());
        b[0x40..0x44].copy_from_slice(b"PE\0\0");
        let coff = 0x44;
        let put16 = |b: &mut Vec<u8>, at: usize, v: u16| b[at..at + 2].copy_from_slice(&v.to_le_bytes());
        let put32 = |b: &mut Vec<u8>, at: usize, v: u32| b[at..at + 4].copy_from_slice(&v.to_le_bytes());
        let put64 = |b: &mut Vec<u8>, at: usize, v: u64| b[at..at + 8].copy_from_slice(&v.to_le_bytes());
        put16(&mut b, coff, 0x8664); // x86-64
        put16(&mut b, coff + 2, 1); // one section
        put16(&mut b, coff + 16, 0xF0); // optional header size
        put16(&mut b, coff + 18, 0x22); // executable, large-address aware
        let opt = coff + 20;
        put16(&mut b, opt, 0x20B); // PE32+
        put32(&mut b, opt + 4, 0x200); // size of code
        put32(&mut b, opt + 16, 0x1000); // entry point
        put32(&mut b, opt + 20, 0x1000); // base of code
        put64(&mut b, opt + 24, 0x1_4000_0000); // image base
        put32(&mut b, opt + 32, 0x1000); // section alignment
        put32(&mut b, opt + 36, 0x200); // file alignment
        put16(&mut b, opt + 40, 6); // OS version
        put16(&mut b, opt + 48, 6); // subsystem version
        put32(&mut b, opt + 56, 0x2000); // size of image
        put32(&mut b, opt + 60, 0x200); // size of headers
        put16(&mut b, opt + 68, 3); // console
        put64(&mut b, opt + 72, 0x10_0000);
        put64(&mut b, opt + 80, 0x1000);
        put64(&mut b, opt + 88, 0x10_0000);
        put64(&mut b, opt + 96, 0x1000);
        put32(&mut b, opt + 108, 16); // data directories
        let sec = opt + 0xF0;
        b[sec..sec + 8].copy_from_slice(b".text\0\0\0");
        put32(&mut b, sec + 8, 1); // virtual size
        put32(&mut b, sec + 12, 0x1000);
        put32(&mut b, sec + 16, 0x200);
        put32(&mut b, sec + 20, 0x200);
        put32(&mut b, sec + 36, 0x6000_0020); // code, execute, read
        b[0x200] = 0xC3;
        b
    }

    #[test]
    fn stamps_icon_and_version() {
        let exe = tiny_exe();
        assert_eq!(inspect(&exe).unwrap(), (false, String::new(), String::new()));
        let mut info = AppInfo::new("Notepad", "notepad");
        info.version = "2.3.1".into();
        info.company = "Ruta Internet SRL".into();
        let stamped = stamp(&exe, &info, &Icon::rapidr_default()).unwrap();
        assert_eq!(inspect(&stamped).unwrap(), (true, "Notepad".into(), "2.3.1.0".into()));
        let image = Image::parse(&stamped[..]).unwrap();
        let icon = image.resource_directory().unwrap().get_main_icon().unwrap().unwrap();
        assert!(icon.starts_with(b"\x89PNG"), "the first picture is a PNG");
        // stamped again (a runner reused): the new details
        info.name = "Other".into();
        let again = stamp(&stamped, &info, &Icon::rapidr_default()).unwrap();
        assert_eq!(inspect(&again).unwrap().1, "Other");
        assert!(stamp(b"MZ not really", &info, &Icon::rapidr_default()).is_err());
    }

    /// The windowed runner is the console runner with its subsystem set to
    /// GUI: one field, nothing else changes.
    #[test]
    fn the_subsystem_is_one_field() {
        let exe = tiny_exe();
        assert_eq!(machine(&exe).unwrap(), MACHINE_X64);
        assert!(!is_gui(&exe).unwrap());
        let mut w = exe.clone();
        set_subsystem(&mut w, true).unwrap();
        assert!(is_gui(&w).unwrap());
        let changed: Vec<usize> = exe.iter().zip(&w).enumerate().filter(|(_, (a, b))| a != b).map(|(i, _)| i).collect();
        assert_eq!(changed, [0x44 + 20 + 68], "only the subsystem's low byte");
        set_subsystem(&mut w, false).unwrap();
        assert_eq!(w, exe);
        // (not a PE; a PE whose subsystem is neither: refused)
        assert!(set_subsystem(&mut b"MZ".to_vec(), true).is_err());
        assert!(machine(b"\x7fELF").unwrap_err().contains("not a Windows executable"));
        let mut driver = exe.clone();
        driver[0x44 + 20 + 68] = 1;
        assert!(set_subsystem(&mut driver, true).unwrap_err().contains("subsystem is 1"));
    }

    /// A stamped runner says everything Properties > Details shows, and
    /// runs as the person who starts it (the manifest).
    #[test]
    fn a_stamped_runner_says_who_it_is() {
        let mut info = AppInfo::new("Tip Calculator", "Tip-Calculator");
        info.version = "1.2.3".into();
        info.company = "Example Ltd".into();
        info.description = "Splits the bill".into();
        info.website = "https://example.com/tips".into();
        let d = details(&stamp(&tiny_exe(), &info, &Icon::rapidr_default()).unwrap()).unwrap();
        let s = |k: &str| d.strings.get(k).cloned().unwrap_or_default();
        assert_eq!(
            [s("ProductName"), s("FileVersion"), s("ProductVersion"), s("CompanyName"), s("FileDescription"), s("LegalCopyright"), s("OriginalFilename"), s("Comments")],
            ["Tip Calculator", "1.2.3.0", "1.2.3", "Example Ltd", "Splits the bill", "© Example Ltd", "Tip-Calculator.exe", "https://example.com/tips"]
        );
        assert!(d.icon && !d.gui && d.machine == MACHINE_X64);
        assert_eq!(d.manifest.as_deref(), Some(LINK_MANIFEST));
    }

    #[test]
    fn the_link_script_carries_the_version_and_the_manifest() {
        let mut info = AppInfo::new("Notepad", "notepad");
        info.version = "2.3.1".into();
        info.company = "Ruta Internet SRL".into();
        info.description = "A small editor".into();
        let rc = link_script(&info, "notepad.manifest").unwrap();
        assert!(rc.starts_with("1 24 \"notepad.manifest\"\n"), "{rc}");
        for want in [
            "FILEVERSION 2,3,1,0",
            "PRODUCTVERSION 2,3,1,0",
            "VALUE \"CompanyName\", \"Ruta Internet SRL\"",
            "VALUE \"FileDescription\", \"A small editor\"",
            "VALUE \"FileVersion\", \"2.3.1.0\"",
            "VALUE \"InternalName\", \"notepad\"",
            "VALUE \"OriginalFilename\", \"notepad.exe\"",
            "VALUE \"ProductName\", \"Notepad\"",
            "VALUE \"ProductVersion\", \"2.3.1\"",
            "VALUE \"Translation\", 0x409, 1200",
        ] {
            assert!(rc.contains(want), "{want} in\n{rc}");
        }
        // (no company, no description: no empty values; the description is the name)
        let plain = link_script(&AppInfo::new("Notepad", "notepad"), "m.manifest").unwrap();
        assert!(!plain.contains("CompanyName"), "{plain}");
        assert!(plain.contains("VALUE \"FileDescription\", \"Notepad\""), "{plain}");
    }

    #[test]
    fn the_link_script_is_safe_for_any_name() {
        // (quotes and backslashes are doubled; anything not plain ASCII is a ? for now)
        let mut info = AppInfo::new("Say \"hi\" é", "say");
        info.description = "C:\\temp\nnext line".into();
        let rc = link_script(&info, "m.manifest").unwrap();
        assert!(rc.contains("VALUE \"ProductName\", \"Say \"\"hi\"\" ?\""), "{rc}");
        assert!(rc.contains("VALUE \"FileDescription\", \"C:\\\\temp?next line\""), "{rc}");
        assert!(!rc.contains('é'));
        // (a version or name the app can't have: an error, no script)
        let mut bad = AppInfo::new("X", "x");
        bad.version = "one".into();
        assert!(link_script(&bad, "m.manifest").is_err());
        assert!(link_script(&AppInfo::new("", "x"), "m.manifest").is_err());
    }

    /// The manifest changes nothing about how the program runs: it only
    /// says who it runs as. (No Windows version list: GetVersionEx answers
    /// as for a program with no manifest; no DPI setting; no UTF-8 code page.)
    #[test]
    fn the_link_manifest_says_only_who_it_runs_as() {
        assert!(LINK_MANIFEST.contains("requestedExecutionLevel level=\"asInvoker\""));
        for not_here in ["supportedOS", "dpiAware", "activeCodePage", "longPathAware", "requireAdministrator", "highestAvailable"] {
            assert!(!LINK_MANIFEST.contains(not_here), "{not_here}");
        }
        assert!(LINK_MANIFEST.starts_with("<?xml") && LINK_MANIFEST.trim_end().ends_with("</assembly>"));
    }

    /// What is linked in survives the stamp: the manifest stays, the details
    /// and the icon are the stamp's.
    #[test]
    fn stamping_keeps_the_manifest_the_program_was_linked_with() {
        let exe = tiny_exe();
        let mut image = Image::parse(&exe[..]).unwrap();
        let mut res = ResourceDirectory::default();
        res.set_manifest(LINK_MANIFEST).unwrap();
        image.set_resource_directory(res).unwrap();
        let linked = image.data().to_vec();
        let mut info = AppInfo::new("Notepad", "notepad");
        info.version = "2.3.1".into();
        let stamped = stamp(&linked, &info, &Icon::rapidr_default()).unwrap();
        assert_eq!(inspect(&stamped).unwrap(), (true, "Notepad".into(), "2.3.1.0".into()));
        let image = Image::parse(&stamped[..]).unwrap();
        assert_eq!(image.resource_directory().unwrap().get_manifest().unwrap().as_deref(), Some(LINK_MANIFEST));
    }
}

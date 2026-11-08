//! The packer: a program and a runner made into an app for a target, then
//! into the archive that is shipped (docs/build-core-design.md). Bytes in,
//! bytes out: no file system, no processes, no clock.

use crate::files::Files;
use crate::info::AppInfo;
use crate::macos::{self, Signature};
use crate::{linux, macho, readme, web, windows, Icon, System, Target};

/// The runner an app starts from, as `runner_bytes(target)` gives it, and
/// the open-source notices that go with it.
pub struct Runner<'a> {
    /// Windows: the console runner (`.exe`; the windowed one is made from
    /// it); macOS: a universal (or thin) Mach-O; Linux: an ELF; the web:
    /// the web runtime's tar ([`web::runtime_tar`]). For a native build,
    /// the program's own executable.
    pub bytes: &'a [u8],
    /// Its `THIRD-PARTY-NOTICES.txt`.
    pub notices: &'a str,
}

/// What the app is made of.
pub struct App<'a> {
    pub info: &'a AppInfo,
    pub icon: &'a Icon,
    /// The program's bytecode (`.rrbc`); `None` for a native build (the
    /// runner is the program).
    pub bytecode: Option<&'a [u8]>,
    /// The program's data files, by their path in the app (`staff.csv`,
    /// `art/logo.svg`, translations as `lang/es.po`).
    pub data: &'a [(String, Vec<u8>)],
    /// An app (macOS' `.app`, Linux' AppDir; Windows' icon and version)
    /// rather than a plain executable; `None`: a windowed program is an
    /// app, a console one a plain executable (on Windows always an app).
    pub bundle: Option<bool>,
    /// What the web page may load (its Content-Security-Policy; the web
    /// target only); `None`: the default policy.
    pub web_needs: Option<&'a rapidr_webbundle::WebNeeds>,
    /// The program's source, only when the user asked for it in the
    /// archive (`source/…`): by path.
    pub source: &'a [(String, Vec<u8>)],
    /// The archive entries' time: seconds since 1970, UTC.
    pub mtime: i64,
    /// The README's language (`en`, `es`).
    pub language: &'a str,
}

impl<'a> App<'a> {
    /// An interpreted app of `bytecode`, nothing else said.
    pub fn new(info: &'a AppInfo, icon: &'a Icon, bytecode: &'a [u8]) -> App<'a> {
        App { info, icon, bytecode: Some(bytecode), data: &[], bundle: None, web_needs: None, source: &[], mtime: 0, language: "en" }
    }
}

/// An app made for a target: its files as a folder build writes them.
#[derive(Debug, Clone)]
pub struct Packed {
    pub target: Target,
    pub files: Files,
    /// What was made, at the top: `Name.app`, `Name.exe`, `Name.AppDir`,
    /// `index.html`, `name`.
    pub main: String,
    /// The file that runs (`Name.app/Contents/MacOS/name` …); `None` for the web.
    pub executable: Option<String>,
    /// An app (bundle) rather than a plain executable.
    pub bundle: bool,
    /// A macOS app's signature.
    pub signature: Option<Signature>,
    /// Things worth saying (data left out, an icon too small, a runner
    /// for one processor only).
    pub notes: Vec<String>,
}

/// An archive: its file name and bytes.
#[derive(Debug, Clone)]
pub struct Archive {
    pub name: String,
    pub bytes: Vec<u8>,
}

/// The notices' file name.
pub const NOTICES_FILE: &str = "THIRD-PARTY-NOTICES.txt";
/// The first-launch help's file name in an archive.
pub const README_FILE: &str = "README.txt";

/// Appends a program's bytecode to a runner: `[rrbc][RRBCEXE1][u32 length]`
/// (the runner reads it from its own end).
pub fn with_payload(stub: &[u8], rrbc: &[u8]) -> Result<Vec<u8>, String> {
    let len = u32::try_from(rrbc.len()).map_err(|_| "bytecode payload exceeds 4 GiB".to_string())?;
    let mut out = Vec::with_capacity(stub.len() + rrbc.len() + 12);
    out.extend_from_slice(stub);
    out.extend_from_slice(rrbc);
    out.extend_from_slice(b"RRBCEXE1");
    out.extend_from_slice(&len.to_le_bytes());
    Ok(out)
}

/// Extensions of program source: never packed as data (only `source`).
const SOURCE_EXTENSIONS: [&str; 7] = ["rr", "bas", "inc", "rqb", "rqw", "rrproj", "rrinc"];

fn is_source(path: &str) -> bool {
    path.rsplit_once('.').is_some_and(|(_, ext)| SOURCE_EXTENSIONS.iter().any(|s| s.eq_ignore_ascii_case(ext)))
}

/// The data files that may go in, checked (a safe path; never source), and
/// notes for those left out.
fn checked_data(data: &[(String, Vec<u8>)], notes: &mut Vec<String>) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut out = Vec::new();
    let mut left = Vec::new();
    for (path, bytes) in data {
        let path = crate::files::safe(path)?;
        if is_source(&path) {
            left.push(path);
            continue;
        }
        out.push((path, bytes.clone()));
    }
    if !left.is_empty() {
        notes.push(format!("left out (program source goes in only when you ask for it): {}", left.join(", ")));
    }
    Ok(out)
}

/// Adds `data` below `folder` in `files`, refusing a name the app's own
/// files have.
fn add_data(files: &mut Files, folder: &str, data: &[(String, Vec<u8>)]) -> Result<(), String> {
    for (path, bytes) in data {
        let at = if folder.is_empty() { path.clone() } else { format!("{folder}/{path}") };
        if files.contains(&at) {
            return Err(format!("the data file \"{path}\" has the name of one of the app's own files: rename it"));
        }
        files.add(&at, bytes.clone())?;
    }
    Ok(())
}

/// The machine a target's Windows runner must be for.
fn windows_machine(target: Target) -> u16 {
    match target {
        Target::WindowsArm64 => windows::MACHINE_ARM64,
        Target::WindowsX86 => windows::MACHINE_X86,
        _ => windows::MACHINE_X64,
    }
}

/// An ELF's machine (`e_machine`), checked for a Linux target.
fn check_elf(bytes: &[u8], target: Target) -> Result<(), String> {
    if bytes.get(0..4) != Some(b"\x7fELF") {
        return Err(format!("the runner for {} is not a Linux executable (ELF)", target.label()));
    }
    // (EI_DATA 1: little-endian, as x86-64's and arm64's are)
    let machine = bytes.get(18..20).map(|b| u16::from_le_bytes([b[0], b[1]])).unwrap_or(0);
    let (want, name) = match target {
        Target::LinuxArm64 => (183u16, "ARM64"),
        _ => (62u16, "x64"),
    };
    if machine != want {
        let is = match machine {
            62 => "x64",
            183 => "ARM64",
            3 => "32-bit x86",
            40 => "32-bit ARM",
            _ => "another processor",
        };
        return Err(format!("the runner for {} is for {is}, not {name}: it was not used", target.label()));
    }
    Ok(())
}

/// Makes the app for `target`: its files, as a folder build writes them
/// (an archive adds its README: [`archive`]).
pub fn pack(target: Target, app: &App, runner: &Runner) -> Result<Packed, String> {
    let info = app.info;
    info.check()?;
    if runner.notices.trim().is_empty() {
        return Err(format!("the runner for {} has no {NOTICES_FILE}: an app isn't shipped without its notices", target.label()));
    }
    let mut notes = app.icon.notes();
    let data = checked_data(app.data, &mut notes)?;
    let console = info.console;
    let system = target.system();
    let bundle = app.bundle.unwrap_or(match system {
        Some(System::Windows) | None => true,
        Some(_) => !console,
    });
    let notices = runner.notices.as_bytes().to_vec();
    let payload = |exe: &[u8]| -> Result<Vec<u8>, String> {
        match app.bytecode {
            Some(code) => with_payload(exe, code),
            None => Ok(exe.to_vec()),
        }
    };
    let mut files = Files::new();
    let (main, executable, signature) = match system {
        Some(System::Windows) => {
            let want = windows_machine(target);
            let is = windows::machine(runner.bytes).map_err(|e| format!("the runner for {}: {e}", target.label()))?;
            if is != want {
                return Err(format!("the runner for {} is for {}, not {}: it was not used", target.label(), windows::machine_name(is), windows::machine_name(want)));
            }
            let mut exe = runner.bytes.to_vec();
            if app.bytecode.is_some() {
                // (one runner: a windowed program's opens no console window)
                windows::set_subsystem(&mut exe, !console)?;
            }
            if bundle {
                exe = windows::stamp(&exe, info, app.icon)?;
            }
            let name = format!("{}.exe", info.exe);
            files.add_executable(&name, payload(&exe)?)?;
            files.add(NOTICES_FILE, notices)?;
            add_data(&mut files, "", &data)?;
            (name.clone(), Some(name), None)
        }
        Some(System::MacOs) => {
            let cpus = macho::cpus(runner.bytes).map_err(|e| format!("the runner for {}: {e}", target.label()))?;
            let exe = match target {
                Target::MacosArm64 => macho::thin(runner.bytes, macho::CPU_TYPE_ARM64)?,
                Target::MacosX64 => macho::thin(runner.bytes, macho::CPU_TYPE_X86_64)?,
                _ => {
                    for (cpu, word) in [(macho::CPU_TYPE_ARM64, "Apple silicon"), (macho::CPU_TYPE_X86_64, "Intel")] {
                        if !cpus.contains(&cpu) {
                            notes.push(format!("the executable has no {word} code: the app won't open on {word} Macs"));
                        }
                    }
                    runner.bytes.to_vec()
                }
            };
            if bundle {
                // (the bytecode in Resources: data after a Mach-O would break its signature)
                let mut resources = Vec::new();
                if let Some(code) = app.bytecode {
                    resources.push((format!("{}.rrbc", info.exe), code.to_vec()));
                }
                resources.push((NOTICES_FILE.to_string(), notices));
                files = macos::app_files(&macos::MacApp { info, icon: app.icon, executable: &exe, resources, minimum_system: macos::MINIMUM_SYSTEM })?;
                let root = macos::app_folder(info);
                add_data(&mut files, &format!("{root}/Contents/Resources"), &data)?;
                let signature = macos::sign_ad_hoc(&mut files, &root, info)?;
                let exe_path = format!("{root}/Contents/MacOS/{}", info.exe);
                (root, Some(exe_path), Some(signature))
            } else {
                files.add_executable(&info.exe, payload(&exe)?)?;
                files.add(NOTICES_FILE, notices)?;
                add_data(&mut files, "", &data)?;
                (info.exe.clone(), Some(info.exe.clone()), None)
            }
        }
        Some(System::Linux) => {
            check_elf(runner.bytes, target)?;
            let exe = payload(runner.bytes)?;
            if bundle {
                files = linux::appdir_files(&linux::AppDir { info, icon: app.icon, executable: &exe, files: Vec::new() })?;
                let root = linux::appdir_folder(info);
                files.add(&format!("{root}/{NOTICES_FILE}"), notices)?;
                add_data(&mut files, &format!("{root}/usr/bin"), &data)?;
                let exe_path = format!("{root}/usr/bin/{}", info.exe);
                (root, Some(exe_path), None)
            } else {
                files.add_executable(&info.exe, exe)?;
                files.add(NOTICES_FILE, notices)?;
                add_data(&mut files, "", &data)?;
                (info.exe.clone(), Some(info.exe.clone()), None)
            }
        }
        None => {
            let code = app.bytecode.ok_or("a web app is the program's bytecode and the web runtime: a native build can't be one")?;
            let runtime = web::Runtime::from_tar(runner.bytes)?;
            let needs = app.web_needs.cloned().unwrap_or_default();
            files = web::web_files(info, app.icon, code, &data, &runtime, runner.notices, &needs)?;
            ("index.html".to_string(), None, None)
        }
    };
    Ok(Packed { target, files, main, executable, bundle, signature, notes })
}

/// The archive's file name: `Name-1.0.0-windows-x64.zip`.
pub fn archive_name(info: &AppInfo, target: Target) -> String {
    format!("{}-{}.{}", info.file_stem(), target.name(), target.archive_ext())
}

/// How hard archives are packed: zlib's level 6 (its default: within a
/// few percent of 9, much faster).
const LEVEL: u32 = 6;

/// The archive that is shipped: the app's files and its `README.txt` (and
/// the source under `source/` when asked for) in a `.zip`, or for Linux a
/// `.tar.gz` with one top folder `Name-1.0.0-linux-x64/` (an AppDir's
/// contents there).
pub fn archive(packed: &Packed, app: &App) -> Result<Archive, String> {
    let target = packed.target;
    let name = archive_name(app.info, target);
    let readme = readme::readme(app.info, target, app.language, packed.bundle);
    let mut files = packed.files.clone();
    let linux = target.system() == Some(System::Linux);
    if linux {
        let top = name.trim_end_matches(".tar.gz").to_string();
        files = if packed.bundle {
            // (the AppDir is the top folder: ./AppRun and ./install.sh right there)
            let mut moved = Files::new();
            for f in files.iter() {
                let rest = f.path.strip_prefix(&format!("{}/", packed.main)).ok_or_else(|| format!("{} is outside the AppDir", f.path))?;
                if f.executable {
                    moved.add_executable(&format!("{top}/{rest}"), f.bytes.clone())?;
                } else {
                    moved.add(&format!("{top}/{rest}"), f.bytes.clone())?;
                }
            }
            moved
        } else {
            files.moved_under(&top)?
        };
        files.add(&format!("{top}/{README_FILE}"), readme.into_bytes())?;
        for (path, bytes) in app.source {
            files.add(&format!("{top}/source/{}", crate::files::safe(path)?), bytes.clone())?;
        }
    } else {
        // (a data file of that name keeps it: the help is then HOW-TO-OPEN.txt)
        let readme_name = if files.contains(README_FILE) { "HOW-TO-OPEN.txt" } else { README_FILE };
        files.add(readme_name, readme.into_bytes())?;
        for (path, bytes) in app.source {
            files.add(&format!("source/{}", crate::files::safe(path)?), bytes.clone())?;
        }
    }
    let format = if linux { rapidr_archive::Format::TarGz } else { rapidr_archive::Format::Zip };
    let mut w = rapidr_archive::Writer::new(format, LEVEL, 0)?;
    for f in files.sorted() {
        if f.executable {
            w.add_executable(&f.path, f.bytes.clone(), app.mtime)?;
        } else {
            w.add(&f.path, f.bytes.clone(), app.mtime)?;
        }
    }
    Ok(Archive { name, bytes: w.finish()? })
}

/// [`pack`] then [`archive`]: one target's archive, and the app it holds.
pub fn build(target: Target, app: &App, runner: &Runner) -> Result<(Packed, Archive), String> {
    let packed = pack(target, app, runner)?;
    let archive = archive(&packed, app)?;
    Ok((packed, archive))
}

/// SHA-256 of `bytes`, in lower-case hex.
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

/// A `SHA256SUMS` file's lines (`<hex>  <name>`, as `sha256sum` writes
/// them and `sha256sum -c` checks them): `existing`'s lines for other
/// files kept, those named in `made` (name, hex) replaced, sorted by name.
pub fn sha256sums(existing: &str, made: &[(String, String)]) -> String {
    let mut lines: Vec<(String, String)> = existing
        .lines()
        .filter_map(|l| {
            let (hex, name) = l.split_once("  ").or_else(|| l.split_once(" *"))?;
            let ok = hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit());
            ok.then(|| (name.trim().to_string(), hex.to_ascii_lowercase()))
        })
        .filter(|(name, _)| !made.iter().any(|(m, _)| m == name))
        .collect();
    lines.extend(made.iter().cloned());
    lines.sort();
    lines.into_iter().map(|(name, hex)| format!("{hex}  {name}\n")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::macho::tests::thin_macho;

    fn elf(machine: u16) -> Vec<u8> {
        let mut b = vec![0u8; 64];
        b[0..4].copy_from_slice(b"\x7fELF");
        b[4] = 2;
        b[5] = 1;
        b[18..20].copy_from_slice(&machine.to_le_bytes());
        b
    }

    fn universal_runner() -> Vec<u8> {
        macho::universal(&[&thin_macho(macho::CPU_TYPE_ARM64, b"arm"), &thin_macho(macho::CPU_TYPE_X86_64, b"intel")]).unwrap()
    }

    fn runner_for(t: Target) -> Vec<u8> {
        match t {
            Target::WindowsX64 => crate::windows::tests::tiny_exe(),
            Target::WindowsArm64 | Target::WindowsX86 => {
                let mut b = crate::windows::tests::tiny_exe();
                b[0x44..0x46].copy_from_slice(&windows_machine(t).to_le_bytes());
                b
            }
            Target::MacosUniversal | Target::MacosArm64 | Target::MacosX64 => universal_runner(),
            Target::LinuxX64 => elf(62),
            Target::LinuxArm64 => elf(183),
            Target::Web => crate::web::tests::tiny_runtime(),
        }
    }

    fn info() -> AppInfo {
        let mut i = AppInfo::new("Tip Calculator", "Tip-Calculator");
        i.version = "1.2.0".into();
        i.company = "Example Ltd".into();
        i.description = "Splits the bill".into();
        i
    }

    const NOTICES: &str = "THIRD-PARTY SOFTWARE NOTICES AND LICENCES\n";

    fn names(a: &Archive) -> Vec<(String, bool)> {
        let limits = rapidr_archive::Limits { max_size: 1 << 30, max_ratio: 0 };
        let opened = rapidr_archive::Archive::open(a.bytes.clone(), &a.name, &limits).unwrap();
        let mut v: Vec<(String, bool)> = opened.entries().iter().map(|e| (e.name.clone(), e.executable)).collect();
        v.sort();
        v
    }

    #[test]
    fn every_target_packs_and_archives() {
        let info = info();
        let icon = Icon::rapidr_default();
        let data = vec![("staff.csv".to_string(), b"a,b\n".to_vec()), ("art/logo.svg".to_string(), b"<svg/>".to_vec())];
        for t in Target::ALL.into_iter().chain([Target::MacosArm64, Target::MacosX64]) {
            let runner = runner_for(t);
            let app = App { data: &data, mtime: 1_793_000_000, ..App::new(&info, &icon, b"RRBC\x01") };
            let (packed, archive) = build(t, &app, &Runner { bytes: &runner, notices: NOTICES }).unwrap_or_else(|e| panic!("{t}: {e}"));
            assert_eq!(archive.name, format!("Tip-Calculator-1.2.0-{}.{}", t.name(), t.archive_ext()));
            let listed = names(&archive);
            let has = |p: &str| listed.iter().find(|(n, _)| n == p).map(|(_, x)| *x);
            match t.system() {
                Some(System::Windows) => {
                    assert_eq!(has("Tip-Calculator.exe"), Some(true), "{t}: {listed:?}");
                    for p in ["staff.csv", "art/logo.svg", NOTICES_FILE, README_FILE] {
                        assert_eq!(has(p), Some(false), "{t}: {p}");
                    }
                    let exe = &packed.files.get("Tip-Calculator.exe").unwrap().bytes;
                    // (the program after the runner; a windowed one)
                    assert!(exe.ends_with(&[b'R', b'R', b'B', b'C', b'E', b'X', b'E', b'1', 5, 0, 0, 0]));
                    let d = crate::windows::details(exe).unwrap();
                    assert!(d.gui && d.icon && d.strings["ProductName"] == "Tip Calculator", "{t}");
                }
                Some(System::MacOs) => {
                    assert_eq!(has("Tip Calculator.app/Contents/MacOS/Tip-Calculator"), Some(true), "{t}: {listed:?}");
                    for p in ["Tip Calculator.app/Contents/Info.plist", "Tip Calculator.app/Contents/Resources/Tip-Calculator.rrbc", "Tip Calculator.app/Contents/Resources/staff.csv", "Tip Calculator.app/Contents/Resources/art/logo.svg", "Tip Calculator.app/Contents/Resources/THIRD-PARTY-NOTICES.txt", README_FILE] {
                        assert_eq!(has(p), Some(false), "{t}: {p}");
                    }
                    let exe = &packed.files.get("Tip Calculator.app/Contents/MacOS/Tip-Calculator").unwrap().bytes;
                    let want: &[u32] = match t {
                        Target::MacosArm64 => &[macho::CPU_TYPE_ARM64],
                        Target::MacosX64 => &[macho::CPU_TYPE_X86_64],
                        _ => &[macho::CPU_TYPE_ARM64, macho::CPU_TYPE_X86_64],
                    };
                    assert_eq!(macho::cpus(exe).unwrap(), want, "{t}");
                    // signed ad hoc by RapidR's signer, in the archive too: every slice, with
                    // the app's identifier, its Info.plist and its seal bound
                    assert_eq!(packed.signature, Some(Signature::AdHoc));
                    assert_eq!(has("Tip Calculator.app/Contents/_CodeSignature/CodeResources"), Some(false), "{t}: {listed:?}");
                    let slices = crate::macho_sign::check(exe).unwrap_or_else(|e| panic!("{t}: {e}"));
                    assert_eq!(slices.len(), want.len(), "{t}");
                    assert!(slices.iter().all(|s| s.identifier == "dev.rapidr.app.tip-calculator" && s.special.len() == 3), "{t}: {slices:?}");
                    let seal = &packed.files.get("Tip Calculator.app/Contents/_CodeSignature/CodeResources").unwrap().bytes;
                    let seal = String::from_utf8_lossy(seal);
                    assert!(seal.contains("<key>Resources/art/logo.svg</key>") && seal.contains("<key>Resources/staff.csv</key>"), "{t}: {seal}");
                }
                Some(System::Linux) => {
                    let top = format!("Tip-Calculator-1.2.0-{}", t.name());
                    for (p, x) in [("AppRun", true), ("install.sh", true), ("usr/bin/Tip-Calculator", true), ("usr/bin/staff.csv", false), ("usr/bin/art/logo.svg", false), (NOTICES_FILE, false), (README_FILE, false), ("dev.rapidr.app.tip-calculator.desktop", false)] {
                        assert_eq!(has(&format!("{top}/{p}")), Some(x), "{t}: {p} in {listed:?}");
                    }
                }
                None => {
                    for p in ["index.html", "loader.js", "rapidrintr_bg.wasm", "rapidrintr.js", "Tip-Calculator.rrbc", "rapidr-assets.js", "manifest.webmanifest", "icons/icon-512.png", "icons/icon-32.png", NOTICES_FILE, README_FILE, "fonts/index.json"] {
                        assert_eq!(has(p), Some(false), "{t}: {p} in {listed:?}");
                    }
                    let html = String::from_utf8(packed.files.get("index.html").unwrap().bytes.clone()).unwrap();
                    assert!(html.contains("<title>Tip Calculator</title>") && html.contains("manifest.webmanifest") && html.contains("Splits the bill"), "{html}");
                }
            }
            // (no file of the app outside its place, nothing twice)
            assert!(listed.iter().all(|(n, _)| !n.starts_with('/') && !n.contains("..")));
        }
    }

    /// The same inputs make the same bytes: SHA256SUMS can be checked by
    /// building again.
    #[test]
    fn archives_are_reproducible() {
        let info = info();
        let icon = Icon::rapidr_default();
        for t in [Target::WindowsX64, Target::LinuxX64, Target::MacosUniversal] {
            let runner = runner_for(t);
            let app = App { mtime: 1_793_000_000, ..App::new(&info, &icon, b"RRBC") };
            let r = Runner { bytes: &runner, notices: NOTICES };
            assert_eq!(build(t, &app, &r).unwrap().1.bytes, build(t, &app, &r).unwrap().1.bytes, "{t}");
        }
    }

    #[test]
    fn the_wrong_runner_is_refused() {
        let info = info();
        let icon = Icon::rapidr_default();
        let app = App::new(&info, &icon, b"RRBC");
        let cases = [
            (Target::WindowsX64, runner_for(Target::WindowsArm64), "is for ARM64, not x64"),
            (Target::WindowsX86, runner_for(Target::WindowsX64), "is for x64, not 32-bit x86"),
            (Target::LinuxArm64, elf(62), "is for x64, not ARM64"),
            (Target::LinuxX64, runner_for(Target::WindowsX64), "not a Linux executable"),
            (Target::MacosUniversal, elf(62), "not a macOS executable"),
            (Target::MacosX64, thin_macho(macho::CPU_TYPE_ARM64, b"arm"), "not Intel"),
            (Target::Web, elf(62), "web runtime"),
        ];
        for (t, runner, says) in cases {
            let e = pack(t, &app, &Runner { bytes: &runner, notices: NOTICES }).unwrap_err();
            assert!(e.contains(says), "{t}: {e}");
        }
        // (no notices: no app)
        let e = pack(Target::LinuxX64, &app, &Runner { bytes: &elf(62), notices: " " }).unwrap_err();
        assert!(e.contains("notices"), "{e}");
        // (a thin runner for the universal app: built, and said)
        let thin = thin_macho(macho::CPU_TYPE_ARM64, b"arm");
        let p = pack(Target::MacosUniversal, &app, &Runner { bytes: &thin, notices: NOTICES }).unwrap();
        assert!(p.notes.iter().any(|n| n.contains("no Intel code")), "{:?}", p.notes);
    }

    #[test]
    fn data_paths_are_guarded() {
        let info = info();
        let icon = Icon::rapidr_default();
        let runner = runner_for(Target::LinuxX64);
        let r = Runner { bytes: &runner, notices: NOTICES };
        for bad in ["../secret.txt", "/etc/passwd", "C:\\boot.ini", "a/../../x"] {
            let data = vec![(bad.to_string(), b"x".to_vec())];
            let app = App { data: &data, ..App::new(&info, &icon, b"RRBC") };
            assert!(pack(Target::LinuxX64, &app, &r).unwrap_err().contains("can't go in the app"), "{bad}");
        }
        // (source is left out, with a note)
        let data = vec![("main.rr".to_string(), b"PRINT 1".to_vec()), ("game.BAS".to_string(), b"x".to_vec()), ("level1.txt".to_string(), b"x".to_vec())];
        let app = App { data: &data, ..App::new(&info, &icon, b"RRBC") };
        let p = pack(Target::WindowsX64, &app, &Runner { bytes: &runner_for(Target::WindowsX64), notices: NOTICES }).unwrap();
        assert!(!p.files.contains("main.rr") && !p.files.contains("game.BAS") && p.files.contains("level1.txt"));
        assert!(p.notes.iter().any(|n| n.contains("main.rr") && n.contains("game.BAS")), "{:?}", p.notes);
        // (a data file named like the app's own: an error, never a silent overwrite)
        let data = vec![("Tip-Calculator.exe".to_string(), b"x".to_vec())];
        let app = App { data: &data, ..App::new(&info, &icon, b"RRBC") };
        assert!(pack(Target::WindowsX64, &app, &Runner { bytes: &runner_for(Target::WindowsX64), notices: NOTICES }).unwrap_err().contains("rename it"));
        // (the source, when asked for: under source/ in the archive only)
        let source = vec![("main.rr".to_string(), b"PRINT 1".to_vec())];
        let app = App { source: &source, ..App::new(&info, &icon, b"RRBC") };
        let (p, a) = build(Target::WindowsX64, &app, &Runner { bytes: &runner_for(Target::WindowsX64), notices: NOTICES }).unwrap();
        assert!(!p.files.contains("source/main.rr"));
        assert!(names(&a).iter().any(|(n, _)| n == "source/main.rr"));
    }

    #[test]
    fn console_programs() {
        let mut info = info();
        info.console = true;
        let icon = Icon::rapidr_default();
        let app = App::new(&info, &icon, b"RRBC");
        // (Windows: the console runner, still with its icon)
        let p = pack(Target::WindowsX64, &app, &Runner { bytes: &runner_for(Target::WindowsX64), notices: NOTICES }).unwrap();
        let d = crate::windows::details(&p.files.get("Tip-Calculator.exe").unwrap().bytes).unwrap();
        assert!(!d.gui && d.icon);
        // (macOS and Linux: a plain executable, the program after it)
        for t in [Target::MacosUniversal, Target::LinuxX64] {
            let p = pack(t, &app, &Runner { bytes: &runner_for(t), notices: NOTICES }).unwrap();
            assert!(!p.bundle && p.main == "Tip-Calculator", "{t}");
            let exe = p.files.get("Tip-Calculator").unwrap();
            assert!(exe.executable && exe.bytes.ends_with(&[b'1', 4, 0, 0, 0]), "{t}");
            assert!(p.files.contains(NOTICES_FILE));
        }
        // (asked for: an app anyway)
        let app = App { bundle: Some(true), ..App::new(&info, &icon, b"RRBC") };
        let p = pack(Target::LinuxX64, &app, &Runner { bytes: &runner_for(Target::LinuxX64), notices: NOTICES }).unwrap();
        assert_eq!(p.main, "Tip Calculator.AppDir");
    }

    #[test]
    fn native_builds_use_the_same_packer() {
        let info = info();
        let icon = Icon::rapidr_default();
        let app = App { bytecode: None, ..App::new(&info, &icon, b"") };
        let exe = runner_for(Target::LinuxX64);
        let p = pack(Target::LinuxX64, &app, &Runner { bytes: &exe, notices: NOTICES }).unwrap();
        assert_eq!(p.files.get("Tip Calculator.AppDir/usr/bin/Tip-Calculator").unwrap().bytes, exe, "nothing appended");
        assert!(pack(Target::Web, &app, &Runner { bytes: &runner_for(Target::Web), notices: NOTICES }).unwrap_err().contains("native"));
    }

    #[test]
    fn sha256sums_lines() {
        assert_eq!(sha256_hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        let a = ("A-1.0.0-web.zip".to_string(), sha256_hex(b"a"));
        let b = ("A-1.0.0-linux-x64.tar.gz".to_string(), sha256_hex(b"b"));
        let first = sha256sums("", &[a.clone()]);
        assert_eq!(first, format!("{}  A-1.0.0-web.zip\n", a.1));
        // (a second build adds its line and keeps the first's; a rebuilt one replaced)
        let both = sha256sums(&first, &[b.clone()]);
        assert_eq!(both.lines().count(), 2);
        assert!(both.starts_with(&b.1), "sorted by name: {both}");
        let again = sha256sums(&both, &[("A-1.0.0-web.zip".to_string(), sha256_hex(b"new"))]);
        assert_eq!(again.lines().count(), 2);
        assert!(again.contains(&sha256_hex(b"new")) && !again.contains(&a.1));
        assert_eq!(sha256sums("garbage\n\n", &[]), "");
    }
}

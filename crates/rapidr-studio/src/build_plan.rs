//! The Build dialog's plan (Run > Build, docs/build-for-every-platform-plan.md
//! §2; design/build-dialog): which systems a project builds for, what each
//! app's zip is called, the command that builds them (`rapidr build
//! --targets …`, the desktop's one path), and what that command's lines say
//! about each app as it builds. ONE place for the dialog, Build Again and the
//! AI's `build_app`: they all build through [`cli_args`] with the project's
//! saved choices, and read the build's lines through [`news`].
//!
//! Pure: no files, no processes. The web Studio uses the same choices and
//! names; its packing is in the page (crate::build's web build).

use rapidr_package::{AppInfo, Target};
use rapidr_project::Project;

/// The systems the dialog offers, in its order (rapidr_package::Target::ALL).
pub const OFFERED: [Target; 7] = Target::ALL;

/// The systems project `p` builds for: its saved choice (`[build]
/// build_for`), else `first` (the first time: this computer's system).
pub fn targets(p: &Project, first: &[Target]) -> Vec<Target> {
    let saved: Vec<Target> = p.build.build_for.iter().filter_map(|n| Target::parse(n).ok()).filter(|t| OFFERED.contains(t)).collect();
    let mut out = if p.build.build_for.is_empty() { first.to_vec() } else { saved };
    out.sort();
    out.dedup();
    out
}

/// The dialog's first choice on this computer: its own system (an offline
/// first Build always works: its runner is the install's own). `None` where
/// RapidR has no runner (then the web).
pub fn first_choice(here: Option<Target>) -> Vec<Target> {
    vec![here.unwrap_or(Target::Web)]
}

/// The system a browser's `navigator.platform` / user agent says (the web
/// Studio's first choice): Windows x64, macOS, Linux x64; else the web.
pub fn target_of_platform(platform: &str) -> Target {
    let p = platform.to_ascii_lowercase();
    if p.contains("mac") || p.contains("iphone") || p.contains("ipad") {
        Target::MacosUniversal
    } else if p.contains("win") {
        if p.contains("arm") { Target::WindowsArm64 } else { Target::WindowsX64 }
    } else if p.contains("linux") && !p.contains("android") {
        if p.contains("aarch64") || p.contains("arm") { Target::LinuxArm64 } else { Target::LinuxX64 }
    } else {
        Target::Web
    }
}

/// Saves `targets` as the project's choice.
pub fn set_targets(p: &mut Project, targets: &[Target]) {
    let mut t: Vec<Target> = targets.iter().copied().filter(|t| OFFERED.contains(t)).collect();
    t.sort();
    t.dedup();
    p.build.build_for = t.iter().map(|t| t.name().to_string()).collect();
    // (the Problems pane's notes follow where it runs: the web, the
    // desktop or both — what Project Options' Targets said before)
    let web = t.contains(&Target::Web);
    let desktop = t.iter().any(|t| t.system().is_some());
    p.build.runs_on = match (desktop, web) {
        (true, true) => "both",
        (false, true) => "web",
        _ => "desktop",
    }
    .to_string();
}

/// The app's details as a build makes them (its name and version name the
/// zips): the project's App info, else its name, else the main file's.
pub fn app_info(p: &Project, main_stem: &str) -> AppInfo {
    let b = &p.build;
    let name = [b.app_name.trim(), p.name.trim(), main_stem.trim()].into_iter().find(|n| !n.is_empty()).unwrap_or("Program").to_string();
    let exe = rapidr_package::exe_name_for(&p.name).or_else(|| rapidr_package::exe_name_for(main_stem)).unwrap_or_else(|| "program".into());
    let mut info = AppInfo::new(&name, &exe);
    if !b.version.trim().is_empty() {
        info.version = b.version.trim().to_string();
    }
    if !b.bundle_id.trim().is_empty() {
        info.bundle_id = b.bundle_id.trim().to_string();
    }
    info.company = b.company.trim().to_string();
    info
}

/// The zip (`.tar.gz` for Linux) a build makes for `t`.
pub fn zip_name(p: &Project, main_stem: &str, t: Target) -> String {
    rapidr_package::archive_name(&app_info(p, main_stem), t)
}

/// The one zip that holds every app ("Everything in one zip").
pub fn all_zip_name(p: &Project, main_stem: &str) -> String {
    format!("{}-all.zip", app_info(p, main_stem).file_stem())
}

/// A suggested app id made from the author and the name
/// (`com.northfield.tipcalculator`): the first word of the author, the name
/// without spaces; `dev.rapidr.app.<name>` without an author.
pub fn suggested_app_id(company: &str, name: &str) -> String {
    let word = |s: &str| s.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_ascii_lowercase();
    let name = word(name);
    let name = if name.is_empty() { "app".to_string() } else { name };
    match company.split_whitespace().map(word).find(|w| !w.is_empty()) {
        Some(c) => format!("com.{c}.{name}"),
        None => format!("dev.rapidr.app.{name}"),
    }
}

/// `rapidr build`'s arguments for the dialog's choices: the main file in
/// full, the project's file (its App info: saved before a build), the
/// systems, the folder the zips go in (in full), the source when asked for.
pub fn cli_args(main: &str, project_file: Option<&str>, p: &Project, out_dir: &str, targets: &[Target]) -> Vec<String> {
    let mut args = vec!["build".to_string(), main.to_string()];
    if let Some(f) = project_file.filter(|f| !f.is_empty()) {
        args.extend(["--project".to_string(), f.to_string()]);
    }
    let names: Vec<&str> = targets.iter().map(|t| t.name()).collect();
    args.extend(["--targets".to_string(), names.join(","), "--interp".to_string(), "--output".to_string(), out_dir.to_string()]);
    if p.build.include_source {
        args.push("--include-source".into());
    }
    args
}

/// `rapidr build`'s arguments for this computer's app as native code (the
/// dialog's Advanced > Native code): a folder build in the same output
/// folder, its generated Rust kept or not.
pub fn native_args(main: &str, project_file: Option<&str>, p: &Project, out_dir: &str) -> Vec<String> {
    let mut args = vec!["build".to_string(), main.to_string()];
    if let Some(f) = project_file.filter(|f| !f.is_empty()) {
        args.extend(["--project".to_string(), f.to_string()]);
    }
    args.extend(["--output".to_string(), out_dir.to_string()]);
    args.push(if p.build.keep_rust { "--keep-rust" } else { "--no-keep-rust" }.to_string());
    args
}

/// The systems of `list` (`windows-x64,web`) whose runners this computer
/// doesn't have yet — they download first, once, from rapidr.dev, checked
/// against RapidR's signature — with the size the cached list of runners
/// gives (`target` or `target:bytes`, comma-separated). Looked for: the
/// install's `runners/<dir>/` (beside `rapidr`, up to three folders above it),
/// and the runners cache (`RAPIDR_RUNNERS_CACHE`, else the build cache's
/// `runners/<version>/<dir>.zst`). On the desktop in a source checkout this
/// computer's runner and the web's are built there: never a download. On the
/// web the page fetches them all, except the web's own (the page is it).
pub fn runners_to_fetch(list: &str, rapidr: Option<&std::path::Path>) -> String {
    let targets: Vec<Target> = list.split([',', ' ']).filter_map(|n| Target::parse(n).ok()).collect();
    let mut out = Vec::new();
    for t in targets {
        if !runner_is_here(t, rapidr) {
            out.push(match listed_size(t) {
                Some(b) => format!("{}:{b}", t.name()),
                None => t.name().to_string(),
            });
        }
    }
    out.join(",")
}

#[cfg(target_arch = "wasm32")]
fn runner_is_here(t: Target, _rapidr: Option<&std::path::Path>) -> bool {
    t == Target::Web
}

#[cfg(target_arch = "wasm32")]
fn listed_size(_t: Target) -> Option<u64> {
    None
}

#[cfg(not(target_arch = "wasm32"))]
fn runner_is_here(t: Target, rapidr: Option<&std::path::Path>) -> bool {
    let dir = t.runner_dir();
    // (an install: runners/<dir>/ near rapidr)
    let mut install = false;
    let mut checkout = false;
    if let Some(exe) = rapidr {
        for a in exe.ancestors().skip(1).take(4) {
            if a.join("runners").join(dir).is_dir() {
                return true;
            }
            if a.join("runners").is_dir() && a.join("release.toml").is_file() {
                install = true;
            }
            if a.join("ide").join("studio.rr").is_file() && a.join("crates").is_dir() {
                checkout = true;
            }
        }
    }
    // (a checkout builds this computer's runner and the web's itself)
    if checkout && !install && (t == Target::Web || Some(t) == Target::host()) {
        return true;
    }
    cache_dir().is_some_and(|c| c.join(rapidr_package_version()).join(format!("{dir}.zst")).is_file())
}

/// RapidR's version (the runners a build uses are this one's).
#[cfg(not(target_arch = "wasm32"))]
fn rapidr_package_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// The runners' cache, as `rapidr build` keeps it.
#[cfg(not(target_arch = "wasm32"))]
fn cache_dir() -> Option<std::path::PathBuf> {
    use std::path::PathBuf;
    let absolute = |v: std::ffi::OsString| Some(PathBuf::from(v)).filter(|p| p.is_absolute());
    if let Some(d) = std::env::var_os("RAPIDR_RUNNERS_CACHE").and_then(absolute) {
        return Some(d);
    }
    let root = if let Some(d) = std::env::var_os("RAPIDR_BUILD_CACHE").and_then(absolute) {
        Some(d)
    } else if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA").and_then(absolute).map(|d| d.join("RapidR").join("Cache"))
    } else if cfg!(target_os = "macos") {
        std::env::home_dir().map(|h| h.join("Library/Caches/RapidR"))
    } else {
        std::env::var_os("XDG_CACHE_HOME").and_then(absolute).or_else(|| std::env::home_dir().map(|h| h.join(".cache"))).map(|d| d.join("rapidr"))
    };
    root.map(|r| r.join("runners"))
}

/// A runner's size as the cached list of runners says it (shown before a
/// download; the download itself is checked against the signed list).
#[cfg(not(target_arch = "wasm32"))]
fn listed_size(t: Target) -> Option<u64> {
    let text = std::fs::read(cache_dir()?.join(rapidr_package_version()).join("runners-manifest.json")).ok()?;
    let json: serde_json::Value = serde_json::from_slice(&text).ok()?;
    json.get("runners")?.as_array()?.iter().find(|r| r.get("target").and_then(|x| x.as_str()) == Some(t.runner_dir()))?.get("size")?.as_u64()
}

/// A command line a terminal reads back as `args` (Copy Command Line):
/// `rapidr` and each argument, quoted when it has to be.
pub fn command_line(args: &[String]) -> String {
    let quote = |a: &str| {
        if !a.is_empty() && a.chars().all(|c| c.is_ascii_alphanumeric() || "-_./:,=@+".contains(c)) {
            a.to_string()
        } else {
            format!("\"{}\"", a.replace('\\', "\\\\").replace('"', "\\\"").replace('$', "\\$").replace('`', "\\`"))
        }
    };
    std::iter::once("rapidr".to_string()).chain(args.iter().map(|a| quote(a))).collect::<Vec<_>>().join(" ")
}

/// What one line of `rapidr build --targets` says, for the dialog's rows.
#[derive(Debug, Clone, PartialEq)]
pub enum News {
    /// "Building tip_calculator.rr for windows-x64, web": the systems, in order.
    Started(Vec<Target>),
    /// Where the zips go.
    Folder(String),
    /// A runner being downloaded for a system (its size as said: "19.0 MB").
    Downloading(Target, String),
    /// A system's app is built: its zip's name, its size, the seconds.
    Built { target: Target, zip: String, bytes: u64, seconds: f64 },
    /// A system's app is not built, and why (a finished sentence).
    Failed { target: Target, why: String },
    /// `SHA256SUMS` written.
    Checksums(String),
    /// "3 of 4 built in 2.8 s".
    Summary { built: usize, of: usize, seconds: f64 },
}

/// The ways a line names a system: the packer's label ("Windows (64-bit,
/// x64)") and the runners' ("Windows 64-bit (x64)").
fn said_as(t: Target) -> [&'static str; 2] {
    let runners = match t {
        Target::WindowsX64 => "Windows 64-bit (x64)",
        Target::WindowsArm64 => "Windows ARM64",
        Target::WindowsX86 => "Windows 32-bit (x86)",
        Target::MacosUniversal | Target::MacosArm64 | Target::MacosX64 => "macOS (Apple silicon and Intel)",
        Target::LinuxX64 => "Linux 64-bit (x64)",
        Target::LinuxArm64 => "Linux ARM64",
        Target::Web => "web (browser)",
    };
    [t.label(), runners]
}

/// The system a text starts with (or names, for `anywhere`), longest name first.
fn named(text: &str, anywhere: bool) -> Option<(Target, usize)> {
    let mut best: Option<(Target, usize)> = None;
    for t in OFFERED {
        for name in said_as(t) {
            let found = if anywhere { text.find(name).map(|i| i + name.len()) } else { text.starts_with(name).then_some(name.len()) };
            if let Some(end) = found {
                if best.is_none_or(|(_, e)| name.len() > e) {
                    best = Some((t, end));
                }
            }
        }
    }
    best
}

/// What `line` (a line `rapidr build --targets` printed) says, if it says
/// something about the apps.
pub fn news(line: &str) -> Option<News> {
    let l = line.trim_end();
    if let Some(rest) = l.strip_prefix("Building ") {
        let (_, list) = rest.rsplit_once(" for ")?;
        let t: Vec<Target> = list.split(',').filter_map(|n| Target::parse(n.trim()).ok()).collect();
        return (!t.is_empty()).then_some(News::Started(t));
    }
    if let Some(dir) = l.strip_prefix("Output folder: ") {
        return Some(News::Folder(dir.trim().to_string()));
    }
    if let Some(path) = l.strip_prefix("Checksums: ") {
        return Some(News::Checksums(path.trim().to_string()));
    }
    if let Some(rest) = l.strip_prefix("Downloading the ") {
        let (t, end) = named(rest, false)?;
        let size = rest[end..].trim_start().strip_prefix("runner").unwrap_or("").trim();
        let size = size.trim_start_matches('(').split(')').next().unwrap_or("").trim().to_string();
        return Some(News::Downloading(t, size));
    }
    // "3 of 4 built in 2.8 s"
    if let Some((counts, secs)) = l.split_once(" built in ") {
        if let Some((a, b)) = counts.split_once(" of ") {
            if let (Ok(built), Ok(of)) = (a.trim().parse(), b.trim().parse()) {
                let seconds = secs.trim().trim_end_matches(" s").parse().unwrap_or(0.0);
                return Some(News::Summary { built, of, seconds });
            }
        }
    }
    // "  Windows (64-bit, x64): Tip-1.0.0-windows-x64.zip (21.0 MB, 0.3 s)"
    // "  macOS (Apple silicon and Intel): not built: …"
    let body = l.trim_start();
    if l.len() != body.len() {
        let (t, end) = named(body, false)?;
        let rest = body[end..].strip_prefix(": ")?;
        if let Some(why) = rest.strip_prefix("not built: ") {
            return Some(News::Failed { target: t, why: why.trim().to_string() });
        }
        let (zip, sizes) = rest.rsplit_once(" (")?;
        let sizes = sizes.trim_end_matches(')');
        let mut parts = sizes.split(", ");
        let mb: f64 = parts.next()?.trim_end_matches(" MB").parse().ok()?;
        let seconds: f64 = parts.next().map(|s| s.trim_end_matches(" s").parse().unwrap_or(0.0)).unwrap_or(0.0);
        return Some(News::Built { target: t, zip: zip.trim().to_string(), bytes: (mb * 1_048_576.0) as u64, seconds });
    }
    None
}

/// [`news`] as one line of tab-separated words, for Studio's code
/// (RPROJECT.BuildNews): `start`, `folder`, `download`, `built`, `failed`,
/// `sums`, `summary`, then their values; "" for any other line.
pub fn news_words(line: &str) -> String {
    let Some(n) = news(line) else { return String::new() };
    let t = |t: Target| t.name().to_string();
    let fields: Vec<String> = match n {
        News::Started(list) => vec!["start".into(), list.into_iter().map(t).collect::<Vec<_>>().join(",")],
        News::Folder(d) => vec!["folder".into(), d],
        News::Downloading(target, size) => vec!["download".into(), t(target), size],
        News::Built { target, zip, bytes, seconds } => vec!["built".into(), t(target), zip, bytes.to_string(), format!("{seconds:.1}")],
        News::Failed { target, why } => vec!["failed".into(), t(target), why.replace(['\t', '\n'], " ")],
        News::Checksums(p) => vec!["sums".into(), p],
        News::Summary { built, of, seconds } => vec!["summary".into(), built.to_string(), of.to_string(), format!("{seconds:.1}")],
    };
    fields.join("\t")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cli_lines_say_what_each_app_did() {
        assert_eq!(news("Building tip_calculator.rr for windows-x64, macos-universal, web"), Some(News::Started(vec![Target::WindowsX64, Target::MacosUniversal, Target::Web])));
        assert_eq!(news("Output folder: /p/build"), Some(News::Folder("/p/build".into())));
        match news("  Windows (64-bit, x64): Tip-Calculator-1.0.0-windows-x64.zip (21.0 MB, 0.3 s)") {
            Some(News::Built { target, zip, bytes, seconds }) => {
                assert_eq!(target, Target::WindowsX64);
                assert_eq!(zip, "Tip-Calculator-1.0.0-windows-x64.zip");
                assert_eq!(bytes, 22_020_096);
                assert!((seconds - 0.3).abs() < 1e-9);
            }
            other => panic!("{other:?}"),
        }
        // (the 32-bit label is longer than the 64-bit one: the right one wins)
        assert!(matches!(news("  Windows (32-bit, x86): T-1.0.0-windows-x86.zip (18.0 MB, 0.2 s)"), Some(News::Built { target: Target::WindowsX86, .. })));
        assert_eq!(
            news("  macOS (Apple silicon and Intel): not built: This runner doesn't match RapidR 2.123.1's list: it was not used."),
            Some(News::Failed { target: Target::MacosUniversal, why: "This runner doesn't match RapidR 2.123.1's list: it was not used.".into() })
        );
        assert!(matches!(news("  the web: T-1.0.0-web.zip (6.0 MB, 0.1 s)"), Some(News::Built { target: Target::Web, .. })));
        assert_eq!(news("Downloading the Windows 32-bit (x86) runner (19.0 MB)…"), Some(News::Downloading(Target::WindowsX86, "19.0 MB".into())));
        assert_eq!(news("3 of 4 built in 2.8 s"), Some(News::Summary { built: 3, of: 4, seconds: 2.8 }));
        assert_eq!(news("Checksums: /p/build/SHA256SUMS"), Some(News::Checksums("/p/build/SHA256SUMS".into())));
        assert_eq!(news("  Tip Calculator 1.0.0 (com.x.tip), icon: tip.png"), None);
        assert_eq!(news("Build succeeded!"), None);
        assert_eq!(news_words("  the web: T-1.0.0-web.zip (6.0 MB, 0.1 s)"), "built\tweb\tT-1.0.0-web.zip\t6291456\t0.1");
    }

    #[test]
    fn choices_and_names() {
        let mut p = Project::default();
        p.name = "Tip Calculator".into();
        assert_eq!(targets(&p, &first_choice(Some(Target::MacosUniversal))), [Target::MacosUniversal]);
        set_targets(&mut p, &[Target::Web, Target::WindowsX64, Target::Web]);
        assert_eq!(p.build.build_for, ["windows-x64", "web"]);
        assert_eq!(p.build.runs_on, "both");
        assert_eq!(targets(&p, &[Target::LinuxX64]), [Target::WindowsX64, Target::Web]);
        set_targets(&mut p, &[Target::Web]);
        assert_eq!(p.build.runs_on, "web");
        p.build.version = "1.2.0".into();
        assert_eq!(zip_name(&p, "tip", Target::LinuxArm64), "Tip-Calculator-1.2.0-linux-arm64.tar.gz");
        assert_eq!(all_zip_name(&p, "tip"), "Tip-Calculator-1.2.0-all.zip");
        assert_eq!(suggested_app_id("Northfield Coding Club", "Tip Calculator"), "com.northfield.tipcalculator");
        assert_eq!(suggested_app_id("", "Tip Calculator"), "dev.rapidr.app.tipcalculator");
        assert_eq!(target_of_platform("MacIntel"), Target::MacosUniversal);
        assert_eq!(target_of_platform("Win32"), Target::WindowsX64);
        assert_eq!(target_of_platform("Linux x86_64"), Target::LinuxX64);
        assert_eq!(target_of_platform("Linux aarch64"), Target::LinuxArm64);
        assert_eq!(target_of_platform("Linux armv8l Android"), Target::Web);
    }

    #[test]
    fn the_command_line() {
        let mut p = Project::default();
        p.build.include_source = true;
        let args = cli_args("/p/tip.rr", Some("/p/Tip.rrproj"), &p, "/p/my build", &[Target::WindowsX64, Target::Web]);
        assert_eq!(args, ["build", "/p/tip.rr", "--project", "/p/Tip.rrproj", "--targets", "windows-x64,web", "--interp", "--output", "/p/my build", "--include-source"]);
        assert_eq!(command_line(&args), "rapidr build /p/tip.rr --project /p/Tip.rrproj --targets windows-x64,web --interp --output \"/p/my build\" --include-source");
        assert_eq!(native_args("/p/tip.rr", None, &p, "/p/build"), ["build", "/p/tip.rr", "--output", "/p/build", "--no-keep-rust"]);
    }
}

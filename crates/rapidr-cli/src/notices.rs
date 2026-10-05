//! `THIRD-PARTY-NOTICES.txt`: the open-source notices every output RapidR
//! builds carries — native and interpreted executables beside them, web
//! builds in their root — so whoever ships a program ships what the
//! licences of the code inside it ask for, without doing anything.
//!
//! The text depends on the *kind* of output, never on the program: a
//! desktop executable for one `<os>-<arch>` (native or interpreted: one file
//! covers both, so they can share a folder), the web (both web builds), or
//! RapidR's own tools (what an install's `share/doc` carries). It is made
//! from the real dependency graph of the crates that kind compiles in
//! (`cargo tree`, normal dependencies, for that target), with each crate's
//! own licence files, plus the components that aren't crates (Rust's
//! standard library, the built-in fonts, C code bundled in crates, the
//! toolchain's and the system's pieces).
//!
//! A checkout generates it (and caches it in `target/notices/`); an install
//! ships it, generated at release time by this same code
//! (`tools/release/stage.py`: `lib/rapidr/notices/<kind>.txt`), so it works
//! offline with no Rust. `rapidr notices [<kind>] [-o FILE]` prints one.
//! docs/licensing.md has the per-output tables and what each licence asks.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use crate::home::{self, Home};

/// The file every output carries.
pub const FILE_NAME: &str = "THIRD-PARTY-NOTICES.txt";

/// Bumped when the text's format changes (checkout caches are keyed by it).
const FORMAT: u32 = 1;

/// What a notices file covers.
#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    /// Native and interpreted desktop executables for `<os>-<arch>`.
    Desktop(String),
    /// Native web builds and web bundles (`bundle-bc`, the web IDE's).
    Web,
    /// RapidR's own executables (`rapidr`, `rapidrw`) for an OS.
    Tools(String),
}

impl Kind {
    pub fn parse(name: &str) -> Result<Kind, String> {
        let kind = match name {
            "web" => Kind::Web,
            _ => match name.strip_prefix("tools-") {
                Some(os) => Kind::Tools(os.to_string()),
                None => Kind::Desktop(name.to_string()),
            },
        };
        kind.triples()?;
        Ok(kind)
    }

    /// `macos`, `web`, `tools-linux`: the shipped file's name.
    pub fn name(&self) -> String {
        match self {
            Kind::Desktop(t) => t.clone(),
            Kind::Web => "web".into(),
            Kind::Tools(os) => format!("tools-{os}"),
        }
    }

    fn os(&self) -> &str {
        match self {
            Kind::Desktop(t) => t.split('-').next().unwrap_or(""),
            Kind::Web => "web",
            Kind::Tools(os) => os,
        }
    }

    /// The Rust targets this kind is built for: Windows has two toolchains
    /// (gnullvm and msvc), the user's choice, so both graphs count.
    fn triples(&self) -> Result<Vec<&'static str>, String> {
        let t = match self {
            Kind::Web => return Ok(vec!["wasm32-unknown-unknown"]),
            Kind::Tools(os) => {
                return Ok(match os.as_str() {
                    "macos" => vec!["aarch64-apple-darwin", "x86_64-apple-darwin"],
                    "windows" => vec!["x86_64-pc-windows-gnullvm", "x86_64-pc-windows-msvc", "aarch64-pc-windows-gnullvm", "aarch64-pc-windows-msvc"],
                    "linux" => vec!["x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu"],
                    _ => return Err(format!("unknown OS '{os}' (macos, windows, linux)")),
                })
            }
            Kind::Desktop(t) => t.as_str(),
        };
        Ok(match t {
            // (universal: both slices)
            "macos" => vec!["aarch64-apple-darwin", "x86_64-apple-darwin"],
            "macos-arm64" => vec!["aarch64-apple-darwin"],
            "macos-x86_64" => vec!["x86_64-apple-darwin"],
            "windows-x86_64" => vec!["x86_64-pc-windows-gnullvm", "x86_64-pc-windows-msvc"],
            "windows-aarch64" => vec!["aarch64-pc-windows-gnullvm", "aarch64-pc-windows-msvc"],
            "linux-x86_64" => vec!["x86_64-unknown-linux-gnu"],
            "linux-aarch64" => vec!["aarch64-unknown-linux-gnu"],
            _ => return Err(format!("unknown target '{t}': macos (universal), <os>-<arch> (macos-arm64, windows-x86_64, linux-x86_64, …), web, or tools-<os>")),
        })
    }

    /// The workspace crates (and features) whose graph is compiled in: a
    /// native program depends on rapidr-runtime-core, an interpreted one is
    /// the runner stub; a native web program on rapidr-runtime-web with its
    /// `kernel` feature (codegen's Cargo.toml), a web bundle runs
    /// rapidr-vm-host-web's wasm.
    pub fn roots(&self) -> (&'static [&'static str], &'static [&'static str]) {
        match self {
            Kind::Desktop(_) => (&["rapidr-runtime-core", "rapidr-runner-stub"], &[]),
            Kind::Web => (&["rapidr-vm-host-web", "rapidr-runtime-web"], &["rapidr-runtime-web/kernel"]),
            Kind::Tools(_) => (&["rapidr-cli", "rapidr-launcher"], &[]),
        }
    }

    fn describe(&self) -> String {
        match self {
            Kind::Desktop(t) => format!("desktop executables for {t} (native and interpreted)"),
            Kind::Web => "web builds (bundle-bc / build --web, and the web IDE's bundles)".into(),
            Kind::Tools(os) => format!("RapidR's own programs for {os} (rapidr, rapidrw, the RapidR Runtime)"),
        }
    }
}

/// `rapidr notices [<kind>] [-o FILE]`: the notices for `kind` (default:
/// this machine's desktop executables).
pub fn command(args: &[String]) -> ExitCode {
    let mut kind = None;
    let mut out = None;
    let mut iter = args.iter();
    while let Some(a) = iter.next() {
        match a.as_str() {
            "-o" | "--output" => out = iter.next().cloned(),
            _ if kind.is_none() && !a.starts_with('-') => kind = Some(a.clone()),
            _ => {
                eprintln!("usage: rapidr notices [<os>-<arch>|web|tools-<os>] [-o FILE]");
                return ExitCode::from(2);
            }
        }
    }
    let kind = match Kind::parse(&kind.unwrap_or_else(home::host_target)) {
        Ok(k) => k,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let text = match text(&kind) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
    };
    match out {
        Some(path) => {
            if let Some(dir) = Path::new(&path).parent().filter(|d| !d.as_os_str().is_empty()) {
                let _ = fs::create_dir_all(dir);
            }
            if let Err(e) = fs::write(&path, &text) {
                eprintln!("write {path}: {e}");
                return ExitCode::from(1);
            }
            eprintln!("wrote {path}");
        }
        None => print!("{text}"),
    }
    ExitCode::SUCCESS
}

/// The notices for `kind`: an install's shipped file, or in a checkout
/// generated from the dependency graph (cached in `target/notices/`).
pub fn text(kind: &Kind) -> Result<String, String> {
    let home = Home::find().ok_or("RapidR's home wasn't found (RAPIDR_HOME, an install, or a checkout)")?;
    if home.release.is_some() {
        let path = home.root.join("notices").join(format!("{}.txt", kind.name()));
        return fs::read_to_string(&path).map_err(|e| format!("{}: {e} (this install has no notices for {})", path.display(), kind.name()));
    }
    let lock = fs::read(home.root.join("Cargo.lock")).unwrap_or_default();
    let key = {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(&lock);
        h.update(format!("{}|{FORMAT}|{}", env!("CARGO_PKG_VERSION"), kind.name()));
        // (and this rapidr: a rebuilt generator makes the text again)
        if let Ok(t) = env::current_exe().and_then(fs::metadata).and_then(|m| m.modified()) {
            h.update(format!("{t:?}"));
        }
        h.finalize().iter().take(8).map(|b| format!("{b:02x}")).collect::<String>()
    };
    let target_dir = env::var_os("CARGO_TARGET_DIR").map(PathBuf::from).unwrap_or_else(|| home.root.join("target"));
    let cache = target_dir.join("notices").join(format!("{}-{key}.txt", kind.name()));
    if let Ok(text) = fs::read_to_string(&cache) {
        return Ok(text);
    }
    let text = generate(&home.root, kind)?;
    if fs::create_dir_all(cache.parent().unwrap()).is_ok() {
        let _ = fs::write(&cache, &text);
    }
    Ok(text)
}

/// Write `THIRD-PARTY-NOTICES.txt` for `kind` into `dir`.
pub fn write(dir: &Path, kind: &Kind) -> Result<PathBuf, String> {
    let text = text(kind)?;
    let path = dir.join(FILE_NAME);
    fs::write(&path, text).map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(path)
}

/// What to say when an output's notices can't be written: the output is
/// there, but must not be shipped without them.
pub fn missing(e: &str) -> String {
    format!("error: {FILE_NAME} could not be written: {e}\n  (the program was built, but don't distribute it without its notices: `rapidr notices` makes them)")
}

/// The HTML a web build's page carries: a comment for whoever reads the
/// source, and a `rel=license` link to the notices (invisible).
pub fn html_head_lines() -> String {
    format!(
        "  <!-- Built with RapidR (MIT). Third-party software notices and licences: {FILE_NAME} -->\n  <link rel=\"license\" href=\"{FILE_NAME}\">\n"
    )
}

// ---------------------------------------------------------------------------
// Generation (a checkout, or release time)
// ---------------------------------------------------------------------------

struct Package {
    name: String,
    version: String,
    license: Option<String>,
    license_file: Option<String>,
    dir: PathBuf,
    authors: Vec<String>,
    url: String,
}

/// One block of licence text, and whose it is.
struct Block {
    title: String,
    text: String,
}

/// A component in the list: a crate or one of the [`extras`].
struct Component {
    name: String,
    version: String,
    /// What it declares (`MIT OR Apache-2.0`).
    declared: String,
    /// What this file complies with (`MIT`).
    used: String,
    url: String,
    note: String,
    blocks: Vec<Block>,
}

/// The only licences a crate compiled into a program may be used under
/// (every kind of output: desktop executables, the web; RapidR's own tools
/// too): permissive, asking at most for a notice, which this file is. No
/// copyleft of any strength (GPL, LGPL, MPL, EPL, …), no data licences, no
/// advertising clauses. deny.toml's allowlist (the whole workspace, build
/// and dev tools included) is this list (a test keeps them equal). In the
/// order a choice among alternatives prefers them: the fewer obligations,
/// the earlier.
const ALLOWED: &[(&str, &str)] = &[
    ("MIT", include_str!("../licenses/MIT.txt")),
    ("Zlib", include_str!("../licenses/Zlib.txt")),
    ("ISC", include_str!("../licenses/ISC.txt")),
    ("BSD-2-Clause", include_str!("../licenses/BSD-2-Clause.txt")),
    ("BSD-3-Clause", include_str!("../licenses/BSD-3-Clause.txt")),
    ("0BSD", include_str!("../licenses/0BSD.txt")),
    ("Unlicense", include_str!("../licenses/Unlicense.txt")),
    ("CC0-1.0", include_str!("../licenses/CC0-1.0.txt")),
    ("BSL-1.0", include_str!("../licenses/BSL-1.0.txt")),
    ("Apache-2.0", include_str!("../licenses/Apache-2.0.txt")),
    ("Apache-2.0 WITH LLVM-exception", include_str!("../licenses/Apache-2.0-WITH-LLVM-exception.txt")),
    ("Unicode-3.0", include_str!("../licenses/Unicode-3.0.txt")),
];

/// What the components that aren't crates (extras) may be under: fonts
/// under the SIL Open Font License, public-domain code (SQLite), and the
/// permissive notices of data and protocol descriptions compiled into
/// crates (HPND-sell-variant and X11: MIT's kin, a notice and no
/// endorsement). Generation fails on any other.
const EXTRA_ALLOWED: &[&str] = &["MIT", "BSD-3-Clause", "OFL-1.1", "public domain", "HPND-sell-variant", "X11"];

/// Crates that must never be compiled into a program: what RapidR replaced
/// so nothing copyleft, cryptographic or data-licensed is shipped
/// (docs/licensing.md). Generation fails on any of them.
const BANNED: &[(&str, &str)] = &[
    ("ring", "cryptography compiled in: TLS is the system's (native-tls)"),
    ("rustls", "TLS compiled in: TLS is the system's (native-tls)"),
    ("aws-lc-rs", "cryptography compiled in: TLS is the system's (native-tls)"),
    ("aws-lc-sys", "cryptography compiled in: TLS is the system's (native-tls)"),
    ("openssl-src", "OpenSSL compiled in: Linux uses the system's libssl.so.3"),
    ("webpki-roots", "CDLA-licensed certificates: the system's are used"),
    ("symphonia", "MPL-2.0: MP3 is nanomp3's"),
    ("symphonia-core", "MPL-2.0: MP3 is nanomp3's"),
    ("symphonia-bundle-mp3", "MPL-2.0: MP3 is nanomp3's"),
    ("font-kit", "pulls MPL-2.0 crates and FreeType: charts draw with ab_glyph"),
    ("dwrote", "MPL-2.0"),
    ("option-ext", "MPL-2.0"),
    ("freetype-sys", "FreeType's licence asks for credit in the documentation"),
    ("wayland-protocols-plasma", "crates.io's is generated from LGPL-2.1-or-later KDE protocol files: RapidR's stand-in (crates/patches) must replace it"),
];

const RAPIDR_LICENSE: &str = include_str!("../../../LICENSE");
const OFL: &str = include_str!("../../rapidr-value/fonts/OFL-1.1.txt");
/// The Noto fallback fonts' licence (their copyright lines first).
const NOTO_OFL: &str = include_str!("../../../fonts/fallback/OFL.txt");
/// mingw-w64's notices, from the LLVM-MinGW release the Windows SDK ships.
const MINGW_RUNTIME: &str = include_str!("../licenses/MinGW-w64-runtime.txt");
const MINGW_COPYING: &str = include_str!("../licenses/MinGW-w64-COPYING.txt");

fn rank(id: &str) -> Option<usize> {
    ALLOWED.iter().position(|(a, _)| *a == id)
}

fn generate(root: &Path, kind: &Kind) -> Result<String, String> {
    let packages = metadata(root)?;
    let (roots, features) = kind.roots();
    let mut wanted: BTreeSet<(String, String)> = BTreeSet::new();
    // RapidR's own crates in the graph (path dependencies)
    let mut own: BTreeSet<String> = BTreeSet::new();
    for triple in kind.triples()? {
        let mut cmd = Command::new(home::rust_tool("cargo"));
        cmd.current_dir(root).args(["tree", "--quiet", "--locked", "-e", "normal", "--prefix", "none", "-f", "{p}", "--target", triple]);
        for r in roots {
            cmd.args(["-p", r]);
        }
        for f in features {
            cmd.args(["--features", f]);
        }
        let out = cmd.output().map_err(|e| format!("cargo tree: {e}"))?;
        if !out.status.success() {
            return Err(format!("cargo tree --target {triple}: {}", String::from_utf8_lossy(&out.stderr).trim()));
        }
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            let mut parts = line.split_whitespace();
            if let (Some(name), Some(version)) = (parts.next(), parts.next()) {
                // (a path dependency is one of RapidR's own crates)
                if line.contains(" (/") || line.contains(":\\") {
                    own.insert(name.to_string());
                } else {
                    wanted.insert((name.to_string(), version.trim_start_matches('v').to_string()));
                }
            }
        }
    }
    if let Some((name, why)) = BANNED.iter().find(|(b, _)| wanted.iter().any(|(n, _)| n == b)) {
        return Err(format!("{name} is in the graph of {}: {why} (docs/licensing.md)", kind.name()));
    }
    let clarified = clarifications(root);
    let mut components = Vec::new();
    for (name, version) in &wanted {
        let p = packages.get(&(name.clone(), version.clone())).ok_or_else(|| format!("{name} {version}: not in cargo metadata"))?;
        components.push(crate_component(p, &clarified)?);
    }
    let crate_names: BTreeSet<&str> = wanted.iter().map(|(n, _)| n.as_str()).chain(own.iter().map(String::as_str)).collect();
    let mut extra = extras(kind, &crate_names, &packages, &wanted)?;
    if let Some(c) = extra.iter().find(|c| c.used.split(" AND ").any(|l| l != "—" && !EXTRA_ALLOWED.contains(&l))) {
        return Err(format!("{}: licence '{}' isn't allowed in a program (notices.rs, EXTRA_ALLOWED)", c.name, c.used));
    }
    // RapidR and Rust first, then the rest by name
    let mut all: Vec<Component> = extra.drain(..2).collect();
    let mut rest: Vec<Component> = extra.into_iter().chain(components).collect();
    rest.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()).then(a.version.cmp(&b.version)));
    all.extend(rest);
    Ok(render(kind, &all))
}

/// `cargo metadata`: every package's licence, folder and links.
fn metadata(root: &Path) -> Result<HashMap<(String, String), Package>, String> {
    let out = Command::new(home::rust_tool("cargo"))
        .current_dir(root)
        .args(["metadata", "--format-version", "1", "--locked"])
        .output()
        .map_err(|e| format!("cargo metadata: {e}"))?;
    if !out.status.success() {
        return Err(format!("cargo metadata: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    let meta: serde_json::Value = serde_json::from_slice(&out.stdout).map_err(|e| format!("cargo metadata: {e}"))?;
    let mut map = HashMap::new();
    for p in meta["packages"].as_array().into_iter().flatten() {
        // (RapidR's own crates are path dependencies: no source)
        if p["source"].is_null() {
            continue;
        }
        let s = |k: &str| p[k].as_str().map(str::to_string);
        let name = s("name").unwrap_or_default();
        let version = s("version").unwrap_or_default();
        let dir = PathBuf::from(s("manifest_path").unwrap_or_default()).parent().map(Path::to_path_buf).unwrap_or_default();
        let url = s("repository").or_else(|| s("homepage")).unwrap_or_else(|| format!("https://crates.io/crates/{name}"));
        let authors = p["authors"].as_array().into_iter().flatten().filter_map(|a| a.as_str().map(str::to_string)).collect();
        map.insert((name.clone(), version.clone()), Package { name, version, license: s("license"), license_file: s("license_file"), dir, authors, url });
    }
    Ok(map)
}

/// The licence expressions deny.toml records for crates that declare none
/// (only a licence file): `cargo deny` and these notices agree.
fn clarifications(root: &Path) -> HashMap<String, String> {
    let text = fs::read_to_string(root.join("deny.toml")).unwrap_or_default();
    let mut out = HashMap::new();
    let mut krate = None;
    for line in text.lines() {
        let line = line.trim();
        let value = |l: &str| l.split_once('=').map(|(_, v)| v.trim().trim_matches('"').to_string());
        if line.starts_with("crate") {
            krate = value(line);
        } else if line.starts_with("expression") {
            if let (Some(c), Some(e)) = (krate.take(), value(line)) {
                out.insert(c, e);
            }
        }
    }
    out
}

// --- licence expressions ---------------------------------------------------

enum Expr {
    Id(String),
    And(Vec<Expr>),
    Or(Vec<Expr>),
}

/// An SPDX expression (`(MIT OR Apache-2.0) AND Unicode-3.0`, and the old
/// `MIT/Apache-2.0`).
fn parse_expr(text: &str) -> Result<Expr, String> {
    let spaced = text.replace('/', " OR ").replace('(', " ( ").replace(')', " ) ");
    let tokens: Vec<&str> = spaced.split_whitespace().collect();
    let mut pos = 0;
    let e = parse_or(&tokens, &mut pos)?;
    if pos != tokens.len() {
        return Err(format!("licence expression '{text}': unexpected '{}'", tokens[pos]));
    }
    Ok(e)
}

fn parse_or(t: &[&str], pos: &mut usize) -> Result<Expr, String> {
    let mut alts = vec![parse_and(t, pos)?];
    while t.get(*pos) == Some(&"OR") {
        *pos += 1;
        alts.push(parse_and(t, pos)?);
    }
    Ok(if alts.len() == 1 { alts.pop().unwrap() } else { Expr::Or(alts) })
}

fn parse_and(t: &[&str], pos: &mut usize) -> Result<Expr, String> {
    let mut all = vec![parse_atom(t, pos)?];
    while t.get(*pos) == Some(&"AND") {
        *pos += 1;
        all.push(parse_atom(t, pos)?);
    }
    Ok(if all.len() == 1 { all.pop().unwrap() } else { Expr::And(all) })
}

fn parse_atom(t: &[&str], pos: &mut usize) -> Result<Expr, String> {
    match t.get(*pos) {
        Some(&"(") => {
            *pos += 1;
            let e = parse_or(t, pos)?;
            if t.get(*pos) != Some(&")") {
                return Err("licence expression: missing ')'".into());
            }
            *pos += 1;
            Ok(e)
        }
        Some(id) if !["AND", "OR", "WITH", ")"].contains(id) => {
            *pos += 1;
            let mut id = id.to_string();
            if t.get(*pos) == Some(&"WITH") {
                let exception = t.get(*pos + 1).ok_or("licence expression: WITH what?")?;
                id = format!("{id} WITH {exception}");
                *pos += 2;
            }
            Ok(Expr::Id(id))
        }
        other => Err(format!("licence expression: unexpected {other:?}")),
    }
}

/// The licences to comply with: all of an AND; of an OR, the alternative
/// whose licence texts the crate ships, then the one with the fewest
/// obligations. A licence outside [`ALLOWED`] is an error.
fn choose(e: &Expr, shipped: &BTreeSet<&str>) -> Result<Vec<String>, String> {
    match e {
        Expr::Id(id) => {
            rank(id).ok_or_else(|| format!("licence '{id}' isn't allowed (deny.toml)"))?;
            Ok(vec![id.clone()])
        }
        Expr::And(all) => {
            let mut out = Vec::new();
            for a in all {
                for id in choose(a, shipped)? {
                    if !out.contains(&id) {
                        out.push(id);
                    }
                }
            }
            Ok(out)
        }
        Expr::Or(alts) => {
            let mut best: Option<(usize, Vec<String>)> = None;
            let mut last_err = None;
            for a in alts {
                match choose(a, shipped) {
                    Ok(ids) => {
                        let missing = ids.iter().filter(|id| !shipped.contains(id.as_str())).count();
                        let score = missing * 1000 + ids.iter().filter_map(|id| rank(id)).sum::<usize>();
                        if best.as_ref().is_none_or(|(s, _)| score < *s) {
                            best = Some((score, ids));
                        }
                    }
                    Err(e) => last_err = Some(e),
                }
            }
            best.map(|(_, ids)| ids).ok_or_else(|| last_err.unwrap_or_default())
        }
    }
}

// --- licence files -----------------------------------------------------------

/// A licence-looking file name: LICENSE*, LICENCE*, COPYING*, COPYRIGHT*,
/// NOTICE*, UNLICENSE, FTL.TXT (FreeType's).
fn is_licence_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    let code = [".rs", ".py", ".c", ".h", ".sh", ".toml", ".json", ".html", ".js", ".in"];
    ["licen", "copying", "copyright", "notice", "unlicense", "ftl.txt"].iter().any(|p| lower.starts_with(p)) && !code.iter().any(|c| lower.ends_with(c))
}

/// Folders inside a crate whose licence files aren't about compiled code.
const SKIP_DIRS: &[&str] = &["tests", "test", "testdata", "testsuite", "examples", "benches", "fixtures", "autotests", "target", ".git", "sqlcipher", "sqlite3mc", "docs-src"];

/// What licence a file holds, by its name, else by its words; `None` when
/// it can't tell (several licences, or a notice): such files are kept.
fn classify(name: &str, text: &str) -> Option<&'static str> {
    let upper = name.to_ascii_uppercase();
    let by_name = [
        ("LLVM", "Apache-2.0 WITH LLVM-exception"),
        ("APACHE", "Apache-2.0"),
        ("MIT", "MIT"),
        ("ZLIB", "Zlib"),
        ("BOOST", "BSL-1.0"),
        ("BSL", "BSL-1.0"),
        ("UNLICENSE", "Unlicense"),
        ("0BSD", "0BSD"),
        ("ISC", "ISC"),
        ("UNICODE", "Unicode-3.0"),
        ("CC0", "CC0-1.0"),
    ];
    if let Some((_, id)) = by_name.iter().find(|(k, _)| upper.contains(k)) {
        return Some(id);
    }
    if upper.starts_with("NOTICE") || upper.starts_with("COPYRIGHT") {
        return None;
    }
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let has = |s: &str| flat.contains(s);
    let mut found = Vec::new();
    if has("Apache License") && has("Version 2.0") {
        found.push(if has("LLVM Exceptions") { "Apache-2.0 WITH LLVM-exception" } else { "Apache-2.0" });
    }
    if has("Permission is hereby granted, free of charge") && !has("Boost Software License") && !has("UNICODE LICENSE") {
        found.push("MIT");
    }
    if has("Redistribution and use in source and binary forms") {
        found.push(if has("Neither the name") { "BSD-3-Clause" } else { "BSD-2-Clause" });
    }
    if has("Permission to use, copy, modify, and/or distribute this software for any purpose with or without fee is hereby granted") {
        found.push(if has("provided that the above copyright notice") { "ISC" } else { "0BSD" });
    }
    if has("This software is provided 'as-is'") {
        found.push("Zlib");
    }
    if has("Boost Software License") {
        found.push("BSL-1.0");
    }
    if has("This is free and unencumbered software") {
        found.push("Unlicense");
    }
    if has("UNICODE LICENSE V3") {
        found.push("Unicode-3.0");
    }
    if has("CC0 1.0 Universal") {
        found.push("CC0-1.0");
    }
    (found.len() == 1).then(|| found[0])
}

/// A crate's licence files: (path relative to the crate, text). The crate's
/// own (its folder, and `license-file`), then licences of code bundled in
/// it (subfolders: a C library, a data file).
fn licence_files(p: &Package) -> Vec<(String, String, bool)> {
    let mut out: Vec<(String, String, bool)> = Vec::new();
    let mut add = |rel: String, path: &Path, bundled: bool| {
        if out.iter().any(|(r, _, _)| *r == rel) {
            return;
        }
        if let Ok(bytes) = fs::read(path) {
            out.push((rel, String::from_utf8_lossy(&bytes).replace("\r\n", "\n"), bundled));
        }
    };
    let mut top: Vec<_> = fs::read_dir(&p.dir).into_iter().flatten().flatten().filter(|e| e.path().is_file()).collect();
    top.sort_by_key(|e| e.file_name());
    for e in top {
        let name = e.file_name().to_string_lossy().into_owned();
        if is_licence_name(&name) {
            add(name, &e.path(), false);
        }
    }
    if let Some(f) = &p.license_file {
        let path = p.dir.join(f);
        add(f.replace('\\', "/"), &path, false);
    }
    let mut stack = vec![(p.dir.clone(), String::new(), 0)];
    while let Some((dir, rel, depth)) = stack.pop() {
        let mut entries: Vec<_> = fs::read_dir(&dir).into_iter().flatten().flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            let name = e.file_name().to_string_lossy().into_owned();
            let sub = if rel.is_empty() { name.clone() } else { format!("{rel}/{name}") };
            if e.path().is_dir() {
                if depth < 4 && !SKIP_DIRS.contains(&name.as_str()) {
                    stack.push((e.path(), sub, depth + 1));
                }
            } else if depth > 0 && is_licence_name(&name) {
                add(sub, &e.path(), true);
            }
        }
    }
    out.sort_by(|a, b| a.2.cmp(&b.2).then(a.0.cmp(&b.0)));
    out
}

fn copyright_line(p: &Package) -> String {
    if p.authors.is_empty() {
        format!("Copyright (c) the {} developers ({})", p.name, p.url)
    } else {
        format!("Copyright (c) {}", p.authors.join(", "))
    }
}

fn crate_component(p: &Package, clarified: &HashMap<String, String>) -> Result<Component, String> {
    let declared = p.license.clone().or_else(|| clarified.get(&p.name).cloned()).ok_or_else(|| format!("{} {}: declares no licence (add a [[licenses.clarify]] to deny.toml)", p.name, p.version))?;
    let files = licence_files(p);
    let classes: Vec<Option<&str>> = files.iter().map(|(rel, text, _)| classify(rel.rsplit('/').next().unwrap_or(rel), text)).collect();
    let shipped: BTreeSet<&str> = files.iter().zip(&classes).filter(|((_, _, bundled), _)| !bundled).filter_map(|(_, c)| *c).collect();
    let expr = parse_expr(&declared).map_err(|e| format!("{} {}: {e}", p.name, p.version))?;
    let used = choose(&expr, &shipped).map_err(|e| format!("{} {}: {e}", p.name, p.version))?;
    let mut blocks = Vec::new();
    for ((rel, text, bundled), class) in files.iter().zip(&classes) {
        // the crate's own files for the licences it is used under, every
        // NOTICE and unclassified notice, and everything bundled
        let keep = *bundled || class.is_none_or(|c| used.iter().any(|u| u == c));
        if keep && !text.trim().is_empty() {
            blocks.push(Block { title: format!("{} {} — {rel}", p.name, p.version), text: text.clone() });
        }
    }
    // a licence it is used under whose text it doesn't ship: the standard
    // text, with the authors as the copyright holders
    for id in &used {
        let has_text = files.iter().zip(&classes).any(|((_, _, bundled), c)| !bundled && *c == Some(id.as_str()));
        if !has_text {
            let template = ALLOWED.iter().find(|(a, _)| a == id).map(|(_, t)| *t).unwrap_or("");
            blocks.push(Block { title: format!("{} {} — {id} (the crate ships no licence file for it)", p.name, p.version), text: format!("{}\n\n{template}", copyright_line(p)) });
        }
    }
    Ok(Component { name: p.name.clone(), version: p.version.clone(), declared, used: used.join(" AND "), url: p.url.clone(), note: String::new(), blocks })
}

// --- what isn't a crate ----------------------------------------------------

fn mit_with(copyright: &str) -> String {
    format!("{copyright}\n\n{}", ALLOWED[0].1)
}

/// The components that aren't crates in the graph: RapidR itself and Rust's
/// standard library (always; first), then by what the graph contains and the
/// OS: the built-in fonts, C code a crate bundles without its licence file,
/// the toolchain's start-up code and the system's libraries.
fn extras(kind: &Kind, crates: &BTreeSet<&str>, packages: &HashMap<(String, String), Package>, wanted: &BTreeSet<(String, String)>) -> Result<Vec<Component>, String> {
    let dir_of = |name: &str| wanted.iter().find(|(n, _)| n == name).and_then(|k| packages.get(k)).map(|p| p.dir.clone());
    let rust = Command::new(home::rust_tool("rustc")).arg("--version").output().ok().filter(|o| o.status.success()).map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default();
    let mut out = vec![
        Component {
            name: "RapidR (its runtime and libraries)".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            declared: "MIT".into(),
            used: "MIT".into(),
            url: "https://github.com/iBobX/RapidR".into(),
            note: "RapidR is not affiliated with RapidQ, its author, or any vendor named in its documentation; see LEGAL.md in RapidR.".into(),
            blocks: vec![Block { title: "RapidR — LICENSE".into(), text: RAPIDR_LICENSE.into() }],
        },
        Component {
            name: "Rust standard library".into(),
            version: rust.strip_prefix("rustc ").unwrap_or("").split_whitespace().next().unwrap_or("").into(),
            declared: "MIT OR Apache-2.0".into(),
            used: "MIT".into(),
            url: "https://github.com/rust-lang/rust".into(),
            note: "core, alloc, std and the crates they are built from (compiler_builtins — with code ported from LLVM's compiler-rt, Apache-2.0 WITH LLVM-exception — hashbrown, libc, memchr, addr2line, gimli, object, miniz_oxide, adler2, rustc-demangle, cfg-if, unwind; dlmalloc on the web): all available under MIT.".into(),
            blocks: vec![Block { title: "Rust standard library — MIT".into(), text: mit_with("Copyright (c) The Rust Project Contributors") }],
        },
    ];
    if crates.contains("rapidr-value") {
        out.push(Component {
            name: "Liberation fonts (Sans, Serif, Mono)".into(),
            version: "2.1.5".into(),
            declared: "OFL-1.1".into(),
            used: "OFL-1.1".into(),
            url: "https://github.com/liberationfonts/liberation-fonts".into(),
            note: "Built into the program unmodified. Reserved Font Names: Liberation (and Arimo, Tinos, Cousine). The OFL lets the fonts be bundled with any software, commercial included; the fonts themselves may not be sold on their own.".into(),
            blocks: vec![Block { title: "Liberation fonts — OFL-1.1".into(), text: OFL.into() }],
        });
    }
    // (the web runtime's fallback fonts: shipped beside it, fonts/)
    if matches!(kind, Kind::Web) {
        out.push(Component {
            name: "Noto fallback fonts (Noto Sans, Noto Sans Symbols, Noto Sans Symbols 2, Noto Sans SC, Noto Sans KR)".into(),
            version: "2.015 / 2.003 / 2.008 / CJK Sans 2.004".into(),
            declared: "OFL-1.1".into(),
            used: "OFL-1.1".into(),
            url: "https://github.com/notofonts".into(),
            note: "Shipped beside the web runtime (fonts/) as subsets split by Unicode range, loaded as the page's text needs them; each CJK chunk is renamed \"<family> NNN\". None of these fonts declares a Reserved Font Name. The OFL lets the fonts be bundled with any software, commercial included; the fonts themselves may not be sold on their own.".into(),
            blocks: vec![Block { title: "Noto fonts — OFL-1.1".into(), text: NOTO_OFL.into() }],
        });
    }
    for (krate, header) in [("libsqlite3-sys", "sqlite3/sqlite3.h"), ("sqlite-wasm-rs", "sqlite3/sqlite3.h")] {
        if let Some(dir) = dir_of(krate) {
            let h = fs::read_to_string(dir.join(header)).unwrap_or_default();
            let version = h.lines().find_map(|l| l.strip_prefix("#define SQLITE_VERSION ")).map(|v| v.trim().trim_matches('"').to_string()).unwrap_or_default();
            let blessing: Vec<&str> = h.lines().skip_while(|l| !l.contains("The author disclaims copyright")).take_while(|l| !l.trim_start().starts_with("***") && !l.contains("*/")).collect();
            out.push(Component {
                name: format!("SQLite (via {krate})"),
                version,
                declared: "public domain".into(),
                used: "public domain".into(),
                url: "https://www.sqlite.org/copyright.html".into(),
                note: "SQLite's source code is in the public domain: no licence or notice is required.".into(),
                blocks: if blessing.is_empty() { vec![] } else { vec![Block { title: "SQLite — sqlite3.h".into(), text: blessing.iter().map(|l| l.trim_start_matches(['*', ' '])).collect::<Vec<_>>().join("\n") }] },
            });
        }
    }
    if crates.contains("sqlite-wasm-rs") {
        out.push(Component {
            name: "musl libc (the C functions SQLite needs, via sqlite-wasm-rs)".into(),
            version: String::new(),
            declared: "MIT".into(),
            used: "MIT".into(),
            url: "https://musl.libc.org".into(),
            note: String::new(),
            blocks: vec![Block { title: "musl libc — MIT".into(), text: mit_with("Copyright © 2005-2020 Rich Felker, et al.") }],
        });
        out.push(Component {
            name: "printf (Eyal Rozenberg, Marco Paland; via sqlite-wasm-rs)".into(),
            version: String::new(),
            declared: "MIT".into(),
            used: "MIT".into(),
            url: "https://github.com/eyalroz/printf".into(),
            note: String::new(),
            blocks: vec![Block { title: "printf — MIT".into(), text: mit_with("Copyright (c) 2014-2019 Marco Paland\nCopyright (c) 2021-2024 Eyal Rozenberg") }],
        });
    }
    if let Some(dir) = dir_of("sctk-adwaita") {
        // (winit's Wayland title bars, drawn by sctk-adwaita: its fallback
        // face, built in; the crate ships no text for it)
        if dir.join("src/title/Cantarell-Regular.ttf").is_file() {
            let body = OFL.split_once("SIL OPEN FONT LICENSE Version 1.1").map(|(_, b)| format!("SIL OPEN FONT LICENSE Version 1.1{b}")).unwrap_or_else(|| OFL.into());
            out.push(Component {
                name: "Cantarell Regular (via sctk-adwaita, Wayland title bars)".into(),
                version: "0.0.5".into(),
                declared: "OFL-1.1".into(),
                used: "OFL-1.1".into(),
                url: "https://gitlab.gnome.org/GNOME/cantarell-fonts".into(),
                note: "Built into the program unmodified, for the title of a window on a Wayland desktop that has the program draw its own (GNOME).".into(),
                blocks: vec![Block { title: "Cantarell — OFL-1.1".into(), text: format!("{CANTARELL_COPYRIGHT}\n\nThis Font Software is licensed under the SIL Open Font License, Version 1.1.\nThis license is copied below, and is also available with a FAQ at:\nhttp://scripts.sil.org/OFL\n\n{body}") }],
            });
        }
    }
    let protocols = wayland_protocol_notices(crates, &dir_of)?;
    if !protocols.is_empty() {
        let mut used: Vec<&str> = protocols.iter().map(|(l, _, _)| *l).collect();
        used.sort();
        used.dedup();
        out.push(Component {
            name: "Wayland protocol descriptions (in wayland-client, wayland-protocols, wayland-protocols-wlr)".into(),
            version: String::new(),
            declared: used.join(" AND "),
            used: used.join(" AND "),
            url: "https://gitlab.freedesktop.org/wayland".into(),
            note: "Those crates' Rust code is generated from these XML protocol descriptions; each description's notice is below.".into(),
            blocks: protocols.into_iter().map(|(_, files, text)| Block { title: format!("Wayland protocols — {}", files.join(", ")), text }).collect(),
        });
    }
    if crates.contains("x11rb-protocol") {
        out.push(Component {
            name: "xcb-proto (the X11 protocol descriptions x11rb-protocol is generated from)".into(),
            version: "1.17.0".into(),
            declared: "X11".into(),
            used: "X11".into(),
            url: "https://gitlab.freedesktop.org/xorg/proto/xcbproto".into(),
            note: String::new(),
            blocks: vec![Block { title: "xcb-proto — COPYING".into(), text: XCB_PROTO.into() }],
        });
    }
    if let Some(dir) = dir_of("read-fonts") {
        // (its table of glyph names, compiled in from Adobe's list)
        let list = fs::read_to_string(dir.join("data/glyphlist.txt")).unwrap_or_default();
        let header: Vec<&str> = list.lines().take_while(|l| l.starts_with('#')).map(|l| l.trim_start_matches('#').trim_start_matches(' ')).filter(|l| !l.starts_with("---")).collect();
        let text = header.split(|l| l.starts_with("Name:")).next().unwrap_or(&[]).join("\n").trim().to_string();
        if !text.contains("Redistribution and use in source and binary forms") {
            return Err(format!("read-fonts: data/glyphlist.txt's notice wasn't found ({})", dir.display()));
        }
        out.push(Component {
            name: "Adobe Glyph List (via read-fonts)".into(),
            version: "2.0".into(),
            declared: "BSD-3-Clause".into(),
            used: "BSD-3-Clause".into(),
            url: "https://github.com/adobe-type-tools/agl-aglfn".into(),
            note: String::new(),
            blocks: vec![Block { title: "Adobe Glyph List — glyphlist.txt".into(), text }],
        });
    }
    if kind.os() == "windows" {
        // What LLVM-MinGW (the SDK's toolchain, the *-pc-windows-gnullvm
        // targets) links into a program: its texts, from the llvm-mingw
        // release the SDK ships (crates/rapidr-cli/licenses/MinGW-w64-*.txt).
        out.push(Component {
            name: "mingw-w64 runtime (LLVM-MinGW: *-pc-windows-gnullvm builds)".into(),
            version: String::new(),
            declared: "ZPL-2.1, with BSD-, ISC- and MIT-style and public-domain parts".into(),
            used: "ZPL-2.1 and the parts' own terms".into(),
            url: "https://www.mingw-w64.org".into(),
            note: "Its start-up objects and run-time library (crt2.o, libmingw32, libmingwex: gdtoa, getopt, parts of the math library, …) are linked statically into programs built with the gnullvm targets — RapidR's own Windows executables and native builds made with the RapidR SDK. Its licence asks for these notices to go with the program in binary form. (A few mingw-w64 headers imported from Wine are LGPL-2.1-or-later; they are only compiled against, never linked.)".into(),
            blocks: vec![
                Block { title: "mingw-w64 — COPYING.MinGW-w64-runtime.txt".into(), text: MINGW_RUNTIME.into() },
                Block { title: "mingw-w64 — COPYING (ZPL-2.1)".into(), text: MINGW_COPYING.into() },
            ],
        });
        out.push(Component {
            name: "LLVM compiler-rt (builtins) and libunwind (LLVM-MinGW: *-pc-windows-gnullvm builds)".into(),
            version: String::new(),
            declared: "Apache-2.0 WITH LLVM-exception".into(),
            used: "Apache-2.0 WITH LLVM-exception".into(),
            url: "https://llvm.org".into(),
            note: "Linked statically into programs built with the gnullvm targets (in place of GCC's libgcc: no GPL code is linked). The LLVM exception lets this embedded code be shipped without its notices; they are here all the same.".into(),
            blocks: vec![Block { title: "LLVM — Apache-2.0 WITH LLVM-exception".into(), text: ALLOWED.iter().find(|(id, _)| *id == "Apache-2.0 WITH LLVM-exception").map(|(_, t)| *t).unwrap_or_default().into() }],
        });
    }
    let system = match kind.os() {
        "macos" => Some("macOS's libraries and frameworks (libSystem, AppKit, Metal, Core Audio, Core MIDI, IOKit, Security for HTTPS, …) are part of the operating system: linked dynamically, not part of this program, not distributed with it."),
        "windows" => Some("Windows' DLLs (kernel32, user32, the Universal C Runtime, SChannel for HTTPS, …) are part of the operating system: linked dynamically, not distributed with this program. The start-up and run-time support code the toolchain links in: with the *-pc-windows-gnullvm targets (LLVM-MinGW), LLVM's compiler-rt and libunwind (Apache-2.0 WITH LLVM-exception, which lets that embedded code be shipped without notices) and the mingw-w64 runtime's start-up objects (the mingw-w64 runtime licence, COPYING.MinGW-w64-runtime.txt in LLVM-MinGW); with *-pc-windows-msvc, Microsoft's C runtime start-up code (and, with +crt-static, as in RapidR's release builds, the Visual C++ runtime), distributed under the Visual Studio licence's terms for its distributable code."),
        "linux" => Some("The GNU C library (glibc), libgcc_s, OpenSSL 3 (libssl.so.3 and libcrypto.so.3, for HTTPS; Apache-2.0), ALSA's libasound, fontconfig, X11, Wayland and xkbcommon are the system's libraries: linked or loaded dynamically from the user's system, not part of this program, not distributed with it (glibc and libasound are LGPL-2.1-or-later, which puts no conditions on a program that only uses the system's shared copies). The start-up files linked into every Linux program (glibc's crt1.o / crti.o, GCC's crtbegin.o) carry licence exceptions for exactly this use."),
        "web" => Some("The program runs in the visitor's web browser. Its JavaScript glue is generated by wasm-bindgen (listed here); the rest of the page (index.html, loader.js, the console) is RapidR's (MIT)."),
        _ => None,
    };
    if let Some(note) = system {
        out.push(Component { name: "System and toolchain".into(), version: String::new(), declared: "—".into(), used: "—".into(), url: String::new(), note: note.into(), blocks: vec![] });
    }
    Ok(out)
}

/// The copyright lines of sctk-adwaita's Cantarell-Regular.ttf (its name
/// table's notice; the rest of that notice is the OFL's preamble).
const CANTARELL_COPYRIGHT: &str = "Copyright (c) 2009-2011, Understanding Limited (dave@understandinglimited.com),\nCopyright (c) 2010-2011, Jakub Steiner (jimmac@gmail.com).";

/// xcb-proto's COPYING (x11rb-protocol is generated from xcb-proto; the
/// crate doesn't ship this text).
const XCB_PROTO: &str = "Copyright (C) 2001-2006 Bart Massey, Jamey Sharp, and Josh Triplett.
All Rights Reserved.

Permission is hereby granted, free of charge, to any person
obtaining a copy of this software and associated
documentation files (the \"Software\"), to deal in the
Software without restriction, including without limitation
the rights to use, copy, modify, merge, publish, distribute,
sublicense, and/or sell copies of the Software, and to
permit persons to whom the Software is furnished to do so,
subject to the following conditions:

The above copyright notice and this permission notice shall
be included in all copies or substantial portions of the
Software.

THE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY
KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE
WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR
PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS
BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR
OTHER DEALINGS IN THE SOFTWARE.

Except as contained in this notice, the names of the authors
or their institutions shall not be used in advertising or
otherwise to promote the sale, use or other dealings in this
Software without prior written authorization from the
authors.";

/// The crates whose code is generated from Wayland protocol descriptions
/// (XML files they ship; crates.io's wayland-protocols-plasma is BANNED).
const WAYLAND_PROTOCOL_CRATES: &[&str] = &["wayland-client", "wayland-protocols", "wayland-protocols-wlr", "wayland-protocols-misc", "wayland-protocols-experimental"];

/// The notices of the protocol descriptions those crates generate code
/// from: (licence, the files, the notice), one per distinct notice. Each
/// must be MIT or HPND-sell-variant (MIT's older X11 kin: a notice, and the
/// authors' names not used to promote the product); any other — KDE's
/// LGPL-2.1-or-later files, say — is an error.
fn wayland_protocol_notices(crates: &BTreeSet<&str>, dir_of: &dyn Fn(&str) -> Option<PathBuf>) -> Result<Vec<(&'static str, Vec<String>, String)>, String> {
    let mut out: Vec<(&'static str, Vec<String>, String)> = Vec::new();
    for krate in WAYLAND_PROTOCOL_CRATES.iter().filter(|c| crates.contains(*c)) {
        let Some(dir) = dir_of(krate) else { continue };
        let mut files = Vec::new();
        let mut stack = vec![dir.clone()];
        while let Some(d) = stack.pop() {
            for e in fs::read_dir(&d).into_iter().flatten().flatten() {
                let path = e.path();
                let name = e.file_name().to_string_lossy().into_owned();
                if path.is_dir() && !SKIP_DIRS.contains(&name.as_str()) {
                    stack.push(path);
                } else if name.ends_with(".xml") {
                    files.push(path);
                }
            }
        }
        files.sort();
        for path in files {
            let xml = fs::read_to_string(&path).unwrap_or_default();
            let rel = format!("{krate}/{}", path.strip_prefix(&dir).unwrap_or(&path).to_string_lossy().replace('\\', "/"));
            let Some(body) = xml.split_once("<copyright>").and_then(|(_, r)| r.split_once("</copyright>")).map(|(c, _)| c) else {
                return Err(format!("{rel}: a Wayland protocol description without a <copyright> notice"));
            };
            let lines: Vec<&str> = body.trim_matches('\n').lines().collect();
            let indent = lines.iter().filter(|l| !l.trim().is_empty()).map(|l| l.len() - l.trim_start().len()).min().unwrap_or(0);
            let text = lines.iter().map(|l| l.get(indent..).unwrap_or("").trim_end()).collect::<Vec<_>>().join("\n").trim().to_string();
            let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
            let licence = if flat.contains("Permission is hereby granted, free of charge") {
                "MIT"
            } else if flat.contains("Permission to use, copy, modify, distribute, and sell this software and its documentation for any purpose is hereby granted without fee") {
                "HPND-sell-variant"
            } else {
                return Err(format!("{rel}: its licence isn't MIT or HPND-sell-variant — it can't be compiled into a program (notices.rs)"));
            };
            match out.iter_mut().find(|(_, _, t)| *t == text) {
                Some((_, names, _)) => names.push(rel),
                None => out.push((licence, vec![rel], text)),
            }
        }
    }
    Ok(out)
}

// --- the text ----------------------------------------------------------------

fn wrap(text: &str, width: usize, indent: &str) -> String {
    let mut out = String::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() && line.len() + 1 + word.len() > width {
            out.push_str(indent);
            out.push_str(&line);
            out.push('\n');
            line.clear();
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        out.push_str(indent);
        out.push_str(&line);
        out.push('\n');
    }
    out
}

fn render(kind: &Kind, components: &[Component]) -> String {
    let rule = "=".repeat(78);
    let mut s = String::new();
    s.push_str("THIRD-PARTY SOFTWARE NOTICES AND LICENCES\n");
    s.push_str(&rule);
    s.push('\n');
    s.push_str(&wrap(
        &format!(
            "This program was built with RapidR {} (https://github.com/iBobX/RapidR). It contains RapidR's runtime and the open-source components listed below, whose licences ask that their copyright notices and licence texts go with the program: they are all in this file. Keep it with the program when you share or sell it (beside the executable, or in the web build's folder); that is all their licences require. None of them restricts commercial or closed-source use.",
            env!("CARGO_PKG_VERSION")
        ),
        78,
        "",
    ));
    s.push('\n');
    s.push_str(&wrap(&format!("Covers: {}. Generated by RapidR from the build's dependency graph.", kind.describe()), 78, ""));
    s.push('\n');

    // blocks, deduplicated by their text
    let mut texts: Vec<(String, Vec<String>)> = Vec::new();
    let mut index_of: BTreeMap<String, usize> = BTreeMap::new();
    let mut refs: Vec<Vec<usize>> = Vec::new();
    for c in components {
        let mut mine = Vec::new();
        for b in &c.blocks {
            let norm = b.text.lines().map(str::trim_end).collect::<Vec<_>>().join("\n").trim().to_string();
            let i = *index_of.entry(norm.clone()).or_insert_with(|| {
                texts.push((norm, Vec::new()));
                texts.len() - 1
            });
            texts[i].1.push(b.title.clone());
            if !mine.contains(&(i + 1)) {
                mine.push(i + 1);
            }
        }
        refs.push(mine);
    }

    s.push_str(&format!("PART 1. COMPONENTS ({})\n{}\n", components.len(), "-".repeat(78)));
    for (c, r) in components.iter().zip(&refs) {
        let version = if c.version.is_empty() { String::new() } else { format!(" {}", c.version) };
        s.push_str(&format!("{}{version}\n", c.name));
        let mut lic = if c.declared == c.used { format!("Licence: {}", c.declared) } else { format!("Licence: {} (used under {})", c.declared, c.used) };
        if !r.is_empty() {
            lic.push_str(&format!(" — texts: {}", r.iter().map(|i| format!("[{i}]")).collect::<Vec<_>>().join(" ")));
        }
        s.push_str(&wrap(&lic, 74, "    "));
        if !c.url.is_empty() {
            s.push_str(&format!("    {}\n", c.url));
        }
        if !c.note.is_empty() {
            s.push_str(&wrap(&c.note, 74, "    "));
        }
    }
    s.push('\n');
    s.push_str(&format!("PART 2. LICENCE TEXTS AND NOTICES ({})\n{}\n", texts.len(), "-".repeat(78)));
    for (i, (text, titles)) in texts.iter().enumerate() {
        s.push('\n');
        s.push_str(&format!("[{}] {}\n", i + 1, titles[0]));
        if titles.len() > 1 {
            s.push_str(&wrap(&format!("(the same text for: {})", titles[1..].join("; ")), 78, ""));
        }
        s.push_str(&"-".repeat(78));
        s.push('\n');
        s.push_str(text);
        s.push('\n');
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(expr: &str, shipped: &[&str]) -> String {
        let shipped: BTreeSet<&str> = shipped.iter().copied().collect();
        choose(&parse_expr(expr).unwrap(), &shipped).unwrap().join(" AND ")
    }

    #[test]
    fn chooses_the_fewest_obligations_among_shipped_texts() {
        assert_eq!(ids("MIT OR Apache-2.0", &["MIT", "Apache-2.0"]), "MIT");
        assert_eq!(ids("MIT/Apache-2.0", &["Apache-2.0"]), "Apache-2.0");
        assert_eq!(ids("Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT", &[]), "MIT");
        assert_eq!(ids("(MIT OR Apache-2.0) AND Unicode-3.0", &["MIT", "Unicode-3.0"]), "MIT AND Unicode-3.0");
        assert_eq!(ids("Apache-2.0 AND ISC", &[]), "Apache-2.0 AND ISC");
        assert!(choose(&parse_expr("GPL-3.0").unwrap(), &BTreeSet::new()).is_err());
        assert!(choose(&parse_expr("MPL-2.0").unwrap(), &BTreeSet::new()).is_err());
        assert!(choose(&parse_expr("LGPL-2.1-or-later").unwrap(), &BTreeSet::new()).is_err());
        assert!(choose(&parse_expr("CDLA-Permissive-2.0").unwrap(), &BTreeSet::new()).is_err());
        assert_eq!(ids("GPL-2.0 OR MIT", &[]), "MIT");
    }

    #[test]
    fn classifies_licence_files() {
        assert_eq!(classify("LICENSE-MIT", ""), Some("MIT"));
        assert_eq!(classify("LICENSE", include_str!("../licenses/Apache-2.0.txt")), Some("Apache-2.0"));
        assert_eq!(classify("LICENSE", include_str!("../licenses/CC0-1.0.txt")), Some("CC0-1.0"));
        assert_eq!(classify("LICENSE", "Mozilla Public License Version 2.0"), None);
        assert_eq!(classify("LICENSE", include_str!("../licenses/BSD-3-Clause.txt")), Some("BSD-3-Clause"));
        assert_eq!(classify("COPYRIGHT", "Copyrights are retained by contributors"), None);
    }

    /// What a program may contain is exactly the permissive list
    /// (docs/licensing.md, LEGAL.md): nothing else creeps in.
    #[test]
    fn allowed_is_the_permissive_list() {
        let mut ours: Vec<&str> = ALLOWED.iter().map(|(id, _)| *id).collect();
        ours.sort();
        let mut list = vec!["MIT", "Apache-2.0", "Apache-2.0 WITH LLVM-exception", "BSD-2-Clause", "BSD-3-Clause", "ISC", "Zlib", "0BSD", "BSL-1.0", "Unlicense", "Unicode-3.0", "CC0-1.0"];
        list.sort();
        assert_eq!(ours, list);
    }

    /// The licences these notices accept are deny.toml's.
    #[test]
    fn allowed_matches_deny_toml() {
        let deny = include_str!("../../../deny.toml");
        let allow = deny.split("allow = [").nth(1).unwrap().split(']').next().unwrap();
        let mut theirs: Vec<&str> = allow.lines().filter_map(|l| l.split('"').nth(1)).collect();
        let mut ours: Vec<&str> = ALLOWED.iter().map(|(id, _)| *id).collect();
        theirs.sort();
        ours.sort();
        assert_eq!(theirs, ours);
    }
}

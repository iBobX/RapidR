//! RARCHIVE (RapidR's): opens zip, 7z, tar, tar.gz, .gz, xz and zstd files
//! — the format found from their first bytes — lists and unpacks their
//! entries; makes zip, tar, tar.gz and .gz files. The same code on every
//! runtime: the engine is rapidr-archive (in memory), and files are read
//! and written through the objects' file functions (the web's page store,
//! the desktop's files).
//!
//! An archive is untrusted: MaxSize and MaxRatio hold what it unpacks to
//! (a zip bomb is an error); ExtractAll writes only below its folder (a
//! name with `..`, an absolute path, a drive, a device refuses the whole
//! extraction, before anything is written; links and devices are never
//! extracted; on the desktop a link already in the folder is never written
//! through); encrypted entries are refused. Every call leaves `Error` ""
//! when it worked, else why not — never a run-time error.

use std::cell::RefCell;
use std::collections::HashMap;

use crate::{v_int, v_str, Value};

#[cfg(feature = "archive")]
use super::codec::{bytes_to_string, string_to_bytes};

/// Its RapidR name (no RapidQ one: RapidR's own).
pub const TYPES: &[&str] = &["RARCHIVE"];

pub fn is_type(type_name: &str) -> bool {
    TYPES.iter().any(|t| t.eq_ignore_ascii_case(type_name))
}

/// The runtime's clock: seconds since 1970 now, and the local time zone's
/// seconds east of UTC at a time (a zip keeps local times).
pub type Clock = (fn() -> i64, fn(i64) -> i64);

fn no_offset(_: i64) -> i64 {
    0
}

fn system_now() -> i64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
    }
    // (the browser has no std clock: the web runtime installs Date.now)
    #[cfg(target_arch = "wasm32")]
    0
}

/// The archive being made: where it goes on Close.
#[cfg(feature = "archive")]
enum Target {
    File(String),
    Stream(String),
}

#[derive(Default)]
struct State {
    #[cfg(feature = "archive")]
    open: Option<rapidr_archive::Archive>,
    #[cfg(feature = "archive")]
    making: Option<(rapidr_archive::Writer, Target, u64)>,
    file_name: String,
    error: String,
    max_size: i64,
    max_ratio: i64,
    level: i64,
    case_sensitive: bool,
}

thread_local! {
    static STORE: RefCell<HashMap<String, State>> = RefCell::new(HashMap::new());
    static CLOCK: std::cell::Cell<Clock> = const { std::cell::Cell::new((system_now, no_offset)) };
}

/// The runtime's clock (installed as an RArchive is made).
pub fn set_clock(now: fn() -> i64, offset: fn(i64) -> i64) {
    CLOCK.with(|c| c.set((now, offset)));
}

#[cfg(feature = "archive")]
fn now() -> i64 {
    CLOCK.with(|c| (c.get().0)())
}

#[cfg(feature = "archive")]
fn offset_at(t: i64) -> i64 {
    CLOCK.with(|c| (c.get().1)(t))
}

/// The defaults: MaxSize 1 GB (256 MB in a browser), MaxRatio 1000, Level 6.
fn new_state() -> State {
    #[cfg(feature = "archive")]
    let (max_size, max_ratio) = {
        let l = rapidr_archive::Limits::default_here();
        (l.max_size as i64, l.max_ratio as i64)
    };
    #[cfg(not(feature = "archive"))]
    let (max_size, max_ratio) = (if cfg!(target_arch = "wasm32") { 256 << 20 } else { 1 << 30 }, 1000);
    State { max_size, max_ratio, level: 6, ..State::default() }
}

pub fn create(id: &str, type_name: &str) -> bool {
    if !is_type(type_name) {
        return false;
    }
    STORE.with(|s| s.borrow_mut().insert(id.to_lowercase(), new_state()));
    true
}

pub fn exists(id: &str) -> bool {
    STORE.with(|s| s.borrow().contains_key(&id.to_lowercase()))
}

fn with<R>(id: &str, f: impl FnOnce(&mut State) -> R) -> Option<R> {
    STORE.with(|s| s.borrow_mut().get_mut(&id.to_lowercase()).map(f))
}

pub fn get(id: &str, prop: &str) -> Option<Value> {
    with(id, |s| {
        Some(match prop {
            "count" => v_int(count(s) as i64),
            "format" => v_str(format_name(s)),
            "filename" => v_str(&s.file_name),
            "error" => v_str(&s.error),
            "maxsize" => v_int(s.max_size),
            "maxratio" => v_int(s.max_ratio),
            "level" => v_int(s.level),
            "casesensitive" => v_int(if s.case_sensitive { -1 } else { 0 }),
            _ => return None,
        })
    })?
}

/// `Some(Ok)`: set (a read-only one left as it is); `None`: not one of its.
pub fn set(id: &str, prop: &str, val: &Value) -> Option<Result<(), String>> {
    with(id, |s| {
        match prop {
            // (0 or less: no limit is never allowed — the default instead)
            "maxsize" => s.max_size = if val.to_i64() > 0 { val.to_i64() } else { new_state().max_size },
            "maxratio" => s.max_ratio = val.to_i64().max(0),
            "level" => s.level = val.to_i64().clamp(0, 9),
            "casesensitive" => s.case_sensitive = val.to_bool(),
            "count" | "format" | "filename" | "error" => {}
            _ => return None,
        }
        Some(Ok(()))
    })?
}

fn count(s: &State) -> usize {
    #[cfg(feature = "archive")]
    {
        if let Some(a) = &s.open {
            return a.entries().len();
        }
        if let Some((w, _, _)) = &s.making {
            return w.count();
        }
    }
    let _ = s;
    0
}

fn format_name(s: &State) -> &'static str {
    #[cfg(feature = "archive")]
    {
        if let Some(a) = &s.open {
            return a.format().name();
        }
        if let Some((w, _, _)) = &s.making {
            return w.format().name();
        }
    }
    let _ = s;
    ""
}

/// True / False as BASIC has them (-1 / 0).
#[cfg(feature = "archive")]
fn flag(b: bool) -> Value {
    v_int(if b { -1 } else { 0 })
}

/// The methods. Never a run-time error: a failure is False (or "", -1)
/// with `Error` saying why.
#[cfg(not(feature = "archive"))]
pub fn call(id: &str, method: &str, _args: &[Value]) -> Option<Result<Value, String>> {
    with(id, |s| s.error = "this program was built without RArchive: build it again with RapidR (it adds RArchive to programs that use it)".into())?;
    let _ = method;
    Some(Ok(v_int(0)))
}

#[cfg(feature = "archive")]
pub fn call(id: &str, method: &str, args: &[Value]) -> Option<Result<Value, String>> {
    exists(id).then_some(())?;
    let arg = |i: usize| args.get(i).cloned().unwrap_or(Value::Null);
    let out: Result<Value, (String, Value)> = match method {
        "open" => open(id, &arg(0)).map(|_| flag(true)).map_err(|e| (e, flag(false))),
        "openstream" => match super::with_stream(&arg(0).to_string_val(), |m| m.data.clone()) {
            Some(bytes) => open_bytes(id, bytes, "").map(|_| flag(true)).map_err(|e| (e, flag(false))),
            None => Err((format!("{} isn't a stream (an RMemoryStream or RFileStream)", arg(0).to_string_val()), flag(false))),
        },
        "opendata" => open_bytes(id, string_to_bytes(&arg(0).to_string_val()), "").map(|_| flag(true)).map_err(|e| (e, flag(false))),
        "close" => close(id).map(|_| flag(true)).map_err(|e| (e, flag(false))),
        "formatof" => Ok(v_str(&format_of(&arg(0)))),
        "find" => Ok(v_int(with(id, |s| s.open.as_ref().and_then(|a| a.find(&arg(0).to_string_val(), s.case_sensitive))).flatten().map_or(-1, |i| i as i64))),
        "name" | "size" | "compressedsize" | "isfolder" | "isencrypted" | "datetime" => Ok(entry_info(id, method, &arg(0))),
        "extract" => extract(id, &arg(0)).map(|b| v_str(&bytes_to_string(&b))).map_err(|e| (e, v_str(""))),
        "extracttext" => extract(id, &arg(0)).map(|b| v_str(&text_of(&b))).map_err(|e| (e, v_str(""))),
        "extracttostream" => extract_to_stream(id, &arg(0), &arg(1).to_string_val()).map(|_| flag(true)).map_err(|e| (e, flag(false))),
        "extracttofile" => extract_to_file(id, &arg(0), &arg(1).to_string_val()).map(|_| flag(true)).map_err(|e| (e, flag(false))),
        "extractall" => extract_all(id, &arg(0).to_string_val()).map(|_| flag(true)).map_err(|e| (e, flag(false))),
        "create" => create_archive(id, &arg(0).to_string_val(), args.get(1).map(Value::to_string_val), false).map(|_| flag(true)).map_err(|e| (e, flag(false))),
        "createstream" => create_archive(id, &arg(0).to_string_val(), args.get(1).map(Value::to_string_val), true).map(|_| flag(true)).map_err(|e| (e, flag(false))),
        "addfile" => add_file(id, &arg(0), args.get(1).map(Value::to_string_val)).map(|_| flag(true)).map_err(|e| (e, flag(false))),
        "addtext" => add(id, &arg(0).to_string_val(), arg(1).to_string_val().into_bytes(), now()).map(|_| flag(true)).map_err(|e| (e, flag(false))),
        "adddata" => add(id, &arg(0).to_string_val(), string_to_bytes(&arg(1).to_string_val()), now()).map(|_| flag(true)).map_err(|e| (e, flag(false))),
        "addstream" => match super::with_stream(&arg(1).to_string_val(), |m| m.data.clone()) {
            Some(bytes) => add(id, &arg(0).to_string_val(), bytes, now()).map(|_| flag(true)).map_err(|e| (e, flag(false))),
            None => Err((format!("{} isn't a stream (an RMemoryStream or RFileStream)", arg(1).to_string_val()), flag(false))),
        },
        "addfolder" => add_folder(id, &arg(0).to_string_val(), args.get(1).map(Value::to_string_val)).map(|_| flag(true)).map_err(|e| (e, flag(false))),
        _ => return None,
    };
    Some(Ok(match out {
        Ok(v) => {
            with(id, |s| s.error.clear());
            v
        }
        Err((e, v)) => {
            with(id, |s| s.error = e);
            v
        }
    }))
}

#[cfg(feature = "archive")]
fn limits(id: &str) -> rapidr_archive::Limits {
    with(id, |s| rapidr_archive::Limits { max_size: s.max_size.max(1) as u64, max_ratio: s.max_ratio.max(0) as u64 }).unwrap_or_else(rapidr_archive::Limits::default_here)
}

/// A file a program names, or a `$RESOURCE`'s handle (the archive built
/// into the program): its bytes and the name to show.
#[cfg(feature = "archive")]
fn source_bytes(v: &Value) -> Result<(Vec<u8>, String), String> {
    if matches!(v, Value::Integer(_) | Value::Double(_)) {
        if let Some(b) = crate::resources::bytes(v.to_i64()) {
            return Ok((b.to_vec(), String::new()));
        }
    }
    let name = v.to_string_val();
    if name.is_empty() {
        return Err("no file name".into());
    }
    // (the current folder's, else the one the program came with — as its
    // pictures and data files: a built app started elsewhere finds them)
    super::read_or_own(&name).map(|b| (b, name.clone())).map_err(|_| format!("can't read {name}: no such file, or it can't be read"))
}

#[cfg(feature = "archive")]
fn open(id: &str, v: &Value) -> Result<(), String> {
    let (bytes, name) = source_bytes(v)?;
    open_bytes(id, bytes, &name)
}

/// Opens `bytes` (closing what was open: an archive being made is
/// dropped unwritten).
#[cfg(feature = "archive")]
fn open_bytes(id: &str, bytes: Vec<u8>, name: &str) -> Result<(), String> {
    let lim = limits(id);
    with(id, |s| {
        s.open = None;
        s.making = None;
        s.file_name = name.to_string();
    });
    let a = rapidr_archive::Archive::open(bytes, name, &lim)?;
    with(id, |s| s.open = Some(a));
    Ok(())
}

/// `FormatOf(FileName)`: what a file (or a `$RESOURCE`) is, from its first
/// bytes — "zip", "7z", "tar", "gzip" … — or "" when it's no archive
/// RapidR knows: a `.z80` snapshot, a text. Nothing is opened.
#[cfg(feature = "archive")]
fn format_of(v: &Value) -> String {
    let Ok((bytes, _)) = source_bytes(v) else { return String::new() };
    rapidr_archive::detect(&bytes).map_or(String::new(), |f| f.name().to_string())
}

/// Entry `which`: an index (0 the first) or a name.
#[cfg(feature = "archive")]
fn entry_index(s: &State, which: &Value) -> Result<usize, String> {
    let a = s.open.as_ref().ok_or("no archive is open (Open, OpenStream or OpenData first)")?;
    match which {
        Value::String(name) => a.find(name, s.case_sensitive).ok_or_else(|| format!("there's no \"{name}\" in the archive")),
        v => {
            let i = v.to_i64();
            usize::try_from(i).ok().filter(|&i| i < a.entries().len()).ok_or_else(|| format!("no entry {i}: the archive has {} (0 to {})", a.entries().len(), a.entries().len() as i64 - 1))
        }
    }
}

/// Name(i), Size(i), CompressedSize(i), IsFolder(i), IsEncrypted(i),
/// DateTime(i): "" / 0 / False for an entry that isn't there.
#[cfg(feature = "archive")]
fn entry_info(id: &str, what: &str, which: &Value) -> Value {
    let r = with(id, |s| {
        let i = entry_index(s, which).ok()?;
        let e = &s.open.as_ref()?.entries()[i];
        Some(match what {
            "name" => v_str(&e.name),
            "size" => v_int(e.size as i64),
            "compressedsize" => v_int(e.compressed_size as i64),
            "isfolder" => flag(e.kind == rapidr_archive::Kind::Folder),
            "isencrypted" => flag(e.encrypted),
            _ => v_str(&e.modified.map(|m| m.local(offset_at(m_unix(m))).to_string()).unwrap_or_default()),
        })
    })
    .flatten();
    r.unwrap_or_else(|| match what {
        "name" | "datetime" => v_str(""),
        "isfolder" | "isencrypted" => flag(false),
        _ => v_int(0),
    })
}

/// The time a stamp is at, for the time zone's offset then.
#[cfg(feature = "archive")]
fn m_unix(m: rapidr_archive::Modified) -> i64 {
    match m {
        rapidr_archive::Modified::Unix(t) => t,
        rapidr_archive::Modified::Local(s) => s.to_unix(0),
    }
}

#[cfg(feature = "archive")]
fn extract(id: &str, which: &Value) -> Result<Vec<u8>, String> {
    let lim = limits(id);
    with(id, |s| {
        let i = entry_index(s, which)?;
        s.open.as_ref().ok_or_else(String::new)?.extract(i, &lim)
    })
    .unwrap_or_else(|| Err("no such RArchive".into()))
}

/// Text: UTF-8 (a byte-order mark dropped) when it is, else one character
/// a byte (Windows' Latin-1).
#[cfg(feature = "archive")]
fn text_of(b: &[u8]) -> String {
    let b = b.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(b);
    match std::str::from_utf8(b) {
        Ok(s) => s.to_string(),
        Err(_) => bytes_to_string(b),
    }
}

/// ExtractToFile: the entry (a file, not a folder) written to the file the
/// program names, once it's all unpacked and checked.
#[cfg(feature = "archive")]
fn extract_to_file(id: &str, which: &Value, file: &str) -> Result<(), String> {
    let folder = with(id, |s| entry_index(s, which).ok().and_then(|i| s.open.as_ref().map(|a| a.entries()[i].kind == rapidr_archive::Kind::Folder))).flatten();
    if folder == Some(true) {
        return Err(format!("\"{}\" is a folder: ExtractAll makes folders", which.to_string_val()));
    }
    if file.trim().is_empty() {
        return Err("ExtractToFile needs a file name".into());
    }
    let bytes = extract(id, which)?;
    super::write_file(file, &bytes)
}

/// The entry's bytes written at the stream's Position (as WriteStr
/// writes), Position after them.
#[cfg(feature = "archive")]
fn extract_to_stream(id: &str, which: &Value, stream: &str) -> Result<(), String> {
    if super::with_stream(stream, |_| ()).is_none() {
        return Err(format!("{stream} isn't a stream (an RMemoryStream or RFileStream)"));
    }
    let bytes = extract(id, which)?;
    super::with_stream_mut(stream, |m| m.write(&bytes));
    Ok(())
}

/// Everything unpacked into `folder` (made if needed): every name checked
/// first, everything unpacked within the limits, then written — so a
/// zip-slip name, an encrypted entry or a bomb writes nothing. Links and
/// devices are left out.
#[cfg(feature = "archive")]
fn extract_all(id: &str, folder: &str) -> Result<(), String> {
    use rapidr_archive::Kind;
    let lim = limits(id);
    let plan = with(id, |s| -> Result<(Vec<Vec<String>>, Vec<(Vec<String>, Vec<u8>)>), String> {
        let a = s.open.as_ref().ok_or("no archive is open (Open, OpenStream or OpenData first)")?;
        // (every name, before anything is unpacked)
        let mut folders = Vec::new();
        for e in a.entries() {
            if matches!(e.kind, Kind::Folder | Kind::File) {
                let parts = rapidr_archive::safe_path(&e.name).map_err(|err| format!("{err}; nothing was extracted"))?;
                if e.kind == Kind::Folder {
                    folders.push(parts);
                }
            }
        }
        let files = a.extract_all(&lim).map_err(|e| format!("{e}; nothing was extracted"))?;
        let files = files.into_iter().map(|(i, data)| (rapidr_archive::safe_path(&a.entries()[i].name).unwrap_or_default(), data)).collect();
        Ok((folders, files))
    })
    .unwrap_or_else(|| Err("no such RArchive".into()))?;
    let (folders, files) = plan;
    if folder.trim().is_empty() {
        return Err("ExtractAll needs a folder (\".\" for the current one)".into());
    }
    if super::NATIVE_FILES.with(std::cell::Cell::get) {
        write_tree(folder, &folders, &files)
    } else {
        // (the web: the page's file store, files by their whole path)
        let base = folder.trim_end_matches(['/', '\\']);
        for (parts, data) in &files {
            super::write_file(&format!("{base}/{}", parts.join("/")), data)?;
        }
        Ok(())
    }
}

/// The desktop's ExtractAll: below `folder` only, never through a link.
#[cfg(all(feature = "archive", not(target_arch = "wasm32")))]
fn write_tree(folder: &str, folders: &[Vec<String>], files: &[(Vec<String>, Vec<u8>)]) -> Result<(), String> {
    use std::path::{Path, PathBuf};
    let base = crate::paths::host(folder);
    std::fs::create_dir_all(&base).map_err(|e| format!("can't make the folder {folder}: {e}"))?;
    let is_link = |p: &Path| std::fs::symlink_metadata(p).is_ok_and(|m| m.file_type().is_symlink());
    // (each place checked before anything is written: nothing that's a
    // link below the folder is followed)
    let place = |parts: &[String]| -> Result<PathBuf, String> {
        let mut p = base.clone();
        for part in parts {
            p.push(part);
            if is_link(&p) {
                return Err(format!("\"{}\" would be written through a link in {folder}: nothing was extracted", parts.join("/")));
            }
        }
        Ok(p)
    };
    let mut planned = Vec::new();
    for parts in folders {
        place(parts)?;
    }
    for (parts, data) in files {
        let p = place(parts)?;
        if p.is_dir() {
            return Err(format!("\"{}\" is a folder in {folder}: nothing was extracted", parts.join("/")));
        }
        planned.push((p, data));
    }
    for parts in folders {
        std::fs::create_dir_all(place(parts)?).map_err(|e| format!("can't make {}: {e}", parts.join("/")))?;
    }
    let mut made: Vec<PathBuf> = Vec::new();
    for (p, data) in planned {
        let fresh = !p.exists();
        let r = p.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|_| std::fs::write(&p, data));
        if let Err(e) = r {
            // (what this call made goes again: no half-extracted archive)
            for m in &made {
                let _ = std::fs::remove_file(m);
            }
            return Err(format!("can't write {}: {e}", p.display()));
        }
        if fresh {
            made.push(p);
        }
    }
    Ok(())
}

#[cfg(all(feature = "archive", target_arch = "wasm32"))]
fn write_tree(folder: &str, _: &[Vec<String>], files: &[(Vec<String>, Vec<u8>)]) -> Result<(), String> {
    let base = folder.trim_end_matches(['/', '\\']);
    for (parts, data) in files {
        super::write_file(&format!("{base}/{}", parts.join("/")), data)?;
    }
    Ok(())
}

/// Create(FileName [, Format]) / CreateStream(Stream [, Format]): a new
/// archive, its format the name's extension or Format ("zip", "tar",
/// "tar.gz", "gzip"); written on Close.
#[cfg(feature = "archive")]
fn create_archive(id: &str, target: &str, format: Option<String>, stream: bool) -> Result<(), String> {
    use rapidr_archive::Format;
    let named = format.as_deref().map(str::trim).filter(|f| !f.is_empty());
    let format = match named {
        Some(f) => Format::from_name(f).ok_or_else(|| format!("\"{f}\" isn't an archive format (zip, tar, tar.gz, gzip)"))?,
        None if stream => Format::Zip,
        None => Format::from_file_name(target).ok_or_else(|| format!("{target}: give it an archive's extension (.zip, .tar, .tar.gz, .gz) or name the format"))?,
    };
    if stream && super::with_stream(target, |_| ()).is_none() {
        return Err(format!("{target} isn't a stream (an RMemoryStream or RFileStream)"));
    }
    if !stream && target.trim().is_empty() {
        return Err("Create needs a file name".into());
    }
    let level = with(id, |s| s.level).unwrap_or(6).clamp(0, 9) as u32;
    let w = rapidr_archive::Writer::new(format, level, offset_at(now()))?;
    with(id, |s| {
        s.open = None;
        s.file_name = if stream { String::new() } else { target.to_string() };
        s.making = Some((w, if stream { Target::Stream(target.to_string()) } else { Target::File(target.to_string()) }, 0));
    });
    Ok(())
}

/// Adds a file to the archive being made; what it holds in all is at most
/// MaxSize.
#[cfg(feature = "archive")]
fn add(id: &str, name: &str, data: Vec<u8>, mtime: i64) -> Result<(), String> {
    let max = limits(id).max_size;
    with(id, |s| {
        let (w, _, total) = s.making.as_mut().ok_or("no archive is being made (Create or CreateStream first)")?;
        let after = total.saturating_add(data.len() as u64);
        if after > max {
            return Err(format!("the archive would hold more than MaxSize ({max} bytes)"));
        }
        w.add(name, data, mtime)?;
        *total = after;
        Ok(())
    })
    .unwrap_or_else(|| Err("no such RArchive".into()))
}

/// The last part of a path (`\` or `/`).
#[cfg(feature = "archive")]
fn last_part(path: &str) -> &str {
    path.trim_end_matches(['/', '\\']).rsplit(['/', '\\']).next().unwrap_or(path)
}

/// AddFile(FileName [, NameInArchive]): the file's bytes (a `$RESOURCE`'s
/// too), kept under its own name (no folders) unless named.
#[cfg(feature = "archive")]
fn add_file(id: &str, file: &Value, name: Option<String>) -> Result<(), String> {
    let (bytes, path) = source_bytes(file)?;
    let name = name.filter(|n| !n.trim().is_empty()).unwrap_or_else(|| last_part(&path).to_string());
    if name.is_empty() {
        return Err("give the file a name in the archive (AddFile(Resource, \"name.ext\"))".into());
    }
    add(id, &name, bytes, modified_time(&path))
}

/// When a file on disk was modified (now, for anything else).
#[cfg(feature = "archive")]
fn modified_time(path: &str) -> i64 {
    #[cfg(not(target_arch = "wasm32"))]
    if super::NATIVE_FILES.with(std::cell::Cell::get) && !path.is_empty() {
        if let Some(t) = std::fs::metadata(crate::paths::host(path)).ok().and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()) {
            return t.as_secs() as i64;
        }
    }
    let _ = path;
    now()
}

/// AddFolder(Folder [, NameInArchive]): a folder on disk and everything in
/// it (links not followed), under its own name unless named.
#[cfg(feature = "archive")]
fn add_folder(id: &str, folder: &str, name: Option<String>) -> Result<(), String> {
    #[cfg(not(target_arch = "wasm32"))]
    if super::NATIVE_FILES.with(std::cell::Cell::get) {
        let root = crate::paths::host(folder);
        if !root.is_dir() {
            return Err(format!("{folder} isn't a folder"));
        }
        let top = name.filter(|n| !n.trim().is_empty()).unwrap_or_else(|| last_part(folder).to_string());
        let top = if top == "." || top.is_empty() { String::new() } else { top };
        let mut stack = vec![(root, top)];
        while let Some((dir, inside)) = stack.pop() {
            let mut children: Vec<_> = std::fs::read_dir(&dir).map_err(|e| format!("can't read {}: {e}", dir.display()))?.filter_map(Result::ok).collect();
            children.sort_by_key(std::fs::DirEntry::file_name);
            if children.is_empty() && !inside.is_empty() {
                with(id, |s| s.making.as_mut().map(|(w, _, _)| w.add_folder(&inside, now()))).flatten().transpose()?;
            }
            for c in children {
                let Ok(kind) = c.file_type() else { continue };
                let child = c.file_name().to_string_lossy().into_owned();
                let in_archive = if inside.is_empty() { child } else { format!("{inside}/{child}") };
                if kind.is_dir() {
                    stack.push((c.path(), in_archive));
                } else if kind.is_file() {
                    let bytes = std::fs::read(c.path()).map_err(|e| format!("can't read {}: {e}", c.path().display()))?;
                    let mtime = c.metadata().ok().and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or_else(now, |d| d.as_secs() as i64);
                    add(id, &in_archive, bytes, mtime)?;
                }
            }
        }
        return Ok(());
    }
    let _ = (id, name);
    Err(format!("AddFolder needs a folder on disk ({folder}): in the browser, add each file with AddFile"))
}

/// Close: an archive being made is written (to its file or stream); an
/// opened one is let go.
#[cfg(feature = "archive")]
fn close(id: &str) -> Result<(), String> {
    let making = with(id, |s| {
        s.open = None;
        s.making.take()
    })
    .flatten();
    let Some((w, target, _)) = making else { return Ok(()) };
    let bytes = w.finish()?;
    match target {
        Target::File(path) => super::write_file(&path, &bytes),
        Target::Stream(stream) => super::with_stream_mut(&stream, |m| m.write(&bytes)).ok_or_else(|| format!("{stream} isn't a stream any more")),
    }
}

/// DECOMPRESS$'s MaxSize when none is given: 256 MB.
pub const DECOMPRESS_MAX_SIZE: i64 = 256 << 20;

/// `COMPRESS$(Data$ [, Format$ [, Level]])` — Format "gzip" (the default),
/// "zlib" or "deflate"; Level 0 (stored) … 9 (smallest), 6 by default — and
/// `DECOMPRESS$(Data$ [, Format$ [, MaxSize]])` — the format found from the
/// first bytes (gzip, zlib, xz, zstd) unless named ("deflate" must be); at
/// most MaxSize bytes (256 MB by default). Data is a string of bytes (one
/// character a byte, as a stream's ReadBinStr gives). Bad data is a
/// run-time error saying why (RArchive's OpenData reports in Error instead).
pub fn builtin(key: &str, args: &[Value]) -> Result<Value, String> {
    let name = if key == "compress" { "COMPRESS$" } else { "DECOMPRESS$" };
    #[cfg(not(feature = "archive"))]
    {
        let _ = args;
        Err(format!("{name}: this program was built without RArchive: build it again with RapidR"))
    }
    #[cfg(feature = "archive")]
    {
        use rapidr_archive::{Codec, Limits};
        let data = string_to_bytes(&args.first().map(Value::to_string_val).unwrap_or_default());
        let format = args.get(1).map(Value::to_string_val).filter(|f| !f.trim().is_empty());
        let codec = match &format {
            Some(f) => Some(Codec::from_name(f).ok_or_else(|| format!("{name}: \"{f}\" isn't a format RapidR packs (gzip, zlib, deflate; xz and zstd unpack only)"))?),
            None => None,
        };
        let out = if key == "compress" {
            let level = args.get(2).map_or(6, |l| l.to_i64().clamp(0, 9)) as u32;
            rapidr_archive::compress(&data, codec.unwrap_or(Codec::Gzip), level)
        } else {
            let max = args.get(2).map(Value::to_i64).filter(|&m| m > 0).unwrap_or(DECOMPRESS_MAX_SIZE);
            rapidr_archive::decompress(&data, codec, &Limits { max_size: max as u64, max_ratio: 0 })
        };
        out.map(|b| v_str(&bytes_to_string(&b))).map_err(|e| format!("{name}: {e}"))
    }
}

/// COMPRESS$ in a native build (a failure is a run-time error).
pub fn rp_compress(args: &[Value]) -> Value {
    builtin("compress", args).unwrap_or_else(|e| crate::runtime_error(&e))
}

/// DECOMPRESS$ in a native build (a failure is a run-time error).
pub fn rp_decompress(args: &[Value]) -> Value {
    builtin("decompress", args).unwrap_or_else(|e| crate::runtime_error(&e))
}

/// The component on a real disk: ExtractAll never writes outside its
/// folder (zip-slip names, links in the archive, a link already in the
/// folder), a bomb writes nothing, what's made reads back.
#[cfg(all(test, feature = "archive", unix))]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    fn fixture(name: &str) -> String {
        let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../rapidr-archive/tests/fixtures").join(name);
        bytes_to_string(&std::fs::read(p).unwrap())
    }

    fn temp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("rapidr-archive-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn call_ok(id: &str, method: &str, args: &[Value]) -> Value {
        call(id, method, args).expect("an RArchive method").expect("never a run-time error")
    }

    fn error(id: &str) -> String {
        get(id, "error").unwrap().to_string_val()
    }

    #[test]
    fn extract_all_stays_in_its_folder() {
        assert!(create("t_ar_slip", "RArchive"));
        let dir = temp("slip");
        let out = dir.join("out");
        let s = |p: &Path| v_str(&p.to_string_lossy());
        // (names with .. and an absolute path: nothing written at all)
        assert_eq!(call_ok("t_ar_slip", "opendata", &[v_str(&fixture("evil-slip.zip"))]).to_i64(), -1);
        assert_eq!(call_ok("t_ar_slip", "extractall", &[s(&out)]).to_i64(), 0);
        assert!(error("t_ar_slip").contains("goes up a folder") && error("t_ar_slip").contains("nothing was extracted"), "{}", error("t_ar_slip"));
        assert!(!out.exists() && !dir.join("evil.txt").exists() && !Path::new("/tmp/rapidr-abs.txt").exists());
        // (a tar's link isn't made; the file "through" it lands in a real folder)
        call_ok("t_ar_slip", "opendata", &[v_str(&fixture("evil-link.tar"))]);
        assert_eq!(call_ok("t_ar_slip", "extractall", &[s(&out)]).to_i64(), -1, "{}", error("t_ar_slip"));
        assert!(!std::fs::symlink_metadata(out.join("escape")).unwrap().file_type().is_symlink());
        assert_eq!(std::fs::read(out.join("escape/rapidr-through-link.txt")).unwrap(), b"bad\n");
        assert_eq!(std::fs::read(out.join("ok.txt")).unwrap(), b"ok\n");
        // (a link already in the folder is never written through)
        let planted = dir.join("planted");
        let outside = dir.join("outside");
        std::fs::create_dir_all(&planted).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, planted.join("escape")).unwrap();
        assert_eq!(call_ok("t_ar_slip", "extractall", &[s(&planted)]).to_i64(), 0);
        assert!(error("t_ar_slip").contains("through a link"), "{}", error("t_ar_slip"));
        assert_eq!(std::fs::read_dir(&outside).unwrap().count(), 0, "nothing reached the link's target");
        assert!(!planted.join("ok.txt").exists(), "nothing written at all");
        // (a bomb: refused before anything is written)
        call_ok("t_ar_slip", "opendata", &[v_str(&fixture("bomb.zip"))]);
        let bombed = dir.join("bombed");
        assert_eq!(call_ok("t_ar_slip", "extractall", &[s(&bombed)]).to_i64(), 0);
        assert!(error("t_ar_slip").contains("MaxRatio"), "{}", error("t_ar_slip"));
        assert!(!bombed.exists());
        // (an encrypted entry: the same)
        call_ok("t_ar_slip", "opendata", &[v_str(&fixture("encrypted.zip"))]);
        assert_eq!(call_ok("t_ar_slip", "extractall", &[s(&bombed)]).to_i64(), 0);
        assert!(error("t_ar_slip").contains("encrypted"));
        assert!(!bombed.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn add_folder_round_trip_without_following_links() {
        assert!(create("t_ar_make", "RARCHIVE"));
        let dir = temp("make");
        let src = dir.join("levels");
        std::fs::create_dir_all(src.join("world1")).unwrap();
        std::fs::create_dir_all(src.join("empty")).unwrap();
        std::fs::write(src.join("world1/a.lvl"), b"AAAA").unwrap();
        std::fs::write(src.join("b.lvl"), [0u8, 255, 1]).unwrap();
        std::os::unix::fs::symlink("/etc", src.join("etc-link")).unwrap();
        let zip = dir.join("levels.tar.gz");
        let s = |p: &Path| v_str(&p.to_string_lossy());
        assert_eq!(call_ok("t_ar_make", "create", &[s(&zip)]).to_i64(), -1, "{}", error("t_ar_make"));
        assert_eq!(call_ok("t_ar_make", "addfolder", &[s(&src)]).to_i64(), -1, "{}", error("t_ar_make"));
        assert_eq!(call_ok("t_ar_make", "addtext", &[v_str("../out.txt"), v_str("x")]).to_i64(), 0);
        assert_eq!(call_ok("t_ar_make", "close", &[]).to_i64(), -1, "{}", error("t_ar_make"));
        assert_eq!(call_ok("t_ar_make", "open", &[s(&zip)]).to_i64(), -1, "{}", error("t_ar_make"));
        assert_eq!(get("t_ar_make", "format").unwrap().to_string_val(), "tar.gz");
        let n = get("t_ar_make", "count").unwrap().to_i64();
        let names: Vec<String> = (0..n).map(|i| call_ok("t_ar_make", "name", &[v_int(i)]).to_string_val()).collect();
        assert_eq!(names, ["levels/b.lvl", "levels/world1/a.lvl", "levels/empty/"], "links not followed");
        assert_eq!(call_ok("t_ar_make", "extract", &[v_str("LEVELS/B.LVL")]).to_string_val(), bytes_to_string(&[0, 255, 1]));
        // (Find is case-insensitive unless CaseSensitive)
        set("t_ar_make", "casesensitive", &v_int(-1));
        assert_eq!(call_ok("t_ar_make", "find", &[v_str("LEVELS/B.LVL")]).to_i64(), -1);
        assert_eq!(get("t_ar_make", "casesensitive").unwrap().to_i64(), -1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn builtins() {
        let packed = builtin("compress", &[v_str("hello hello hello"), v_str("zlib"), v_int(9)]).unwrap();
        assert_eq!(builtin("decompress", &[packed]).unwrap().to_string_val(), "hello hello hello");
        assert!(builtin("decompress", &[v_str("plain")]).unwrap_err().starts_with("DECOMPRESS$: not compressed data"));
        assert!(builtin("compress", &[v_str("x"), v_str("lz4")]).unwrap_err().contains("isn't a format"));
        let big = builtin("compress", &[v_str(&"A".repeat(5000))]).unwrap();
        assert!(builtin("decompress", &[big, v_str(""), v_int(100)]).unwrap_err().contains("MaxSize (100 bytes)"));
    }
}

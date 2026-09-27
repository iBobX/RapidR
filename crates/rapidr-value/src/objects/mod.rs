//! RapidQ's non-visual objects — QFONT, QMEMORYSTREAM, QBITMAP and
//! QIMAGELIST — implemented once for every runtime. The desktop and web
//! runtimes create them in `rp_create_component` and route property and
//! method access here first (`get`/`set`/`call`), so both behave the same.
//!
//! Objects are keyed by component id (lowercase), like the runtimes'
//! component registries. An object argument (`Mem.CopyFrom(Other, 0)`,
//! `Bitmap.Draw(0, 0, Sprite)`) arrives as that id; an image can also be a
//! BMP file name or the `data:` URL a bitmap's `.BMP` property returns.

pub mod bitmap;
pub mod codec;
pub mod font;
pub mod grid;
pub mod imagelist;
pub mod list;
pub mod listview;
pub mod memstream;

use std::cell::RefCell;
use std::collections::HashMap;

use crate::{v_int, v_str, Value};
use bitmap::Bitmap;
use codec::{base64_decode, encode_bmp, BMP_DATA_URL};
use font::Font;
use grid::StringGrid;
use imagelist::ImageList;
use list::ItemList;
use listview::ListView;
use memstream::MemStream;

/// RapidR's names for the object types (RapidQ's QFONT is RFONT, …).
pub const TYPES: &[&str] = &["RFONT", "RMEMORYSTREAM", "RFILESTREAM", "RBITMAP", "RIMAGELIST"];

pub fn is_object_type(type_name: &str) -> bool {
    TYPES.contains(&type_name.to_ascii_uppercase().as_str())
}

enum Object {
    Font(Font),
    Stream(MemStream),
    Bitmap(Bitmap),
    ImageList(ImageList),
    /// QLISTVIEW's columns and items; the runtime draws the widget.
    ListView(ListView),
    /// QSTRINGGRID's cells, sizes and selection; the runtime draws it.
    Grid(StringGrid),
    /// QLISTBOX's / QCOMBOBOX's items and selection; the runtime draws it.
    List(ItemList),
}

/// Reads a whole file (the runtime installs one; the web runtime's reads
/// from the page's own files).
pub type FileReader = fn(&str) -> Result<Vec<u8>, String>;
pub type FileWriter = fn(&str, &[u8]) -> Result<(), String>;

fn std_read(path: &str) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("can't read {path}: {e}"))
}

fn std_write(path: &str, bytes: &[u8]) -> Result<(), String> {
    std::fs::write(path, bytes).map_err(|e| format!("can't write {path}: {e}"))
}

thread_local! {
    static OBJECTS: RefCell<HashMap<String, Object>> = RefCell::new(HashMap::new());
    static FILE_IO: RefCell<(FileReader, FileWriter)> = RefCell::new((std_read, std_write));
    /// False once a runtime replaces the file functions (the web): files
    /// are then read and written whole through them.
    static NATIVE_FILES: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
}

/// Replaces how objects read and write files (the web runtime has no file
/// system).
pub fn set_file_io(reader: FileReader, writer: FileWriter) {
    FILE_IO.with(|io| *io.borrow_mut() = (reader, writer));
    NATIVE_FILES.with(|n| n.set(false));
}

fn read_file(path: &str) -> Result<Vec<u8>, String> {
    let reader = FILE_IO.with(|io| io.borrow().0);
    reader(path)
}

pub(crate) fn write_file(path: &str, bytes: &[u8]) -> Result<(), String> {
    let writer = FILE_IO.with(|io| io.borrow().1);
    writer(path, bytes)
}

/// Creates the object if `type_name` is one of [`TYPES`]; `false` otherwise.
pub fn create(id: &str, type_name: &str) -> bool {
    let object = match type_name.to_ascii_uppercase().as_str() {
        "RFONT" => Object::Font(Font::default()),
        "RMEMORYSTREAM" | "RFILESTREAM" => Object::Stream(MemStream::default()),
        "RBITMAP" => Object::Bitmap(Bitmap::default()),
        "RIMAGELIST" => Object::ImageList(ImageList::default()),
        "RLISTVIEW" => Object::ListView(ListView::default()),
        "RSTRINGGRID" => Object::Grid(StringGrid::default()),
        "RLISTBOX" => Object::List(ItemList::new(false)),
        "RCOMBOBOX" => Object::List(ItemList::new(true)),
        _ => return false,
    };
    OBJECTS.with(|o| {
        o.borrow_mut().entry(id.to_lowercase()).or_insert(object);
    });
    true
}

/// Whether `id` is a QLISTVIEW (its runtime widget redraws after a change).
pub fn is_listview(id: &str) -> bool {
    with(id, |o| matches!(o, Object::ListView(_))).unwrap_or(false)
}

/// Reads a QLISTVIEW's data (to draw it).
pub fn with_listview<R>(id: &str, f: impl FnOnce(&ListView) -> R) -> Option<R> {
    with(id, |o| match o {
        Object::ListView(l) => Some(f(l)),
        _ => None,
    })?
}

/// Whether `id` is a QLISTBOX or QCOMBOBOX (its widget redraws after a
/// change).
pub fn is_list(id: &str) -> bool {
    with(id, |o| matches!(o, Object::List(_))).unwrap_or(false)
}

/// Reads a QLISTBOX's / QCOMBOBOX's items (to draw them).
pub fn with_list<R>(id: &str, f: impl FnOnce(&ItemList) -> R) -> Option<R> {
    with(id, |o| match o {
        Object::List(l) => Some(f(l)),
        _ => None,
    })?
}

/// Changes a list's selection from its widget (the user picked an item).
pub fn with_list_mut<R>(id: &str, f: impl FnOnce(&mut ItemList) -> R) -> Option<R> {
    with(id, |o| match o {
        Object::List(l) => Some(f(l)),
        _ => None,
    })?
}

/// Whether `id` is a QSTRINGGRID (its runtime widget redraws after a change).
pub fn is_grid(id: &str) -> bool {
    with(id, |o| matches!(o, Object::Grid(_))).unwrap_or(false)
}

/// Reads a QSTRINGGRID's data (to draw it).
pub fn with_grid<R>(id: &str, f: impl FnOnce(&StringGrid) -> R) -> Option<R> {
    with(id, |o| match o {
        Object::Grid(g) => Some(f(g)),
        _ => None,
    })?
}

/// Changes a QSTRINGGRID's data from its widget (the user edited a cell or
/// selected one).
pub fn with_grid_mut<R>(id: &str, f: impl FnOnce(&mut StringGrid) -> R) -> Option<R> {
    with(id, |o| match o {
        Object::Grid(g) => Some(f(g)),
        _ => None,
    })?
}

pub fn exists(id: &str) -> bool {
    OBJECTS.with(|o| o.borrow().contains_key(&id.to_lowercase()))
}

fn with<R>(id: &str, f: impl FnOnce(&mut Object) -> R) -> Option<R> {
    OBJECTS.with(|o| o.borrow_mut().get_mut(&id.to_lowercase()).map(f))
}

/// A property of object `id`; `None` if `id` isn't an object or the
/// property isn't one it implements (the runtime then keeps it as a plain
/// stored property).
pub fn get(id: &str, prop: &str) -> Option<Value> {
    let prop = prop.to_lowercase();
    with(id, |o| match o {
        Object::Font(f) => f.get(&prop),
        // Functions called without parentheses: `S$ = Mem.ReadLine`.
        Object::Stream(m) if matches!(prop.as_str(), "readline" | "readln" | "readall") => m.call(&prop, &[]),
        Object::Stream(m) => m.get(&prop),
        Object::Bitmap(b) => b.get(&prop),
        Object::ImageList(l) => l.get(&prop),
        Object::ListView(l) => l.get(&prop),
        Object::Grid(g) => g.get(&prop),
        Object::List(l) => l.get(&prop),
    })?
}

/// Sets a property; `Some(Ok)` if handled, `Some(Err)` if it failed (a BMP
/// that can't be loaded), `None` if not an object property.
pub fn set(id: &str, prop: &str, val: &Value) -> Option<Result<(), String>> {
    let prop = prop.to_lowercase();
    if prop == "bmp" && matches!(with(id, |o| matches!(o, Object::Bitmap(_))), Some(true)) {
        let loaded = load_image(val);
        return Some(loaded.map(|src| {
            with(id, |o| {
                if let Object::Bitmap(b) = o {
                    if src.transparent {
                        (b.transparent, b.transparent_color) = (true, src.transparent_color);
                    }
                    b.img = src.img;
                }
            });
        }));
    }
    with(id, |o| match o {
        Object::Font(f) => f.set(&prop, val).then_some(Ok(())),
        Object::Stream(m) => m.set(&prop, val).then_some(Ok(())),
        Object::Bitmap(b) => b.set(&prop, val),
        Object::ImageList(l) => l.set(&prop, val).then_some(Ok(())),
        Object::ListView(l) => l.set(&prop, val).then_some(Ok(())),
        Object::Grid(g) => g.set(&prop, val).then_some(Ok(())),
        Object::List(l) => l.set(&prop, val).then_some(Ok(())),
    })?
}

/// Reads a property of another component (for QRECT arguments, which are
/// plain property bags in the runtime).
pub type PropReader<'a> = &'a dyn Fn(&str, &str) -> Value;

/// Calls a method; `None` if `id` isn't an object or it has no such method.
/// `Some(Err)` is a failure to report (e.g. a file that can't be read).
pub fn call(id: &str, method: &str, args: &[Value], props: PropReader) -> Option<Result<Value, String>> {
    let method = method.to_lowercase();
    let arg = |i: usize| args.get(i).cloned().unwrap_or(Value::Null);
    // Methods that need another object or a file, handled outside the
    // object so the registry isn't borrowed twice.
    let kind = with(id, |o| match o {
        Object::Font(_) => "font",
        Object::Stream(_) => "stream",
        Object::Bitmap(_) => "bitmap",
        Object::ImageList(_) => "imagelist",
        Object::ListView(_) => "listview",
        Object::Grid(_) => "grid",
        Object::List(_) => "list",
    })?;
    // A file opened for reading can't be written.
    if kind == "stream" && memstream::WRITE_METHODS.contains(&method.as_str()) {
        let read_only = with(id, |o| matches!(o, Object::Stream(m) if m.file.as_ref().is_some_and(|f| !f.writable)))?;
        if read_only {
            return Some(Err(format!("{method}: the file was opened for reading (fmOpenRead)")));
        }
    }
    match (kind, method.as_str()) {
        ("stream", "open") => Some(open_file(id, &arg(0).to_string_val(), if args.len() > 1 { arg(1).to_i64() } else { 0 })),
        ("stream", "copyfrom") => {
            let src = arg(0).to_string_val();
            let n = arg(1).to_i64();
            let bytes = with(&src, |o| match o {
                Object::Stream(s) => {
                    if n <= 0 {
                        s.pos = 0;
                    }
                    Some(s.read(if n <= 0 { usize::MAX } else { n as usize }))
                }
                _ => None,
            })
            .flatten();
            let bytes = bytes?;
            with(id, |o| if let Object::Stream(m) = o { m.write(&bytes) });
            Some(Ok(Value::Null))
        }
        ("bitmap", "loadfromfile") => Some(read_file(&arg(0).to_string_val()).and_then(|bytes| {
            with(id, |o| match o {
                Object::Bitmap(b) => b.load_bmp_bytes(&bytes),
                _ => Ok(()),
            })
            .unwrap_or(Ok(()))
            .map(|_| Value::Null)
        })),
        ("bitmap", "savetofile") => {
            let bytes = with(id, |o| match o {
                Object::Bitmap(b) => encode_bmp(&b.img),
                _ => Vec::new(),
            })?;
            Some(write_file(&arg(0).to_string_val(), &bytes).map(|_| Value::Null))
        }
        ("bitmap", "savetostream") => {
            let bytes = with(id, |o| match o {
                Object::Bitmap(b) => encode_bmp(&b.img),
                _ => Vec::new(),
            })?;
            with(&arg(0).to_string_val(), |o| if let Object::Stream(m) = o { m.write(&bytes) });
            Some(Ok(Value::Null))
        }
        ("bitmap", "loadfromstream") => {
            let bytes = with(&arg(0).to_string_val(), |o| match o {
                Object::Stream(m) => m.read(usize::MAX),
                _ => Vec::new(),
            })
            .unwrap_or_default();
            Some(
                with(id, |o| match o {
                    Object::Bitmap(b) => b.load_bmp_bytes(&bytes),
                    _ => Ok(()),
                })?
                .map(|_| Value::Null),
            )
        }
        ("bitmap", "draw") => {
            let src = match load_image(&arg(2)) {
                Ok(src) => src,
                Err(e) => return Some(Err(e)),
            };
            with(id, |o| if let Object::Bitmap(b) = o { b.draw(arg(0).to_i64(), arg(1).to_i64(), &src) });
            Some(Ok(Value::Null))
        }
        ("bitmap", "copyrect" | "stretchdraw") => {
            let rect = |v: &Value| {
                let r = v.to_string_val();
                let n = |p: &str| props(&r, p).to_i64();
                (n("left"), n("top"), n("right"), n("bottom"))
            };
            let (dest, src_value, src_rect) =
                if method == "copyrect" { (rect(&arg(0)), arg(1), Some(rect(&arg(2)))) } else { (rect(&arg(0)), arg(1), None) };
            let src = match load_image(&src_value) {
                Ok(src) => src,
                Err(e) => return Some(Err(e)),
            };
            let src_rect = src_rect.unwrap_or((0, 0, src.img.width as i64, src.img.height as i64));
            with(id, |o| if let Object::Bitmap(b) = o { b.copy_rect(dest, &src, src_rect) });
            Some(Ok(Value::Null))
        }
        ("imagelist", "addbmpfile" | "addbmphandle" | "insertbmpfile" | "insertbmphandle") => {
            let insert = method.starts_with("insert");
            let (at, source, mask) = if insert { (arg(0).to_i64(), arg(1), args.get(2)) } else { (i64::MAX, arg(0), args.get(1)) };
            let src = match load_image(&source) {
                Ok(src) => src,
                Err(e) => return Some(Err(e)),
            };
            let mask = mask.map(|m| m.to_i64() as u32 & 0xFFFFFF);
            with(id, |o| if let Object::ImageList(l) = o { l.insert(at.max(0) as usize, &src, mask); });
            Some(Ok(Value::Null))
        }
        // SaveToFile / LoadFromFile / …Stream (File$ or S, RowOffset,
        // ColOffset, MaxRows): rows of cells joined by the Separator.
        ("grid", "savetofile" | "savetostream") => {
            let (rows, cols, max) = grid_range(args);
            let text = with(id, |o| match o {
                Object::Grid(g) => g.to_text(rows, cols, max),
                _ => String::new(),
            })?;
            if method == "savetofile" {
                Some(write_file(&arg(0).to_string_val(), text.as_bytes()).map(|_| Value::Null))
            } else {
                with(&arg(0).to_string_val(), |o| if let Object::Stream(m) = o { m.write(text.as_bytes()) });
                Some(Ok(Value::Null))
            }
        }
        ("grid", "loadfromfile" | "loadfromstream") => {
            let (rows, cols, max) = grid_range(args);
            let bytes = if method == "loadfromfile" {
                match read_file(&arg(0).to_string_val()) {
                    Ok(b) => b,
                    Err(e) => return Some(Err(e)),
                }
            } else {
                with(&arg(0).to_string_val(), |o| match o {
                    Object::Stream(m) => m.read(usize::MAX),
                    _ => Vec::new(),
                })
                .unwrap_or_default()
            };
            let text = String::from_utf8_lossy(&bytes).into_owned();
            with(id, |o| if let Object::Grid(g) = o { g.load_text(&text, rows, cols, max) });
            Some(Ok(Value::Null))
        }
        // One item per line.
        ("list", "loadfromfile") => {
            let bytes = match read_file(&arg(0).to_string_val()) {
                Ok(b) => b,
                Err(e) => return Some(Err(e)),
            };
            let text = String::from_utf8_lossy(&bytes).into_owned();
            with(id, |o| if let Object::List(l) = o { l.load_text(&text) });
            Some(Ok(Value::Null))
        }
        ("list", "savetofile") => {
            let text = with(id, |o| match o {
                Object::List(l) => l.to_text(),
                _ => String::new(),
            })?;
            Some(write_file(&arg(0).to_string_val(), text.as_bytes()).map(|_| Value::Null))
        }
        ("imagelist", "getbmp") => {
            let i = arg(0).to_i64();
            with(id, |o| match o {
                Object::ImageList(l) if i >= 0 && (i as usize) < l.images.len() => {
                    Ok(v_str(&l.images[i as usize].data_url()))
                }
                _ => Ok(v_str("")),
            })
        }
        ("imagelist", "draw") => {
            // Draw(Target, X, Y, Index): onto a QBITMAP here; other targets
            // (a QCANVAS) are drawn by the runtime from `image_at`.
            let i = arg(3).to_i64();
            let image = with(id, |o| match o {
                Object::ImageList(l) if i >= 0 => l.images.get(i as usize).cloned(),
                _ => None,
            })??;
            let drawn = with(&arg(0).to_string_val(), |o| match o {
                Object::Bitmap(b) => {
                    b.draw(arg(1).to_i64(), arg(2).to_i64(), &image);
                    true
                }
                _ => false,
            });
            drawn.filter(|d| *d).map(|_| Ok(Value::Null))
        }
        _ => with(id, |o| match o {
            Object::Font(f) => f.call(&method, args),
            Object::Stream(m) => m.call(&method, args),
            Object::Bitmap(b) => b.call(&method, args),
            Object::ImageList(l) => l.call(&method, args),
            Object::ListView(l) => l.call(&method, args),
            Object::Grid(g) => g.call(&method, args),
            Object::List(l) => l.call(&method, args),
        })?
        .map(Ok)
        // A property read written like a call (`Icons.Count` compiled as one).
        .or_else(|| if args.is_empty() { get(id, &method).map(Ok) } else { None }),
    }
}

/// A grid file method's (RowOffset, ColOffset, MaxRows): all rows unless
/// MaxRows is given.
fn grid_range(args: &[Value]) -> (usize, usize, usize) {
    let n = |i: usize| args.get(i).map(|v| usize::try_from(v.to_i64()).unwrap_or(0));
    (n(1).unwrap_or(0), n(2).unwrap_or(0), n(3).filter(|&m| m > 0).unwrap_or(usize::MAX))
}

/// `File.Open(name, mode)` (RAPIDQ.INC: fmCreate = 65535, fmOpenRead = 0,
/// fmOpenWrite = 1, fmOpenReadWrite = 2): the stream holds the file's bytes
/// (none for fmCreate) and writes go through to the file.
fn open_file(id: &str, path: &str, mode: i64) -> Result<Value, String> {
    let create = mode == 65535;
    let writable = create || mode == 1 || mode == 2;
    let native = NATIVE_FILES.with(std::cell::Cell::get);
    let data = if create { Vec::new() } else { read_file(path)? };
    #[cfg(not(target_arch = "wasm32"))]
    let handle = if native && writable {
        let mut options = std::fs::OpenOptions::new();
        options.read(true).write(true);
        if create {
            options.create(true).truncate(true);
        }
        Some(options.open(path).map_err(|e| format!("can't open {path}: {e}"))?)
    } else {
        None
    };
    if create && !native {
        write_file(path, &[])?;
    }
    let sink = memstream::FileSink {
        path: path.to_string(),
        writable,
        #[cfg(not(target_arch = "wasm32"))]
        handle,
    };
    with(id, |o| {
        if let Object::Stream(m) = o {
            *m = MemStream { data, pos: 0, file: Some(sink) };
        }
    });
    Ok(v_int(-1))
}

/// A stream's bytes (web runtime: `File.Download`).
pub fn stream_bytes(id: &str) -> Option<Vec<u8>> {
    with(id, |o| match o {
        Object::Stream(m) => Some(m.data.clone()),
        _ => None,
    })?
}

/// Replaces a stream's content (web runtime: a picked or fetched file).
pub fn stream_load(id: &str, bytes: Vec<u8>) {
    with(id, |o| {
        if let Object::Stream(m) = o {
            m.data = bytes;
            m.pos = 0;
        }
    });
}

/// The image a value names: a QBITMAP's id, the `data:` URL of a bitmap's
/// `.BMP`, or a BMP file.
pub fn load_image(v: &Value) -> Result<Bitmap, String> {
    let s = v.to_string_val();
    if let Some(Some(b)) = with(&s, |o| match o {
        Object::Bitmap(b) => Some(b.clone()),
        _ => None,
    }) {
        return Ok(b);
    }
    let mut b = Bitmap::default();
    match s.strip_prefix(BMP_DATA_URL) {
        Some(data) => {
            let (data, fragment) = data.split_once('#').unwrap_or((data, ""));
            b.load_bmp_bytes(&base64_decode(data).ok_or("invalid BMP data")?)?;
            if let Some(Ok(color)) = fragment.strip_prefix("transparent=").map(str::parse::<u32>) {
                b.transparent = true;
                b.transparent_color = color;
            }
        }
        None => b.load_bmp_bytes(&read_file(&s)?)?,
    }
    Ok(b)
}

/// Image `index` of image list `id` (for drawing it onto a canvas).
pub fn image_at(id: &str, index: i64) -> Option<Bitmap> {
    with(id, |o| match o {
        Object::ImageList(l) if index >= 0 => l.images.get(index as usize).cloned(),
        _ => None,
    })?
}

/// The component properties `X.Font = F` sets, if `F` is a QFONT.
pub fn font_properties(id: &str) -> Option<Vec<(&'static str, Value)>> {
    with(id, |o| match o {
        Object::Font(f) => Some(f.component_properties()),
        _ => None,
    })?
}

/// `DIM lbl(1 TO 3) AS QLABEL`: an array holding one object id per element,
/// `lbl(1)`, `lbl(2)`, … (`grid(0,1)` for more dimensions), and those ids in
/// order. Runtimes create a component for each id.
pub fn object_ids(name: &str, bounds: &[(i64, i64)]) -> Result<(Value, Vec<String>), String> {
    let array = crate::BasicArray::new(bounds.to_vec(), Value::Null)?;
    let mut ids = Vec::with_capacity(array.data.len());
    let mut index: Vec<i64> = bounds.iter().map(|b| b.0).collect();
    for _ in 0..array.data.len() {
        let parts: Vec<String> = index.iter().map(i64::to_string).collect();
        ids.push(format!("{name}({})", parts.join(",")));
        // Row-major: the last index moves fastest.
        for d in (0..index.len()).rev() {
            if index[d] < bounds[d].1 {
                index[d] += 1;
                break;
            }
            index[d] = bounds[d].0;
        }
    }
    let data = ids.iter().map(|id| v_str(id)).collect();
    let value = Value::Array(std::rc::Rc::new(std::cell::RefCell::new(crate::BasicArray { bounds: bounds.to_vec(), data })));
    Ok((value, ids))
}

/// `__component_array(kind, name, lo1, hi1, …)` arguments: (kind, name, bounds).
pub fn component_array_args(args: &[Value]) -> (String, String, Vec<(i64, i64)>) {
    let kind = args.first().map(Value::to_string_val).unwrap_or_default();
    let name = args.get(1).map(Value::to_string_val).unwrap_or_default();
    let bounds = args.get(2..).unwrap_or(&[]).chunks(2).map(|b| (b[0].to_i64(), b.get(1).map_or(0, Value::to_i64))).collect();
    (kind, name, bounds)
}

/// Appends bytes to memory stream `id` (e.g. data a QFILESTREAM read, for
/// `Mem.CopyFrom(File, n)`).
pub fn stream_write(id: &str, bytes: &[u8]) -> bool {
    with(id, |o| match o {
        Object::Stream(m) => {
            m.write(bytes);
            true
        }
        _ => false,
    })
    .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{v_int, v_null};

    fn no_props(_: &str, _: &str) -> Value {
        v_null()
    }

    fn call_ok(id: &str, method: &str, args: &[Value]) -> Value {
        call(id, method, args, &no_props).expect("handled").expect("ok")
    }

    #[test]
    fn object_id_arrays() {
        let (a, ids) = object_ids("lbl", &[(1, 3)]).unwrap();
        assert_eq!(ids, ["lbl(1)", "lbl(2)", "lbl(3)"]);
        let Value::Array(a) = a else { panic!() };
        assert_eq!(a.borrow().get(&[2]).unwrap().to_string_val(), "lbl(2)");
        let (g, ids) = object_ids("g", &[(0, 1), (1, 2)]).unwrap();
        assert_eq!(ids, ["g(0,1)", "g(0,2)", "g(1,1)", "g(1,2)"]);
        let Value::Array(g) = g else { panic!() };
        assert_eq!(g.borrow().get(&[1, 1]).unwrap().to_string_val(), "g(1,1)");
    }

    #[test]
    fn objects_work_together() {
        assert!(create("T_Bmp", "RBITMAP") && create("t_mem", "RMEMORYSTREAM") && create("t_copy", "RMEMORYSTREAM"));
        assert!(!create("t_x", "RBUTTON"));
        set("t_bmp", "Width", &v_int(4)).unwrap().unwrap();
        set("t_bmp", "height", &v_int(3)).unwrap().unwrap();
        call_ok("t_bmp", "pset", &[v_int(1), v_int(2), v_int(0xFF)]);
        // Through a memory stream and back into another bitmap.
        call_ok("t_bmp", "savetostream", &[v_str("t_mem")]);
        call_ok("t_copy", "copyfrom", &[v_str("t_mem"), v_int(0)]);
        set("t_copy", "position", &v_int(0)).unwrap().unwrap();
        create("t_bmp2", "RBITMAP");
        call_ok("t_bmp2", "loadfromstream", &[v_str("t_copy")]);
        assert_eq!(call_ok("t_bmp2", "pixel", &[v_int(1), v_int(2)]).to_i64(), 0xFF);
        // `.BMP` round-trips through its data URL.
        let url = get("t_bmp", "bmp").unwrap();
        create("t_bmp3", "RBITMAP");
        set("t_bmp3", "bmp", &url).unwrap().unwrap();
        assert_eq!(get("t_bmp3", "width").unwrap().to_i64(), 4);
        assert!(set("t_bmp3", "bmp", &v_str("/no/such/file.bmp")).unwrap().is_err());
        // A transparent bitmap's `.BMP` keeps its transparent color.
        set("t_bmp", "transparent", &v_int(-1)).unwrap().unwrap();
        let drawn = load_image(&get("t_bmp", "bmp").unwrap()).unwrap();
        assert!(drawn.transparent && drawn.transparent_color == 0xFFFFFF);
        // A font's properties for `X.Font = F`.
        create("t_font", "RFONT");
        set("t_font", "size", &v_int(20)).unwrap().unwrap();
        assert!(font_properties("t_font").unwrap().contains(&("fontsize", v_int(20))));
        assert!(get("t_bmp", "nosuchprop").is_none() && get("t_nobody", "width").is_none());
    }
}

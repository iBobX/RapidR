//! RapidQ's non-visual objects — QFONT, QMEMORYSTREAM, QBITMAP and
//! QIMAGELIST — implemented once for every runtime. The desktop and web
//! runtimes create them in `rp_create_component` and route property and
//! method access here first (`get`/`set`/`call`), so both behave the same.
//!
//! Objects are keyed by component id (lowercase), like the runtimes'
//! component registries. An object argument (`Mem.CopyFrom(Other, 0)`,
//! `Bitmap.Draw(0, 0, Sprite)`) arrives as that id; an image can also be a
//! BMP file name or the `data:` URL a bitmap's `.BMP` property returns.

pub mod bevel;
pub mod bitmap;
pub mod codec;
pub mod dirtree;
pub mod tree;
pub mod font;
pub mod filelist;
pub mod grid;
pub mod header;
pub mod imagelist;
pub mod list;
pub mod listview;
pub mod memstream;
pub mod menu;
pub mod printer;
pub mod text;
pub mod tabcontrol;
pub mod textedit;
pub mod trackbar;

use std::cell::RefCell;
use std::collections::HashMap;

use crate::{v_int, v_str, Value};
use bitmap::Bitmap;
use codec::{base64_decode, encode_bmp, BMP_DATA_URL, SVG_DATA_URL};
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
    /// QDIRTREE's directories; the runtime shows its rows.
    DirTree(dirtree::DirTree),
    /// QTREEVIEW's nodes; the runtime shows them.
    Tree(tree::TreeView),
    /// The global PRINTER's document.
    Printer(printer::Printer),
    /// QEDIT's / QRICHEDIT's text and selection; the runtime shows them.
    Text(textedit::TextEdit),
    /// QTRACKBAR's range, position and ticks; the runtime draws its shapes.
    TrackBar(trackbar::TrackBar),
    /// QTABCONTROL's tabs and selection; the runtime draws its ops.
    TabControl(tabcontrol::TabControl),
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
    /// QHEADER's sections; its surface is a canvas in OBJECTS.
    static HEADERS: RefCell<HashMap<String, header::Header>> = RefCell::new(HashMap::new());
}

/// Sends a finished print job somewhere (`Printer.EndDoc`).
pub type PrintHook = fn(&printer::PrintJob) -> Result<(), String>;

thread_local! {
    static PRINT_HOOK: std::cell::Cell<PrintHook> = const { std::cell::Cell::new(default_print) };
}

/// Replaces where `Printer.EndDoc` sends documents (the web runtime: the
/// browser's print dialog).
pub fn set_print_hook(hook: PrintHook) {
    PRINT_HOOK.with(|h| h.set(hook));
}

/// Sends a print job to the runtime's print hook (LFLUSH, rapidr_value::lprint).
pub fn print_job(job: &printer::PrintJob) -> Result<(), String> {
    let hook = PRINT_HOOK.with(std::cell::Cell::get);
    hook(job)
}

/// The printers `Printer.Printers(i)` lists: the system's (CUPS `lpstat`),
/// read once; the web's hook prints through the browser.
pub fn printer_names() -> Vec<String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        static NAMES: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
        NAMES
            .get_or_init(|| {
                std::process::Command::new("lpstat")
                    .arg("-e")
                    .output()
                    .ok()
                    .filter(|o| o.status.success())
                    .map(|o| String::from_utf8_lossy(&o.stdout).lines().map(str::trim).filter(|l| !l.is_empty()).map(String::from).collect())
                    .unwrap_or_default()
            })
            .clone()
    }
    #[cfg(target_arch = "wasm32")]
    vec!["Browser".to_string()]
}

/// Where documents go by default: to `RAPIDR_PRINT_TO` (a PDF file, or a
/// directory to put it in) when set — tests never print on paper; else to
/// the chosen printer with `lp`; with no printer (or no `lp`), a PDF in the
/// current directory.
fn default_print(job: &printer::PrintJob) -> Result<(), String> {
    let name = {
        let t: String = job.title.chars().filter(|c| c.is_alphanumeric() || " -_".contains(*c)).collect();
        format!("{}.pdf", if t.trim().is_empty() { "RapidR print" } else { t.trim() })
    };
    let save = |path: std::path::PathBuf| -> Result<(), String> {
        std::fs::write(&path, &job.pdf).map_err(|e| format!("can't write {}: {e}", path.display()))?;
        eprintln!("[rapidr] printed to {}", path.display());
        Ok(())
    };
    if let Some(to) = std::env::var_os("RAPIDR_PRINT_TO") {
        let to = std::path::PathBuf::from(to);
        return save(if to.is_dir() { to.join(name) } else { to });
    }
    #[cfg(not(target_arch = "wasm32"))]
    if !job.printer.is_empty() || !printer_names().is_empty() {
        let file = std::env::temp_dir().join(format!("rapidr-print-{}.pdf", std::process::id()));
        std::fs::write(&file, &job.pdf).map_err(|e| format!("can't write {}: {e}", file.display()))?;
        let mut lp = std::process::Command::new("lp");
        if !job.printer.is_empty() {
            lp.args(["-d", &job.printer]);
        }
        lp.args(["-n", &job.copies.to_string()]);
        if !job.title.is_empty() {
            lp.args(["-t", &job.title]);
        }
        if lp.arg(&file).status().is_ok_and(|s| s.success()) {
            return Ok(());
        }
    }
    save(std::path::PathBuf::from(name))
}

/// Replaces how objects read and write files (the web runtime has no file
/// system).
pub fn set_file_io(reader: FileReader, writer: FileWriter) {
    FILE_IO.with(|io| *io.borrow_mut() = (reader, writer));
    NATIVE_FILES.with(|n| n.set(false));
}

pub(crate) fn read_file(path: &str) -> Result<Vec<u8>, String> {
    if let Some(bytes) = crate::resources::read_path(path) {
        return bytes;
    }
    let reader = FILE_IO.with(|io| io.borrow().0);
    reader(path)
}

pub(crate) fn write_file(path: &str, bytes: &[u8]) -> Result<(), String> {
    let writer = FILE_IO.with(|io| io.borrow().1);
    writer(path, bytes)
}

/// Creates the object if `type_name` is one of [`TYPES`]; `false` otherwise.
pub fn create(id: &str, type_name: &str) -> bool {
    if menu::create(id, type_name) {
        return true;
    }
    let object = match type_name.to_ascii_uppercase().as_str() {
        "RFONT" => Object::Font(Font::default()),
        "RMEMORYSTREAM" | "RFILESTREAM" => Object::Stream(MemStream::default()),
        "RBITMAP" => Object::Bitmap(Bitmap::default()),
        "RIMAGE" => Object::Bitmap(Bitmap { picture: true, ..Bitmap::default() }),
        "RCANVAS" => Object::Bitmap(Bitmap::new_canvas()),
        "RHEADER" => {
            HEADERS.with(|h| {
                h.borrow_mut().entry(id.to_lowercase()).or_default();
            });
            Object::Bitmap(Bitmap::new_canvas())
        }
        "RIMAGELIST" => Object::ImageList(ImageList::default()),
        "RLISTVIEW" => Object::ListView(ListView::default()),
        "RSTRINGGRID" => Object::Grid(StringGrid::default()),
        "RLISTBOX" => Object::List(ItemList::new(false)),
        "RSTRINGLIST" => Object::List(ItemList::new_string_list()),
        "RFILELISTBOX" => Object::List(ItemList::new_file_list()),
        "RDIRTREE" => Object::DirTree(dirtree::DirTree::default()),
        "RTREEVIEW" => Object::Tree(tree::TreeView::default()),
        "RCOMBOBOX" => Object::List(ItemList::new(true)),
        "REDIT" => Object::Text(textedit::TextEdit::new(false)),
        "RRICHEDIT" | "RMEMO" => Object::Text(textedit::TextEdit::new(true)),
        "RTRACKBAR" => Object::TrackBar(trackbar::TrackBar::default()),
        "RTABCONTROL" => Object::TabControl(tabcontrol::TabControl::default()),
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

/// Changes a QLISTVIEW (from what the user does to its control).
pub fn with_listview_mut<R>(id: &str, f: impl FnOnce(&mut ListView) -> R) -> Option<R> {
    with(id, |o| match o {
        Object::ListView(l) => Some(f(l)),
        _ => None,
    })?
}

/// An image list's image size (None: `id` isn't one).
fn imagelist_size(id: &str) -> Option<(i64, i64)> {
    if id.is_empty() {
        return None;
    }
    with(id, |o| match o {
        Object::ImageList(l) => Some((l.width, l.height)),
        _ => None,
    })?
}

fn imagelist_images(id: &str) -> Vec<Bitmap> {
    if id.is_empty() {
        return Vec::new();
    }
    with(id, |o| match o {
        Object::ImageList(l) => l.images.clone(),
        _ => Vec::new(),
    })
    .unwrap_or_default()
}

/// Gives QLISTVIEW `id`'s control its size, font and image sizes (the
/// runtime, before it paints it or passes it the mouse or a key).
pub fn listview_setup(id: &str, width: i64, height: i64, font: &Font, focused: bool) -> bool {
    let Some((small, large, state)) = with_listview(id, |lv| (lv.small_images.clone(), lv.large_images.clone(), lv.state_images.clone())) else {
        return false;
    };
    let view = listview::View { width, height, font: font.clone(), small: imagelist_size(&small), large: imagelist_size(&large), state: imagelist_size(&state) };
    with_listview_mut(id, |lv| {
        lv.set_view(view);
        lv.focused = focused;
    })
    .is_some()
}

/// QLISTVIEW `id`'s control as the screen shows it (after
/// [`listview_setup`]); `background`: its Color.
pub fn listview_paint(id: &str, background: u32) -> Option<Bitmap> {
    let (small, large, state) = with_listview(id, |lv| (lv.small_images.clone(), lv.large_images.clone(), lv.state_images.clone()))?;
    let images = listview::Images { small: imagelist_images(&small), large: imagelist_images(&large), state: imagelist_images(&state) };
    with_listview_mut(id, |lv| lv.paint(background, &images))
}

/// Whether `id` is a QLISTBOX or QCOMBOBOX (its widget redraws after a
/// change).
pub fn is_list(id: &str) -> bool {
    with(id, |o| matches!(o, Object::List(_))).unwrap_or(false)
}

/// Whether `id` is a QTREEVIEW.
pub fn is_tree(id: &str) -> bool {
    with(id, |o| matches!(o, Object::Tree(_))).unwrap_or(false)
}

/// Reads or changes a QTREEVIEW's nodes (to show them, or from its widget).
pub fn with_tree<R>(id: &str, f: impl FnOnce(&mut tree::TreeView) -> R) -> Option<R> {
    with(id, |o| match o {
        Object::Tree(t) => Some(f(t)),
        _ => None,
    })?
}

/// Whether `id` is a QHEADER.
pub fn is_header(id: &str) -> bool {
    HEADERS.with(|h| h.borrow().contains_key(&id.to_lowercase()))
}

/// Reads or changes a QHEADER's sections (to show them, or from the mouse).
pub fn with_header<R>(id: &str, f: impl FnOnce(&mut header::Header) -> R) -> Option<R> {
    HEADERS.with(|h| h.borrow_mut().get_mut(&id.to_lowercase()).map(f))
}

/// Paints QHEADER `id`'s faces on its surface, `width` × `height`; returns
/// the owner-drawn sections (index, pressed, rect) for OnDrawSection.
pub fn paint_header(id: &str, width: i64, height: i64) -> Vec<header::OwnerDrawn> {
    let Some(model) = with_header(id, |h| h.clone()) else { return Vec::new() };
    with_canvas(id, width, height, |b| {
        let font = b.font.clone();
        model.paint(b, &font)
    })
    .unwrap_or_default()
}

/// Whether `id` is a QDIRTREE (its widget shows its rows again after a
/// change).
pub fn is_dirtree(id: &str) -> bool {
    with(id, |o| matches!(o, Object::DirTree(_))).unwrap_or(false)
}

/// Reads or changes a QDIRTREE (to show it, or from its widget).
pub fn with_dirtree<R>(id: &str, f: impl FnOnce(&mut dirtree::DirTree) -> R) -> Option<R> {
    with(id, |o| match o {
        Object::DirTree(t) => Some(f(t)),
        _ => None,
    })?
}

/// Whether `id` is a QFILELISTBOX (setting its Directory fires OnChange).
pub fn is_file_list(id: &str) -> bool {
    with(id, |o| matches!(o, Object::List(l) if l.files.is_some())).unwrap_or(false)
}

/// Reads a QLISTBOX's / QCOMBOBOX's items (to draw them).
/// An owner-drawn list box's item `i` as the screen shows it — (width,
/// height, RGBA, scale): `scale` device pixels a pixel, see
/// `Bitmap::display_rgba` — `width` pixels wide, text in `font` (see
/// `list::ItemList::render_item`).
pub fn list_item_pixels(id: &str, i: usize, width: i64, font: &Font) -> Option<(usize, usize, Vec<u8>, usize)> {
    with_list(id, |l| l.render_item(i, width, font).display_rgba())
}

/// Image `i` of QIMAGELIST `id` as the screen shows it (width, height,
/// RGBA, scale): a tree node's icon.
pub fn imagelist_pixels(id: &str, i: i64) -> Option<(usize, usize, Vec<u8>, usize)> {
    with(id, |o| match o {
        Object::ImageList(l) => usize::try_from(i).ok().and_then(|i| l.images.get(i)).map(|b| b.clone().display_rgba()),
        _ => None,
    })?
}

/// A tree node's icon as the screen shows it — (width, height, RGBA,
/// scale): its state image (`StateIndex` in `StateImages`; index 0 is
/// none, as in Windows) and then its image (`image` in `Images`), side by
/// side, 2 pixels apart; `None` when it has neither.
pub fn tree_icon(images: &str, state_images: &str, image: i64, state: i64) -> Option<(usize, usize, Vec<u8>, usize)> {
    let state = (state > 0 && !state_images.is_empty()).then(|| imagelist_pixels(state_images, state)).flatten();
    let image = (!images.is_empty()).then(|| imagelist_pixels(images, image)).flatten();
    let (a, b) = match (state, image) {
        (None, None) => return None,
        (Some(one), None) | (None, Some(one)) => return Some(one),
        (Some(a), Some(b)) => (a, b),
    };
    let scale = a.3.max(b.3);
    let gap = 2 * scale;
    let (w, h) = (a.0 + gap + b.0, a.1.max(b.1));
    let mut rgba = vec![0u8; w * h * 4];
    for (x0, (pw, ph, px, _)) in [(0, &a), (a.0 + gap, &b)] {
        let y0 = (h - ph) / 2;
        for y in 0..*ph {
            let (from, to) = (y * pw * 4, ((y0 + y) * w + x0) * 4);
            rgba[to..to + pw * 4].copy_from_slice(&px[from..from + pw * 4]);
        }
    }
    Some((w, h, rgba, scale))
}

pub fn with_list<R>(id: &str, f: impl FnOnce(&ItemList) -> R) -> Option<R> {
    with(id, |o| match o {
        Object::List(l) => Some(f(l)),
        _ => None,
    })?
}

/// A QMEMORYSTREAM / QFILESTREAM's buffer (`memory`: its Pointer).
pub fn with_stream<R>(id: &str, f: impl FnOnce(&MemStream) -> R) -> Option<R> {
    with(id, |o| match o {
        Object::Stream(s) => Some(f(s)),
        _ => None,
    })?
}

pub fn with_stream_mut<R>(id: &str, f: impl FnOnce(&mut MemStream) -> R) -> Option<R> {
    with(id, |o| match o {
        Object::Stream(s) => Some(f(s)),
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

/// Every QSTRINGGRID's name (lowercase).
pub fn grid_names() -> Vec<String> {
    OBJECTS.with(|o| o.borrow().iter().filter(|(_, v)| matches!(v, Object::Grid(_))).map(|(k, _)| k.clone()).collect())
}

/// Whether `id` is a QEDIT / QRICHEDIT (its text model is here).
pub fn is_textedit(id: &str) -> bool {
    with(id, |o| matches!(o, Object::Text(_))).unwrap_or(false)
}

/// QEDIT / QRICHEDIT CopyToClipboard, CutToClipboard, PasteFromClipboard
/// (and Copy / Cut / Paste) with the runtime's clipboard; `None` for other
/// methods.
pub fn textedit_clipboard(id: &str, method: &str, clip_get: &dyn Fn() -> String, clip_set: &dyn Fn(&str)) -> Option<Value> {
    let m = method.to_lowercase();
    if !matches!(m.as_str(), "copytoclipboard" | "copy" | "cuttoclipboard" | "cut" | "pastefromclipboard" | "paste") || !is_textedit(id) {
        return None;
    }
    if m.starts_with("paste") {
        let text = clip_get();
        with_textedit_mut(id, |t| {
            if !t.read_only {
                t.replace_selection(&text);
            }
        });
    } else {
        let (sel, read_only) = with_textedit(id, |t| (t.get("seltext").map(|v| v.to_string_val()).unwrap_or_default(), t.read_only)).unwrap_or_default();
        if !sel.is_empty() {
            clip_set(&sel);
        }
        if m.starts_with("cut") && !read_only {
            with_textedit_mut(id, |t| t.replace_selection(""));
        }
    }
    Some(Value::Null)
}

/// Reads a QEDIT's / QRICHEDIT's text model (to show it).
pub fn is_tabcontrol(id: &str) -> bool {
    with(id, |o| matches!(o, Object::TabControl(_))).unwrap_or(false)
}

/// A QTABCONTROL's model, to read.
pub fn with_tabcontrol<R>(id: &str, f: impl FnOnce(&tabcontrol::TabControl) -> R) -> Option<R> {
    with(id, |o| match o {
        Object::TabControl(t) => Some(f(t)),
        _ => None,
    })?
}

/// A QTABCONTROL's model, to change (the user's clicks and keys).
pub fn with_tabcontrol_mut<R>(id: &str, f: impl FnOnce(&mut tabcontrol::TabControl) -> R) -> Option<R> {
    with(id, |o| match o {
        Object::TabControl(t) => Some(f(t)),
        _ => None,
    })?
}

pub fn is_trackbar(id: &str) -> bool {
    with(id, |o| matches!(o, Object::TrackBar(_))).unwrap_or(false)
}

/// A QTRACKBAR's model, to read.
pub fn with_trackbar<R>(id: &str, f: impl FnOnce(&trackbar::TrackBar) -> R) -> Option<R> {
    with(id, |o| match o {
        Object::TrackBar(t) => Some(f(t)),
        _ => None,
    })?
}

/// A QTRACKBAR's model, to change (the user's keys and mouse).
pub fn with_trackbar_mut<R>(id: &str, f: impl FnOnce(&mut trackbar::TrackBar) -> R) -> Option<R> {
    with(id, |o| match o {
        Object::TrackBar(t) => Some(f(t)),
        _ => None,
    })?
}

pub fn with_textedit<R>(id: &str, f: impl FnOnce(&textedit::TextEdit) -> R) -> Option<R> {
    with(id, |o| match o {
        Object::Text(t) => Some(f(t)),
        _ => None,
    })?
}

/// Changes a QEDIT's / QRICHEDIT's text model from its widget (the user
/// typed or selected).
pub fn with_textedit_mut<R>(id: &str, f: impl FnOnce(&mut textedit::TextEdit) -> R) -> Option<R> {
    with(id, |o| match o {
        Object::Text(t) => Some(f(t)),
        _ => None,
    })?
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

/// Whether `id` is a QIMAGE (its runtime widget shows its picture again
/// after a change).
pub fn is_picture(id: &str) -> bool {
    with(id, |o| matches!(o, Object::Bitmap(b) if b.picture)).unwrap_or(false)
}

/// Reads a QIMAGE's picture (to show it).
pub fn with_picture<R>(id: &str, f: impl FnOnce(&mut Bitmap) -> R) -> Option<R> {
    with(id, |o| match o {
        Object::Bitmap(b) if b.picture => Some(f(b)),
        _ => None,
    })?
}

/// Whether `id` is a QCANVAS (its runtime widget shows the surface again
/// after a change).
pub fn is_canvas(id: &str) -> bool {
    with(id, |o| matches!(o, Object::Bitmap(b) if b.canvas)).unwrap_or(false)
}

/// The font a component's properties describe (`Font = Font`, `Font.Size = …`
/// keep them as `fontname`, `fontsize` (points), `fontcolor`, `fontbold`, …):
/// what text drawn for the component (a form's surface, a list's items) uses.
pub fn font_from_props(id: &str, props: &dyn Fn(&str, &str) -> Value) -> Font {
    let name = props(id, "fontname").to_string_val();
    let flag = |p: &str| props(id, p).to_bool();
    let size = props(id, "fontsize").to_i64();
    Font {
        name: if name.trim().is_empty() { "Arial".into() } else { name },
        size: if size > 0 { size } else { 10 },
        color: props(id, "fontcolor").to_i64() & 0xFFFFFF,
        styles: u8::from(flag("fontbold")) | u8::from(flag("fontitalic")) << 1 | u8::from(flag("fontunderline")) << 2 | u8::from(flag("fontstrikeout")) << 3,
    }
}

/// Gives form `id` its own drawing surface (`Form.TextOut`, …) the first
/// time it's drawn on; `color` is the form's.
pub fn create_form_surface(id: &str, color: i64) {
    OBJECTS.with(|o| {
        o.borrow_mut().entry(id.to_lowercase()).or_insert_with(|| Object::Bitmap(Bitmap::new_form_surface(color as u32)));
    });
}

/// A form's `Color` as a &HBBGGRR integer: the runtimes keep it as an
/// integer or as a `#rrggbb` string (themes, the web); a form without one
/// is the usual light grey.
pub fn form_color(v: &Value) -> i64 {
    match v {
        Value::Integer(_) | Value::Double(_) => v.to_i64() & 0xFFFFFF,
        Value::Null => 0xF0F0F0,
        other => {
            let s = other.to_string_val();
            match s.strip_prefix('#').filter(|h| h.len() == 6).and_then(|h| u32::from_str_radix(h, 16).ok()) {
                Some(rgb) => i64::from((rgb & 0xFF) << 16 | (rgb & 0xFF00) | rgb >> 16),
                None if s.trim().is_empty() => 0xF0F0F0,
                None => other.to_i64() & 0xFFFFFF,
            }
        }
    }
}

/// Whether the form `id` has a drawing surface.
pub fn is_form_surface(id: &str) -> bool {
    with(id, |o| matches!(o, Object::Bitmap(b) if b.form)).unwrap_or(false)
}

/// Whether `method` draws (or measures) on a bitmap / canvas — what a QFORM
/// gets a surface for.
pub fn is_drawing_method(method: &str) -> bool {
    matches!(
        method,
        "pset" | "line" | "rectangle" | "fillrect" | "circle" | "roundrect" | "paint" | "draw" | "copyrect" | "stretchdraw"
            | "textout" | "textwidth" | "textheight" | "pixel" | "cls" | "clear" | "drawtext" | "fillcircle" | "ellipse" | "setpixel"
    )
}

/// Reads a QCANVAS's surface (to show it), first giving it the control's
/// size `width` × `height`.
pub fn with_canvas<R>(id: &str, width: i64, height: i64, f: impl FnOnce(&mut Bitmap) -> R) -> Option<R> {
    with(id, |o| match o {
        Object::Bitmap(b) if b.canvas => {
            b.fit(width, height);
            Some(f(b))
        }
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
/// The global PRINTER exists once it's used.
fn ensure_printer(id: &str) {
    if id.eq_ignore_ascii_case("printer") && !exists(id) {
        OBJECTS.with(|o| o.borrow_mut().insert("printer".into(), Object::Printer(printer::Printer::default())));
    }
}

pub fn get(id: &str, prop: &str) -> Option<Value> {
    ensure_printer(id);
    let prop = prop.to_lowercase();
    if let Some(v) = menu::get(id, &prop) {
        return Some(v);
    }
    // `Mem.Pointer`: the address of its first byte (crate::memory).
    if prop == "pointer" && with(id, |o| matches!(o, Object::Stream(_))) == Some(true) {
        return Some(Value::Integer(crate::memory::stream_pointer(id)));
    }
    if let Some(v) = with_header(id, |h| h.get(&prop)).flatten() {
        return Some(v);
    }
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
        Object::DirTree(t) => t.get(&prop),
        Object::Tree(t) => t.get(&prop),
        Object::Printer(p) => p.get(&prop),
        Object::Text(t) => t.get(&prop),
        Object::TrackBar(t) => t.get(&prop),
        Object::TabControl(t) => t.get(&prop),
    })?
}

/// Sets a property; `Some(Ok)` if handled, `Some(Err)` if it failed (a BMP
/// that can't be loaded), `None` if not an object property.
pub fn set(id: &str, prop: &str, val: &Value) -> Option<Result<(), String>> {
    ensure_printer(id);
    let prop = prop.to_lowercase();
    if menu::is_menu(id) {
        return menu::set(id, &prop, val).map(Ok);
    }
    if with_header(id, |h| h.set(&prop, val)) == Some(true) {
        return Some(Ok(()));
    }
    // `Printer.Font = Font` / `Bitmap.Font = Font`: the QFONT's settings.
    if prop == "font" && matches!(with(id, |o| matches!(o, Object::Printer(_)) || matches!(o, Object::Bitmap(b) if !b.form)), Some(true)) {
        let font = with(&val.to_string_val(), |o| match o {
            Object::Font(f) => Some(f.clone()),
            _ => None,
        })
        .flatten()?;
        with(id, |o| match o {
            Object::Printer(p) => p.font = font,
            Object::Bitmap(b) => b.font = font,
            _ => {}
        });
        return Some(Ok(()));
    }
    // `Tab.TabInactiveFont = Font`: the QFONT's settings, for its inactive tabs.
    if prop == "tabinactivefont" && is_tabcontrol(id) {
        let font = with(&val.to_string_val(), |o| match o {
            Object::Font(f) => Some(f.clone()),
            _ => None,
        })
        .flatten()?;
        with_tabcontrol_mut(id, |t| {
            t.inactive_font = Some(font);
            t.revision += 1;
        });
        return Some(Ok(()));
    }
    // `BMPHandle = GRID_BMP`: a `$RESOURCE` (rapidr_value::resources).
    // (a QIMAGE's ICOHandle / Icon: an icon is its picture)
    let icon = matches!(prop.as_str(), "icohandle" | "icon") && matches!(with(id, |o| matches!(o, Object::Bitmap(b) if b.picture)), Some(true));
    if (icon || matches!(prop.as_str(), "bmp" | "bmphandle")) && matches!(with(id, |o| matches!(o, Object::Bitmap(b) if !b.form)), Some(true)) {
        let loaded = load_image(val);
        return Some(loaded.map(|mut src| {
            with(id, |o| {
                if let Object::Bitmap(b) = o {
                    if src.transparent {
                        (b.transparent, b.transparent_color) = (true, src.transparent_color);
                    }
                    b.take_display(&mut src);
                    b.img = src.img;
                    // (an image with soft edges has its own transparency)
                    b.alpha = src.alpha;
                    if b.alpha.is_none() {
                        b.auto_transparent_color();
                    }
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
        Object::DirTree(t) => t.set(&prop, val).map(|_| Ok(())),
        Object::Tree(t) => t.set(&prop, val).then_some(Ok(())),
        Object::Printer(p) => p.set(&prop, val).then_some(Ok(())),
        Object::Text(t) => t.set(&prop, val).then_some(Ok(())),
        Object::TrackBar(t) => t.set(&prop, val).then_some(Ok(())),
        Object::TabControl(t) => t.set(&prop, val).then_some(Ok(())),
    })?
}

/// Reads a property of another component (for QRECT arguments, which are
/// plain property bags in the runtime).
pub type PropReader<'a> = &'a dyn Fn(&str, &str) -> Value;

/// A QRECT argument's (Left, Top, Right, Bottom): a `DIM R AS QRECT` (a
/// record), or a component's property bag by name (OnDrawItem's Rect).
pub fn rect_of(v: &Value, props: PropReader) -> (i64, i64, i64, i64) {
    if let Value::Object(inst) = v {
        let fields = inst.fields.borrow();
        let n = |p: &str| inst.names.iter().position(|f| f.eq_ignore_ascii_case(p)).and_then(|i| fields.get(i)).map_or(0, Value::to_i64);
        return (n("left"), n("top"), n("right"), n("bottom"));
    }
    let r = v.to_string_val();
    let n = |p: &str| props(&r, p).to_i64();
    (n("left"), n("top"), n("right"), n("bottom"))
}

/// Calls a method; `None` if `id` isn't an object or it has no such method.
/// `Some(Err)` is a failure to report (e.g. a file that can't be read).
pub fn call(id: &str, method: &str, args: &[Value], props: PropReader) -> Option<Result<Value, String>> {
    ensure_printer(id);
    let method = method.to_lowercase();
    if let Some(v) = menu::call(id, &method, args) {
        return Some(Ok(v));
    }
    if let Some(v) = with_header(id, |h| h.call(&method, args)).flatten() {
        return Some(Ok(v));
    }
    let arg = |i: usize| args.get(i).cloned().unwrap_or(Value::Null);
    // QRICHEDIT LoadFromFile / SaveToFile: the text, lines ending CR LF.
    if is_textedit(id) && matches!(method.as_str(), "loadfromfile" | "savetofile") {
        let path = arg(0).to_string_val();
        return Some(if method == "loadfromfile" {
            read_file(&path).map(|bytes| {
                let text: String = bytes.iter().map(|&b| char::from(b)).collect();
                with(id, |o| {
                    if let Object::Text(t) = o {
                        t.set_text(&text);
                    }
                });
                Value::Null
            })
        } else {
            let text = with(id, |o| match o {
                Object::Text(t) => t.text(),
                _ => String::new(),
            })
            .unwrap_or_default();
            write_file(&path, &text.chars().map(|c| c as u32 as u8).collect::<Vec<u8>>()).map(|_| Value::Null)
        });
    }
    // QSTRINGLIST AddList(Other): the other list's strings appended.
    if method == "addlist" {
        let other = with(&arg(0).to_string_val(), |o| match o {
            Object::List(l) => Some(l.items.clone()),
            _ => None,
        })
        .flatten()?;
        let args: Vec<Value> = other.into_iter().map(Value::String).collect();
        return with(id, |o| match o {
            Object::List(l) => l.call("additems", &args).map(Ok),
            _ => None,
        })
        .flatten();
    }
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
        Object::DirTree(_) => "dirtree",
        Object::Tree(_) => "tree",
        Object::Printer(_) => "printer",
        Object::Text(_) => "text",
        Object::TrackBar(_) => "trackbar",
        Object::TabControl(_) => "tabcontrol",
    })?;
    // A file opened for reading can't be written.
    if kind == "stream" && memstream::WRITE_METHODS.contains(&method.as_str()) {
        let read_only = with(id, |o| matches!(o, Object::Stream(m) if m.file.as_ref().is_some_and(|f| !f.writable)))?;
        if read_only {
            return Some(Err(format!("{method}: the file was opened for reading (fmOpenRead)")));
        }
    }
    // Drawing on a QIMAGE without a picture: first one the control's size
    // (read before borrowing the registry: `props` may read objects too).
    let drawing = matches!(method.as_str(), "pset" | "line" | "rectangle" | "fillrect" | "circle" | "roundrect" | "paint" | "draw" | "copyrect" | "stretchdraw")
        || (method == "pixel" && args.len() >= 3);
    // A QCANVAS is always the control's size; a QFORM's surface its client
    // area's, drawing in the form's font.
    if let Some(form) = with(id, |o| matches!(o, Object::Bitmap(b) if b.canvas).then(|| matches!(o, Object::Bitmap(b) if b.form)))? {
        let (w, h) = if form { (props(id, "clientwidth").to_i64(), props(id, "clientheight").to_i64()) } else { (props(id, "width").to_i64(), props(id, "height").to_i64()) };
        let font = form.then(|| font_from_props(id, props));
        with(id, |o| {
            if let Object::Bitmap(b) = o {
                b.fit(w, h);
                if let Some(f) = font {
                    b.font = f;
                }
            }
        });
    }
    if drawing && with(id, |o| matches!(o, Object::Bitmap(b) if b.picture && b.img.pixels.is_empty()))? {
        let (w, h) = (props(id, "width").to_i64(), props(id, "height").to_i64());
        with(id, |o| if let Object::Bitmap(b) = o { b.resize(w, h) });
    }
    match (kind, method.as_str()) {
        // A TYPE's bytes as RapidQ lays them out (crate::memory).
        ("stream", "writeudt") => {
            let bytes = crate::memory::udt_bytes(&arg(0));
            with(id, |o| if let Object::Stream(m) = o { m.write(&bytes) });
            Some(Ok(Value::Null))
        }
        ("stream", "readudt") => {
            let n = crate::memory::udt_bytes(&arg(0)).len();
            let bytes = with(id, |o| match o {
                Object::Stream(m) => m.read(n),
                _ => Vec::new(),
            })?;
            crate::memory::udt_from_bytes(&arg(0), &bytes);
            Some(Ok(Value::Null))
        }
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
        ("printer", "enddoc") => {
            let job = with(id, |o| match o {
                Object::Printer(p) => p.end_doc(),
                _ => None,
            })
            .flatten();
            let hook = PRINT_HOOK.with(std::cell::Cell::get);
            Some(job.map_or(Ok(()), |j| hook(&j)).map(|_| Value::Null))
        }
        // Printer.Draw(x, y, BMP) / StretchDraw(Rect, BMP) / CopyRect(D, Image, S).
        ("printer", "draw" | "stretchdraw" | "copyrect") => {
            let rect = |v: &Value| rect_of(v, props);
            let source = if method == "draw" { arg(2) } else { arg(1) };
            let mut src = match load_image(&source) {
                Ok(src) => src,
                Err(e) => return Some(Err(e)),
            };
            let (x, y, w, h) = match method.as_str() {
                "draw" => (arg(0).to_i64(), arg(1).to_i64(), src.img.width as i64, src.img.height as i64),
                _ => {
                    let (l, t, r, b) = rect(&arg(0));
                    (l, t, r - l, b - t)
                }
            };
            // CopyRect copies part of the image (S).
            if method == "copyrect" {
                let s = rect(&arg(2));
                let mut part = Bitmap::default();
                part.resize(s.2 - s.0, s.3 - s.1);
                part.copy_rect((0, 0, s.2 - s.0, s.3 - s.1), &src, s);
                src = part;
            }
            with(id, |o| if let Object::Printer(p) = o { p.draw(printer::PageOp::Image(x, y, w, h, src)) });
            Some(Ok(Value::Null))
        }
        // OnDrawCell's `Sender.Draw(x, y, Bitmap.BMP)` on a grid.
        ("grid", "draw") => {
            let src = match load_image(&arg(2)) {
                Ok(src) => src,
                Err(e) => return Some(Err(e)),
            };
            let (x, y) = (arg(0).to_i64(), arg(1).to_i64());
            with(id, |o| if let Object::Grid(g) = o { g.record(x, y, |l, t| grid::CellDraw::Image(x - l, y - t, src)) });
            Some(Ok(Value::Null))
        }
        // OnDrawItem's `Sender.Draw(x, y, Bitmap.BMP)` on a list box.
        ("list", "draw") => {
            let src = match load_image(&arg(2)) {
                Ok(src) => src,
                Err(e) => return Some(Err(e)),
            };
            let (x, y) = (arg(0).to_i64(), arg(1).to_i64());
            with(id, |o| if let Object::List(l) = o { l.record(x, y, |l, t| grid::CellDraw::Image(x - l, y - t, src)) });
            Some(Ok(Value::Null))
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
            let rect = |v: &Value| rect_of(v, props);
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
        // (…ICOFile / …ICOHandle: an icon keeps its own see-through parts)
        ("imagelist", "addbmpfile" | "addbmphandle" | "insertbmpfile" | "insertbmphandle" | "addicofile" | "addicohandle" | "inserticofile" | "inserticohandle") => {
            let insert = method.starts_with("insert");
            let (at, source, mask) = if insert { (arg(0).to_i64(), arg(1), args.get(2)) } else { (i64::MAX, arg(0), args.get(1)) };
            let src = match load_image(&source) {
                Ok(src) => src,
                Err(e) => return Some(Err(e)),
            };
            let mask = mask.map(|m| m.to_i64() as u32 & 0xFFFFFF);
            let icon = method.contains("ico");
            with(id, |o| {
                if let Object::ImageList(l) = o {
                    if icon {
                        l.insert_icon(at.max(0) as usize, &src);
                    } else {
                        l.insert(at.max(0) as usize, &src, mask);
                    }
                }
            });
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
        // A node per line, a tab per level.
        ("tree", "loadfromfile") => {
            let bytes = match read_file(&arg(0).to_string_val()) {
                Ok(b) => b,
                Err(e) => return Some(Err(e)),
            };
            let text = String::from_utf8_lossy(&bytes).into_owned();
            with(id, |o| if let Object::Tree(t) = o { t.load_text(&text) });
            Some(Ok(Value::Null))
        }
        ("tree", "savetofile") => {
            let text = with(id, |o| match o {
                Object::Tree(t) => t.to_text(),
                _ => String::new(),
            })?;
            Some(write_file(&arg(0).to_string_val(), text.as_bytes()).map(|_| Value::Null))
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
        ("imagelist", "getbmp" | "getico") => {
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
        _ => call_object(id, &method, args),
    }
}

fn call_object(id: &str, method: &str, args: &[Value]) -> Option<Result<Value, String>> {
    with(id, |o| match o {
        Object::Font(f) => f.call(method, args),
        Object::Stream(m) => m.call(method, args),
        Object::Bitmap(b) => b.call(method, args),
        Object::ImageList(l) => l.call(method, args),
        Object::ListView(l) => l.call(method, args),
        Object::Grid(g) => g.call(method, args),
        Object::List(l) => l.call(method, args),
        Object::DirTree(t) => t.call(method, args),
        Object::Tree(t) => t.call(method, args),
        Object::Printer(p) => p.call(method, args),
        Object::Text(t) => t.call(method, args),
        Object::TrackBar(t) => t.call(method, args),
        Object::TabControl(t) => t.call(method, args),
    })?
    .map(Ok)
    // A property read written like a call (`Icons.Count` compiled as one).
    .or_else(|| if args.is_empty() { get(id, method).map(Ok) } else { None })
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
    // A `$RESOURCE` handle (`AddBMPHandle GRID_BMP`).
    if matches!(v, Value::Integer(_) | Value::Double(_)) {
        let bytes = crate::resources::bytes(v.to_i64()).ok_or_else(|| format!("no resource {}", v.to_i64()))?;
        let mut b = Bitmap::default();
        b.load_bmp_bytes(&bytes)?;
        return Ok(b);
    }
    let s = v.to_string_val();
    if let Some(Some(b)) = with(&s, |o| match o {
        Object::Bitmap(b) => Some(b.clone()),
        _ => None,
    }) {
        return Ok(b);
    }
    let mut b = Bitmap::default();
    match s.strip_prefix(BMP_DATA_URL).or_else(|| s.strip_prefix(SVG_DATA_URL)) {
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

/// Whether an `Icon` / `IcoHandle` value names an icon (not unset or 0).
pub fn has_icon(v: &Value) -> bool {
    match v {
        Value::Integer(n) => *n != 0,
        Value::Double(d) => *d != 0.0,
        Value::Null => false,
        _ => !v.to_string_val().trim().is_empty(),
    }
}

/// A window icon as the screen shows it — (width, height, RGBA, scale) —
/// from `Icon` (a file) or `IcoHandle` (a `$RESOURCE`): an ICO, BMP, PNG or
/// SVG; `None` when there's none or it can't be read.
pub fn icon_pixels(v: &Value) -> Option<(usize, usize, Vec<u8>, usize)> {
    if !has_icon(v) {
        return None;
    }
    let mut b = load_image(v).ok()?;
    (b.img.width > 0 && b.img.height > 0).then(|| b.display_rgba())
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

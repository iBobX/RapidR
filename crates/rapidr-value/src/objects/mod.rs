//! RapidQ's non-visual objects — QFONT, QMEMORYSTREAM, QBITMAP and
//! QIMAGELIST — implemented once for every runtime. The desktop and web
//! runtimes create them in `rp_create_component` and route property and
//! method access here first (`get`/`set`/`call`), so both behave the same.
//!
//! Objects are keyed by component id (lowercase), like the runtimes'
//! component registries. An object argument (`Mem.CopyFrom(Other, 0)`,
//! `Bitmap.Draw(0, 0, Sprite)`) arrives as that id; an image can also be a
//! BMP file name or the `data:` URL a bitmap's `.BMP` property returns.

pub mod a11y;
pub mod avi;
pub mod bevel;
pub mod bitmap;
pub mod cgi;
pub mod code;
pub mod comport;
pub mod codec;
pub mod d3d;
pub mod design;
pub mod digdisplay;
pub mod directx;
pub mod download;
pub mod joystick;
pub mod dirtree;
pub mod tree;
pub mod font;
pub mod glass;
pub mod filelist;
pub mod grid;
pub mod header;
pub mod icons;
pub mod imagelist;
pub mod list;
pub mod listview;
pub mod media;
pub mod memstream;
pub mod midifile;
pub mod menu;
pub mod ops;
pub mod printer;
pub mod record;
pub mod rqlib;
pub mod synth;
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
    /// QREGISTRY: its root and open key (the keys: crate::registry's store).
    Registry(crate::registry::Registry),
    /// RDESIGNSURFACE's designed components and selection; the runtime
    /// draws its ops and passes it the mouse.
    Design(design::DesignSurface),
    /// QDXSCREEN's back buffer and what the last Flip showed (directx.rs).
    DxScreen(directx::DxScreen),
    /// QDXIMAGELIST's pictures (a DelphiX image library).
    DxImageList(directx::DxImageList),
    /// QDXTIMER's frame counter (FrameRate).
    DxTimer(directx::DxTimer),
    /// QDXSOUND's sound and where it plays.
    DxSound(directx::DxSound),
    /// QDXJOYSTICK's state (joystick.rs).
    DxJoystick(joystick::DxJoystick),
    /// QRECT's / QNOTIFYICONDATA's fields (record.rs).
    Record(record::Record),
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
    /// QDIGDISPLAY's Display (its surface is a canvas in OBJECTS).
    static DIGITS: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
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

pub fn read_file(path: &str) -> Result<Vec<u8>, String> {
    if let Some(bytes) = crate::resources::read_path(path) {
        return bytes;
    }
    let reader = FILE_IO.with(|io| io.borrow().0);
    reader(path)
}

pub fn write_file(path: &str, bytes: &[u8]) -> Result<(), String> {
    let writer = FILE_IO.with(|io| io.borrow().1);
    writer(path, bytes)
}

/// Creates the object if `type_name` is one of [`TYPES`]; `false` otherwise.
pub fn create(id: &str, type_name: &str) -> bool {
    // (the newest of its type: what `QFORM` as a value is; a QFORMMDI is
    // created as a QFORM, and is the newest QFORMMDI too)
    crate::note_made(type_name, crate::Made::Component(id.to_string()));
    if type_name.eq_ignore_ascii_case("RFORM") && crate::mdi::is_mdi(id) {
        crate::note_made("RFORMMDI", crate::Made::Component(id.to_string()));
    }
    if menu::create(id, type_name) {
        return true;
    }
    // (the DirectX lane's: QD3DFRAME … — the scene's own store)
    if d3d::create(id, type_name) {
        return true;
    }
    // (the I/O and media lane's: QCGI … — their own store, rqlib.rs)
    if rqlib::create(id, type_name) {
        return true;
    }
    let object = match type_name.to_ascii_uppercase().as_str() {
        // (its Color clWindowText, as RapidQ's: RC.EXE)
        "RFONT" => Object::Font(Font { color: crate::component_defaults::CL_WINDOW_TEXT, ..Font::default() }),
        "RMEMORYSTREAM" | "RFILESTREAM" => Object::Stream(MemStream::default()),
        "RBITMAP" => Object::Bitmap(Bitmap::default()),
        "RIMAGE" => Object::Bitmap(Bitmap { picture: true, ..Bitmap::default() }),
        "RCANVAS" => Object::Bitmap(Bitmap::new_canvas()),
        // (a canvas showing its Display: digdisplay.rs)
        "RDIGDISPLAY" => {
            DIGITS.with(|d| d.borrow_mut().insert(id.to_lowercase(), "0".into()));
            let mut b = Bitmap::new_canvas();
            let (w, h) = digdisplay::size("0");
            b.fit(w, h);
            digdisplay::draw(&mut b, "0");
            Object::Bitmap(b)
        }
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
        "RCODEEDITOR" => Object::Text(textedit::TextEdit::code()),
        "RTRACKBAR" => Object::TrackBar(trackbar::TrackBar::default()),
        "RTABCONTROL" => Object::TabControl(tabcontrol::TabControl::default()),
        "RREGISTRY" => Object::Registry(crate::registry::Registry::default()),
        "RDESIGNSURFACE" => Object::Design(design::DesignSurface::default()),
        "RDXSCREEN" => Object::DxScreen(directx::DxScreen::default()),
        "RDXIMAGELIST" => Object::DxImageList(directx::DxImageList::default()),
        "RDXTIMER" => Object::DxTimer(directx::DxTimer::default()),
        "RDXSOUND" => Object::DxSound(directx::DxSound::default()),
        "RDXJOYSTICK" => Object::DxJoystick(joystick::DxJoystick::default()),
        "RRECT" => Object::Record(record::Record::new(record::Kind::Rect)),
        "RNOTIFYICONDATA" => Object::Record(record::Record::new(record::Kind::NotifyIconData)),
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

/// Whether `id` is an RDESIGNSURFACE (its control is drawn again after a
/// change).
pub fn is_design(id: &str) -> bool {
    with(id, |o| matches!(o, Object::Design(_))).unwrap_or(false)
}

/// Reads an RDESIGNSURFACE (to draw it).
pub fn with_design<R>(id: &str, f: impl FnOnce(&design::DesignSurface) -> R) -> Option<R> {
    with(id, |o| match o {
        Object::Design(d) => Some(f(d)),
        _ => None,
    })?
}

/// Changes an RDESIGNSURFACE from its control (the mouse on it).
pub fn with_design_mut<R>(id: &str, f: impl FnOnce(&mut design::DesignSurface) -> R) -> Option<R> {
    with(id, |o| match o {
        Object::Design(d) => Some(f(d)),
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

/// Pixel (x, y) of bitmap / canvas / picture `id`, if it has one there.
pub fn bitmap_pixel(id: &str, x: i64, y: i64) -> Option<u32> {
    with(id, |o| match o {
        Object::Bitmap(b) => b.pixel(x, y),
        _ => None,
    })
    .flatten()
}

/// QCANVAS `id`'s backdrop — its parent's colour, &HBBGGRR, where nothing
/// is drawn ([`bitmap::Bitmap::set_backdrop`]). Whether it changed.
pub fn set_backdrop(id: &str, bgr: u32) -> bool {
    with(id, |o| match o {
        Object::Bitmap(b) if b.canvas && !b.form => b.set_backdrop(bgr),
        _ => false,
    })
    .unwrap_or(false)
}

/// QDIGDISPLAY `id`'s Display (`None`: it isn't one).
pub fn digdisplay_text(id: &str) -> Option<String> {
    DIGITS.with(|d| d.borrow().get(&id.to_lowercase()).cloned())
}

/// Draws QDIGDISPLAY `id`'s Display onto its surface again (it was
/// painted, resized or cleared); its size for the runtime to give the
/// control (`None`: it isn't one).
pub fn digdisplay_redraw(id: &str) -> Option<(i64, i64)> {
    let text = digdisplay_text(id)?;
    let (w, h) = digdisplay::size(&text);
    with(id, |o| {
        if let Object::Bitmap(b) = o {
            b.fit(w, h);
            digdisplay::draw(b, &text);
        }
    });
    Some((w, h))
}

/// Whether `id` is a QDXSCREEN (its runtime widget shows the front buffer
/// again after a Flip).
pub fn is_dxscreen(id: &str) -> bool {
    with(id, |o| matches!(o, Object::DxScreen(_))).unwrap_or(false)
}

/// Reads or changes a QDXSCREEN (to show what its last Flip showed).
pub fn with_dxscreen<R>(id: &str, f: impl FnOnce(&mut directx::DxScreen) -> R) -> Option<R> {
    with(id, |o| match o {
        Object::DxScreen(s) => Some(f(s)),
        _ => None,
    })?
}

/// QDXSCREEN `id`'s form was shown: it's set up, `true` the first time
/// (the runtime fires OnInitialize and OnInitializeSurface). `props`: the
/// control's Width, Height, AutoSize and FullScreen.
pub fn dxscreen_initialize(id: &str, props: PropReader) -> bool {
    let c = dxscreen_control(id, props);
    with_dxscreen(id, |s| directx::initialize(s, c.width, c.height, c.follows())).unwrap_or(false)
}

/// What a QDXSCREEN's control says about its picture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DxControl {
    pub width: i64,
    pub height: i64,
    /// AutoSize (True unless set).
    pub autosize: bool,
    /// AllowStretch (False unless set: RC.EXE reads 0).
    pub stretch: bool,
    pub fullscreen: bool,
}

impl DxControl {
    /// The surface follows the control's size (AutoSize; not in
    /// FullScreen, whose surface is the display's mode).
    pub fn follows(&self) -> bool {
        self.autosize && !self.fullscreen
    }
}

/// A QDXSCREEN control's Width, Height, AutoSize, AllowStretch and
/// FullScreen.
pub fn dxscreen_control(id: &str, props: PropReader) -> DxControl {
    let flag = |p: &str, default: bool| match props(id, p) {
        Value::Null => default,
        v => v.to_bool(),
    };
    DxControl {
        width: props(id, "width").to_i64(),
        height: props(id, "height").to_i64(),
        autosize: flag("autosize", true),
        stretch: flag("allowstretch", false),
        fullscreen: flag("fullscreen", false),
    }
}

/// QDXTIMER `id` fires at `now_ms` (the runtime's clock): it counts the
/// frame for FrameRate.
pub fn dxtimer_fired(id: &str, now_ms: f64) {
    with(id, |o| if let Object::DxTimer(t) = o { t.tick(now_ms) });
}

/// Whether `id` is a QRECT or a QNOTIFYICONDATA: its fields are stored
/// only here, as RapidQ stores them (no geometry rounding, no layout).
pub fn is_record(id: &str) -> bool {
    with(id, |o| matches!(o, Object::Record(_))) == Some(true)
}

/// Reads a QRECT or a QNOTIFYICONDATA (record.rs).
pub fn with_record<R>(id: &str, f: impl FnOnce(&record::Record) -> R) -> Option<R> {
    with(id, |o| match o {
        Object::Record(r) => Some(f(r)),
        _ => None,
    })?
}

/// Whether `id` is a QDXJOYSTICK.
/// Bytes written into a stream where it stands (QCOMPORT's Read).
pub(crate) fn stream_append(id: &str, bytes: &[u8]) {
    with(id, |o| {
        if let Object::Stream(m) = o {
            m.write(bytes);
        }
    });
}

pub fn is_dxjoystick(id: &str) -> bool {
    with(id, |o| matches!(o, Object::DxJoystick(_))) == Some(true)
}

/// QDXJOYSTICK `id`'s look for its events (joystick.rs): the events to fire,
/// with their arguments.
pub fn dxjoystick_look(id: &str) -> Vec<(&'static str, Vec<Value>)> {
    with(id, |o| match o {
        Object::DxJoystick(j) => j.look(),
        _ => Vec::new(),
    })
    .unwrap_or_default()
}

/// RapidQ's font for every component, a canvas's and a new QFONT's (RC.EXE:
/// `Font.Name` reads MS Sans Serif, `Font.Size` 8 — Delphi's default; the
/// face RapidR draws it with is RapidR Sans, `objects::text`).
pub const DEFAULT_FONT_NAME: &str = "MS Sans Serif";
pub const DEFAULT_FONT_SIZE: i64 = 8;

/// A component's font property `flat` (`fontname`, `fontsize`, `fontbold`
/// …) as RapidQ has it: its own when the program set one, else its
/// parent's, followed live (Delphi's ParentFont), else RapidQ's default
/// (MS Sans Serif, 8, no styles) — what the program reads and what's drawn.
pub fn inherited_font_prop(id: &str, flat: &str, props: &dyn Fn(&str, &str) -> Value) -> Value {
    let mut at = id.to_string();
    for _ in 0..32 {
        let v = props(&at, flat);
        if !matches!(v, Value::Null) && !(flat == "fontname" && v.to_string_val().trim().is_empty()) {
            return v;
        }
        let parent = props(&at, "parent").to_string_val();
        if parent.is_empty() || parent.eq_ignore_ascii_case(&at) {
            break;
        }
        at = parent;
    }
    match flat {
        "fontname" => v_str(DEFAULT_FONT_NAME),
        "fontsize" => crate::v_int(DEFAULT_FONT_SIZE),
        _ => crate::v_int(0),
    }
}

/// The font properties a component takes as its own before the program
/// first changes one of them (`prop`, any spelling): Delphi's ParentFont
/// ends there, so the font it had from its parents stays its own and a
/// parent's later change doesn't reach it (RC.EXE: a label made bold, then
/// `Form.Font.AddStyles(fsItalic)` — the form italic, the label not).
/// Empty when `prop` isn't a font name, size or style, or the component
/// already has its own font (`__ownfont`, which the list then sets).
pub fn own_font_from_parents(id: &str, prop: &str, props: &dyn Fn(&str, &str) -> Value) -> Vec<(&'static str, Value)> {
    if font_flat_name(prop).is_none() || props(id, "__ownfont").to_bool() {
        return Vec::new();
    }
    let mut out: Vec<(&'static str, Value)> = ["fontname", "fontsize", "fontbold", "fontitalic", "fontunderline", "fontstrikeout"]
        .into_iter()
        .filter(|f| matches!(props(id, f), Value::Null))
        .map(|f| (f, inherited_font_prop(id, f, props)))
        .collect();
    out.push(("__ownfont", crate::v_bool(true)));
    out
}

/// The flat property a component's font property is kept as (`font.name`,
/// `fontname` → `fontname`; size, bold, italic, underline, strikeout), for
/// [`inherited_font_prop`]; None for the colour (read its own way) and the
/// rest.
pub fn font_flat_name(prop: &str) -> Option<&'static str> {
    let p = prop.to_ascii_lowercase();
    let p = p.strip_prefix("font.").map(|s| format!("font{s}")).unwrap_or(p);
    ["fontname", "fontsize", "fontbold", "fontitalic", "fontunderline", "fontstrikeout"].into_iter().find(|f| *f == p)
}

/// The font a component's properties describe (`Font = Font`, `Font.Size = …`
/// keep them as `fontname`, `fontsize` (points), `fontcolor`, `fontbold`, …):
/// what text drawn for the component (a form's surface, a list's items) uses.
pub fn font_from_props(id: &str, props: &dyn Fn(&str, &str) -> Value) -> Font {
    let name = inherited_font_prop(id, "fontname", props).to_string_val();
    let flag = |p: &str| inherited_font_prop(id, p, props).to_bool();
    let size = inherited_font_prop(id, "fontsize", props).to_i64();
    Font {
        name: if name.trim().is_empty() { DEFAULT_FONT_NAME.into() } else { name },
        size: if size > 0 { size } else { DEFAULT_FONT_SIZE },
        color: color_bgr(props(id, "fontcolor").to_i64()) as i64,
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
        Value::Integer(_) | Value::Double(_) => color_bgr(v.to_i64()) as i64,
        // (never set: the button face, the theme's — a form's clBtnFace)
        Value::Null => color_bgr(crate::component_defaults::CL_BTN_FACE) as i64,
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

/// A program's colour number as &HBBGGRR: Windows' system colours
/// (`&H80000000 + index`: clBtnFace, clWindow, … — what RapidQ's Color
/// reads until the program sets one) in the current theme's colours, as
/// Delphi's ColorToRGB turns them; any other number its low 24 bits.
pub fn color_bgr(n: i64) -> u32 {
    let n32 = n as u32;
    if n32 & 0xFF00_0000 == 0x8000_0000 && n32 & 0x00FF_FF00 == 0 {
        let rgb = crate::theme::current().system_color(n32 & 0xFF);
        return ((rgb & 0xFF) << 16) | (rgb & 0xFF00) | (rgb >> 16);
    }
    n32 & 0xFFFFFF
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
            | "textout" | "textwidth" | "textheight" | "pixel" | "cls" | "clear" | "drawtext" | "fillcircle" | "ellipse" | "setpixel" | "rect"
            | "textrect"
    )
}

/// A TextRect's colour argument: -1 (or clNone) is none — transparent.
fn text_color_arg(v: Option<&Value>) -> Option<u32> {
    v.map(Value::to_i64).filter(|v| *v >= 0 || (*v as u32) & 0xFF00_0000 == 0x8000_0000).map(color_bgr)
}

/// `TextRect(Rect, x, y, S$, fc, bc)` on a bitmap / canvas / QIMAGE /
/// form surface / QDXSCREEN's back buffer, in its font (text.rs).
fn bitmap_text_rect(b: &mut Bitmap, rect: (i64, i64, i64, i64), args: &[Value]) {
    let n = |i: usize| args.get(i).map_or(0, Value::to_i64);
    let text = args.get(3).map(Value::to_string_val).unwrap_or_default();
    let color = if args.len() > 4 { color_bgr(n(4)) } else { color_bgr(b.font.color) };
    let font = b.font.clone();
    text::text_rect(b, rect, n(1), n(2), &text, &font, color, text_color_arg(args.get(5)));
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

/// Forgets object `id` (a kernel-drawn dialog's components once it
/// closes; a test's store starting again); whether there was one.
pub fn remove(id: &str) -> bool {
    OBJECTS.with(|o| o.borrow_mut().remove(&id.to_lowercase()).is_some())
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
    if d3d::exists(id) {
        return d3d::get(id, &prop);
    }
    if rqlib::exists(id) {
        return rqlib::get(id, &prop);
    }
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
    if prop == "display" {
        if let Some(text) = digdisplay_text(id) {
            return Some(v_str(&text));
        }
    }
    with(id, |o| match o {
        Object::Font(f) => f.get(&prop),
        // Functions called without parentheses: `S$ = Mem.ReadLine`.
        Object::Stream(m) if matches!(prop.as_str(), "readline" | "readln" | "readall" | "readbyte") => m.call(&prop, &[]),
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
        Object::Registry(r) => r.get(&prop),
        Object::Design(d) => d.get(&prop),
        Object::DxScreen(s) => s.get(&prop),
        Object::DxImageList(l) => l.get(&prop),
        Object::DxTimer(t) => t.get(&prop),
        Object::DxSound(s) => s.get(&prop),
        Object::DxJoystick(j) => j.get(&prop),
        Object::Record(r) => r.get(&prop),
    })?
}

/// Sets a property; `Some(Ok)` if handled, `Some(Err)` if it failed (a BMP
/// that can't be loaded), `None` if not an object property.
pub fn set(id: &str, prop: &str, val: &Value) -> Option<Result<(), String>> {
    ensure_printer(id);
    let prop = prop.to_lowercase();
    if d3d::exists(id) {
        return d3d::set(id, &prop, val).map(Ok);
    }
    if rqlib::exists(id) {
        return rqlib::set(id, &prop, val);
    }
    if menu::is_menu(id) {
        return menu::set(id, &prop, val).map(Ok);
    }
    if with_header(id, |h| h.set(&prop, val)) == Some(true) {
        return Some(Ok(()));
    }
    // A QDIGDISPLAY's Display: drawn at once (the runtime sizes the control).
    if prop == "display" && digdisplay_text(id).is_some() {
        DIGITS.with(|d| d.borrow_mut().insert(id.to_lowercase(), val.to_string_val()));
        digdisplay_redraw(id);
        return Some(Ok(()));
    }
    // `Printer.Font = Font` / `Bitmap.Font = Font`: the QFONT's settings.
    // A QDXSOUND's FileName: the WAV file read.
    if prop == "filename" && with(id, |o| matches!(o, Object::DxSound(_))) == Some(true) {
        let name = val.to_string_val();
        let bytes = read_file(&name);
        return with(id, |o| match o {
            Object::DxSound(s) => Some(bytes.and_then(|b| s.load(id, &name, &b))),
            _ => None,
        })
        .flatten();
    }
    if prop == "font" && matches!(with(id, |o| matches!(o, Object::Printer(_) | Object::DxScreen(_)) || matches!(o, Object::Bitmap(b) if !b.form)), Some(true)) {
        let font = with(&val.to_string_val(), |o| match o {
            Object::Font(f) => Some(f.clone()),
            _ => None,
        })
        .flatten()?;
        with(id, |o| match o {
            Object::Printer(p) => p.font = font,
            Object::Bitmap(b) => b.font = font,
            Object::DxScreen(s) => s.back.font = font,
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
                    b.touch();
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
        Object::Registry(r) => r.set(&prop, val).then_some(Ok(())),
        Object::Design(d) => d.set(&prop, val).then_some(Ok(())),
        Object::DxScreen(s) => s.set(&prop, val),
        Object::DxSound(s) => s.set(id, &prop, val),
        Object::DxJoystick(j) => j.set(&prop, val),
        Object::Record(r) => r.set(&prop, val).then_some(Ok(())),
        Object::DxImageList(_) | Object::DxTimer(_) => None,
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
    if d3d::exists(id) {
        return d3d::call(id, &method, args);
    }
    if rqlib::exists(id) {
        return rqlib::call(id, &method, args);
    }
    if let Some(v) = menu::call(id, &method, args) {
        return Some(Ok(v));
    }
    // (RapidR's own icons: Bitmap.LoadIcon, ImageList.AddIcon)
    if let Some(r) = icons::call(id, &method, args) {
        return Some(r);
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
                with(id, |o| {
                    if let Object::Text(t) = o {
                        // (RapidR's code editor reads a UTF-8 source as
                        // UTF-8 — its BOM off — as the compiler does; any
                        // other file, and every RapidQ text box, a byte a
                        // character)
                        let body = bytes.strip_prefix(b"\xEF\xBB\xBF".as_slice()).unwrap_or(&bytes);
                        let utf8 = if t.code { std::str::from_utf8(body).ok() } else { None };
                        t.utf8 = utf8.is_some_and(|s| !s.is_ascii());
                        let text: String = match utf8 {
                            Some(s) => s.to_string(),
                            None => bytes.iter().map(|&b| char::from(b)).collect(),
                        };
                        t.set_text(&text);
                    }
                });
                Value::Null
            })
        } else {
            let (text, utf8) = with(id, |o| match o {
                Object::Text(t) => (t.text(), t.utf8),
                _ => (String::new(), false),
            })
            .unwrap_or_default();
            let bytes = if utf8 { text.into_bytes() } else { text.chars().map(|c| c as u32 as u8).collect::<Vec<u8>>() };
            write_file(&path, &bytes).map(|_| Value::Null)
        });
    }
    // QRICHEDIT LoadFromStream / SaveToStream: the same text as the file
    // methods, from the stream's position to its end / written at it.
    if is_textedit(id) && matches!(method.as_str(), "loadfromstream" | "savetostream") {
        let stream = arg(0).to_string_val();
        if method == "loadfromstream" {
            let bytes = with(&stream, |o| match o {
                Object::Stream(m) => Some(m.read(usize::MAX)),
                _ => None,
            })
            .flatten();
            let Some(bytes) = bytes else { return Some(Err(format!("{stream} is not a QFILESTREAM or QMEMORYSTREAM"))) };
            let text: String = bytes.iter().map(|&b| char::from(b)).collect();
            with(id, |o| {
                if let Object::Text(t) = o {
                    t.set_text(&text);
                }
            });
        } else {
            let text = with(id, |o| match o {
                Object::Text(t) => t.text(),
                _ => String::new(),
            })
            .unwrap_or_default();
            let bytes: Vec<u8> = text.chars().map(|c| c as u32 as u8).collect();
            if with(&stream, |o| if let Object::Stream(m) = o { m.write(&bytes) }).is_none() {
                return Some(Err(format!("{stream} is not a QFILESTREAM or QMEMORYSTREAM")));
            }
        }
        return Some(Ok(Value::Null));
    }
    // QSTRINGLIST LoadFromStream(S): the list becomes the stream's text from
    // its position to its end (lines end at CR LF, LF or CR), the stream at
    // its end. SaveToStream(S) does the same in RapidQ (RC.EXE: the list is
    // read from the stream, the stream isn't written), so it does here too.
    if matches!(method.as_str(), "loadfromstream" | "savetostream") && with(id, |o| matches!(o, Object::List(_)))? {
        let stream = arg(0).to_string_val();
        let bytes = with(&stream, |o| match o {
            Object::Stream(m) => Some(m.read(usize::MAX)),
            _ => None,
        })
        .flatten();
        let Some(bytes) = bytes else { return Some(Err(format!("{stream} is not a QFILESTREAM or QMEMORYSTREAM"))) };
        let text: String = bytes.iter().map(|&b| char::from(b)).collect();
        with(id, |o| {
            if let Object::List(l) = o {
                l.load_stream_text(&text);
            }
        });
        return Some(Ok(Value::Null));
    }
    // QSTRINGLIST AddList(Other): the other list's strings appended
    // (anything but a list adds nothing).
    if method == "addlist" {
        let Some(other) = with(&arg(0).to_string_val(), |o| match o {
            Object::List(l) => Some(l.items.clone()),
            _ => None,
        })
        .flatten() else {
            return with(id, |o| matches!(o, Object::List(_))).filter(|&l| l).map(|_| Ok(Value::Null));
        };
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
        Object::Registry(_) => "registry",
        Object::Design(_) => "design",
        Object::DxScreen(_) => "dxscreen",
        Object::DxImageList(_) => "dximagelist",
        Object::DxTimer(_) => "dxtimer",
        Object::DxSound(_) => "dxsound",
        Object::DxJoystick(_) => "dxjoystick",
        Object::Record(_) => "record",
    })?;
    // A file opened for reading can't be written.
    if kind == "stream" && memstream::WRITE_METHODS.contains(&method.as_str()) {
        let read_only = with(id, |o| matches!(o, Object::Stream(m) if m.file.as_ref().is_some_and(|f| !f.writable)))?;
        if read_only {
            return Some(Err(format!("{method}: the file was opened for reading (fmOpenRead)")));
        }
    }
    // A QDXSCREEN's surface follows its control's size (AutoSize).
    if kind == "dxscreen" {
        let c = dxscreen_control(id, props);
        with_dxscreen(id, |s| s.follow_control(c.width, c.height, c.follows()));
        // Its 3D methods: the scene (d3d.rs), drawn on its back buffer.
        if d3d::is_screen_method(&method) {
            return with_dxscreen(id, |s| d3d::screen_call(id, s, &method, args)).flatten();
        }
    }
    // Drawing on a QIMAGE without a picture: first one the control's size
    // (read before borrowing the registry: `props` may read objects too).
    let drawing = matches!(method.as_str(), "pset" | "line" | "rectangle" | "fillrect" | "circle" | "roundrect" | "paint" | "draw" | "copyrect" | "stretchdraw" | "textrect")
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
        // CopyFrom(Stream, Bytes): Bytes from the other stream's position
        // (0: all of it, from its start). RapidQ stops the program when the
        // other stream has fewer left (EReadError "Stream read error").
        ("stream", "copyfrom") => {
            let src = arg(0).to_string_val();
            let n = arg(1).to_i64();
            let bytes = with(&src, |o| match o {
                Object::Stream(s) => {
                    if n <= 0 {
                        s.pos = 0;
                    }
                    let want = if n <= 0 { s.data.len() } else { n as usize };
                    Some(s.read(want)).filter(|b| b.len() == want)
                }
                _ => None,
            });
            let bytes = match bytes {
                Some(Some(b)) => b,
                Some(None) => return Some(Err(format!("stream read error ({src} has fewer than {n} bytes left)"))),
                None => return Some(Err(format!("{src} is not a QFILESTREAM or QMEMORYSTREAM"))),
            };
            with(id, |o| if let Object::Stream(m) = o { m.write(&bytes) });
            Some(Ok(Value::Null))
        }
        // MemCopyFrom(Address, Bytes): the memory's bytes written at the
        // position; MemCopyTo(Address, Bytes): the bytes at the position
        // written to memory (rapidr_value::memory: an address of the
        // program's own, never raw memory). Both move the position on.
        ("stream", "memcopyfrom") => {
            let n = arg(1).to_i64();
            if n <= 0 {
                return Some(Ok(Value::Null));
            }
            let bytes = match crate::memory::read(arg(0).to_i64(), n as usize) {
                Ok(b) => b,
                Err(e) => return Some(Err(e)),
            };
            with(id, |o| if let Object::Stream(m) = o { m.write(&bytes) });
            Some(Ok(Value::Null))
        }
        ("stream", "memcopyto") => {
            let n = arg(1).to_i64();
            if n <= 0 {
                return Some(Ok(Value::Null));
            }
            let bytes = with(id, |o| match o {
                Object::Stream(m) => m.read(n as usize),
                _ => Vec::new(),
            })?;
            Some(crate::memory::write(arg(0).to_i64(), &bytes).map(|_| Value::Null))
        }
        // (`Image.Load file`: RapidR's other name)
        ("bitmap", "loadfromfile" | "load") => Some(read_file(&arg(0).to_string_val()).and_then(|bytes| {
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
        // OnDrawCell's / OnDrawItem's TextRect(Rect, x, y, S$, fc, bc),
        // CopyRect(D, Image, S) and StretchDraw(Rect, BMP): kept on the
        // cell / item under the rectangle's top left, as their other
        // drawing is (a list box that isn't owner-drawn: nothing kept).
        ("grid" | "list", "textrect" | "copyrect" | "stretchdraw") => {
            let r = rect_of(&arg(0), props);
            let op: Box<dyn FnOnce(i64, i64) -> grid::CellDraw> = if method == "textrect" {
                let n = |i: usize| args.get(i).map_or(0, Value::to_i64);
                let (x, y, fc, bc) = (n(1), n(2), color_bgr(n(4)), text_color_arg(args.get(5)));
                let text = arg(3).to_string_val();
                Box::new(move |l, t| grid::CellDraw::TextRect((r.0 - l, r.1 - t, r.2 - l, r.3 - t), x - l, y - t, text, fc, bc))
            } else {
                let src = match load_image(&arg(1)) {
                    Ok(src) => src,
                    Err(e) => return Some(Err(e)),
                };
                // (the picture, or its part S, scaled to the rectangle now:
                // drawn as a picture of that size)
                let part = if method == "copyrect" { rect_of(&arg(2), props) } else { (0, 0, src.img.width as i64, src.img.height as i64) };
                let (l, t, w, h) = (r.0.min(r.2), r.1.min(r.3), (r.2 - r.0).abs(), (r.3 - r.1).abs());
                let mut scaled = Bitmap::default();
                scaled.resize(w, h);
                scaled.copy_rect((0, 0, w, h), &src, part);
                Box::new(move |cl, ct| grid::CellDraw::Image(l - cl, t - ct, scaled))
            };
            let (ax, ay) = (r.0.min(r.2), r.1.min(r.3));
            with(id, |o| match o {
                Object::Grid(g) => g.record(ax, ay, op),
                Object::List(l) if l.owner_drawn() => l.record(ax, ay, op),
                _ => {}
            });
            Some(Ok(Value::Null))
        }
        // TextWidth / TextHeight on a grid or a list box: in the control's
        // font, what its cells' / items' text is drawn in.
        ("grid" | "list", "textwidth" | "textheight") => {
            let font = font_from_props(id, props);
            let (w, h) = text::text_size(&arg(0).to_string_val(), &font);
            Some(Ok(v_int(if method == "textwidth" { w } else { h })))
        }
        ("bitmap", "textrect") => {
            let r = rect_of(&arg(0), props);
            with(id, |o| if let Object::Bitmap(b) = o { bitmap_text_rect(b, r, args) });
            Some(Ok(Value::Null))
        }
        // Printer.TextRect(Rect, x, y, S$, fc, bc): the Rect's four numbers first.
        ("printer", "textrect") => {
            let (l, t, r, b) = rect_of(&arg(0), props);
            let mut flat = vec![v_int(l), v_int(t), v_int(r), v_int(b)];
            flat.extend(args.iter().skip(1).cloned());
            with(id, |o| match o {
                Object::Printer(p) => p.call("textrect", &flat),
                _ => None,
            });
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
            // Draw(Target, X, Y, Index): image Index onto a QBITMAP, QIMAGE,
            // QCANVAS (bitmaps all) or a QDXSCREEN's back buffer (RC.EXE
            // takes those four, not a form); an Index out of range draws
            // nothing. The runtime shows the target again.
            let i = arg(3).to_i64();
            let image = with(id, |o| match o {
                Object::ImageList(l) if i >= 0 => l.images.get(i as usize).cloned(),
                _ => None,
            })?;
            let target = arg(0).to_string_val();
            // (a QIMAGE without a picture gets one its size first, as for
            // its own drawing methods; a QCANVAS is always its size)
            let sized = with(&target, |o| matches!(o, Object::Bitmap(b) if (b.picture && b.img.pixels.is_empty()) || (b.canvas && !b.form)));
            if sized == Some(true) {
                let (w, h) = (props(&target, "width").to_i64(), props(&target, "height").to_i64());
                with(&target, |o| match o {
                    Object::Bitmap(b) if b.canvas => b.fit(w, h),
                    Object::Bitmap(b) => b.resize(w, h),
                    _ => {}
                });
            }
            let (x, y) = (arg(1).to_i64(), arg(2).to_i64());
            with(&target, |o| match (o, image) {
                (Object::Bitmap(b), Some(image)) => b.draw(x, y, &image),
                (Object::DxScreen(s), Some(image)) => s.back.draw(x, y, &image),
                _ => {}
            });
            Some(Ok(Value::Null))
        }
        // QDXSCREEN: a picture onto the back buffer — Draw(x, y, BMP),
        // StretchDraw(Rect, BMP), CopyRect(D, Image, S) — and TextRect.
        ("dxscreen", "draw") => {
            let src = match load_image(&arg(2)) {
                Ok(src) => src,
                Err(e) => return Some(Err(e)),
            };
            with(id, |o| if let Object::DxScreen(s) = o { s.back.draw(arg(0).to_i64(), arg(1).to_i64(), &src) });
            Some(Ok(Value::Null))
        }
        ("dxscreen", "copyrect" | "stretchdraw") => {
            let rect = |v: &Value| rect_of(v, props);
            let (dest, src_value, src_rect) =
                if method == "copyrect" { (rect(&arg(0)), arg(1), Some(rect(&arg(2)))) } else { (rect(&arg(0)), arg(1), None) };
            let src = match load_image(&src_value) {
                Ok(src) => src,
                Err(e) => return Some(Err(e)),
            };
            let src_rect = src_rect.unwrap_or((0, 0, src.img.width as i64, src.img.height as i64));
            with(id, |o| if let Object::DxScreen(s) = o { s.back.copy_rect(dest, &src, src_rect) });
            Some(Ok(Value::Null))
        }
        ("dxscreen", "textrect") => {
            let r = rect_of(&arg(0), props);
            with(id, |o| if let Object::DxScreen(s) = o { bitmap_text_rect(&mut s.back, r, args) });
            Some(Ok(Value::Null))
        }
        // QDXIMAGELIST: an image library from a file, a `$RESOURCE` or a
        // stream.
        ("dximagelist", "loadfromfile" | "loadfromresource" | "loadfromstream") => {
            let bytes = match method.as_str() {
                "loadfromfile" => read_file(&arg(0).to_string_val()),
                "loadfromresource" => crate::resources::bytes(arg(0).to_i64()).map(|b| b.to_vec()).ok_or_else(|| format!("no resource {}", arg(0).to_i64())),
                _ => Ok(with(&arg(0).to_string_val(), |o| match o {
                    Object::Stream(m) => m.read(usize::MAX),
                    _ => Vec::new(),
                })
                .unwrap_or_default()),
            };
            Some(bytes.and_then(|b| with(id, |o| if let Object::DxImageList(l) = o { l.load(&b) } else { Ok(()) }).unwrap_or(Ok(()))).map(|_| Value::Null))
        }
        // Draw(Item, X, Y, Pattern) onto its Parent screen's back buffer.
        ("dximagelist", "draw") => {
            let (item, pattern) = (arg(0).to_i64(), arg(3).to_i64());
            let picture = with(id, |o| match o {
                Object::DxImageList(l) if item >= 0 => l.items.get_mut(item as usize).and_then(|p| directx::pattern_bitmap(p, pattern)),
                _ => None,
            })
            .flatten();
            let screen = props(id, "parent").to_string_val();
            if let Some(picture) = picture {
                with(&screen, |o| if let Object::DxScreen(s) = o { s.back.draw(arg(1).to_i64(), arg(2).to_i64(), &picture) });
            }
            Some(Ok(Value::Null))
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
        Object::Registry(r) => r.call(method, args),
        Object::Design(d) => d.call(method, args),
        Object::DxScreen(s) => s.call(method, args),
        Object::DxSound(s) => s.call(id, method),
        Object::DxJoystick(j) => j.call(method, args),
        Object::DxImageList(_) | Object::DxTimer(_) | Object::Record(_) => None,
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

    /// TextRect, CopyRect, StretchDraw, RoundRect, TextWidth on lists and
    /// grids (OnDrawItem / OnDrawCell): kept on the item / cell under the
    /// rectangle's top left; a list that isn't owner-drawn keeps nothing
    /// but answers. TextRect on a bitmap and a QDXSCREEN's back buffer;
    /// ImageList.Draw onto a QDXSCREEN.
    #[test]
    fn owner_drawing_rects() {
        // QRECTs as property bags (OnDrawItem's Rect).
        let props = |id: &str, p: &str| match (id, p) {
            ("r", "left") => v_int(130),
            ("r", "top") => v_int(25),
            ("r", "right") => v_int(194),
            ("r", "bottom") => v_int(49),
            ("half", "right") => v_int(2),
            ("half", "bottom") => v_int(2),
            ("t_dl", "fontname") => v_str("Arial"),
            ("t_dl", "fontsize") => v_int(12),
            _ => v_null(),
        };
        assert!(create("t_dg", "RSTRINGGRID") && create("t_dl", "RLISTBOX") && create("t_dsrc", "RBITMAP"));
        set("t_dsrc", "width", &v_int(4)).unwrap().unwrap();
        set("t_dsrc", "height", &v_int(4)).unwrap().unwrap();
        call("t_dg", "textrect", &[v_str("r"), v_int(132), v_int(27), v_str("x"), v_int(0), v_int(0xFF)], &props).unwrap().unwrap();
        call("t_dg", "roundrect", &[v_int(131), v_int(26), v_int(150), v_int(40), v_int(6), v_int(6), v_int(0xFF00)], &props).unwrap().unwrap();
        call("t_dg", "stretchdraw", &[v_str("r"), v_str("t_dsrc")], &props).unwrap().unwrap();
        call("t_dg", "copyrect", &[v_str("r"), v_str("t_dsrc"), v_str("half")], &props).unwrap().unwrap();
        let kept = with("t_dg", |o| match o {
            Object::Grid(g) => g.owner_drawing.get(&(2, 1)).cloned(),
            _ => None,
        })
        .flatten()
        .unwrap();
        assert!(matches!(&kept[0], grid::CellDraw::TextRect((0, 0, 64, 24), 2, 2, t, 0, Some(0xFF)) if t == "x"), "{:?}", kept[0]);
        assert!(matches!(kept[1], grid::CellDraw::RoundRect(1, 1, 20, 15, 6, 6, 0xFF00)));
        assert!(matches!(&kept[2], grid::CellDraw::Image(0, 0, b) if (b.img.width, b.img.height) == (64, 24)));
        assert!(matches!(&kept[3], grid::CellDraw::Image(0, 0, b) if (b.img.width, b.img.height) == (64, 24)));
        // The list isn't owner-drawn: answered, nothing kept.
        call("t_dl", "additems", &[v_str("a")], &props).unwrap().unwrap();
        for (m, a) in [("pset", vec![v_int(1), v_int(1), v_int(0)]), ("rectangle", vec![v_int(0), v_int(0), v_int(5), v_int(5), v_int(0)]), ("textrect", vec![v_str("r"), v_int(0), v_int(0), v_str("x"), v_int(0), v_int(-1)])] {
            assert!(call("t_dl", m, &a, &props).is_some(), "{m}");
        }
        assert!(with("t_dl", |o| matches!(o, Object::List(l) if l.owner_drawing.is_empty())).unwrap());
        // TextWidth / TextHeight in the control's font.
        let w = call("t_dl", "textwidth", &[v_str("Hello")], &props).unwrap().unwrap().to_i64();
        let h = call("t_dl", "textheight", &[v_str("Hello")], &props).unwrap().unwrap().to_i64();
        assert_eq!((w, h), (36, 18));
        // A QDXSCREEN's TextRect: the shared one, on its back buffer.
        assert!(create("t_ddx", "RDXSCREEN"));
        call("t_ddx", "init", &[v_int(60), v_int(20)], &props).unwrap().unwrap();
        call("t_ddx", "textrect", &[v_str("half"), v_int(0), v_int(0), v_str("WWWW"), v_int(0xFFFFFF), v_int(0xFF)], &props).unwrap().unwrap();
        let px = |x, y| with("t_ddx", |o| match o {
            Object::DxScreen(s) => s.back.pixel(x, y),
            _ => None,
        })
        .flatten();
        assert_eq!((px(1, 1), px(3, 3)), (Some(0xFF), px(40, 10)), "inside the rectangle: its background; outside: as it was");
        // ImageList.Draw onto its back buffer; an Index out of range draws nothing.
        assert!(create("t_dil", "RIMAGELIST"));
        let mut red = Bitmap::default();
        red.resize(2, 2);
        red.fill_rect(0, 0, 2, 2, 0xFF);
        with("t_dil", |o| {
            if let Object::ImageList(l) = o {
                l.insert(0, &red, None);
            }
        });
        call("t_dil", "draw", &[v_str("t_ddx"), v_int(50), v_int(10), v_int(0)], &props).unwrap().unwrap();
        call("t_dil", "draw", &[v_str("t_ddx"), v_int(20), v_int(5), v_int(7)], &props).unwrap().unwrap();
        assert_eq!((px(51, 11), px(21, 6)), (Some(0xFF), px(40, 10)));
        for id in ["t_dg", "t_dl", "t_dsrc", "t_ddx", "t_dil"] {
            remove(id);
        }
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

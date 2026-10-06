//! QOPENDIALOG / QSAVEDIALOG / QFILEDIALOG on the web: the user's real
//! files through the browser's own pickers, as the desktop's dialogs are
//! the system's (rfd). docs/web-host-plan.md §3.8 and docs/manual/web.md
//! say what each browser does.
//!
//! - **Open**: the File System Access API's `showOpenFilePicker` where the
//!   page may show it (Chromium, a top-level page), else the IDE's
//!   (`RAPIDR_FILE_HOST`: its preview frame has an opaque origin, which may
//!   not show one), else `<input type=file>` (Firefox, Safari). The files
//!   picked are read whole before Execute returns, into the page's file
//!   store under their names: the program reads them with its ordinary file
//!   I/O (LoadFromFile, QFILESTREAM, OPEN … FOR INPUT).
//! - **Save**: `showSaveFilePicker` (the program's name proposed, its
//!   Filter as the file types), likewise directly or through the IDE. Where
//!   the browser has none, the host asks for the name in a kernel-drawn box
//!   (`kernel_web`), and what the program writes to that name later goes out
//!   as a download.
//! - **Writes**: a name a dialog answered is the real file's from then on —
//!   what the program writes to it ([`written`], from the page's file
//!   store) goes to that file (its handle, the IDE's handle by token, or a
//!   download), the last write winning; any other name stays in the
//!   browser's private store, as before.
//!
//! A picker opens only within a user's gesture (a click, a key: the
//! browser's transient activation). Execute called with none left — from
//! a timer, long after the click — gets [`Picked::NeedsGesture`], and the
//! host shows a small kernel-drawn box whose button is that gesture.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use js_sys::{Array, Function, Object, Promise, Reflect, Uint8Array};
use rapidr_ui_app::file_dialog::Request;
use rapidr_value::file_dialog as fd;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};

/// What a picker answered.
pub enum Picked {
    /// The names of the files picked (their data in the page's store).
    Files(Vec<String>),
    /// The user cancelled.
    Cancelled,
    /// No user gesture is left to open a picker with.
    NeedsGesture,
    /// This browser (or frame) has no save picker: the host asks for the
    /// name itself.
    NoSavePicker,
}

/// A picker's answer, given once (the file input's change or cancel).
type Once = Rc<RefCell<Option<Box<dyn FnOnce(Picked)>>>>;

/// Where a real file's writes go.
#[derive(Clone)]
enum Target {
    /// Its FileSystemFileHandle.
    Handle(JsValue),
    /// The IDE's handle for it, by token (`RAPIDR_FILE_HOST.write`).
    Token(JsValue),
    /// A download of that name.
    Download,
}

thread_local! {
    /// The real files the dialogs answered, by file key: (their name, where
    /// their writes go).
    static TARGETS: RefCell<HashMap<String, (String, Target)>> = RefCell::new(HashMap::new());
    /// Writes waiting to go out, by file key (the last one wins).
    static OUT: RefCell<HashMap<String, Vec<u8>>> = RefCell::new(HashMap::new());
    /// The file keys being written now.
    static BUSY: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

/// How long a download waits for more writes to the same name (a program
/// opening a file FOR OUTPUT writes it empty first, and its data at CLOSE,
/// perhaps a time slice later): one download, of the last.
const DOWNLOAD_QUIET_MS: i32 = 400;

fn window() -> Option<web_sys::Window> {
    web_sys::window()
}

fn get(target: &JsValue, key: &str) -> JsValue {
    Reflect::get(target, &JsValue::from_str(key)).unwrap_or(JsValue::UNDEFINED)
}

fn set(target: &Object, key: &str, value: &JsValue) {
    let _ = Reflect::set(target, &JsValue::from_str(key), value);
}

/// `target.method(args…)`'s promise (a throw: a rejected one).
fn call(target: &JsValue, method: &str, args: &[JsValue]) -> Promise {
    let Ok(f) = get(target, method).dyn_into::<Function>() else {
        return Promise::reject(&JsValue::from_str(&format!("no {method}")));
    };
    let r = match args {
        [] => f.call0(target),
        [a] => f.call1(target, a),
        [a, b] => f.call2(target, a, b),
        _ => f.call3(target, &args[0], &args[1], &args[2]),
    };
    match r {
        Ok(v) => v.dyn_into::<Promise>().unwrap_or_else(|v| Promise::resolve(&v)),
        Err(e) => Promise::reject(&e),
    }
}

/// An error's DOMException name ("AbortError", "SecurityError" …).
fn error_name(e: &JsValue) -> String {
    get(e, "name").as_string().unwrap_or_default()
}

fn warn(text: &str) {
    web_sys::console::warn_1(&JsValue::from_str(&format!("[rapidr] {text}")));
}

/// The IDE's pickers for its preview frame (`RAPIDR_FILE_HOST`), if any.
fn file_host() -> Option<JsValue> {
    let host = get(&window()?.into(), "RAPIDR_FILE_HOST");
    host.is_object().then_some(host)
}

/// Whether this page may show the File System Access picker `name`
/// (`showOpenFilePicker`, `showSaveFilePicker`): the browser has it, and
/// the page isn't a frame of another origin (or an opaque one), which may
/// not show one.
fn direct(name: &str) -> Option<Function> {
    let w = window()?;
    let f = get(&w.clone().into(), name).dyn_into::<Function>().ok()?;
    if w.origin() == "null" {
        return None;
    }
    let top = w.top().ok().flatten()?;
    let same = top == w || Reflect::get(&top, &JsValue::from_str("location")).ok().and_then(|l| Reflect::get(&l, &JsValue::from_str("href")).ok()).is_some_and(|h| h.is_string());
    same.then_some(f)
}

/// Whether the browser still has a user's gesture to open a picker with
/// (`navigator.userActivation.isActive`; a browser that can't tell: yes).
pub fn gesture_active() -> bool {
    let Some(w) = window() else { return true };
    let ua = get(&w.navigator().into(), "userActivation");
    if !ua.is_object() {
        return true;
    }
    get(&ua, "isActive").as_bool().unwrap_or(true)
}

/// Whether a save picker can be shown here (directly or the IDE's).
pub fn has_save_picker() -> bool {
    file_host().is_some_and(|h| get(&h, "save").is_function()) || direct("showSaveFilePicker").is_some()
}

/// The pickers' options for `req` (File System Access).
fn options(req: &Request) -> Object {
    let o = Object::new();
    let (types, accept_all) = fd::picker_types(&req.filters, req.filter_index);
    let list = Array::new();
    for (description, exts) in types {
        let t = Object::new();
        if !description.is_empty() {
            set(&t, "description", &JsValue::from_str(&description));
        }
        let accept = Object::new();
        // (a type no system knows: the patterns' extensions only, none of
        // the system's for a real one added)
        set(&accept, "application/x-rapidr", &exts.iter().map(|e| JsValue::from_str(e)).collect::<Array>());
        set(&t, "accept", &accept);
        list.push(&t);
    }
    if list.length() > 0 {
        set(&o, "types", &list);
    }
    set(&o, "excludeAcceptAllOption", &JsValue::from_bool(!accept_all));
    // (the folder the last dialog was in, as the system's dialogs recall it)
    set(&o, "id", &JsValue::from_str("rapidr-files"));
    if req.save {
        let name = fd::with_default_ext(req.file_name.as_deref().unwrap_or(""), &req.default_ext);
        if !name.is_empty() {
            set(&o, "suggestedName", &JsValue::from_str(&name));
        }
    } else {
        set(&o, "multiple", &JsValue::from_bool(req.multi));
    }
    o
}

/// A real file's name from now on: its writes go to `target`.
fn remember(name: &str, target: Option<Target>) {
    let key = crate::object_web::file_key(name);
    TARGETS.with(|t| {
        let mut t = t.borrow_mut();
        match target {
            Some(target) => {
                t.insert(key, (name.to_string(), target));
            }
            None => {
                t.remove(&key);
            }
        }
    });
}

/// A file picked to open: its data in the page's store (no write-back).
fn keep(name: &str, bytes: Vec<u8>, target: Option<Target>) {
    crate::object_web::web_store_file(name, bytes);
    remember(name, target);
}

/// Open: the user's file(s); `done` gets what was picked. `gestured`:
/// asked from the click on the host's box (`Picked::NeedsGesture`).
pub fn open(req: &Request, gestured: bool, done: impl FnOnce(Picked) + 'static) {
    if !gestured && !gesture_active() {
        done(Picked::NeedsGesture);
        return;
    }
    let opts = options(req);
    let accept = fd::html_accept(&req.filters);
    let multi = req.multi;
    if let Some(host) = file_host().filter(|h| get(h, "open").is_function()) {
        let p = call(&host, "open", &[opts.into()]);
        spawn_local(async move {
            match JsFuture::from(p).await {
                Ok(list) => {
                    let mut names = Vec::new();
                    for f in Array::from(&list).iter() {
                        let Some(name) = get(&f, "name").as_string() else { continue };
                        let bytes = Uint8Array::new(&get(&f, "bytes")).to_vec();
                        keep(&name, bytes, Some(Target::Token(get(&f, "token"))));
                        names.push(name);
                    }
                    done(if names.is_empty() { Picked::Cancelled } else { Picked::Files(names) });
                }
                Err(e) => fallback_open(&e, &accept, multi, gestured, done),
            }
        });
        return;
    }
    if let Some(f) = direct("showOpenFilePicker") {
        let p = match f.call1(&JsValue::UNDEFINED, &opts) {
            Ok(v) => v.dyn_into::<Promise>().unwrap_or_else(|v| Promise::resolve(&v)),
            Err(e) => Promise::reject(&e),
        };
        spawn_local(async move {
            match JsFuture::from(p).await {
                Ok(handles) => {
                    let mut names = Vec::new();
                    for h in Array::from(&handles).iter() {
                        let read = async {
                            let file = JsFuture::from(call(&h, "getFile", &[])).await?;
                            let buffer = JsFuture::from(call(&file, "arrayBuffer", &[])).await?;
                            Ok::<_, JsValue>((get(&file, "name").as_string().unwrap_or_default(), Uint8Array::new(&buffer).to_vec()))
                        };
                        match read.await {
                            Ok((name, bytes)) => {
                                keep(&name, bytes, Some(Target::Handle(h.clone())));
                                names.push(name);
                            }
                            Err(e) => warn(&format!("can't read the file picked: {}", error_name(&e))),
                        }
                    }
                    done(if names.is_empty() { Picked::Cancelled } else { Picked::Files(names) });
                }
                Err(e) => fallback_open(&e, &accept, multi, gestured, done),
            }
        });
        return;
    }
    input_open(&accept, multi, done);
}

/// Whether a picker's failure `e` is the browser's "no user gesture" —
/// a SecurityError or NotAllowedError before the user gave one through
/// the host's box (`gestured`: after it, the picker isn't allowed here at
/// all, and asking again would go round in circles).
fn wants_gesture(e: &JsValue, gestured: bool) -> bool {
    !gestured && (!gesture_active() || matches!(error_name(e).as_str(), "SecurityError" | "NotAllowedError"))
}

/// A File System Access picker failed with `e`: cancelled (AbortError), no
/// gesture left, or a picker this page may not show — the file input then.
fn fallback_open(e: &JsValue, accept: &str, multi: bool, gestured: bool, done: impl FnOnce(Picked) + 'static) {
    match error_name(e).as_str() {
        "AbortError" => done(Picked::Cancelled),
        _ if wants_gesture(e, gestured) => done(Picked::NeedsGesture),
        name => {
            warn(&format!("the browser's file picker failed ({name}): its file input instead"));
            input_open(accept, multi, done);
        }
    }
}

/// Open through `<input type=file>` (one group of types: every filter's
/// extensions, any file when the Filter has "All files"). Its `cancel`
/// event is the user's Cancel.
fn input_open(accept: &str, multi: bool, done: impl FnOnce(Picked) + 'static) {
    let doc = crate::page_web::document();
    let Ok(input) = doc.create_element("input").map(|e| e.unchecked_into::<web_sys::HtmlInputElement>()) else {
        done(Picked::Cancelled);
        return;
    };
    input.set_type("file");
    input.set_multiple(multi);
    if !accept.is_empty() {
        input.set_accept(accept);
    }
    input.set_class_name("rr-file-input");
    let _ = input.style().set_property("display", "none");
    if let Some(body) = doc.body() {
        let _ = body.append_child(&input);
    }
    let done: Once = Rc::new(RefCell::new(Some(Box::new(done))));
    let changed = {
        let (done, source) = (done.clone(), input.clone());
        Closure::<dyn FnMut()>::new(move || {
            let Some(done) = done.borrow_mut().take() else { return };
            let files: Vec<web_sys::File> = source.files().map(|l| (0..l.length()).filter_map(|i| l.item(i)).collect()).unwrap_or_default();
            source.remove();
            spawn_local(async move {
                let mut names = Vec::new();
                for file in files {
                    let Ok(buffer) = JsFuture::from(file.array_buffer()).await else { continue };
                    let name = file.name();
                    // (a file input's file can't be written back: its writes
                    // stay in the browser's store)
                    keep(&name, Uint8Array::new(&buffer).to_vec(), None);
                    names.push(name);
                }
                done(if names.is_empty() { Picked::Cancelled } else { Picked::Files(names) });
            });
        })
    };
    let cancelled = {
        let (done, source) = (done.clone(), input.clone());
        Closure::<dyn FnMut()>::new(move || {
            source.remove();
            if let Some(done) = done.borrow_mut().take() {
                done(Picked::Cancelled);
            }
        })
    };
    let _ = input.add_event_listener_with_callback("change", changed.as_ref().unchecked_ref());
    let _ = input.add_event_listener_with_callback("cancel", cancelled.as_ref().unchecked_ref());
    changed.forget();
    cancelled.forget();
    input.click();
}

/// Save: where the user saves the file; `done` gets its name. No picker
/// here: [`Picked::NoSavePicker`] (the host asks for the name, and
/// [`save_as_download`] makes its writes downloads).
pub fn save(req: &Request, gestured: bool, done: impl FnOnce(Picked) + 'static) {
    if !has_save_picker() {
        done(Picked::NoSavePicker);
        return;
    }
    if !gestured && !gesture_active() {
        done(Picked::NeedsGesture);
        return;
    }
    let opts = options(req);
    let (p, through_ide) = match file_host().filter(|h| get(h, "save").is_function()) {
        Some(host) => (call(&host, "save", &[opts.into()]), true),
        None => {
            let p = match direct("showSaveFilePicker").map(|f| f.call1(&JsValue::UNDEFINED, &opts)) {
                Some(Ok(v)) => v.dyn_into::<Promise>().unwrap_or_else(|v| Promise::resolve(&v)),
                Some(Err(e)) => Promise::reject(&e),
                None => Promise::reject(&JsValue::from_str("no save picker")),
            };
            (p, false)
        }
    };
    spawn_local(async move {
        match JsFuture::from(p).await {
            Ok(v) if v.is_null() || v.is_undefined() => done(Picked::Cancelled),
            Ok(v) => {
                let name = get(&v, "name").as_string().unwrap_or_default();
                if name.is_empty() {
                    done(Picked::Cancelled);
                    return;
                }
                remember(&name, Some(if through_ide { Target::Token(get(&v, "token")) } else { Target::Handle(v) }));
                done(Picked::Files(vec![name]));
            }
            Err(e) => match error_name(&e).as_str() {
                "AbortError" => done(Picked::Cancelled),
                _ if wants_gesture(&e, gestured) => done(Picked::NeedsGesture),
                name => {
                    warn(&format!("the browser's save picker failed ({name}): a download instead"));
                    done(Picked::NoSavePicker);
                }
            },
        }
    });
}

/// A name the user gave where there's no save picker: what the program
/// writes to it goes out as a download of that name.
pub fn save_as_download(name: &str) {
    remember(name, Some(Target::Download));
}

/// The program wrote `bytes` to `path` (the page's file store has them):
/// to the real file too, when a dialog answered that name.
pub fn written(path: &str, bytes: &[u8]) {
    let key = crate::object_web::file_key(path);
    let Some((name, target)) = TARGETS.with(|t| t.borrow().get(&key).cloned()) else { return };
    OUT.with(|o| o.borrow_mut().insert(key.clone(), bytes.to_vec()));
    if BUSY.with(|b| !b.borrow_mut().insert(key.clone())) {
        // (being written: this one follows)
        return;
    }
    if let Target::Download = target {
        let later = Closure::once_into_js(move || {
            if let Some(bytes) = OUT.with(|o| o.borrow_mut().remove(&key)) {
                download(&name, &bytes);
            }
            BUSY.with(|b| b.borrow_mut().remove(&key));
        });
        if let Some(w) = window() {
            let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(later.unchecked_ref(), DOWNLOAD_QUIET_MS);
        }
        return;
    }
    spawn_local(async move {
        while let Some(bytes) = OUT.with(|o| o.borrow_mut().remove(&key)) {
            let result = match &target {
                Target::Handle(h) => write_handle(h, &bytes).await,
                Target::Token(token) => match file_host() {
                    Some(host) => JsFuture::from(call(&host, "write", &[token.clone(), Uint8Array::from(bytes.as_slice()).into()])).await.map(|_| ()),
                    None => Err(JsValue::from_str("the IDE is gone")),
                },
                Target::Download => Ok(()),
            };
            if let Err(e) = result {
                let why = error_name(&e);
                warn(&format!("can't write {name} to the computer ({}); the browser keeps it", if why.is_empty() { e.as_string().unwrap_or_default() } else { why }));
            }
        }
        BUSY.with(|b| b.borrow_mut().remove(&key));
    });
}

/// `bytes` into the file of handle `h` (its whole content).
async fn write_handle(h: &JsValue, bytes: &[u8]) -> Result<(), JsValue> {
    let w = JsFuture::from(call(h, "createWritable", &[])).await?;
    JsFuture::from(call(&w, "write", &[Uint8Array::from(bytes).into()])).await?;
    JsFuture::from(call(&w, "close", &[])).await?;
    Ok(())
}

/// `bytes` downloaded as `name` (the browser's Downloads, or where it asks).
fn download(name: &str, bytes: &[u8]) {
    let parts = Array::of1(&Uint8Array::from(bytes));
    let options = web_sys::BlobPropertyBag::new();
    options.set_type("application/octet-stream");
    let Ok(blob) = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &options) else { return };
    let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) else { return };
    let doc = crate::page_web::document();
    let Ok(a) = doc.create_element("a").map(|e| e.unchecked_into::<web_sys::HtmlAnchorElement>()) else { return };
    a.set_href(&url);
    a.set_download(&fd::file_title(name));
    let _ = a.style().set_property("display", "none");
    if let Some(body) = doc.body() {
        let _ = body.append_child(&a);
    }
    a.click();
    a.remove();
    let revoke = Closure::once_into_js(move || {
        let _ = web_sys::Url::revoke_object_url(&url);
    });
    if let Some(w) = window() {
        let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(revoke.unchecked_ref(), 30_000);
    }
}

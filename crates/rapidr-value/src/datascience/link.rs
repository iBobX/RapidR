//! Linked data (docs/data-science-plan.md §3, §4): sources, data sets, the
//! current row, and the controls bound to them — Delphi's and Xojo's
//! linked components, the RapidR way, one model for native builds, the
//! interpreter, the web and RapidR Studio's designer.
//!
//! - **Data sets** hold a table — a frame in the engine's store under the
//!   component's own name: an RDATAFRAME, an **RCSV** (a CSV file read
//!   lazily: setting FileName marks it stale, it reads once when its table
//!   is first needed), an RJSON used as a source (its document's records).
//! - **`RDataFrame.Source`** names another data set: the frame gets its rows
//!   (a copy) at once, and again whenever the source changes (a new
//!   FileName, Load, the source frame changed); `Refresh` asks again.
//! - **Cursors**: an **RDataSource** (DataSet = a data set) owns a current
//!   row; a control bound straight to a data set uses that data set's own
//!   cursor, so `Grid.DataSource = People` and `Edit.DataSource = People`
//!   move together with nothing else to set.
//! - **Bound controls**: `DataSource` + `DataField` on REdit (its Text) and
//!   RLabel (its Caption) show the current row's field; an REdit the user
//!   changes writes it back into the data set's frame ([`edited`]).
//!   **RDBGrid** draws its data set straight from the frame (the UI
//!   kernel's `components/dbgrid.rs`; its view sorted by a heading's
//!   click, the data untouched).
//!
//! The module keeps the links and what every bound control last showed.
//! After anything that may change data — a data component's member, a link
//! property, a user's edit, a click in a grid — the runtime calls [`sync`]
//! and applies the [`Update`]s it returns with its own property setter and
//! redraw (`rp_comp_set`), and fires the events. Frames carry versions
//! ([`super::frame::version`]), so a sync with nothing changed is a few
//! comparisons, and one with no links at all is nothing.
//!
//! Ids are component names, lowercase (`key`). On a design surface the
//! designer makes the designed form's data components under the surface's
//! own names (`surface:people`, [`design_begin`]): links resolve inside the
//! same surface, files are read relative to the designed program's folder
//! and only inside it, nothing is written.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use super::frame::{self, Frame};
use super::key;
use crate::{v_int, v_null, v_str, Value};

/// A Boolean property's value as RapidQ's components read one: 1 or 0.
fn flag(b: bool) -> Value {
    v_int(b as i64)
}

/// A condition's value as BASIC's True and False: -1 or 0 (so `NOT` of
/// it is the other one — NOT is bitwise).
fn truth(b: bool) -> Value {
    v_int(-(b as i64))
}

/// The components this module models whole (their properties, methods and
/// events): the runtimes route them here (`objects::get` / `set` / `call`).
pub const TYPES: &[&str] = &["RCSV", "RDATASOURCE"];

/// Most rows a source reads at design time (the designer shows them; a
/// grid draws only what is on screen).
pub const DESIGN_ROWS: usize = 1_000_000;

/// What the runtime does after a change ([`sync`]).
#[derive(Clone, Debug, PartialEq)]
pub enum Update {
    /// Component, property, value: a bound control shows its field
    /// (`Edit.Text`, `Label.Caption`).
    Set(String, String, Value),
    /// The component is drawn again (an RDBGrid whose data or row moved).
    Redraw(String),
    /// Component, event, arguments: OnLoad, OnError(Message), OnDataChange.
    Fire(String, String, Vec<Value>),
    /// A message for the program's author (stderr, the page's console).
    Warn(String),
}

/// An RCSV.
#[derive(Clone, Debug)]
struct Csv {
    file: String,
    delim: u8,
    header: bool,
    /// Its data given as text (`Text = …`) instead of a file.
    text: Option<String>,
    /// To be read before its table is next needed.
    stale: bool,
    error: String,
}

impl Default for Csv {
    fn default() -> Csv {
        Csv { file: String::new(), delim: b',', header: true, text: None, stale: false, error: String::new() }
    }
}

/// An RDataSource.
#[derive(Clone, Debug)]
struct Dsrc {
    dataset: String,
    enabled: bool,
    /// (data set, version, row) when OnDataChange last fired.
    seen: Option<(String, u64, usize)>,
}

impl Default for Dsrc {
    fn default() -> Dsrc {
        Dsrc { dataset: String::new(), enabled: true, seen: None }
    }
}

/// A current row and Delphi's EOF / BOF.
#[derive(Clone, Copy, Debug, Default)]
struct Cursor {
    row: usize,
    eof: bool,
    bof: bool,
}

/// A control bound by DataSource + DataField.
#[derive(Clone, Debug, Default)]
struct Bound {
    source: String,
    field: String,
    /// The value it was last given.
    shown: Option<String>,
}

/// An RDBGrid's view.
#[derive(Clone, Debug, Default)]
struct Grid {
    source: String,
    top: usize,
    read_only: bool,
    /// The column its view is sorted by, descending.
    sort: Option<(usize, bool)>,
    /// The view's order for (data set, version, sort): data rows in view
    /// order, and each data row's place in the view.
    order: Option<(String, u64, (usize, bool), Rc<Vec<usize>>, Rc<Vec<usize>>)>,
    /// (data set, version, row, top, sort) when last drawn.
    seen: Option<(String, u64, usize, usize, Option<(usize, bool)>)>,
}

/// An RDataFrame with a Source.
#[derive(Clone, Debug, Default)]
struct Pull {
    source: String,
    /// (the source's id, its version) when the rows were last copied.
    pulled: Option<(String, u64)>,
}

#[derive(Default)]
struct State {
    /// The types of the components links can name or bind.
    types: HashMap<String, String>,
    csv: HashMap<String, Csv>,
    dsrc: HashMap<String, Dsrc>,
    cursors: HashMap<String, Cursor>,
    bound: HashMap<String, Bound>,
    grids: HashMap<String, Grid>,
    pulls: HashMap<String, Pull>,
    /// RJSONs used as sources: their FileName (the designer's), and
    /// whether their document must be read again.
    json_file: HashMap<String, String>,
    json_stale: HashMap<String, bool>,
    /// Design surfaces (`surface` → the designed program's folder).
    design: HashMap<String, String>,
    /// Events and messages waiting for the next sync.
    out: Vec<Update>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
    /// Something changed since the last sync (a frame, a link, a cursor).
    static DIRTY: Cell<bool> = const { Cell::new(false) };
    /// The runtime's RJSON documents: a component's JSON text.
    static JSON_READER: Cell<Option<fn(&str) -> Option<String>>> = const { Cell::new(None) };
    /// What a design-time read last gave: (path, options) → (length, hash, frame).
    static DESIGN_CACHE: RefCell<HashMap<(String, u8, bool), (usize, u64, Frame)>> = RefCell::new(HashMap::new());
}

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    STATE.with(|s| f(&mut s.borrow_mut()))
}

fn mark() {
    DIRTY.with(|d| d.set(true));
}

/// Frame `name` changed (its version moved on): the next [`sync`] looks.
pub fn frame_changed(_name: &str) {
    mark();
}

/// Whether a [`sync`] may have something to do.
pub fn pending() -> bool {
    DIRTY.with(Cell::get) || with(|s| !s.out.is_empty())
}

/// The runtime's RJSON documents (installed once by each runtime): the
/// JSON text of RJSON `name`, if it has a document (or a FileName).
pub fn set_json_reader(reader: fn(&str) -> Option<String>) {
    JSON_READER.with(|r| r.set(Some(reader)));
}

/// RJSON `name`'s document changed (Parse, Set, LoadFile …): a frame that
/// takes its rows from it gets them again.
pub fn json_changed(name: &str) {
    let id = key(name);
    if with(|s| s.json_stale.contains_key(&id)) {
        with(|s| s.json_stale.insert(id, true));
        mark();
    }
}

/// `target`, a component named by component `from`'s property, as an id:
/// on a design surface, the same surface's.
fn resolve(from: &str, target: &str) -> String {
    let t = key(target);
    if t.is_empty() {
        return t;
    }
    match key(from).split_once(':') {
        Some((surface, _)) if !t.contains(':') => format!("{surface}:{t}"),
        _ => t,
    }
}

/// The design surface `id` is on.
fn surface_of(id: &str) -> Option<String> {
    key(id).split_once(':').map(|(s, _)| s.to_string())
}

/// A text as a Boolean the way RapidR reads one ("True", -1, 1, "yes").
fn truthy(v: &Value) -> bool {
    match v {
        Value::String(s) => matches!(s.trim().to_ascii_lowercase().as_str(), "true" | "yes" | "-1" | "1"),
        v => v.to_bool(),
    }
}

/// A Delimiter as given: its first character, or a name (`tab`, `\t`).
fn delimiter_of(v: &Value) -> u8 {
    let s = v.to_string_val();
    match s.trim().to_ascii_lowercase().as_str() {
        "tab" | "\\t" | "\t" => b'\t',
        "semicolon" => b';',
        "comma" | "" => b',',
        "space" => b' ',
        "pipe" => b'|',
        _ => s.bytes().next().filter(u8::is_ascii).unwrap_or(b','),
    }
}

// ---------------------------------------------------------------------------
// Components made, properties, methods
// ---------------------------------------------------------------------------

/// Component `id` of `type_name` made (every component: the runtimes call
/// it from `objects::create`). Whether this module models it whole
/// ([`TYPES`]).
pub fn create(id: &str, type_name: &str) -> bool {
    let ty = type_name.trim().to_ascii_uppercase();
    let ty = crate::designer::model::canonical_type(&ty);
    let relevant = matches!(ty.as_str(), "RCSV" | "RDATASOURCE" | "RDBGRID" | "RDATAFRAME" | "RJSON" | "REDIT" | "RLABEL");
    if !relevant {
        return false;
    }
    let id = key(id);
    with(|s| {
        s.types.insert(id.clone(), ty.clone());
        s.bound.remove(&id);
        s.cursors.remove(&id);
        match ty.as_str() {
            "RCSV" => {
                s.csv.insert(id.clone(), Csv::default());
            }
            "RDATASOURCE" => {
                s.dsrc.insert(id.clone(), Dsrc::default());
            }
            "RDBGRID" => {
                s.grids.insert(id.clone(), Grid { read_only: true, ..Grid::default() });
            }
            "RJSON" => {
                s.json_stale.insert(id.clone(), true);
            }
            _ => {}
        }
    });
    TYPES.contains(&ty.as_str())
}

/// Whether any component links can name has been made (most programs:
/// none — every lookup is skipped).
fn active() -> bool {
    with(|s| !(s.csv.is_empty() && s.dsrc.is_empty() && s.grids.is_empty() && s.pulls.is_empty() && s.bound.is_empty() && s.json_file.is_empty()))
}

/// Whether this module models component `id` whole (an RCSV, an
/// RDataSource).
pub fn exists(id: &str) -> bool {
    let id = key(id);
    with(|s| s.csv.contains_key(&id) || s.dsrc.contains_key(&id))
}

/// Whether `id` is an RDBGrid.
pub fn is_grid(id: &str) -> bool {
    let id = key(id);
    with(|s| s.grids.contains_key(&id))
}

fn type_of(id: &str) -> String {
    with(|s| s.types.get(id).cloned().unwrap_or_default())
}

/// Component `id`'s property `prop` (lowercase) — an RCSV's, an
/// RDataSource's, an RDBGrid's; None: not one this module keeps.
pub fn get(id: &str, prop: &str) -> Option<Value> {
    if !active() {
        return None;
    }
    let id = key(id);
    let ty = type_of(&id);
    match ty.as_str() {
        "RCSV" => {
            let rows = || {
                ensure(&id);
                frame::with_frame(&id, |f| (f.height(), f.width(), f.columns.join(",")))
            };
            Some(match prop {
                "filename" => v_str(&with(|s| s.csv.get(&id).map(|c| c.file.clone()).unwrap_or_default())),
                "delimiter" => v_str(&with(|s| s.csv.get(&id).map(|c| (c.delim as char).to_string()).unwrap_or_default())),
                "hasheader" => flag(with(|s| s.csv.get(&id).is_some_and(|c| c.header))),
                "rowcount" => v_int(rows().0 as i64),
                "colcount" => v_int(rows().1 as i64),
                "columns" => v_str(&rows().2),
                "text" => {
                    ensure(&id);
                    let delim = with(|s| s.csv.get(&id).map_or(b',', |c| c.delim));
                    v_str(&frame::with_frame(&id, |f| frame::to_csv_opts(f, delim, true)))
                }
                "error" => {
                    ensure(&id);
                    v_str(&with(|s| s.csv.get(&id).map(|c| c.error.clone()).unwrap_or_default()))
                }
                _ => return None,
            })
        }
        "RDATASOURCE" => {
            let (dataset, enabled) = with(|s| s.dsrc.get(&id).map(|d| (d.dataset.clone(), d.enabled)).unwrap_or_default());
            let set = resolve(&id, &dataset);
            let rows = || {
                if set.is_empty() || !enabled {
                    return 0;
                }
                ensure(&set);
                frame::with_frame(&set, Frame::height)
            };
            Some(match prop {
                "dataset" => v_str(&dataset),
                "enabled" => flag(enabled),
                "row" => {
                    clamp(&id, rows());
                    v_int(cursor(&id).row as i64)
                }
                "recordcount" => v_int(rows() as i64),
                // (True as BASIC's -1, so `WHILE NOT DS.EOF` ends: NOT is
                // bitwise, NOT 1 would be -2)
                "eof" => {
                    let n = rows();
                    clamp(&id, n);
                    truth(n == 0 || cursor(&id).eof)
                }
                "bof" => {
                    let n = rows();
                    clamp(&id, n);
                    truth(n == 0 || cursor(&id).bof)
                }
                "state" => v_str(if set.is_empty() || !enabled { "inactive" } else { "browse" }),
                _ => return None,
            })
        }
        "RDBGRID" => {
            let g = with(|s| s.grids.get(&id).cloned())?;
            Some(match prop {
                "datasource" => v_str(&g.source),
                "readonly" => flag(g.read_only),
                "toprow" => v_int(g.top as i64),
                "row" => v_int(grid(&id).map_or(0, |d| d.row) as i64),
                "rowcount" => v_int(grid(&id).map_or(0, |d| d.rows) as i64),
                "colcount" => v_int(grid(&id).map_or(0, |d| frame::with_frame(&d.dataset, Frame::width)) as i64),
                "columns" => v_str(&grid(&id).map(|d| frame::with_frame(&d.dataset, |f| f.columns.join(","))).unwrap_or_default()),
                "sortcolumn" => v_int(g.sort.map_or(-1, |(c, _)| c as i64)),
                "sortdescending" => flag(g.sort.is_some_and(|(_, d)| d)),
                _ => return None,
            })
        }
        _ => None,
    }
}

/// Sets component `id`'s property `prop` (lowercase). `Some`: this module
/// keeps it (an RCSV's, an RDataSource's, an RDBGrid's own); `None`: not
/// its — but a link property of another component (`DataSource`,
/// `DataField`, an RDataFrame's `Source`) is noted on the way, and the
/// runtime keeps the value too.
pub fn set(id: &str, prop: &str, val: &Value) -> Option<Result<(), String>> {
    if !active() && !matches!(prop, "datasource" | "datafield" | "source") {
        return None;
    }
    let id = key(id);
    let ty = type_of(&id);
    let text = val.to_string_val();
    let r = match (ty.as_str(), prop) {
        ("RCSV", "filename") => {
            with(|s| {
                if let Some(c) = s.csv.get_mut(&id) {
                    c.file = text.trim().to_string();
                    c.text = None;
                    c.stale = true;
                }
            });
            Some(Ok(()))
        }
        ("RCSV", "delimiter") => {
            with(|s| {
                if let Some(c) = s.csv.get_mut(&id) {
                    c.delim = delimiter_of(val);
                    c.stale = true;
                }
            });
            Some(Ok(()))
        }
        ("RCSV", "hasheader") => {
            with(|s| {
                if let Some(c) = s.csv.get_mut(&id) {
                    c.header = truthy(val);
                    c.stale = true;
                }
            });
            Some(Ok(()))
        }
        ("RCSV", "text") => {
            with(|s| {
                if let Some(c) = s.csv.get_mut(&id) {
                    c.text = Some(text.clone());
                    c.file.clear();
                    c.stale = true;
                }
            });
            Some(Ok(()))
        }
        ("RCSV", "rowcount" | "colcount" | "columns" | "error") => Some(Err(format!("{prop} can only be read"))),
        ("RDATASOURCE", "dataset") => {
            with(|s| {
                if let Some(d) = s.dsrc.get_mut(&id) {
                    d.dataset = text.trim().to_string();
                }
                s.cursors.remove(&id);
            });
            Some(Ok(()))
        }
        ("RDATASOURCE", "enabled") => {
            with(|s| {
                if let Some(d) = s.dsrc.get_mut(&id) {
                    d.enabled = truthy(val);
                }
            });
            Some(Ok(()))
        }
        ("RDATASOURCE", "row") => {
            move_to(&id, Move::To(val.to_i64()));
            Some(Ok(()))
        }
        ("RDATASOURCE", "recordcount" | "eof" | "bof" | "state") => Some(Err(format!("{prop} can only be read"))),
        ("RDBGRID", "datasource") => {
            with(|s| {
                if let Some(g) = s.grids.get_mut(&id) {
                    g.source = text.trim().to_string();
                    g.sort = None;
                    g.top = 0;
                    g.seen = None;
                }
            });
            Some(Ok(()))
        }
        ("RDBGRID", "readonly") => {
            with(|s| {
                if let Some(g) = s.grids.get_mut(&id) {
                    g.read_only = truthy(val);
                }
            });
            Some(Ok(()))
        }
        ("RDBGRID", "row") => {
            if let Some(d) = grid(&id) {
                move_to(&d.owner, Move::To(val.to_i64()));
            }
            Some(Ok(()))
        }
        ("RDBGRID", "toprow") => {
            let top = usize::try_from(val.to_i64()).unwrap_or(0);
            with(|s| {
                if let Some(g) = s.grids.get_mut(&id) {
                    g.top = top;
                }
            });
            Some(Ok(()))
        }
        ("RDBGRID", "rowcount" | "colcount" | "columns" | "sortcolumn" | "sortdescending") => Some(Err(format!("{prop} can only be read"))),
        // (link properties of other components: noted, kept by the runtime too)
        ("RDATAFRAME", "source") => {
            with(|s| {
                s.pulls.insert(id.clone(), Pull { source: text.trim().to_string(), pulled: None });
            });
            mark();
            ensure(&id);
            return None;
        }
        (_, "datasource") if ty != "RDBGRID" => {
            with(|s| {
                let b = s.bound.entry(id.clone()).or_default();
                b.source = text.trim().to_string();
                b.shown = None;
            });
            mark();
            return None;
        }
        (_, "datafield") => {
            with(|s| {
                let b = s.bound.entry(id.clone()).or_default();
                b.field = text.trim().to_string();
                b.shown = None;
            });
            mark();
            return None;
        }
        // (the designer's: an RJSON's FileName, read as a source)
        ("RJSON", "filename") if surface_of(&id).is_some() => {
            with(|s| {
                s.json_file.insert(id.clone(), text.trim().to_string());
                s.json_stale.insert(id.clone(), true);
            });
            mark();
            return None;
        }
        _ => None,
    };
    if r.is_some() {
        mark();
    }
    r
}

/// Calls component `id`'s method `method` (lowercase): an RCSV's, an
/// RDataSource's. None: not one of them.
pub fn call(id: &str, method: &str, args: &[Value]) -> Option<Result<Value, String>> {
    let id = key(id);
    let ty = type_of(&id);
    let arg = |i: usize| args.get(i).map(Value::to_string_val).unwrap_or_default();
    let r = match (ty.as_str(), method) {
        ("RCSV", "load") => {
            with(|s| {
                if let Some(c) = s.csv.get_mut(&id) {
                    c.stale = true;
                }
            });
            ensure(&id);
            let ok = with(|s| s.csv.get(&id).is_some_and(|c| c.error.is_empty()));
            Ok(v_int(ok as i64))
        }
        ("RCSV", "save") => {
            ensure(&id);
            let (file, delim, header) = with(|s| s.csv.get(&id).map(|c| (c.file.clone(), c.delim, c.header)).unwrap_or_default());
            let target = if args.is_empty() { file } else { arg(0).trim().to_string() };
            if surface_of(&id).is_some() {
                return Some(Ok(v_int(0)));
            }
            if target.is_empty() {
                queue(Update::Fire(id.clone(), "onerror".into(), vec![v_str("no FileName to save to")]));
                queue(Update::Warn(format!("RCSV {id}: Save has no FileName to write to")));
                return Some(Ok(v_int(0)));
            }
            let text = frame::with_frame(&id, |f| frame::to_csv_opts(f, delim, header));
            match frame::write_text(&target, &text) {
                Ok(()) => Ok(v_int(1)),
                Err(e) => {
                    let msg = format!("can't write \"{target}\" ({e})");
                    queue(Update::Fire(id.clone(), "onerror".into(), vec![v_str(&msg)]));
                    queue(Update::Warn(format!("RCSV {id}: {msg}")));
                    Ok(v_int(0))
                }
            }
        }
        ("RCSV", "clear") => {
            with(|s| {
                if let Some(c) = s.csv.get_mut(&id) {
                    c.file.clear();
                    c.text = None;
                    c.stale = false;
                    c.error.clear();
                }
            });
            frame::set_frame(&id, Frame::default());
            Ok(v_null())
        }
        ("RDATASOURCE", "first") => {
            move_to(&id, Move::First);
            Ok(v_null())
        }
        ("RDATASOURCE", "last") => {
            move_to(&id, Move::Last);
            Ok(v_null())
        }
        ("RDATASOURCE", "next") => {
            move_to(&id, Move::By(1));
            Ok(v_null())
        }
        ("RDATASOURCE", "prior") => {
            move_to(&id, Move::By(-1));
            Ok(v_null())
        }
        ("RDATASOURCE", "moveby") => {
            move_to(&id, Move::By(args.first().map_or(0, Value::to_i64)));
            Ok(v_null())
        }
        ("RDATASOURCE", "field") => Ok(v_str(&field_value(&id, &arg(0)).unwrap_or_default())),
        ("RDATASOURCE", "setfield") => {
            set_field(&id, &arg(0), &arg(1));
            Ok(v_null())
        }
        ("RDATASOURCE", "refresh") => {
            let set = with(|s| s.dsrc.get(&id).map(|d| resolve(&id, &d.dataset))).unwrap_or_default();
            refresh(&set);
            Ok(v_null())
        }
        _ => return None,
    };
    mark();
    Some(r)
}

/// RDATAFRAME `name`'s members this module answers (`Refresh`); None:
/// the frame's own.
pub fn dataframe_call(name: &str, method: &str) -> Option<Value> {
    match method {
        "refresh" => {
            refresh(&key(name));
            Some(v_null())
        }
        _ => None,
    }
}

/// Data set `id` read again from where its rows come from.
fn refresh(id: &str) {
    if id.is_empty() {
        return;
    }
    with(|s| {
        if let Some(c) = s.csv.get_mut(id) {
            c.stale = true;
        }
        if let Some(p) = s.pulls.get_mut(id) {
            p.pulled = None;
        }
        if s.json_stale.contains_key(id) {
            s.json_stale.insert(id.to_string(), true);
        }
    });
    mark();
    ensure(id);
}

fn queue(u: Update) {
    with(|s| s.out.push(u));
}

// ---------------------------------------------------------------------------
// Data sets made current
// ---------------------------------------------------------------------------

/// Data set `id`'s table made current: an RCSV read if stale, an RJSON's
/// records if its document changed, an RDataFrame's rows copied again from
/// its Source if that changed.
pub fn ensure(id: &str) {
    ensure_at(&key(id), 0);
}

fn ensure_at(id: &str, depth: u8) {
    if depth > 16 || id.is_empty() {
        return;
    }
    match type_of(id).as_str() {
        "RCSV" => {
            if with(|s| s.csv.get(id).is_some_and(|c| c.stale)) {
                load_csv(id);
            }
        }
        "RJSON" => {
            if with(|s| s.json_stale.get(id).copied().unwrap_or(false)) {
                load_json(id);
            }
        }
        "RDATAFRAME" => {
            let Some(src) = with(|s| s.pulls.get(id).map(|p| p.source.clone())) else { return };
            let src = resolve(id, &src);
            if src.is_empty() || src == id {
                return;
            }
            ensure_at(&src, depth + 1);
            let now = (src.clone(), frame::version(&src));
            if with(|s| s.pulls.get(id).and_then(|p| p.pulled.clone())) == Some(now.clone()) {
                return;
            }
            with(|s| {
                if let Some(p) = s.pulls.get_mut(id) {
                    p.pulled = Some(now);
                }
            });
            frame::set_frame(id, frame::frame(&src));
        }
        _ => {}
    }
}

/// A data file's text at run time: `file` as given (the current folder,
/// then where the engine looks: `frame::read_text`), else beside the
/// program (`rapidr run data/app.rr` reads `data/people.csv` for
/// "people.csv", as the designer does).
fn read_beside(file: &str) -> Result<String, String> {
    match frame::read_text(file) {
        Ok(t) => Ok(t),
        Err(e) => {
            let f = file.replace('\\', "/");
            let relative = !(f.starts_with('/') || f.as_bytes().get(1) == Some(&b':') || f.contains("://"));
            let program = crate::command_line::path().replace('\\', "/");
            match program.rsplit_once('/') {
                Some((dir, _)) if relative && !dir.is_empty() => frame::read_text(&format!("{dir}/{f}")).map_err(|_| e),
                _ => Err(e),
            }
        }
    }
}

/// Why file `file` couldn't be read, in plain words (`e`: the system's
/// message, maybe "can't read <path>: <why>").
fn unreadable(file: &str, e: &str) -> String {
    let why = e.rsplit_once(": ").map_or(e, |(_, w)| w);
    let why = why.split(" (os error").next().unwrap_or(why).trim();
    if why.to_ascii_lowercase().contains("no such file") || why.to_ascii_lowercase().contains("not found") {
        format!("there is no file \"{file}\"")
    } else {
        format!("can't read \"{file}\": {why}")
    }
}

/// A design-time path: inside the designed program's folder only.
fn design_path(base: &str, file: &str) -> Result<String, String> {
    let f = file.replace('\\', "/");
    if f.contains("://") {
        return Err(format!("\"{file}\" is not read at design time (only files in the project)"));
    }
    let base = base.trim_end_matches('/');
    if f.starts_with('/') || f.as_bytes().get(1) == Some(&b':') {
        let inside = !base.is_empty() && f.starts_with(&format!("{base}/")) && !f.split('/').any(|p| p == "..");
        return if inside { Ok(f) } else { Err(format!("\"{file}\" is outside the project: not read at design time")) };
    }
    if f.split('/').any(|p| p == "..") {
        return Err(format!("\"{file}\" is outside the project: not read at design time"));
    }
    Ok(if base.is_empty() { f } else { format!("{base}/{f}") })
}

/// A file's text at design time (no other folders tried).
fn design_text(path: &str) -> Result<String, String> {
    crate::objects::read_file(path).map(|b| String::from_utf8(b).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned()))
}

/// A quick fingerprint of a file's text (the design-time cache's).
fn fingerprint(text: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for chunk in text.as_bytes().chunks(8) {
        let mut w = [0u8; 8];
        w[..chunk.len()].copy_from_slice(chunk);
        h = (h ^ u64::from_le_bytes(w)).wrapping_mul(0x0000_0100_0000_01b3).rotate_left(29);
    }
    h
}

fn load_csv(id: &str) {
    let Some(c) = with(|s| s.csv.get(id).cloned()) else { return };
    let design = surface_of(id).map(|surface| with(|s| s.design.get(&surface).cloned().unwrap_or_default()));
    let result: Result<Frame, String> = if let Some(text) = &c.text {
        Ok(frame::parse_csv_opts(text, c.delim, c.header))
    } else if c.file.is_empty() {
        Ok(Frame::default())
    } else if let Some(base) = &design {
        design_path(base, &c.file).and_then(|path| {
            let text = design_text(&path).map_err(|e| unreadable(&c.file, &e))?;
            let k = (path, c.delim, c.header);
            let fp = (text.len(), fingerprint(&text));
            if let Some(f) = DESIGN_CACHE.with(|d| d.borrow().get(&k).filter(|(n, h, _)| (*n, *h) == fp).map(|(_, _, f)| f.clone())) {
                return Ok(f);
            }
            let mut f = frame::parse_csv_opts(&text, c.delim, c.header);
            if f.height() > DESIGN_ROWS {
                f = f.head(DESIGN_ROWS);
            }
            DESIGN_CACHE.with(|d| d.borrow_mut().insert(k, (fp.0, fp.1, f.clone())));
            Ok(f)
        })
    } else {
        read_beside(&c.file).map(|t| frame::parse_csv_opts(&t, c.delim, c.header)).map_err(|e| unreadable(&c.file, &e))
    };
    let (f, error) = match result {
        Ok(f) => (f, String::new()),
        Err(e) => (Frame::default(), e),
    };
    with(|s| {
        if let Some(c) = s.csv.get_mut(id) {
            c.stale = false;
            c.error = error.clone();
        }
    });
    frame::set_frame(id, f);
    if error.is_empty() {
        queue(Update::Fire(id.to_string(), "onload".into(), Vec::new()));
    } else {
        queue(Update::Fire(id.to_string(), "onerror".into(), vec![v_str(&error)]));
        queue(Update::Warn(format!("RCSV {id}: {error}")));
    }
}

fn load_json(id: &str) {
    let file = with(|s| s.json_file.get(id).cloned().unwrap_or_default());
    if surface_of(id).is_none() && file.is_empty() && JSON_READER.with(Cell::get).is_none() {
        // (the runtime hasn't said where documents are yet: later)
        return;
    }
    with(|s| s.json_stale.insert(id.to_string(), false));
    let text = match surface_of(id) {
        Some(surface) => {
            if file.is_empty() {
                return;
            }
            let base = with(|s| s.design.get(&surface).cloned().unwrap_or_default());
            design_path(&base, &file).and_then(|p| design_text(&p))
        }
        None => match JSON_READER.with(Cell::get).and_then(|r| r(id)) {
            Some(t) => Ok(t),
            None if !file.is_empty() => frame::read_text(&file),
            None => return,
        },
    };
    match text.map(|t| frame::parse_json_frame(&t)) {
        Ok(Some(mut f)) => {
            if surface_of(id).is_some() && f.height() > DESIGN_ROWS {
                f = f.head(DESIGN_ROWS);
            }
            frame::set_frame(id, f);
        }
        Ok(None) => queue(Update::Warn(format!("RJSON {id}: its document isn't JSON records ([{{…}}, …], JSON Lines or columns)"))),
        Err(e) => queue(Update::Warn(format!("RJSON {id}: can't read \"{file}\" ({e})"))),
    }
}

// ---------------------------------------------------------------------------
// Cursors
// ---------------------------------------------------------------------------

/// A link's target as (data set, cursor owner): an RDataSource's data set
/// and itself; any other data set itself, its own cursor. None: nothing to
/// show (no target, an RDataSource with no DataSet or not Enabled).
fn dataset_of(target: &str) -> Option<(String, String)> {
    let t = key(target);
    if t.is_empty() {
        return None;
    }
    let ty = with(|s| s.types.get(&t).cloned());
    match ty.as_deref() {
        Some("RDATASOURCE") => with(|s| {
            let d = s.dsrc.get(&t)?;
            let set = resolve(&t, &d.dataset);
            (d.enabled && !set.is_empty() && set != t).then_some((set, t.clone()))
        }),
        Some("RDATAFRAME" | "RCSV" | "RJSON") => Some((t.clone(), t)),
        // (anything else that has a frame by its name; an RSQLITE is the
        // web's own binding's until rapidr-db joins this one: plan §4.6)
        _ => frame::has_frame(&t).then(|| (t.clone(), t)),
    }
}

fn cursor(owner: &str) -> Cursor {
    with(|s| s.cursors.get(owner).copied().unwrap_or(Cursor { row: 0, eof: false, bof: true }))
}

/// Cursor `owner` kept inside a data set of `rows` rows.
fn clamp(owner: &str, rows: usize) {
    with(|s| {
        let c = s.cursors.entry(owner.to_string()).or_insert(Cursor { row: 0, eof: false, bof: true });
        if rows == 0 {
            c.row = 0;
        } else if c.row >= rows {
            c.row = rows - 1;
        }
    });
}

enum Move {
    First,
    Last,
    By(i64),
    To(i64),
}

/// The rows of cursor `owner`'s data set (made current first).
fn owner_rows(owner: &str) -> usize {
    match dataset_of(owner) {
        Some((set, _)) => {
            ensure(&set);
            frame::with_frame(&set, Frame::height)
        }
        None => 0,
    }
}

fn move_to(owner: &str, m: Move) {
    let owner = key(owner);
    let rows = owner_rows(&owner);
    with(|s| {
        let c = s.cursors.entry(owner.clone()).or_insert(Cursor { row: 0, eof: false, bof: true });
        let last = rows.saturating_sub(1) as i64;
        let (row, eof, bof) = match m {
            Move::First => (0, rows == 0, true),
            Move::Last => (last, true, rows == 0),
            Move::By(d) => {
                let want = c.row as i64 + d;
                (want.clamp(0, last), rows == 0 || want > last, rows == 0 || want < 0)
            }
            Move::To(r) => (r.clamp(0, last), rows == 0, rows == 0),
        };
        *c = Cursor { row: row as usize, eof, bof };
    });
    mark();
}

/// Moves the cursor of what component `id` is bound to (an RDBGrid's row
/// clicked, the arrows) to data row `row`.
pub fn set_row(id: &str, row: usize) {
    let id = key(id);
    let owner = match with(|s| s.grids.get(&id).map(|g| g.source.clone())) {
        Some(src) => dataset_of(&resolve(&id, &src)).map(|(_, o)| o),
        None => Some(id),
    };
    if let Some(o) = owner {
        move_to(&o, Move::To(row as i64));
    }
}

/// The current row's value of `field` through RDataSource / data set
/// `owner` (None: no such column, or no row).
fn field_value(owner: &str, field: &str) -> Option<String> {
    let (set, owner) = dataset_of(owner)?;
    ensure(&set);
    let rows = frame::with_frame(&set, Frame::height);
    clamp(&owner, rows);
    let row = cursor(&owner).row;
    frame::with_frame(&set, |f| {
        let c = f.col(field)?;
        (row < f.height()).then(|| f.cell(row, c).map(|s| s.into_owned()).unwrap_or_default())
    })
}

/// The current row's `field` through `owner` becomes `value` (an empty
/// value: missing).
fn set_field(owner: &str, field: &str, value: &str) {
    let Some((set, owner)) = dataset_of(owner) else { return };
    ensure(&set);
    let row = cursor(&owner).row;
    let changed = frame::with_frame(&set, |f| f.col(field).filter(|_| row < f.height()).map(|c| (c, f.cell(row, c).map(|s| s.into_owned()).unwrap_or_default() != value)));
    if let Some((c, true)) = changed {
        frame::with_frame_mut(&set, |f| f.set_cell(row, c, (!value.is_empty()).then_some(value)));
    }
}

/// The user changed bound control `id`'s text to `text` (an REdit's
/// OnChange): its field takes it. Whether it is bound.
pub fn edited(id: &str, text: &str) -> bool {
    let id = key(id);
    if surface_of(&id).is_some() {
        return false;
    }
    let Some(b) = with(|s| s.bound.get(&id).cloned()) else { return false };
    if b.source.is_empty() || b.field.is_empty() || type_of(&id) != "REDIT" {
        return false;
    }
    with(|s| {
        if let Some(b) = s.bound.get_mut(&id) {
            b.shown = Some(text.to_string());
        }
    });
    set_field(&resolve(&id, &b.source), &b.field, text);
    true
}

// ---------------------------------------------------------------------------
// The grid's view
// ---------------------------------------------------------------------------

/// What an RDBGrid shows now.
#[derive(Clone, Debug, PartialEq)]
pub struct GridData {
    /// Its data set (the frame's name) and the cursor it follows.
    pub dataset: String,
    pub owner: String,
    pub rows: usize,
    /// The current row (a data row).
    pub row: usize,
    pub version: u64,
    pub top: usize,
    pub sort: Option<(usize, bool)>,
    pub read_only: bool,
}

/// RDBGrid `id`'s data (its data set made current first). None: not
/// bound to anything that has a table.
pub fn grid(id: &str) -> Option<GridData> {
    let id = key(id);
    let g = with(|s| s.grids.get(&id).cloned())?;
    let (dataset, owner) = dataset_of(&resolve(&id, &g.source))?;
    ensure(&dataset);
    let rows = frame::with_frame(&dataset, Frame::height);
    clamp(&owner, rows);
    let sort = g.sort.filter(|(c, _)| *c < frame::with_frame(&dataset, Frame::width));
    Some(GridData { version: frame::version(&dataset), row: cursor(&owner).row, top: g.top.min(rows.saturating_sub(1)), rows, dataset, owner, sort, read_only: g.read_only })
}

/// RDBGrid `id`'s view order while it is sorted: the data rows in view
/// order, and each data row's place in the view.
pub fn grid_order(id: &str, d: &GridData) -> Option<(Rc<Vec<usize>>, Rc<Vec<usize>>)> {
    let id = key(id);
    let sort = d.sort?;
    if let Some((rows, places)) = with(|s| {
        s.grids.get(&id).and_then(|g| g.order.as_ref()).filter(|(set, v, k, _, _)| *set == d.dataset && *v == d.version && *k == sort).map(|(_, _, _, r, p)| (r.clone(), p.clone()))
    }) {
        return Some((rows, places));
    }
    let order = frame::with_frame(&d.dataset, |f| f.sorted_order(sort.0, sort.1));
    let mut places = vec![0; order.len()];
    for (i, &r) in order.iter().enumerate() {
        if r < places.len() {
            places[r] = i;
        }
    }
    let (rows, places) = (Rc::new(order), Rc::new(places));
    with(|s| {
        if let Some(g) = s.grids.get_mut(&id) {
            g.order = Some((d.dataset.clone(), d.version, sort, rows.clone(), places.clone()));
        }
    });
    Some((rows, places))
}

/// RDBGrid `id`'s first row shown (in view order).
pub fn set_grid_top(id: &str, top: usize) {
    let id = key(id);
    with(|s| {
        if let Some(g) = s.grids.get_mut(&id) {
            g.top = top;
        }
    });
}

/// RDBGrid `id`'s heading `col` clicked: its view sorted by that column,
/// then descending, then back to the data's order.
pub fn grid_title_click(id: &str, col: usize) {
    let id = key(id);
    with(|s| {
        if let Some(g) = s.grids.get_mut(&id) {
            g.sort = match g.sort {
                Some((c, false)) if c == col => Some((col, true)),
                Some((c, true)) if c == col => None,
                _ => Some((col, false)),
            };
        }
    });
    mark();
}

// ---------------------------------------------------------------------------
// Sync
// ---------------------------------------------------------------------------

/// What changed since the last sync, as the runtime applies it: bound
/// controls showing their fields again, grids drawn again, events.
pub fn sync() -> Vec<Update> {
    if !DIRTY.with(Cell::get) || !active() {
        DIRTY.with(|d| d.set(false));
        return with(|s| std::mem::take(&mut s.out));
    }
    // (every data set a link names, made current)
    let (frames, dsrcs, bound, grids) = with(|s| {
        let frames: Vec<String> = s.pulls.keys().cloned().collect();
        let dsrcs: Vec<String> = s.dsrc.keys().cloned().collect();
        let bound: Vec<(String, Bound)> = s.bound.iter().filter(|(_, b)| !b.source.is_empty() && !b.field.is_empty()).map(|(k, b)| (k.clone(), b.clone())).collect();
        let grids: Vec<String> = s.grids.keys().cloned().collect();
        (frames, dsrcs, bound, grids)
    });
    for f in &frames {
        ensure(f);
    }
    // (RDataSources: their rows kept inside, OnDataChange)
    for d in &dsrcs {
        let Some((set, owner)) = dataset_of(d) else { continue };
        ensure(&set);
        let rows = frame::with_frame(&set, Frame::height);
        clamp(&owner, rows);
        let now = (set.clone(), frame::version(&set), cursor(&owner).row);
        // (the first look only notes it: OnDataChange is a change — the
        // program's form may not even be made yet)
        let fire = with(|s| {
            let ds = s.dsrc.get_mut(d)?;
            let first = ds.seen.is_none();
            (ds.seen.as_ref() != Some(&now)).then(|| ds.seen = Some(now)).filter(|_| !first)
        });
        if fire.is_some() {
            queue(Update::Fire(d.clone(), "ondatachange".into(), Vec::new()));
        }
    }
    // (bound controls: their field of the current row)
    for (id, b) in bound {
        let prop = match type_of(&id).as_str() {
            "RLABEL" => "caption",
            "REDIT" => "text",
            _ => continue,
        };
        let target = resolve(&id, &b.source);
        if dataset_of(&target).is_none() {
            continue;
        }
        let value = field_value(&target, &b.field).unwrap_or_default();
        if b.shown.as_deref() != Some(value.as_str()) {
            with(|s| {
                if let Some(b) = s.bound.get_mut(&id) {
                    b.shown = Some(value.clone());
                }
            });
            queue(Update::Set(id.clone(), prop.into(), v_str(&value)));
        }
    }
    // (grids whose data, row or view changed: drawn again)
    for id in grids {
        let Some(d) = grid(&id) else { continue };
        let now = (d.dataset.clone(), d.version, d.row, d.top, d.sort);
        let redraw = with(|s| {
            let g = s.grids.get_mut(&id)?;
            (g.seen.as_ref() != Some(&now)).then(|| g.seen = Some(now))
        });
        if redraw.is_some() {
            queue(Update::Redraw(id.clone()));
        }
    }
    DIRTY.with(|d| d.set(false));
    with(|s| std::mem::take(&mut s.out))
}

// ---------------------------------------------------------------------------
// Design time
// ---------------------------------------------------------------------------

/// Design surface `surface` (re)builds its designed form's data
/// components: what it had is forgotten, files are read from `folder` (the
/// designed program's) and only inside it.
pub fn design_begin(surface: &str, folder: &str) {
    let surface = key(surface);
    let prefix = format!("{surface}:");
    let gone: Vec<String> = with(|s| {
        let gone: Vec<String> = s.types.keys().filter(|k| k.starts_with(&prefix)).cloned().collect();
        for k in &gone {
            s.types.remove(k);
            s.csv.remove(k);
            s.dsrc.remove(k);
            s.cursors.remove(k);
            s.bound.remove(k);
            s.grids.remove(k);
            s.pulls.remove(k);
            s.json_file.remove(k);
            s.json_stale.remove(k);
        }
        s.design.insert(surface.clone(), folder.replace('\\', "/"));
        s.out.retain(|u| !matches!(u, Update::Set(i, ..) | Update::Redraw(i) | Update::Fire(i, ..) if i.starts_with(&prefix)));
        gone
    });
    for k in gone {
        frame::remove_frame(&k);
    }
}

/// The designed form's links worked out (after [`design_begin`] and the
/// components' properties): what its bound controls show, as
/// (component, property, value); events and redraws are the designer's
/// to ignore, messages are returned for its banner.
pub fn design_sync(surface: &str) -> (Vec<(String, String, Value)>, Vec<String>) {
    let prefix = format!("{}:", key(surface));
    let mut sets = Vec::new();
    let mut warnings = Vec::new();
    let mut keep = Vec::new();
    for u in sync() {
        match u {
            Update::Set(id, p, v) if id.starts_with(&prefix) => sets.push((id, p, v)),
            Update::Warn(w) if w.contains(&prefix) => warnings.push(w.replace(&prefix, "")),
            Update::Set(..) | Update::Redraw(..) | Update::Fire(..) | Update::Warn(..) => {
                // (a running program's, sharing the thread: kept for it)
                let mine = match &u {
                    Update::Redraw(i) | Update::Fire(i, ..) => i.starts_with(&prefix),
                    _ => false,
                };
                if !mine {
                    keep.push(u);
                }
            }
        }
    }
    if !keep.is_empty() {
        with(|s| s.out.extend(keep));
        mark();
    }
    (sets, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup(names: &[(&str, &str)]) {
        for (n, t) in names {
            create(n, t);
        }
    }

    fn set_s(id: &str, prop: &str, v: &str) {
        set(id, prop, &v_str(v));
    }

    const PEOPLE: &str = "name,age,city\nbob,30,NYC\namy,25,LA\nLee,41,NYC\n";

    #[test]
    fn csv_source_frame_datasource_and_bound_controls() {
        setup(&[("t1csv", "RCSV"), ("t1df", "RDATAFRAME"), ("t1ds", "RDATASOURCE"), ("t1edit", "REDIT"), ("t1label", "RLABEL"), ("t1grid", "RDBGRID")]);
        set_s("t1csv", "text", PEOPLE);
        set_s("t1df", "source", "T1CSV");
        assert_eq!(frame::with_frame("t1df", Frame::height), 3);
        set_s("t1ds", "dataset", "t1df");
        set_s("t1edit", "datasource", "t1ds");
        set_s("t1edit", "datafield", "name");
        set_s("t1label", "datasource", "t1ds");
        set_s("t1label", "datafield", "AGE");
        set_s("t1grid", "datasource", "t1ds");
        let ups = sync();
        assert!(ups.contains(&Update::Set("t1edit".into(), "text".into(), v_str("bob"))), "{ups:?}");
        assert!(ups.contains(&Update::Set("t1label".into(), "caption".into(), v_str("30"))));
        assert!(ups.contains(&Update::Redraw("t1grid".into())));
        // (the first look isn't a change)
        assert!(!ups.iter().any(|u| matches!(u, Update::Fire(_, e, _) if e == "ondatachange")), "{ups:?}");
        assert!(ups.contains(&Update::Fire("t1csv".into(), "onload".into(), vec![])));
        // (nothing changed: nothing to do)
        assert!(sync().is_empty());
        call("t1ds", "next", &[]);
        let ups = sync();
        assert!(ups.contains(&Update::Set("t1edit".into(), "text".into(), v_str("amy"))));
        assert!(ups.contains(&Update::Fire("t1ds".into(), "ondatachange".into(), vec![])));
        assert_eq!(get("t1ds", "row"), Some(v_int(1)));
        // (the user types: the frame takes it, nothing echoes back)
        assert!(edited("t1edit", "Amy"));
        assert_eq!(frame::frame("t1df").cell(1, 0).as_deref(), Some("Amy"));
        let ups = sync();
        assert!(!ups.iter().any(|u| matches!(u, Update::Set(id, ..) if id == "t1edit")), "{ups:?}");
        assert!(ups.contains(&Update::Redraw("t1grid".into())));
        // (EOF the Delphi way)
        call("t1ds", "next", &[]);
        assert_eq!(get("t1ds", "eof"), Some(v_int(0)));
        call("t1ds", "next", &[]);
        assert_eq!(get("t1ds", "eof"), Some(v_int(-1)));
        assert_eq!(get("t1ds", "row"), Some(v_int(2)));
        assert_eq!(call("t1ds", "field", &[v_str("city")]), Some(Ok(v_str("NYC"))));
        // (the source changes: the frame follows)
        set_s("t1csv", "text", "name,age\nzed,9\n");
        let ups = sync();
        assert!(ups.contains(&Update::Set("t1edit".into(), "text".into(), v_str("zed"))), "{ups:?}");
        assert!(ups.contains(&Update::Set("t1label".into(), "caption".into(), v_str("9"))));
        assert_eq!(get("t1ds", "recordcount"), Some(v_int(1)));
    }

    #[test]
    fn grid_bound_straight_shares_the_frames_cursor() {
        setup(&[("t2df", "RDATAFRAME"), ("t2grid", "RDBGRID"), ("t2edit", "REDIT")]);
        frame::set_frame("t2df", frame::parse_csv(PEOPLE));
        set_s("t2grid", "datasource", "t2df");
        set_s("t2edit", "datasource", "t2df");
        set_s("t2edit", "datafield", "city");
        sync();
        set_row("t2grid", 1);
        let ups = sync();
        assert!(ups.contains(&Update::Set("t2edit".into(), "text".into(), v_str("LA"))), "{ups:?}");
        let d = grid("t2grid").unwrap();
        assert_eq!((d.rows, d.row), (3, 1));
        // (a heading clicked: the view sorted, the data as it was)
        grid_title_click("t2grid", 1);
        let d = grid("t2grid").unwrap();
        let (order, places) = grid_order("t2grid", &d).unwrap();
        assert_eq!(*order, vec![1, 0, 2]);
        assert_eq!(places[1], 0);
        assert_eq!(frame::frame("t2df").cell(0, 0).as_deref(), Some("bob"));
        grid_title_click("t2grid", 1);
        let d = grid("t2grid").unwrap();
        assert_eq!(*grid_order("t2grid", &d).unwrap().0, vec![2, 0, 1]);
        grid_title_click("t2grid", 1);
        assert!(grid("t2grid").unwrap().sort.is_none());
    }

    #[test]
    fn design_time_names_and_paths() {
        assert_eq!(resolve("ds1:grid1", "People"), "ds1:people");
        assert_eq!(resolve("grid1", "People"), "people");
        assert!(design_path("/p/app", "data/x.csv").is_ok_and(|p| p == "/p/app/data/x.csv"));
        assert!(design_path("/p/app", "../secret.csv").is_err());
        assert!(design_path("/p/app", "/etc/passwd").is_err());
        assert!(design_path("/p/app", "/p/app/x.csv").is_ok());
        assert!(design_path("/p/app", "https://x/y.csv").is_err());
        design_begin("t3s", "");
        create("t3s:csv", "RCSV");
        create("t3s:edit", "REDIT");
        set_s("t3s:csv", "text", PEOPLE);
        set_s("t3s:edit", "datasource", "CSV");
        set_s("t3s:edit", "datafield", "name");
        let (sets, _) = design_sync("t3s");
        assert_eq!(sets, vec![("t3s:edit".to_string(), "text".to_string(), v_str("bob"))]);
        // (a save at design time writes nothing)
        assert_eq!(call("t3s:csv", "save", &[v_str("/tmp/never.csv")]), Some(Ok(v_int(0))));
        design_begin("t3s", "");
        assert!(!frame::has_frame("t3s:csv"));
    }
}

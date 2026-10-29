//! Linked data (docs/data-science-plan.md §3, §4): sources, data sets, the
//! current row, and the controls bound to them — the
//! linked components, the RapidR way, one model for native builds, the
//! interpreter, the web and RapidR Studio's designer.
//!
//! - **Data sets** hold a table — a frame in the engine's store under the
//!   component's own name: an RDATAFRAME, an **RCSV** (a CSV file read
//!   lazily: setting FileName marks it stale, it reads once when its table
//!   is first needed; FileName can name a `$RESOURCE`, the file built into
//!   the program — the designer reads the file it names), an RJSON used as
//!   a source (its document's records).
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
//! - **Bound charts**: an **RPlot** with a `DataSource` (a data set, or an
//!   RDataSource's) draws its rows as a chart — `Kind` (bar, barh, line,
//!   scatter, area, step, pie, hist), `X` (the column along the bottom:
//!   names for bars and pies), `Y` (one column or several, a series each) —
//!   again whenever the data change ([`chart`]); its titles and labels stay
//!   the plot's own, and its imperative methods draw as before when it has
//!   no DataSource.
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

/// A current row and RapidQ's EOF / BOF.
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
    /// Why the text the user typed isn't its field's value (a number
    /// column's letters): its `DataError`; "" when the field took it.
    error: String,
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

/// An RPlot bound to a data set.
#[derive(Clone, Debug)]
struct Chart {
    source: String,
    kind: String,
    x: String,
    y: String,
    /// (data set, version) when last drawn.
    seen: Option<(String, u64)>,
}

impl Default for Chart {
    fn default() -> Chart {
        Chart { source: String::new(), kind: "bar".into(), x: String::new(), y: String::new(), seen: None }
    }
}

/// A chart's kinds (`Kind`), as RPlot's methods name them.
pub const CHART_KINDS: &[&str] = &["bar", "barh", "line", "scatter", "area", "step", "pie", "hist"];

/// Most rows a bound chart draws (the first ones): bars and points past
/// this would be a blur, and a million rows would cost a frame's time.
pub const CHART_ROWS: usize = 10_000;

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
    charts: HashMap<String, Chart>,
    pulls: HashMap<String, Pull>,
    /// RJSONs used as sources: their FileName (the designer's), and
    /// whether their document must be read again.
    json_file: HashMap<String, String>,
    json_stale: HashMap<String, bool>,
    /// Why an RJSON's records couldn't be read (its `Error`; none: they could).
    json_error: HashMap<String, String>,
    /// Design surfaces (`surface` → the designed program's folder).
    design: HashMap<String, String>,
    /// Design surfaces' `$RESOURCE`s: `surface` → (NAME → its file).
    design_res: HashMap<String, HashMap<String, String>>,
    /// Events and messages waiting for the next sync.
    out: Vec<Update>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
    /// Something changed since the last sync (a frame, a link, a cursor).
    static DIRTY: Cell<bool> = const { Cell::new(false) };
    /// The runtime's RJSON documents: a component's JSON text.
    static JSON_READER: Cell<Option<JsonReader>> = const { Cell::new(None) };
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

/// A runtime's RJSON documents: RJSON `name`'s JSON text — its document,
/// else its FileName's file —, `Err((FileName, why))` when that file can't
/// be read, None when it has neither.
pub type JsonReader = fn(&str) -> Option<Result<String, (String, String)>>;

/// The runtime's RJSON documents (installed once by each runtime): the
/// JSON text of RJSON `name`, if it has a document (or a FileName).
pub fn set_json_reader(reader: JsonReader) {
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
    let relevant = matches!(ty.as_str(), "RCSV" | "RDATASOURCE" | "RDBGRID" | "RDATAFRAME" | "RJSON" | "REDIT" | "RLABEL" | "RPLOT" | "RSQLITE" | "RMYSQL");
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
            "RPLOT" => {
                s.charts.insert(id.clone(), Chart::default());
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
    with(|s| !(s.csv.is_empty() && s.dsrc.is_empty() && s.grids.is_empty() && s.charts.is_empty() && s.pulls.is_empty() && s.bound.is_empty() && s.json_file.is_empty()))
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
        "RPLOT" => {
            let c = with(|s| s.charts.get(&id).cloned())?;
            Some(match prop {
                "datasource" => v_str(&c.source),
                "kind" => v_str(&c.kind),
                "x" => v_str(&c.x),
                "y" => v_str(&c.y),
                _ => return None,
            })
        }
        // (an RJSON as a source: why its records couldn't be read — its
        // other members are the object's own)
        "RJSON" if prop == "error" => {
            ensure(&id);
            Some(v_str(&with(|s| s.json_error.get(&id).cloned().unwrap_or_default())))
        }
        // (a bound REdit: why what the user typed isn't its field's value)
        "REDIT" if prop == "dataerror" => Some(v_str(&with(|s| s.bound.get(&id).map(|b| b.error.clone()).unwrap_or_default()))),
        _ => None,
    }
}

/// Sets component `id`'s property `prop` (lowercase). `Some`: this module
/// keeps it (an RCSV's, an RDataSource's, an RDBGrid's own); `None`: not
/// its — but a link property of another component (`DataSource`,
/// `DataField`, an RDataFrame's `Source`) is noted on the way, and the
/// runtime keeps the value too.
pub fn set(id: &str, prop: &str, val: &Value) -> Option<Result<(), String>> {
    if !active() && !matches!(prop, "datasource" | "datafield" | "source" | "filename" | "kind" | "x" | "y") {
        return None;
    }
    let id = key(id);
    let ty = type_of(&id);
    let text = val.to_string_val();
    let r = match (ty.as_str(), prop) {
        ("RCSV", "filename") => {
            // (a $RESOURCE's name — its handle, a number —: the file built
            // into the program, read through its pseudo path)
            let file = match val {
                Value::Integer(_) | Value::Double(_) => crate::resources::path(val.to_i64()).unwrap_or_else(|| text.trim().to_string()),
                _ => text.trim().to_string(),
            };
            with(|s| {
                if let Some(c) = s.csv.get_mut(&id) {
                    c.file = file;
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
                    let source = text.trim().to_string();
                    // (bound again to what it shows — a program that binds
                    // before each query, its ORDER BY's rows streaming in —:
                    // the sort stays, so a heading's next click goes on from
                    // it, ascending → descending → none; another source
                    // starts unsorted)
                    if !source.eq_ignore_ascii_case(&g.source) {
                        g.sort = None;
                    }
                    g.source = source;
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
        // (RapidR's: the program says how the view is sorted — a database's
        // rows it sorted itself with ORDER BY, -1 for none: the arrow
        // follows; another data set's view is sorted by it)
        ("RDBGRID", "sortcolumn") => {
            let c = val.to_i64();
            with(|s| {
                if let Some(g) = s.grids.get_mut(&id) {
                    g.sort = usize::try_from(c).ok().map(|c| (c, g.sort.is_some_and(|(_, d)| d)));
                }
            });
            mark();
            Some(Ok(()))
        }
        ("RDBGRID", "sortdescending") => {
            let d = truthy(val);
            with(|s| {
                if let Some(g) = s.grids.get_mut(&id) {
                    g.sort = g.sort.map(|(c, _)| (c, d));
                }
            });
            mark();
            Some(Ok(()))
        }
        ("RDBGRID", "rowcount" | "colcount" | "columns") => Some(Err(format!("{prop} can only be read"))),
        ("RPLOT", "datasource" | "kind" | "x" | "y") => {
            let kind = text.trim().to_ascii_lowercase();
            if prop == "kind" && !CHART_KINDS.contains(&kind.as_str()) {
                return Some(Err(format!("Kind \"{}\" isn't a chart's kind: {}", text.trim(), CHART_KINDS.join(", "))));
            }
            with(|s| {
                if let Some(c) = s.charts.get_mut(&id) {
                    let field = if prop == "datasource" {
                        &mut c.source
                    } else if prop == "x" {
                        &mut c.x
                    } else if prop == "y" {
                        &mut c.y
                    } else {
                        &mut c.kind
                    };
                    *field = if prop == "kind" { kind } else { text.trim().to_string() };
                    c.seen = None;
                }
            });
            Some(Ok(()))
        }
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
        // (an RJSON's FileName: its records read again — the designer's
        // from the file; a program's through its document, else the file)
        ("RJSON", "filename") => {
            with(|s| {
                if surface_of(&id).is_some() {
                    s.json_file.insert(id.clone(), text.trim().to_string());
                }
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
            // (the program's own value: as it gives it)
            let _ = set_field(&id, &arg(0), &arg(1), false);
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
/// then where the engine looks: `frame::read_text`), else among the
/// program's own files (`rapidr run data/app.rr` reads `data/people.csv`
/// for "people.csv", as the designer does; a built app the one it came
/// with: `command_line::files_folder`, a macOS app's Contents/Resources).
pub fn read_beside(file: &str) -> Result<String, String> {
    crate::command_line::read_or_programs_own(file, frame::read_text)
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
pub fn design_path(base: &str, file: &str) -> Result<String, String> {
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
    crate::objects::read_file(path).map(|b| crate::objects::codec::decode_text(&b))
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
    let mut file = with(|s| s.json_file.get(id).cloned().unwrap_or_default());
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
            Some(Ok(t)) => Ok(t),
            Some(Err((named, why))) => {
                file = named;
                Err(why)
            }
            None if !file.is_empty() => frame::read_text(&file),
            None => {
                with(|s| s.json_error.remove(id));
                return;
            }
        },
    };
    let error = match text.map(|t| frame::parse_json_frame(&t)) {
        Ok(Some(mut f)) => {
            if surface_of(id).is_some() && f.height() > DESIGN_ROWS {
                f = f.head(DESIGN_ROWS);
            }
            frame::set_frame(id, f);
            None
        }
        Ok(None) => {
            queue(Update::Warn(format!("RJSON {id}: its document isn't JSON records ([{{…}}, …], JSON Lines or columns)")));
            Some(if file.is_empty() { "its document isn't JSON records ([{…}, …], JSON Lines or columns)".to_string() } else { format!("\"{file}\" isn't JSON records ([{{…}}, …], JSON Lines or columns)") })
        }
        Err(e) => {
            queue(Update::Warn(format!("RJSON {id}: can't read \"{file}\" ({e})")));
            Some(unreadable(&file, &e))
        }
    };
    with(|s| match error {
        Some(e) => {
            s.json_error.insert(id.to_string(), e);
        }
        None => {
            s.json_error.remove(id);
        }
    });
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
        // (a database component: its last query's rows, once it has some —
        // a background query's as they arrive: rapidr-db)
        Some("RSQLITE" | "RMYSQL") => frame::has_frame(&t).then(|| (t.clone(), t)),
        // (anything else that has a frame by its name)
        _ => frame::has_frame(&t).then(|| (t.clone(), t)),
    }
}

/// Whether `id` is a database component (RSQLITE, RMYSQL): its rows are
/// sorted by the program asking again (ORDER BY), never in a grid's view.
fn is_database(id: &str) -> bool {
    matches!(type_of(&key(id)).as_str(), "RSQLITE" | "RMYSQL")
}

/// Whether something names component `id` as its data: a grid's, a bound
/// control's or a chart's DataSource, an RDataSource's DataSet, a frame's
/// Source (rapidr-db then keeps its Query's rows as a frame too).
pub fn is_source(id: &str) -> bool {
    let id = key(id);
    if !active() {
        return false;
    }
    with(|s| {
        let names = |from: &String, target: &String| !target.is_empty() && resolve(from, target) == id;
        s.grids.iter().any(|(k, g)| names(k, &g.source))
            || s.bound.iter().any(|(k, b)| names(k, &b.source))
            || s.charts.iter().any(|(k, c)| names(k, &c.source))
            || s.dsrc.iter().any(|(k, d)| names(k, &d.dataset))
            || s.pulls.iter().any(|(k, p)| names(k, &p.source))
    })
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

/// Data set `id`'s rows were replaced by new ones (a database component's
/// next query): the cursors over it — its own, an RDataSource's — back on
/// the first row, and the grids that show it scrolled to the top (a
/// current row left at the old rows' last would scroll the new ones there).
pub fn rows_replaced(id: &str) {
    let id = key(id);
    let first = Cursor { row: 0, eof: false, bof: true };
    with(|s| {
        let dsrcs: Vec<String> = s.dsrc.iter().filter(|(k, d)| !d.dataset.is_empty() && resolve(k, &d.dataset) == id).map(|(k, _)| k.clone()).collect();
        s.cursors.insert(id.clone(), first);
        for d in &dsrcs {
            s.cursors.insert(d.clone(), first);
        }
        for (k, g) in s.grids.iter_mut() {
            let src = resolve(k, &g.source);
            if !g.source.is_empty() && (src == id || dsrcs.contains(&src)) {
                g.top = 0;
            }
        }
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
/// value: missing) — `check`ed (what the user typed): only when it is one
/// of the column's type, a number column taking numbers only (Err: why
/// not, the field unchanged).
fn set_field(owner: &str, field: &str, value: &str, check: bool) -> Result<(), String> {
    let Some((set, owner)) = dataset_of(owner) else { return Ok(()) };
    ensure(&set);
    let row = cursor(&owner).row;
    let changed = frame::with_frame(&set, |f| {
        f.col(field)
            .filter(|_| row < f.height())
            .map(|c| (c, f.cell(row, c).map(|s| s.into_owned()).unwrap_or_default() != value, if check { misfit(f, c, field, value) } else { None }))
    });
    match changed {
        Some((_, _, Some(why))) => Err(why),
        Some((c, true, None)) => {
            frame::with_frame_mut(&set, |f| f.set_cell(row, c, (!value.is_empty()).then_some(value)));
            Ok(())
        }
        _ => Ok(()),
    }
}

/// Why `value` can't be column `c`'s (named `field`) of `f`: a number
/// column's text that isn't a number. None: it can (an empty one: a
/// missing value).
fn misfit(f: &Frame, c: usize, field: &str, value: &str) -> Option<String> {
    let v = value.trim();
    if v.is_empty() {
        return None;
    }
    match f.dtype(c) {
        frame::DType::Int if v.parse::<i64>().is_err() => Some(format!("{field} is a whole number: \"{value}\" isn't one")),
        frame::DType::Float if super::parse_num(v).is_none() => Some(format!("{field} is a number: \"{value}\" isn't one")),
        _ => None,
    }
}

/// The columns of the data set component `id` is bound to (its
/// DataSource; an RDBGrid's too) — what its DataField can name, as the
/// inspector offers them (on a design surface: `surface:name`, as the
/// designer read the data). Empty: not bound, or no data.
pub fn bound_columns(id: &str) -> Vec<String> {
    let id = key(id);
    let source = with(|s| s.bound.get(&id).map(|b| b.source.clone()).or_else(|| s.grids.get(&id).map(|g| g.source.clone())).or_else(|| s.charts.get(&id).map(|c| c.source.clone())));
    let Some((set, _)) = source.and_then(|src| dataset_of(&resolve(&id, &src))) else { return Vec::new() };
    ensure(&set);
    frame::with_frame(&set, |f| f.columns.clone())
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
    let target = resolve(&id, &b.source);
    let r = set_field(&target, &b.field, text, true);
    // (taken: what it shows is the field's; refused: the user's text stays
    // in the edit — the field keeps its value, DataError says why — until
    // the row or the field changes)
    let shown = match &r {
        Ok(()) => text.to_string(),
        Err(_) => field_value(&target, &b.field).unwrap_or_default(),
    };
    with(|s| {
        if let Some(b) = s.bound.get_mut(&id) {
            b.shown = Some(shown);
            b.error = r.err().unwrap_or_default();
        }
    });
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
    // (a database's rows: the program sorts them, asking again with ORDER
    // BY on OnTitleClick — SortColumn / SortDescending say what was asked)
    if is_database(&d.dataset) {
        return None;
    }
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
// The chart's data
// ---------------------------------------------------------------------------

/// What a bound RPlot draws now: its series from the data set's rows, and
/// the axes' labels its columns give (for a plot without its own).
#[derive(Clone, Debug, PartialEq)]
pub struct ChartData {
    pub series: Vec<super::plot::Series>,
    pub xlabel: String,
    pub ylabel: String,
    /// More than one series: a legend says which is which.
    pub legend: bool,
    pub dataset: String,
    pub version: u64,
}

/// RPlot `id`'s data (its data set made current first). None: no
/// DataSource, or nothing with a table behind it.
pub fn chart(id: &str) -> Option<ChartData> {
    let id = key(id);
    let c = with(|s| s.charts.get(&id).cloned())?;
    if c.source.is_empty() {
        return None;
    }
    let (dataset, _) = dataset_of(&resolve(&id, &c.source))?;
    ensure(&dataset);
    let version = frame::version(&dataset);
    let (series, xlabel, ylabel) = frame::with_frame(&dataset, |f| chart_series(f, &c.kind, &c.x, &c.y));
    let legend = series.len() > 1;
    Some(ChartData { series, xlabel, ylabel, legend, dataset, version })
}

/// A frame's rows as a chart of `kind`: column `x` along the bottom (the
/// first column if none is named), the `y` columns (comma-separated; the
/// numeric columns but x if none) a series each — and the labels.
fn chart_series(f: &Frame, kind: &str, x: &str, y: &str) -> (Vec<super::plot::Series>, String, String) {
    use super::plot::{Series, PALETTE};
    let rows = f.height().min(CHART_ROWS);
    let xc = if x.trim().is_empty() { (f.width() > 0).then_some(0) } else { f.col(x.trim()) };
    let numeric = |c: usize| f.dtype(c).numeric();
    let mut ys: Vec<usize> = y.split(',').map(str::trim).filter(|n| !n.is_empty()).filter_map(|n| f.col(n)).collect();
    if ys.is_empty() && y.trim().is_empty() {
        ys = (0..f.width()).filter(|&c| Some(c) != xc && numeric(c)).collect();
    }
    if kind == "pie" {
        ys.truncate(1);
    }
    let num = |r: usize, c: usize| f.cell(r, c).and_then(|s| super::parse_num(s.trim()));
    let names: Vec<String> = match xc {
        Some(c) => (0..rows).map(|r| f.cell(r, c).map(|s| s.into_owned()).unwrap_or_default()).collect(),
        None => (0..rows).map(|r| r.to_string()).collect(),
    };
    // (numbers along x for lines and points over a numeric column; names
    // — bars, a pie — otherwise)
    let along = xc.filter(|&c| numeric(c) && matches!(kind, "line" | "scatter" | "area" | "step"));
    let style = match kind {
        "line" => "-",
        "scatter" => "o",
        k => k,
    };
    let mut out = Vec::new();
    for (i, &yc) in ys.iter().enumerate() {
        let label = f.columns.get(yc).cloned().unwrap_or_default();
        let color = PALETTE[i % PALETTE.len()].to_string();
        match kind {
            "pie" => {
                let values: Vec<f64> = (0..rows).map(|r| num(r, yc).unwrap_or(0.0)).collect();
                out.push(Series { x: values.clone(), y: values, label: names.join(","), color: String::new(), style: "pie".into(), categories: Vec::new() });
            }
            "hist" => {
                let data: Vec<f64> = (0..rows).filter_map(|r| num(r, yc)).collect();
                if let Some(s) = super::plot::hist_series(&data, 10, label, color) {
                    out.push(s);
                }
            }
            _ => {
                let (xs, values, categories) = match along {
                    Some(c) => {
                        let pts: Vec<(f64, f64)> = (0..rows).filter_map(|r| Some((num(r, c)?, num(r, yc)?))).collect();
                        (pts.iter().map(|p| p.0).collect(), pts.iter().map(|p| p.1).collect(), Vec::new())
                    }
                    None => ((0..rows).map(|r| r as f64).collect::<Vec<f64>>(), (0..rows).map(|r| num(r, yc).unwrap_or(0.0)).collect(), names.clone()),
                };
                let (sx, sy) = if kind == "barh" { (values, xs) } else { (xs, values) };
                out.push(Series { x: sx, y: sy, label, color, style: style.into(), categories });
            }
        }
    }
    let xlabel = if matches!(kind, "pie" | "hist") { String::new() } else { xc.and_then(|c| f.columns.get(c).cloned()).unwrap_or_default() };
    let ylabel = if ys.len() == 1 && !matches!(kind, "pie") { f.columns.get(ys[0]).cloned().unwrap_or_default() } else { String::new() };
    if kind == "barh" {
        (out, ylabel, xlabel)
    } else {
        (out, xlabel, ylabel)
    }
}

// ---------------------------------------------------------------------------
// Sync
// ---------------------------------------------------------------------------

/// What changed since the last sync, as the runtime applies it: bound
/// controls showing their fields again, grids drawn again, events.
pub fn sync() -> Vec<Update> {
    if !DIRTY.with(Cell::get) || !active() {
        DIRTY.with(|d| d.set(false));
        return take_out(None);
    }
    sync_scope(None);
    DIRTY.with(|d| d.set(false));
    take_out(None)
}

/// Whether component `id` is in a sync's scope: a program's (`None`: no
/// design surface's `surface:` names) or design surface `prefix`'s.
fn in_scope(id: &str, prefix: Option<&str>) -> bool {
    match prefix {
        None => !id.contains(':'),
        Some(p) => id.starts_with(p),
    }
}

/// The queued updates of a scope; a program's sync drops the designer's
/// (its events and messages are the designer's to ignore).
fn take_out(prefix: Option<&str>) -> Vec<Update> {
    with(|s| {
        let surfaces: Vec<String> = s.design.keys().map(|k| format!("{k}:")).collect();
        let designers = |text: &str| surfaces.iter().any(|p| text.contains(p.as_str()));
        let (mine, rest): (Vec<Update>, Vec<Update>) = std::mem::take(&mut s.out).into_iter().partition(|u| match u {
            Update::Set(id, ..) | Update::Redraw(id) | Update::Fire(id, ..) => in_scope(id, prefix),
            Update::Warn(w) => match prefix {
                None => !designers(w),
                Some(p) => w.contains(p),
            },
        });
        // (a program's sync: the designer's leftovers go; a designer's:
        // the program's stay for it)
        if prefix.is_some() {
            s.out = rest;
        }
        mine
    })
}

/// Sync for one scope (see [`in_scope`]): its data sets current, its
/// cursors kept inside, its bound controls' and grids' updates queued.
fn sync_scope(prefix: Option<&str>) {
    // (every data set a link names, made current)
    let (frames, dsrcs, bound, grids, charts) = with(|s| {
        let frames: Vec<String> = s.pulls.keys().filter(|k| in_scope(k, prefix)).cloned().collect();
        let dsrcs: Vec<String> = s.dsrc.keys().filter(|k| in_scope(k, prefix)).cloned().collect();
        let bound: Vec<(String, Bound)> = s.bound.iter().filter(|(k, b)| in_scope(k, prefix) && !b.source.is_empty() && !b.field.is_empty()).map(|(k, b)| (k.clone(), b.clone())).collect();
        let grids: Vec<String> = s.grids.keys().filter(|k| in_scope(k, prefix)).cloned().collect();
        let charts: Vec<String> = s.charts.iter().filter(|(k, c)| in_scope(k, prefix) && !c.source.is_empty()).map(|(k, _)| k.clone()).collect();
        (frames, dsrcs, bound, grids, charts)
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
        // (a database component's bound widgets are the web runtime's own
        // binding's for now: plan §4.6)
        if is_database(&target) || dataset_of(&target).is_none() {
            continue;
        }
        let value = field_value(&target, &b.field).unwrap_or_default();
        if b.shown.as_deref() != Some(value.as_str()) {
            with(|s| {
                if let Some(b) = s.bound.get_mut(&id) {
                    b.shown = Some(value.clone());
                    b.error.clear();
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
    // (charts whose data changed: drawn again)
    for id in charts {
        let Some((set, _)) = with(|s| s.charts.get(&id).map(|c| c.source.clone())).and_then(|src| dataset_of(&resolve(&id, &src))) else { continue };
        ensure(&set);
        let now = (set.clone(), frame::version(&set));
        let redraw = with(|s| {
            let c = s.charts.get_mut(&id)?;
            (c.seen.as_ref() != Some(&now)).then(|| c.seen = Some(now))
        });
        if redraw.is_some() {
            queue(Update::Redraw(id.clone()));
        }
    }
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
            s.charts.remove(k);
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

/// Design surface `surface`'s designed form, its data side made from the
/// form's CREATE blocks (after [`design_begin`]): its data sets (RCSV,
/// RJson, RDataFrame, RDataSource) and the links of its grids and bound
/// controls, as the designer reads them — the kernel's designer view
/// calls it as it builds the form, the inspector before it lists a
/// DataField's columns (so they're there before anything is drawn).
pub fn design_build(surface: &str, folder: &str, design: &crate::designer::FormDesign) {
    use crate::designer::value::PropValue;
    use crate::designer::Item;
    design_begin(surface, folder);
    let surface = key(surface);
    for id in design.ids() {
        let Some(n) = design.node(id) else { continue };
        let ty = n.canonical.clone();
        if !matches!(ty.as_str(), "RCSV" | "RJSON" | "RDATAFRAME" | "RDATASOURCE" | "RDBGRID" | "REDIT" | "RLABEL" | "RPLOT") {
            continue;
        }
        let k = format!("{surface}:{}", n.name.to_lowercase());
        create(&k, &ty);
        for item in &n.body {
            let Item::Prop(p) = item else { continue };
            let prop = crate::designer::model::prop_key(&p.name);
            // (an RCSV's FileName naming a $RESOURCE: the file it builds in —
            // the name may read as its handle, a number, to the designer)
            let resource = (prop == "filename").then(|| with(|s| s.design_res.get(&surface).and_then(|m| m.get(&p.value.trim().to_ascii_uppercase())).cloned())).flatten();
            let v = match resource {
                Some(f) => Some(v_str(&f)),
                None => match design.read_value(&p.value) {
                    PropValue::Number(f) if f.fract() == 0.0 && f.abs() < 9.0e15 => Some(v_int(f as i64)),
                    PropValue::Number(f) => Some(Value::Double(f)),
                    PropValue::Str(s) => Some(v_str(&s)),
                    _ => design_ref(&prop, &p.value),
                },
            };
            if let Some(v) = v {
                set(&k, &prop, &v);
            }
        }
    }
}

/// Design surface `surface`'s designed program's `$RESOURCE`s (its name,
/// the file it builds in): an RCSV whose FileName names one reads that
/// file while the form is designed. Given before [`design_build`].
pub fn design_resources(surface: &str, resources: &[(String, String)]) {
    let map = resources.iter().map(|(n, f)| (n.to_ascii_uppercase(), f.clone())).collect();
    with(|s| s.design_res.insert(key(surface), map));
}

/// The `$RESOURCE NAME AS "file"` lines of a program's text: (NAME, file).
pub fn resources_in(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|l| {
            let l = l.trim();
            let rest = l.get(..9).filter(|h| h.eq_ignore_ascii_case("$resource"))?;
            let rest = l[rest.len()..].trim_start();
            let (name, after) = rest.split_once(char::is_whitespace)?;
            let after = after.trim_start();
            let after = after.get(..2).filter(|a| a.eq_ignore_ascii_case("as")).map(|_| after[2..].trim_start())?;
            let file = after.strip_prefix('"')?.split('"').next()?;
            Some((name.to_string(), file.to_string()))
        })
        .collect()
}

/// A link property's value written as a component's name (`DataSource =
/// People`): the name.
pub fn design_ref(prop: &str, text: &str) -> Option<Value> {
    let t = text.trim();
    let name = !t.is_empty() && t.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') && t.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_');
    (matches!(prop, "datasource" | "dataset" | "source") && name).then(|| v_str(t))
}

/// The designed form's links worked out (after [`design_begin`] and the
/// components' properties): what its bound controls show, as
/// (component, property, value); events and redraws are the designer's
/// to ignore, messages are returned for its banner.
pub fn design_sync(surface: &str) -> (Vec<(String, String, Value)>, Vec<String>) {
    let prefix = format!("{}:", key(surface));
    sync_scope(Some(&prefix));
    let mut sets = Vec::new();
    let mut warnings = Vec::new();
    for u in take_out(Some(&prefix)) {
        match u {
            Update::Set(id, p, v) => sets.push((id, p, v)),
            Update::Warn(w) => warnings.push(w.replace(&prefix, "")),
            // (the designer runs no events; it draws again anyway)
            Update::Redraw(..) | Update::Fire(..) => {}
        }
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
        // (EOF as data sets report it)
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
        // (what the inspector offers for DataField)
        assert_eq!(bound_columns("t2edit"), ["name", "age", "city"]);
        assert_eq!(bound_columns("t2grid"), ["name", "age", "city"]);
        assert!(bound_columns("nobody").is_empty());
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

    /// (Database Studio's finding) A grid's sort stays as rows come: rows
    /// appended to its data set are sorted in; bound again to the same
    /// source (a program binding before each query) it keeps its sort, so
    /// a heading's next click goes on from it — ascending, descending,
    /// none; another source starts unsorted.
    #[test]
    fn a_grids_sort_stays_as_rows_come_and_its_source_is_set_again() {
        setup(&[("t9df", "RDATAFRAME"), ("t9other", "RDATAFRAME"), ("t9grid", "RDBGRID")]);
        frame::set_frame("t9df", frame::parse_csv(PEOPLE));
        frame::set_frame("t9other", frame::parse_csv(PEOPLE));
        set_s("t9grid", "datasource", "t9df");
        grid_title_click("t9grid", 1);
        // rows appended (a stream's next chunk): sorted in by age
        let mut b = frame::FrameBuilder::new(&["name", "age", "city"], 2);
        for (n, a, c) in [("zoe", "1", "NY"), ("max", "99", "SF")] {
            b.push(0, Some(n));
            b.push(1, Some(a));
            b.push(2, Some(c));
            b.end_row();
        }
        frame::with_frame_mut("t9df", |f| f.append(&b.finish(0)));
        let d = grid("t9grid").unwrap();
        assert_eq!((d.rows, d.sort), (5, Some((1, false))));
        let (order, _) = grid_order("t9grid", &d).unwrap();
        assert_eq!(order.first(), Some(&3), "zoe (1) first: {order:?}");
        assert_eq!(order.last(), Some(&4), "max (99) last: {order:?}");
        // bound again to the same source (any case): the sort stays, the
        // next click descends
        set_s("t9grid", "datasource", "T9DF");
        assert_eq!(get("t9grid", "sortcolumn"), Some(v_int(1)));
        grid_title_click("t9grid", 1);
        assert_eq!(grid("t9grid").unwrap().sort, Some((1, true)));
        assert_eq!(get("t9grid", "sortdescending"), Some(v_int(1)));
        // a sort the program set (its ORDER BY) is where the cycle goes on
        set("t9grid", "sortcolumn", &v_int(2));
        set("t9grid", "sortdescending", &v_int(0));
        set_s("t9grid", "datasource", "t9df");
        grid_title_click("t9grid", 2);
        assert_eq!(grid("t9grid").unwrap().sort, Some((2, true)));
        grid_title_click("t9grid", 2);
        assert_eq!(grid("t9grid").unwrap().sort, None);
        // another source: unsorted
        grid_title_click("t9grid", 0);
        set_s("t9grid", "datasource", "t9other");
        assert_eq!(get("t9grid", "sortcolumn"), Some(v_int(-1)));
    }

    #[test]
    fn cycles_disabled_sources_and_options() {
        // (two frames each other's Source: no endless pulling)
        setup(&[("t4a", "RDATAFRAME"), ("t4b", "RDATAFRAME")]);
        frame::set_frame("t4a", frame::parse_csv(PEOPLE));
        set_s("t4b", "source", "t4a");
        set_s("t4a", "source", "t4b");
        sync();
        assert_eq!(frame::with_frame("t4b", Frame::height), 3);
        // (an RDataSource not Enabled: its controls don't follow, its grid shows nothing)
        setup(&[("t4ds", "RDATASOURCE"), ("t4edit", "REDIT"), ("t4grid", "RDBGRID")]);
        set_s("t4ds", "dataset", "t4b");
        set_s("t4edit", "datasource", "t4ds");
        set_s("t4edit", "datafield", "name");
        set_s("t4grid", "datasource", "t4ds");
        set("t4ds", "enabled", &v_int(0));
        let ups = sync();
        assert!(!ups.iter().any(|u| matches!(u, Update::Set(id, ..) if id == "t4edit")), "{ups:?}");
        assert!(grid("t4grid").is_none());
        assert_eq!(get("t4ds", "state"), Some(v_str("inactive")));
        set("t4ds", "enabled", &v_str("True"));
        assert!(sync().contains(&Update::Set("t4edit".into(), "text".into(), v_str("bob"))));
        // (a tab-separated file without a header; a missing file's message)
        setup(&[("t4tab", "RCSV"), ("t4bad", "RCSV")]);
        set_s("t4tab", "delimiter", "tab");
        set("t4tab", "hasheader", &v_int(0));
        set_s("t4tab", "text", "1\t2\n3\t4\n");
        assert_eq!(get("t4tab", "columns"), Some(v_str("column_1,column_2")));
        assert_eq!(get("t4tab", "rowcount"), Some(v_int(2)));
        set_s("t4bad", "filename", "no_such_file_anywhere.csv");
        assert_eq!(get("t4bad", "rowcount"), Some(v_int(0)));
        assert_eq!(get("t4bad", "error"), Some(v_str("there is no file \"no_such_file_anywhere.csv\"")));
        let ups = sync();
        assert!(ups.iter().any(|u| matches!(u, Update::Fire(id, e, _) if id == "t4bad" && e == "onerror")), "{ups:?}");
        assert_eq!(call("t4bad", "load", &[]), Some(Ok(v_int(0))));
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

    #[test]
    fn resources_named_in_a_program() {
        assert_eq!(resources_in("x\n$RESOURCE STAFF_CSV AS \"staff.csv\"\n  $resource Pic as \"a b.png\" ' c\n$RESOURCEX y AS \"n\""), [("STAFF_CSV".to_string(), "staff.csv".to_string()), ("Pic".to_string(), "a b.png".to_string())]);
    }

    #[test]
    fn a_chart_draws_its_data_set_and_follows_it() {
        setup(&[("t5csv", "RCSV"), ("t5df", "RDATAFRAME"), ("t5ds", "RDATASOURCE"), ("t5plot", "RPLOT")]);
        set_s("t5csv", "text", "name,dept,salary,rating\nAda,Eng,90,4.5\nBob,Ops,60,3\nCy,Eng,75,4\n");
        set_s("t5df", "source", "t5csv");
        // (no DataSource: the plot's own methods draw it)
        assert_eq!(chart("t5plot"), None);
        assert_eq!(get("t5plot", "kind"), Some(v_str("bar")));
        set_s("t5plot", "datasource", "t5df");
        set_s("t5plot", "x", "name");
        set_s("t5plot", "y", "salary");
        let d = chart("t5plot").expect("bound");
        assert_eq!(d.series.len(), 1);
        assert_eq!(d.series[0].categories, ["Ada", "Bob", "Cy"]);
        assert_eq!(d.series[0].y, vec![90.0, 60.0, 75.0]);
        assert_eq!((d.series[0].style.as_str(), d.xlabel.as_str(), d.ylabel.as_str(), d.legend), ("bar", "name", "salary", false));
        // (Y empty: every numeric column but X, a legend)
        set_s("t5plot", "y", "");
        let d = chart("t5plot").unwrap();
        assert_eq!(d.series.iter().map(|s| s.label.as_str()).collect::<Vec<_>>(), ["salary", "rating"]);
        assert!(d.legend);
        // (a line over a numeric X: its numbers; a pie: X's names, Y's first column)
        set_s("t5plot", "kind", "line");
        set_s("t5plot", "x", "rating");
        set_s("t5plot", "y", "salary");
        let d = chart("t5plot").unwrap();
        assert_eq!((d.series[0].x.clone(), d.series[0].style.as_str()), (vec![4.5, 3.0, 4.0], "-"));
        set_s("t5plot", "kind", "PIE");
        set_s("t5plot", "x", "name");
        let d = chart("t5plot").unwrap();
        assert_eq!((d.series[0].label.as_str(), d.series[0].x.clone()), ("Ada,Bob,Cy", vec![90.0, 60.0, 75.0]));
        set_s("t5plot", "kind", "hist");
        assert_eq!(chart("t5plot").unwrap().series[0].y.iter().sum::<f64>(), 3.0);
        assert!(matches!(set("t5plot", "kind", &v_str("donut")), Some(Err(_))));
        // (through an RDataSource; the data change: drawn again)
        set_s("t5ds", "dataset", "t5df");
        set_s("t5plot", "kind", "bar");
        set_s("t5plot", "datasource", "t5ds");
        let _ = sync();
        assert!(!sync().contains(&Update::Redraw("t5plot".into())));
        crate::datascience::frame::with_frame_mut("t5df", |f| f.set_cell(0, 2, Some("95")));
        frame_changed("t5df");
        assert!(sync().contains(&Update::Redraw("t5plot".into())));
        assert_eq!(chart("t5plot").unwrap().series[0].y[0], 95.0);
        assert_eq!(bound_columns("t5plot"), ["name", "dept", "salary", "rating"]);
    }
}

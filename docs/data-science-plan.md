# Data science with RapidR — the plan (I7, the WOW 1.0 priority)

Robert's priority for the WOW 1.0 (ROADMAP.md, stage I7, "PRIORITY for the WOW 1.0"; docs/ide-plan.md §I7): RapidR as the best **and** the easiest data-science tool — a school kid can use it, it is fast on millions of rows, its RDataFrame covers what Polars covers in RapidR's own way, sources and data-aware components link Delphi- and Xojo-style, everything is live in RapidR Studio's designer, and every component is AI- and MCP-ready through the language registry.

This document is the architecture and the plan: the engine, the source components, the data-aware components, the BASIC API, the web's limits, licences, performance budgets and the benchmark suite, how each piece registers in the language registry, and the phases with their sizes. §12 records what phase 1 (lane DATA-ARCH) built and measured.

**Decisions this plan keeps** (made earlier, unchanged):

- **D7 — one engine, our own** (`rapidr_value::datascience`, shared by native builds, the interpreter and the web; polars and ndarray stay out). The engine grows columnar *behind the same members*; nothing a program sees changes unless this plan says so.
- **RapidQ is untouched.** Every component and member here is RapidR's own (an extension, `origin = rapidr`); RapidQ programs compile and run exactly as before (RC.EXE stays the ground truth; the corpus suite runs on every release). Properties added to RapidQ's components (`DataSource`, `DataField` on REdit, RLabel …) are additive: a program that never sets them sees nothing new.
- **One implementation, no fallbacks; web parity.** Every model lives in `rapidr-value` (or `rapidr-db`) and every runtime calls it; a runtime adds only what only it can do (where `PRINT` goes, a redraw, the page's files).

## 0. Principles

1. **Easy first.** One obvious way to do each thing; plain-English names (`Filter`, `Sort`, `GroupBy`, `Source`, `DataSource`, `DataField`), plain-English errors that say what to do (`RDataFrame.Filter: there's no column "Agee" — the columns are name, age, city`). Visual building first: drop a source in the designer, point a grid at it, see the rows.
2. **Fast at scale.** A million rows in well under a second for every everyday operation on this Mac (an M4 Max), never more than a few seconds on a school's five-year-old laptop; the UI never waits on data it doesn't show (a grid draws only its visible rows).
3. **The same answers everywhere.** Native, interpreted, the web, and the designer at design time run the same code on the same data and give byte-identical results (frames printed, cells, statistics, charts' pixels).
4. **Linked and live.** A source, a frame, a data source and the controls bound to them are components connected by properties; changing one updates the others, at design time and at run time.
5. **AI-ready by construction.** Every component, property, method and event is in the language registry with a type, a doc and — for links — the kinds of component it accepts, so the IDE's MCP server and RAI in users' programs know what exists and can build and drive data apps.
6. **Safe.** Design time never writes and reads only the project's files; database connections never put passwords into source; on the web nothing reaches a server the program didn't name.

## 1. Where we start (2026-10-15)

- **Engine** (`rapidr_value::datascience::frame`, 2,000 lines): a column keeps its cells' text end to end (offsets and a null mask beside it) and works out its type and numbers once, lazily. Measured on this Mac (`cargo run --release -p rapidr-value --example frame_bench`, 1 M rows, 36.6 MB of CSV): load 112 ms, filter 42 ms, sort 223 ms, group-by 83 ms, left join 62 ms, describe 85 ms, save 82 ms. Single-threaded; every transform copies every cell's text.
- **Members**: RDataFrame's ~60 methods in pandas' style (in-place: `df.Filter …`, `df.Sort …`), RNum (NumPy-style arrays), RPlot (one model, one renderer: the UI kernel's ops, pixel-identical on desktop and web).
- **Data binding**: none on the desktop. The **web runtime alone** binds widgets to an RSQLITE's current row (`DataSource` / `DataField`, `object_web::rp_sync_bound_widgets`, `kernel_web::bound_input`) — a parity gap this plan closes (§4.6).
- **RJSON** exists (a JSON document: Parse, Get / Set by path, LoadFile …), implemented twice (runtime-core and the web runtime).
- **Icons and toolbox group** for the data components exist already (`rapidr-icons`: RDATASOURCE, RDBGRID, RDBNAVIGATOR …, the toolbox's "Data" group).

## 2. The columnar engine

### 2.1 Columns and types

A frame is named columns of equal height. A column is one of:

| Type (`dtypes`) | Storage | Notes |
|---|---|---|
| `i64` | `Vec<i64>` + validity | whole numbers |
| `f64` | `Vec<f64>` + validity | NaN is a value, null is not (see 2.2) |
| `bool` | bitmap + validity | `true` / `false` (any case when read) |
| `str` | UTF-8 bytes + `u32`/`u64` offsets + validity | |
| `date`, `datetime`, `duration` (phase 3) | `i32` days / `i64` µs + validity | parsed from ISO 8601 and the program's locale |
| `cat` (phase 3) | dictionary codes `u32` + a `str` dictionary | what group-by keys and low-cardinality text become |

**Buffers are shared and copy-on-write** (`Arc`): a frame copied (`DF.Source = CSV1`, a design-time preview, a grid's view) costs nothing until one of them changes.

**What a program reads never changes because of storage.** RapidR's frames promise that a cell reads back as it was written (`"30.50"` stays `"30.50"`; `"007"` stays `"007"`), and that a column's type is what its cells are. The engine keeps that promise exactly: a column is stored typed when that is lossless — every cell's text is the canonical text of its value (`i64::to_string`, RapidR's own number text `float_to_str`, lower-case `true` / `false`) — and keeps its text otherwise (with its numbers worked out once, as today). Real data is overwhelmingly canonical, so it is stored typed; the rare column that isn't stays exact. The kernels see both through one view: a column's numbers (`&[f64]` / `&[i64]`), its validity, and its cells' text (`Cow<str>`: borrowed from text storage, written on demand from typed storage).

### 2.2 Missing values

- A **null** is a missing value: an empty CSV field, `NA`, `N/A`, `null`, `NULL`, `NaN` as text, JSON's `null`. Validity is a bitmap per column (absent when nothing is null).
- **Rules, the same in every kernel:** a filter never passes a null; sorts put nulls last (ascending and descending); aggregates skip nulls (`count` counts non-nulls, `mean` of no values is null); a join key that is null matches nothing; group-by keeps a null group (last); `fillna` / `dropna` as today. Arithmetic on a null gives null (phase 3 expressions). These are pandas' and Polars' defaults.

### 2.3 Chunks

Columns are stored in **chunks** (64 Ki rows a chunk, or a whole column for small frames): the unit of parallel work, of streaming (a chunk at a time from a file bigger than memory) and of appending (a new chunk, no reallocation of the old ones). Phase 1 keeps one chunk a column (contiguous `Vec`s, simplest and fastest in memory); phase 4 adds chunked storage behind the same column view.

### 2.4 Kernels: threads and SIMD

- **Threads**: work splits by rows (chunks) or by columns, on `std::thread::scope` with `available_parallelism()` workers (no thread-pool dependency; a pool only if measurements ask for one). Below ~50,000 rows everything runs on the calling thread (a thread costs more than the work). On the web (`wasm32-unknown-unknown`, no threads yet) the same code runs on the calling thread; wasm threads (SharedArrayBuffer behind COOP / COEP) are phase 6.
- **SIMD**: kernels are tight loops over typed slices written so LLVM vectorizes them (comparisons into bitmaps, sums, min / max, gathers); the web build already enables `simd128`. Hand-written `std::arch` only where a measurement shows a loop that doesn't vectorize.
- **The kernels**: CSV parsing (records split at quote-aware line boundaries, chunks parsed in parallel, types inferred per chunk and merged), filter (a selection vector or bitmap, then a parallel gather of every column), sort (numbers sorted as `(key, row)` pairs — radix-able, parallel merge of sorted runs; text by slices; stable), group-by (hash on typed keys, partitioned by hash across threads, aggregates in one pass), hash join (build on the smaller side, probe in parallel), describe / statistics (one pass per column, parallel across columns), take / gather (parallel across columns).

### 2.5 Lazy plans and the optimizer (phase 3)

RDataFrame's methods stay eager (a kid's mental model: the frame changes now). Under them, and under the transform components (RDFFilter, RDFSort, RDFGroup, RDFJoin, RDFCompute, RDFSelect, RDFLimit — Delphi's chained datasets) and `SQL`, is a **logical plan**: `Scan(source, columns, predicate) → Filter → Project → Join → GroupBy → Sort → Limit`. The optimizer: predicate pushdown (into the scan: a CSV's rows are tested as they're parsed, a Parquet's row groups skipped by their statistics, a SQL source's `WHERE`), projection pushdown (only the columns used are parsed / fetched), limit pushdown (a designer preview reads 1,000 rows, not a million), filter fusion, constant folding, and re-using an unchanged sub-plan's result (the designer re-evaluates a chain whenever a property changes). `df.Explain` prints the optimized plan.

### 2.6 Streaming and out-of-core (phase 4)

A scan yields chunks; filters, projections, aggregates and joins against a small side stream through them, so a 20 GB CSV can be filtered and grouped in a few hundred MB. Sorting more than memory spills sorted runs to temporary files (merged at the end). `df.SaveToCSV` / Parquet sinks stream too. On the desktop files are memory-mapped where the OS allows; on the web a `File` is read in slices (`Blob.slice`).

### 2.7 Expressions

Conditions and computed columns are **BASIC expressions over column names** — `Age > 30 AND City = "NYC"`, `Price * Qty`, `UCASE$(Name)`, `FORMAT$(Date, "yyyy-mm")` — parsed by RapidR's own parser and compiled to the engine's expression tree, evaluated a chunk at a time over typed columns. No second language to learn and no injection: the text is parsed into a tree, never evaluated as code. (Phase 3; phase 1's `Filter` keeps its `column, op, value` and `"column op value"` forms.)

### 2.8 SQL over frames (phase 3)

`PRINT df.SQL("SELECT city, AVG(age) FROM df GROUP BY city")` and, across frames, `R.SQL("SELECT … FROM people JOIN depts USING (dept)")` where every RDATAFRAME, RCSV … is a table by its name. A SQL subset (SELECT / FROM / JOIN / WHERE / GROUP BY / HAVING / ORDER BY / LIMIT, the common functions) parsed by `sqlparser` (Apache-2.0) into the same logical plan. The result is a frame (in `df.SQL` the frame itself becomes it, as every other method).

## 3. Sources as linked components

### 3.1 One interface

Every source is a non-visual component (it sits in the designer's tray) with the same shape:

| Member | Meaning |
|---|---|
| `FileName` / `Url` (files) or `Connection` + `SQL` / `TableName` (databases) | where the data is |
| options (`Delimiter`, `HasHeader`, `Sheet`, `Encoding` …) | how to read it |
| `RowCount`, `ColCount`, `Columns` (read) | its table's shape |
| `Load` (read it again), `Save([FileName])` (write its table) | read and write |
| `OnLoad`, `OnError(Message)` | events |

A source *is* a data set: `Grid.DataSource = Sales` shows it, `DF.Source = Sales` gives a frame its rows (a copy that costs nothing until changed: §2.1), and every source writes (`Save`, or `Post` for databases). Inside, a source implements one Rust trait (`schema`, `scan(columns, predicate, limit) → chunks`, `write(frame)`, `preview(limit)`), so the engine's pushdowns, the designer's previews and the AI tools treat them all alike.

Loading is lazy: setting `FileName` (and `Delimiter` …) marks the source stale, and it reads once, when its table is first needed — a CREATE block that sets three properties reads the file once.

### 3.2 The components

| Component | Reads / writes | Crate (licence) | Desktop | Web | Phase |
|---|---|---|:-:|:-:|:-:|
| **RCSV** | CSV / TSV (any delimiter, quotes, header or not) | our own parser | ✓ | ✓ (the page's files, HTTP) | **1** |
| **RJSON** (existing, now also a source) | records `[{…}]`, JSON Lines, columns `{"a": […]}` | our own reader; serde_json (MIT / Apache-2.0, in the tree) | ✓ | ✓ | **1** |
| RExcel | .xlsx / .xls / .ods read; .xlsx write | calamine (MIT) / rust_xlsxwriter (MIT OR Apache-2.0) | ✓ | ✓ | 2 |
| RParquet | Parquet read / write | `parquet` (Apache-2.0) + codecs (§8) | ✓ | ✓ (bigger module, loaded on use) | 2 |
| RArrow | Arrow IPC / Feather v2 read / write | `arrow-ipc` (Apache-2.0) | ✓ | ✓ | 2 |
| RAvro | Avro read / write | `apache-avro` (Apache-2.0) | ✓ | ✓ | 5 |
| RDeltaLake | Delta tables read (write later) | `delta_kernel` (Apache-2.0); not `deltalake` (pulls DataFusion + tokio) | ✓ | — (desktop only: many files, object stores) | 5 |
| RSQLite | SQLite (existing RSQLITE, now also a source) | rusqlite (MIT) + SQLite (public domain), in the tree | ✓ | ✓ (wasm SQLite, in the tree) | 2 |
| RMySQL | MySQL / MariaDB (existing RMYSQL, now also a source) | `mysql` (MIT OR Apache-2.0), in the tree | ✓ | ✗ desktop only | 2 |
| RPostgres | PostgreSQL and Amazon Redshift | `postgres` (MIT OR Apache-2.0) + rustls (§8) | ✓ | ✗ desktop only | 5 |
| RSQLServer | Microsoft SQL Server / Azure SQL | `tiberius` (MIT OR Apache-2.0) + rustls | ✓ | ✗ desktop only | 5 |
| RS3, RAzureBlob, RAzureFile | cloud files (any of the formats above inside) | `object_store` (Apache-2.0 OR MIT) for S3 / Blob; Azure Files over its REST API (our own small client) | ✓ | ✓ over HTTPS (CORS and a presigned / SAS URL) | 6 |
| RDBConnection + RDBQuery / RDBTable | ide-plan's dataset model for SQL sources | rapidr-db | ✓ | SQLite only | 2 |
| Oracle | — | `oracle` needs Oracle's own client (Instant Client, Oracle's licence) | **not planned**: a legal and dependency review first | — | — |

Every new crate goes through `cargo deny check licenses bans` with this repository's `deny.toml` (permissive only: MIT, Apache-2.0, BSD, ISC, Zlib, Unicode, BSL-1.0, Unlicense; nothing GPL, LGPL, AGPL, or MPL) and THIRD_PARTY_NOTICES.md is regenerated. §8 has the table with versions.

### 3.3 Passwords and secrets

A database source's `Password` is never written into source by the designer: at design time it comes from the OS keychain (`rapidr-secrets`, I8), at run time from the program (`DB.Password = …`, an environment variable, the keychain through RKeychain). Cloud credentials the same way; on the web, presigned / SAS URLs (no long-lived keys in a page).

## 4. Data-aware components

### 4.1 The model: data sets, cursors, links

- A **data set** is anything that holds a table: an RDATAFRAME, an RCSV, an RJSON used as a source, an RSQLITE's last query, later every source and transform. Its table is a frame in the engine's store, under its name.
- A **cursor** is a current row. `RDataSource` owns one; a control bound straight to a data set (no RDataSource) uses that data set's own default cursor — so `Grid1.DataSource = People` and `Edit1.DataSource = People` move together with nothing else to set (the easy way), and an RDataSource gives a second, independent cursor, events and `Enabled` (the full way).
- **Links** are properties holding a component's name: `DF.Source`, `DS.DataSet`, `Grid.DataSource`, `Edit.DataSource` + `Edit.DataField`. The inspector edits them with a picker of the components on the form that fit (the registry's `kinds`).

All of it is one module, `rapidr_value::datascience::link`, shared by every runtime and by the designer. It keeps the links, the cursors and what every bound control last showed (the data set's version and row); after anything that may change data — a method or property of a data component, a user's edit, a click in a grid — `link::sync` works out which bound controls are stale and hands the runtime a short list of updates (`Edit1.Text = "Lee"`, "draw Grid1 again"), which the runtime applies with its own `rp_comp_set` / redraw. Frames carry a version that changes whenever they do, so `sync` is a few comparisons when nothing changed.

### 4.2 RDataSource

| | |
|---|---|
| Properties | `DataSet` (a data set), `Row` (the current row, from 0; set it to move), `RecordCount`, `EOF`, `BOF`, `Enabled` (off: the bound controls show nothing and don't follow), `State` (`"inactive"`, `"browse"`) |
| Methods | `First`, `Prior`, `Next`, `Last`, `Field(Name)` (the current row's value), `SetField(Name, Value)`, `Refresh` |
| Events | `OnDataChange` (the row moved, or the data changed), `OnStateChange` |

Phase 5 adds Delphi's editing states (`Edit`, `Insert`, `Post`, `Cancel`, `Delete`, `State = "edit" / "insert"`) for database sources that post back by primary key (rapidr-db's dataset model, ide-plan §I7), and `MasterSource` + `MasterFields` / `DetailFields` for master–detail (the detail's rows are those matching the master's current row, re-filtered as the master moves).

### 4.3 Bound controls: DataSource + DataField

`DataSource` (an RDataSource or any data set) and `DataField` (a column's name) on:

| Control | Shows | Edits back | Phase |
|---|---|---|:-:|
| REdit | `Text` = the field | the user's change (each OnChange) | **1** |
| RLabel | `Caption` = the field | — | **1** |
| RCheckBox | `Checked` = `true` / `1` / `yes` | a click | 2 |
| RComboBox | `Text` = the field (`ListSource` / `ListField` for lookups, phase 5) | a pick or typing | 2 |
| RDateTime (new, phase 3) | the date | a pick | 3 |
| RMemo, RRichEdit | `Text` | typing | 2 |
| RImage | a picture column (bytes or a file name) | — | 5 |

A control writes back into the data set's frame (in memory); the data set's own `Save` (or a database's `Post`) writes it out. Programs that never set these properties are untouched; setting them is RapidR's own extension on RapidQ's components (origin `rapidr` in the registry).

### 4.4 RDBGrid

A grid drawn by the UI kernel straight from a frame — **virtual rows**: it draws only the rows on screen, so a million rows scroll at the frame rate and cost no memory beyond the frame.

| | |
|---|---|
| Properties | `DataSource` (an RDataSource or a data set), `ReadOnly` (default True in phase 1), `Row` (the current row of the data set), `TopRow`, `RowCount`, `ColCount`, `Columns` (read), `SortColumn` / `SortDescending` (read) |
| Events | `OnCellClick(Row, Col)`, `OnTitleClick(Col)` (OnClick, OnDblClick, OnKeyDown … as any component) |
| The user | a click selects a row (moves the cursor: the bound edits follow), the arrows / Page Up / Down / Home / End move it, the wheel and the scroll bar scroll; **a click on a heading sorts the grid's view** by that column (numbers as numbers, nulls last), again descending, a third time back to the data's order — the data itself stays as it is; numbers right-aligned, a missing value shown dimmed as `null` |
| Screen readers | a table: the headings as column headers, rows read as "Row 12 of 1,000,000: name Lee, age 41, city NYC"; the current row selected |

Phase 2 adds editing in place (Enter / F2 / typing, Esc), column widths by dragging, a filter row; phase 5 the Columns editor (titles, widths, formats, visible), lookups.

### 4.5 RDBNavigator and RPlot bound to columns (phase 2)

- **RDBNavigator** — First / Prior / Next / Last / Insert / Delete / Edit / Post / Cancel / Refresh buttons over a `DataSource`, each enabled by the data set's state (`VisibleButtons`).
- **RPlot** gains `DataSource` (or a data set), `Kind` (line, bar, barh, scatter, area, step, hist, pie), `X`, `Y` (one or several columns), `ColorBy` / `Series`, `Stacked` — drawn live as the data changes, its imperative methods unchanged. At design time it shows the real data (today: sample bars).

### 4.6 One binding for databases too (phase 2)

The web's RSQLITE binding becomes this one: an RSQLITE (later every SQL source) is a data set whose rows are its last query's, its cursor the row FetchRow is on; `rapidr-db` answers the link's questions (columns, the current row, writing a field back by the result's `id`) through a hook every runtime installs. The web's own `rp_sync_bound_widgets` / `bound_input` go, and the desktop gets the binding (parity).

## 5. The BASIC API

**Naming rules.** RapidR's own names (`RCSV`, `RDataSource`, `RDBGrid`), PascalCase members, verbs for methods (`Load`, `Save`, `Filter`, `Sort`, `GroupBy`, `First`, `Next`), nouns for properties (`FileName`, `DataSource`, `DataField`, `Row`, `RowCount`); one obvious name each (aliases only where pandas / Polars users would type another name first, as today's `Sort_values`); rows and columns counted from 0 (as `Cell(row, col)`); booleans as True / False; text in, text out (a cell reads as text; numbers convert as BASIC converts them).

**Phase 1** (built, §12):

```basic
CREATE Form AS RFORM
    ' (the data components: in the designer's tray, read live there)
    CREATE People AS RCSV
        FileName = "people.csv"      ' Delimiter = ",", HasHeader = True
    END CREATE
    CREATE DF AS RDATAFRAME
        Source = People              ' the frame gets People's rows
    END CREATE
    CREATE Nav AS RDATASOURCE
        DataSet = DF
    END CREATE
    CREATE Grid AS RDBGRID
        DataSource = Nav              ' or straight: DataSource = DF
        Align = alClient
    END CREATE
    CREATE NameEdit AS REDIT
        DataSource = Nav: DataField = "name"
    END CREATE
END CREATE

DF.Filter "age", ">", 30             ' the grid and the edit follow
Nav.Next                             ' so does the current row
PRINT Nav.Field("name"); Nav.Row; Nav.RecordCount
DF.Refresh                           ' read People again
People.Save "people2.csv"
```

**Later phases** (§11): `df.Filter "Age > 30 AND City = ""NYC"""` (expressions), `df.GroupBy "City", "Sales:sum, Qty:mean"`, `df.Join other, "id", "left"`, `df.Pivot …` / `Melt …`, `df.Rolling …`, window functions (`Rank`, `Shift`, `CumSum` over groups), dates (`Year(Date)`), strings (`Contains`, `Replace`, `Split`), `df.SQL(…)`, `df.Lazy` … `df.Collect` for programs that want to build a plan by hand, `df.Explain`.

**Errors** are plain English and say what to do, through each runtime's warning channel (stderr / the console) and `OnError` where the component has one; nothing a kid sees is a Rust panic or a stack trace.

## 6. Live in the IDE

- **Design time.** The designer builds the designed form's data components itself, under the surface's own names (`surface:People`, never the program's), the same model the program runs: a source's file is read (inside the project only — a path outside it, or a URL, needs the user's yes once per project; read-only; databases opened read-only, never DML), frames are copied from their sources, the grid / edits / labels show the data. Results are cached by (file, size, modification time, options) so a property change re-draws without re-reading; a file changed on disk is read again (phase 2: a watcher; phase 1: when the design is rebuilt). Phase 1 caps a design-time read at 1,000,000 rows (the whole of a normal file; RDBGrid draws only what shows).
- **RDataPreview** (phase 2) — the public panel: schema (column, type, nulls, distinct), rows (virtual grid, sort / filter locally), quick stats (count, mean, median, std, min, max, top values, a histogram sparkline per column), the plan that produced it; on any source / transform selected in the designer, and on any frame while debugging.
- **Pickers** — `DataSource`, `DataSet`, `Source` are component properties (`type = "component"`, `kinds = […]`): the inspector's drop-down lists the fitting components on the form (exists today). `DataField` drops the bound data set's columns (built in phase 1: the inspector asks the link module, which has what the designer read).
- **Completion and checking** (phase 3, with I3): column names complete inside `DataField`, `Filter`, `Sort`, expressions and `SQL`; an unknown column is a squiggle.
- **Debugger** (phase 3, with I6): a frame variable shows as a table in data tips and Variables (first rows, shape, types); the Immediate window evaluates `df.Filter …` on a copy.

## 7. The web

What a browser can do, the web runtime does, with the same code:

- **Files**: the page's files (the project's, uploaded, saved by the program), read and written by every file source; big files in slices.
- **HTTP(S)**: `FileName = "https://…"` (CORS permitting) for CSV, JSON, Parquet, Excel …; cloud storage over HTTPS with presigned / SAS URLs.
- **SQLite**: the wasm SQLite already in the runtime.
- **Not possible in a browser: direct database connections** (MySQL, PostgreSQL, Redshift, SQL Server need raw TCP). Those components exist on the web so programs compile and run, and say so once, clearly: `RPostgres: a web page can't connect to a database server directly — run this program on the desktop, or reach the database through a web service (RHTTP).` (today's RMYSQL message, in the same words for each). Design time on the web the same.
- **Threads**: none yet (the kernels run on the calling thread; a million-row load is ~2× the desktop's single-thread time). Phase 6: wasm threads where the page is cross-origin isolated (COOP / COEP), the same kernels.
- **Memory**: a wasm32 page has at most 4 GB; frames over ~1 GB say so (`frame too big for a web page: N rows`) instead of failing obscurely; streaming (phase 4) keeps scans small.
- **Security**: the page's strict CSP stays (no `eval`, no inline scripts); a URL source fetches only what the program names, with the browser's CORS rules.

## 8. Licences

Policy: permissive only (MIT, Apache-2.0, BSD-2/3, ISC, Zlib, Unicode-3.0, BSL-1.0, Unlicense), checked per crate with `cargo deny check licenses bans` (`deny.toml`) before a connector lands, THIRD_PARTY_NOTICES.md regenerated (tools/third_party_notices.py). Nothing copyleft (GPL, LGPL, AGPL, MPL), compiled in or linked, ever.

### 8.1 Crates per connector

Checked on crates.io and the crates' repositories, 2026-10-15 (the latest stable versions then). "Web" says whether the crate can build for `wasm32-unknown-unknown` at all; each still needs a build test before it lands.

| Use | Crate (version) | Licence | Web | C / system code | Decision |
|---|---|---|:-:|---|---|
| CSV, JSON | ours; serde_json (in the tree) | — ; MIT OR Apache-2.0 | ✓ | none | phase 1 |
| SQLite | rusqlite 0.40 (in the tree) + SQLite | MIT; SQLite public domain | ✓ (sqlite-wasm-rs) | SQLite's C, bundled | in use |
| MySQL / MariaDB | mysql 28 (in the tree) | MIT OR Apache-2.0 | ✗ (TCP) | none (no TLS feature) | in use, desktop |
| Excel read | calamine 0.36 | MIT | likely (pure Rust) | none | phase 2 |
| Excel write | rust_xlsxwriter 0.99 | MIT OR Apache-2.0 | likely (pure Rust) | none | phase 2 |
| Parquet | parquet 60 (no default features) | Apache-2.0 | ✓ without `zstd` | the `zstd` codec is C (zstd-sys) | phase 2: snap, lz4_flex, brotli, flate2 (pure Rust) everywhere; zstd desktop-only or a pure-Rust decoder (`ruzstd`, MIT) — to measure |
| Arrow IPC | arrow-ipc 60, arrow-array 60 | Apache-2.0 (arrow-array: Apache-2.0 AND MIT) | likely | none by default | phase 2 |
| Avro | apache-avro 0.22 (default features) | Apache-2.0 | likely | none by default (bzip2 / xz / zstd are opt-in C) | phase 5 |
| Delta Lake | delta_kernel 0.29 | Apache-2.0 | core only | build-time protoc (vendored) | phase 5, desktop. **Not** `deltalake` 1.1: tokio + object_store + its own fork of the kernel (buoyant_kernel) on a different Arrow major — heavy and out of step |
| PostgreSQL / Redshift | postgres 0.19 (tokio-postgres 0.7) | MIT OR Apache-2.0 | ✗ (TCP) | none; TLS separate | phase 5, desktop |
| SQL Server | tiberius 0.13, **without** default `native-tls` (rustls instead) | MIT OR Apache-2.0 | ✗ (TCP) | default native-tls = OpenSSL on Linux | phase 5, desktop |
| TLS for the above | rustls 0.23 with the **ring** provider; webpki-roots 1.0 | Apache-2.0 OR ISC OR MIT; ring: Apache-2.0 AND ISC; webpki-roots: CDLA-Permissive-2.0 (add to deny.toml's allow-list after review) | — | ring's C / asm via cc; avoid aws-lc-rs (cmake, no wasm) | phase 5 |
| S3, Azure Blob | object_store 0.14 (cloud features) | MIT OR Apache-2.0 | partly (cloud features not on wasm: the web uses presigned / SAS URLs over fetch) | ring | phase 6, desktop; the web over HTTPS |
| Azure Files | no official Rust crate — our own small REST client over the HTTP stack already in the tree | — | ✓ (fetch) | none | phase 6 |
| SQL over frames | sqlparser 0.63 | Apache-2.0 | ✓ | none | phase 3 |
| Fast byte search | memchr 2.8 (in the tree) | Unlicense OR MIT | ✓ (SIMD on wasm too) | none | phase 1 if a measurement asks |
| Oracle | oracle 0.6 (UPL-1.0 OR Apache-2.0) + **Oracle Instant Client** (Oracle's proprietary OTN licence; distributors must bind users to it) | — | ✗ | ODPI-C + Oracle's client | **not planned** — flagged for legal review; never bundled |

Notes: no copyleft code in any mandatory dependency above. zstd's C library is dual-licensed (BSD-3-Clause, or GPL-2.0 at the user's choice; RapidR takes BSD-3 — if it's ever compiled in, THIRD_PARTY_NOTICES.md says which). Default features are chosen deliberately: several crates' defaults pull C code (parquet → zstd, tiberius → OpenSSL, mysql_async → zlib, azure crates → aws-lc).

## 9. Performance budgets and the benchmark suite

### 9.1 Budgets (this Mac: M4 Max, 16 cores; release build)

| 1,000,000 rows (5 columns, 37 MB CSV) | Phase 1 budget | Polars-class goal (phase 3) |
|---|--:|--:|
| load CSV (from memory) | 120 ms | 40 ms |
| filter (numeric) | 25 ms | 5 ms |
| sort (numeric, descending) | 120 ms | 30 ms |
| group-by (text key, mean of 3 columns) | 60 ms | 15 ms |
| left join (1 M × 5) | 60 ms | 20 ms |
| describe | 60 ms | 10 ms |
| save CSV | 100 ms | 40 ms |
| **load + filter + sort + group-by** | **< 1 s** (hard), target 300 ms | 100 ms |
| 10 M rows, the same pipeline | < 5 s | 1 s |
| the web (Chromium, one thread) | ≤ 3 × the desktop's | ≤ 2 × |
| RDBGrid, 1 M rows: first paint / scroll a page | < 16 ms / < 8 ms | |
| designer: a 100,000-row CSV linked to a grid, shown | < 300 ms | |

Budgets are enforced by `tools/bench/frame_bench` (exit code 1 when a budget is missed by more than 25 %, the noise margin), run by `tools/regress.sh perf`.

### 9.2 The suite (tools/bench/)

- `tools/bench/README.md` — what each benchmark measures and how to run it.
- `frame_bench` (Rust, `cargo run --release -p rapidr-value --example frame_bench [rows]`) — the engine alone: the operations above on the same generated data, with budgets.
- `bench_rapidr.rr` — the same pipeline as a RapidR program (`rapidr run` interpreted, `rapidr build` native), so the numbers include the VM and the runtime.
- `bench_pandas.py`, `bench_polars.py` — the same data and operations in pandas and Polars, run only where those are installed (`python3 tools/bench/bench_pandas.py`; never dependencies of RapidR, never shipped); `tools/bench/compare.sh` runs everything available and prints one table.
- `make_data.py` / the Rust generator — the same deterministic data for all (a CSV written once to `tests/results/bench/`).

## 10. The language registry: IntelliSense, MCP, AI

Every component here has a complete registry entry (`crates/rapidr-lang/data/components/datascience.toml`): `kind = "component"`, `visual`, `size`, the group "Data science", a doc in plain words, every property with its type (`component` + `kinds` for links, `editor = "file"` for `FileName`), default, access and design flags, every method with its parameters, return and doc, every event with its parameters. Properties added to REdit / RLabel carry `origin = "rapidr"`. From it come the compilers' component list, the IDE's completion and help, the inspector's editors and pickers, the manual's reference pages and the AI system prompt (docs/ide-ai.md) — no hand-written list anywhere else.

For MCP and RAI (I8), the data components also answer **one introspection interface** the registry describes: `describe` (its properties and links), `columns` (name, type, nulls), `preview(n)` (the first rows as JSON), `stats` (describe), and `call(member, args)` — so an agent can inspect a source's schema, propose RCSV → RDataFrame → RDataSource → RDBGrid / RPlot as CREATE blocks, apply them, and see the data in the designer; the per-project data access level (none / schema / sample / full) is enforced in that layer (docs/ide-ai.md §6).

## 11. Phases and sizes

Sizes in sessions (one session ≈ one lane-day).

| Phase | What | Size |
|---|---|--:|
| **1 — the slice** (lane DATA-ARCH, this document) | this plan; the engine core: typed, columnar, multithreaded storage behind RDataFrame (load 1 M rows, filter / sort / group-by < 1 s, measured); RCSV and RJSON as sources, `RDataFrame.Source`; RDataSource; `DataSource` + `DataField` on REdit and RLabel; RDBGrid fed straight from a frame; live in Studio's designer; registry entries; benchmarks (tools/bench/) against pandas / Polars; examples and the manual's "Data science with RapidR" chapter with screenshots | 4–6 |
| 2 — linked everywhere | RExcel, RParquet, RArrow; RSQLite / RMySQL as sources and §4.6's one binding; RCheckBox / RComboBox / RMemo bound; RDBGrid editing and sizing; RDBNavigator; RPlot bound to columns (live at design time); RDataPreview; DataField's column picker | 6–8 |
| 3 — the toolkit | expressions; select / filter / sort / group-by with many aggregates; joins (inner / left / outer / cross / asof); pivot / melt; window, rolling and cumulative functions; dates and times; strings; missing values; lazy plans with the optimizer; SQL over frames; RDF transform components; completion in expressions; frames in the debugger | 8–10 |
| 4 — big data | chunked columns, streaming scans and sinks, out-of-core sort, memory-mapped files; 10 M-row and 1 B-row (streamed) benchmarks | 4–5 |
| 5 — databases and editing | RPostgres (+ Redshift), RSQLServer, RAvro, RDeltaLake (read); editing states, Post / Cancel, master–detail, lookups, RDateTime | 6–8 |
| 6 — cloud and web threads | RS3, RAzureBlob, RAzureFile; wasm threads; the data studio example | 4–5 |
| 7 — schools | guided lessons in Studio, a "data" project template, plain-English error audit, the manual chapter completed; the six legal checks before anything that stores student data or accounts | 3–4 |
| **Total** | | **35–46** |

## 12. Phase 1 — what was built (2026-10-15)

**Linked data** (`rapidr_value::datascience::link`, one model for native builds, the interpreter, the web and the designer):

- **RCSV** — FileName, Delimiter, HasHeader, Text; RowCount, ColCount, Columns, Error; Load, Save, Clear; OnLoad, OnError. Read lazily (once when first needed); a relative file is found in the current folder, then beside the program.
- **RJson's records as a source**; its FileName or document read again when it changes.
- **RDataFrame.Source** (RCSV, RJson, RDataFrame) — a copy pulled at once and again when the source's version changes; `Refresh`. Cycles and self-references are ignored (depth limit 16).
- **RDataSource** — DataSet, Row, RecordCount, EOF / BOF (BASIC's True, -1, so `NOT` works), Enabled, State; First, Prior, Next, Last, MoveBy, Field, SetField, Refresh; OnDataChange (not on the first look: a form isn't made yet then).
- **DataSource + DataField on REdit (Text) and RLabel (Caption)** — a data set straight (its own cursor) or an RDataSource; typing in a bound REdit writes back (no echo loop: a value equal to what the control shows is never set again).
- **RDBGrid** (UI kernel, `components/dbgrid.rs`) — virtual rows; headings, row numbers, numbers right-aligned, `null` dimmed; current row by click / keys; a heading click sorts the **view** (a permutation cached per data version and sort; the data and every other bound control untouched); wheel, bars, held-arrow repeat; OnCellClick, OnTitleClick; an accessible grid (headings row, the rows on screen as "Row r of n" with their values, the current one selected).
- **The runtimes**: after every `rp_comp_set` / `rp_comp_method` (a flag test when nothing changed) and every kernel event (`rapidr-ui-app`'s dispatch), `link::sync()`'s updates are applied with the runtime's own setter, redraw and events — `links.rs` (desktop) and `links_web.rs` (web), 50 lines each.
- **The designer** (`link::design_build`, called by the kernel's designer view and by the inspector): the designed form's RCSV / RJson / RDataFrame / RDataSource are made under `surface:name` (a program's sync never touches them, nor the designer's a program's) and read from the designed program's folder only (`..`, absolute paths outside it and URLs refused; nothing written; up to 1,000,000 rows; parsed results cached by path, options, length and a content hash), so grids, edits and labels show the data at design time — desktop and web. The inspector's DataField drops the bound data set's columns.
- **Registry**: RCSV, RDATASOURCE, RDBGRID; Source / Refresh on RDATAFRAME; DataSource / DataField on REDIT / RLABEL (origin rapidr); value-method rules (`IF People.Load`); `lang_dispatch.py` sites. RCSV in the toolbox's Data group with the data file's icon.

**Verified**: unit tests (link: 8 tests; kernel: a million-row grid draws < 200 texts; design time draws linked data; the whole workspace); the `inspector_datafield` GUI fixture (DataField's list, a column picked — native, interpreted and the web alike); the desktop GUI events (every case), the visual gallery (37 cases match their goldens), RapidR Studio's flows on both hosts (one pre-existing flaky case, `debug-pause-waiting`, fails about one run in three with the base commit's binary too); `datascience_link` conformance case on vm, codegen and the web; `lang_conformance` for RCSV / RDATASOURCE / RDBGRID / RDATAFRAME / REDIT / RLABEL / RJSON on vm and web; the `rdbgrid` GUI fixture — native and interpreted agree, the web's window byte-identical to the desktop's and the accessibility trees equal; the three new examples on run, interp, native and web (`tests/examples_run.mjs`); RapidR Studio building a linked form with its own tools (the manual's `linked-build` scene) and showing `linked_data.rr` / `json_source.rr` live, desktop and web.

**Engine** (lane DATA-ENGINE, `frame.rs` + `frame/{column,csv,json,kernels,par}.rs`): typed columns (i64 / f64 / bool / str) with validity bitmaps and `Arc` buffers, kept typed only when every cell's text is its value's own (so nothing a program reads changes), CSV parsed in parallel chunks, parallel filter / gather / sort (merged runs) / group-by / join / describe on `std::thread::scope` (sequential under ~50,000 rows and always on wasm32), no new dependency. Identical output proven by a differential test against the previous engine (`tests/frame_oracle.rs`: tens of thousands of random programs, parallel paths forced; 25,000 + 2,000 + 20 big tables in the deep run).

| 1,000,000 rows, this Mac (M4 Max, 16 cores; load average 20–30 from other lanes) | before | after, all cores | after, 1 thread | pandas 3.0.1 | budget |
|---|--:|--:|--:|--:|--:|
| load CSV | 100.2 ms | 11.8 | 69.9 | 287.6 | 120 |
| filter | 39.3 | 2.9 | 12.6 | 10.0 | 25 |
| sort (descending) | 203.0 | 10.2 | 51.6 | 149.2 | 120 |
| group-by (mean) | 83.5 | 4.8 | 19.0 | 22.1 | 60 |
| left join | 62.6 | 3.4 | 20.1 | 50.4 | 60 |
| describe | 85.5 | 10.1 | 28.2 | 28.5 | 60 |
| save CSV | 82.3 | 7.3 | 54.4 | 615.5 | 100 |
| **load + filter + sort + group-by** | **266.9** | **22.7** | **116.2** | **371.6** | **< 1 s** |
| the same, 10,000,000 rows | 4,394.5 | 273.1 | 1,571.7 | 5,793 | < 5 s |

Through a RapidR program (`tools/bench/bench_rapidr.rr`, 1 M rows, the file read from disk): pipeline 24.5 ms interpreted, 25.3 ms native. In `examples/data/million_rows.rr`: read 13 ms, filter 2 ms, sort 11 ms, group 5 ms. **On the web** (Chromium on the same Mac, wasm, one thread): read 145 ms, filter 10 ms, sort 61 ms, group 33 ms. Polars isn't installed here (`bench_polars.py` says so and exits). Every phase-1 budget is met on all cores and on one thread; most beat the phase-3 "Polars-class" goals already. Left for later phases: explicit SIMD (the kernels rely on auto-vectorization), chunked columns, a parallel `to_json`, lazy plans.

## 13. Open questions for Robert

1. **Bound controls write back on every keystroke** (simple, immediate). Delphi posts when the user leaves the field; the dataset editing states of phase 5 (Edit / Post / Cancel) would make that the rule for database sources. Keep per-keystroke for in-memory frames?
2. **`RDataFrame.Source` re-pulls when the source changes**, discarding what the program did to the frame (Filter, Sort …). That keeps "the frame shows the file" true for a child; phase 3's transform components (RDFFilter …) are the way to keep a pipeline. Agreed?
3. **Data components outside the form** (top-level `CREATE People AS RCSV`) run fine but the designer reads only the form's own (the tray's) — as Delphi's form designer. Should top-level data components show at design time too?
4. **Oracle**: not planned (its client's licence) unless a legal review says otherwise.
5. **Polars isn't installed on this Mac** (pandas 3.0.1 is): `pip install polars` (a download — Robert's call) and `tools/bench/compare.sh` adds its column.

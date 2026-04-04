# RapidR — BASIC to Native Rust Transpiler

[![Rust](https://img.shields.io/badge/Rust-2021-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)

**RapidR** is an experiment. The idea is to implement a BASIC-to-Rust transpiler and native runtime that compiles `.rr` source files into standalone Rust projects, producing fast, native executables. At the current stage, it provides **49+ GUI components** (P-prefixed: `RForm`, `RButton`, `RStringGrid`, …), **100+ built-in functions**, database access (MySQL + SQLite), networking, data science components, and a self-hosted **Visual IDE** — all compiled to native code via FLTK.

> **Note:** RapidR is *inspired by* and *aims for basic compatibility with* the original RapidQ BASIC language, but it is **not** a clone or drop-in replacement. RapidR extends the language with data science components (RNum, RPlot, RDataFrame), enhanced networking, and modern tooling while preserving as much backward compatibility as practical.

---

## Table of Contents

- [System Architecture](#system-architecture)
- [RapidR Status](#rapidr-status)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [The Compiler Engine](#the-compiler-engine)
- [Preprocessor Directives](#preprocessor-directives)
- [The Runtime Library](#the-runtime-library)
  - [GUI Components](#gui-components)
  - [Built-in Functions](#built-in-functions)
  - [Database](#database)
  - [Networking](#networking)
  - [Data Science Components](#data-science-components-rnum-rplot-rdataframe)
- [The Self-Hosted IDE](#the-self-hosted-ide)
- [VS Code Extension](#vs-code-extension)
- [Syntax Reference](#syntax-reference)
- [Test Suite](#test-suite)
- [Development Conventions](#development-conventions)
- [Credits](#credits)
- [License](#license)

---

## System Architecture

The Rust workspace under `crates/` provides a full transpilation pipeline that generates standalone Rust projects from `.rr` source files:

1. **Compiler Frontend (`crates/rapidr-{lexer,parser,preprocessor,ast,diagnostics}/`)** — Lexes, preprocesses, and parses BASIC syntax into an AST.
2. **Code Generator (`crates/rapidr-codegen-rust/`)** — Walks the AST and emits Rust source code targeting the native runtime.
3. **Native Runtime (`crates/rapidr-runtime-core/`)** — FLTK-based GUI, built-in functions, database (MySQL + SQLite), networking, data science, and file I/O.
4. **CLI (`crates/rapidr-cli/`)** — Command-line interface with `codegen` command to generate Rust projects from `.rr` files.

## Status

RapidR has reached **functional transpiler status**. The Rust workspace provides a complete pipeline from `.rr` source through parsing, Rust code generation, and compilation to native executables. All 29 example programs compile and run, including the self-hosted IDE.

### Crate Architecture (8 crates)

| Crate | Description |
|-------|-------------|
| `rapidr-cli` | Command-line entry point — `version`, `preprocess`, `lex`, `parse`, `codegen` commands |
| `rapidr-diagnostics` | `TextSpan`, `SourceLocation`, and `Diagnostic` types |
| `rapidr-ast` | Shared AST data structures (program, statement, declaration, expression nodes) |
| `rapidr-preprocessor` | Directive handling (`$DEFINE`, `$IFDEF`, `$INCLUDE`, `$MACRO`, `$THEME`, etc.) |
| `rapidr-lexer` | Lexical analysis — keywords, literals, directives, operators, suffixes |
| `rapidr-parser` | Recursive-descent parser producing typed AST from token stream |
| `rapidr-codegen-rust` | **Rust code generator** — walks AST, emits Rust source targeting `rapidr-runtime-core` (~2,100 lines) |
| `rapidr-runtime-core` | **Native runtime** — GUI (FLTK), builtins, database (MySQL/SQLite), networking, data science (ndarray/polars/plotters), file I/O |

### Key Capabilities

- **Native GUI via FLTK** — Forms, buttons, labels, edits, panels, tabs, string grids, combo boxes, code editors, design surfaces, MDI, splitters, scroll boxes, and more
- **FLTK Themes** — `$THEME` directive supports: Classic, Aero, Metro, AquaClassic, Greybird, Blue, Dark, HighContrast; also `$THEME AUTO` for OS-based selection
- **Global variable mechanism** — Module-level `DIM` variables use thread-local storage (`gv()`/`gs()` accessors), correctly shared across all SUBs/FUNCTIONs
- **User-Defined Types** — `TYPE...END TYPE` with fields, inheritance, constructors, and methods
- **Database** — MySQL (via `mysql` crate) and SQLite (via `rusqlite`) with property-based API
- **Networking** — TCP sockets, server sockets, HTTP client
- **100+ built-in functions** — String, math, file I/O, system operations
- **Self-hosted IDE** — The visual form designer (`examples/ide.rr`) compiles to a native FLTK application
- **Data science** — RNum (ndarray), RDataFrame (polars), RPlot (plotters) components for array math, dataframes, and plotting
- **Raw Rust injection** — `RUSTSTART`/`RUSTEND` blocks allow inline Rust code in `.rr` sources

### Validation

```bash
# Run all 70 Rust unit tests
cargo test

# Generate and build an example
cargo run -- codegen examples/hello_world.rr /tmp/hello
cd /tmp/hello && cargo build && ./target/debug/hello_world

# Generate and run the IDE
cargo run -- codegen examples/ide.rr /tmp/ide_rust
cd /tmp/ide_rust && cargo build && ./target/debug/ide
```

---

## Installation

### Prerequisites

- Rust toolchain (edition 2021+) — install via [rustup](https://rustup.rs/)
- A C/C++ compiler (for FLTK compilation) — Xcode Command Line Tools on macOS, `build-essential` on Linux, MSVC on Windows

### Building

```bash
git clone https://github.com/iBobX/RapidP-BASIC.git
cd RapidP-BASIC
cargo build --release
```

---

## Quick Start

### Hello World

Create `hello.rr`:
```basic
PRINT "Hello, World!"
```

Generate a Rust project, build, and run:
```bash
cargo run -- codegen hello.rr /tmp/hello
cd /tmp/hello && cargo build
./target/debug/hello
```

### Launch the IDE

```bash
cargo run -- codegen examples/ide.rr /tmp/ide_rust
cd /tmp/ide_rust && cargo build && ./target/debug/ide
```

### CLI Options

```
cargo run -- <command> [options]
```

| Command | Description |
|---------|-------------|
| `codegen <file.rr> <outdir>` | Generate a Rust project from a `.rr` file |
| `lex <file.rr>` | Dump token stream |
| `parse <file.rr>` | Dump AST |
| `preprocess <file.rr>` | Dump preprocessed source |
| `version` | Print version |

Optional flags for `codegen`:

| Flag | Description |
|------|-------------|
| `--release` | Build in release mode (optimized) |
| `--debug` | Build in debug mode (default) |

---

## The Compiler Engine

The transpilation pipeline mirrors a classic multi-pass compiler:

| Stage | Crate | Description |
|-------|-------|-------------|
| Preprocessing | `rapidr-preprocessor` | Handles `$INCLUDE`, `$DEFINE`, `$IFDEF`/`$IFNDEF`, `$MACRO`, `$APPTYPE`, `$THEME`, and other directives |
| Lexical Analysis | `rapidr-lexer` | Tokenizes the source into keywords, identifiers, literals, operators, and directives |
| Parsing | `rapidr-parser` | Recursive-descent parser producing an AST; correctly scopes `FOR…NEXT`, `DO…LOOP`, `IF…END IF`, `SELECT CASE`, and `CREATE…END CREATE` blocks |
| AST Nodes | `rapidr-ast` | Shared AST data structures: program, statement, declaration, expression nodes |
| Code Generation | `rapidr-codegen-rust` | Walks the AST and emits Rust source code targeting `rapidr-runtime-core` |

---

## Preprocessor Directives

| Directive | Description | Example |
|-----------|-------------|---------|
| `$INCLUDE` | Include an external source file | `$INCLUDE "rapidp.inc"` |
| `$DEFINE` | Define a text substitution constant | `$DEFINE MAX_SIZE 100` |
| `$UNDEF` | Undefine a previously defined constant | `$UNDEF MAX_SIZE` |
| `$IFDEF` / `$IFNDEF` | Conditional compilation | `$IFDEF DEBUG` |
| `$ELSE` / `$ENDIF` | Conditional branches | `$ELSE` … `$ENDIF` |
| `$MACRO` | Define a simple or parameterized macro | `$MACRO SQUARE(x) = (x) * (x)` |
| `$APPTYPE` | Set application type (`GUI` or `CONSOLE`) | `$APPTYPE CONSOLE` |
| `$OPTIMIZE` | Optimization hint (pass-through) | `$OPTIMIZE ON` |
| `$ESCAPECHARS` | Enable escape character processing | `$ESCAPECHARS ON` |
| `$THEME` | Set FLTK theme (Rust only) | `$THEME AquaClassic` or `$THEME AUTO` |

**Available themes (Rust / FLTK):** `Classic`, `Aero`, `Metro`, `AquaClassic`, `Greybird`, `Blue`, `Dark`, `HighContrast`, `AUTO` (selects by OS: AquaClassic on macOS, Aero on Windows, Greybird on Linux).

---

## The Runtime Library

The runtime (`crates/rapidr-runtime-core/`) provides all the P-prefixed components and built-in functions that compiled programs link against.

### GUI Components

Powered by **FLTK** (via the `fltk` crate), the runtime provides **49+ component classes**.

#### Forms & Containers

| Component | Description |
|-----------|-------------|
| `RForm` | Top-level window |
| `RFormMDI` | MDI parent form |
| `RPanel` | Container panel |
| `RGroupBox` | Labeled group container |
| `RTabControl` | Tabbed container |
| `RSplitter` | Resizable split pane |
| `RScrollBox` | Scrollable container |

#### Input Controls

| Component | Description |
|-----------|-------------|
| `RButton` | Push button |
| `REdit` | Single-line text input |
| `RRichEdit` | Multi-line rich text editor |
| `RCodeEditor` | Code editor with syntax highlighting |
| `RCheckBox` | Checkbox toggle |
| `RRadioButton` | Radio button |
| `RComboBox` | Drop-down combo box |
| `RScrollBar` | Scroll bar |
| `RTrackBar` | Slider / track bar |

#### Display Components

| Component | Description |
|-----------|-------------|
| `RLabel` | Static text label |
| `RCanvas` | Drawing surface |
| `RImage` | Image display (supports `loadfromfile`, `loadfromplot`, `bmpwidth`/`bmpheight`) |
| `RProgressBar` | Progress indicator |
| `RLine` | Horizontal/vertical line |
| `RStatusBar` | Status bar with panels |
| `RHTML` | HTML display widget |

#### List & Grid Components

| Component | Description |
|-----------|-------------|
| `RStringGrid` | Spreadsheet-style grid |
| `RListBox` | List box |
| `RFileListBox` | File listing list box |
| `RListView` | Multi-column list view |
| `RTreeView` | Tree view with nodes |

#### Menu Components

| Component | Description |
|-----------|-------------|
| `RMainMenu` | Menu bar |
| `RMenuItem` | Menu item (supports sub-menus) |
| `RPopupMenu` | Context / popup menu |

#### Dialogs

| Component | Description |
|-----------|-------------|
| `ROpenDialog` | File open dialog |
| `RSaveDialog` | File save dialog |
| `RFileDialog` | General file dialog |
| `RColorDialog` | Color picker dialog |
| `RFontDialog` | Font picker dialog |

#### Utility Components

| Component | Description |
|-----------|-------------|
| `RTimer` | Timer with `ontimer` event |
| `RFont` | Font configuration object |
| `RIcon` | Icon resource |
| `RImageList` | Image list collection |
| `RFileStream` | File I/O stream |
| `RMemoryStream` | In-memory byte stream |
| `RStringList` | String collection |
| `RIni` | INI file reader/writer |
| `RPrinter` | Print support |
| `RMidi` | MIDI playback |
| `RDesignSurface` | Visual form designer surface (used by IDE) |

#### Event Handling

Components support event handlers:

```basic
CREATE Form1 AS RForm
  Caption = "My App"
  CREATE Button1 AS RButton
    Caption = "Click Me"
    OnClick = HandleClick
  END CREATE
END CREATE

SUB HandleClick(Sender AS RButton)
  ShowMessage "Button clicked!"
END SUB
```

---

### Built-in Functions

Over **100 built-in functions** covering string manipulation, math, file I/O, system operations, and more. These functions are implemented in `crates/rapidr-runtime-core/src/builtins.rs` and are available globally.

#### String Functions

| Function | Description |
|----------|-------------|
| `LEFT$(s, n)` | Left n characters |
| `RIGHT$(s, n)` | Right n characters |
| `MID$(s, start[, len])` | Substring (1-based) |
| `LEN(s)` | String length |
| `INSTR([start,] s, sub)` | Find substring (1-based) |
| `RINSTR(s, sub)` | Reverse find substring |
| `UCASE$(s)` / `LCASE$(s)` | Case conversion |
| `LTRIM$(s)` / `RTRIM$(s)` / `TRIM$(s)` | Whitespace trimming |
| `CHR$(n)` / `ASC(s)` | Character ↔ ASCII code |
| `SPACE$(n)` | n-space string |
| `STRING$(n, c)` | Repeat character |
| `STR$(n)` / `VAL(s)` | Number ↔ string conversion |
| `REPLACE$(s, old, new)` | String replacement |
| `INSERT$(s, pos, sub)` | Insert substring |
| `DELETE$(s, start, count)` | Delete from string |
| `REVERSE$(s)` | Reverse string |
| `FIELD$(s, delim, n)` | Extract delimited field |
| `TALLY(s, sub)` | Count occurrences |
| `FORMAT$(fmt, val)` | Formatted output |
| `CONVBASE$(num, from, to)` | Base conversion |
| `HEX$(n)` / `OCT$(n)` / `BIN$(n)` | Numeric base formatting |
| `HEXTODEC(s)` | Hex to decimal |

#### Math Functions

| Function | Description |
|----------|-------------|
| `ABS(n)` | Absolute value |
| `SGN(n)` | Sign (-1, 0, 1) |
| `SQR(n)` | Square root |
| `SIN(n)` / `COS(n)` / `TAN(n)` | Trigonometric |
| `ATN(n)` / `ASIN(n)` / `ACOS(n)` | Inverse trig |
| `EXP(n)` / `LOG(n)` | Exponential / natural log |
| `CEIL(n)` / `FLOOR(n)` | Rounding |
| `FIX(n)` / `FRAC(n)` | Integer / fractional part |
| `ROUND(n[, dec])` | Round to decimal places |
| `CINT(n)` / `CLNG(n)` | Convert to integer/long |
| `RND[(n)]` | Random — `RND` returns 0.0–1.0; `RND(n)` returns 0 to n-1 |
| `RANDOMIZE [seed]` | Seed random generator |
| `IIF(cond, true, false)` | Inline conditional |
| `RGB(r, g, b)` | Color value |

#### File I/O

| Function | Description |
|----------|-------------|
| `OPEN(file, mode, num)` | Open file for I/O |
| `CLOSE(num)` | Close file handle |
| `PRINT #num, ...` | Write to file |
| `WRITE #num, ...` | Write comma-delimited |
| `LINE INPUT(num)` | Read line from file |
| `EOF(num)` | End-of-file check |
| `LOF(num)` | Length of file (open handle) |
| `SEEK(num, pos)` | Seek in file |
| `FREEFILE` | Next available file number |
| `FILELEN(file)` | File size in bytes |
| `FILEEXISTS(file)` | Check file existence |
| `DIREXISTS(path)` | Check directory existence |
| `DIR$(pattern)` | Directory listing |
| `KILL(file)` | Delete file |
| `MKDIR(path)` / `RMDIR(path)` | Create/remove directory |
| `RENAME(old, new)` | Rename file |
| `CHDIR(path)` / `CURDIR$` | Change/get directory |

#### System & Memory

| Function | Description |
|----------|-------------|
| `SHELL(cmd)` / `SHELLWAIT(cmd)` | Execute system command |
| `ENVIRON$(var)` | Get environment variable |
| `COMMAND$` | Command-line arguments |
| `SLEEP(ms)` | Pause execution |
| `TIMER` | Seconds since midnight |
| `DATE$` / `TIME$` | Current date/time strings |
| `DOEVENTS` | Process pending GUI events |
| `SOUND(freq, dur)` | Play tone (pygame / system fallback) |
| `BEEP` | System beep |
| `PLAYWAV(file)` | Play WAV file |
| `SHOWMESSAGE(msg)` | Message box |
| `MESSAGEBOX(hwnd, text, cap, flags)` | Win-style message box |
| `PEEK(addr)` / `POKE(addr, val)` | Read/write console screen buffer |
| `LOCATE(row, col)` | Set cursor position |
| `COLOR(fg, bg)` | Set console colors |
| `CLS` | Clear screen buffer |
| `CSRLIN` / `POS(0)` | Get cursor row/column |
| `SIZEOF(var)` | Size of variable |
| `MEMCPY` / `MEMSET` / `MEMCMP` | Memory operations |
| `CODEPTR(func)` | Get function reference |

#### Array Functions

| Function | Description |
|----------|-------------|
| `LBOUND(arr)` | Lower bound (always 0) |
| `UBOUND(arr)` | Upper bound |
| `QUICKSORT(arr)` | In-place sort |
| `INITARRAY(arr, ...)` | Initialize with values |

---

### Database

#### RMySQL

Full MySQL/MariaDB client via the `mysql` crate:

```basic
DIM db AS RMySQL
db.host = "localhost"
db.user = "root"
db.password = "pass"
db.database = "mydb"
db.open
db.sql = "SELECT * FROM users"
db.query
```

#### RSQLite

SQLite database via the `rusqlite` crate (bundled):

```basic
DIM db AS RSQLite
db.database = "app.db"
db.open
db.sql = "CREATE TABLE IF NOT EXISTS items (id INTEGER PRIMARY KEY, name TEXT)"
db.execute
```

---

### Networking

#### RSocket

Full TCP client with SSL support and event-driven I/O:

```basic
DIM sock AS RSocket
sock.host = "example.com"
sock.port = 80
sock.onconnect = OnConnect
sock.ondataready = OnData
sock.open
```

| Property/Method | Description |
|----------------|-------------|
| `host`, `port` | Connection target |
| `open` / `close` | Connect / disconnect |
| `readline` / `writeline` | Line-oriented I/O |
| `usessl` | Enable SSL/TLS |
| `timeout` | Socket timeout (seconds) |
| `bind` / `listen` / `accept` | Server-side operations |
| `onconnect`, `ondisconnect`, `ondataready`, `onerror` | Event handlers |

#### RServerSocket

Threaded TCP server with per-client management:

```basic
DIM server AS RServerSocket
server.port = 9000
server.onclientconnect = OnClient
server.ondatareceived = OnData
server.start
```

| Property/Method | Description |
|----------------|-------------|
| `port` | Listen port |
| `start` / `stop` | Server lifecycle |
| `broadcast(msg)` | Send to all connected clients |
| `onclientconnect`, `ondatareceived`, `onclientdisconnect` | Event handlers |

#### RHTTP

Simple HTTP client with SSL:

```basic
DIM http AS RHTTP
http.url = "https://api.example.com/data"
http.getpage
PRINT http.document
```

| Property/Method | Description |
|----------------|-------------|
| `url` | Target URL |
| `getpage` / `post(data)` | HTTP GET / POST |
| `document` | Response body |
| `responseheaders` | Response headers dict |

---

### Data Science Components: RNum, RPlot, RDataFrame

These components provide data science capabilities backed by native Rust crates: **ndarray** for array math, **polars** for dataframes, and **plotters** for chart rendering. They are feature-gated under the `datascience` feature (enabled by default).

#### RNum (With some NumPy-Compatibility)

Backed by the `ndarray` crate — **60+ methods** for creation, aggregation, element-wise math, arithmetic, ordering, cumulative operations, linear algebra, boolean/search, and random generation.

```basic
DIM arr AS RNum
arr.arange 0, 10, 1
PRINT arr.sum       ' 45.0
PRINT arr.mean      ' 4.5
arr.sin             ' Apply sin() element-wise
arr.normalize       ' Normalize to unit vector
```

| Category | Methods |
|----------|---------|
| **Creation** | `arange(start,stop,step)`, `linspace(start,stop,n)`, `zeros(n)`, `ones(n)`, `full(n,val)`, `fromlist("v1,v2,...")` |
| **Aggregation** | `sum`, `mean`, `min`, `max`, `std`, `var`, `median`, `argmin`, `argmax`, `count`, `ptp` |
| **Element-wise** | `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `sqrt`, `abs`, `exp`, `log`, `log2`, `log10`, `floor`, `ceil`, `round`, `sign`, `reciprocal`, `square`, `negative` |
| **Arithmetic** | `add(val)`, `subtract(val)`, `multiply(val)`, `divide(val)`, `power(exp)`, `mod(div)`, `clip(lo,hi)` |
| **Ordering** | `sort`, `reverse`, `unique`, `shuffle`, `append("vals")`, `slice(start,end)` |
| **Cumulative** | `cumsum`, `cumprod`, `diff` |
| **Linear Algebra** | `dot(other)`, `norm`, `normalize` |
| **Boolean/Search** | `any`, `all`, `where`/`nonzero`, `searchsorted(val)` |
| **Random** | `rand(n)`, `randn(n)`, `uniform(lo,hi,n)`, `randint(lo,hi,n)`, `choice(n)` |
| **Output** | `tolist`, `print`/`show`, `clear` |
| **Properties** | `size`/`length`/`len`, `data`, `shape`, `ndim`, `dtype` |

#### RPlot (With some Matplotlib-Compatibility)

Backed by the `plotters` crate — **20+ methods** for line, bar, scatter, step, area, histogram, pie charts, annotations, and image export.

```basic
DIM plt AS RPlot
plt.title = "Sales Analysis"
plt.xlabel = "Quarter"
plt.ylabel = "Revenue"
plt.grid = TRUE
plt.bar "Q1,Q2,Q3,Q4", "120,340,250,410", "steelblue", "Revenue"
plt.hline 280, "red", "Average"
plt.legend
plt.savefig "sales.png"
```

| Category | Methods |
|----------|---------|
| **Line/Series** | `plot(x,y,color,label)`, `step(x,y,color,label)`, `area(x,y,color,label)`/`fill_between` |
| **Bar Charts** | `bar(labels,values,color,label)`, `barh(labels,values,color,label)` |
| **Scatter** | `scatter(x,y,color,label)` |
| **Statistical** | `hist(data,bins,color,label)`, `pie(labels,values,colors)` |
| **References** | `hline(y,color,label)`/`axhline`, `vline(x,color,label)`/`axvline` |
| **Annotations** | `annotate(text,x,y,color)` |
| **Layout** | `legend(position)`, `figsize(w,h,dpi)`, `xlim(min,max)`, `ylim(min,max)`, `xscale(type)`, `yscale(type)` |
| **Output** | `savefig(filename)`/`save`, `clear` |
| **Properties** | `title`, `xlabel`, `ylabel`, `grid`, `width`, `height`, `dpi` |
| **Colors** | 30+ named colors (red, blue, steelblue, coral, etc.) plus hex `#RRGGBB`; auto-color palette for multi-series |

#### RDataFrame (With some Pandas-Compatibility)

Backed by the `polars` crate — **40+ methods** for CSV/JSON I/O, selection, filtering, grouping, column operations, statistics, sampling, joins, transforms, and GUI grid binding.

```basic
DIM df AS RDataFrame
df.loadfromcsv "employees.csv"
PRINT df.describe
df.filter "salary", ">", "50000"
df.sort "name", 1
df.togrid "Grid1"          ' Populate RStringGrid with DataFrame
```

| Category | Methods |
|----------|---------|
| **I/O** | `loadfromcsv(file)`/`readcsv`, `savetocsv(file)`, `loadfromjson(file)`, `savetojson(file)` |
| **Selection** | `head(n)`, `tail(n)`, `cell(row,col)`, `cellbyname(row,name)`/`at`, `setcell(row,col,val)`, `iloc(row)`, `select("col1,col2,...")` |
| **Sorting** | `sort(col,asc)`, `sort_values(col,asc)` |
| **Filtering** | `filter(col,op,val)` (operators: `>`, `<`, `>=`, `<=`, `=`, `!=`, `contains`), `query(expr)` |
| **Grouping** | `groupby(col,aggCol,func)` — functions: `mean`, `sum`, `count`, `min`, `max`, `first`, `last` |
| **Columns** | `drop(col)`, `rename(old,new)`, `addcolumn(name,values)` |
| **Missing Data** | `fillna(value)`, `dropna()` |
| **Statistics** | `describe`, `value_counts(col)`, `nunique(col)`, `corr(col1,col2)` |
| **Sampling** | `sample(n)`, `nlargest(n,col)`, `nsmallest(n,col)` |
| **Info** | `info`, `dtypes`, `shape`, `columns`, `rows`/`rowcount` |
| **Merge/Join** | `merge(otherDf,onCol,how)` (inner/left/right/outer/cross), `concat(otherDf)` |
| **Transform** | `transpose`, `apply(col,func)` (upper/lower/abs/round/sqrt/log), `replace(col,old,new)` |
| **Display** | `tostring`/`show`/`print`, `togrid(gridName)` — populates RStringGrid with headers and data |
| **Properties** | `rowcount`/`height`/`nrows`, `colcount`/`width`/`ncols`, `columns`, `shape`, `empty` |

#### RImage + Plot Integration

`RImage` can display plots directly:

```basic
DIM plt AS RPlot
DIM img AS RImage
plt.plot x, y, "red", "Data"
plt.savefig "temp.png"
img.loadfromfile "temp.png"
```

---

## The Self-Hosted IDE

The project ships with its own robust **Visual Form Designer & Code Editor** (`ide.rr`).

- **Self-hosting**: Written purely in RapidR BASIC, serving as the ultimate benchmark of the transpiler's completeness.
- **Native compilation**: Compiles to a native FLTK GUI application.
- **Component Palette**: Drag-and-drop components onto a visual `RCanvas` design surface — includes all GUI components plus data science components (RNum, RDataFrame, RPlot).
- **8-Handle Resize**: Full directional drag-and-resize with hit-testing mathematics.
- **Property Grid**: Double-editable spreadsheet for properties like `Caption`, `Color` (with popup pickers), `Font`, `CsvFile`, `DataSource`, `Title`, `XLabel`, `YLabel`, `Grid`.
- **Event Grid**: Browse and bind event handlers; double-clicking auto-generates SUB stubs.
- **Code Editor**: Integrated `RCodeEditor` with syntax highlighting.
- **Code Generation**: VB-style auto-stub generation — the IDE transpiles your visual design into `.rr` source code, including data science property emission.
- **Data Science Integration**: Non-visual RNum, RDataFrame, and RPlot components appear in the toolbar and are managed as 48×48 icon placeholders; RDataFrame supports `CsvFile` and `DataSource` properties; RStringGrid supports `DataSource` binding to a DataFrame.
- **Global state management**: Module-level variables (SelIndex, ShowingCode, event arrays) are properly shared across all callbacks via thread-local storage.

```bash
cargo run -- codegen examples/ide.rr /tmp/ide_rust
cd /tmp/ide_rust && cargo build && ./target/debug/ide
```

---

## VS Code Extension

A comprehensive VS Code extension is included at `utilities/vscodeext/rapidr/` providing production-grade language support for `.rr` files.

### Features

- **Syntax Highlighting** — Full TextMate grammar: keywords, types, components (45+), built-in functions (100+), directives, comments, strings, numbers, Rust blocks (`RUSTSTART`/`RUSTEND`)
- **IntelliSense** — Context-aware autocomplete for:
  - Component properties, methods, and events (dot-completion: `Button1.`)
  - WITH block member access (`.Property`)
  - CREATE block property/event suggestions
  - Preprocessor directives (`$INCLUDE`, `$DEFINE`, etc.)
  - Type completions after `AS`
  - All built-in functions with snippet insertion
  - User-defined SUBs, FUNCTIONs, variables, constants, and TYPE members
- **Hover Documentation** — Rich Markdown hover for:
  - All 45+ component types with property/method/event listings
  - Data science method signatures with parameter descriptions
  - Built-in function signatures
  - Keyword descriptions
  - User-defined TYPE structures with field and method listings
- **Signature Help** — Parameter hints when typing function calls, including both built-in functions and component methods (RNum, RDataFrame, RPlot)
- **Document Symbols** — Outline view showing SUBs, FUNCTIONs, TYPEs, CREATE blocks, CONSTs, and DIM variables
- **Diagnostics** — Real-time validation on save: unclosed block detection, unterminated strings
- **Code Snippets** — 40+ snippets including:
  - Language constructs: `if`, `for`, `while`, `select`, `sub`, `func`, `type`
  - Component creation: `createform`, `createbutton`, `creategrid`, `createcodeeditor`, ...
  - Data science: `createnum`, `createdf`, `createplot`, `dfload`, `dffilter`, `dfgroupby`, `plotline`, `plotbar`, `plotscatter`
  - Application templates: `rpcons` (console), `rpgui` (GUI), `rpdb` (database), `rpdata` (data science)
- **Compile Integration** — Compile and run from VS Code:
  - `Ctrl+Shift+B` / `Cmd+Shift+B` — Compile
  - `F5` — Compile and Run
  - Status bar button: "▶ RapidR"
- **Code Folding** — Automatic folding for IF/FOR/WHILE/DO/SUB/FUNCTION/TYPE/CREATE/WITH blocks and `$IFDEF`/`$ENDIF` regions
- **Auto-Indent** — Smart indentation rules for all block structures

### Installation

```bash
cd utilities/vscodeext/rapidr
vsce package --no-dependencies
code --install-extension rapidr-2.0.0.vsix
```

Or install the pre-built `.vsix` from the `utilities/vscodeext/rapidr/` directory.

---

## Demo Examples

Three demo applications showcase the data science components with full GUI integration:

| Demo | Components | Description |
|------|-----------|-------------|
| `demo_plot.rr` | `RPlot` + `RImage` | Generates sine/cosine plots and bar charts, displays them inside a `RImage` on a form |
| `demo_num.rr` | `RNum` + `RStringGrid` | Array math operations (element-wise, statistics, linspace, dot product) shown in a grid |
| `demo_dataframe.rr` | `RDataFrame` + `RStringGrid` | Loads CSV data, supports sort, filter, group-by, and summary statistics in a grid |

Run any of them:
```bash
cargo run -- codegen examples/demo_num.rr /tmp/demo_num
cd /tmp/demo_num && cargo build && ./target/debug/demo_num
```

> **Note:** The RDataFrame demo expects `examples/demo_dataframe_data.csv` (included) for sample employee data.

---

## Syntax Reference

### Variables & Types

```basic
DIM x AS INTEGER
DIM name AS STRING
DIM values(100) AS DOUBLE
DIM flag AS LONG
```

### TYPE (User-Defined Types)

```basic
TYPE PersonType
  Name AS STRING
  Age AS INTEGER
END TYPE

DIM person AS PersonType
person.Name = "Alice"
person.Age = 30
```

### SUB & FUNCTION

```basic
SUB Greet(name AS STRING)
  PRINT "Hello, " + name
END SUB

FUNCTION Add(a AS INTEGER, b AS INTEGER) AS INTEGER
  Add = a + b
END FUNCTION
```

### Control Flow

```basic
' IF...THEN...ELSE
IF x > 10 THEN
  PRINT "Large"
ELSEIF x > 5 THEN
  PRINT "Medium"
ELSE
  PRINT "Small"
END IF

' FOR...NEXT
FOR i = 1 TO 10
  PRINT STR$(i)
NEXT i

' WHILE...WEND
WHILE x < 100
  x = x * 2
WEND

' DO...LOOP
DO
  x = x + 1
LOOP UNTIL x >= 50

' SELECT CASE
SELECT CASE grade
  CASE "A"
    PRINT "Excellent"
  CASE "B", "C"
    PRINT "Good"
  CASE ELSE
    PRINT "Try harder"
END SELECT
```

### CREATE Blocks

```basic
CREATE Form1 AS RForm
  Caption = "My Application"
  Width = 640
  Height = 480
  Center
  CREATE Panel1 AS RPanel
    Align = 5
    CREATE Button1 AS RButton
      Caption = "OK"
      Left = 10
      Top = 10
      OnClick = HandleOK
    END CREATE
  END CREATE
END CREATE

Form1.ShowModal
```

---

## Test Suite

### Rust Tests

```bash
cargo test
```

| Crate | Tests | Coverage |
|-------|-------|----------|
| `rapidr-codegen-rust` | 6 | DIM, FOR, assignment, expression codegen |
| `rapidr-lexer` | 7 | Token types, keywords, operators, literals |
| `rapidr-preprocessor` | 1 | Directive handling |
| `rapidr-parser` | 15 | AST generation for all language constructs |
| `rapidr-diagnostics` | 1 | Span and diagnostic types |
| `rapidr-ast` | 9 | AST node validation |
| `rapidr-cli` | 31 | CLI commands, example compilation smoke tests |

---

## Development Conventions

- Transpiler expects strict block encapsulation for multi-line `IF…THEN` (must have `END IF`).
- New GUI components should be added to `gui_create_widget()` in `gui.rs` and registered in `is_component_type_name()` in the codegen.
- Global dependencies are resolved at the start of `SUB` blocks during code generation. Avoid deep nesting that obscures global modification scope.
- The runtime uses thread-local storage for all component state and global variables (`gv()`/`gs()` accessors).

---

## Credits

- **Roberto Berrospe** ([@iBobX](https://github.com/iBobX)) — Creator, architect, and lead developer
- **VS Code Copilot with Claude Opus 4.6** — AI pair-programming assistant for feature implementation, testing, and documentation

---

## License

This project is licensed under the [MIT License](LICENSE).

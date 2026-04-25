# RapidR — BASIC to Native Rust Transpiler

[![Rust](https://img.shields.io/badge/Rust-2021-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)

**RapidR** is an experiment. The idea is to implement a BASIC-to-Rust transpiler and native runtime that compiles `.rr` source files into standalone Rust projects, producing fast, native executables. At the current stage, it provides **52+ GUI components** (R-prefixed: `RForm`, `RButton`, `RCoolBtn`, `ROvalBtn`, `RJson`, `RStringGrid`, …), **9 web-exclusive components** (`RWebView`, `RDOM`, `RJavaScript`, …), **100+ built-in functions**, database access (MySQL + SQLite), networking, JSON processing, data science components, and a self-hosted **Visual IDE** — all compiled to native code via FLTK or to **WebAssembly** for browser deployment.

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
  - [Self-Hosted Web IDE — `web-ide/`](#self-hosted-web-ide--web-ide)
- [Web Compilation (WASM)](#web-compilation-wasm)
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
2. **Code Generator (`crates/rapidr-codegen-rust/`)** — Walks the AST and emits Rust source code targeting either the native or web runtime.
3. **Native Runtime (`crates/rapidr-runtime-core/`)** — FLTK-based GUI, built-in functions, database (MySQL + SQLite), networking, data science, and file I/O.
4. **Web Runtime (`crates/rapidr-runtime-web/`)** — Browser-based GUI via DOM/Canvas, web-exclusive components, WASM-compatible built-ins, and `wasm-bindgen` interop.
5. **CLI (`crates/rapidr-cli/`)** — Command-line interface with `codegen` and `--web` commands.

## Status

RapidR v1.0.0 has reached **functional transpiler status**. The Rust workspace provides a complete pipeline from `.rr` source through parsing, Rust code generation, and compilation to native executables. All 29 example programs compile and run, including the self-hosted IDE.

### Crate Architecture

The Cargo workspace contains **two cooperating pipelines** in addition to
the supporting crates:

- **Native Rust codegen** (`rapidr-codegen-rust` → `rapidr-runtime-core` /
  `rapidr-runtime-web`) — fastest path; compiles `.rr` to a Rust project
  then to a native binary or wasm-bindgen web app.
- **Bytecode interpreter `rapidrintr`** (`rapidr-bcgen` → `.rrbc` →
  `rapidr-vm` + a `Host`) — single self-contained artifact that runs the
  same `.rr` natively *or* in the browser without re-compiling. Used by
  the self-hosted Web IDE in [`web-ide/`](web-ide/).

| Crate | Path | Description |
|-------|------|-------------|
| `rapidr-cli` | `crates/` | CLI entry point — `version`, `preprocess`, `lex`, `parse`, `codegen`, `build`, `build-bc`, `run-bc`, `bundle-bc` |
| `rapidr-diagnostics` | `crates/` | `TextSpan`, `SourceLocation`, and `Diagnostic` types |
| `rapidr-ast` | `crates/` | Shared AST data structures |
| `rapidr-value` | `crates/` | Shared `Value` enum used by codegen and the VM |
| `rapidr-preprocessor` | `crates/` | `$DEFINE`, `$IFDEF`, `$INCLUDE`, `$MACRO`, `$THEME`, … |
| `rapidr-lexer` | `crates/` | Tokenization |
| `rapidr-parser` | `crates/` | Recursive-descent parser → AST |
| `rapidr-rrcss` | `crates/` | Tiny CSS subset used by the web runtime for style props |
| `rapidr-codegen-rust` | `crates/` | AST → Rust source targeting `rapidr-runtime-core` or `rapidr-runtime-web` |
| `rapidr-runtime-core` | `crates/` | Native runtime — FLTK GUI, builtins, MySQL/SQLite, networking, data science, file I/O |
| `rapidr-runtime-web` | `crates/` | Web runtime — DOM/Canvas GUI, web-exclusive components, in-memory SQLite, RSocket-over-WebSocket |
| `rapidr-buildserver` | `crates/` | (Legacy) axum HTTP build service used by `examples/web_ide.rr`. Superseded by the self-contained [`web-ide/`](web-ide/). |
| `rapidr-bytecode`     | `interpreter/` | `.rrbc` format: `RRBC` magic + ~50 stack opcodes |
| `rapidr-bcgen`        | `interpreter/` | AST → bytecode lowering (mirrors `rapidr-codegen-rust`) |
| `rapidr-vm`           | `interpreter/` | `Vm<Host>` stack interpreter with frames/globals/`Host` trait |
| `rapidr-vm-host-native` | `interpreter/` | `Host` impl backed by `rapidr-runtime-core` |
| `rapidr-vm-host-web`  | `interpreter/` | wasm cdylib — exposes **both** `compile(src) → Vec<u8>` *and* `rapidr_run_bc(bytes)` from one ~830 KB module |
| `rapidr-compiler-wasm`| `interpreter/` | Stand-alone wasm-bindgen wrapper exposing only `compile()` |
| `rapidr-webbundle`    | `interpreter/` | Builds the static `.zip` produced by `rapidr bundle-bc` |
| `rapidr-runner-stub`  | `interpreter/` | Host-stub binary used as the prefix for `--interp` self-contained executables |
### Key Capabilities

- **Native GUI via FLTK** — Forms, buttons, labels, edits, panels, tabs, string grids, combo boxes, code editors, design surfaces, splitters, scroll boxes, and more
- **Web GUI via WASM** — Same component API compiled to WebAssembly for browser deployment, plus 9 web-exclusive components (RWebView, RDOM, RJavaScript, RWebStorage, RWebAudio, RWebVideo, RWebNotification, RWebGeolocation, RRouter)
- **FLTK Themes** — `$THEME` directive supports: Classic, Aero, Metro, AquaClassic, Greybird, Blue, Dark, HighContrast; also `$THEME AUTO` for OS-based selection
- **Global variable mechanism** — Module-level `DIM` variables use thread-local storage (`gv()`/`gs()` accessors), correctly shared across all SUBs/FUNCTIONs
- **User-Defined Types** — `TYPE...END TYPE` with fields, inheritance, constructors, and methods
- **Database** — MySQL (via `mysql` crate) and SQLite (via `rusqlite`) with property-based API
- **Networking** — TCP sockets, server sockets, HTTP client
- **JSON** — `RJson` component for parsing, generating, dot-path access, and file I/O (cross-platform: desktop + web)
- **100+ built-in functions** — String, math, file I/O, system operations
- **Self-hosted IDE (native)** — The visual form designer (`examples/ide.rr`) compiles to a native FLTK application
- **Self-hosted IDE (web)** — [`web-ide/`](web-ide/) is a fully self-contained browser IDE: visual designer + Run + Build (downloadable static `.zip`), all backed by a single combined wasm that holds **both** the compiler and the bytecode interpreter — no build server, no network calls
- **Multi-form apps** — Multiple top-level `RFORM` windows behave like ordinary OS windows; `Parent="Form1"` nests one form inside another. `OnLoad`/`OnClose` lifecycle events fire on both runtimes; `ShowModal` works on web via a dimmed backdrop overlay
- **Data science** — RNum (ndarray), RDataFrame (polars), RPlot (plotters) components for array math, dataframes, and plotting
- **Raw Rust injection** — `RUSTSTART`/`RUSTEND` blocks allow inline Rust code in `.rr` sources

### Validation

```bash
# Run all unit tests
cargo test

# Compile and run an example
./rapidr --release examples/hello_world.rr
./examples/hello_world

# Generate and build the IDE
./rapidr build examples/ide.rr examples/ide_rust --release
./examples/ide

# Compile for the web (WASM)
./rapidr --web examples/hello_web.rr
# Serve and open in browser
cd examples/hello_web_web && python3 -m http.server 8080
```

### Browser IDE (self-contained, zero-backend)

The new **`web-ide/`** is a fully self-hosted browser IDE. It loads a single
combined WebAssembly module (`rapidrintr.wasm`) that exposes **both** the
compiler (`compile(source) → bytecode`) and the runtime
(`rapidr_run_bc(bytes)`). No build server, no CDN, no network calls — everything
runs in the visitor's browser:

```bash
# 1. Build the compiler + the combined web wasm (once)
./build.sh --release
bash tools/build_web_artifacts.sh           # → target/web/rapidrintr.{js,wasm}

# 2. Serve the IDE (any static server works)
python3 -m http.server 8765                 # from the repo root
open http://localhost:8765/web-ide/index.html
```

Inside the IDE:

- **Design tab** — visual form designer. Click a tool in the toolbox
  (Button, Label, Edit, CheckBox, …) to drop it onto a live, real-runtime
  design surface. The right panel is a tree + editable property grid.
  Every change regenerates `.rr` source and re-renders the surface, so what
  you build is byte-identical to what ships.
- **Code tab** — the regenerated source. Editing here updates the design
  preview live (debounced). Pick any example from the dropdown to load it
  into both panes.
- **▶ Run** — compiles the current source in-browser and runs it in the
  Preview pane. No server round-trip.
- **⬇ Build** — compiles + packages a static `.zip` (containing
  `index.html`, `loader.js`, `rapidrintr.{js,wasm}`, `<project>.rrbc`).
  Drop it on any HTTP host (GitHub Pages, S3, plain nginx).
- **＋ New** — start a blank designer project.

Files in [`web-ide/`](web-ide/):

| File | Role |
|------|------|
| [index.html](web-ide/index.html) | IDE shell — Design / Code tabs and Preview pane |
| [host.js](web-ide/host.js) | Controller — wasm boot, designer wiring, Run, Build |
| [designer.js](web-ide/designer.js) | Pure model + `serialize(model) → .rr` source |
| [preview.html](web-ide/preview.html) | Sandboxed runtime iframe (used for both Preview and the design surface) |
| [zip.js](web-ide/zip.js) | In-browser STORED PKZIP writer for the Build button |
| `runtime/` | Symlink to `target/web/` produced by `tools/build_web_artifacts.sh` |

The legacy `examples/web_ide.rr` + `crates/rapidr-buildserver` stack is
kept in the tree for compatibility but is no longer the recommended path —
prefer `web-ide/` for any new work.

---

## Installation

### Prerequisites

- **Rust toolchain** (edition 2021+) — install via [rustup](https://rustup.rs/):
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  source $HOME/.cargo/env
  ```
- **C/C++ compiler** (needed to build the FLTK GUI library):
  - **macOS:** `xcode-select --install`
  - **Linux (Debian/Ubuntu):** `sudo apt install build-essential cmake libx11-dev libxext-dev libxft-dev libxinerama-dev libfontconfig1-dev libpango1.0-dev`
  - **Windows:** Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (MSVC)
- **CMake** (FLTK build dependency):
  - **macOS:** `brew install cmake`
  - **Linux:** `sudo apt install cmake`
  - **Windows:** Bundled with Visual Studio Build Tools
- **(Optional) Node.js + npm** — needed only if you want to build the VS Code extension:
  - Install from [nodejs.org](https://nodejs.org/) or via your package manager
  - Install the VS Code Extension packaging tool: `npm install -g @vscode/vsce`
- **(Optional) wasm-pack** — needed only for WebAssembly compilation:
  ```bash
  rustup target add wasm32-unknown-unknown
  cargo install wasm-bindgen-cli
  ```

### Step-by-Step: Build the Compiler

```bash
# 1. Clone the repository
git clone https://github.com/iBobX/RapidR.git
cd RapidR

# 2. Build the compiler (release mode, produces ./rapidr binary)
./build.sh

# 3. Verify the binary works
./rapidr version
```

The `build.sh` script compiles all crates in release mode and copies the binary to `./rapidr` at the project root.

### Step-by-Step: Build & Run the Visual IDE

```bash
# 1. Generate the Rust project for the IDE
./rapidr codegen examples/ide.rr examples/ide_rust --release

# 2. The IDE binary is at examples/ide — run it
./examples/ide
```

Or build from a fresh checkout:
```bash
./rapidr build examples/ide.rr examples/ide_rust --release
```

### Step-by-Step: Build the VS Code Extension

The extension provides syntax highlighting, snippets, and theme support for `.rr` files.

```bash
# 1. Make sure you have Node.js, npm, and vsce installed
npm install -g @vscode/vsce

# 2. Package the extension (creates a .vsix file)
./build_vsc_extension.sh

# 3. Or package and install directly into VS Code
./build_vsc_extension.sh install
```

### Step-by-Step: Compile a Web (WASM) Project

```bash
# 1. Compile a .rr file for the web
./rapidr --web examples/hello_web.rr

# 2. Serve the generated files
cd examples/hello_web_web
python3 -m http.server 8080

# 3. Open http://localhost:8080 in your browser
```

---

## Quick Start

### Hello World

Create `hello.rr`:
```basic
PRINT "Hello, World!"
```

Compile and run:
```bash
./rapidr --release hello.rr
./hello
```

Or generate a Rust project manually:
```bash
./rapidr codegen hello.rr /tmp/hello
cd /tmp/hello && cargo build
./target/debug/hello
```

### Launch the IDE

```bash
./rapidr build examples/ide.rr examples/ide_rust --release
./examples/ide
```

### CLI Options

```
./rapidr <command> [options]
```

| Command | Description |
|---------|-------------|
| `codegen <file.rr> [outdir]` | Generate a Rust project from a `.rr` file |
| `build <file.rr> [outdir] [flags]` | Generate, build, and copy binary alongside source |
| `build-bc <file.rr> [-o out.rrbc]` | Compile to portable bytecode (no Rust toolchain needed at runtime) |
| `run-bc <file.rrbc>` | Run a `.rrbc` via the native `Host` |
| `bundle-bc <file.rr> [-o out.zip]` | Compile + package a static web bundle (HTML + `rapidrintr.{js,wasm}` + `.rrbc`) |
| `lex <file.rr>` | Dump token stream |
| `parse <file.rr>` | Dump AST |
| `preprocess <file.rr>` | Dump preprocessed source |
| `version` | Print version |

Flags for `build` (and the bare `rapidr <file.rr>` shortcut):

| Flag | Description |
|------|-------------|
| `--release` / `-r` | Build in release mode (optimized) |
| `--debug`   / `-d` | Build in debug mode (default) |
| `--web`     / `-w` | Target WebAssembly via the Rust codegen + wasm-bindgen pipeline |
| `--interp`  / `-i` | Target the bytecode interpreter — emits a self-contained native binary (or, with `--web`, a static `.zip`) that has no Rust toolchain dependency at runtime |

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

### Bytecode Pipeline (`rapidrintr`)

In addition to the Rust-codegen path, RapidR ships a portable **bytecode
interpreter** (`rapidrintr`). The same source can be compiled to a
compact `.rrbc` artifact and executed by a stack VM — natively on
desktop/console *or* in the browser via WebAssembly. No re-compile step
is needed to ship to the web.

| Stage | Crate | Description |
|-------|-------|-------------|
| Bytecode format | `rapidr-bytecode` | `RRBC` magic + ~50 stack opcodes, hand-rolled little-endian (de)serialisation |
| Bytecode generator | `rapidr-bcgen` | Lowers AST → bytecode (mirrors `rapidr-codegen-rust`) |
| Stack VM | `rapidr-vm` | `Vm<Host>` interpreter with frames, globals, and a small `Host` trait |
| Native host | `rapidr-vm-host-native` | `Host` impl backed by `rapidr-runtime-core` (FLTK, sockets, SQLite, FFI) |
| Web host | `rapidr-vm-host-web` | `Host` impl backed by `rapidr-runtime-web` (DOM, canvas) — `wasm-bindgen` cdylib |
| In-browser compiler | `rapidr-compiler-wasm` | `wasm-bindgen` wrapper exposing `compile(source) -> Vec<u8>` |
| Web bundler | `rapidr-webbundle` | Builds a static `.zip` containing `index.html`, `loader.js`, `rapidrintr.{wasm,js}`, `<project>.rrbc` |

CLI entry points:

```sh
# Unified build — Rust codegen (compiled, fastest):
rapidr build hello.rr                          # desktop
rapidr build hello.rr --web                    # WebAssembly via wasm-bindgen

# Same source, bytecode pipeline (single self-contained artifact):
rapidr build hello.rr --interp                 # one self-contained exe (stub + .rrbc)
rapidr build hello.rr --web --interp           # static .zip (rapidrintr.wasm + .rrbc)

# Low-level primitives (still available):
rapidr build-bc  hello.rr -o hello.rrbc        # source -> bytecode
rapidr run-bc    hello.rrbc                    # run via NativeHost
rapidr bundle-bc hello.rr -o hello-web.zip     # source -> hostable web bundle
```

When `--interp` is set on `build`, the CLI generates a `.rrbc` and
appends it to a pre-built `rapidrintr-runner` stub binary (footer:
`[rrbc bytes][magic "RRBCEXE1"][u32 length]`). The runner reads its own
appended payload at startup via `current_exe()` and runs it with
`rapidr-vm-host-native`. The compiled and interpreted modes are
end-to-end equivalent for every console example in the test matrix
(`tests/full_matrix.sh`).

The bundle is fully static: unzip and serve from any HTTP host
(GitHub Pages, S3, plain nginx, `python3 -m http.server`).

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
| `$APPTYPE` | Set application type (`GUI`, `CONSOLE`, or `WEB`) | `$APPTYPE WEB` |
| `$OPTIMIZE` | Optimization hint (pass-through) | `$OPTIMIZE ON` |
| `$ESCAPECHARS` | Enable escape character processing | `$ESCAPECHARS ON` |
| `$THEME` | Set FLTK theme (Rust only) | `$THEME AquaClassic` or `$THEME AUTO` |

**Available themes (Rust / FLTK):** `Classic`, `Aero`, `Metro`, `AquaClassic`, `Greybird`, `Blue`, `Dark`, `HighContrast`, `AUTO` (selects by OS: AquaClassic on macOS, Aero on Windows, Greybird on Linux).

---

## The Runtime Library

The runtime (`crates/rapidr-runtime-core/`) provides all the R-prefixed components and built-in functions that compiled programs link against.

### GUI Components

Powered by **FLTK** (via the `fltk` crate), the runtime provides **51+ component classes**.

#### Forms & Containers

| Component | Description |
|-----------|-------------|
| `RForm` | Top-level window |
| `RFormMDI` | MDI parent form (WIP) |
| `RPanel` | Container panel |
| `RGroupBox` | Labeled group container |
| `RTabControl` | Tabbed container |
| `RSplitter` | Resizable split pane |
| `RScrollBox` | Scrollable container |

#### Input Controls

| Component | Description |
|-----------|-------------|
| `RButton` | Push button |
| `RCoolBtn` | Flat/toggle toolbar button with multi-state BMP images and group behavior |
| `ROvalBtn` | Oval/round button with custom Color, ColorHighlight, ColorShadow |
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
| `RJson`          | JSON parsing, generation, dot-path access, and file I/O |

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

These components provide data science capabilities backed by native Rust crates: **ndarray** for array math, **polars** for dataframes, **plotters** for chart rendering, and **image** for in-memory PNG encoding. They are feature-gated under the `datascience` feature (enabled by default).

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

`RImage` can display plots directly via in-memory PNG rendering — no temporary files are created:

```basic
DIM plt AS RPlot
DIM img AS RImage
plt.plot x, y, "red", "Data"
img.loadfromplot plt    ' Renders to PNG bytes in memory, loads directly into FLTK widget
```

---

## Web Compilation (WASM)

RapidR can compile `.rr` programs to **WebAssembly** for deployment in any
modern browser. Two pipelines are supported and both produce ordinary
static sites that work on any HTTP host:

1. **Bytecode + interpreter (recommended)** — `rapidr bundle-bc file.rr`
   produces a `.zip` containing `index.html`, the combined
   `rapidrintr.{js,wasm}` (~830 KB, holds **both** compiler and
   runtime), and a tiny `<project>.rrbc`. No Rust toolchain or
   `wasm-bindgen` install is needed at build time beyond the prebuilt
   wasm. This is what the self-hosted [Web IDE](#self-hosted-web-ide--web-ide)
   uses.
2. **Rust codegen → wasm-bindgen** — `rapidr build file.rr --web` runs
   the same Rust codegen used for native binaries, but against
   `rapidr-runtime-web` and the `wasm32-unknown-unknown` target. Produces
   `index.html`, project-named `.js` bindings, and a per-program
   `_bg.wasm`. Slightly smaller per-app payloads, but every program
   ships its own wasm.

### How the bytecode pipeline works

| Step | Crate | Output |
|------|-------|--------|
| Source → AST | `rapidr-parser` | typed AST |
| AST → bytecode | `rapidr-bcgen` | `.rrbc` (`RRBC` magic + ~50 stack opcodes) |
| Bytecode → exec | `rapidr-vm` | runs against a `Host` impl |
| Web `Host` | `rapidr-vm-host-web` | wasm cdylib exposing `compile()` and `rapidr_run_bc()` |
| Static bundle | `rapidr-webbundle` | `index.html`, `loader.js`, `rapidrintr.{js,wasm}`, `<name>.rrbc` |

### How the Rust-codegen pipeline works

The `--web` flag activates the codegen path:

1. **Code generation** — emits Rust targeting `rapidr-runtime-web` instead of `rapidr-runtime-core`
2. **Rust → WASM** — `cargo build --target wasm32-unknown-unknown`
3. **wasm-bindgen** — generates JS glue
4. **Output** — a static directory with `index.html`, `.js`, `_bg.wasm`

### Web Form Window Management

Web forms (`RForm`) behave like desktop windows:

- **Titlebar** — Each form has a styled titlebar displaying the caption with minimize (−), maximize (□), and close (✕) buttons
- **Drag-to-move** — Click and drag the titlebar to reposition forms anywhere on the page
- **Z-index stacking** — Clicking a form brings it to front; multiple overlapping forms work as expected
- **Minimize** — Collapses the form to a taskbar button at the bottom of the viewport; click to restore
- **Maximize** — Expands to fill the viewport; click again to restore to original size and position
- **Close** — Hides the form (sets `display: none`)
- **Tab controls** — Tab buttons are clickable with visual active-state highlighting and `OnChange` event firing

### Prerequisites

```bash
# Add the WASM target
rustup target add wasm32-unknown-unknown

# Install wasm-bindgen CLI
cargo install wasm-bindgen-cli
```

### Usage

Bytecode + interpreter (recommended — single static `.zip`):

```bash
# 1. Build the combined wasm once
bash tools/build_web_artifacts.sh

# 2. Compile + bundle
./rapidr bundle-bc examples/hello_web.rr -o /tmp/hello-web.zip

# 3. Unzip and serve
unzip /tmp/hello-web.zip -d /tmp/hello_web && cd /tmp/hello_web
python3 -m http.server 8080
# Open http://localhost:8080
```

Rust codegen (per-program wasm):

```bash
./rapidr build --web examples/hello_web.rr
ls examples/hello_web_web/
# index.html  hello_web.js  hello_web_bg.wasm
cd examples/hello_web_web && python3 -m http.server 8080
```

Or just open the [Web IDE](#self-hosted-web-ide--web-ide) and click **⬇ Build**.

You can also use `$APPTYPE WEB` in your source to indicate a web application:

```basic
' My Web App
$APPTYPE WEB

CREATE MainForm AS RForm
    Caption = "Hello Web"
    Width = 600
    Height = 400

    CREATE Label1 AS RLabel
        Caption = "Welcome to RapidR Web!"
        Left = 20
        Top = 20
    END CREATE

    CREATE Button1 AS RButton
        Caption = "Click Me"
        Left = 20
        Top = 60
        Width = 120
        Height = 35
        OnClick = OnButtonClick
    END CREATE
END CREATE

SUB OnButtonClick(Sender AS POBJECT)
    SHOWMESSAGE "Hello from the browser!"
END SUB
```

### Component Compatibility

Most standard GUI components work in both native and web targets:

| Component | Native (FLTK) | Web (WASM) | Notes |
|-----------|:---:|:---:|-------|
| RForm | ✅ | ✅ | Rendered as a `<div>` with titlebar, minimize/maximize/close, drag-to-move, z-index stacking |
| RButton | ✅ | ✅ | HTML `<button>` element |
| RLabel | ✅ | ✅ | HTML `<span>` element |
| REdit | ✅ | ✅ | HTML `<input>` element |
| RCanvas | ✅ | ✅ | HTML `<canvas>` element |
| RPanel | ✅ | ✅ | `<div>` container |
| RListBox | ✅ | ✅ | HTML `<select>` element |
| RComboBox | ✅ | ✅ | HTML `<select>` element |
| RCheckBox | ✅ | ✅ | HTML `<input type="checkbox">` |
| RRadioButton | ✅ | ✅ | HTML `<input type="radio">` |
| RStringGrid | ✅ | ✅ | HTML `<table>` element |
| RTabControl | ✅ | ✅ | Tabbed `<div>` with clickable tab buttons and visual highlighting |
| RProgressBar | ✅ | ✅ | HTML `<progress>` element |
| RTimer | ✅ | ✅ | JavaScript `setInterval` |
| RRichEdit | ✅ | ✅ | HTML `<textarea>` |
| RCodeEditor | ✅ | ✅ | HTML `<textarea>` with monospace |
| RImage | ✅ | ✅ | HTML `<img>` element |

### Web-Exclusive Components

These components are **only available** when compiling with `--web`:

| Component | Description |
|-----------|-------------|
| `RWebView` | Embedded HTML viewer (renders as `<iframe>`) — properties: `url`, `html`, `sandbox` |
| `RDOM` | Direct DOM element creation — create arbitrary HTML tags, set `innerHTML`, `cssClass`, `cssStyle`, call `setAttribute`, `addClass`, `querySelector` |
| `RJavaScript` | Execute arbitrary JavaScript — `Eval("code")` returns result, `Call("func", args...)` invokes JS functions |
| `RWebStorage` | Browser localStorage/sessionStorage — `Set(key, val)`, `Get(key)`, `Remove(key)`, `Clear`, `Keys`, `HasKey(key)` |
| `RWebAudio` | HTML5 audio player — properties: `src`, `volume`, `loop`, `controls`; methods: `Play`, `Pause`, `Stop`, `Seek(time)` |
| `RWebVideo` | HTML5 video player — same API as RWebAudio plus `poster`, `fullscreen`, visual positioning |
| `RWebNotification` | Browser push notifications — `RequestPermission`, then `Show` with `title` and `body` properties |
| `RWebGeolocation` | Browser geolocation — `GetPosition` stores `latitude`, `longitude`, `accuracy` |
| `RRouter` | SPA hash-based router — `Navigate(route)`, `Back`, `Forward`, `OnRouteChange` event |

### Web Examples

Six example programs demonstrate web compilation:

| Example | Components Used | Description |
|---------|----------------|-------------|
| `hello_web.rr` | RForm, RLabel, REdit, RButton | Basic form with a click counter |
| `web_calculator.rr` | RForm, REdit, RButton, RLabel | Calculator with 17 buttons and display |
| `web_canvas.rr` | RForm, RCanvas, RButton, RLabel | Drawing app with freehand, circle, rectangle modes |
| `web_todo.rr` | RForm, RListBox, REdit, RButton, RCheckBox, RLabel | Todo list with add/remove/clear |
| `web_dashboard.rr` | RForm, RTimer, RTabControl, RStringGrid, RComboBox, RProgressBar, RLabel | Dashboard with live clock, tabs, and data grid |
| `web_datascience.rr` | RForm, RTabControl, RStringGrid, RNum, RDataFrame, RPlot, RSQLite | Data science demo with RNum math, DataFrame CRUD, line/bar/pie charts, and in-memory SQL |

Compile and test any of them — either via the bytecode bundler or the Rust-codegen path:

```bash
# Bytecode (single combined wasm + .rrbc, recommended)
./rapidr bundle-bc examples/web_calculator.rr -o /tmp/calc.zip
unzip -o /tmp/calc.zip -d /tmp/calc && (cd /tmp/calc && python3 -m http.server 8080)

# OR: Rust codegen (per-program wasm)
./rapidr build --web examples/web_calculator.rr
cd examples/web_calculator_web && python3 -m http.server 8080
# Open http://localhost:8080
```

---

## The Self-Hosted IDE

The project ships with its own experimental and WIP **Visual Form Designer & Code Editor** (`ide.rr`).

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
./rapidr build --release examples/ide.rr examples/ide_rust
./examples/ide
```
**Important:** The IDE is a work in progress and has lots of quirks, but it demonstrates the full capabilities of the RapidR runtime. For Code Editor syntax highlighting and IntelliSense, I recommend using the VS Code extension described below.

### Self-Hosted Web IDE — `web-ide/` (v1.0.0)

In addition to the native FLTK IDE above, RapidR ships a **fully self-contained
browser IDE** under [`web-ide/`](web-ide/). It is **not** a `.rr` program — it
is plain HTML/JS that drives the same combined wasm module
(`rapidrintr.wasm`) used to ship `bundle-bc` apps. That module exposes
**both** the compiler and the bytecode interpreter, so the entire
edit → compile → run → export loop happens in the visitor's browser, with
no build server and no network calls.

**Highlights (1.0):**

- **VB6-style multi-form designer.** Each form is its own design / code
  pair of MDI tabs; switch with the project tree. Designer, code, and
  preview always stay in sync — every property edit re-emits source and
  re-renders the active form.
- **22-component toolbox**, organised into three groups in the sidebar
  and split between *visible* (drop on the form) and *non-visual* (drop
  into the design tray below the form):

  | Group | Components |
  |-------|------------|
  | Common Controls | `RButton` `RLabel` `REdit` `RCheckBox` `RRadioBtn` `RComboBox` `RListBox` `RImage` `RPanel` `RGroupBox` `RProgressBar` `RTrackBar` |
  | Data & Web | `RDataFrame` `RPlot` (visible) · `RNum` `RJson` `RStringList` (tray) |
  | I/O & Storage | `RTimer` `RSqlite` `RFileStream` `RHttp` `RWebStorage` |

- **Property grid** with type-aware editors: enum dropdowns, asset
  pickers, **live color picker** with realtime preview + `OK` confirm,
  **font picker** dialog (family / size / weight / style), booleans,
  numbers, multi-line strings.
- **Asset pipeline.** *File → Upload Asset…* attaches images / CSVs /
  JSON to the project. Asset properties (`picture`, `dataset`, `csvfile`,
  `imageurl`, …) get a dropdown of project assets plus a `+` shortcut.
  Assets are serialized into the `.rrproj` JSON and packed into the
  built bundle under `assets/<name>` (STORED — no recompression of
  PNG / JPEG).
- **Run & Build.**
  - **▶ Run** compiles current source via `compile()` in wasm and runs
    the resulting `.rrbc` in the Preview iframe via `rapidr_run_bc()`.
  - **⬇ Build** writes a STORED PKZIP in-browser containing
    `index.html`, `loader.js`, `rapidrintr.{js,wasm}`,
    `<project>.rrbc`, optional `assets/…`, and a `manifest.json`
    carrying the IDE version + build timestamp.
  - Bundled `index.html` ships a strict
    `Content-Security-Policy` meta tag (`default-src 'self'`,
    `frame-src 'none'`, `object-src 'none'`,
    `script-src 'self' 'wasm-unsafe-eval'`) — drop on any HTTP host.
- **Examples dropdown** — load any of the 14+ shipped web examples
  (`hello_web`, `web_calculator`, `web_dashboard`, `demo_sqlite`,
  `demo_plot`, `demo_chat_client`, …) into both panes for inspection
  or editing.
- **About / License / View Source** dialogs, version surfaced in the
  status bar (`v1.0.0`) and `<title>`, third-party attributions in
  [`LICENSES.md`](LICENSES.md).
- **Security.** All user-controlled strings interpolated into IDE DOM
  (form names, module names, widget names) are escaped. No `eval()` /
  `new Function()` outside vendored Monaco + wasm-bindgen glue. The
  preview iframe is sandboxed.

**Run it:**

```bash
./build.sh --release                       # native CLI
bash tools/build_web_artifacts.sh          # → target/web/rapidrintr.{js,wasm}
python3 -m http.server 8765                # any static server works
open http://localhost:8765/web-ide/index.html
```

---

## VS Code Extension

A comprehensive VS Code extension is included at `utilities/vscodeext/rapidr/` providing production-grade language support for `.rr` files.

### Features

- **Syntax Highlighting** — Full TextMate grammar: keywords, types, components (55+, including web-exclusive), built-in functions (100+), directives, comments, strings, numbers, Rust blocks (`RUSTSTART`/`RUSTEND`)
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
- **Code Snippets** — 50+ snippets including:
  - Language constructs: `if`, `for`, `while`, `select`, `sub`, `func`, `type`
  - Component creation: `createform`, `createbutton`, `creategrid`, `createcodeeditor`, ...
  - Web components: `createwebview`, `createdom`, `createjs`, `createwebstorage`, `createwebaudio`, `createwebvideo`, `createnotification`, `createrouter`
  - Dialogs & canvas: `createcolordialog`, `createfontdialog`, `canvassetfont`
  - Data science: `createnum`, `createdf`, `createplot`, `dfload`, `dffilter`, `dfgroupby`, `plotline`, `plotbar`, `plotscatter`
  - Application templates: `rpcons` (console), `rpgui` (GUI), `rpweb` (web/WASM), `rpdb` (database), `rpdata` (data science)
- **Compile Integration** — Compile and run from VS Code:
  - `Ctrl+Shift+B` / `Cmd+Shift+B` — Compile
  - `F5` — Compile and Run
  - `Ctrl+Shift+W` / `Cmd+Shift+W` — Compile for Web (WASM)
  - "Compile for Web and Serve" — Compiles, starts a local HTTP server, and opens browser
  - Status bar button: "▶ RapidR"
- **Code Folding** — Automatic folding for IF/FOR/WHILE/DO/SUB/FUNCTION/TYPE/CREATE/WITH blocks and `$IFDEF`/`$ENDIF` regions
- **Auto-Indent** — Smart indentation rules for all block structures

### Installation

```bash
cd utilities/vscodeext/rapidr
npx @vscode/vsce package --no-dependencies
code --install-extension rapidr-2.5.0.vsix
```

Or just run `./build_vsc_extension.sh install` from the repo root to build and install in one step. The pre-built `.vsix` is also dropped at the repo root after packaging.

---

## Demo Examples

### Native Demos

Three demo applications showcase the data science components with full GUI integration:

| Demo | Components | Description |
|------|-----------|-------------|
| `demo_plot.rr` | `RPlot` + `RImage` | Generates sine/cosine line plots, bar charts, and pie charts, displays them inside a `RImage` on a form |
| `demo_num.rr` | `RNum` + `RStringGrid` | Array math operations (element-wise, statistics, linspace, dot product) shown in a grid |
| `demo_dataframe.rr` | `RDataFrame` + `RStringGrid` | Loads CSV data, supports sort, filter, group-by, and summary statistics in a grid |

Run any of them:
```bash
./rapidr build --release examples/demo_num.rr
./examples/demo_num
```

> **Note:** The RDataFrame demo expects `examples/demo_dataframe_data.csv` (included) for sample employee data.

### Web Demos

Five web applications demonstrate browser deployment via WASM:

| Demo | Components | Description |
|------|-----------|-------------|
| `hello_web.rr` | RForm, RLabel, REdit, RButton | Basic "Hello Web" with click counter |
| `web_calculator.rr` | RForm, REdit, RButton, RLabel | Full calculator with 17 buttons, display, and history |
| `web_canvas.rr` | RForm, RCanvas, RButton, RLabel | Drawing app with freehand, circles, rectangles, and color picker |
| `web_todo.rr` | RForm, RListBox, REdit, RButton, RCheckBox, RLabel | Todo list with add, remove, clear, and item counter |
| `web_dashboard.rr` | RForm, RTimer, RTabControl, RStringGrid, RComboBox, RProgressBar, RLabel | Live dashboard with clock, tabs, data grid, and progress animation |
| `web_datascience.rr` | RForm, RTabControl, RStringGrid, RNum, RDataFrame, RPlot, RSQLite | Data science demo: array math, DataFrame CRUD, sin/cos plots, bar/pie charts, in-memory SQL |

Run any of them:
```bash
./rapidr bundle-bc examples/web_calculator.rr -o /tmp/calc.zip
unzip -o /tmp/calc.zip -d /tmp/calc && (cd /tmp/calc && python3 -m http.server 8080)
# Open http://localhost:8080
```

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

### End-to-End Matrices

| Test | What it verifies |
|------|------------------|
| [tests/full_matrix.sh](tests/full_matrix.sh) | Compiles every example through every back-end (`compile`, `--interp` native, `bundle-bc` web) |
| [tests/bc_smoke.sh](tests/bc_smoke.sh) | Round-trip: `build-bc` → `run-bc` parity with `compile`+run |
| [tests/web_smoke.mjs](tests/web_smoke.mjs) | In-browser `compile()` produces byte-identical output to the native CLI |
| [tests/web_matrix.mjs](tests/web_matrix.mjs) | CLI bundles every browser-runnable example, navigates Chromium to it, asserts no console errors |
| [tests/web_ide_smoke.mjs](tests/web_ide_smoke.mjs) | The Web IDE boots and Run renders the preview |
| [tests/web_ide_designer.mjs](tests/web_ide_designer.mjs) | Toolbox + property grid generate valid `.rr` source and the design surface renders |
| [tests/web_ide_e2e.mjs](tests/web_ide_e2e.mjs) | Full IDE round-trip: load example → Run → Build → unzip → serve → assert bundle renders match the IDE preview |

The browser tests need a static server at the repo root and Chromium via
Playwright (installed under `tests/node_modules/`):

```bash
python3 -m http.server 8765 &
( cd tests && node web_matrix.mjs && node web_ide_e2e.mjs )
```

---

## Development Conventions

- Transpiler expects strict block encapsulation for multi-line `IF…THEN` (must have `END IF`).
- New GUI components should be added to `gui_create_widget()` in `gui.rs` and registered in `is_component_type_name()` in the codegen.
- Global dependencies are resolved at the start of `SUB` blocks during code generation. Avoid deep nesting that obscures global modification scope.
- The runtime uses thread-local storage for all component state and global variables (`gv()`/`gs()` accessors).

---

## Credits

- **Roberto Berrospe** ([@iBobX](https://github.com/iBobX)) — Creator, architect, and lead developer
- **VS Code Copilot with Claude** — AI pair-programming assistant for feature implementation, testing, and documentation

---

## License

This project is licensed under the [MIT License](LICENSE).

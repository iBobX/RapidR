# RapidR contributor manual

> For people (and AI assistants) working on RapidR itself: how the code is
> organised, the rules it follows, how to build, test and release it. The
> language and its components are described for users in
> [the user manual](docs/manual/README.md); the plans and the reasoning behind
> each subsystem are in [docs/](docs/). Keep this file current when the
> architecture changes.
>
> **Last updated:** 2026-10-06, RapidR 2.117.0.

## Contents

1. [What RapidR is](#1-what-rapidr-is)
2. [Rules the code follows](#2-rules-the-code-follows)
3. [The repository](#3-the-repository)
4. [The compiler front end](#4-the-compiler-front-end)
5. [The two backends](#5-the-two-backends)
6. [Shared models: `rapidr-value`](#6-shared-models-rapidr-value)
7. [The UI kernel and its hosts](#7-the-ui-kernel-and-its-hosts)
8. [The runtimes](#8-the-runtimes)
9. [The CLI, the Runtime and installs](#9-the-cli-the-runtime-and-installs)
10. [Building](#10-building)
11. [Testing](#11-testing)
12. [RapidQ's compiler as the ground truth](#12-rapidqs-compiler-as-the-ground-truth)
13. [Licences and notices](#13-licences-and-notices)
14. [Releasing](#14-releasing)
15. [How to add things](#15-how-to-add-things)
16. [The plan documents](#16-the-plan-documents)

---

## 1. What RapidR is

An original implementation, in pure Rust, of a compiler and runtimes that
are compatible with RapidQ BASIC (William Yu's Rapid-Q, last released 2006)
and extend it. It is not a port or a clone of RapidQ: no RapidQ code is
used; RapidQ's documented and observed behaviour is the specification.

One source program can be:

- **compiled natively**: translated to a Rust project that links
  `rapidr-runtime-core`, then built by cargo (`rapidr build`);
- **interpreted**: compiled to bytecode (`.rrbc`) and run by the VM —
  the RapidR Runtime (`rapidr run`), or a standalone executable that is the
  VM with the program appended (`rapidr build --interp`);
- **run in a browser**: the same VM compiled to WebAssembly with the web
  runtime (`rapidr bundle-bc`), or the native backend targeting
  `rapidr-runtime-web` and wasm32 (`rapidr build --web`).

All three draw windows with RapidR's own UI kernel.

## 2. Rules the code follows

These are project decisions; a change that breaks one needs the
maintainer's agreement first.

1. **RapidQ-exact.** What RapidQ has behaves exactly as in RapidQ — checked
   against RapidQ's compiler (RC.EXE) when the documentation is unclear
   (§12). RapidR's extensions are additive: they never change what an
   existing program does.
2. **One semantics.** Native builds, the interpreter and the web give the
   same results; the conformance suite runs every case on each.
3. **One implementation, no fallbacks.** A behaviour lives in one place —
   usually a shared model in `rapidr-value` used by every runtime. When a
   replacement is solid, the old path is deleted (FLTK and the web's DOM
   host are gone).
4. **Native is native.** A native build never embeds the VM; an interpreted
   build never needs Rust.
5. **No Win32 emulation.** RapidQ's features map to portable code. Windows
   DLL calls are compile errors that name the portable alternative. OLE is
   the one RapidQ object family not implemented.
6. **Permissive dependencies only** (MIT, Apache-2.0, BSD, ISC, Zlib, …;
   `deny.toml`), credited in the notices every build carries (§13).
7. **Never silently wrong.** Every construct compiles correctly or gives a
   diagnostic with a line and column.
8. **Local checks, no remote CI.** GitHub Actions run only by hand; the
   full check is `tools/regress.sh` on the developer's machine and VMs.

## 3. The repository

```
crates/                  the compiler, the runtimes, the UI kernel, the CLI
interpreter/             the bytecode format, generator, VM and its hosts
tests/                   conformance cases, GUI fixtures, browser suites
tools/                   test drivers, ground-truth tools, release scripts
web-ide/                 the web IDE (HTML/JS + Monaco) — to be replaced by the kernel IDE
examples/                example programs (examples/README.md); ide.rr is the desktop IDE
docs/                    plans and references; docs/manual/ is the user manual
design/brand/            logo, icons, banner (original artwork, MIT)
fonts/                   the web's fallback fonts' sources
utilities/vscodeext/     the VS Code extension: a client of rapidr lsp / rapidr dap (VSCODE_EXTENSION.md)
```

| Crate | Role |
|---|---|
| `rapidr-cli` | the `rapidr` command: builds, `run` / `open` / `info` / `ide` (`launch.rs`), finding its home (`home.rs`), `setup` (`setup.rs`), the notices generator (`notices.rs`), macOS universal slices (`macos.rs`) |
| `rapidr-preprocessor` | `$INCLUDE`, `$DEFINE`, `$IFDEF`, `$MACRO`, `$RESOURCE`, `$OPTION ICON`, VB `#If`, the built-in RAPIDQ.INC constants, RapidR's own versions of RapidQ include libraries (`src/libraries/`), Windows-1252 decoding |
| `rapidr-lexer`, `rapidr-parser` | tokens; a recursive-descent parser that reports every bad line |
| `rapidr-ast` | the AST, and the lowering passes both backends run (§4) |
| `rapidr-diagnostics` | spans and diagnostics |
| `rapidr-codegen-rust` | AST → Rust (desktop or web target) |
| `interpreter/rapidr-bytecode` | the `.rrbc` format, the builtin registry (`BUILTINS`) |
| `interpreter/rapidr-bcgen` | AST → bytecode |
| `interpreter/rapidr-vm` | the VM, generic over a `Host` (`#![forbid(unsafe_code)]`) |
| `interpreter/rapidr-vm-host-native` | the VM's host on `rapidr-runtime-core` |
| `interpreter/rapidr-vm-host-web` | the VM and the compiler as one wasm module (`rapidrintr`) on `rapidr-runtime-web` |
| `interpreter/rapidr-compiler-wasm` | the compiler alone as wasm |
| `interpreter/rapidr-webbundle` | the static web bundle (`bundle-bc`) |
| `interpreter/rapidr-runner-stub` | `rapidrintr-runner[w]`: the VM that `--interp` executables are made of |
| `rapidr-value` | the shared `Value` and every shared model (§6) |
| `rapidr-db` | RSQLITE and RMYSQL for every runtime |
| `rapidr-runtime-core` | the desktop runtime (native builds and the native VM host) |
| `rapidr-runtime-web` | the browser runtime |
| `rapidr-ui-kernel` | the GUI-free UI kernel (§7) |
| `rapidr-ui-render` | the kernel's display lists drawn (vello_cpu, or a vello scene for wgpu) |
| `rapidr-ui-app` | the program's side of the kernel, host-neutral: events, windows, the modal list, timers, waits, menus, dialogs' requests, the GUI test hooks |
| `rapidr-ui-host-winit` | the desktop host: winit, vello on the GPU or vello_cpu, AccessKit, system menus and file dialogs, a headless host for tests |
| `rapidr-ui-host-web` | the web host: canvases, DOM input and input methods, the ARIA mirror |
| `rapidr-launcher` | `rapidrw`: Windows' windowed launcher, the macOS apps' executable |
| `crates/patches/wayland-protocols-plasma` | RapidR's own MIT replacement of a crate generated from LGPL protocol files (§13) |
| `rapidr-buildserver` | legacy build server of the old `examples/web_ide.rr`; not used by anything shipped |
| `rapidr-ui-proto` | the desktop host's first prototype (excluded from the workspace) |

## 4. The compiler front end

```
source ─ preprocessor ─ lexer ─ parser ─ AST ─ rapidr_ast lowering passes ─┬─ codegen-rust → Rust project → cargo
                                                                            └─ bcgen → .rrbc → VM + host
```

- The preprocessor returns the text, a line map (errors in included code
  point at the include file and line), the app type and the `$RESOURCE`s.
  A first `#!` line is dropped.
- Identifiers are case-insensitive; type suffixes (`$ % & ! # ? ?? ???`)
  are part of a name and give its type.
- **Lowering passes in `rapidr-ast`**, run by both backends so they can't
  disagree: `objects` (TYPEs with methods, constructors, events and
  inheritance → plain routines and slots), `library` (RapidQ library TYPEs
  RapidR implements, `ENVIRON`), `memory` (VARPTR & co.), `numeric` (stores
  into declared numeric types), `tray_calls`, `type_values`,
  `suffix_routines`, `suffix_vars`, `for_locals`, `array_refs`,
  `stream_arrays`, `implicit_scope`, `create_property_reads`, … Each file
  documents the RapidQ rule it implements.
- `rapidr_ast::COMPONENT_TYPES` is the list of components both backends
  create — the language registry's; `canonical_type_name` maps RapidQ's Q
  names to R names.
- The language registry (`crates/rapidr-lang`, IDE stage I0,
  [docs/ide-plan.md](docs/ide-plan.md)): every component, member, builtin,
  statement, directive, constant and keyword, with RapidQ / RapidR origin,
  types, defaults and docs, in `crates/rapidr-lang/data/*.toml` (its
  `src/lib.rs` says how to edit them). `rapidr lang export --all` writes
  what it generates (the manual's reference pages, the VS Code extension's
  and the web IDE's language data); `rapidr lang export --json` /
  `--prompt` give the IDE's completion data and the AI prompt's language
  section. Tests tie it to the code: `cargo test -p rapidr-lang` (the
  compilers' lists, BUILTINS, the lexer's and parser's keywords, the
  preprocessor's directives and constants, members' value rule, generated
  files current), `tools/lang_dispatch.py --check` (every name the
  runtimes' dispatch answers is in it), `tests/lang_conformance.mjs` (a
  program per component on every runtime).

## 5. The two backends

### Native (`rapidr-codegen-rust`)

Generates `src/main.rs` and a `Cargo.toml` that points at the runtime crates
(a checkout's `crates/`, or an install's home). Module-level variables are
slots in a thread-local vector (`gv(i)` / `gs(i, v)`); typed locals stay
Rust numbers where they can; GOTO / GOSUB lower to a state machine
(`jumps.rs`); `RUSTSTART` blocks are pasted in. The generated program uses
`rapidr_runtime_core::prelude` (or the web runtime's) and calls the same
shared functions the VM's hosts call.

### Bytecode (`interpreter/`)

- **Format 3** (`rapidr-bytecode`): `RRBC`, the format, flags, then a
  header every later format keeps — its length, `MIN_RUNTIME` (the oldest
  runtime that runs the program, 3 × u16) and the app type. Runtimes read
  formats 2 and 3; a newer program is refused with "this program needs
  RapidR Runtime x.y.z or newer". **Raise `MIN_RUNTIME`** whenever the
  compiler starts writing something older runtimes don't know (an opcode,
  a builtin). Decoding checks every count against the bytes (hostile files
  can't exhaust memory).
- `BUILTINS` lists every builtin; a unit test checks it equals each host's
  dispatch table, and the generator rejects unknown names at compile time.
- The VM runs events only when the program waits (ShowModal, DOEVENTS, a
  dialog, the end of the main program), queued — never re-entrantly. On the
  web it runs in ~10 ms slices so busy loops don't freeze the page.
- The interpreter refuses `DECLARE … LIB` and `RUSTSTART` with a clear
  message.

## 6. Shared models: `rapidr-value`

Where RapidQ behaviour is implemented once for every runtime: `Value`;
strings, numbers and formatting (`strings`, `numeric`, `format` —
RapidQ's PRINT and STR$ rules); `builtins` (the string / math builtins);
`basic_files` (OPEN / PRINT # / INPUT #); `objects::*` (QFONT, QBITMAP,
streams, image lists, lists, grids, trees, list views, headers, tab
controls, text editing, menus, the printer, DirectX and Direct3D (`d3d/`),
joysticks, media, AVI and MIDI decoders, the synthesizer, CGI, serial ports,
downloads, glass frames, bevels, digit displays, accessibility nodes, drawing
ops); `layout` (Align, Anchors, constraints); `theme`; `registry`
(QREGISTRY's per-user store and Windows' registry); `globals` (Screen,
Application, Clipboard, Mouse); `events`, `input`; `dialogs`, `mdi`,
`window_state`, `tray`, `memory`, `component_defaults`, `members`
(method-or-property reads); `datascience` (RNUM, RDATAFRAME, RPLOT: arrays,
frames with their CSV / JSON readers and printed form, the charts' model —
a runtime adds only PRINT, grids and drawing through `datascience::Host`).

A runtime's job is to store components, call these models, and pass input
and drawing to its host.

## 7. The UI kernel and its hosts

- **`rapidr-ui-kernel`** (builds for wasm too; checked in `regress.sh
  unit`): forms as retained trees over the component store; each component
  kind (`src/components/`, registered in `KINDS`) paints into a display
  list, hit-tests, handles keys and mouse in RapidQ's event order, and
  describes itself for accessibility; focus with TabOrder / TabStop; parley
  text and editors with IMEs; kernel-drawn dialogs (message boxes, colour
  and font dialogs); themes (classic by default, modern, dark, high
  contrast).
- **`rapidr-ui-render`** draws a display list: vello_cpu, or a vello scene
  for wgpu. Both hosts use it, so desktop and web pixels match.
- **`rapidr-ui-app`**: the program's side, behind two traits (`Program`,
  `Windows`): kernel events as the program's events, the modal list, the
  timer heap, waits (ShowModal, DOEVENTS, dialogs, INPUT$), menus, the GUI
  test script and hooks (`testhooks.rs`).
- **Desktop host** (`rapidr-ui-host-winit`): winit pumped from the
  program's own loop (`pump_app_events`), vello on wgpu or vello_cpu
  (`RAPIDR_RENDERER`; software-only GPUs such as WARP and llvmpipe get the
  CPU), AccessKit, the macOS menu bar, rfd's Open / Save dialogs, arboard's
  clipboard, a headless host for tests. Plan and results:
  [docs/desktop-host-plan.md](docs/desktop-host-plan.md).
- **Web host** (`rapidr-ui-host-web`), the web's only host: windows drawn
  into canvases at `devicePixelRatio`, input and IMEs through the DOM, an
  ARIA mirror of the kernel's tree, web-only components as page elements
  over the canvas (`overlay_web.rs`), Noto fallback fonts loaded by Unicode
  range. Plan and results: [docs/web-host-plan.md](docs/web-host-plan.md).

Coordinates are RapidQ's logical pixels everywhere; everything is drawn at
the device scale.

## 8. The runtimes

- **`rapidr-runtime-core`** (desktop): `object.rs` (the component store and
  `rp_comp_*` dispatch), `builtins.rs`, `ui/` (the facade over the kernel
  host: `kernel.rs`, `kernel_store.rs`, `program.rs`), `datascience.rs`
  (the shared data-science model's desktop side: PRINT, grids, plotters
  charts), `network.rs` (sockets, RHTTP over the
  system's TLS), `io.rs` / `serial.rs`, `media.rs`, `sound.rs` (rodio,
  nanomp3), `directx.rs`, `joystick*`, `ffi.rs` (DLL calls, native builds
  only), `terminal.rs`. Features (all on by default): `database`, `network`, `gui`, `datascience`, `audio`, `ffi`, `gamepad`.
- **`rapidr-runtime-web`**: the same API for wasm — `object_web.rs`,
  `kernel_web.rs` (the kernel host, the VM's waits), `overlay_web.rs`,
  `dialog_web.rs`, `database_web.rs` (SQLite in wasm), `network_web.rs`
  (fetch, WebSocket), `datascience_web.rs` (the shared model's web side: charts on a canvas),
  `webapi_web.rs` (web-only components), `fonts_web.rs`, `tray_web.rs`.

## 9. The CLI, the Runtime and installs

- `home.rs`: an install's home is found by one rule — `RAPIDR_HOME`; else
  `<exe>/../lib/rapidr` with its `release.toml`; else the source checkout.
  An install's home is laid out like a checkout for what builds read.
- `launch.rs`: `run`, `open` (the desktop's double click: console programs
  get a terminal on macOS / Linux; downloaded files — quarantine, Mark of
  the Web, Linux's origin xattr — ask once, remembered by SHA-256 in
  `trusted-files.txt`), `info`, `ide`, `about`.
- `setup.rs`: Rust for native builds through rustup, never changing the
  user's default toolchain (unit-tested); the PATH link.
- `notices.rs`: `rapidr notices` and the `THIRD-PARTY-NOTICES.txt` every
  build writes.
- Windows native builds use the `*-pc-windows-gnullvm` toolchain with the
  LLVM-MinGW the SDK ships; `RAPIDR_TOOLCHAIN=msvc` for Microsoft's.
- File associations, the macOS apps, the Linux MIME types and `.desktop`
  files: [docs/release-packaging.md](docs/release-packaging.md).

## 10. Building

```sh
cargo build --release -p rapidr-cli && cp target/release/rapidr .   # the CLI (./build.sh does the same for the workspace)
tools/build_web_artifacts.sh       # the web interpreter → target/web (wasm-pack; wasm SIMD; target/wasm-simd)
python3 tools/fonts.py fetch       # the CJK fallback fonts' sources, once (pinned by SHA-256)
```

- macOS: Xcode's command-line tools. Linux: `build-essential pkg-config
  libfontconfig1-dev libasound2-dev libssl-dev` (HTTPS uses the system's
  OpenSSL 3, linked dynamically — never vendored). Windows: MSVC Build
  Tools or LLVM-MinGW.
- wasm-bindgen is pinned to 0.2.129 (workspace and generated web projects).
- A checkout builds its own `--interp` runners with cargo; native builds of
  programs reuse `target/`. Native builds of the full runtime take room:
  the test runners delete each one after it ran.

## 11. Testing

**`tools/regress.sh`** is the full local check before a commit. Stages
(`tools/regress.sh conformance gui` runs only those):

| Stage | What |
|---|---|
| `unit` | `cargo test --workspace` (the language registry's generated files current among it); the kernel, `rapidr-ui-app` and `rapidr-lang` built for wasm32; the registry's reverse check (`tools/lang_dispatch.py --check`) |
| `conformance` | `tests/conformance/run.mjs`: every case in `tests/conformance/cases/` natively and interpreted (`.expected`, `.expected-error`, `.expected-runtime-error`, `.input`; `' xfail:` markers); `tests/lang_conformance.mjs`: the registry's program per component, interpreted and native (known default gaps: `tests/lang/gaps.txt`) |
| `examples` | `tools/native_examples.sh`: the examples build natively |
| `gui` | `tests/native_gui_events.mjs`: the GUI fixtures (`tests/fixtures/*.bas`) on the kernel's headless host, native and interpreted, at 1× and 2× — events, dumps, captures, accessibility trees, themes |
| `web` | the conformance suite in Chromium; the desktop's captures (`tests/gui_captures.mjs`) against the web host's windows and accessibility trees at 1× and 2× (`web_gui_parity.mjs`, `web_a11y.mjs`); the web IDE and bundle suites |
| `legal` | `cargo deny check licenses bans`, `tools/third_party_notices.py --check`, `tools/check_notices.py` (builds a program each way and checks its notices list every crate) |

- Needs `./rapidr` built, the web artifacts, and the repository served on
  port 8765 (`python3 -m http.server 8765 --bind 127.0.0.1`) for the web
  stage. The script points `RAPIDR_PRINT_TO` and `RAPIDR_REGISTRY` at
  scratch files: **tests never print on a real printer or touch the real
  registry** — set both yourself whenever you run programs by hand.
- GUI test hooks (`crates/rapidr-ui-app/src/testhooks.rs`):
  `RAPIDR_CAPTURE=<prefix>` (run, fire the events, dump, save every window
  as BMP, exit), `RAPIDR_TEST_EVENTS=b1.onclick,…`, `RAPIDR_TEST_DUMP=
  lbl.caption,…`, `RAPIDR_TEST_A11Y`, `RAPIDR_SCALE`, scripted dialogs
  (`RAPIDR_TEST_FILE_DIALOG`, `…_COLOR_DIALOG`, `…_FONT_DIALOG`,
  `…_MESSAGE_DIALOG`, `RAPIDR_TEST_DIALOG_HOLD`), devices
  (`RAPIDR_TEST_COMPORT`, `…_JOYSTICK`, `…_MIDI`, `…_WAVE_IN`, `…_SOUND`,
  `…_CLIPBOARD`), `RAPIDR_CAPTURE_WINDOWS=1` (real windows instead of the
  headless host).
- Real input on macOS (`tools/real_input.py`), what screen readers see
  (`tools/macos/ax_dump.swift`, `tools/windows/uia_probe.ps1`).
- Other systems: `tools/linux/check.sh` (the committed tree on Ubuntu in
  Docker, headless and on X11); the Windows 11 and Ubuntu Parallels VMs
  (conformance, GUI events with real windows, `tools/windows/
  registry_check.ps1`).
- RapidQ's corpus: `tools/rapidq_corpus.py` (classifies RapidQ's 386
  examples), `tools/corpus_compare.mjs` (native against interpreted),
  `tools/corpus_native.sh`.
- Fixtures' media: `tools/make_avi_fixtures.py`, `make_media_fixture.py`,
  `make_dx_fixture.py`, `make_digit_bitmaps.py`.

## 12. RapidQ's compiler as the ground truth

RC.EXE runs in the Windows 11 VM. `tools/rc_probe.sh` compiles and runs a
folder of probe programs there and returns RapidQ's exact output;
`tools/rapidq_truth.py` runs the conformance cases, RapidQ's console
examples or a probe folder through RC.EXE and RapidR and lists every
difference (`--write-expected` pins RapidQ's output as a case's
`.expected`). Programs that could print or touch the registry are never run
in the VM. RC.EXE's error texts: `.reference/rapidq-compiler-messages.txt`
(local, not in git). Details, and every judgement call made so far:
[docs/rapidq-ground-truth.md](docs/rapidq-ground-truth.md).

## 13. Licences and notices

- `deny.toml` allows only permissive licences, for the whole workspace, and
  bans the crates RapidR replaced (rustls / ring / webpki-roots for HTTPS,
  symphonia for MP3, font-kit for charts, crates.io's
  `wayland-protocols-plasma`).
- `THIRD_PARTY_NOTICES.md` (generated: `python3 tools/third_party_notices.py`)
  and `LICENSES.md` (vendored JavaScript, fonts, C code) cover the
  repository.
- Every build writes `THIRD-PARTY-NOTICES.txt` from its real dependency
  graph (`crates/rapidr-cli/src/notices.rs`); an install ships them per
  target in `lib/rapidr/notices/`.
- Details: [LEGAL.md](LEGAL.md), [docs/licensing.md](docs/licensing.md).
  New dependencies: permissive, credited, and the `legal` stage must pass.

## 14. Releasing

Each verified change is committed to `development` with a version bump
(`Cargo.toml`'s workspace version), a `CHANGELOG.md` entry and ROADMAP
ticks. Installers are built locally — on the Mac and in the VMs, never by
remote CI — by `tools/release/`: `prepare.sh` (the source archive, the web
interpreter, the IDE's bytecode, the web bundle, the SBOM, the release
notes from `docs/release-notes-template.md`), `macos.sh`, `linux-vm.sh`,
`windows-vm.sh`, `smoke.sh`, `finish.sh` (licence files, `SHA256SUMS`).
`stage.py` lays out an install (it ships `docs/manual/` in every SDK, under
`share/doc/rapidr/manual/`); `home.py` the runtime's sources and vendored
crates. Publishing (`gh release create`) is done by hand. Step by step:
[docs/release-packaging.md](docs/release-packaging.md).

## 15. How to add things

**A builtin**: implement it once (in `rapidr_value::builtins` when it's the
same everywhere); add its name to `interpreter/rapidr-bytecode/src/
builtins.rs` `BUILTINS` and to each host's dispatch (the unit test fails
otherwise) — `rapidr-vm-host-native`, `rapidr-vm-host-web`; emit it in
`rapidr-codegen-rust` (`builtin_function_call`) for native builds and the
web runtime's `builtins.rs`; a conformance case; raise `MIN_RUNTIME`; its
entry in `crates/rapidr-lang/data/builtins.toml` (`cargo test -p
rapidr-lang` fails otherwise), then `rapidr lang export --all`.

**A component**: its model in `rapidr-value/src/objects/` (state, drawing
ops, keys, accessibility); its kind in `rapidr-ui-kernel/src/components/`
(registered in `KINDS`); its entry, members and docs in
`crates/rapidr-lang/data/components/` (which makes it one of
`COMPONENT_TYPES`), then `rapidr lang export --all`; defaults in
`rapidr_value::component_defaults`; the desktop and web runtimes'
dispatch; a GUI fixture in `tests/fixtures/` with a case in
`tests/gui_parity_cases.mjs` (it then runs native, interpreted and in the
browser).

**A statement or syntax**: lexer, parser, AST; a lowering pass in
`rapidr-ast` when both backends can share it; otherwise both `bcgen` and
`codegen-rust`; conformance cases, with RC.EXE's output when RapidQ has it.

**A directive**: `rapidr-preprocessor`; document it in
`docs/manual/language.md`.

**Docs**: user-visible changes go into `docs/manual/` and CHANGELOG.md; the
reference pages are regenerated, never edited.

## 16. The plan documents

| Document | |
|---|---|
| [docs/rapidq-ground-truth.md](docs/rapidq-ground-truth.md) | RC.EXE in a VM: the tools and every decision taken from it |
| [docs/desktop-host-plan.md](docs/desktop-host-plan.md) | from FLTK to the UI kernel on the desktop: architecture and stage results |
| [docs/web-host-plan.md](docs/web-host-plan.md) | the kernel in the browser: stages W1–W11 |
| [docs/directx-plan.md](docs/directx-plan.md) | QDX* and QD3D*: what each maps to, the rasterizer, the judgement calls |
| [docs/io-media-plan.md](docs/io-media-plan.md) | QCGI, QCOMPORT, QDOWNLOAD, QMIDI, QWAVE, QVIDEO, QCDAUDIO |
| [docs/ide-plan.md](docs/ide-plan.md), [ide-components.md](docs/ide-components.md), [ide-ai.md](docs/ide-ai.md) | RapidR Studio: stages I0–I9, its public components, AI and MCP |
| [docs/q-and-r-components.md](docs/q-and-r-components.md) | how RapidQ's and RapidR's component names coexist |
| [docs/licensing.md](docs/licensing.md) | what each output contains, licence by licence |
| [docs/release-packaging.md](docs/release-packaging.md), [release-notes-template.md](docs/release-notes-template.md) | installs, file types, building and publishing a release |
| [ROADMAP.md](ROADMAP.md) | the living plan: phases, findings, the timeline |

# RapidR Roadmap

> Goal: the best RapidQ / VB-style compiler, interpreter, debugger and IDE ever —
> compatible, secure by default, AI-native, and open source.
>
> Created 2026-09-24 from a codebase review at v2.8.1 (`development` @ `d458126`).
> Keep this file current: tick boxes as work lands, add findings as they are discovered.

---

## Guiding principles

1. **Never silently wrong.** Every construct either compiles correctly or produces a clear diagnostic with line/column. No dropped statements.
2. **One semantics.** The bytecode VM is the reference implementation. The Rust codegen backend must produce identical output on every conformance program (differential testing in CI).
3. **One source of truth.** A single language registry (components, properties, methods, events, builtins, signatures, docs) generates the IDE completion data, the VS Code extension data, the manual, and the AI system prompt. Today `web-ide/lang-data.js` and `utilities/vscodeext/rapidr/src/languageData.js` are maintained separately.
4. **Secure by default.** Untrusted code, imported projects, and AI output never get more privilege than they asked for.
5. **Measured compatibility.** A public "% of real RapidQ/VB corpus that compiles and runs" number, tracked over time.

Status: open source (MIT). Monetization is explicitly deferred. Possible future direction: build our own apps with RapidR (also the best dogfooding).

---

## Baseline findings (2026-09-24)

### Compatibility / correctness (verified by running programs, see Appendix A)

> **All fixed** — re-checked on both backends in v2.55.0 (`tests/conformance/cases/early_findings.bas`, `builtins_manual.bas`); kept as history.

| Construct | Result |
|---|---|
| `$INCLUDE "RAPIDQ.INC"` | Hard error — file not found |
| `QFORM`, `QBUTTON`, … | Compile, but runtime only matches `"RFORM"` etc. → nothing created |
| `? "text"` | Lexer error `Unexpected character: ?` |
| `GOSUB` / `GOTO` / labels / `RETURN` | **Silently dropped** — wrong execution order, program exits early |
| `INC i` / `DEC i` | Silently ignored |
| `TYPE … EXTENDS QFORM` + `EVENT … END EVENT` | No `EVENT` keyword; compiles to 1 fn with no error |
| `DECLARE SUB` | bcgen warning "statement not yet lowered: Declare" |
| `REPLACESUBSTR$("aXbX","X","-")` | Returns empty string (VM) |
| `INSERT$("XY","hello",2)` | Wrong result (VM) |
| `FORMAT$("%05d", 42)` | `42` instead of `00042` |
| `STRF$(3.14159, 0, 5, 2)` | Ignores format args |
| `PRINT a, b, c` | No tab zones between items |
| Parser | Tolerant — 13-line program parsed to 9 statements with no diagnostics |

Working: `IIF`, `FIELD$`, `TALLY`, `RINSTR`, `CONVBASE$`, `HEX$`, `CHR$`, `ASC`, `MID$`, `UCASE$`, `LTRIM$`, `DELETE$`, `REVERSE$`, `CREATE … END CREATE` with R-types, `DEFINT`, single-line `IF … THEN … ELSE`, `:` separators.

| ~~`MID$` / `LEFT$` / `RIGHT$` byte slicing → crash on `"héllo"`~~ | fixed v2.11.0 (shared char-based `rapidr_value::strings`) |

**Conformance suite findings (2026-09-24, `tests/conformance`, both backends):**

| Bug | VM | Codegen |
|---|---|---|
| ~~Identifiers (variables, SUBs) case-sensitive~~ fixed v2.10.0 | ok | ok |
| Bare calls inside CREATE (`Center`) dropped → forms never centered (found v2.10.0, fixed) | ok | ok |
| ~~`FOR … STEP -n` runs zero times~~ fixed v2.10.1 | ok | ok |
| ~~`CASE 2, 3` jumps to program start → infinite loop~~ fixed v2.10.1 | ok | ok |
| ~~`CASE a TO b` / `CASE IS > n` wrong branch~~ fixed v2.10.1 | ok | ok |
| ~~Interpreter arrays were comma-separated strings (DIM didn't allocate, no 2-D, commas corrupt); codegen 1-D only, LBOUND/UBOUND broken~~ real `Value::Array` on both, fixed v2.11.0 | ok | ok |
| ~~FUNCTION return-by-name returns empty~~ fixed v2.10.1 | ok | ok |
| ~~`BYREF` ignored (VM) / doesn't compile (codegen)~~ fixed v2.10.1 | ok | ok |
| ~~Whole-number float results print as `1024.0`~~ fixed v2.10.1 | ok | ok |
| ~~`;`/`,` in PRINT wrong on both~~ separators + 14-col zones, fixed v2.11.0 | ok | ok |
| GOTO/GOSUB/labels: VM done v2.12.0; codegen refuses with a clear error (needs state-machine lowering) | ok | ✗ |
| ~~Bare `END` silently ignored (program ran into its subroutines)~~ fixed v2.12.0 | ok | ok |
| ~~EXIT SUB/FUNCTION only left loops; EXIT FOR in nested WHILE left the WHILE~~ fixed v2.10.1 | ok | ok |
| Calls to unknown SUBs / builtins: ~~VM silent no-op~~ compile error (v2.10.0); codegen confusing rustc error | ok | ✗ |

Open questions — settled in v2.55.0 from the manual: `INSERT$(insert, source, index)`; numbers print as Delphi's `FloatToStr` (15 significant digits: `0.1 + 0.2` → `0.3`), which RapidQ's `FORMAT$` / `STRF$` (Delphi `Format` / `FloatToStrF`) point to; both backends print `x:5` (no leading space). Still unconfirmed without a real RapidQ: the 14-column PRINT zone width.

Other notes: ~84 unit tests for ~40k LoC; no cross-backend conformance tests.

### Security (found by code reading — not yet exploited/verified dynamically)

| ID | Sev | Issue | Location | Fix |
|---|---|---|---|---|
| SEC-01 | **Critical** | Legacy build server binds `0.0.0.0`, `CorsLayer::permissive()`, compiles arbitrary source incl. `RUSTSTART` via cargo → RCE from LAN or any visited website | `crates/rapidr-buildserver/src/main.rs:105`, `:112` | Delete crate, or bind `127.0.0.1` + random token + origin allowlist |
| SEC-02 | High | Preview iframe `sandbox="allow-scripts allow-same-origin"` neutralizes the sandbox; app code runs with IDE origin (can read future API keys, modify IDE) | `web-ide/index.html:253` | Serve preview from a separate origin; postMessage-only bridge |
| SEC-03 | High | `message` listeners don't check `e.source`/origin; `postMessage(..., "*")` | `web-ide/host.js:4550`, `web-ide/preview.html:58` | Verify `e.source === iframe.contentWindow` + per-session nonce; explicit targetOrigin |
| SEC-04 | High | JS injection: `RStringList.SaveToFile` escapes `"` but not `\` before `eval` (payload `\");alert(1);//`) | `crates/rapidr-runtime-web/src/object_web.rs:1310` | Use `web_sys` Blob + anchor, no eval |
| SEC-05 | Medium | `Caption`/`Text` set via `innerHTML` for non-label elements → XSS when showing DB/HTTP/AI data | `crates/rapidr-runtime-web/src/gui_web.rs:235`, `:604` (15 innerHTML sites total) | `textContent` by default; explicit `.HTML` property for markup |
| SEC-06 | Medium | 6 `js_sys::eval` sites with string-built JS. **Also a functional bug:** bundle CSP (`script-src 'self' 'wasm-unsafe-eval'`) blocks eval, so `RHttp`, `Sound`/`Beep`, `SaveToFile` likely fail in deployed bundles; `connect-src 'self'` blocks external APIs; `frame-src 'none'` blocks `RWebView` | `network_web.rs:58`, `:125`; `builtins.rs:550`, `:569`; `object_web.rs:1314`; `gui_web.rs:1269`; CSP in `web-ide/zip.js:91` | Replace with `web_sys` calls; clippy `disallowed_methods` for eval outside `RJavaScript`; derive CSP from components used |
| SEC-07 | Medium | SQL APIs take raw strings, no parameter binding (`query_map([], …)`) → SQLi by default | `crates/rapidr-runtime-core/src/database.rs:103` (+ MySQL path, web DB) | Add parameter binding API (`?` placeholders + `.AddParam`/array arg); document; teach AI |
| SEC-08 | Review | ~~23 `unsafe` in web host; event dispatcher stores leaked raw `*mut Vm` → possible aliasing/UB on re-entrant events~~ fixed in v2.30.0: events are queued and run by the VM itself; the VM and both hosts `#![forbid(unsafe_code)]`. Remaining: 68 `unsafe` in FFI | `crates/rapidr-runtime-core/src/ffi.rs` | Document invariants; Miri |
| SEC-09 | Gap | ~~No cargo-deny~~ (done v2.8.4); no fuzzing, no SECURITY.md; IDE loads Google Fonts (third-party) | — | Phase 0 + Phase 6 |
| SEC-11 | High | Build server preview path traversal: `dir.join(rel).starts_with(dir)` does not catch `..` → arbitrary file read | `crates/rapidr-buildserver/src/main.rs` (`serve_preview_path`) | Reject any non-`Normal` path component |
| SEC-12 | Medium | `RWebView.HTML` uses `srcdoc` with default sandbox `allow-scripts allow-same-origin` → HTML runs with the app's origin | `crates/rapidr-runtime-web/src/gui_web.rs` (`"html"` prop, iframe creation ~:2201) | Drop `allow-same-origin` for `srcdoc` content by default; opt-in property |
| SEC-13 | Low | IDE preview: `RHttp` to relative URLs needs CORS headers from the IDE's server now that the preview is opaque-origin; origin-bound browser APIs (notifications) may be unavailable in preview | `web-ide/preview.html` | Document; optionally proxy same-server requests through the IDE bridge (needs async RHttp) |
| SEC-10 | Gap | `$INCLUDE` resolves arbitrary paths; projects with `RUSTSTART` / `DECLARE … LIB` get no warning on open/build | `crates/rapidr-preprocessor/src/lib.rs` | Confine includes to project root in IDE/MCP contexts; "native-privileged" project flag + confirmation |

### IDE / debugger baseline

Has: Monaco editor, regex-based completion/hover/signature help, visual designer, debugger (breakpoints, step in/over/out, stack, variables, watch list, component properties), assets manager, zip build, themes.
Missing: compiler diagnostics as editor markers, ~~undo/redo~~ (done v2.9.0), immediate-window evaluation (stubs), conditional breakpoints/logpoints, native/DAP debugging. `web-ide/host.js` is a 4,587-line monolith.

---

## Phase 0 — Stabilize & secure (~3 weeks)

**Sprint 1 (~2 weeks)**
- [x] SEC-01 + SEC-11: `rapidr-buildserver` locked down — binds 127.0.0.1 (override `RAPIDR_BUILDSERVER_HOST` warns), Host/Origin loopback guard (blocks cross-site + DNS rebinding), CORS loopback-only, path traversal fixed; unit tests + live curl probes (v2.8.2)
- [x] SEC-02 / SEC-03: preview iframe runs without `allow-same-origin` (opaque origin), runtime shipped in at boot, private `MessageChannel` handshake instead of window messages; `tests/web_ide_preview_isolation.mjs` (fails 11/15 on 2.8.2, passes on 2.8.3) (v2.8.3)
- [x] SEC-04: `SaveToFile` now uses shared `trigger_download` Blob helper (no eval); wasm check passes — verified in Chromium with and without bundle CSP (v2.8.2)
- [x] SEC-05: Caption/Text fallback and `PRINT` → `#rr-console` now plain text; markup only via `RDOM.InnerHTML` / `RWebView.HTML` (v2.8.2)
- [x] SEC-06: `RHttp`, `BEEP`, `SOUND` via `web_sys` (XHR / Web Audio); only `RJavaScript.Eval` remains; `clippy.toml` bans `js_sys::eval` (v2.8.2)
- [x] Unsupported constructs → hard diagnostics: parser errors with line/col for every bad line; bytecode compiler rejects unknown SUB/FUNCTION names (shared builtin registry) and statements it can't run; no catch-all arm left (v2.10.0). Codegen still reports unknown calls only via rustc.
- [x] IDE diagnostics: squiggles via `setModelMarkers` in the right form/module, clickable Errors panel, live checking while typing (v2.10.0; errors travel as `line:col: error:` text — move to a structured wasm API when the language service lands)
- [x] `tests/conformance/` harness (`run.mjs`) — now 34 pass / 4 known failures (all codegen): `*.bas` + `*.expected` / `*.expected-error`, VM **and** Rust codegen, xfail markers for known bugs, runs in CI — 15 seed cases, 7 pass / 23 known failures
- [x] Fix conformance failures (table above) until every case passes on both backends (all pass; re-verified v2.55.0)
- [x] `cargo-deny` (advisories, licenses, bans, sources) + `.github/workflows/ci.yml` (deny, workspace tests, eval lint); 5 vulnerable crates patched; native build fixed on Rust 1.98 (`ethnum`) (v2.8.4)
- [x] Undo/redo in the IDE: snapshot-based project history, menu/toolbar/Ctrl+Z/Ctrl+Shift+Z/Ctrl+Y, 100 steps, `tests/web_ide_undo.mjs` (v2.9.0)
- [x] `tests/web_ide_bugfixes.mjs` and `tests/web_ide_phaseF.mjs` pass; every `tests/web_ide_*.mjs` runs in `tools/regress.sh` (local; CI is manual-only) (v2.71.0)

**Rest of Phase 0**
- [ ] Upgrade `mysql` crate to drop `proc-macro-error2` (unmaintained, future-incompatible: will stop compiling on a future Rust like `ethnum` did)
- [ ] Confirm first CI run on GitHub (Linux FLTK/ALSA system packages untested)
- [x] `SECURITY.md` → GitHub private vulnerability reporting (repo setting must be enabled by owner)
- [x] Track all of `tests/` in git (generated outputs ignored)
- [ ] Run the web IDE Playwright suites in CI (wasm-pack build + static server + Playwright)
- [ ] SEC-07: SQL parameter binding (SQLite, MySQL, web SQLite)
- [ ] CSP generated per bundle from components used (e.g. `'unsafe-eval'` only if `RJavaScript` is used; `connect-src` for `RHttp`/`RAI` hosts; `frame-src` for `RWebView`)
- [ ] Single language registry → generate `lang-data.js`, VS Code data, manual sections

## Phase 1 — RapidQ & VB compatibility (~6–8 weeks)

**Direction (2026-09-25):** no Windows-only compatibility. RapidQ's own
features (built-ins, components, console, dialogs) are mapped to portable
Rust implementations on desktop and web. Calls into Windows DLLs stay a
clear error (naming a RapidR equivalent when one exists). Any library we
add must be open source with a permissive license (`deny.toml`) and be
credited: `THIRD_PARTY_NOTICES.md` (generated, checked in CI), README and
the IDE's About dialog.

Next up, in order:
- [x] `THIRD_PARTY_NOTICES.md` generated from the real dependency graph (`tools/third_party_notices.py`, `--check` in CI); linked from README, LICENSES.md and the IDE About dialog; shipped in every web bundle; native C/C++ libraries credited in LICENSES.md §7 (v2.16.1)
- [x] Windows DLL calls: error says RapidR doesn't emulate Windows and names the portable equivalent (SHELL, RCANVAS, RSQLITE, RSOCKET, …) (v2.17.0)
- [x] Console: `CLS`, `COLOR`, `LOCATE`, `CSRLIN`, `POS` as ANSI sequences on both backends; the IDE Output panel renders them (web-ide/ansi_screen.js) (v2.17.0)
- [x] Web bundles: an on-page console for programs that PRINT (`web-ide/bundle_console.js` + `ansi_screen.js`, CLI and IDE bundles) (v2.21.0)
- [x] Omitted arguments (`INSTR(, a, b)`, `COLOR , 1`); builtins without parentheses (`TIMER`, `CSRLIN`, …) in the VM too (v2.17.0)
- [x] `REDIM` keeping data (resized in place; creates the array without a DIM; `REDIM PRESERVE`), `INV` (v2.18.0)
- [x] Dialogs with buttons: `MESSAGEBOX`, `MESSAGEDLG` (FLTK dialogs on desktop; browser alert/confirm on the web, which can't offer a third button) (v2.19.0)
- [x] Web: in-page dialogs with any buttons for MESSAGEBOX/MESSAGEDLG/SHOWMESSAGE/INPUT — the VM suspends and resumes (`VmError::Suspended`, `Vm::resume_with`) (v2.22.0)
- [x] `INPUT` per the manual: prompt printed, whole line, stored as text/number by DIM type or suffix (both backends) (v2.22.0)
- [x] `SLEEP` in seconds (as RapidQ), `DOEVENTS` (the desktop runs FLTK's pending events and timers; the web pauses the program so the browser goes on), `INKEY$` (QBasic's keys: the terminal, the program's windows, the page), timers enabled again ticking again, on native, interpreter and web (v2.80.0)
- [x] `INPUT$(n)`: returns on the n-th key, no polling (terminal poll, window event loop, page keydown) (v2.83.0)
- [ ] The Rust-compiled web build's waits (it has no suspension: browser dialogs, SLEEP doesn't wait)
- [x] RapidQ objects QFONT, QBITMAP, QIMAGELIST, QMEMORYSTREAM, shared by both runtimes (`rapidr_value::objects`); `Canvas.Draw`; desktop fonts, canvas-relative drawing, left-aligned labels; `RAPIDR_CAPTURE` window capture for checking desktop rendering (v2.20.0)
- [x] Arrays of components and indexed sub-objects (also inside CREATE), both backends (v2.26.0)
- [x] Components render their indexed sub-objects: QSTATUSBAR panels and QLISTVIEW columns/items/sub-items (shared model in `rapidr_value::objects::listview`), desktop and web (v2.29.0)
- [x] QLISTVIEW as RapidQ has it, drawn by the shared model on native, interpreter and web: vsIcon / vsSmallIcon / vsList / vsReport with LargeImages / SmallImages / StateImages, CheckBoxes, MultiSelect (Ctrl / Shift / Ctrl+A), SortType stText, RowSelect, GridLines, HotTrack, HideSelection, ColumnClick and column resizing, scroll bars and the wheel, the keyboard, in-place caption editing (ReadOnly False: F2, a click on the selected item); OnChange (Index, Change), OnClick, OnDblClick, OnColumnClick (v2.77.0)
- [x] Objects as values with compile-time field slots, one shared front end (`rapidr_ast::objects`) for both backends; arrays of TYPE objects; instance-bound EVENT handlers; method pointers (v2.27.0)
- [x] Remove the interpreter compiler's old object code (setup_instance, TypeInfo, …) and codegen's UDT-struct path, now unused (v2.28.1)
- [x] Speed: slot globals (both backends), allocation-free array access and frame reuse in the VM (v2.28.0)
- [x] Declared numeric types enforced in both backends (`rapidr_ast::numeric`, v2.31.0)
- [x] Native typed locals: numeric SUB/FUNCTION locals are Rust `i64`/`f64` with native arithmetic, conditions and FOR loops (`typed.rs`, v2.32.0)
- [x] Native typed main-program variables (atomic statics) and BYVAL parameters (v2.38.0)
- [ ] Speed next: typed main-program variables (globals no SUB touches), typed BYVAL parameters and FUNCTION results, typed builtins (ABS, INT, SQR…), typed array elements; fewer clones in generated code; `Module::add_string` is a linear search at compile time
- [x] Native builds of real RapidQ programs: all 116 corpus programs that compile to bytecode `cargo check` natively (from 49; objects by name, WITH on any object, indexed properties, builtin-named variables, nested SUBs, argument fitting) — `tools/corpus_native.sh` (v2.33.0)
- [x] Native corpus programs run like the interpreter: `tools/corpus_compare.mjs` runs both builds and compares output and window captures — 99/102 identical, the other 3 use RND/TIMER (v2.38.0)
- [x] Corpus gaps both backends share (found by corpus_compare): ~~QMEMORYSTREAM SaveArray/LoadArray~~ (v2.39.0), ~~QIMAGE `BMPHandle` from `$RESOURCE` + drawing (othello's board)~~ (v2.40.0), ~~Printer~~ (v2.45.0, PDF through CUPS / the browser)
- [x] Corpus comparison after v2.40.0: 98/101 identical (3 RND/TIMER); rotate.bas needs its missing `$RESOURCE` file
- [x] `$RESOURCE` on both backends: files built into the program (bytecode section / `include_bytes!`), RESOURCE(n), RESOURCECOUNT, EXTRACTRESOURCE, `Stream.ExtractRes`, BMPHandle, AddBMPHandle (v2.40.0)
- [x] QIMAGE as a picture from the shared Bitmap model on desktop and web: BMP / BMPHandle, AutoSize, Stretch, Center, Transparent, drawing, Pixel; RapidQ's mouse events; MOUSEX / MOUSEY (v2.40.0)
- [x] `$RESOURCE` in the web IDE: the files are the project's assets (v2.52.0)
- [x] The conformance suite (`tests/web_conformance.mjs`) and the desktop GUI fixtures (`tests/web_gui_parity.mjs`) run in the browser too (v2.52.0): 53 of 55 conformance cases and 21 GUI checks match; the rest are marked
- [x] Web: a `ShowModal` in the main program waits (the IDE shows the startup form after the program's own statements, as RapidQ's designer does); the program ends when its main body finishes with no form open (timers stop); `Form.Repaint`; web parity checks for them and for a resized form with a dragged splitter (`align_layout`) (v2.61.0)
- [x] Resources: ICOHandle / Icon (forms v2.73.0, QIMAGE v2.76.0); ICO, PNG and JPEG resources and files wherever a bitmap goes, in the shared model (v2.73.0 / v2.76.0) (PLAYWAV of files and resources: v2.45.0)
- [x] Mouse event arguments in RapidQ's order everywhere (v2.59.0)
- [x] QSTRINGGRID runtime: one model for desktop (FLTK table) and web; Cell(col,row), sizes, fixed rows/cols, insert/delete/swap, Separator files/streams, selection, in-place editing, ellipsis columns, OnSelectCell/OnSetEditText/OnEllipsisClick (v2.34.0)
- [x] QSTRINGGRID OnDrawCell with the grid's drawing methods, on desktop and web (v2.42.0)
- [x] QSTRINGGRID goRangeSelect, gcsList drop-downs, goColSizing / goRowSizing on desktop and web (v2.43.0)
- [x] QSTRINGGRID extras: row / column moving by the mouse (goRowMoving / goColMoving: a header cell dragged), VisibleRowCount / VisibleColCount, RapidR's MoveCol / MoveRow, on native, interpreter and web (v2.78.0); a selected range read by the program: RapidQ has no property for it (goRangeSelect's range stays the user's)
- [x] `Align` (alTop / alBottom / alLeft / alRight / alClient) on desktop and web from one layout function (Delphi's AlignControls), live geometry on the desktop, form resizing with OnResize (v2.35.0)
- [x] QSPLITTER dragging on desktop and web (Delphi TSplitter: neighbour, MinSize, OnMoved) (v2.37.0)
- [x] Form Width/Height include the frame (29px caption, 1px borders) and ClientWidth/ClientHeight exclude it and the menu, the same on desktop and web; bsNone has no frame (v2.37.0)
- [x] QLISTBOX / QCOMBOBOX from one shared model on desktop and web: Item, ItemCount, ItemIndex, AddItems (kept only the last item on the desktop), InsertItem, DelItems, Sorted, MultiSelect/Selected/SelCount, Text, Load/SaveToFile (v2.36.0)
- [x] QLISTBOX MultiSelect drawn and picked on the desktop; QCOMBOBOX csDropDown edit box on desktop and web (v2.41.0)
- [x] QFILELISTBOX and QDIRTREE on desktop and web from shared models (v2.44.0)
- [x] QLISTBOX owner-draw (OnDrawItem) (v2.50.0)
- [x] QLISTBOX `Columns`; owner-drawn QCOMBOBOX (v2.60.0)
- [x] Type suffixes as declared types: `?` BYTE, `??` WORD, `???` DWORD, `%` SHORT, `&` LONG, `!` SINGLE, `#` DOUBLE (v2.55.0)
- [x] Security: overflow-safe integer ops, string size cap, VM call-depth limit, builtin and compiler fuzzing (v2.28.0)
- [x] Security: the VM hosts are sound on re-entrant events — the runtime queues handlers, the VM runs them at safe points and serves ShowModal's wait itself; no `unsafe` in the VM or its hosts (v2.30.0)
- [ ] Fuzzing in CI
- [x] QFILESTREAM on the shared stream code; `Stream.Read(var)` (v2.26.0)
- [x] Streams: ReadUDT/WriteUDT (v2.56.0; LoadArray/SaveArray and typed `Read(var)` / `Write(var)` sizes done in v2.39.0, ExtractRes in v2.40.0)
- [x] QBITMAP text (`TextOut`, `TextWidth`/`TextHeight`, Font) from built-in Liberation fonts (OFL) on every platform (v2.46.0)
- [x] QCANVAS drawn by the shared bitmap model (text, fonts, `Pixel`, same pixels on desktop and web); desktop `Rect`/`FillRect` now take corners like RapidQ (v2.47.0)
- [x] QCANVAS/QFORM `OnPaint`: form built, resize, Repaint/Refresh/Update (v2.48.0)
- [x] Drawing on a QFORM itself (`Form.TextOut`, `Form.Line`, …) on desktop and web from the shared bitmap model; a form paints again when resized (v2.49.0)
- [x] Owner-drawn QLISTBOX (`Style`, `ItemHeight`, `OnDrawItem`) on desktop and web from the shared list model (v2.50.0); all of RAPIDQ.INC's option constants (v2.50.0)
- [x] RapidQ syntax gaps found by the example corpus: `Arr()` arguments, multi-dimensional TYPE array fields, `THEN:` blocks, `=>` / `=<`, `&H…&`, `CASE list stmt`, line-numbered `NEXT` / `DATA`, per-file `$ESCAPECHARS` (v2.51.0); 134 of 386 example programs compile, all of them build natively (`python3 tools/rapidq_corpus.py ~/Downloads/Rapidq/examples`)
- [x] Native builds: routines that differ only by a type suffix, `DEFSTR` arrays next to same-named locals, `GOSUB` / labels inside SELECT CASE (v2.54.0)
- [ ] Native builds: a line label / `GOSUB` inside `WITH` or `CREATE`
- [x] `INPUT #`, `LINE INPUT #`, BASIC file I/O on the web, `CREATE name(dims) AS type`, keywords as names (v2.53.0)
- [x] `DIM s AS STRING * n` (stores cut to n), `CBOOL`, `ON ERROR …` accepted (ignored), an unclosed `WITH` closed by `END SUB` (v2.54.0)
- [ ] Real `ON ERROR RESUME NEXT` / `Err` (VB): resuming after a run-time error needs recovery points in the VM and native code; fixed strings also need padding to `n` in `WriteUDT`/`ReadUDT`
- [x] The runtime continues after an event's handler has run (`rp_fire_event_then`, `rapidr_value::events`): at once natively; in the interpreter the continuation travels with the queued event and the VM hands it back when the handler returns, even after a dialog (v2.57.0)
- [x] Events that return values through by-reference parameters: `OnClose(Action)`, `OnSelectCell(…, CanSelect)`, `OnMeasureItem(Index, Height)`, `OnListDropDown(Col, Row, S)` on native, interpreter and web (v2.58.0). QTREEVIEW's `OnChanging` / `OnExpanding` / `OnCollapsing` / `OnEditing` / `OnEdited` come with QTREEVIEW's nodes (below)
- [x] Keyboard and mouse events with RapidQ's arguments on desktop and web: OnKeyDown / OnKeyUp (Key, Shift), OnKeyPress (Key), OnMouseDown / OnMouseUp (Button, X, Y, Shift), OnMouseMove (X, Y, Shift) — `rapidr_value::input` (v2.59.0). Not yet: `KeyPreview` order (the form after the control), a handler changing `Key` to swallow it, `KillMessage`
- [x] QFORMMDI: child windows (AddChild by Handle, frames with title bar / minimize / maximize / close, drag, resize), activation, next / previous, cascade, tiling, arrange icons, minimize / maximize / restore all, GetChild / ChildExist / FreeChild, Child* properties, OnChildActive / OnChildClose (ChildResult) / OnChildResize — one model (`rapidr_value::mdi`) for the desktop and the web (v2.57.0). Not yet: `MDIMenu` (the window list in a menu), `ChildIcon`, `SetDeskBar`
- [x] Components given a parent after their form is shown get their widget then (desktop; the web did) (v2.57.0); `Handle` for components (v2.57.0)
- [x] Owner-drawn QCOMBOBOX (`csOwnerDrawFixed` / `csOwnerDrawVariable`: OnDrawItem, OnMeasureItem) and QLISTBOX `Columns` on desktop and web from the shared list model (v2.60.0)
- [x] QLISTBOX `TabWidth`, `ExtendedSelect` (v2.63.0)
- [ ] Grid `OnDrawCell` text from the shared fonts (the grid draws it with FLTK / the browser); `ExtendedSelect` for plain (not drawn) multi-select lists on the desktop (FLTK's own browser)
- [x] Desktop look: `$THEME` / `RAPIDR_THEME` name any fltk-theme theme or scheme or FLTK scheme; the interpreter honors `$THEME` too; Linux defaults to a light look (was Dark); fltk 1.5 / fltk-theme 0.7.9 are current (v2.62.0)
- [ ] Modern platform looks by default (fltk-theme's Aqua on macOS, Fluent on Windows): RapidR's buttons, grids and lists need styling for those schemes first (default-colored buttons vanish, Fluent draws grid headers wrong); fltk-theme's `crystal` scheme panics (upstream)
- [x] QIMAGELIST AddICOFile / AddICOHandle / InsertICO… / GetICO (an icon scaled whole to the list's size) and `ImageList.Draw` onto a canvas (v2.76.0)
- [ ] `Rotate (xOrigin, yOrigin, Angle)` on QBITMAP / QCANVAS / QIMAGE: the manual doesn't say the direction or what fills the uncovered area — needs a real RapidQ to compare
- [x] `rapidr build --interp` always has cargo refresh `rapidrintr-runner` (v2.24.0)
- [x] QTREEVIEW from a shared model (`rapidr_value::objects::tree`): nodes numbered depth-first, AddItems / AddChildItems / InsertItem / DelItems / Clear / Sort, Expand / Collapse / FullExpand / FullCollapse, GetItemAt, Item(i).Text / ImageIndex / SelectedIndex / StateIndex / HasChildren / Selected / Expanded / Count / Level / IsVisible / Parent, Images icons, LoadFromFile / SaveToFile, OnChanging / OnExpanding / OnCollapsing (answering), OnChange / OnExpanded / OnCollapsed / OnDeletion / OnClick / OnDblClick on native, interpreter and web (v2.64.0)
- [x] QTREEVIEW in-place editing: F2 or a click on the selected node, `OnEditing (Index, AllowEdit)` / `OnEdited (Index, S)` answering, Enter / leaving keeps, Escape drops, ReadOnly on native, interpreter and web (v2.65.0)
- [x] QTREEVIEW: StateImages (beside the node's image), OnGetImageIndex / OnGetSelectedIndex (asked when what's shown changes), HideSelection, GetItemAt(X, Y) on native, interpreter and web (v2.74.0)
- [x] RapidQ global objects `Screen`, `Application`, `Clipboard`, `Mouse` from a shared model (system clipboard on the desktop), component `Cursor` codes, QRECT fields default 0 on native, interpreter and web (v2.65.0)
- [x] `Canvas.CopyRect(D AS QRECT, Image, S AS QRECT)` with DIMmed QRECTs; `Application.Icon` / `IcoHandle` and `Form.Icon` / `IcoHandle` (ICO, BMP, PNG or SVG; the web's title bars and page icon); ICO and PNG pictures everywhere a bitmap goes; a new QIMAGE sized by its first picture (v2.73.0)
- [ ] `Screen.CaptureToBMP` / `CaptureToFile` (RAPIDQ2.INC, the Windows desktop): with the Windows-only objects
- [x] RapidQ's include libraries: a TYPE extending a component created inside a form, `TypeName.Field` in its PROPERTY SET, fields redeclaring the component's properties, FOR counters and `Result` in its code; QPANEL BevelOuter / BevelInner / BevelWidth / BorderWidth (QBevel.inc works; QDigDisplay.inc with RapidR's own digit bitmaps, `examples/digdisplay`) (v2.66.0)
- [x] RapidQ syntax found by the include folder and the example corpus: an object field's own properties (`P.MoverRect.Top`), keywords as a TYPE's field / method names (`Step`, `Open`, `Data`, `FUNCTION Create`), keyword parameters and variables (`select`, `case`, `type = 2`), `END PROPERTY SET`, `STRUCT … END STRUCT`, `ByVal` in calls, `_` stuck to a name as a line continuation, comment lines inside continued statements (v2.68.0)
- [x] SUBI / FUNCTIONI closed by END SUB / END FUNCTION; a method of any object expression (`This.Names.Item(2)`, `printer.Font.DelStyles(3)`); RUN (and SHELL / SHELLWAIT through `cmd /C` on Windows); INITARRAY (v2.69.0)
- [x] Dotted TYPE field names (`hdr.hwndFrom AS LONG`, `Table.Name(150) AS STRING`): a record inside the record, on both backends (v2.75.0)
- [ ] RapidQ's include folder: 79 of 108 libraries compile (`./rapidr build-bc` on each `include/*.inc` from that folder; 76 before dotted fields, 78 before templates — an earlier "85" was counted another way); the rest call the Windows API (SENDMESSAGE, GetDC, …), miss include files, or have typos (`&HFFFF0000???`, `TYPE X<Size>`)
- [ ] Example corpus: 153 of 386 compile (`python3 tools/rapidq_corpus.py ~/Downloads/Rapidq/examples --include ~/Downloads/Rapidq/include`); most of the rest call the Windows API, miss include files, or have typos RapidQ couldn't compile either
- [x] SVG wherever a bitmap goes — QBITMAP / QIMAGE (`LoadFromFile`, `BMP`, `BMPHandle`), QIMAGELIST (`AddBMPFile`, `AddBMPHandle`), `Canvas.Draw`, `$RESOURCE` — drawn by resvg in the shared model with soft edges (per-pixel alpha, kept through `.BMP`) on native, interpreter and web (v2.67.0)
- [x] SVG form / application icons (v2.73.0)
- [x] QOUTLINE (a tree view: AddLines by indentation, AddChild(Index, S), Insert, Item(i), Row, LineCount); QOPENDIALOG / QSAVEDIALOG / QFILEDIALOG from a shared model (RapidQ filters, FilterIndex, InitialDir, DefaultExt, MultiSelect, Files(), SelCount, FileTitle; an in-page dialog on the web with Upload) on native, interpreter and web (v2.71.0)
- [x] QHEADER from a shared model: AddSections, Clear, `Sections(i)` Caption / Width / MinWidth / MaxWidth / Alignment / AllowClick / Style, drawn on like a canvas; sections clicked and resized with the mouse (resize cursor on an edge), OnSectionClick / OnSectionTrack (begin, move, end) / OnSectionResize, owner-drawn sections through OnDrawSection (Index, Pressed, Rect) on native, interpreter and web (v2.72.0)
- [x] High-DPI: canvases, form surfaces, QIMAGE pictures, owner-drawn list / combo items, grid images and tree icons shown at the screen's scale (FLTK `pixels_per_unit` on Retina, the browser's `devicePixelRatio`): each bitmap keeps what the screen shows next to the pixels programs read (`Pixel`, `.BMP`, flood fills — unchanged, checked by the GUI suites at 2×); text, lines and ellipses drawn finer, SVGs drawn at the scale (v2.70.0)
- [x] High-DPI: grid cells' own drawing on the web at the screen's scale (v2.75.0)
- [ ] High-DPI leftover: RapidR's own IDE icons as vectors
- [ ] The rest of `rapidr_ast::RAPIDQ_OBJECTS_NOT_YET_IMPLEMENTED`
- [x] Native builds catch up: GOTO/GOSUB, STATIC, same compile errors as the VM (v2.23.0); function pointers (v2.24.0). Principle: native builds are compiled Rust only, never the embedded interpreter (v2.24.1)
- [x] The Rust backend compiles OOP TYPEs (methods, CONSTRUCTOR, EVENT, EXTENDS, PROPERTY SET, composition, CREATE of a TYPE, object array fields) — objects.rs (v2.25.0)
- [ ] Consider generating Rust from a shared, typed IR (one front end for both backends) so they can't drift

- [ ] `$DIALECT RAPIDQ | VB6 | RAPIDR` (Q-aliases, ByRef default, `Me`/`This`, rounding rules)
- [x] Built-in `RAPIDQ.INC` (colors, `mr*`, `MB_*`/`ID*`, `bs*`, `ws*`, `al*`, `mb*`, `fm*`, `VK_*`) as a single line (v2.13.0)
- [x] Q→R type names via `rapidr_ast::canonical_type_name` + single `COMPONENT_TYPES` list (v2.13.0)
- [ ] Backends' component list lacks some IDE toolbox components (RImageList, RIni, RLine, RIcon, RMemoryStream) — reconcile with lang-data
- [x] `$INCLUDE` source map: errors in included code report the include file and line (CLI, v2.15.0); the IDE's multi-file mapping is separate
- [x] Real RapidQ includes: `RAPIDR_INCLUDE_PATH`, `\` paths, case-insensitive names, absolute `c:\rapidq\…` paths, Windows-1252 source, `$DEFINE`s shared across includes, `WIN32` predefined, VB `#If/#Else/#End If/#Const` (v2.15.0)
- [x] Errors in code the program can't reach don't block it: native-only features anywhere, unknown names/unsupported features only in `$INCLUDE`d libraries (v2.16.0)
- [x] RAPIDQ2.INC compiles and runs (v2.16.0)
- [x] `?` as PRINT (v2.13.0)
- [x] `INC`/`DEC` (both backends, shared desugaring in rapidr-ast) and `PRINT` separators / 14-column zones (v2.11.0)
- [x] `GOSUB`/`GOTO`/labels (line numbers too) and bare `END` in the VM (v2.12.0)
- [x] Codegen: state-machine lowering for labels/GOTO/GOSUB (jumps.rs) (v2.23.0)
- [x] Web: `END` stops the program in both web builds (forms close, timers stop, no more events) (v2.55.0); labels inside SELECT CASE in native builds (v2.54.0; WITH / CREATE: item above)
- [x] `TYPE … EXTENDS` with `EVENT … END EVENT`, `CONSTRUCTOR`, methods, inheritance, `Sender`, implicit `This` members — VM (v2.14.0)
- [x] Codegen: OOP TYPEs in the Rust backend (v2.25.0)
- [x] RapidQ OOP per manual ch. 10 (VM): `PROPERTY SET`, type name as the instance (`TForm.Focus`, `WITH TForm`), `EXTENDS QObject`, `TYPE X AS QFORM`, `PUBLIC:/PRIVATE:/PROTECTED:`, `obj.Func` without parentheses (v2.15.0)
- [x] Composition, nested object access, object arrays, component sub-objects (VM, v2.16.0)
- [x] OOP: `Super.X`, `obj.Inherit<Event>`, custom events (`AS EVENT(Template)` fields given a SUB, fired with CALLFUNC), templates (`TYPE T<DataType, Size>`, one TYPE per set of arguments) on both backends (v2.81.0)
- [ ] Runtimes: implement the remaining RapidQ objects (QFONT, QBITMAP, QIMAGELIST, QMEMORYSTREAM done in v2.20.0) and indexed sub-objects (`item.caption(i)` / `item.caption=(i, v)` method names emitted by the VM)
- [x] `$OPTION ICON "app.ico"` (the program's icon, built in; a missing one leaves the default) and `$OPTION BYREF` on both backends; `$OPTION EXPLICIT` / BYTECODE / GTK / INKEY$ / VBDLL / WEAKTYPE accepted (v2.79.0)
- [x] `$OPTION DIM type` (undeclared variables' type) and `$OPTION DECIMAL` (VAL's decimal character) — v2.82.0
- [x] Undeclared variables are DOUBLE as RapidQ's; RapidQ's `Type mismatch` error for a string stored into a number (v2.84.0)
- [x] `$TYPECHECK ON/OFF` and `$OPTION EXPLICIT`: RapidQ's `Undeclared identifier` (v2.85.0)
- [x] RapidQ's argument-count, duplicate-DIM and RESULT-outside-FUNCTION errors (v2.86.0); its other messages are in `.reference/rapidq-compiler-messages.txt`
- [x] RapidQ's `Property X of Y is read-only.` for its components' read-only properties (v2.87.0)
- [x] `REPLACESUBSTR$`; string functions character-based and shared (`rapidr_value::strings`) (v2.11.0)
- [x] Fix builtins per the manual (v2.55.0, shared `rapidr_value::format` / `builtins`): `INSERT$(insert, source, index)` ("hi","Hello",3 → "Hehillo"), `FORMAT$` = Delphi `Format()` (`%.5d` zero-pads, `%05d` doesn't), `STRF$` = Delphi `FloatToStrF(v, ffGeneral/ffExponent/ffFixed/ffNumber, precision, digits)` (+ audit all builtins vs `.reference/` docs)
- [x] RapidQ syntax (v2.15.0): full `DIM`/`DEFxxx` grammar (per-name AS, untyped = VARIANT, `(a,b)(n)` groups, `= v` / `= {…}` initializers, `STRING * n`), `STATIC` (VM), `i++`/`x += y`, `s$[i]`, `"jello" - "l"`, `@var` by reference, `name()` array params, `CASE x: stmt`, `PUBLIC/PRIVATE/GLOBAL`, literal suffixes, lenient strings + `_` inside strings, keyword-named SUBs (`SUB Close`), RESULT, NOT/MOD precedence, `NOT=`, comparisons are -1/0, WITH in the VM
- [x] Codegen: `STATIC` in SUB/FUNCTION (renamed to a per-routine global slot) (v2.23.0)
- [x] `FUNCTIONI`/`SUBI`, `SHL`/`SHR`, `DATA`/`READ`/`RESTORE`, `SWAP`, `$ESCAPECHARS`, function pointers (VM) (v2.16.0)
- [x] `INV`, empty arguments `INSTR(,a,b)`, console `LOCATE`/`CLS`/`COLOR`/`CSRLIN`, `REDIM` (keeps data) (verified v2.55.0). `VARPTR` / `MEMCPY` / `MEMSET`: memory-safe virtual memory (v2.56.0)
- [x] Codegen: function pointers (v2.24.0)
- [ ] SUB/FUNCTION pointers as Win32 callbacks (native FFI)
- [ ] Win32 shim table for top ~50 `DECLARE … LIB "user32"/"kernel32"/"shell32"` calls; clear warnings for the rest
- [ ] VB6: `On Error GoTo/Resume Next`, `Optional`, `ParamArray`, `Property Get/Let/Set`, `Enum`, `Static`, `ReDim Preserve`, `For Each`, `_` continuation, `Select Case Is/To`, `Like`
- [ ] Modern `TRY/CATCH`
- [ ] Runtime errors carry source line on both backends
- [ ] **RapidQ importer**: folder/zip → follow `$INCLUDE` → `CREATE` trees become designer forms → modules → compatibility report
- [ ] **VB6 importer**: `.vbp` + `.frm` (`Begin VB.Form …`) + `.bas`
- [ ] Imported projects with `RUSTSTART`/`DECLARE LIB` flagged native-privileged (SEC-10)
- [x] Corpus: the original RapidQ distribution (386 example programs, 126 includes, manual) → `tools/rapidq_corpus.py` reports compile pass-rate and top blockers (v2.15.0: 82/386 compile, from 46)
- [ ] Corpus: raise the compile rate; then run programs, not just compile them (golden outputs for console examples)
- [ ] **Needs from user:** real RapidQ programs and any RapidQ IDE project-file samples (format not yet confirmed)

## Phase 2 — Debugger (~6 weeks)

- [ ] Immediate window: evaluate expressions/statements in the paused frame (compile snippet against frame symbols)
- [ ] Edit & Continue: hot-swap recompiled function when signature unchanged; Set Next Statement
- [ ] Conditional breakpoints, hit counts, logpoints; break on runtime error
- [ ] Reverse stepping: ring buffer of VM state deltas + recorded host-call results
- [ ] Event timeline (event → handler → duration)
- [ ] "Inspect element": click widget in running preview → properties/handlers
- [ ] Line profiler: per-line instruction counts → gutter heatmap
- [ ] `rapidr dap` Debug Adapter Protocol server (VS Code extension + any DAP client)
- [ ] Policy: debug on VM, ship on either backend (guaranteed by differential tests)

## Phase 3 — IDE (~5 weeks, can overlap Phase 2)

**Direction (user, 2026-09-28): a professional IDE in the tradition of RapidQ / VB6 / Delphi / Xojo — an MDI workspace on the desktop and on the web alike.** Starts once RapidQ compatibility is complete across native, interpreter and web.

- [ ] MDI workspace: form designers, code editors, the property inspector, the project tree and the running program as child windows inside one main window (cascade / tile, window menu), built on the same child-window model as QFORMMDI (`rapidr_value::mdi`); the running program's forms as windows in the workspace instead of the web preview iframe
- [ ] Desktop IDE rebuilt on it (the current IDE modified), the web IDE the same workspace in the browser
- [ ] Split `web-ide/host.js` into modules
- [ ] Compiler-backed language service: go-to-definition, references, rename, outline, typed completion (replace regex `resolveVariableType`)
- [ ] VB-style events tab (double-click → handler stub), menu editor, tab-order editor, code/designer toggle
- [ ] IndexedDB autosave; File System Access API open/save
- [ ] Tauri desktop shell: same IDE + native `.exe`/`.app` builds via FLTK backend

## Phase 4 — AI in the IDE (~6 weeks)

- [ ] Provider adapters: Anthropic Messages + OpenAI-compatible (OpenAI, DeepSeek, OpenRouter, Ollama, custom). Verify DeepSeek CORS; optional proxy fallback
- [ ] Tool registry (MCP-shaped): `read_file`, `apply_edit`, `add_form`, `add_widget`, `set_property`, `compile`, `run`, `inspect_component`, `get_variables`, `click`, `screenshot_preview`, `lookup_docs`
- [ ] System prompt generated from the language registry + examples; `lookup_docs` for details; prompt caching
- [ ] Agent loop: plan → edit → compile → fix → run → verify; checkpoints + undo
- [ ] `rapidr mcp` (stdio) + Tauri MCP server for external agents
- [ ] "Fix with AI" quick-fix, inline completions, screenshot → form, "Explain this crash"
- [ ] Security model: keys only in IDE origin (requires SEC-02), optional WebCrypto passphrase, never in `.rrproj`/bundles/logs; permission tiers (read auto / edit auto+checkpoint / run sandboxed, network off by default / external = ask); imported content treated as data

## Phase 5 — `RAI` component (~5 weeks)

```basic
CREATE AI AS RAI
    Provider = "anthropic" : Model = "claude-sonnet-5"
    Endpoint = "https://myapp.com/ai-proxy"   ' or ApiKey for local/dev
    AutoTools = True
    OnToken = AIToken : OnResponse = AIDone : OnToolCall = AIApprove
END CREATE

'@tool "Adds an item to the inventory"
SUB AddItem(Name AS STRING, Qty AS INTEGER)
```

- [ ] Compiler emits tool manifest (JSON schema) for `'@tool` SUBs/FUNCTIONs into bytecode metadata
- [ ] Runtime validates tool args against schema before `invoke_function`
- [ ] Component tools read-only by default; `OnToolCall` veto; per-turn call cap; `AI.Log` audit trail
- [ ] Async: `fetch` + event dispatch on web; background thread + UI-thread events on native
- [ ] AI-written code tools behind `AllowCodeTools = True`, run in `SandboxHost` (capability-filtering `Host` wrapper: no file/net/DB/FFI/RJavaScript unless granted; fuel + memory limits)
- [ ] Compiler warning on literal keys (`sk-`, `sk-ant-`, …); `Endpoint` proxy mode; open-source reference proxy (small Rust binary); CSP auto-allows provider host
- [ ] `RAIChat` drop-in chat widget

## Phase 6 — Security audit (continuous + formal pre-3.0)

- [ ] Threat model (`docs/THREAT_MODEL.md`), trust boundaries: untrusted source/projects → compiler/IDE; app ↔ users/data (XSS, SQLi); AI output / prompt injection; native (FFI, `RUSTSTART`, file ops); supply chain; network services (MCP, proxy)
- [ ] `cargo-fuzz` targets: lexer, preprocessor (include recursion, macro blow-up limits), parser, bcgen, `Module::from_bytes`, VM execution with fuel — goal: no panics/hangs
- [ ] Full `unsafe` review + Miri (SEC-08)
- [ ] IDE: strict CSP + Trusted Types; self-host fonts
- [ ] `SECURITY.md` + private disclosure process; Dependabot; release checksums + SBOM
- [ ] Community security review call before 3.0

---

## Appendix A — Reproduction programs

Build/run with: `./rapidr build-bc file.bas -o file.rrbc && ./rapidr run-bc file.rrbc`

**rq1.bas** — classic RapidQ GUI (fails: `RAPIDQ.INC`, Q-types, `DECLARE`)
```basic
$APPTYPE GUI
$TYPECHECK ON
$INCLUDE "RAPIDQ.INC"

DECLARE SUB Button1Click (Sender AS QBUTTON)

CREATE Form AS QFORM
    Caption = "Hello"
    Width = 320
    Height = 240
    Center
    CREATE Button1 AS QBUTTON
        Caption = "Click me"
        Left = 10: Top = 10
        OnClick = Button1Click
    END CREATE
END CREATE

SUB Button1Click (Sender AS QBUTTON)
    ShowMessage "Hi " + STR$(10)
    Form.Caption = "Clicked"
END SUB

Form.ShowModal
```

**rq2.bas** — control flow (fails: `?`; GOSUB/GOTO silently dropped)
```basic
DEFINT i, j
DIM A(1 TO 10) AS INTEGER
i = 1: j = 2
IF i = 1 THEN PRINT "one" ELSE PRINT "other"
? "question mark print"
GOSUB MySub
PRINT "after gosub"
GOTO Done
MySub:
  PRINT "in gosub"
RETURN
Done:
PRINT IIF(i > 0, "pos", "neg")
```
Expected: `one` / `question mark print` / `in gosub` / `after gosub` / `pos`.
Actual (without the `?` line): `one` / `after gosub` / `in gosub` then exits.

**rq3.bas** — extended form type (EVENT unsupported, no error)
```basic
TYPE TMyForm EXTENDS QFORM
    Counter AS INTEGER
    EVENT OnClick
        This.Counter = This.Counter + 1
        This.Caption = STR$(This.Counter)
    END EVENT
    CONSTRUCTOR
        Caption = "Custom"
        Width = 200
    END CONSTRUCTOR
END TYPE
DIM F AS TMyForm
F.ShowModal
```

**rq4.bas** — builtins
```basic
DIM i AS INTEGER
i = 5
PRINT IIF(i > 0, "pos", "neg")                 ' pos        (ok)
PRINT FIELD$("a,b,c", ",", 2)                  ' b          (ok)
PRINT TALLY("banana", "a")                     ' 3          (ok)
PRINT REPLACESUBSTR$("aXbX", "X", "-")         ' a-b-       (got: empty)
PRINT RINSTR("hello", "l")                     ' 4          (ok)
PRINT STRF$(3.14159, 0, 5, 2)                  ' formatted  (got: 3.14159)
PRINT CONVBASE$("255", 10, 16)                 ' FF         (ok)
PRINT LTRIM$("  x"), UCASE$("abc"), MID$("hello", 2, 3), HEX$(255), CHR$(65), ASC("A")
                                               ' tab zones  (got: no separators)
PRINT FORMAT$("%05d", 42)                      ' 00042      (got: 42)
PRINT DELETE$("hello", 2, 2), INSERT$("XY", "hello", 2), REVERSE$("abc")
                                               ' INSERT$ wrong
INC i
PRINT i                                        ' 6          (got: 5)
```

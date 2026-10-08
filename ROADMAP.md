# RapidR Roadmap

> Goal: the best RapidQ-compatible, VB-style BASIC compiler, interpreter, debugger and IDE ever —
> compatible, secure by default, AI-native, and open source. A modern RapidQ-compatible system:
> old programs keep working; new ones get high-DPI, accessible, responsive
> interfaces, the data-science and AI stacks, and the same behaviour on the
> desktop (native and interpreted), the web and, later, mobile. Target: 2027
> the year of the best BASIC language and IDE (see "Timeline" at the end).
>
> Created 2026-09-24 from a codebase review at v2.8.1 (`development` @ `d458126`).
> Keep this file current: tick boxes as work lands, add findings as they are discovered.

---

## Guiding principles

1. **Never silently wrong.** Every construct either compiles correctly or produces a clear diagnostic with line/column. No dropped statements.
2. **One semantics.** The bytecode VM is the reference implementation. The Rust codegen backend must produce identical output on every conformance program (differential testing in CI).
3. **One source of truth.** A single language registry (components, properties, methods, events, builtins, signatures, docs) generates the IDE completion data, the VS Code extension data, the manual, and the AI system prompt. Done: `rapidr lang export` generates them (the old HTML web IDE's hand-written data went with that IDE, 2026-10-08).
4. **Secure by default.** Untrusted code, imported projects, and AI output never get more privilege than they asked for.
5. **Measured compatibility.** A public "% of real RapidQ/VB corpus that compiles and runs" number, tracked over time.
6. **Logical pixels, device rendering.** A program's coordinates are RapidQ's pixels (1/96 inch, Windows at 100 %) on every platform, so old layouts stay as they were; everything is *drawn* at the screen's real resolution (text, lines, shapes, SVG), so it's sharp on high-DPI screens. Nothing old code reads changes; what's new (the scale, SVG, @2x images) is additive.
7. **One UI kernel.** Each component is a shared model (`rapidr_value::objects`) that lays itself out, draws itself as vector ops, hit-tests, and describes itself (accessibility and AI); the runtimes only render the ops and pass input. Already the case for menus, text edits, list / tree / grid / list views, tab controls, track bars and scroll bars — every component moves there. It's what makes the desktop, the web and mobile identical, high-DPI, accessible and inspectable by AI from one place.
8. **Accessible by default.** Every program is usable by keyboard and by screen readers without the author doing anything; the accessibility tree comes from the same shared models.

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
| SEC-02 | High | Preview iframe `sandbox="allow-scripts allow-same-origin"` neutralizes the sandbox; app code runs with IDE origin (can read future API keys, modify IDE) | the old HTML web IDE's preview frame (deleted 2026-10-08) | Serve preview from a separate origin; postMessage-only bridge |
| SEC-03 | High | `message` listeners don't check `e.source`/origin; `postMessage(..., "*")` | the old HTML web IDE's host and preview page (deleted 2026-10-08) | Verify `e.source === iframe.contentWindow` + per-session nonce; explicit targetOrigin |
| SEC-04 | High | JS injection: `RStringList.SaveToFile` escapes `"` but not `\` before `eval` (payload `\");alert(1);//`) | `crates/rapidr-runtime-web/src/object_web.rs:1310` | Use `web_sys` Blob + anchor, no eval |
| SEC-05 | Medium | `Caption`/`Text` set via `innerHTML` for non-label elements → XSS when showing DB/HTTP/AI data | `crates/rapidr-runtime-web/src/gui_web.rs:235`, `:604` (15 innerHTML sites total) | `textContent` by default; explicit `.HTML` property for markup |
| SEC-06 | Medium | 6 `js_sys::eval` sites with string-built JS. **Also a functional bug:** bundle CSP (`script-src 'self' 'wasm-unsafe-eval'`) blocks eval, so `RHttp`, `Sound`/`Beep`, `SaveToFile` likely fail in deployed bundles; `connect-src 'self'` blocks external APIs; `frame-src 'none'` blocks `RWebView` | `network_web.rs:58`, `:125`; `builtins.rs:550`, `:569`; `object_web.rs:1314`; `gui_web.rs:1269`; CSP in the old HTML web IDE's zip exporter (deleted 2026-10-08) | Replace with `web_sys` calls; clippy `disallowed_methods` for eval outside `RJavaScript`; derive CSP from components used |
| SEC-07 | ~~Medium~~ fixed v2.114.0 | SQL APIs take raw strings, no parameter binding (`query_map([], …)`) → SQLi by default | `crates/rapidr-runtime-core/src/database.rs:103` (+ MySQL path, web DB) | Add parameter binding API (`?` placeholders + `.AddParam`/array arg); document; teach AI |
| SEC-08 | Review | ~~23 `unsafe` in web host; event dispatcher stores leaked raw `*mut Vm` → possible aliasing/UB on re-entrant events~~ fixed in v2.30.0: events are queued and run by the VM itself; the VM and both hosts `#![forbid(unsafe_code)]`. Remaining: 68 `unsafe` in FFI | `crates/rapidr-runtime-core/src/ffi.rs` | Document invariants; Miri |
| SEC-09 | Gap | ~~No cargo-deny~~ (done v2.8.4); no fuzzing, no SECURITY.md; IDE loads Google Fonts (third-party) | — | Phase 0 + Phase 6 |
| SEC-11 | High | Build server preview path traversal: `dir.join(rel).starts_with(dir)` does not catch `..` → arbitrary file read | `crates/rapidr-buildserver/src/main.rs` (`serve_preview_path`) | Reject any non-`Normal` path component |
| SEC-12 | ~~Medium~~ fixed (SEC-15 pass) | `RWebView.HTML` uses `srcdoc` with default sandbox `allow-scripts allow-same-origin` → HTML runs with the app's origin | `crates/rapidr-runtime-web/src/gui_web.rs` (`"html"` prop, iframe creation ~:2201) | Drop `allow-same-origin` for `srcdoc` content by default; opt-in property |
| SEC-13 | Low | IDE preview: `RHttp` to relative URLs needs CORS headers from the IDE's server now that the preview is opaque-origin; origin-bound browser APIs (notifications) may be unavailable in preview | the old HTML web IDE's preview page (deleted 2026-10-08; RapidR Studio's run frame, `ide/web/run.html`, has the same opaque origin) | Document; optionally proxy same-server requests through the IDE bridge (needs async RHttp) |
| SEC-10 | Gap | `$INCLUDE` resolves arbitrary paths; projects with `RUSTSTART` / `DECLARE … LIB` get no warning on open/build | `crates/rapidr-preprocessor/src/lib.rs` | Confine includes to project root in IDE/MCP contexts; "native-privileged" project flag + confirmation |

### IDE / debugger baseline

Has: Monaco editor, regex-based completion/hover/signature help, visual designer, debugger (breakpoints, step in/over/out, stack, variables, watch list, component properties), assets manager, zip build, themes.
Missing: compiler diagnostics as editor markers, ~~undo/redo~~ (done v2.9.0), immediate-window evaluation (stubs), conditional breakpoints/logpoints, native/DAP debugging. (This was the old HTML / Monaco web IDE, a 4,587-line `host.js`; RapidR Studio replaced it and it was deleted on 2026-10-08.)

---

## Phase 0 — Stabilize & secure (~3 weeks)

**Sprint 1 (~2 weeks)**
- [x] SEC-01 + SEC-11: `rapidr-buildserver` locked down — binds 127.0.0.1 (override `RAPIDR_BUILDSERVER_HOST` warns), Host/Origin loopback guard (blocks cross-site + DNS rebinding), CORS loopback-only, path traversal fixed; unit tests + live curl probes (v2.8.2)
- [x] SEC-02 / SEC-03: preview iframe runs without `allow-same-origin` (opaque origin), runtime shipped in at boot, private `MessageChannel` handshake instead of window messages; `tests/web_ide_preview_isolation.mjs` (fails 11/15 on 2.8.2, passes on 2.8.3) (v2.8.3; since 2026-10-08 `tests/studio_run_frame.mjs` on RapidR Studio's run frame)
- [x] SEC-04: `SaveToFile` now uses shared `trigger_download` Blob helper (no eval); wasm check passes — verified in Chromium with and without bundle CSP (v2.8.2)
- [x] SEC-05: Caption/Text fallback and `PRINT` → `#rr-console` now plain text; markup only via `RDOM.InnerHTML` / `RWebView.HTML` (v2.8.2)
- [x] SEC-06: `RHttp`, `BEEP`, `SOUND` via `web_sys` (XHR / Web Audio); only `RJavaScript.Eval` remains; `clippy.toml` bans `js_sys::eval` (v2.8.2)
- [x] Unsupported constructs → hard diagnostics: parser errors with line/col for every bad line; bytecode compiler rejects unknown SUB/FUNCTION names (shared builtin registry) and statements it can't run; no catch-all arm left (v2.10.0). Codegen still reports unknown calls only via rustc.
- [x] IDE diagnostics: squiggles via `setModelMarkers` in the right form/module, clickable Errors panel, live checking while typing (v2.10.0; errors travel as `line:col: error:` text — move to a structured wasm API when the language service lands)
- [x] `tests/conformance/` harness (`run.mjs`) — now 34 pass / 4 known failures (all codegen): `*.bas` + `*.expected` / `*.expected-error`, VM **and** Rust codegen, xfail markers for known bugs, runs in CI — 15 seed cases, 7 pass / 23 known failures
- [x] Fix conformance failures (table above) until every case passes on both backends (all pass; re-verified v2.55.0)
- [x] `cargo-deny` (advisories, licenses, bans, sources) + `.github/workflows/ci.yml` (deny, workspace tests, eval lint); 5 vulnerable crates patched; native build fixed on Rust 1.98 (`ethnum`) (v2.8.4)
- [x] Undo/redo in the IDE: snapshot-based project history, menu/toolbar/Ctrl+Z/Ctrl+Shift+Z/Ctrl+Y, 100 steps, `tests/web_ide_undo.mjs` (v2.9.0)
- [x] `tests/web_ide_bugfixes.mjs` and `tests/web_ide_phaseF.mjs` pass; every `tests/web_ide_*.mjs` runs in `tools/regress.sh` (local; CI is manual-only) (v2.71.0; the suites were retired with the old IDE on 2026-10-08: docs/studio-wow.md §7)

**Rest of Phase 0**
- [x] Upgrade `mysql` crate to drop `proc-macro-error2` (v2.113.0: mysql 28 without derive / TLS / system zlib) (unmaintained, future-incompatible: will stop compiling on a future Rust like `ethnum` did)
- [ ] Confirm first CI run on GitHub (Linux system packages for the UI kernel host and ALSA untested)
- [x] Local checks on the other systems (no remote CI): Linux in Docker (`tools/linux/check.sh`: build, conformance, GUI events headless and through Xvfb) and an Ubuntu 24.04 ARM VM with real Wayland windows; a Windows 11 ARM VM — build (RHTTP on SChannel: no clang), conformance, the GUI events headless and with real windows, UI Automation (`tools/windows/uia_probe.ps1`). Found and fixed: `.exe` names, Windows paths in the generated Cargo.toml, INPUT$ from a pipe, Wayland's applied sizes, labels' text for screen readers, WARP crashing vello (software GPUs draw on the CPU) (v2.115.0)
- [x] `SECURITY.md` → GitHub private vulnerability reporting (repo setting must be enabled by owner)
- [x] Track all of `tests/` in git (generated outputs ignored)
- [ ] Run the web IDE Playwright suites in CI (wasm-pack build + static server + Playwright)
- [x] SEC-07: SQL parameter binding (SQLite, MySQL, web SQLite) — `Query(sql, values…)`, `AddParam`, `ClearParams`; the web runs SQLite itself (wasm) instead of its imitation (v2.114.0)
- [x] CSP generated per bundle from components used (`'unsafe-eval'` only with RJAVASCRIPT; `connect-src https: wss:` with RHTTP / RSOCKET / RDOWNLOAD, plus the origins the program spells out; `frame-src` only with RWEBVIEW; `object-src 'none'`, `base-uri 'none'`); the author's additions by `--csp`; `_headers` / `.htaccess` with the same policy plus `frame-ancestors` (SEC-15, docs/security-audit.md)
- [x] SEC-12: RWEBVIEW's frame defaults to no `allow-same-origin` (its `Sandbox` is the opt-in), and its Html runs in `rapidr-webview.html`, a frame of its own, not in an `srcdoc` that would inherit the page's policy (SEC-15)
- [x] SEC-16: the project's and its files' names escaped for where they go (HTML text / attribute, JS string, URL, file name); no name is written into a script (loader.js and start.js are fixed files)
- [x] SEC-17: the release's web bundle is RapidR Studio's (`tools/release/web.sh` → `tools/build_studio_web.sh`); program bundles carry only rapidr-webbundle's own `web/` files; the old HTML / Monaco IDE is deleted (2026-10-08)
- [x] SEC-18: `rapidr lsp` reads only the workspace folders and the open documents' folders; `rapidr dap` launches only RapidR programs and passes no loader variables (`LD_*`, `DYLD_*`, `PATH` …); `rapidr mcp`'s authentication designed (docs/security-audit.md §7)
- [x] SEC-20: memmap2 0.9.11, anyhow 1.0.104, `unsound = "all"` in deny.toml, stale ignore gone, `.cargo/audit.toml`; ttf-parser parses only RapidR's built-in fonts (pinned by tests/security/supply_chain.mjs)
- [ ] Sign `SHA256SUMS` (minisign proposed, docs/security-audit.md §8): Robert to decide on the key
- [ ] Single language registry → generate `lang-data.js`, VS Code data, manual sections (planned as the IDE's stage I0, [docs/ide-plan.md](docs/ide-plan.md))

## Phase 1 — RapidQ & VB compatibility (~6–8 weeks)

**Direction (2026-09-25):** no Windows-only compatibility. RapidQ's own
features (built-ins, components, console, dialogs) are mapped to portable
Rust implementations on desktop and web. Calls into Windows DLLs stay a
clear error (naming a RapidR equivalent when one exists). Any library we
add must be open source with a permissive license (`deny.toml`) and be
credited: `THIRD_PARTY_NOTICES.md` (generated, checked in CI), README and
the IDE's About dialog.

**Refined (2026-10-03):** RapidQ had a Linux version too — everything the
Windows and Linux RapidQ shared (its standard API) gets 100 % support, and
that's the "portable corpus" we measure. RapidQ's DirectX objects (QDXSCREEN,
QD3D*, QDXTIMER, QDXIMAGELIST, QDXJOYSTICK, …) are translated, not emulated,
onto **wgpu** (MIT / Apache: commercial programs allowed), which maps to
DirectX 12, Vulkan, Metal and WebGPU, so the same 3D program runs on the
desktop and the web. OLE / COM stays last (Windows-only builds).

- [x] Portable corpus classified by `tools/rapidq_corpus.py` (v2.103.0): of RapidQ's 386 examples, 123 use only the shared API — **all 123 compile** (169 call DLLs, 32 DirectX, 15 OLE, 7 DOS-era port I/O, 17 miss an include / resource the corpus doesn't have, 23 aren't RapidQ: other BASICs' code or typos, each listed with its reason in the tool)
- [x] The portable corpus *running* alike native vs interpreted (`tools/corpus_compare.mjs`, v2.104.0): every program that doesn't depend on random numbers / the clock / the network behaves identically (8 differences found and fixed)
- [x] RapidQ's own compiler as the ground truth ([docs/rapidq-ground-truth.md](docs/rapidq-ground-truth.md)): RC.EXE runs in a Windows VM, `tools/rapidq_truth.py` compares its programs' output with RapidR's (conformance cases, console corpus programs, probes). ~30 differences found and fixed on native, interpreted and web: PRINT's numbers (9 decimals, 32-bit whole numbers) and comma (no zones), STR$ (9 digits), INT / ROUND / CINT / CEIL / FLOOR, stores truncating vs parameters rounding, SINGLE, DWORD, `\` / MOD / bit operators, division by zero, NaN, VAL, HEX$, REPLACE$, INV, `""` in strings, STRING * n, implicit SUB variables, DIM without AS, IF … PRINT … ELSE, CASE IS … AND, property sets, Boolean properties reading 1, TIMER; judgment calls listed there
- [ ] RC.EXE ground truth, next: the GUI side (component defaults and events, read back through a console probe), the console statements on screen (`s_*` probes), RESOURCE(), QREGISTRY under `HKCU\Software\RapidR-Test`; the last digits of PRINT for values ≥ 1E7
- [x] The same comparison against the web runtime (`tests/corpus_web_compare.mjs`, v2.105.0): output plus every form's and component's properties as the program reads them, browser vs desktop interpreter — every program that doesn't depend on random numbers / the clock / the network / the machine (`Application.ExeName`, `CURDIR$`) behaves identically; 12 differences found and fixed (forms shown unasked, OnShow missing on the web, files and `$RESOURCE`s in any case, unparented components, button padding, menu height, fractional geometry, `Parent = QFORM`, desktop SHOWMESSAGE)
- [x] Desktop: `Form.Visible = True` shows the form and fires OnShow, as Show does (RapidQ/Delphi; it did nothing for a form not shown yet) — `Visible = 1` inside the form's CREATE shows it once the program waits; OnShow from `Visible = True` on the web too (v2.106.0)
- [x] QFORM's Visible reads False until the form is shown (RapidQ's default; both runtimes read True) (v2.106.0)
- [x] Web: the VM yields to the page every few milliseconds, so a busy loop (`cpuhog: GOTO cpuhog`, a game loop without DoEvents) doesn't freeze the tab — the IDE's Stop always works (v2.106.0, `tests/web_vm_yield.mjs`)
- [ ] DirectX objects on wgpu (desktop and web): QDXSCREEN (2D surface, sprites, blits), QDXIMAGELIST, QDXTIMER, QDXJOYSTICK (gamepads: `gilrs`), QD3D* (meshes, textures, frames, lights, camera) as a retained-mode scene drawn by wgpu — staged in [docs/directx-plan.md](docs/directx-plan.md)
  - [x] D1: QDXSCREEN 2D (back buffer, Flip, drawing, Init / AutoSize / AllowStretch, set-up events), QDXIMAGELIST (`.DXG` libraries), QDXTIMER (Interval 0, FrameRate) on native, interpreted and web (`dx_screen` fixture)
  - [x] D1b: FullScreen, ActiveOnly, Rotate, View.*, Cursor, a screen added to a shown form, the screen font (MS Sans Serif 8) (`dx_more` fixture)
  - [x] D2: QDXSOUND (rodio / Web Audio) (`dx_sound` fixture)
  - [x] D3–D4: the QD3D* scene and API, the `.X` loader, the software rasterizer, shadows (`d3d_scene`, `d3d_xfile` fixtures; RapidQ's own look can't be compared: `d3drm.dll` left Windows with Vista) (v2.116.0)
  - [ ] D5: the wgpu renderer — **parked** (the software rasterizer is fast enough: Park.x, 29k faces, 4.4 ms at 1×, 8.1 ms at 2×; revisit when a real program measurably needs it — docs/directx-plan.md "Stage D5: parked")
  - [x] D6: joysticks — RapidQ's undocumented QDXJOYSTICK (IsLeft … Button(n), Update; found in RC.EXE) plus RapidR's X / Buttons / POV / events, on gilrs (Windows, macOS), evdev (Linux) and the Gamepad API (`dx_joystick` fixture)

Next up, in order:
- [x] `THIRD_PARTY_NOTICES.md` generated from the real dependency graph (`tools/third_party_notices.py`, `--check` in CI); linked from README, LICENSES.md and the IDE About dialog; shipped in every web bundle; native C/C++ libraries credited in LICENSES.md §7 (v2.16.1)
- [x] Licence compliance for users: every output carries a generated `THIRD-PARTY-NOTICES.txt` (native / interpreted executables beside them, web builds in their root, linked from index.html) with every licence text, from the build's real graph (`rapidr notices`, offline in an install); `LEGAL.md` (what users may do, what to ship, trademarks, no warranty), `docs/licensing.md` (per-output tables, licence obligations, codecs and patents); RapidQ-derived library bodies and manual-example tests rewritten; `tools/regress.sh legal` (cargo deny, the notices for every kind). Open: a professional review before the first release (docs/licensing.md §8)
- [x] Legal hardening: nothing copyleft, cryptographic or data-licensed in any program — HTTPS on the OS's TLS (Security.framework, SChannel, the system's OpenSSL 3 on Linux; ring/rustls/webpki-roots gone), MP3 by nanomp3 (symphonia's MPL gone), charts' text in Liberation Sans via ab_glyph (font-kit, dwrote, option-ext, FreeType gone), winit's KDE blur bindings replaced by RapidR's stand-in (`crates/patches/wayland-protocols-plasma`); shipped graphs allow only the permissive list (deny.toml, notices.rs `ALLOWED`/`BANNED`, check_notices.py), `regress.sh legal` exits 1 on any violation. Open: AT-SPI interface names (docs/licensing.md §8.1)
- [x] RapidQ review (`docs/legal/rapidq-review.md`): RapidQ's terms and rights (freeware; rights sold to REAL Software, now Xojo, in 2000), trademark searches, the law on re-implementing it, RC.EXE as a black box; full-history provenance scan (no RapidQ file ever committed); built-in RAPIDQ.INC constants regrouped by public origin (same names and values, pinned); the last example-derived test code, fixture data and manual quotes rewritten; CONTRIBUTING.md, NOTICE, `docs/legal/clean-room.md`. Open for the owner: keep or rename "RapidR", `rapidr.dev`, a DNPI filing, GitHub's cache of the rewritten commits (review §13)
- [x] Windows DLL calls: error says RapidR doesn't emulate Windows and names the portable equivalent (SHELL, RCANVAS, RSQLITE, RSOCKET, …) (v2.17.0)
- [x] Console: `CLS`, `COLOR`, `LOCATE`, `CSRLIN`, `POS` as ANSI sequences on both backends; the IDE Output panel renders them (v2.17.0; now RapidR Studio's ROutputConsole, `rapidr_value::panels::console`)
- [x] Web bundles: an on-page console for programs that PRINT (`bundle_console.js` + `ansi_screen.js`, now `interpreter/rapidr-webbundle/web/`) (v2.21.0)
- [x] Omitted arguments (`INSTR(, a, b)`, `COLOR , 1`); builtins without parentheses (`TIMER`, `CSRLIN`, …) in the VM too (v2.17.0)
- [x] `REDIM` keeping data (resized in place; creates the array without a DIM; `REDIM PRESERVE`), `INV` (v2.18.0)
- [x] Dialogs with buttons: `MESSAGEBOX`, `MESSAGEDLG` (dialogs the UI kernel draws on the desktop; browser alert/confirm on the web, which can't offer a third button) (v2.19.0)
- [x] Web: in-page dialogs with any buttons for MESSAGEBOX/MESSAGEDLG/SHOWMESSAGE/INPUT — the VM suspends and resumes (`VmError::Suspended`, `Vm::resume_with`) (v2.22.0)
- [x] `INPUT` per the manual: prompt printed, whole line, stored as text/number by DIM type or suffix (both backends) (v2.22.0)
- [x] `SLEEP` in seconds (as RapidQ), `DOEVENTS` (the desktop runs its pending events and timers; the web pauses the program so the browser goes on), `INKEY$` (QBasic's keys: the terminal, the program's windows, the page), timers enabled again ticking again, on native, interpreter and web (v2.80.0)
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
- [x] QSTRINGGRID runtime: one model for desktop and web; Cell(col,row), sizes, fixed rows/cols, insert/delete/swap, Separator files/streams, selection, in-place editing, ellipsis columns, OnSelectCell/OnSetEditText/OnEllipsisClick (v2.34.0)
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
- [x] Native builds: a line label / `GOSUB` inside `WITH` or `CREATE` (v2.91.0)
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
- [ ] Grid `OnDrawCell` text from the shared fonts in the browser too (the desktop's UI kernel draws it with them; `ExtendedSelect` for plain multi-select lists came with the shared list model the kernel draws)
- [x] Desktop look: `$THEME` honored by native and interpreted programs (v2.62.0); since FLTK's removal (Stage 11) the UI kernel draws Windows' classic look (RapidQ's) and says once when a program names another
- [x] Kernel themes beside the classic look: a modern one for new programs and a high-contrast one, drawn from the same models (`$THEME` picks; old programs keep classic) — modern, dark, highcontrast; `Application.Theme`, `RAPIDR_THEME`, `auto` (v2.116.0); next: the web on the same table (W3+), `auto` following a change while running, the code editor's colours
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
- [ ] RapidQ's include folder: 81 of 108 libraries compile (`./rapidr build-bc` on each `include/*.inc` from that folder; 76 before dotted fields, 78 before templates — an earlier "85" was counted another way); the rest call the Windows API (SENDMESSAGE, GetDC, …), miss include files, or have typos (`&HFFFF0000???`, `TYPE X<Size>`)
- [ ] Example corpus: 158 of 386 compile (`python3 tools/rapidq_corpus.py ~/Downloads/Rapidq/examples --include ~/Downloads/Rapidq/include`); most of the rest call the Windows API, miss include files, or have typos RapidQ couldn't compile either
- [x] SVG wherever a bitmap goes — QBITMAP / QIMAGE (`LoadFromFile`, `BMP`, `BMPHandle`), QIMAGELIST (`AddBMPFile`, `AddBMPHandle`), `Canvas.Draw`, `$RESOURCE` — drawn by resvg in the shared model with soft edges (per-pixel alpha, kept through `.BMP`) on native, interpreter and web (v2.67.0)
- [x] SVG form / application icons (v2.73.0)
- [x] QOUTLINE (a tree view: AddLines by indentation, AddChild(Index, S), Insert, Item(i), Row, LineCount); QOPENDIALOG / QSAVEDIALOG / QFILEDIALOG from a shared model (RapidQ filters, FilterIndex, InitialDir, DefaultExt, MultiSelect, Files(), SelCount, FileTitle; an in-page dialog on the web with Upload) on native, interpreter and web (v2.71.0)
- [x] QHEADER from a shared model: AddSections, Clear, `Sections(i)` Caption / Width / MinWidth / MaxWidth / Alignment / AllowClick / Style, drawn on like a canvas; sections clicked and resized with the mouse (resize cursor on an edge), OnSectionClick / OnSectionTrack (begin, move, end) / OnSectionResize, owner-drawn sections through OnDrawSection (Index, Pressed, Rect) on native, interpreter and web (v2.72.0)
- [x] High-DPI: canvases, form surfaces, QIMAGE pictures, owner-drawn list / combo items, grid images and tree icons shown at the screen's scale (the window's scale factor on Retina, the browser's `devicePixelRatio`): each bitmap keeps what the screen shows next to the pixels programs read (`Pixel`, `.BMP`, flood fills — unchanged, checked by the GUI suites at 2×); text, lines and ellipses drawn finer, SVGs drawn at the scale (v2.70.0)
- [x] High-DPI: grid cells' own drawing on the web at the screen's scale (v2.75.0)
- [ ] High-DPI leftover: RapidR's own IDE icons as vectors
- [x] RapidQ's UI and data objects from that list, checked against RC.EXE and RapidQ's include libraries, on native, interpreter and web: QRECT and QNOTIFYICONDATA (RC.EXE's records and errors), the system tray (Shell_NotifyIcon onto macOS' status items, Windows' notification area, Linux StatusNotifierItem, a strip on the web; a form's WndProc hears it), QBEVEL and QDIGDISPLAY built in (the include's own TYPE when included), QDIRLISTVIEW and QDOCKFORM as RapidR's own BASIC libraries, QGLASSFRAME; DIR$ / FileRec as RapidQ's (Unreleased)
- [ ] The tray's Windows and Linux code built and clicked on those systems; a QDOCKFORM dragged with real input
- [ ] The rest of `rapidr_ast::RAPIDQ_OBJECTS_NOT_YET_IMPLEMENTED`
  - I/O and media lane (docs/io-media-plan.md): [x] QCGI and the ENVIRON statement · [x] QCOMPORT · [x] QDOWNLOAD · [x] QMIDI · [x] QWAVE · [x] QCDAUDIO (no drive) · [x] QVIDEO (AVI: DIB, RLE, Video 1, Cinepak, MJPEG; PCM sound) · [x] QMIDI's built-in synthesizer
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
- [x] `$TYPECHECK ON/OFF` and `$OPTION EXPLICIT`: RapidQ's `Undeclared identifier` (v2.85.0) for stores and `Undefined symbol` for reads (RC.EXE-checked)
- [x] RapidQ's argument-count, duplicate-DIM and RESULT-outside-FUNCTION errors (v2.86.0); its other messages are in `.reference/rapidq-compiler-messages.txt`
- [x] RapidQ's `Property X of Y is read-only.` for its components' read-only properties (v2.87.0)
- [x] INKEY$ extended keys as CHR$(27) + scan code (RapidQ's manual) and `$OPTION INKEY$ TRAPALL/DEFAULT` (v2.88.0)
- [x] RapidQ built-ins QUICKSORT, TAB, ATAN, GET$, SETCONSOLETITLE, CHDRIVE (v2.90.0); LPRINT / LFLUSH; bare property reads inside CREATE; an undeclared variable only read is 0 (v2.91.0)
- [x] QSOCKET's numbered-socket API, QFORM Add/DelBorderIcons (v2.93.0); menus from a shared model: ShortCut, Checked, RadioItem, Enabled, MenuIndex, AddItems/Insert/DelItems/DelIndex, QPOPUPMENU Popup / AutoPopup (v2.94.0)
- [x] RapidQ API audit (manual vs runtimes, `scratch` script): QEDIT / QRICHEDIT (v2.97.0), QTRACKBAR (v2.98.0), QTABCONTROL (v2.99.0), QFORM / QSCROLLBOX AutoScroll (v2.100.0), QREGISTRY (v2.101.0)
- [x] QREGISTRY on Windows' own registry in native and interpreted Windows builds (the per-user store elsewhere; `RAPIDR_REGISTRY` still names a file): one set of TRegistry answers for both, checked on Windows 11 ARM (`tools/windows/registry_check.ps1`, an `--ignored` unit test) (v2.116.0)
- [ ] First public release once RapidQ compatibility and the MDI IDE are done: release notes saying RapidR targets full RapidQ compatibility on all three runtimes (native compiler, interpreter, web), extends it (data-science stack, AI stack, …), and is not a clone of RapidQ — an original implementation written from the ground up in pure Rust
  - [ ] Release binaries on a GitHub release, built locally (no remote CI): download, install, start working. macOS (universal: arm64 + x86_64, .dmg / .pkg), Windows (x64 + ARM64, an installer), Linux (x86_64 + aarch64: .deb, .tar.gz / AppImage); checksums + SBOM; the web IDE as a static bundle (tools: `tools/release/`, `docs/release-packaging.md` — every artifact built locally and smoke-tested: macOS universal on the Mac, Linux x86_64 + aarch64 (glibc 2.31 baseline, Zig) in the Ubuntu VM, Windows x64 + ARM64 in the Windows VM; original icons)
  - [x] Programs built as proper apps with icons (B-PKG, Unreleased): `rapidr build` makes `Name.app` on macOS (Info.plist, `.icns`, signed ad hoc; the interpreted program's bytecode in Resources), the `.exe`'s icon and version resources on Windows (pure Rust, cross builds too), `Name.AppDir` on Linux (`rapidr install-app` for the menu); the icon from `--icon` / `.rrproj` / `$OPTION ICON` / RC.EXE's `-g` as .icns, .ico, .png or .svg, else RapidR's program icon (`design/brand`); Studio's Build, Reveal and Project Options; `tools/studio_app.sh` (RapidR Studio.app from a checkout). Checked in Finder (double-click), Windows 11's Explorer, Ubuntu's dock and app grid. Left: Developer ID signing / notarization and Authenticode signing of users' apps (their certificates); an AppImage packer
  - [ ] A downloaded install builds programs on its own: interpreted standalone executables with no Rust installed; native builds with the runtime's sources shipped (vendored, offline) and the Rust toolchain set up by the installer or `rapidr setup` (done: `crates/rapidr-cli/src/home.rs`, vendored offline builds checked with an empty cargo home; Windows ships a trimmed LLVM-MinGW — no Visual Studio)
  - [ ] A RapidR Runtime of its own (installable without the IDE, like a JRE) that registers the file types per user: `.rrbc` (compiled program) runs on double-click; `.rr` / `.bas` open in the IDE with a "Run" action; console vs windowed launcher chosen from the program's APPTYPE; the bytecode header carries its format and minimum runtime version (a clear "needs RapidR Runtime ≥ x.y" message); `#!/usr/bin/env rapidr` scripts on macOS/Linux; a downloaded file (quarantine / Mark of the Web) asks once before it runs; uninstall removes the associations. Standalone executables stay for programs that ship without the runtime (built: bytecode format 3, `rapidr run/open/info/ide`, `rapidrw`, per-OS registrations checked and removed in the smoke tests)
  - [ ] Volunteer testing: a "Help test RapidR" section and badge in the README, and a pinned "Testing RC x.y" issue with a per-platform checklist, at each release candidate. The issue forms are in place: bug report, RapidQ compatibility, test report (`.github/ISSUE_TEMPLATE/`)
  - [ ] **Release quality gates** — nothing ships until all are green: (1) fresh-user journeys on each platform from a clean account (download → install → RapidR Studio → run an example → design a form → native + web build → double-click a `.rrbc` → clean uninstall; the machine left as found — e.g. `rapidr setup` never changes an existing Rust's defaults); (2) compatibility: all suites on the three runtimes + the RapidQ corpus against RC.EXE; (3) security review of everything touching the user's system (installers, file types, setup, the IDE's MCP server, network) + fuzzing the parser and the bytecode loader; (4) accessibility with real screen readers (VoiceOver, Narrator/NVDA, Orca) and keyboard only; (5) performance budgets measured per platform (startup, typing latency, build times, sizes); (6) legal: the licence guard, notices in every output, LEGAL.md, and **RapidR's own names everywhere** (the user, 2026-10-08: RButton, RLabel, RForm… in Studio, docs, templates and examples; QBUTTON and the other RapidQ names stay accepted by the compiler silently, no flag; "Import RapidQ Project or File…" / `rapidr import-rapidq` converts a copy, originals untouched; `examples/rapidq/` stays RapidQ-style as the compatibility demo); (7) the "WOW" pass: first run, polish, docs, examples, a short demo (CSV → live chart in the designer, AI help, one source on desktop and web) — the checklist is [docs/studio-wow.md](docs/studio-wow.md): its 79 P0 items (5 done, 36 partial, 38 missing on 2026-10-08) all green
  - [ ] Installers checked in the VMs (Windows 11 ARM incl. x64 emulation, Ubuntu ARM) and on this Mac; code signing / notarization (Apple Developer ID, Windows certificate) — the user's decision
  - [x] The examples for newcomers: `examples/` by topic (26 programs and the IDE, all RapidR's own, media made by `tools/make_example_media.py`), indexed by `examples/README.md`; each one RUNS on every runtime it claims (`tests/examples_run.mjs`, the `examples` stage of `tools/regress.sh`); shipped in SDK installs, `rapidr examples` lists and copies them
- [x] Default component sizes as RapidQ's, the same on every runtime (the desktop's QBUTTON is 80 × 25, the web's 100 × 30; `tools/RQInclude.bi` lists RapidQ's: QBUTTON 75 × 25, QEDIT 120 × 25, QPANEL 150 × 100, …)
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
- [x] **RapidQ importer, names** (R-NAMES phase 1, 2026-10-08): `rapidr import-rapidq <file | folder | .rrproj>` → a copy with RapidR's names (token-aware on the compiler's parse), `$INCLUDE`s followed (`.bas` / `.rqw` / `.rqb` / `.rq` / `.inc`), a report, each program proved to compile to identical bytecode (RapidQ's corpus: 175 / 175 of the programs RapidR compiles); `rapidr upgrade-names <file> [--dry-run]` (crates/rapidr-import, docs/q-and-r-components.md §6)
- [ ] **RapidQ importer, the rest**: zip input; `CREATE` trees become designer forms; modules; a compatibility report (what RapidR doesn't run yet, by program); RapidR Studio's File ▸ Import (R-NAMES phase 2)
- [ ] **VB6 importer**: `.vbp` + `.frm` (`Begin VB.Form …`) + `.bas`
- [ ] Imported projects with `RUSTSTART`/`DECLARE LIB` flagged native-privileged (SEC-10)
- [x] Corpus: the original RapidQ distribution (386 example programs, 126 includes, manual) → `tools/rapidq_corpus.py` reports compile pass-rate and top blockers (v2.15.0: 82/386 compile, from 46)
- [ ] Corpus: raise the compile rate; then run programs, not just compile them (golden outputs for console examples — started: `.reference/rapidq_golden/` (local only, never committed), RapidQ's own output of the deterministic console examples, `tools/rapidq_truth.py golden`)
- [ ] **Needs from user:** real RapidQ programs and any RapidQ IDE project-file samples (format not yet confirmed)

## Phase 1B — Modern foundations (high-DPI, accessibility, responsive)

Planned 2026-10-03 with the user: RapidQ was made for 96-dpi screens; today's are high-DPI and vector. Compatible *and* modern, transparently.

**High-DPI (principle 6)**
- [ ] Audit every runtime path against principle 6 at 1×, 1.5×, 2×, 3× (the web parity suite and, since v2.106.0, the desktop GUI events run at 2× in `tools/regress.sh`; next: screenshots compared per scale, 1.5× and 3×)
- [ ] What programs read stays logical: Left / Top / Width / Height, ClientWidth, `Screen.Width` / `Height` (logical, as browsers' CSS pixels and macOS points), and `Screen.PixelsPerInch` stays 96 (Delphi programs that scale by `PixelsPerInch / 96` would otherwise scale twice)
- [x] New, additive: `Screen.Scale` / `Form.Scale` (device pixels per logical pixel), `OnScaleChanged` (a form moved to a screen with another scale) (v2.106.0; OnScaleChanged not yet tried on a real two-screen setup)
- [ ] Drawing surfaces (QCANVAS, a form's own, QBITMAP): the pixel API stays logical (`Pixel`, `PSET`, BMP in and out), vector drawing (Line, Circle, Rectangle, TextOut, fills) is drawn at device resolution — finish what `bitmap.rs`'s HiRes layer started, for every drawing method
- [ ] Images: bitmaps drawn at their logical size, smoothly scaled; SVG accepted everywhere a picture is (QIMAGE, icons, QIMAGELIST, buttons' glyphs, `$RESOURCE`); `name@2x.png` / `@3x` picked automatically when present
- [ ] `$OPTION SCALING LEGACY`: a program that needs exact pixels (pixel art, screen grabbing) draws at 1× and is enlarged as a whole, crisp (nearest neighbour)
- [ ] Optional: follow the system text size (`Application.FollowSystemTextSize`, off by default: it changes layouts)

**Accessibility (principle 8)**
- [ ] Shared models describe themselves: role, name (Caption / Text / Hint), value, state, actions — one accessibility tree per form
- [x] Desktop: AccessKit (MIT / Apache) from that tree; web: ARIA roles and live regions from the same tree (v2.113.0); keyboard: TabOrder, visible focus, mnemonics (`&File`), Escape / Enter on dialogs
- [x] New, additive: `AccessibleName`, `AccessibleDescription`; a high-contrast theme (v2.116.0: `$THEME highcontrast`)
- [ ] No toolkit widgets are left on the desktop (FLTK removed): every component is the kernel's and gains accessibility through its model's description — track the ones whose description is still generic

**Responsive layout (additive to Align)**
- [x] `Anchors` (Delphi's akLeft / akTop / akRight / akBottom) and `Constraints` (MinWidth …) on every component, in `rapidr_value::layout` (v2.107.0)
- [ ] Flow and grid containers (Delphi's TFlowPanel / TGridPanel) for layouts that reflow on small screens — the base for mobile

**One UI kernel (principle 7)**
- [ ] Move the remaining components to shared models: button / check box / radio button / label / panel / group box / combo box / status bar / tool bar / progress / up-down / date picker
- [x] **New desktop host (decided direction 2026-10-03, after a prototype):** replace FLTK with RapidR's own UI kernel on permissive Rust crates — `winit` (windows and input; Windows / macOS / Linux / iOS / Android), `wgpu` + `vello` (GPU vector drawing; `tiny-skia` CPU fallback; wgpu is also the DirectX objects' layer), `AccessKit` (screen readers), `parley` / `cosmic-text` (text shaping, editing, IME), native where users notice: macOS menu bar (`muda`), file / colour / font dialogs (`rfd`), clipboard (`arboard`). Not native widgets (wxWidgets & co.): they look and measure differently per OS and don't exist on the web, against "the same everywhere". Steps: prototype host next to FLTK running a few fixtures → port component by component (text editing and QRICHEDIT last) → switch when the whole regression passes → FLTK removed. Timing: after the corpus push, before the IDE (the IDE is built on it). Themes: Windows-classic for old programs, a modern one for new
  - [x] Prototype (`crates/rapidr-ui-proto`, its own lockfile; v2.105.0): a QFORM with QLABEL, QBUTTON, QEDIT (parley editing, IME commits, clipboard), QTRACKBAR and QTABCONTROL drawn by vello from the shared models' ops, logical pixels at the screen's scale (`RAPIDR_SCALE`), Tab focus, AccessKit tree (VoiceOver reads and presses it), muda menu bar, rfd dialog, offscreen `--capture`; 13 MB release binary, ~0.9 ms per frame. All 170 dependencies permissive
  - [x] Stage 0 of the integration (v2.108.0): pump-driven event loop with nested modals, timers and live resize; CPU renderer (vello_cpu) within tolerance of the GPU one; headless host with byte-identical captures; wasm-bindgen aligned at 0.2.129 across the workspace; one font size rule (Windows' rounding) — results in the plan
  - [x] Stage 1 (facade, host switch, deferred-handler safety net, shared test-hook parsing) and Stage 2 (`rapidr-ui-kernel`) (v2.109.0)
  - [x] Stage 3: `rapidr-ui-host-winit` + the kernel glue; 16 of 54 GUI fixtures identical on FLTK and the kernel (native and interpreted, 1× and 2×); the hosts matrix in `tools/regress.sh` (v2.110.0) — next: the component lanes (Stage 5), surfaces, text, dialogs, platform
  - [x] Stage 5 lanes — containers, buttons & menus, lists: 43 of 54 GUI fixtures identical on both hosts (v2.111.0) — next: surfaces, text, dialogs, platform
  - [x] Stages 6–9 (surfaces, text, dialogs, platform): every GUI fixture identical on both hosts — 0 pending (v2.112.0). Before the switch (Stage 11): a hands-on check on an unlocked screen (GPU windows, a real drag, a held menu, IME, a real file sheet), then RDESIGNSURFACE / RCODEEDITOR (Stage 10)
  - [x] Stage 10 (RDESIGNSURFACE / RCODEEDITOR as shared models; the IDE in RapidR runs on the kernel), Stage 12 (web ARIA from the same accessibility rules, `tests/web_a11y.mjs`), and the lanes' follow-ups: double clicks in the VCL's order, real in-place editors, edit after a pause, F10 / Alt menus, size grip, message box icons, colour / font dialog options, QFORM.WindowState (v2.113.0). Left before the switch (Stage 11): the hands-on check on an unlocked screen
  - [x] Stage 11: FLTK removed — the UI kernel is RapidR's only desktop host, with no fallback (`gui.rs`, the `fltk` / `fltk-theme` dependencies and `RAPIDR_HOST` gone; `rapidr build --host kernel` only notes it's no longer needed; the GUI suite runs every fixture native and interpreted on the one host, 1× and 2×); sizes and build times before / after in the plan
  - [x] After the switch, checked by hand on a real Mac screen (GPU windows, IME, double clicks, the menu bar, a real Open panel, message boxes, VoiceOver's tree): timers keep firing while a native menu is held; console programs never start the windowing system and run without a display; Linux checked locally in Docker (`tools/linux/check.sh`: conformance 224, GUI events headless and on X11 windows) (v2.114.0). Next: Windows and Ubuntu desktops in VMs (Parallels), kernel themes beyond classic
  - [x] The interpreter serves every wait for the user as it serves ShowModal: MESSAGEBOX / MESSAGEDLG / SHOWMESSAGE / MSGBOX, the Open / Save / colour / font dialogs, INPUT$ and a kernel-drawn Popup — timers' handlers run during them (nested dialogs, a timer closing a modal form under a box), not in a burst afterwards, and the answer comes back as in a native build; a timer whose handler waits doesn't fire again until it returns, and none waits behind another handler that waits, in both builds; dialogs start the program's timers as ShowModal does (`dialog_timers` GUI case, `RAPIDR_TEST_DIALOG_HOLD` / `RAPIDR_TEST_MESSAGE_DIALOG`) (v2.116.0)
- [ ] **The web on the same UI kernel** ([docs/web-host-plan.md](docs/web-host-plan.md)): the kernel drawn on a `<canvas>` (vello_cpu, wasm SIMD), input and IME through hidden text fields, an ARIA mirror of the kernel's accessibility tree, DOM overlays only for web-only components; the DOM runtime deleted once parity holds (no fallback). All licences permissive (fonts SIL OFL); Tailwind not needed
  - [x] W0 spike: byte-identical pixels with the desktop at 1× and 2×, Chrome's accessibility tree equal to the kernel's, ~0.2–0.5 ms a form's frame; W1 `rapidr-ui-render` shared by both hosts (v2.114.0)
  - [x] W2 `rapidr-ui-app`: the host-neutral half of the program glue behind `Program` / `Windows` (v2.115.0) — next: W3 (the web host proper; move the desktop's `Desktop` into the crate too), W4 (dialogs and ShowModal as waits the VM serves)
  - [x] W3 the web host proper (`?host=kernel`; the DOM host stays the default). It covers:
    - windows on the page with kernel-drawn frames, stacking, moving, sizing and WindowState;
    - devicePixelRatio and canvas context loss;
    - input, IME, the clipboard and autofill;
    - the ARIA mirror in Rust;
    - `WebStore` and the web runtime's `Program` / `Windows`;
    - `Desktop` moved into `rapidr-ui-app`;
    - the registry's shared defaults table (step 1);
    - a wasm SIMD build.

    64 of 68 browser GUI cases run on it, with windows byte-identical to the desktop's and equal accessibility trees. Next: W4 (the VM's waits, the kernel's dialogs and timers on the page), then W5–W9 in parallel lanes.
  - [x] W4 the VM's waits (ShowModal, DOEVENTS, INPUT$), the kernel's message boxes, colour / font and Open / Save dialogs (W8 pulled forward), timers on the shared heap, native web builds on the kernel host; resize from every edge, minimized windows, autofill hints: 70 of 70 browser GUI cases on the kernel host
  - [ ] The kernel host as the only web host (the user's direction, 2026-10-05). The HTML web IDE stays as the harness until the kernel-drawn MDI IDE runs in the page.
    - [x] The default for the IDE preview, `bundle-bc` and `rapidr build --web` (`?host=dom` until deletion).
    - [x] Every web suite on it (31 web_* suites, parity, a11y, conformance).
    - [x] Web-only components as overlays, with the popup layer above them (W6).
    - [x] The fallback fonts (W7): Noto symbols and CJK on demand beside the web runtime, shipped in bundles and installs (emoji next).
    - [x] Then the DOM host and the `RAPIDR_WEB_HOST` switches deleted (W11): `gui_web`, `a11y_web`, `menu_web`, the DOM dialogs, `rapidr-rrcss`; the web API components in `webapi_web`.
    - [x] Open / Save on the user's real files through the browser's own pickers (File System Access, a file input, a download where there's no save picker; the IDE shows them for its sandboxed preview); the last in-page dialog (the "Save As" list) gone, and with it the modal-focus bug it had; the modal rule checked with real input for every web dialog (`web_modal_focus`, `web_file_dialogs`).
  - [x] The integration, staged in [docs/desktop-host-plan.md](docs/desktop-host-plan.md) (event loop via winit's `pump_app_events`, kernel / host crates behind `RAPIDR_HOST`, wasm-bindgen aligned so the host joins the workspace, a FLTK × kernel × native × interpreted matrix): Kernel crate (GUI-free: models, ops, focus, input, accessibility) + winit host behind a switch next to FLTK, starting with the components already drawn from shared models (tab control, track bar, scroll bars, list / tree / grid views, menus); then canvas / bitmaps on vello images, QEDIT / QMEMO on parley, QRICHEDIT last; CPU fallback (vello_cpu / tiny-skia); wgpu's wasm-bindgen pin aligned with the web build before it joins the workspace

## Phase 2 — Debugger (~6 weeks)

The IDE plan ([docs/ide-plan.md](docs/ide-plan.md)) schedules the immediate window, conditional breakpoints / hit counts / logpoints / break on error and DAP in stage I6, edit-and-continue and "inspect element" in I5; reverse stepping, the event timeline and the line profiler stay here, after I6.


- [ ] Immediate window: evaluate expressions/statements in the paused frame (compile snippet against frame symbols)
- [ ] Edit & Continue: hot-swap recompiled function when signature unchanged; Set Next Statement
- [ ] Conditional breakpoints, hit counts, logpoints; break on runtime error
- [ ] Reverse stepping: ring buffer of VM state deltas + recorded host-call results
- [ ] Event timeline (event → handler → duration)
- [ ] "Inspect element": click widget in running preview → properties/handlers
- [ ] Line profiler: per-line instruction counts → gutter heatmap
- [ ] `rapidr dap` Debug Adapter Protocol server (VS Code extension + any DAP client)
- [ ] Policy: debug on VM, ship on either backend (guaranteed by differential tests)

## Phase 3 — IDE (stages I0–I9; planned in [docs/ide-plan.md](docs/ide-plan.md))

**Direction (user, 2026-09-28): a professional IDE in the tradition of RapidQ / VB6 / Delphi / Xojo — an MDI workspace on the desktop and on the web alike.** Starts once RapidQ compatibility is complete across native, interpreter and web.

**Planned 2026-10-05** ([docs/ide-plan.md](docs/ide-plan.md), with [docs/ide-components.md](docs/ide-components.md), [docs/ide-ai.md](docs/ide-ai.md), [docs/q-and-r-components.md](docs/q-and-r-components.md)): "pro" like Xojo or Xcode — a WYSIWYG designer with smart guides, a fast editor with real IntelliSense, a debugger, live editing, Delphi / Lazarus-style linked data components with the data-science stack first-class, and AI through MCP. One codebase on the UI kernel for the desktop and the web; every building block a public R component users can use in their own programs (the IDE is assembled from them); the IDE's shell a RapidR program; the program under development in its own process / sandboxed frame, driven by one session protocol. The HTML / Monaco web IDE and `examples/ide.rr` are deleted when I1 reaches parity (no fallback). Open decisions for the user: the plan's §9 (name, project format, how much is RapidR, default AI provider, extension sandboxing, …). Replaces this phase's earlier list: splitting the HTML IDE's `host.js` and a Tauri desktop shell are no longer needed (the HTML IDE goes; the kernel host is the desktop shell); the events tab, menu and tab-order editors, autosave and File System Access are in I1 / I4.

**R-NAMES — RapidR's names everywhere** (decided by Robert 2026-10-08, for the first release; legal is a main reason: visibly original, "compatible with RapidQ", not a clone — [docs/q-and-r-components.md](docs/q-and-r-components.md), plan and results in [docs/ide-plan.md](docs/ide-plan.md#r-names-results-phase-1-and-the-plan-for-phase-2--rapidrs-names-everywhere-2026-10-08))
- [x] Phase 1: RapidR's names (`RButton`) the default in the registry's exports, the manual, the language service (completion, hovers, "RapidQ name: QBUTTON"), the examples (but `examples/rapidq/`), the docs and the template; the compilers always accept both names; the importer and `upgrade-names` (crates/rapidr-import) with the corpus proof; `.rqw` / `.rqb` / `.rq` sources everywhere; a RapidQ-compatible project warns on RapidR's names of RapidQ's components
- [ ] Phase 2 (after the editor and designer lanes merge): RapidR Studio's toolbox, inspector and completion under RapidR's names; the designer and completion follow each file's style (`NameStyle`, never mixed); File ▸ "Import RapidQ Project or File…"; "Upgrade this file to RapidR names"; Studio's own strings and `ide/` code (the R-NAMES legal scan's list in docs/ide-plan.md)
- [ ] Decide (Robert): built-in RAPIDQ.INC constants without the `$INCLUDE` line for RapidR's own files (today the line stays, as in RapidQ)

**I0 — Foundations** (L, 12–16 sessions)
- [ ] The language registry (`rapidr-lang`): every component (Q and R names, which RapidQ has), property (type, default, editor, origin), method, event, builtin, statement and directive, with docs in our words; tests tie it to both runtimes both ways; generates the IDE's completion data, the VS Code data, the manual's reference sections and the AI prompt; `COMPONENT_TYPES` and the runtimes' name lists generated (Phase 0's "single language registry")
- [ ] Project format `.rrproj` v2 (`rapidr-project`), reading the web IDE's JSON v1; plain `.bas` / `.rr` files open without a project
- [ ] The program session protocol (`rapidr-session`): run / stop / pause / breakpoints / stepping / variables / evaluate / set property / output / forms, over a pipe (desktop child process) and a `MessageChannel` (web sandboxed frame); today's `DebugSession` becomes one transport
- [ ] VM: per-file breakpoints through the source map (the web compile path fills it too), pause on demand, an evaluator for a paused frame, writing variables, break on runtime error
- [ ] Parser for tools: comments and directives kept as trivia (corpus files reproduce byte for byte), a byte-level origin map through `$INCLUDE`, lexer error recovery, a public semantic model from bcgen's scopes — no change to any program's meaning

**I1 — Shell** (L, 16–22 sessions)
- [ ] RDockManager: docked / tabbed / auto-hide / floating panels, layouts saved and restored, all by keyboard; the documents area an MDI client on `rapidr_value::mdi` (cascade / tile) or tabs
- [x] RProjectTree, RToolbox (groups "RapidQ" and "RapidR", names as the designer writes them — R-NAMES phase 2 changes both), RPropertyInspector (typed editors from the registry, RapidR extensions badged, Events tab), ROutputConsole (ANSI), RCommandPalette, a real RToolBar kind — in RapidR Studio, properties two-way with the designer and the code (S-PANELS, 2026-10-08)
- [ ] The shell (`ide/`, a RapidR program on public components only), commands and shortcuts (VB6 / Delphi scheme), settings, modern / dark / high-contrast themes with editor colours, vector icons, OFL fonts
- [ ] Run / Stop / Build through `RProgramSession` / `RProject` on both hosts (Build: `RPROJECT.Build` makes the app for the desktop it runs on, B-PKG; the web shows the program instead)
- [ ] Web: the IDE page on the canvas host, IndexedDB / OPFS autosave, File System Access open / save, zip import / export, web bundle build
- [x] The HTML / Monaco web IDE and Monaco deleted (2026-10-08, Robert's decision: RapidR Studio on the UI kernel is the web IDE). Its `tests/web_ide_*.mjs` suites: the web runtime's re-pointed at its own page (`tests/web_run.mjs`) or Studio's run frame (`tests/studio_run_frame.mjs`), the IDE features' listed in docs/studio-wow.md §7, the old internals' deleted
- [ ] Parity with `examples/ide.rr`, then `examples/ide.rr` deleted

**I2 — Code editor** (L, 14–20 sessions)
- [ ] `rapidr-editor`: rope buffer (ropey, MIT), multi-cursor transactions, undo / redo history, find / replace (regex, in selection, in files), bracket matching, auto-indent, folding
- [ ] Declarative language definitions (TOML: tokens, comments, strings, brackets, indentation, folding, snippets) and colour schemes; RapidQ / RapidR BASIC generated from the registry; no tree-sitter
- [ ] RCodeEditor grown on the kernel (the current API kept): virtualized view, gutter, squiggles, completion / hover / signature popups, minimap; RDiffView
- [ ] IME on every host; accessibility with AccessKit text runs and the web mirror's window of lines
- [ ] Performance targets met (plan §6.2: typing p99 ≤ 16 ms desktop / 33 ms web on a 100,000-line file; a 10 MB file opens in ≤ 300 ms)

**I3 — Language service** (L, 14–20 sessions)
- [ ] `rapidr-langsvc` (native and wasm): semantic model, completion of locals / globals / SUBs / FUNCTIONs / TYPEs / components and members (Q and R names alike), hover, signature help, definition, references, rename, outline, live diagnostics in RapidQ's wording, code actions, formatting
- [ ] The "RapidQ-compatible project" diagnostics: every RapidR-only component, member, builtin, statement, directive and non-RapidQ Q name, with code actions
- [ ] `rapidr lsp`; the VS Code extension an LSP client (its regex providers deleted)

**I4 — Visual designer** (L, 16–22 sessions)
- [ ] RFormDesigner: real components drawn WYSIWYG, multi-select, rubber band, eight handles, keyboard nudging / resizing; RDESIGNSURFACE's API on the same model
- [ ] Grid snapping and smart guides: edges, centres, baselines, parent centring, margins, equal spacing with distances
- [ ] Align / distribute / same size / z-order; anchors and constraints editor; containers and reparenting
- [ ] RComponentTray, RTabOrderEditor, RMenuEditor; double-click → event handler
- [ ] Two-way CREATE-block sync with minimal text edits, one undo history with the editor, user code byte-identical; new components written in the file's own style (RapidR's names, or RapidQ's in a RapidQ-style file: R-NAMES), existing names kept
- [ ] The corpus round trip: every form of the 386 RapidQ examples and `examples/` opens and saves byte-identically

**I5 — Live** (M, 8–12 sessions; after the first release)
- [ ] Remote forms: the program's forms as MDI windows in the workspace (display lists + accessibility trees through the session), "inspect element"
- [ ] Live property preview into the running program
- [ ] Hot reload of changed SUBs / FUNCTIONs; edit-and-continue while paused; "restart needed" with the reason otherwise

**I6 — Debugger** (M, 8–12 sessions; the basic part in the first release)
- [ ] Basic: breakpoints in any file with conditions, step in / over / out, pause, run to cursor, call stack, locals / globals / watches evaluated by the VM, data tips, break on runtime error, the immediate window
- [ ] Hit counts, logpoints; frames opened in the data preview
- [ ] `rapidr dap` for VS Code and other DAP clients

**I7 — Linked data and data science** (L, 18–26 sessions; the core in the first release)
- [x] One data-science engine on every runtime (D7 decided: our own, `rapidr_value::datascience` — RNUM, RDATAFRAME and RPLOT's model shared by native, interpreted and web; polars and ndarray dropped; conformance cases `datascience_*` on all three runtimes)
- [ ] RPlot drawn by the kernel (vector, accessible, identical on desktop and web) — replacing today's two renderers (plotters on the desktop, a canvas on the web) of the one chart model
- [ ] Core: RDBConnection / RDBQuery / RDBTable (SQLite), RDataFile (CSV, JSON), `RDataFrame.Source`, RDFFilter / RDFSort / RDFGroup / RDFCompute, RDataSource, RDBGrid, RPlot's declarative binding — live at design time (read-only), with the tray, component-reference pickers and RDataPreview (schema, rows, quick stats)
- [ ] Later: Parquet, MySQL at design time, RDFJoin / RDFSelect / RDFLimit, editable datasets with RDBEdit & co. and RDBNavigator, lookups, column-name completion in expressions

**I8 — Smart: AI through MCP** (L, 16–24 sessions; after the first release — a read-only MCP server is a stretch for it)
- [ ] `rapidr-ai`: Anthropic, OpenAI-compatible (OpenAI, DeepSeek, xAI Grok, Ollama, LM Studio, custom) and Gemini over plain HTTPS, streaming and tool calls normalized, no vendor SDKs; `rapidr ai-proxy` for the web
- [ ] `rapidr-secrets`: macOS Keychain, Windows Credential Manager, Linux Secret Service; web keys per session (opt-in passphrase-encrypted)
- [ ] The IDE's MCP server (`rapidr-mcp`, own implementation): `rapidr mcp` over a user-only local socket with a token, optional loopback HTTP; started / stopped from the IDE; tools for project, edits, forms, run, the running app, debug, data
- [ ] The floating AI button and assistant (RAIChat): context chips, diffs before apply, checkpoints, permission tiers (ask before edits / running), "what was sent"
- [ ] Data privacy levels per project (none / schema / sample / full) enforced in the tool layer, redaction, local models recommended; data tools build transforms and plots ("group sales by month and plot bars")
- [ ] Components as AI tools in users' programs through RAI (the editor, the designer, data sources)

**I9 — Extensions** (M, 8–12 sessions)
- [ ] `.rrext` packages: declarative contributions (languages, themes, snippets, templates) and RapidR code in a capability-filtered VM with fuel limits, talking through the `IDE` object
- [ ] Permissions prompted at install, optional signatures, per-user install on both hosts

**First public release bar** (plan §8): I0–I4, I6's basic part and I7's core, the HTML IDE deleted, the accessibility and performance targets met. The experience bar, item by item with acceptance tests: [docs/studio-wow.md](docs/studio-wow.md).

## Phase 3B — Look: one RapidR look (first release), OS looks, native widgets (after; the user, 2026-10-06)

Programs choose how they look with `$THEME`.
- [ ] **`$THEME rapidr` — the default, in the first release** (the user, 2026-10-06): RapidR Studio's own look for every program (one set of theme tokens shared by Studio and programs), pixel-identical on every OS and on the web, in light, dark and high contrast (following the system's setting unless the program picks one; contrast checked: WCAG AA everywhere, AAA in high contrast). Default font Inter (fonts a program names keep their face); RapidQ's system colours (clBtnFace …) follow the theme, RGB a program sets is painted as written. Compatibility is layout and behaviour, not 1998's paint: component sizes, positions and client areas stay RapidQ's in every theme (a test compares them); text measures in the real font (size chosen by a clipping audit over `examples/` and the RapidQ corpus). Today's `modern` / `dark` / `highcontrast` grow into it; the old names stay as aliases. `$THEME classic` gives RapidQ's exact Windows look (RapidR Sans, readable at every scale); RapidR Studio's "Preview in classic" shows a program in it without editing the source.
- [ ] **`$THEME native`**: drawn themes that follow the OS the program runs on: Windows 11, macOS and Linux (GNOME / Adwaita style), light and dark following the system. On the web the page detects the visitor's OS and uses its theme. A program can also name one directly (`$THEME macos`, `windows11`, `linux`). Our own drawings only, never the OS's images (as Flutter and Qt do).
- [ ] **Native widgets — after the first release** (the user, 2026-10-08: first a working IDE and a working RapidQ out there, then this). A separate, reusable MIT Rust library (working name **Tela**; check the name is free on crates.io) with one backend per OS, called straight from Rust: **AppKit** on macOS and **UIKit** on iOS / iPadOS through the `objc2` crates (MIT), the **Win32 common controls** on Windows (RapidQ's own controls, so its behaviour comes nearly free; modern styles, dark mode, Windows 11 materials; WinUI later if wanted) through `windows-rs` (MIT / Apache). SwiftUI isn't used: RapidQ's model is imperative (a control at x, y, properties, events), which AppKit and UIKit match and SwiftUI doesn't. The UI kernel stays the brain (properties, events, layout, accessibility tree, RapidQ-exact behaviour); the native library only creates the real controls and reports input, so programs don't change. Linux and the web keep the drawn look (GTK and Qt are LGPL / GPL). `$THEME native` selects it; it becomes the default per OS only once a run over the 386 RapidQ examples shows layouts don't clip or overlap (native controls have their own metrics); new Studio projects can use it at once. First step after the release: a macOS proof of concept (a form with a button, a label and an edit in real AppKit, driven by a RapidQ program) to measure the cost. Note: the App Store doesn't require native widgets; store readiness is signing, notarization, the App Sandbox, privacy manifests and MSIX packaging, tracked under release packaging.

## Phase 3C — RapidR on microcontrollers (after the first release; the user, 2026-10-08)

RapidR BASIC running ON an ESP32 (and later other boards), not only talking to one over QCOMPORT.
- [ ] **A target for `rapidr build`**: the ESP32 family through Espressif's official Rust support (esp-rs: `esp-hal`, MIT / Apache-2.0); the RISC-V chips (ESP32-C3 / C6) on stable Rust, the Xtensa ones (ESP32, S3) through Espressif's toolchain. RapidR's native path (BASIC → Rust) makes it a new target, not a new compiler; `no_std` runtime subset for the board.
- [ ] **RapidR objects for the board**: pins (digital, analog, PWM), timers, I²C / SPI / UART, Wi-Fi, Bluetooth LE, deep sleep, sensors; displays (TFT, OLED, e-paper) through `embedded-graphics` and display drivers (MIT / Apache), with an RForm-like drawing model for small screens. Names are RapidR's own (R prefix), docs in our own words.
- [ ] **RapidR Studio**: an "ESP32" target, Run = build + flash (espflash, MIT / Apache) + serial monitor (QCOMPORT's IoT extras), board picker, examples.
- [ ] **Spike first**: blink + a display "Hello" + Wi-Fi on an ESP32 Robert has, measuring binary size and build time.

## Phase 4 — AI in the IDE (~6 weeks)

Planned as the IDE's stage I8 ([docs/ide-plan.md](docs/ide-plan.md), [docs/ide-ai.md](docs/ide-ai.md)): one provider layer over plain HTTPS (Anthropic, OpenAI-compatible incl. DeepSeek / xAI / Ollama / LM Studio, Gemini), keys in the OS keychain, the IDE's own MCP server (`rapidr mcp` over a user-only local socket + token), the floating assistant with diffs before apply, permission tiers, data privacy levels. The items below are kept as the checklist; where they differ, the plan wins (e.g. no Tauri: the kernel host is the desktop shell).


- [ ] Provider adapters: Anthropic Messages + OpenAI-compatible (OpenAI, DeepSeek, OpenRouter, Ollama, custom). Verify DeepSeek CORS; optional proxy fallback
- [ ] Tool registry (MCP-shaped): `read_file`, `apply_edit`, `add_form`, `add_widget`, `set_property`, `compile`, `run`, `inspect_component`, `get_variables`, `click`, `screenshot_preview`, `lookup_docs`
- [ ] System prompt generated from the language registry + examples; `lookup_docs` for details; prompt caching
- [ ] Agent loop: plan → edit → compile → fix → run → verify; checkpoints + undo
- [ ] `rapidr mcp` (stdio) + Tauri MCP server for external agents
- [ ] The IDE's own MCP server, desktop (local socket / stdio) and web (a bridge from the page), so Claude, OpenAI, DeepSeek or any MCP client can drive the IDE: create forms and components, set properties, write handlers, build, run, debug, read diagnostics, take screenshots — with the same permission tiers as the built-in assistant
- [ ] "Fix with AI" quick-fix, inline completions, screenshot → form, "Explain this crash"
- [ ] Security model: keys only in IDE origin (requires SEC-02), optional WebCrypto passphrase, never in `.rrproj`/bundles/logs; permission tiers (read auto / edit auto+checkpoint / run sandboxed, network off by default / external = ask); imported content treated as data

## Phase 5 — `RAI` component (~5 weeks)

Built on I8's provider layer and tool providers ([docs/ide-ai.md](docs/ide-ai.md) §8): the IDE's components (editor, designer, data sources) can be attached to `RAI` as tools in users' programs, with the same data access levels.


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
- [ ] **AI that uses the running app.** The UI kernel's description of a form (the accessibility tree, principle 7) as JSON: each component's type, role, name, text / value, state, bounds and the actions it allows. RAI gets tools on it — read, set text, click, pick, invoke a method, wait for an event — so a model can operate the program the user built (fill a form, edit a document, research on the web and put the result in a grid). The developer decides what's exposed (new `AIVisible` / `AIActions` properties; read-only by default; anything destructive asks through `OnToolCall` or the user)
- [ ] The same as an opt-in, local-only MCP server of a running program (token-authenticated), so external agents can operate RapidR apps too

## Phase 6 — Security audit (continuous + formal pre-3.0)

- [ ] Threat model (`docs/THREAT_MODEL.md`), trust boundaries: untrusted source/projects → compiler/IDE; app ↔ users/data (XSS, SQLi); AI output / prompt injection; native (FFI, `RUSTSTART`, file ops); supply chain; network services (MCP, proxy)
- [ ] `cargo-fuzz` targets: lexer, preprocessor (include recursion, macro blow-up limits), parser, bcgen, `Module::from_bytes`, VM execution with fuel — goal: no panics/hangs
- [ ] Full `unsafe` review + Miri (SEC-08)
- [ ] IDE: strict CSP + Trusted Types; self-host fonts
- [ ] `SECURITY.md` + private disclosure process; Dependabot; release checksums + SBOM
- [ ] Community security review call before 3.0

## Phase 7 — Mobile (after both IDEs are complete)

- [ ] Step 1: the web runtime in a native shell (Tauri 2's mobile targets, MIT / Apache) for iOS and Android — the same behaviour as the web from day one
- [ ] Touch: tap = click, long press = right click, drag / pinch / swipe events (additive), safe areas, the on-screen keyboard
- [ ] Device components (additive): camera, location, sensors, notifications, share, biometrics, files
- [ ] Responsive layouts (Phase 1B) as the way forms fit phones; a phone / tablet preview in both IDEs' designers
- [ ] Building, signing and running on devices from the IDE (iOS needs macOS and Xcode; Android its SDK)
- [ ] Step 2, if needed for speed: the UI kernel's own renderer natively on mobile

---

## Timeline (planned 2026-10-03; adjust as work lands)

| When | Milestone |
|------|-----------|
| Q4 2026 | RapidQ compatibility complete (portable corpus programs compile and run alike on all three runtimes); Phase 1B foundations under way (high-DPI audit, accessibility tree, Anchors) |
| Q1 2027 | IDE stages I0–I4, the basic debugger (I6) and linked data live at design time (I7's core) on the desktop and the web → **first public release** (see the release item in Phase 1 and docs/ide-plan.md §8) |
| Q2 2027 | AI in the IDE and its MCP server (I8), live editing (I5), RAI with tool-calling and AI that uses the running app |
| Q3 2027 | Mobile step 1; data-science / database / AI stacks polished (high-DPI charts, more databases) |
| Q4 2027 | Security audit (Phase 6), documentation, examples, community |

Risks to watch: the corpus' long tail (programs built on Win32 calls need a "portable corpus" definition, since there's no Win32 emulation); desktop accessibility of components whose models still describe themselves generically; the size of the IDE work; app-store tooling for mobile.

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

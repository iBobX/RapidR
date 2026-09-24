# Changelog

All notable changes to RapidR are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the
project uses [Semantic Versioning](https://semver.org/). Planned work lives in
[ROADMAP.md](ROADMAP.md); security finding IDs (`SEC-xx`) refer to it.

## [Unreleased]

## [2.10.0] — 2026-09-24

**Nothing is silently skipped anymore.** Lines the compiler couldn't parse, and calls
to routines that don't exist, used to compile anyway and quietly do nothing in the
IDE and the bytecode interpreter. They're now compile errors with a line and column,
underlined in the editor. Programs that "ran" while ignoring broken lines will now
show those errors; that's intended.

### Added
- **Errors underlined in the IDE editor.** Every compile error is shown as a red
  squiggle in the right form or module, listed in the Errors panel with
  `Form1 (line 6, col 1): …` (click to jump to the line), and re-checked live while
  you type. Run, Debug and Build show all errors instead of one raw message.
- **Parser error reporting.** Every unparseable line is reported, not just the first.
  Messages explain the problem: `'STEP' is a reserved word and can't be used as a
  name`, `GOSUB is not supported yet`, `Unexpected 'y' after the end of the statement`.
  This includes statements inside single-line `IF … THEN`.
- **Unknown SUB/FUNCTION check** in the bytecode compiler, against a shared builtin
  registry (`rapidr-bytecode::builtins`). A test keeps the registry identical to both
  VM hosts' dispatch tables. Specific hints for `INC`/`DEC`, DLL functions
  (`DECLARE … LIB`), `VARPTR`, and line labels.
- The bytecode interpreter now runs `OPEN`, `CLOSE`, `PRINT #`, `WRITE #` and `SEEK`
  (native). These statements were silently skipped before. In the browser, `PRINT #`/
  `WRITE #` report a clear error.
- `SOUND` and `PLAYSOUND` in the bytecode interpreter (both hosts).
- `tests/web_ide_diagnostics.mjs` (14 checks) and new parser unit tests.

### Fixed
- **Identifiers are case-insensitive in the bytecode interpreter**, as in BASIC.
  `total`/`Total`/`TOTAL` were three different variables, and calling `mysub` for
  `SUB MySub` silently did nothing. The debugger still shows names as written.
- **Calls inside `CREATE` blocks** (`Center`, `AddItems(…)`) now call the object being
  created, as in RapidQ and the Rust backend. The interpreter used to drop them, so
  forms were never centered in the IDE.
- A call to an unknown builtin at run time is now an error instead of `null`.
- `examples/web_datascience.rr` used `step` (a reserved word) as a variable. The
  lines were silently skipped, so its trig table computed garbage. Renamed to
  `stepSize`.
- The parser's examples test only looked at `.rp` files, so it tested nothing. It now
  checks all 44 `.rr` examples.

### Changed
- `rapidr_parser::parse_tokens` / `parse_file` return `Result<Program, ParseError>`,
  with every diagnostic. `parse_tokens_recovering` gives a best-effort tree plus
  diagnostics for tools.
- `rapidr build-bc`, `bundle-bc` and `--interp` report errors as `file:line:col`.
- Conformance suite: 11 pass / 19 known failures (was 7 / 23). `case_insensitive`,
  `syntax_error` and `unknown_sub_error` now pass.

### Verification
- `cargo test --workspace` passes, including new parser tests and the builtin-registry
  sync tests.
- 42 of the 44 examples compile with the stricter compiler. The other 2 use native-only
  features (DLL calls, `VARPTR`) and now get a clear error.
- Passing: `web_ide_diagnostics`, `web_ide_smoke`, `web_ide_e2e` (6/6), `web_ide_designer`,
  `web_ide_debugger_test`, `web_ide_preview_isolation`, `web_ide_undo`, `web_ide_round3`,
  `web_ide_round4`, `web_ide_assets`, `web_ide_tree_validation`. (`web_ide_bugfixes` and
  `web_ide_phaseF`: known pre-existing failures only.)

### Added
- `SECURITY.md`: how to report vulnerabilities privately through GitHub's
  "Report a vulnerability", plus supported versions and scope.
- `.claude/launch.json`: a `web-ide` config that serves the repo on
  `http://localhost:8765` (open `/web-ide/index.html`). The IDE must be served over
  HTTP; opening it as a `file://` page breaks module and wasm loading.

- **Conformance suite** (`tests/conformance/`): small BASIC programs with the output
  correct BASIC must produce, run on both the bytecode VM and the Rust codegen backend,
  and part of CI. Known bugs are marked per backend and reported as known failures;
  a case that starts passing fails the run so its marker gets removed. There are 15
  seed cases. It already found about a dozen silent-correctness bugs, listed in
  ROADMAP.md; for example, identifiers are case-sensitive in the VM, and
  `CASE 2, 3` loops forever in the VM.

### Changed
- The whole `tests/` folder is now tracked. Only generated output is ignored:
  `node_modules`, `.matrix`, `.ide-matrix`, `screenshots`, `web-screenshots`, `results`.
- Ten test scripts wrote screenshots to a hard-coded personal path. They now write to
  `tests/screenshots/`, which can be overridden with `RAPIDR_SHOT_DIR`.

## [2.9.0] — 2026-09-24

### Added
- **Undo / Redo in the web IDE.** It was a stub before ("not yet implemented").
  - Works from the Edit menu, the toolbar, and **Ctrl/Cmd+Z**, **Ctrl/Cmd+Shift+Z**
    and **Ctrl/Cmd+Y**.
  - Covers every kind of project edit: adding, moving, resizing and deleting widgets,
    property-grid changes, adding/removing forms and modules, and code.
  - Each user action is one step. A whole drag or a burst of typing counts as a single
    step, grouped after a short pause.
  - History holds up to 100 steps. A new edit clears Redo. New/Open Project and loading
    an example start a fresh history.
  - Inside the code editor, Ctrl/Cmd+Z stays the editor's own fine-grained text undo.
    The toolbar and menu Undo also revert code edits.
  - Undo/Redo grey out when there's nothing to undo or redo.
  - Undo is the safety net the planned AI agent needs before it edits projects.
- `tests/web_ide_undo.mjs` (26 checks). It drives real interactions: toolbox clicks,
  a mouse drag, the property grid, the keyboard, typing in the code editor, and New
  Project. It fails on 2.8.4.

### How it works
- Snapshot-based: after an interaction settles, the project is serialized and compared
  with the last recorded state, so no individual edit site needs changing. Asset data
  (large data URLs) is shared by reference between snapshots rather than copied.

### Verification
- `web_ide_undo` passes (26/26); on the 2.8.4 IDE it fails as expected.
- Checked by hand in the browser: adding a button enables Undo, and Undo removes it
  from the designer and the project tree.
- Passing: `web_ide_smoke`, `web_ide_e2e` (6/6), `web_ide_designer`,
  `web_ide_debugger_test`, `web_ide_preview_isolation`, `web_ide_round3`, `web_ide_round4`,
  `web_ide_assets`, `web_ide_tree_validation`. `web_ide_bugfixes` and `web_ide_phaseF` still
  fail only on their known pre-existing assertions.

## [2.8.4] — 2026-09-24

Supply-chain security release (ROADMAP SEC-09) and toolchain fix.

### Security
- Updated dependencies with published vulnerabilities (all compatible patch releases):
  - `rustls` 0.23.37 → 0.23.45: RUSTSEC-2026-0285, TLS 1.3 handshake messages accepted
    across encryption levels.
  - `rustls-webpki` 0.103.10 → 0.103.15: RUSTSEC-2026-0098 and -0099 (certificate name
    constraints) and RUSTSEC-2026-0104 (panic in CRL parsing).
  - `crossbeam-epoch` 0.9.18 → 0.9.21: RUSTSEC-2026-0204.
  - `rand` 0.9.2 → 0.9.5: RUSTSEC-2026-0097 (unsound).
  - `spin` 0.9.8 → 0.9.9 (yanked version).

  `rustls`/`rustls-webpki` are the TLS stack behind native `RHttp` (via `ureq`).
- Added `deny.toml` and **`cargo deny check`** (advisories, licenses, bans, sources). It
  now passes cleanly. The two remaining unmaintained-crate notices are documented
  exceptions: `proc-macro-error2` is compile-time only, via `mysql`, and `ttf-parser` is an
  optional `fltk` dependency that isn't enabled.
- Added CI (`.github/workflows/ci.yml`): `cargo-deny`, `cargo test --workspace`, and
  a clippy check that fails the build if `js_sys::eval` is used in the web runtime.

### Fixed
- **The native runtime and CLI failed to build on current Rust (1.98).** `ethnum` 1.5.2,
  used by polars, transmuted `()` into `TryFromIntError`, which gained a field. Updated
  to 1.5.3.
- `rapidr version` printed a hard-coded `0.1.0`; it now prints the real version.
- `rapidr-compiler-wasm` and `rapidr-webbundle` were missing the workspace MIT license.

### Changed
- All workspace crates are marked `publish = false` (their internal path dependencies
  already made them unpublishable; this makes it explicit and prevents accidental
  publishing).

### Verification
- `cargo deny check`: advisories ok, bans ok, licenses ok, sources ok.
- `cargo test --workspace`: all 97 tests pass. The CLI rebuilds and compiles and runs
  bytecode programs.
- Lint confirmed both ways: passes on current code, fails when `js_sys::eval` is added.
- Web artifacts rebuilt. Passing: `web_ide_smoke`, `web_ide_e2e` (6/6),
  `web_ide_designer`, `web_ide_debugger_test`, `web_ide_preview_isolation`,
  `web_ide_round3`, `web_ide_round4`.
- The CI workflow hasn't run on GitHub yet; the Linux `test` job's system packages
  are untested until the first push.

## [2.8.3] — 2026-09-24

Security release: programs in the IDE preview are now isolated from the IDE.

### Security
- **Preview sandbox isolation (SEC-02, high).** The preview iframe used
  `sandbox="allow-scripts allow-same-origin"`, which cancels the sandbox: any program run
  in the IDE could read the IDE's `localStorage` (and future AI API keys), read and modify
  the IDE's DOM, and impersonate the IDE. The iframe now runs **without
  `allow-same-origin`**, in an opaque origin. The IDE sends the runtime into the frame at
  boot, because an opaque-origin page can't load same-server modules without CORS.
- **Private message channel (SEC-03, high).** IDE ↔ preview traffic used to go through
  `window.postMessage` with no sender checks, so any window could fake log, status or
  debugger events. The IDE now hands the preview a private `MessageChannel` port in a
  one-time handshake (checked with `e.source`; stale runs are dropped by a generation
  counter), and all traffic uses that port. The IDE no longer listens for preview messages
  on `window`, and the IDE window no longer imports `rapidr_run_bc`.

### Fixed
- `PRINT` output from the preview appeared **twice** in the Output panel. It is now
  delivered once.
- Downloads from programs running in the preview (`SaveToFile`, `RFileStream`) were silently
  blocked by the sandbox; `allow-downloads` is now granted.
- A forged debugger message from another window could throw an error in the IDE; such
  messages are now ignored.

### Changed
- `RWebStorage` / `localStorage` in the IDE preview now goes through a stand-in that
  saves data per project under `rapidr-app-storage:<project>` in the IDE, capped at
  1 MB. `sessionStorage` lasts for one run. Built bundles use the real browser storage,
  unchanged.
- Known limitation, IDE preview only: `RHttp` requests to relative URLs on the IDE's own
  server now need that server to send CORS headers (the `.htaccess` in `DEPLOY.md` does;
  `python3 -m http.server` doesn't). Browser APIs that need a real origin, such as
  notifications, may not work in the preview. Built bundles are not affected.

### Verification
- New `tests/web_ide_preview_isolation.mjs` (15 checks). It runs a hostile program that
  tries to read a planted fake API key from IDE `localStorage` and the IDE DOM, and to
  forge IDE messages, and then forges messages from the outside. All checks pass on 2.8.3.
  **On 2.8.2 the same test fails 11 checks**: the program read the fake key, read the
  DOM, forged the status bar, and a forged debug-pause threw an error in the IDE.
- Preview dialogs (`ShowMessage`) and downloads confirmed working in Chromium.
- Passing: `web_ide_smoke`, `web_ide_e2e` (6/6, same preview output sizes as before),
  `web_ide_designer`, `web_ide_debugger_test` (pause and step over the port),
  `web_ide_round3`, `web_ide_round4`, `web_ide_assets`, `web_ide_tree_validation`.
  Three tests (`round4`, `bugfixes`, `phaseF`) now inspect the preview through Playwright's
  frame API. `web_ide_bugfixes` and `web_ide_phaseF` still fail on assertions that fail
  identically on 2.8.2 (About-dialog credits, project restore), unrelated to this change.

## [2.8.2] — 2026-09-24

Security hardening release (Roadmap Phase 0, Sprint 1).

### Security
- **Build server no longer reachable from the network or from websites (SEC-01, critical).**
  `rapidr-buildserver` compiles arbitrary code, including `RUSTSTART` blocks built with cargo.
  It previously listened on `0.0.0.0` with fully permissive CORS. It now:
  - binds to `127.0.0.1` by default (`RAPIDR_BUILDSERVER_HOST` overrides, with a warning);
  - rejects requests whose `Host` or `Origin` is not loopback, blocking cross-site
    requests and DNS rebinding;
  - only answers CORS preflights from loopback origins.
- **Build server path traversal fixed (SEC-11).** `/preview/<id>/../..` could read files
  outside the build directory; paths with `..` or absolute components are now rejected.
- **JavaScript injection in `RStringList.SaveToFile` fixed (SEC-04).** List contents were
  spliced into `eval`'d JavaScript with incomplete escaping. Downloads now go through a
  Blob URL with the content passed as data.
- **Captions and `PRINT` no longer interpret HTML (SEC-05).** `Caption`/`Text` on buttons
  and other elements used `innerHTML`, and `PRINT` appended to `#rr-console` as HTML, so
  displaying database, HTTP or other untrusted data could inject scripts. Both now use
  plain text. Use `RDOM.InnerHTML` or `RWebView.HTML` when markup is intended.
- **Removed string-built `eval` from the web runtime (SEC-06).** `RHttp.Get/Post`,
  `BEEP` and `SOUND` now call `XMLHttpRequest` and Web Audio through `web_sys`.
  `RJavaScript.Eval` is the only remaining `eval`, by design.
- Added `clippy.toml` banning `js_sys::eval` so it can't be reintroduced accidentally.

### Fixed
- `RHttp`, `BEEP`, `SOUND` and `RStringList.SaveToFile` now work in bundles built by the
  IDE, whose Content-Security-Policy blocks `eval`.
- `RHttp` no longer mis-parses response bodies (the hand-rolled JSON round-trip was removed).
- `BEEP`/`SOUND` reuse one `AudioContext` instead of creating one per call (browsers cap them).

### Added
- `ROADMAP.md` — the plan toward full RapidQ/VB compatibility, the debugger, AI features
  and the security audit.
- `CHANGELOG.md` (this file).

### Verification
- Build server: unit tests for loopback host/origin and path checks; live requests confirmed
  local requests succeed while cross-site, DNS-rebinding and `null`-origin requests get 403.
- Web runtime: a hostile test program (HTML payloads in captions and `PRINT`, an injection
  payload in `SaveToFile`, `RHttp` GET/POST, `BEEP`) run in Chromium both without a CSP and
  under the IDE bundle's CSP — no script execution, no CSP violations, all features working.
- Existing Playwright suites pass: `web_ide_smoke`, `web_ide_e2e` (6/6), `web_ide_designer`,
  `web_ide_debugger_test`.

## [2.8.1] — 2026-06-01

### Fixed
- Dynamic `RDOM` tag swapping to fix background CSS leakage.

## [2.8.0]

### Added
- Visual widget outline, Assets Manager explorer, and module/form identifier
  sanitization suggestions.

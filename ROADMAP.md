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

| `MID$` / `LEFT$` / `RIGHT$` (web runtime) | Slice by bytes → panic (app crash) on multi-byte UTF-8 text, e.g. `"héllo"` (`crates/rapidr-runtime-web/src/builtins.rs` `rp_mid`/`rp_right`; check native + VM too) |

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
| SEC-08 | Review | 23 `unsafe` in web host; event dispatcher stores leaked raw `*mut Vm` → possible aliasing/UB on re-entrant events (during `ShowModal`, sync XHR). 68 `unsafe` in FFI | `interpreter/rapidr-vm-host-web/src/lib.rs:237`; `crates/rapidr-runtime-core/src/ffi.rs` | Re-entrancy guard / `Rc<RefCell<>>`; document invariants; Miri |
| SEC-09 | Gap | No cargo-audit / cargo-deny, no fuzzing, no SECURITY.md; IDE loads Google Fonts (third-party) | — | Phase 0 + Phase 6 |
| SEC-11 | High | Build server preview path traversal: `dir.join(rel).starts_with(dir)` does not catch `..` → arbitrary file read | `crates/rapidr-buildserver/src/main.rs` (`serve_preview_path`) | Reject any non-`Normal` path component |
| SEC-12 | Medium | `RWebView.HTML` uses `srcdoc` with default sandbox `allow-scripts allow-same-origin` → HTML runs with the app's origin | `crates/rapidr-runtime-web/src/gui_web.rs` (`"html"` prop, iframe creation ~:2201) | Drop `allow-same-origin` for `srcdoc` content by default; opt-in property |
| SEC-10 | Gap | `$INCLUDE` resolves arbitrary paths; projects with `RUSTSTART` / `DECLARE … LIB` get no warning on open/build | `crates/rapidr-preprocessor/src/lib.rs` | Confine includes to project root in IDE/MCP contexts; "native-privileged" project flag + confirmation |

### IDE / debugger baseline

Has: Monaco editor, regex-based completion/hover/signature help, visual designer, debugger (breakpoints, step in/over/out, stack, variables, watch list, component properties), assets manager, zip build, themes.
Missing: compiler diagnostics as editor markers, **undo/redo (stub at `web-ide/host.js:2288`)**, immediate-window evaluation (stubs), conditional breakpoints/logpoints, native/DAP debugging. `web-ide/host.js` is a 4,587-line monolith.

---

## Phase 0 — Stabilize & secure (~3 weeks)

**Sprint 1 (~2 weeks)**
- [x] SEC-01 + SEC-11: `rapidr-buildserver` locked down — binds 127.0.0.1 (override `RAPIDR_BUILDSERVER_HOST` warns), Host/Origin loopback guard (blocks cross-site + DNS rebinding), CORS loopback-only, path traversal fixed; unit tests + live curl probes (v2.8.2)
- [ ] SEC-02 / SEC-03: separate preview origin, verify message source + nonce
- [x] SEC-04: `SaveToFile` now uses shared `trigger_download` Blob helper (no eval); wasm check passes — verified in Chromium with and without bundle CSP (v2.8.2)
- [x] SEC-05: Caption/Text fallback and `PRINT` → `#rr-console` now plain text; markup only via `RDOM.InnerHTML` / `RWebView.HTML` (v2.8.2)
- [x] SEC-06: `RHttp`, `BEEP`, `SOUND` via `web_sys` (XHR / Web Audio); only `RJavaScript.Eval` remains; `clippy.toml` bans `js_sys::eval` (v2.8.2)
- [ ] Unsupported constructs → hard diagnostics (parser + bcgen + codegen); no silent drops
- [ ] Wasm `compile()` returns structured diagnostics → Monaco `setModelMarkers`
- [ ] `tests/conformance/` harness: `*.bas` + `*.expected`, runs on VM **and** Rust codegen; seed with Appendix A programs
- [ ] `cargo-deny` (advisories + licenses) in CI
- [ ] Undo/redo in the IDE (needed before AI edits)

**Rest of Phase 0**
- [ ] SEC-07: SQL parameter binding (SQLite, MySQL, web SQLite)
- [ ] CSP generated per bundle from components used (e.g. `'unsafe-eval'` only if `RJavaScript` is used; `connect-src` for `RHttp`/`RAI` hosts; `frame-src` for `RWebView`)
- [ ] Single language registry → generate `lang-data.js`, VS Code data, manual sections

## Phase 1 — RapidQ & VB compatibility (~6–8 weeks)

- [ ] `$DIALECT RAPIDQ | VB6 | RAPIDR` (Q-aliases, ByRef default, `Me`/`This`, rounding rules)
- [ ] Built-in virtual `RAPIDQ.INC` (constants, colors, key codes, `mr*`, `MB_*`)
- [ ] Central Q→R type alias table (`QFORM`→`RFORM`, … all components)
- [ ] `?` as PRINT, `INC`/`DEC`, `PRINT` tab zones
- [ ] `GOSUB`/`GOTO`/labels (VM: jumps; codegen: state-machine transform for fns with labels)
- [ ] `TYPE … EXTENDS` with `EVENT … END EVENT`, `CONSTRUCTOR`
- [ ] Accept `$TYPECHECK`, `$RESOURCE`, `$OPTION ICON`, etc.; forward `DECLARE SUB` as no-op
- [ ] Fix builtins: `REPLACESUBSTR$`, `INSERT$`, `FORMAT$`, `STRF$` (+ audit all builtins vs RapidQ docs)
- [ ] Win32 shim table for top ~50 `DECLARE … LIB "user32"/"kernel32"/"shell32"` calls; clear warnings for the rest
- [ ] VB6: `On Error GoTo/Resume Next`, `Optional`, `ParamArray`, `Property Get/Let/Set`, `Enum`, `Static`, `ReDim Preserve`, `For Each`, `_` continuation, `Select Case Is/To`, `Like`
- [ ] Modern `TRY/CATCH`
- [ ] Runtime errors carry source line on both backends
- [ ] **RapidQ importer**: folder/zip → follow `$INCLUDE` → `CREATE` trees become designer forms → modules → compatibility report
- [ ] **VB6 importer**: `.vbp` + `.frm` (`Begin VB.Form …`) + `.bas`
- [ ] Imported projects with `RUSTSTART`/`DECLARE LIB` flagged native-privileged (SEC-10)
- [ ] Corpus: collect 50–100 real RapidQ programs (+ VB6 samples); publish pass-rate
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
